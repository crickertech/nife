//! The QEMU test harness.
//!
//! `cargo test` builds the kernel with `cfg(test)`, the runner in
//! `.cargo/config.toml` boots it under QEMU, and we report pass/fail by asking QEMU
//! to exit with a status code via semihosting. Cargo reads that status and calls it
//! a pass or a failure.
//!
//! Set up on day one on purpose. The alternative is debugging by `println!` for a
//! year (DECISIONS.md §7).

use core::sync::atomic::{AtomicBool, AtomicPtr, AtomicU64, AtomicUsize, Ordering};

use crate::arch::semihosting;
use crate::{print, println};

// The hang watchdog: **two independent mechanisms, because there are two ways a test never
// finishes.** Both run off the timer IRQ, so they cost a couple of atomic loads per tick and cannot
// perturb the scheduling they watch. Either one firing fails the run with a thread dump.
//
// **1. The no-progress heartbeat (a lost wakeup).** [`note_progress`] bumps a counter on every
// observable kernel step: a completed IPC rendezvous or device-IRQ wake (`sched::wake` /
// `wake_load_aware`) and every line of console output (`console::_print`, which covers each test's
// "ok"). Progress is also credited when any online core is running a real, non-idle thread
// ([`any_core_running_real_work`]). If none of that happens for ~60 s the run fails. That second
// signal is what lets `std_net` pass honestly: it spends its ~300 s in net_stack's *userspace* smoltcp
// poll, CPU-bound, making no wake and no output for stretches over a minute, yet a real thread runs
// the whole time. A genuine lost wakeup is the opposite: every thread `Blocked`, every core parked on
// its idle thread.
//
// **2. The per-test wall-clock ceiling (a livelock).** The heartbeat above cannot see a livelock that
// *makes progress*, and that is not hypothetical: the RedoxFS repeat-write loop spins in an allocator
// commit while still doing blk IPC, so every rendezvous reset the heartbeat and a failure that used to
// be a loud 60 s trip became an infinite silent hang at ~400% CPU. A progress-only instrument cannot
// distinguish that from healthy work, so it needs a second question: not "is anything happening?" but
// "has this test taken longer than it is allowed to?" [`Testable::run`] stamps a start time and a
// budget per test; this fails the run when the budget is exceeded **even though progress is being
// made**. Turning a loud failure into a silent hang is strictly worse than the flake the heartbeat
// fixed, which is why both mechanisms are live.
//
// What each can and cannot see, stated plainly:
//
//   - The heartbeat catches a total stall (deadlock, lost wakeup) fast, and catches it wherever it
//     happens, including before any test starts. It is blind to any loop that keeps doing IPC.
//   - The ceiling catches any test that does not terminate, livelock included, regardless of how busy
//     it looks. It cannot fire faster than the budget, so it is a backstop, not a diagnostic.
//   - Neither can tell a livelock from slow-but-correct work *while it is running*; only the budget,
//     which is a human declaration of expected cost, separates them. That is the honest limit.
//   - **The heartbeat credits work by ANY thread, including leftovers from earlier tests, and that
//     blinded it once for real** (milestone 31 phase 2). The FS server died of a stack overflow, its
//     client blocked on a `CALL` nobody would ever answer, and nothing in that test made progress
//     again; but processes left spinning by earlier tests kept `any_core_running_real_work` true, so
//     the 60 s stall never registered and only the ceiling fired. Attributing a thread to the running
//     test would fix it and the kernel cannot: a test's processes are ordinary processes. The defence
//     is the ceiling, plus reading the thread dump's **address-space roots** rather than its program
//     counters, which is what tells a leftover spinner from the process under test.
//   - **A ceiling failure reports the ceiling, not the cost.** "ran 900 s" against a 900 s budget is
//     the budget being spent and says nothing about how long the work would take. The same incident
//     had that number read as evidence of honest slowness, which sent an investigation looking for a
//     slow path in a test whose server was already dead. Raising a budget to "measure" something
//     returns only the new budget.
//   - `scripts/qemu-bounded.sh` remains the outermost backstop, for a kernel that wedges so hard the
//     timer IRQ itself stops. It did NOT fire in the reported case because that run invoked `cargo`
//     directly instead of going through the wrapper: **a bypassable backstop is not a backstop**,
//     which is precisely why the ceiling belongs in the kernel, where nothing can route around it.
static HEARTBEAT: AtomicU64 = AtomicU64::new(0);
static WATCH_LAST_HB: AtomicU64 = AtomicU64::new(0);
static WATCH_STALL_TICKS: AtomicU64 = AtomicU64::new(0);

/// When the running test started, in `arch::timer::now()` counter units, or 0 for "no test is
/// running" (during boot, and between tests), which disables the ceiling.
static TEST_START: AtomicU64 = AtomicU64::new(0);

/// The running test's wall-clock budget in counter units, and its name, for the failure report. The
/// name is a `&'static str` split into pointer and length so the tick path never takes a lock; it is
/// only reassembled on the failure path.
static TEST_BUDGET: AtomicU64 = AtomicU64::new(0);
static TEST_NAME_PTR: AtomicPtr<u8> = AtomicPtr::new(core::ptr::null_mut());
static TEST_NAME_LEN: AtomicUsize = AtomicUsize::new(0);

/// **A test's fixture is not there, on this machine, and that is not this test's fault.**
///
/// Milestone 145: the on-board test-suite exit (milestone 16a) ran the `#[test_case]` suite on
/// the VisionFive 2 for the first time, and found roughly thirty tests that correctly expect a
/// synthetic device only `xtask`'s QEMU runners attach (virtio-rng, virtio-gpu, an NVMe
/// controller, ...). None of them was wrong; none of them had ever needed to run anywhere else.
/// `Testable::run` had no way to record a third outcome besides pass (return) and fail (panic),
/// so a test with no fixture had exactly one honest option: crash the whole suite, which is what
/// `nvme.rs`'s end-to-end test did.
///
/// The boot tour already has the shape this borrows: `main.rs` prints "skipped (no 'outlaw'
/// program in the initrd)" instead of asserting a fixture that may not be there. `skip!()` is
/// the same move inside a `#[test_case]`: call it where the old code called `.expect(...)`, and
/// it prints the same message a passing test would have, tagged `skipped` instead of `ok`, then
/// returns from the calling function. The macro (not a function) is what makes the early return
/// reach the test: a function can only return `()` and hand back control, not unwind its caller.
pub(crate) static SKIP_REASON: AtomicPtr<u8> = AtomicPtr::new(core::ptr::null_mut());
pub(crate) static SKIP_REASON_LEN: AtomicUsize = AtomicUsize::new(0);
/// How many tests this run has skipped, for the final line. Read once, at the end; never
/// compared against anything today, which is milestone 145's own open question (a run that
/// skips more over time is a fact worth someone eventually gating on, not silently absorbing).
static SKIPPED: AtomicUsize = AtomicUsize::new(0);

/// **Skip the current test**, because the fixture it needs is not attached to this boot.
///
/// `reason` should name the missing thing the way the old `.expect(...)` message did ("no
/// virtio-rng device on the mmio bus"), because that string is now the only record of why the
/// test did not run. Must be called from directly inside a `#[test_case]` function (it returns
/// from its caller); calling it from a nested helper returns out of the helper instead, which
/// is a bug at the call site, not in the macro.
#[cfg(test)]
#[allow(unused_macros)]
macro_rules! skip {
    ($reason:expr) => {{
        let reason: &'static str = $reason;
        $crate::testing::SKIP_REASON.store(
            reason.as_ptr() as *mut u8,
            core::sync::atomic::Ordering::Relaxed,
        );
        $crate::testing::SKIP_REASON_LEN.store(reason.len(), core::sync::atomic::Ordering::Relaxed);
        return;
    }};
}
#[cfg(test)]
#[allow(unused_imports)]
pub(crate) use skip;

// ---------------------------------------------------------------------------------------------
// The frame ledger.
//
// **A boot has one pool of physical frames and no test gives an account of what it took.** That is
// how the aarch64 suite spent its way to `Unmappable(OutOfPageFrames)` in whatever test happened to
// spawn last, one run in three, three milestones in a row blaming the wrong code (notes/frames.md).
// The instrument that settled it in milestone 107 was four lines in `memory_region::create`, thrown away
// after one run; this is the same idea kept, so the next person reads a number instead of building
// one.
//
// Two numbers per test, because they answer different questions and only the second fails a boot:
// how many frames the test never gave back, and the longest run still allocatable afterwards. A
// suite can hold a comfortable free total and refuse a 128-page request; 137 free with no run of
// 128 is the measured case.
//
// Costs one bitmap scan per test (O(total), ~32k frames), between tests, off every hot path.
// ---------------------------------------------------------------------------------------------

/// Free frames when the first test started: the ledger's opening balance.
static FRAMES_AT_START: AtomicUsize = AtomicUsize::new(0);
/// Whether [`FRAMES_AT_START`] has been stamped (0 is a legal reading, so a flag rather than a
/// sentinel).
static FRAMES_STAMPED: AtomicBool = AtomicBool::new(false);
/// The reading taken at the top of the test now running, and that test's name. The charge against a
/// test is the drop from *its* reading to the *next* one, which is why both are carried forward.
static FRAMES_AT_PREV: AtomicUsize = AtomicUsize::new(0);
static PREV_NAME_PTR: AtomicPtr<u8> = AtomicPtr::new(core::ptr::null_mut());
static PREV_NAME_LEN: AtomicUsize = AtomicUsize::new(0);
/// The worst single spender so far, and its name, for the closing summary. Same pointer/length
/// trick as the test name above: no allocation, no lock.
static WORST_SPEND: AtomicUsize = AtomicUsize::new(0);
static WORST_NAME_PTR: AtomicPtr<u8> = AtomicPtr::new(core::ptr::null_mut());
static WORST_NAME_LEN: AtomicUsize = AtomicUsize::new(0);

/// Report a test's frame cost once it reaches this many frames. Below it, silence: a test that
/// spends two pages on an endpoint is not news, and a number on every line buries the ones that
/// are. Sixteen frames is 64 KiB, about the smallest thing a service-shaped test takes.
const PAGE_FRAME_REPORT_MIN: usize = 16;

/// **What the whole suite may leave unreturned, in frames**, checked once at the end of the run.
///
/// Not zero, and the difference is accounted rather than shrugged at. A boot legitimately keeps some
/// of what its tests built: the FS service and the credential store are wired once and shared by
/// every later test that needs them, `root_supervisor`'s budget cannot be reclaimed at all (it was
/// `SPLIT`, and a parent with live children refuses), and each kernel endpoint costs a page that is
/// never freed by design. notes/frames.md lists every item and what it is for.
///
/// What must not *grow* is the per-test residue, and this ceiling sits just above the measured total
/// so that a new service-shaped test which forgets to hand its memory back fails **here**, naming
/// itself, instead of three tests later as `OutOfPageFrames` in something innocent.
///
/// **One number for both architectures, and it is one because they were measured separately and
/// came out together**: 14031 on aarch64 and 13787 on riscv64 (2026-08-16, remeasured after merging
/// milestones 54 and 55; 13999 and 13791 before that). The two boots build
/// different sets of services, so that agreement is a coincidence rather than a property, and if it
/// ever stops holding the honest fix is a per-architecture pair here rather than a looser single
/// ceiling.
///
/// **Raised again, 2026-08-22, milestone 49's login service.** `login`'s test suite wires one
/// instance of the service (a memoized `DONE` flag, `credential_service`'s and `fs_service`'s own
/// shape) with a 640-frame construction budget: `crate::memory_region::create` reserves that whole amount
/// the moment the service is spawned, whether or not every page is later split into an object, which
/// is this ledger's own documented property (a reservation costs what it reserves). It never gives
/// the budget back, on purpose: the service keeps serving logins for the life of the boot, the same
/// shape `credential_service`'s 1656-frame credential store already is in this ledger. Measured
/// aarch64 total: 15624 kept. notes/frames.md's held-frames list carries the itemised account.
///
/// **Raised again, same day, same milestone, a capability table bug rather than a new feature.** `mint()` in
/// `user/src/login.rs` used to leak one of `login`'s own sixteen capability table slots per successful login
/// (the caretaker's construction region capability was never freed), which bounded the service to
/// exactly eight logins ever regardless of `CONSTRUCTION_UT`'s size. The fix needed a regression
/// test that actually crosses that old ceiling, so `login_tests.rs`'s `CONSTRUCTION_PAGES` grew from
/// 640 to 1408 (nine real logins' worth instead of three) to cover it. Every one of those extra 768
/// pages is permanent for the same reason the 640 above are: `user/src/login.rs`'s BUGS still names
/// the caretaker's construction *memory* (as opposed to the capability table slot this lane fixed) as never
/// reclaimed. 16200 + 768 = 16968.
///
/// **Raised again, 2026-08-23, milestone 155's provisioning tool.** Its own guest suite
/// (`kernel/src/user/identity_provisioning_tests.rs`) needs a credential store *before* it is
/// sealed, which the tree's one shared fixture cannot offer once it returns (`credential_tests.rs`'s
/// own doc: the seal deletes the provision endpoint at both ends). So the suite wires a **second**,
/// independent `credential_service` instance, the same permanent shape the first one already is in
/// this ledger (`CRED_BUDGET_PAGES` 1536 + `CRED_STACK_PAGES` 16 = 1552, reserved once at `start()`
/// and never given back, because nothing in this tree tears a credential service down), plus the
/// small cost of the two `identity_provisioner` invocations run against it and the one
/// `fs_subtree_caretaker` its headline test builds to prove the subtree that tool created is real.
/// Measured directly from the suite's own `[that test kept N frames]` lines rather than
/// re-derived: 1606 (setup: the second instance plus both provisioning attempts) + 53 (the
/// caretaker) = 1659. 16968 + 1659 = 18627. See notes/frames.md's held-frames list for the same
/// account.
///
/// **Raised again, 2026-08-25, milestone 139 round 4's `MappedWindow` migration.** Isolated by
/// bisecting CI's own `build + test (host + QEMU)` logs rather than a local run (see below for
/// why a local measurement could not be used directly): the aarch64 leg's "never returned" count
/// held at **18621** across five independent group-build runs spanning 2026-08-21 through the
/// commit just before this migration (`c152b752`), then moved to **18626** across two independent
/// runs of the migration's own merge commit (`a176da29`) and stayed there through two more
/// no-op (markdown-only) commits on top. Both populations are internally consistent (zero
/// variance within seven and four samples respectively), so the +5 is attributed to
/// `202831a3`/`c94f5d21` (`painter`/`window`/`display`/`display_terminal` onto
/// `user_rt::mapped_window::MappedWindow`) with confidence, even though the exact byte-level
/// accounting below is not.
///
/// The mechanism: `MappedWindow::check` panics with a **formatted** message (`"MappedWindow:
/// offset {off:#x}, size {size}, is outside the {}-byte window"`), and pulling in Rust's
/// `core::fmt` panic-with-arguments machinery once is enough to drag it into the whole binary.
/// Measured directly (`llvm-size` on the four migrated programs, dev profile, before and after):
/// `painter` grew from 11,648 to 65,320 text bytes, `window` from 13,784 to 68,268, `display`
/// from 15,220 to 69,496, all roughly +54 KiB, while `display_terminal` (which already pulled in
/// formatting elsewhere) grew only ~5 KiB. `display` is the standing candidate for where this
/// becomes *permanent*: `display_tests.rs`'s own comment says outright that "the driver is a
/// long-lived server and never exits" for the rest of that test's boot, and `user.rs::load`
/// sizes a process's whole `AddressSpace` from its ELF segments' page count (`content = sum of
/// every PT_LOAD segment's pages + 1`), so a bigger permanently-resident binary should cost more
/// permanently-resident frames by construction.
///
/// **What this account does not claim.** The per-program arithmetic above (`display`'s own
/// segments alone measure roughly +14 pages between the same two commits) does not cleanly sum
/// to +5, and that specific test's own `[that test kept N frames]` line moved by only +1 across
/// the same CI comparison. The instrument's own documented limitation
/// (`report_frame_ledger`'s attribution is by whichever test was running, not by the code that
/// spent it, and `#[test_case]` registration order can shift when unrelated code size changes
/// elsewhere) means a clean per-test diff is not trustworthy evidence here, only the suite-wide
/// total is. So: **the +5 total is established by direct, repeated CI measurement; which of the
/// four migrated programs it comes from, in what proportion, is not.** Reopen this if `display`,
/// `painter`, `window` or `display_terminal` gains another persistent test fixture and the total
/// jumps again, since that would be the corroborating data point this entry does not have.
///
/// **A second, smaller effect rides on top, and is not this migration's.** The same code,
/// rebuilt and rerun locally (macOS, the same pinned QEMU version, `.qemu-version` matches CI's
/// `11.0.2`) measured **18627**, consistently, across six separate local runs with zero variance,
/// one frame above CI's own clean 18626. And the one CI run that actually exercises this PR
/// (`toolchain/nightly-bump`, after the bump to `nightly-2026-08-25` plus two markdown-only
/// commits, otherwise identical code to `a176da29`) measured **18628**, one frame above that
/// again. Ruled out directly: a `clippy`-only fix in `user/src/swish.rs` (an indexed loop
/// rewritten to an iterator, landing alongside this budget change) made no difference when tried
/// with and without it, and the QEMU version is identical between environments. Not ruled out:
/// genuine cross-environment or run-to-run nondeterminism in a suite that runs real SMP guest
/// code across four emulated cores under TCG, which is exactly the kind of variance
/// `report_frame_ledger`'s own accompanying `BUGS` note already warns neither ceiling is a tight
/// bound against. This entry raises the budget enough to cover the worst value actually observed
/// (18628) plus a margin matching this ledger's own historical headroom (the 16968 -> 18627 raise
/// carried none, and zero headroom is exactly what let 139 round 4's real cost turn a passing gate
/// red); it does not claim to have explained the last one or two frames of it.
///
/// 18626 (the migration's own confirmed cost, 18621 + 5) + 6 (restored headroom, matching the gap
/// this budget carried before it silently spent nearly all of it) = 18632.
///
/// **Raised again, 2026-08-26, milestone 49's channel-per-client login.** Measured against a full
/// aarch64 run of `origin/main` at `d8f1d9bb` on the same machine, minutes apart, rather than
/// against this comment's own history: main kept **18631** (one frame under this budget) and this
/// branch keeps **19054**. The +423 has two named parts and a remainder this instrument cannot
/// attribute, stated as such:
///
/// - **+320, `login_tests.rs`'s `CONSTRUCTION_PAGES`, 1856 -> 2176.** That constant's own doc
///   carries the page-by-page account; the short version is that this milestone adds a test
///   (`two_clients_connecting_together_get_independent_channels_and_neither_observes_the_others_secret`)
///   which leaves **two more sessions logged in**, at the 128 pages a live session costs, and
///   `user/src/login.rs` gains a 32-page `CHANNEL_UT_PAGES` split once at startup. 1664 + 256 + 32
///   = 1952 permanently resident, and 2176 is that with the same ~10% margin 1856 carried over
///   1664.
/// - **+104, `login_service.rs`'s `CLIENT_SCRATCH_UT_PAGES`.** Every spawned `login_test_client`
///   role now needs a four-page region of its own to pay for mapping the staging page
///   `login_proto::CONNECT` delegates it, and nothing reclaims it when the role exits. Twenty-six
///   roles run across this suite. This is the one part of the raise that is scaffolding rather than
///   a property under test, and it is recorded at that constant as work someone could take.
/// - **The remaining ~0 to 40 frames are not attributed**, and the per-test diff says why: nine
///   unrelated tests moved by amounts that cancel in pairs (`+36/-32`, `+16/-38`, `+32/-32`,
///   `+62/-62`), which is exactly the `#[test_case]`-registration-order drift
///   `report_frame_ledger`'s own BUGS note warns the per-test attribution suffers from. Only the
///   suite-wide total is evidence.
///
/// 19054 (measured) + 6 (the same headroom the 18626 -> 18632 raise restored, for the one-or-two
/// frame cross-environment variance that raise documented) = 19060.
///
/// **Raised again, 2026-08-26, milestone 47's `printenv` (DECISIONS §111, `date`'s own shape one
/// manifest field over), landing on top of the 19060 raise above rather than the 18632 it was
/// separately measured against.** `kernel/src/user/printenv_tests.rs`'s four new `#[test_case]`s
/// join a suite that already carries milestone 49's channel-per-client login, so the number this
/// constant needs is the two changes measured together, not 19060 + 85 by arithmetic: this ledger's
/// own convention (see every raise above) is a real run, not a sum of two separate ones, because
/// per-test frame cost is not guaranteed additive across unrelated changes (`report_frame_ledger`'s
/// own BUGS note on registration-order drift is exactly this risk). Measured on CI's own aarch64 run
/// (this milestone's local host reproduces an unrelated, pre-existing flake in
/// `caretaker_teardown_reclaims_a_full_session_worth_of_memory` that does not occur in CI, so CI's
/// own log is the trustworthy source here): **19142** frames kept, the higher of two runs in the
/// same job (19142 and 18921). 19142 + 15 (headroom, this ledger's own precedent for raising past a
/// value it has just spent close to all of) = 19157.
///
/// Raising it is a decision, not a formality: read the `[that test kept N frames]` lines the run
/// prints, find who grew, and be able to say why that growth is permanent.
const SUITE_PAGE_FRAME_BUDGET: usize = 19_157;

/// **The longest run of free frames the boot must still have at the end**, in frames.
///
/// The other half of the gate, and the half that names the actual failure. Loading any program calls
/// `AddressSpace::new`, which calls `memory_region::create`, which calls `alloc_contiguous`: what a boot
/// runs out of is not memory, it is a **contiguous run**, and the two can be far apart. Milestone
/// 107 measured 137 frames free with no run of 128; the boot this gate was written for ended with
/// 216 free and no run longer than **117**, so any test loading a program bigger than that failed as
/// `Unmappable(OutOfPageFrames)`, in whichever test happened to be next rather than in the one that
/// spent the memory. [`SUITE_PAGE_FRAME_BUDGET`] alone would not have caught that, because a suite can
/// pass a residue ceiling and still be fragmented into uselessness.
///
/// 1024 frames is 4 MiB, comfortably more than the largest program this boot loads (`redoxfs_server` and
/// `net_stack` are the big ones, a few hundred pages with their page tables) and comfortably under
/// the 14080 measured after reclamation. It is a floor with room, not a target.
const SUITE_MIN_FREE_RUN: usize = 1024;

/// **Charge the test that just finished, and open an account for the one about to start.**
///
/// Called at the top of every test, and once more from the ledger, so the readings **partition the
/// run**: what a test is charged is the drop between its own reading and the next one, and every
/// frame lost between the first test and the last is charged to exactly one test. That matters more
/// than it sounds. Reading free frames before and after the test body instead attributes only what
/// the test spent *while it was running*, and a test that spawns a service and returns as soon as it
/// has its report leaves the service still mapping its heap: on the first measured aarch64 boot that
/// under-attribution was **17362 of the 29091 frames**, a clear majority landing nowhere.
///
/// The cost is a one-test lag in the transcript, which is why the line is printed after the previous
/// test's `ok` rather than on it.
fn charge_previous(now: usize, next: &'static str) {
    let prev_ptr = PREV_NAME_PTR.load(Ordering::Relaxed);
    if !prev_ptr.is_null() {
        let spent = FRAMES_AT_PREV.load(Ordering::Relaxed).saturating_sub(now);
        if spent >= PAGE_FRAME_REPORT_MIN {
            println!("    [that test kept {spent} frames]");
        }
        if spent > WORST_SPEND.load(Ordering::Relaxed) {
            WORST_SPEND.store(spent, Ordering::Relaxed);
            WORST_NAME_PTR.store(prev_ptr, Ordering::Relaxed);
            WORST_NAME_LEN.store(PREV_NAME_LEN.load(Ordering::Relaxed), Ordering::Relaxed);
        }
    }
    FRAMES_AT_PREV.store(now, Ordering::Relaxed);
    PREV_NAME_PTR.store(next.as_ptr() as *mut u8, Ordering::Relaxed);
    PREV_NAME_LEN.store(next.len(), Ordering::Relaxed);
}

/// The worst spender's name, reassembled. Only called by the closing summary.
fn worst_spender_name() -> Option<&'static str> {
    let ptr = WORST_NAME_PTR.load(Ordering::Relaxed);
    let len = WORST_NAME_LEN.load(Ordering::Relaxed);
    if ptr.is_null() || len == 0 {
        return None;
    }
    // SAFETY: the pair was stored from a `&'static str` (`core::any::type_name`), which lives for
    // the whole program, and is only ever overwritten by another such pair.
    unsafe { core::str::from_utf8(core::slice::from_raw_parts(ptr as *const u8, len)).ok() }
}

/// Print the ledger, and fail the run if the boot ends with no usable contiguous run
/// ([`SUITE_MIN_FREE_RUN`]) or with more kept than is accounted for ([`SUITE_PAGE_FRAME_BUDGET`]).
fn report_page_frame_ledger() {
    if !FRAMES_STAMPED.load(Ordering::Relaxed) {
        return; // no test ran; nothing was spent
    }
    let end = crate::memory::free_page_frames();
    // Close the last test's account, so the charges partition the whole run with nothing left over.
    charge_previous(end, "");
    let start = FRAMES_AT_START.load(Ordering::Relaxed);
    let run = crate::memory::largest_free_run();
    let spent = start.saturating_sub(end);
    println!(
        "frames: {start} free before the first test, {end} after the last ({spent} never returned); \
         longest free run {run}"
    );
    if let Some(name) = worst_spender_name() {
        println!(
            "  the biggest single spender was {name} at {} frames",
            WORST_SPEND.load(Ordering::Relaxed)
        );
    }
    if run < SUITE_MIN_FREE_RUN {
        println!();
        println!(
            "FRAME LEDGER: the boot ends with no free run longer than {run} frames, under the \
             {SUITE_MIN_FREE_RUN} this gate requires. This is the failure rather than a warning \
             about one: loading a program takes a contiguous run, so the next test to load anything \
             substantial would fail as Unmappable(OutOfPageFrames), and it would do so in whichever \
             test happened to be next rather than in the one that spent the memory. Read the \
             `[that test kept N frames]` lines above for who grew. See notes/frames.md."
        );
        semihosting::exit(semihosting::EXIT_FAILURE);
    }
    if spent > SUITE_PAGE_FRAME_BUDGET {
        println!();
        println!(
            "FRAME LEDGER: the suite kept {spent} frames against a budget of {SUITE_PAGE_FRAME_BUDGET}. \
             A test built a service and did not hand its memory back. Read the \
             `[that test kept N frames]` lines above for who grew; the reclaim path is \
             `kill_thread` + `sched::reclaim_region`, wrapped as `user::holding::Holding`. Do not \
             raise the budget without an account of what is permanent and why. See notes/frames.md."
        );
        semihosting::exit(semihosting::EXIT_FAILURE);
    }
}

/// Report a test's duration once it reaches this many seconds. Below it, silence: most tests are
/// milliseconds and a duration on every line would bury the signal. Above it, the number is what makes
/// a [`SLOW_TESTS`] entry an evidence-based declaration rather than a guess: until this existed, the
/// only way to learn a test's real cost was to set its budget too low on purpose and read the failure.
const SLOW_REPORT_SECS: u64 = 5;

/// **The default per-test wall-clock budget.** Deliberately tight: almost every test in this suite is
/// milliseconds, and a handful of the userspace ones are a few seconds. 90 s is far above anything
/// honest while still being a real net, so a two-second unit test that starts spinning fails in a
/// minute and a half rather than never.
const DEFAULT_BUDGET_SECS: u64 = 90;

/// **The known-slow tests, each declaring its own cost.** A test whose honest runtime exceeds
/// [`DEFAULT_BUDGET_SECS`] must say so here, with the reason, which is the point: the exception is
/// visible, reviewable, and attached to an explanation instead of being absorbed into one enormous
/// global limit that protects nothing. Matched as a substring of the full test path, so the entry
/// covers a test on both architectures.
///
/// Keep budgets roughly 2x the measured time: enough headroom that host load or a debug build does
/// not produce a flaky failure, tight enough to still catch a hang.
const SLOW_TESTS: &[(&str, u64)] = &[
    // Measured ~300 to 344 s on aarch64: a serial net_stack<->std_exerciser pipeline whose time is spent in
    // net_stack's userspace smoltcp poll (DHCP, DNS, then a TCP echo). The longest honest test we have,
    // and the reason a single global ceiling would have to be uselessly large.
    ("std_net_runs_over_the_socket_contract", 700),
];

/// The budget for a test, by name: its [`SLOW_TESTS`] entry if it has one, else the default.
fn budget_secs_for(name: &str) -> u64 {
    let mut secs = DEFAULT_BUDGET_SECS;
    for &(needle, allowed) in SLOW_TESTS {
        if name.contains(needle) {
            secs = allowed;
        }
    }
    secs
}

/// Record one step of forward progress for the hang watchdog (test builds only). Cheap enough to sit
/// on the wake and console paths: one relaxed increment. See the module note above.
#[inline]
pub fn note_progress() {
    HEARTBEAT.fetch_add(1, Ordering::Relaxed);
}

/// Is any online core running a real thread (not its idle fallback)? A lost-wakeup hang leaves every
/// core parked on idle; a slow-but-live test (a userspace CPU-bound loop like `std_net`'s smoltcp
/// poll) always has one running. Read-only across the per-CPU blocks; racy by nature, which a
/// heartbeat sampled once per tick tolerates.
fn any_core_running_real_work() -> bool {
    // The online set, not `0..count` (first-silicon sweep, 2026-08-14): with the VisionFive 2's
    // {1,2,3} online, the count-as-index scan read parked slot 0's statics and never looked at
    // cpu 3, so a suite whose only live work sat on cpu 3 would read as hung.
    crate::smp::online_cpus().any(|c| {
        let pc = crate::cpu::of(c);
        let cur = pc.current.load(Ordering::Relaxed);
        cur != crate::cpu::NO_TID && cur != pc.idle.load(Ordering::Relaxed)
    })
}

/// Called from the timer IRQ each tick (test builds only; see `timer::tick`). Only the boot core
/// watches, so any dump happens once. The boot core is `arch::boot_cpu_id()` (0 on aarch64, but on
/// RISC-V whichever hart QEMU booted), which is also the one hart that ticks in a single-hart test.
pub fn watchdog_tick() {
    if crate::cpu::id() != crate::arch::boot_cpu_id() {
        return;
    }

    // Mechanism 2 first: a test over its budget fails even while it is making progress, which is the
    // whole point (a livelock doing IPC keeps the heartbeat below perfectly healthy).
    check_test_ceiling();

    const STALL_LIMIT: u64 = 6000; // ticks at 100 Hz = 60 s with no progress at all
    let hb = HEARTBEAT.load(Ordering::Relaxed);
    let progress = hb != WATCH_LAST_HB.load(Ordering::Relaxed) || any_core_running_real_work();
    if progress {
        WATCH_LAST_HB.store(hb, Ordering::Relaxed);
        WATCH_STALL_TICKS.store(0, Ordering::Relaxed);
        return;
    }
    if WATCH_STALL_TICKS.fetch_add(1, Ordering::Relaxed) + 1 == STALL_LIMIT {
        println!();
        println!(
            "WATCHDOG: no progress for ~60 s. Every core idle, every thread blocked: a lost-wakeup hang."
        );
        crate::sched::dump_threads();
        semihosting::exit(semihosting::EXIT_FAILURE);
    }
}

/// **Has the running test exceeded its wall-clock budget?** (mechanism 2; see the module note.) Fails
/// the run with the test's name, how long it actually ran, and its budget, so a future livelock reads
/// as "this test exceeded its budget while making progress" and not as an anonymous timeout.
fn check_test_ceiling() {
    let start = TEST_START.load(Ordering::Relaxed);
    if start == 0 {
        return; // no test running: boot, or between tests
    }
    let budget = TEST_BUDGET.load(Ordering::Relaxed);
    let elapsed = crate::arch::timer::now().wrapping_sub(start);
    if budget == 0 || elapsed <= budget {
        return;
    }

    // Over budget. Disarm first, so anything that prints or wakes from here (the dump) cannot
    // re-enter this path and report twice.
    TEST_START.store(0, Ordering::Relaxed);

    let hz = crate::arch::timer::frequency().max(1);
    println!();
    println!(
        "WATCHDOG: test exceeded its {} s budget (ran {} s) WHILE MAKING PROGRESS: a livelock, not a \
         lost wakeup. The no-progress heartbeat cannot see this, which is why the per-test ceiling \
         exists. If this test is honestly this slow, give it an entry in testing.rs SLOW_TESTS.",
        budget / hz,
        elapsed / hz,
    );
    if let Some(name) = current_test_name() {
        println!("  test: {name}");
    }
    crate::sched::dump_threads();
    semihosting::exit(semihosting::EXIT_FAILURE);
}

/// The running test's name, reassembled from the pointer/length pair stamped by [`Testable::run`].
/// Only called on the failure path.
fn current_test_name() -> Option<&'static str> {
    let ptr = TEST_NAME_PTR.load(Ordering::Relaxed);
    let len = TEST_NAME_LEN.load(Ordering::Relaxed);
    if ptr.is_null() || len == 0 {
        return None;
    }
    // SAFETY: the pair was stored from a `&'static str` (`core::any::type_name`), which lives for the
    // whole program, and is only ever overwritten by another such pair.
    unsafe {
        let bytes = core::slice::from_raw_parts(ptr as *const u8, len);
        core::str::from_utf8(bytes).ok()
    }
}

/// **A budget denominated in timer ticks delivered to this core, for a test that has to wait.**
///
/// The unit is the whole point (milestone 62). A `timer::now()` deadline keeps running whether the
/// guest executes an instruction or not, so on a contended host a wall-clock budget shrinks in the
/// only currency a test cares about, which is guest work. Delivered ticks move the other way: a
/// descheduled emulator misses deadlines, so **fewer** ticks arrive over the same stretch of wall
/// clock and the budget stretches under exactly the conditions that made the wall-clock one wrong.
/// This is the same move the drift twins made in `arch/*/timer.rs` (assert the law, not the rate),
/// spent on a wait instead of on a clock.
///
/// It is a budget rather than a wait loop because the two callers poll differently and both are
/// right to. `sched::tests::within_ticks` must not yield, since the thing it waits for is a
/// preemption. `smp::tests::a_migrated_kernel_thread_keeps_its_hart_pointer` must yield, because it
/// is draining workers that share its core, and it checks a *second* assertion on every turn, so it
/// cannot be written as a condition handed to somebody else's loop. What the two share is the
/// budget's arithmetic, and that is all this holds.
///
/// **A change of core re-anchors instead of subtracting.** `timer::ticks()` is per core (§11) and a
/// steal can move this thread between two reads (§28.3), so comparing the counters either side of a
/// migration compares two unrelated numbers. Re-anchoring is also the honest reading: the budget is
/// "how many preemption opportunities this core gave me", and after a migration the answer starts
/// again. The cost is that a thread migrating repeatedly could stretch the budget without bound,
/// which is why nothing here is the last line of defence: the harness's per-test wall-clock ceiling
/// is (see the module note, mechanism 2), and it fails the run whatever the ticks say.
///
/// # EXAMPLES
///
/// ```ignore
/// // Two seconds' worth of preemption opportunities on a quiet host, and more wall clock than that
/// // on a busy one.
/// let mut budget = TickBudget::new(2 * crate::arch::timer::TICK_HZ);
/// while !done() {
///     assert!(!budget.expired(), "the workers never drained");
///     crate::sched::yield_now();
/// }
/// ```
#[cfg(test)]
pub struct TickBudget {
    core: usize,
    start: u64,
    ticks: u64,
}

#[cfg(test)]
impl TickBudget {
    /// Start a budget of `ticks` timer ticks on whatever core is running now.
    pub fn new(ticks: u64) -> Self {
        let (core, start) = Self::sample();
        Self { core, start, ticks }
    }

    /// Has the budget run out? Re-anchors and returns `false` if this thread changed core.
    pub fn expired(&mut self) -> bool {
        let (core, now) = Self::sample();
        if core != self.core {
            self.core = core;
            self.start = now;
            return false;
        }
        now - self.start >= self.ticks
    }

    /// Read the core id and that core's tick count as a pair, re-reading if we moved between the two
    /// reads. Without this the pair can name one core's id and another's counter, which is the
    /// migration hazard this type exists to handle arriving inside the handler for it.
    fn sample() -> (usize, u64) {
        loop {
            let core = crate::cpu::id();
            let ticks = crate::arch::timer::ticks();
            if crate::cpu::id() == core {
                return (core, ticks);
            }
        }
    }
}

/// Lets us print a test's name before running it. `core::any::type_name` gives us
/// the full path of the function, which is close enough to a test name.
pub trait Testable {
    fn run(&self);
}

impl<T: Fn()> Testable for T {
    fn run(&self) {
        let name = core::any::type_name::<T>();

        // The frame ledger, before this test's name is printed: the reading closes the *previous*
        // test's account (and prints its charge under its own `ok` line) and opens this one's. The
        // opening balance is taken at the first test rather than at boot, because what the kernel
        // spends coming up is not a test's doing and is not what this measures.
        let frames_now = crate::memory::free_page_frames();
        if !FRAMES_STAMPED.swap(true, Ordering::Relaxed) {
            FRAMES_AT_START.store(frames_now, Ordering::Relaxed);
        }
        charge_previous(frames_now, name);

        print!("test {name} ... ");
        HEARTBEAT.fetch_add(1, Ordering::Relaxed); // tell the watchdog this test started

        // Arm the per-test wall-clock ceiling (mechanism 2). The name goes first and the start time
        // last, because a non-zero start is what arms the check: this way the watchdog never sees a
        // live budget with a stale name.
        TEST_NAME_PTR.store(name.as_ptr() as *mut u8, Ordering::Relaxed);
        TEST_NAME_LEN.store(name.len(), Ordering::Relaxed);
        TEST_BUDGET.store(
            budget_secs_for(name) * crate::arch::timer::frequency(),
            Ordering::Relaxed,
        );
        let start = crate::arch::timer::now();
        // `now()` could legitimately be 0 on the very first tick of a fresh counter, and 0 is the
        // disarmed sentinel, so nudge it. One counter unit of slack is nothing against a 90 s budget.
        TEST_START.store(if start == 0 { 1 } else { start }, Ordering::Relaxed);

        self();

        // Disarm: between tests there is no budget to exceed, and the next test arms its own.
        TEST_START.store(0, Ordering::Relaxed);

        // A test that called skip!() left a reason here instead of returning normally. Report it
        // and stop: the frame-ledger charge and the stack-intact check below still apply (a
        // skipped test can still smash the stack on its way out), but there is no "ok" to print.
        let skip_ptr = SKIP_REASON.swap(core::ptr::null_mut(), Ordering::Relaxed);
        if !skip_ptr.is_null() {
            let len = SKIP_REASON_LEN.swap(0, Ordering::Relaxed);
            // SAFETY: skip!() only ever stores the pointer and length of a &'static str it holds
            // for the duration of the call, and the swap above is the only reader, so this runs
            // at most once per store.
            let reason = unsafe {
                core::str::from_utf8_unchecked(core::slice::from_raw_parts(skip_ptr, len))
            };
            SKIPPED.fetch_add(1, Ordering::Relaxed);

            let elapsed =
                crate::arch::timer::now().saturating_sub(start) / crate::arch::timer::frequency();
            if elapsed >= SLOW_REPORT_SECS {
                print!("[{elapsed} s] ");
            }
            assert!(
                crate::stack::intact(),
                "this test smashed the stack (headroom: {})",
                crate::stack::headroom()
            );
            println!("skipped: {reason}");
            return;
        }

        // Report what a test actually cost, if it cost anything worth knowing. The ceiling already
        // needs a start time, so this is free, and it closes a real gap: a SLOW_TESTS budget is a
        // human declaration of expected cost, and until now there was no way to learn the cost except
        // by setting a budget too low on purpose and reading the failure. That is a bad way to find
        // out, and it is exactly the position I was in when std_fs tripped the 90 s ceiling with no
        // number attached. Anything under the threshold stays silent so the transcript is unchanged.
        let elapsed =
            crate::arch::timer::now().saturating_sub(start) / crate::arch::timer::frequency();
        if elapsed >= SLOW_REPORT_SECS {
            print!("[{elapsed} s] ");
        }

        // A test that overflows the stack corrupts the kernel and then fails somewhere
        // else entirely, often in a *later* test, or by hanging with no output at all.
        // Checking here pins the blame on the test that actually did it.
        //
        // This is not hypothetical. It is how milestone 3 went. See notes/stack.md.
        assert!(
            crate::stack::intact(),
            "this test smashed the stack (headroom: {})",
            crate::stack::headroom()
        );

        println!("ok");
    }
}

/// **The livelock probe** (feature `watchdog_probe`, EXPECTED TO FAIL the run). Proof that the
/// per-test ceiling catches what the no-progress heartbeat cannot: this loops forever while doing a
/// full IPC rendezvous every iteration, so [`note_progress`] fires constantly and the heartbeat sees
/// a perfectly healthy kernel. That is the shape of the RedoxFS repeat-write livelock (spinning in an
/// allocator commit while still serving blk IPC), which turned a loud 60 s watchdog trip into an
/// infinite silent hang at ~400% CPU.
///
/// Run it with:
///
/// ```text
/// scripts/qemu-bounded.sh 200 cargo test -p kernel \
///     --features watchdog_probe --target aarch64-unknown-none-softfloat
/// ```
///
/// Expected: the run FAILS after this test's 90 s default budget, naming the test and its runtime.
/// Without the ceiling it hangs until the outer bound kills it, which is the regression this guards.
#[cfg(all(test, feature = "watchdog_probe"))]
#[test_case]
fn a_livelock_that_keeps_doing_ipc_trips_the_per_test_ceiling() {
    let ep = crate::sched::create_rendezvous();

    // A partner that answers forever. Both sides rendezvous every pass, so every pass is a wake, and
    // a wake is "progress" as far as the heartbeat is concerned.
    crate::sched::spawn(move || {
        loop {
            let _ = crate::sched::ipc_recv(ep);
        }
    })
    .expect("probe: could not spawn the partner");

    loop {
        crate::sched::ipc_send(ep, [1, 2, 3]);
    }
}

/// Runs every `#[test_case]` in the crate, then exits QEMU.
///
/// A panic anywhere in here lands in the panic handler, which exits with a failure
/// status instead. So there is no "count the failures" logic: the first failing
/// assertion terminates the run. Crude, but a kernel with a failed invariant has no
/// business continuing anyway.
pub fn runner(tests: &[&dyn Testable]) {
    println!();
    println!("running {} tests", tests.len());
    println!();

    for test in tests {
        test.run();
    }

    println!();
    #[cfg(test)]
    // the runner itself is compiled in every build; the instrument only exists in test
    crate::stack::report_high_water();
    report_page_frame_ledger();

    println!();
    // "passed" counts only the tests that actually ran to an "ok"; a skipped test (skip!(), no
    // fixture on this boot) is not a pass, and rolling it into either bucket silently would hide
    // exactly the fact milestone 145 exists to keep visible. A board run says "244 passed, 31
    // skipped" rather than a bare 275 that looks identical to QEMU's full count.
    let skipped = SKIPPED.load(Ordering::Relaxed);
    if skipped > 0 {
        println!(
            "test result: ok. {} passed, {skipped} skipped",
            tests.len() - skipped
        );
    } else {
        println!("test result: ok. {} passed", tests.len());
    }

    semihosting::exit(semihosting::EXIT_SUCCESS)
}
