//! **The workload that does not stop, and the thing that watches it** (milestone 219).
//!
//! `design/fatal-risks.md`'s fifth entry says the concurrency may be wrong in ways QEMU cannot show
//! and that arrive one at a time, forever, and names its decisive experiment as sustained
//! multi-core stress with the load-sensitive assertions live. Until this module existed there was
//! nothing to sustain: the boot tour printed its last line and called `arch::halt()`, and a board
//! sat in `wfi` indefinitely.
//!
//! This is the other end of the boot. With `--features soak` the tour does not halt; it builds a
//! pool of user-mode workers and then watches them forever.
//!
//! # The division of labour, which is the design decision this module embodies
//!
//! **The workload is a user program and the detection is in the kernel.** The defect this risk
//! actually produced was `sched::wake_load_aware` making a receiver `Ready` without a delivery,
//! which is observable *and causable* from userspace through the real syscall path. A kernel-mode
//! stress loop would drive a path no real program takes, which is the weaker experiment: it would
//! be testing an artefact of the test. But a user program cannot assert about kernel internals and
//! should not try, so the assertions stay here, where the trace counters are.
//!
//! `user/src/soaker.rs` is the workload and its header carries the argument for *what* it stresses.
//! This file is the supervisor: it wires the pairs, samples them, and decides.
//!
//! # The tick route, and what it is for (milestone 221)
//!
//! The workload above is saturated, and a saturated workload **never crosses cores**. Milestone
//! 219 measured that and DECISIONS 138 (*how a saturated workload is made to hand threads across
//! cores*) explained it: a rendezvous wake is local by design (DECISIONS §28.2),
//! `sched::wake_load_aware` is reachable only from a device interrupt, and a work steal needs an
//! idle core and a queued thread at the same moment, which a busy machine never has. So the
//! `crossings` count froze at fifteen inside the first second and stayed there through two million
//! round trips.
//!
//! That mattered because `design/fatal-risks.md`'s fifth entry has two halves and only one of them
//! was runnable. The half that was not is the one the risk's single observed defect lived on: a
//! receiver made `Ready` with nothing delivered, on radon, in `wake_load_aware`, reached from
//! `sched::irq_notify`.
//!
//! **So this module signals a rendezvous from `sched::on_tick`**, which all three architectures'
//! timer dispatchers call in real interrupt context on every core, and adds one worker role that
//! blocks on that rendezvous through the `Irq::WAIT` a device driver already uses. A tick then runs
//! the identical sequence a device interrupt runs: [`signal_waiters`] -> `sched::irq_notify` ->
//! `Rendezvous::signal` -> `handshake.serve` -> `wake_load_aware` -> `pick_wake_target` ->
//! `place_on` -> the reschedule IPI.
//!
//! **What crosses is the waiters, not the pairs**, and nothing here may be read as saying
//! otherwise. Rendezvous wakes stay local whatever else is happening, so the callers and responders
//! are exactly as pinned as they were before. This sustains the *wake protocol* across cores under
//! load. It does not make the IPC workload migrate, and only a periodic rebalancer would, which
//! DECISIONS 138 declines. The heartbeat says so in words on every run.
//!
//! # What "it passed" means
//!
//! A soak that ends with nothing printed proves very little, so this one does not end with nothing
//! printed. Every [`BEAT_SECONDS`] it prints one line carrying a **round-trip total**, which is the
//! number a later run is compared against, plus the counters that make the total interpretable:
//! how many of those handoffs actually crossed cores, how contended the machine was, and whether
//! the wake gate ever refused anything.
//!
//! What the number is *for* is comparison, and it has three honest uses and no fourth:
//!
//! - **Between architectures**, so a rate that is an order of magnitude off on one of them is a
//!   question rather than a surprise.
//! - **Between QEMU and silicon**, which is the comparison risk 5 is actually about.
//! - **Against the same machine a month later**, where a large drop is a regression in the IPC path
//!   that no functional test would fail on.
//!
//! It is **not** evidence that the concurrency is correct. See this module's `BUGS`.
//!
//! # How a hang is told from a slow run, agreed with the watcher rather than invented here
//!
//! The heartbeat is on the **wall clock**, not on the work. A machine doing one round trip a second
//! still prints every [`BEAT_SECONDS`], with a rate that says so; a machine doing none still prints,
//! and its stall check fires. Silence means the thing that prints is itself wedged, which is the
//! only thing silence is allowed to mean.
//!
//! `crates/board_console` is the other half of that agreement. It watches for a gap longer than its
//! `--quiet-after` (fifteen seconds by default, three missed beats) and reports `WentQuiet`, exit
//! status 2. Its `Stage::Soak` is reached by [`START_MARKER`] below, and reaching it is what
//! re-arms the quiet check that a completed boot tour otherwise suppresses: a kernel that has
//! halted is *supposed* to be quiet, and one that is soaking is not.
//!
//! # BUGS
//!
//! - **A soak that finds nothing is weak evidence, and a green run must never be quoted as proof
//!   that the concurrency is correct.** This is the milestone's own headline caveat and it is
//!   repeated here because this file is where a reader meets the green run. What a clean eight
//!   hours licenses is one sentence: *this machine did N cross-core IPC round trips without the
//!   wake gate refusing one, without a wrong reply, and without a worker stalling.* It licenses
//!   nothing about the interleavings that did not occur, and the ones that did not occur are where
//!   the remaining bugs are.
//! - **The supervisor burns a thread's worth of scheduling.** It yields in a loop rather than
//!   blocking on a timer, because this kernel has no sleep-until primitive a kernel thread can use.
//!   That is load on the machine under test, which is not entirely a cost (it is one more thread
//!   contending) but it is not free either, and it is why the round-trip rate here is not
//!   comparable with `script/bench`'s IPC numbers.
//! - **A worker that panics takes its thread with it and nothing restarts it.** The stall check
//!   sees a counter stop and fails the run, which is the right outcome, but the report says
//!   "stalled" where "died" would be more use. Reviving workers would need a supervision tree here
//!   and would blur what the run is measuring.
//! - **The round-trip total is sampled while workers are writing it** and may be a few short. See
//!   `soak_page`'s own `BUGS`.
//! - **The tick route does not exercise an interrupt controller.** The timer is not a
//!   controller-routed source, so the claim, mask and complete sequence (the GIC on aarch64, the
//!   PLIC on riscv64, the local APIC on `x86_64`) is not on this path. What is on it is the wake
//!   protocol, which is what the defect was on.
//! - **The hook fires on a timer, which is the one thing the workload cannot starve.** That is why
//!   it works, and it is also why a run with it on says nothing about what happens without it.
//! - **A soak with the tick route on is a different soak.** Every waiter is one more thread and
//!   every migration is real work, so the round-trip rate falls. The rate is comparable with
//!   another run of this same build and with nothing else; see notes/soak.md's table.
//! - **Nothing here sets a duration, and nothing here should.** The kernel soaks until the power
//!   goes away. The watcher decides when enough is enough, which is what makes the QEMU run and the
//!   bench run the same experiment with a different deadline.

use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use paging::Flags;
use soak_page::MAX_WORKERS;

use crate::cap::{Rights, irq_cap, rendezvous_cap};
use crate::user::{self, Mapping, Spawn};
use crate::{arch, memory, println, sched, smp};

/// How often the supervisor speaks.
///
/// Five seconds, against `board_console`'s fifteen-second default quiet window: three missed beats
/// before a run is called a hang. Two would be tight on a board whose console is shared with
/// U-Boot's leftovers; four would take a minute to notice a wedge.
const BEAT_SECONDS: u64 = 5;

/// The line that tells the watcher a soak has begun.
///
/// `crates/board_console`'s recogniser matches this prefix, so the two agree by construction rather
/// than by both being remembered. Changing it means changing that recogniser, and its tests will
/// say so.
const START_MARKER: &str = "soak: started";

/// Wire the pool and never come back.
///
/// The caller is the boot thread at the end of the tour, and this replaces its `arch::halt()`.
pub fn run() -> ! {
    let cores = smp::online_count();
    let groups = topology_groups(cores);
    let callers = CALLERS_PER_GROUP;

    let Some(image) = user::program("soaker") else {
        println!("soak: FAILED: no 'soaker' program in the initrd archive; nothing to run");
        arch::halt();
    };

    // Zeroed, so a first sample cannot read stale RAM as progress that already happened.
    let Some(frame) = memory::alloc_zeroed() else {
        println!("soak: FAILED: no frame for the shared progress page");
        arch::halt();
    };
    let shared = frame.addr();

    // **One tick rendezvous per group** (milestone 221), for the reason `signal_waiters` gives at
    // length: a shared one lets whichever waiter is already running drain a burst of signals through
    // `Rendezvous`'s `pending` path while its queued peers starve, and a loaded host produces
    // exactly those bursts. Measured rather than feared: the shared version failed a run.
    let mut tick_endpoints = [0u64; MAX_GROUPS];
    for slot in tick_endpoints.iter_mut().take(groups) {
        *slot = sched::create_rendezvous();
    }
    // Routed before any waiter exists, so none can meet an unbound route; the signalling itself is
    // still switched on last, below. See `bind_tick_routes`.
    if !bind_tick_routes(&tick_endpoints[..groups]) {
        println!(
            "soak: FAILED: one of interrupts {}..={TICK_INTID_TOP} is already routed, so a tick \
             route would steal it; see TICK_INTID_TOP in kernel/src/soak.rs",
            tick_intid(groups - 1)
        );
        arch::halt();
    }

    // **What milestone 240 needs and nothing else records.** The kernel decides where every worker
    // goes and then forgets; these two arrays are that decision kept, so the census below can be
    // printed from what happened rather than from what the placement policy usually does.
    let mut tids = [0u64; MAX_WORKERS];
    let mut placed = [u8::MAX; MAX_WORKERS];

    for group in 0..groups {
        let endpoint = sched::create_rendezvous();
        // The responder first. A caller whose responder is not yet receiving simply parks on the
        // rendezvous, so the order is not load-bearing; starting the answering half first keeps a
        // fresh boot's first few beats from reading as a stall.
        for member in 0..MEMBERS_PER_GROUP {
            let role = role_of(member);
            let index = worker_index(group, member);
            // A waiter holds an `Irq` capability and nothing else; it never sends or receives on
            // the group's endpoint and must not be able to. That is the same least-authority rule
            // the other three roles get from their rights, one object over.
            let grant = if role == ROLE_WAITER {
                irq_cap(tick_intid(group))
            } else if role == ROLE_RESPONDER {
                rendezvous_cap(endpoint, Rights::READ)
            } else {
                rendezvous_cap(endpoint, Rights::WRITE)
            };
            let started = sched::spawn_reporting_placement(move || {
                user::run(
                    image,
                    Spawn {
                        arg0: role,
                        arg1: index as u64,
                        // A distinct, odd, well-spread seed per worker: the jitter is what keeps
                        // the pairs from locking into one interleaving, and two workers handed the
                        // same seed would defeat it for that pair.
                        arg2: (index as u64)
                            .wrapping_mul(0x9E37_79B9_7F4A_7C15)
                            .wrapping_add(1),
                        grants: &[grant],
                        maps: &[Mapping {
                            va: soak_page::VA,
                            phys: shared,
                            flags: Flags::user_data(),
                        }],
                    },
                )
            });
            let Some((tid, cpu)) = started else {
                println!(
                    "soak: FAILED: could not spawn worker {index} of {}",
                    groups * MEMBERS_PER_GROUP
                );
                arch::halt();
            };
            tids[index] = tid;
            // `pick_spawn_target` returns an online cpu id, and `MAX_CPUS` is what bounds one, so
            // this cannot truncate. Saturating rather than `as` so that a future topology which
            // broke that assumption would report `u8::MAX`, the census's own "no answer" value,
            // instead of a wrong core number a reader would believe.
            placed[index] = u8::try_from(cpu).unwrap_or(u8::MAX);
        }
    }

    // **Last, and after every worker exists** (milestone 221). Signalling earlier would hand the
    // first waiter a backlog to drain before it ever blocked, so the first beat would measure setup
    // rather than the machine. Harmless either way (a signal is counted, not lost), which is why
    // this is the half that waits and the routing above is the half that does not.
    arm_tick_hook(groups);

    let workers = groups * MEMBERS_PER_GROUP;
    println!();
    println!(
        "{START_MARKER} {groups} groups of one responder, {callers} callers, \
         {GRINDERS_PER_GROUP} grinder and {WAITERS_PER_GROUP} tick waiter ({workers} user \
         threads) on {cores} online core(s), beating every {BEAT_SECONDS}s"
    );
    println!(
        "soak: silence longer than a few beats is a hang, not a slow run; the beat is on the wall \
         clock and does not depend on the workload making progress"
    );
    println!(
        "soak: a clean run is a number to compare against, NOT evidence that the concurrency is \
         correct (design/roadmap/219-a-workload-that-does-not-stop.md)"
    );
    println!(
        "soak: the threads that cross cores are the tick waiters, NOT the rendezvous pairs; a \
         rising crossings count is the wake protocol sustained across cores under load, not the \
         IPC workload migrating (design/roadmap/221-a-soak-that-crosses-cores.md)"
    );
    println!(
        "soak: rounds counts IPC round trips and wakes counts tick-route wakes; they are separate \
         because they are separate quantities, and neither rate compares with a soak built \
         differently"
    );

    print_census(
        "where the kernel placed each worker at spawn",
        groups,
        &placed[..workers],
    );
    println!(
        "{CENSUS_MARKER} this is the boot-time lottery and NOT where the run settles: a rendezvous \
         wake queues the peer on the waker's own core (DECISIONS 138, how a saturated workload is \
         made to hand threads across cores), so each group converges onto one core within a few \
         exchanges"
    );
    println!(
        "{CENSUS_MARKER} the beat line's drifted= is how many responders, callers and grinders are \
         no longer on the core the census above put them on; while it reads 0 that census is \
         current, and a nonzero one is followed by the census that replaces it"
    );
    println!(
        "{CENSUS_MARKER} tick waiters are excluded from drifted= because their movement is the \
         point rather than a surprise, and it is already crossings= \
         (design/roadmap/221-a-soak-that-crosses-cores.md)"
    );

    watch(shared, workers, &tids, &placed)
}

/// **The prefix every census line carries** (milestone 240).
///
/// Deliberately not `soak:`. `crates/board_console`'s recogniser matches two substrings on that
/// prefix, [`START_MARKER`] and `soak: t=`, and a census is neither the start of a run nor a
/// heartbeat; giving it its own word keeps a block of census lines from having to be proven
/// harmless against a recogniser it has nothing to do with. It is also what a reader greps for,
/// which is the whole point of printing it.
///
/// Name provisional (milestone 240): calef names what a reader meets.
const CENSUS_MARKER: &str = "soak-census:";

/// Which role the `member`-th thread of a group plays.
///
/// One function rather than the ladder it replaced, because the spawn loop and the census have to
/// agree about it: a census that labelled the roles by a second copy of this arithmetic would be
/// an instrument that could disagree with the thing it measures.
fn role_of(member: usize) -> u64 {
    if member == 0 {
        ROLE_RESPONDER
    } else if member <= CALLERS_PER_GROUP {
        ROLE_CALLER
    } else if member <= CALLERS_PER_GROUP + GRINDERS_PER_GROUP {
        ROLE_GRINDER
    } else {
        ROLE_WAITER
    }
}

/// The one letter a census spends on a role. See [`print_census`].
fn role_letter(role: u64) -> char {
    match role {
        ROLE_RESPONDER => 'R',
        ROLE_CALLER => 'C',
        ROLE_GRINDER => 'G',
        _ => 'W',
    }
}

/// **Print which roles are on which core, one line per core** (milestone 240).
///
/// `cpus[i]` is where worker `i` is, and `u8::MAX` means nothing knows yet. The tokens are
/// `<letter><group>`, so `G0 G3` on one line is that core drawing two grinders, which is the shape
/// milestone 240's block names and the one a reader of a log alone could not previously see.
///
/// A core with no workers gets a line too. That is the informative case rather than the empty one:
/// four cores online and one of them idle is an explanation, and a census that printed only the
/// occupied cores would hide it.
///
/// This states where the threads are and draws no conclusion from it, which is the milestone's own
/// constraint: if a series of boots shows the rate does not follow the placement, that is the
/// result, and this must not have prejudged it.
fn print_census(what: &str, groups: usize, cpus: &[u8]) {
    println!(
        "{CENSUS_MARKER} {what}: R=responder, C=caller, G=grinder, W=tick waiter, and the number \
         after each letter is its group"
    );
    let mut accounted = 0usize;
    for core in smp::online_cpus() {
        let here = u8::try_from(core).unwrap_or(u8::MAX);
        let n = cpus.iter().filter(|&&c| c == here).count();
        accounted += n;
        crate::print!("{CENSUS_MARKER} core={core} threads={n}");
        for (i, &c) in cpus.iter().enumerate() {
            if c == here {
                crate::print!(
                    " {}{}",
                    role_letter(role_of(i % MEMBERS_PER_GROUP)),
                    i / MEMBERS_PER_GROUP
                );
            }
        }
        println!();
    }
    // Everything the per-core loop did not name. Two causes, and both are worth a line rather than
    // a silent shortfall: a worker that has not been switched to yet (its `last_cpu` is still
    // unset), and a core id that is not in the online set, which would be a bug in placement rather
    // than in this census and which `groups` is printed beside so the arithmetic can be checked.
    let missing = cpus.len() - accounted;
    if missing != 0 {
        let unknown = cpus.iter().filter(|&&c| c == u8::MAX).count();
        println!(
            "{CENSUS_MARKER} unplaced={missing} of which not-yet-run={unknown} (groups={groups}, \
             online={})",
            smp::online_count()
        );
    }
}

/// `arg0` for the half that answers, matching `user/src/soaker.rs`.
const ROLE_RESPONDER: u64 = 0;
/// `arg0` for the half that calls.
const ROLE_CALLER: u64 = 1;
/// `arg0` for the half that only computes.
const ROLE_GRINDER: u64 = 2;
/// `arg0` for the role that blocks on the tick route (milestone 221).
const ROLE_WAITER: u64 = 3;

/// How many callers share one responder.
const CALLERS_PER_GROUP: usize = 3;

/// How many pure-compute threads share a group.
const GRINDERS_PER_GROUP: usize = 1;

/// How many tick waiters share a group (milestone 221).
///
/// One, and the number is doing real work rather than being a placeholder. Every waiter is blocked
/// on the *same* rendezvous, and a tick raises one signal, which wakes one of them; so the number
/// of waiters is the number of threads the wake protocol has to choose between, and it wants to be
/// comparable with the core count so that `pick_wake_target` has somewhere to put them. One per
/// group is one per core, since [`topology_groups`] builds a group per core.
const WAITERS_PER_GROUP: usize = 1;

/// How many worker threads one group has.
///
/// One responder, its callers, its grinders and its waiter. Spelled once because it is the divisor
/// in [`worker_index`], the multiplier in the spawn loop, and the bound [`topology_groups`] fits
/// under: three places that were three copies of the same sum before milestone 221 added a fourth
/// term to it.
const MEMBERS_PER_GROUP: usize = 1 + CALLERS_PER_GROUP + GRINDERS_PER_GROUP + WAITERS_PER_GROUP;

/// **The highest interrupt number the tick routes are bound to** (milestone 221). Group `g` uses
/// `TICK_INTID_TOP - g`.
///
/// These name no hardware and none of the three architectures can deliver them, which is the whole
/// requirement: `sched::bind_irq` writes a slot in a flat array that `sched::irq_route` reads, and
/// these only have to be slots nothing else in a soak boot claims.
///
/// **Counting down from 255, and each architecture rules the band out for its own reason.** On
/// aarch64 and riscv64 a routed interrupt is delivered only if something enabled it at the
/// controller, and nothing enables these; the highest number either architecture's device tree
/// actually hands out is two orders of magnitude below them. On `x86_64` the top of the band is
/// `arch::x86_64::irq::SPURIOUS_VECTOR`, which the trap handler answers in its own arm before
/// `irq_route` is ever asked, and the rest of it sits at the far end of the MSI band, which is
/// allocated upward from `MSI_VECTOR_BASE` (0xc0) and would need more than fifty devices to reach.
///
/// **None of that is what makes this safe**, and saying so is the point: the arithmetic is an
/// argument and [`arm_tick_routes`] is a mechanism. It asks `sched::irq_route` about every number
/// it is about to take, and refuses to start a soak whose tick routes would steal somebody else's
/// interrupt. A soak boot runs the whole tour first, so every device that is going to claim an
/// interrupt has already claimed it by the time that check runs.
///
/// Name provisional (milestone 221): calef names public items.
const TICK_INTID_TOP: u32 = 255;

/// The most groups the shared page has room for, and so the most tick routes there can be.
const MAX_GROUPS: usize = MAX_WORKERS / MEMBERS_PER_GROUP;

/// The interrupt number group `g`'s waiter holds. See [`TICK_INTID_TOP`].
const fn tick_intid(g: usize) -> u32 {
    TICK_INTID_TOP - g as u32
}

/// **The rendezvous [`signal_waiters`] signals, one per group, each plus one; zero means empty.**
///
/// Plus one for the same reason `sched::IRQ_ROUTES` does it: a rendezvous name may legitimately be
/// zero, and this needs a value that means "nothing here" without a second word to say so.
///
/// Plain atomics rather than anything under a lock, because the reader is an interrupt handler on
/// every core: taking a lock to find out *whether there is anything to do* is a thing that can go
/// wrong, and a single load cannot be. Written once each, by the boot thread, after every worker
/// exists.
static TICK_ROUTES: [AtomicU64; MAX_GROUPS] = [const { AtomicU64::new(0) }; MAX_GROUPS];

/// How many of [`TICK_ROUTES`] are armed. Zero until [`arm_tick_routes`] has bound them all, which
/// is what keeps the hook inert during setup and what [`signal_waiters`] loads first.
static TICK_GROUPS: AtomicUsize = AtomicUsize::new(0);

/// Which route the next tick signals, anywhere on the machine. See [`signal_waiters`].
static TICK_CURSOR: AtomicUsize = AtomicUsize::new(0);

/// **Signal one tick route. Called from `sched::on_tick`, in interrupt context, on every core.**
///
/// This is the whole of milestone 221's kernel-side mechanism, and it is short because the path it
/// wants was already built: `sched::irq_notify` is the function a device interrupt goes through, it
/// documents itself as safe from interrupt context (`IPC_TABLES` is an `IrqSafeMutex`, DECISIONS
/// §9, so the interrupted code on this core cannot have been holding it), and everything
/// interesting is downstream of it.
///
/// Not armed on an ordinary boot, because there is no ordinary boot: the whole module is behind
/// `--features soak`. Within a soak boot it stays inert until [`arm_tick_routes`] runs, which is
/// after every worker has been spawned, so the ticks that land during setup are not counted against
/// a waiter that does not exist yet.
///
/// # One route per group, round-robin, and it is a fix rather than a flourish
///
/// The first version signalled **one** rendezvous that every waiter blocked on, and it failed a run
/// on a loaded host: three of four waiters made no progress for a whole beat and the soak reported
/// them as wedged. The cause is in `crates/ipc`'s `Rendezvous`, and it is correct behaviour there:
/// `recv` takes a **pending** signal before it looks at the receiver queue, because a driver must
/// never miss an interrupt that already happened. So when ticks arrive in a burst, which is what a
/// guest on a busy host sees, whichever waiter is already running drains the whole backlog through
/// the `pending` path and never queues, while its peers sit at the head of a queue nothing pops.
/// One waiter is fed and the rest starve.
///
/// A rendezvous per group removes the shared resource, so a backlog can only ever belong to the
/// waiter it accumulated for. The cursor is global rather than derived from `cpu::id()` so that
/// every route is covered whatever the topology: [`topology_groups`] builds at least two groups even
/// on a single-core machine, where a scheme keyed on the core would leave the second waiter to
/// starve for the whole run.
///
/// Name provisional (milestone 221): calef names public items.
pub fn signal_waiters() {
    let groups = TICK_GROUPS.load(Ordering::Acquire);
    if groups == 0 {
        return;
    }
    // Relaxed: all this counter has to do is differ from its neighbours over time. It may skip or
    // repeat a number under contention without anything being wrong, because the routes are
    // equivalent and the fairness it buys is statistical rather than exact.
    let which = TICK_CURSOR.fetch_add(1, Ordering::Relaxed) % groups;
    match TICK_ROUTES[which].load(Ordering::Relaxed) {
        0 => {}
        route => sched::irq_notify(route - 1),
    }
}

/// Bind one tick route per group. `false` if any of the interrupt numbers is already spoken for,
/// which is a refusal to start rather than a thing to work around.
///
/// Every number is checked **before** any of them is bound, so a refusal leaves the tree as it found
/// it rather than half-routed.
///
/// **Called before the first waiter is spawned, and the order is a bug fix.** It was called after,
/// so that ticks would not accumulate against waiters that did not exist yet, and that reasoning was
/// right about the backlog and wrong about the race: a waiter that reached `Irq::WAIT` before its
/// route existed got `WrongObject`, and a refused waiter has nowhere to report to, so it stopped
/// counting and the run failed one beat later with four workers "wedged". Binding first closes the
/// window entirely, because [`TICK_GROUPS`] is what actually starts the signalling and that is still
/// stored last. The backlog this used to avoid is now bounded and harmless: each route is its own,
/// its waiter drains it at once, and the first beat absorbs it.
fn bind_tick_routes(endpoints: &[sched::RendezvousId]) -> bool {
    if endpoints.is_empty() || endpoints.len() > MAX_GROUPS {
        return false;
    }
    if (0..endpoints.len()).any(|g| sched::irq_route(tick_intid(g)).is_some()) {
        return false;
    }
    for (g, &ep) in endpoints.iter().enumerate() {
        sched::bind_irq(tick_intid(g), ep);
        TICK_ROUTES[g].store(ep + 1, Ordering::Relaxed);
    }
    true
}

/// Start the signalling: from here on, every tick on every core signals a route.
///
/// Release against [`signal_waiters`]'s acquire, and it is the edge that matters: a core that reads
/// a nonzero group count must also see every route [`bind_tick_routes`] stored and every `bind_irq`
/// behind them, or a tick could signal a slot that is still empty.
fn arm_tick_hook(groups: usize) {
    TICK_GROUPS.store(groups, Ordering::Release);
}

/// How many groups to build on a machine with `cores` cores.
///
/// One group per core, and at least two so a single-core QEMU leg still runs something with more
/// than one endpoint in it. Capped by what the shared page has slots for.
fn topology_groups(cores: usize) -> usize {
    let groups = cores.max(2);
    groups.min(MAX_WORKERS / MEMBERS_PER_GROUP)
}

/// Which slot of the shared page belongs to this group's `member`.
///
/// Members are laid out consecutively, member 0 being the group's responder, so a stalled index in
/// a report names its group by arithmetic a reader can do in their head.
fn worker_index(group: usize, member: usize) -> usize {
    group * MEMBERS_PER_GROUP + member
}

/// Read one `u64` out of the shared page through the direct map.
fn read_counter(shared: u64, offset: u64) -> u64 {
    // SAFETY: `shared` is a frame this module allocated and never freed, the direct map names it,
    // and `offset` comes from `soak_page`, whose host tests prove every offset it produces lands
    // inside one page.
    unsafe { core::ptr::read_volatile((arch::mmu::phys_to_virt(shared) + offset) as *const u64) }
}

/// Beat, sample, judge; forever, or until something is wrong.
///
/// `tids` and `placed` are milestone 240's: the thread and the core each worker was given at spawn,
/// so that every beat can say whether the census printed at the top of the log is still true.
fn watch(shared: u64, workers: usize, tids: &[u64; MAX_WORKERS], placed: &[u8; MAX_WORKERS]) -> ! {
    let hz = arch::timer::frequency();
    let started = arch::timer::now();
    let mut previous = [0u64; MAX_WORKERS];
    let mut here = [u8::MAX; MAX_WORKERS];
    // The arrangement the log currently claims, which starts as the spawn placement `run` printed.
    let mut census = *placed;
    let mut last_total = 0u64;
    let mut last_wakes = 0u64;
    let mut last_at = started;
    let mut beat = 0u64;

    loop {
        // Yield until the beat is due. `yield_now` rather than a spin: this thread is one more
        // contender for the machine, and one that gives its core back between checks contends
        // usefully instead of stealing a core from the workload.
        let due = arch::timer::now().wrapping_add(hz.wrapping_mul(BEAT_SECONDS));
        while arch::timer::now().wrapping_sub(due) > u64::MAX / 2 {
            sched::yield_now();
        }

        beat += 1;
        let now = arch::timer::now();
        let elapsed = now.wrapping_sub(started) / hz.max(1);
        let window = (now.wrapping_sub(last_at) / hz.max(1)).max(1);

        let mut total = 0u64;
        let mut wakes = 0u64;
        let mut mismatches = 0u64;
        let mut stalled = 0usize;
        let mut first_stalled = usize::MAX;
        for (i, last) in previous.iter_mut().take(workers).enumerate() {
            let rounds = read_counter(shared, soak_page::rounds(i));
            let woken = read_counter(shared, soak_page::wakes(i));
            mismatches = mismatches.wrapping_add(read_counter(shared, soak_page::mismatches(i)));
            total = total.wrapping_add(rounds);
            wakes = wakes.wrapping_add(woken);
            // **One question for every role** (milestone 221): did *either* of this worker's two
            // progress counters move? A waiter leaves `rounds` at zero forever and the other three
            // roles leave `wakes` at zero forever, so summing them is a progress measure that needs
            // no table of who plays which part, and cannot be fooled: neither counter ever goes
            // down, so a sum that is unchanged means both terms were.
            let progress = rounds.wrapping_add(woken);
            if progress == *last {
                stalled += 1;
                if first_stalled == usize::MAX {
                    first_stalled = i;
                }
            }
            *last = progress;
        }

        // **Is the census the log last printed still true?** (milestone 240.) One lock acquisition
        // and `workers` comparisons once every [`BEAT_SECONDS`], which is what lets a census be
        // printed on a change instead of on every beat: while this reads zero the reader knows the
        // block above them describes the machine right now, and they are told rather than trusting.
        //
        // **Measured against the last census rather than against spawn**, and the difference is the
        // whole value. Against spawn it becomes a constant a few beats in (11 of 20 on the aarch64
        // QEMU leg) and then says nothing further; against the census it returns to zero after each
        // reprint, so a nonzero one always means "there is a newer block below".
        //
        // Counted over the roles whose movement is news. A tick waiter migrating is milestone 221's
        // whole point and is already `crossings=`, so folding it in would make this rise on a
        // healthy run and mean nothing. A thread reading `u8::MAX` has not run yet and has no
        // second core to have gone to, which is not drift either.
        sched::last_cpus(&tids[..workers], &mut here[..workers]);
        let drifted = (0..workers)
            .filter(|&i| {
                role_of(i % MEMBERS_PER_GROUP) != ROLE_WAITER
                    && here[i] != u8::MAX
                    && here[i] != census[i]
            })
            .count();

        let refused = sched::wake_refusals();
        let rate = total.wrapping_sub(last_total) / window;
        let wake_rate = wakes.wrapping_sub(last_wakes) / window;
        // One line, one beat, every field named. A log a person greps six weeks later is worth more
        // than a table that lines up on a terminal nobody kept.
        println!(
            "soak: t={elapsed}s beat={beat} rounds={total} rate={rate}/s wakes={wakes} \
             wakerate={wake_rate}/s workers={workers} refused={refused} mismatch={mismatches} \
             stalled={stalled} drifted={drifted} crossings={} remote={} steals={} deferred={}",
            sched::migrations(),
            sched::remote_placements(),
            sched::steals_served(),
            sched::wakes_deferred(),
        );
        last_total = total;
        last_wakes = wakes;
        last_at = now;

        // **Re-census when the answer changed, and only then** (milestone 240). A block every beat
        // would double an already dense log; a block never would leave the start census standing
        // after it stopped being true, which the aarch64 QEMU leg showed happens inside the first
        // five seconds. Printing on the change carries a current census whenever one exists and is
        // quiet the rest of the time, and `drifted=` is what makes the quiet readable.
        if drifted != 0 {
            print_census(
                "where the workers are NOW, the arrangement above having changed",
                workers / MEMBERS_PER_GROUP,
                &here[..workers],
            );
            census = here;
        }

        // The three verdicts, in the order a reader would want them: the one that is a finding
        // about the kernel, then the one that is a finding about delivery, then the one that is a
        // finding about liveness.
        //
        // The first beat is exempt from the stall check and only from that one: workers that have
        // not been scheduled yet have not stalled, they have not started.
        let verdict = if refused != 0 {
            Some(
                "the wake gate refused a wake: a waker made a parked receiver Ready with nothing \
                  delivered (sched::wake_load_aware, the boot-8 gate). This is the defect \
                  design/fatal-risks.md risk 5 is about.",
            )
        } else if mismatches != 0 {
            Some(
                "a caller got back a word that was not the answer to what it sent, or a CALL \
                  arrived without its reply capability. The IPC path delivered the wrong message.",
            )
        } else if stalled != 0 && beat > 1 {
            Some(
                "a worker made no progress for a whole beat while the machine kept running: it \
                  is wedged, or it died.",
            )
        } else {
            None
        };

        if let Some(why) = verdict {
            println!("soak: FAILED at t={elapsed}s beat={beat}: {why}");
            if first_stalled != usize::MAX {
                // Group and member, not a partner index. The old line printed `first_stalled ^ 1`
                // and called it the partner, which was arithmetic from a two-member group that no
                // longer exists; naming the group points a reader at every thread that could have
                // wedged this one, which is what they actually need.
                println!(
                    "soak: first stalled worker is {first_stalled}: group {}, member {} of \
                     {MEMBERS_PER_GROUP} (member 0 is the responder, then {CALLERS_PER_GROUP} \
                     callers, then {GRINDERS_PER_GROUP} grinder, then the tick waiter)",
                    first_stalled / MEMBERS_PER_GROUP,
                    first_stalled % MEMBERS_PER_GROUP,
                );
            }
            // The census before the dump, because a failure is the one moment a reader has the
            // whole log in front of them and wants to know the shape of the machine that produced
            // it. It costs four lines once, on a run that is ending anyway.
            print_census(
                "where the workers were when this run failed",
                workers / MEMBERS_PER_GROUP,
                &here[..workers],
            );
            // The thread dump before the panic, because the panic handler does not print one and
            // the per-core event rings are the whole reason they exist: a soak failure with no path
            // into the wedge is the end state alone, which is the thing first-silicon diagnostics
            // were built because nobody could read.
            sched::dump_threads();
            panic!("soak failed at t={elapsed}s beat={beat}: {why}");
        }

        // Keep the boot-stage breadcrumb honest for anyone reading a dump: the tour is over and the
        // soak is what is running.
        let _ = crate::arch::exceptions::SVC_COUNT.load(Ordering::Relaxed);
    }
}
