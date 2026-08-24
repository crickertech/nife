//! **The login service: authentication produces capabilities** (milestone 49,
//! design/roadmap/49-users-and-attribution.md, DECISIONS §109).
//!
//! Unix login authenticates and then mutates a global identity field, which is uid's whole trick
//! and the thing this system refuses to have. This process authenticates a presented identity
//! against the credential service milestone 56 already built, and on success hands back **a
//! capability set** instead: a fresh directory, a fresh budget, a **logout ticket** (see "Reclaiming
//! a session" below), and (see this program's BUGS) not yet a terminal. It is the powerbox pattern
//! with the human at one end, answering the question milestone 49's own doc named and left open: who
//! gets which capabilities at startup, which used to be a fact baked into
//! `crates/system_initializer` at build time and is, for the one login path this program serves, a
//! fact decided here at run time instead.
//!
//! # What "produces capabilities" means, concretely
//!
//! A successful login does not narrow a capability this process already holds and hand a copy on
//! (that would make every principal a viewer of the same underlying object, which is the shared-
//! endpoint anti-pattern DECISIONS §109 names and rejects three times over: the compositor, the FS
//! server's handle table, the fault endpoint). It **builds a fresh `fs_subtree_caretaker`**, the
//! same construction `crates/system_initializer` performs for a directory-granted spawn, out of a
//! region split off this process's own construction budget. Two different successful logins are
//! therefore two different endpoint *objects*: distinguishable, independently revocable, and each
//! nameable only by the principal that established it, which is the channel-shaped attribution
//! DECISIONS §109 decided on.
//!
//! # Which subtree a principal gets (see BUGS for the rest)
//!
//! **Each identity is attenuated to its own subtree, named by the identity string itself, used
//! directly** (DECISIONS §117, 2026-08-23: no separate lookup table). `chris` and `corinne` land in
//! two different, independently-scoped subtrees; neither can name the other's, and neither is the
//! old shared fixture subtree earlier slices of this program used for everyone. The subtree must
//! already exist: this program never creates one (`identity_provisioner`, milestone 155, does that
//! at provisioning time, deliberately not auto-vivified here; see that decision's own reasoning for
//! why provision-time creation was chosen over creating it on a principal's first login). An
//! authenticated identity with no provisioned subtree is refused, folded into the same
//! [`login_proto::DENIED`] a wrong password gets (see this program's BUGS on why). Per-principal
//! subtree *scoping* (which subtree a grant may name at all, as opposed to *which one this program
//! picks*) is milestone 47's already-built mechanism (`fs_subtree_caretaker`'s whole reason to
//! exist); this program only decides the name.
//!
//! # Reclaiming a session, resolved 2026-08-23
//!
//! This program's BUGS used to name two candidate shapes for giving back a caretaker's construction
//! memory and pick neither: a principal's supervision endpoint reaching this process, or a caretaker
//! `Untyped::DESTROY`ed by name. Investigating both against this tree's own precedent found a third,
//! smaller than either, and it is what [`mint`] now builds.
//!
//! **The fourth delegated capability is [`mint`]'s own `region`, undropped.** A successful login
//! used to end with this process calling `cap_delete` on its own copy of the caretaker's
//! construction region the moment the caretaker confirmed descent (see the cspace-ceiling fix this
//! BUGS section used to describe below `mint`'s own comment). That capability is not discarded
//! anymore: it is delegated to the authenticated client, narrowed to `WRITE` (the one right
//! `Untyped::DESTROY` needs, per `abi::untyped::DESTROY`'s own doc), the same "delegate, then drop
//! our own copy" pattern already used for the directory and the budget. The client now holds its own
//! **logout ticket**: an `Untyped` capability with nothing left to `SPLIT` or `RETYPE` (the region's
//! whole budget was spent building the caretaker), whose only remaining use is `DESTROY`. Calling it
//! reclaims the caretaker's TCB, address space, and endpoints, and the pages come home to
//! [`CONSTRUCTION_UT`] under §13 region ownership (the region's builder, not its destroyer), exactly
//! the outcome the BUGS section asked for and none of the two originally-named options were small
//! enough to build outright.
//!
//! **Why this needed no session and no new supervision plumbing.** The candidate this process never
//! supervises its caretaker at all (`mint` calls `cap_delete` on the caretaker's own TCB the instant
//! it starts, and sets no fault endpoint), which milestone 152's own doc names as the gap DECISIONS
//! §92 left open ("this says nothing about a caretaker with no client, because none exists yet").
//! Building a supervision endpoint that reaches back into this process to ask for a specific
//! principal's teardown is exactly the durable-session machinery 152 is scoped to design in general;
//! this slice does not need it, because the caretaker's own construction region is the only thing
//! that ever needs tearing down, and its builder (this process, transiently, for the width of one
//! `mint` call) can hand the means to do that directly to the one party who should hold it, the
//! client, without keeping anything itself.
//!
//! **Why `Untyped::DESTROY` actually works here, checked against §32's own documented gap rather
//! than assumed.** A supervisor's `Endpoint::REAP` only collects an *already-dead* thread (§32:
//! "it authorizes collecting a corpse, not killing"); killing a *live* one needs `Untyped::DESTROY`'s
//! stronger right, and that refuses permanently against a thread `Blocked` on an endpoint outside the
//! region being destroyed (notes/hung-component.md's case (c), the open, unsolved half of the hung-
//! component taxonomy). The caretaker built by [`mint`] is never in that shape: its own client-facing
//! endpoint (`narrow_ep`, the fourth capability's sibling) is retyped directly from `region`, so the
//! caretaker's steady state (parked in `recv_cap` between requests) is case (b), "blocked on an
//! endpoint whose region the supervisor can destroy", which `notes/hung-component.md` already
//! documents as working, with collateral: destroying `region` drains `narrow_ep`'s wait queue,
//! aborts the caretaker's blocked receive, and the armed kill lands at the caretaker's next
//! scheduling. The one narrow exception is the instant the caretaker is mid-`forward` to the file
//! service (a `CALL` on `FS_EP`, which `region` does not own): a `DESTROY` attempted in that exact
//! window is refused, transiently, the same shape `crates/system_initializer::reclaim` already
//! retries for a directory grant's own caretaker. A client is expected to retry a bounded few times
//! on `NotPermitted`, the same idiom, rather than treat one refusal as final; see
//! `user/src/login_test_client.rs`'s own teardown role for a worked example.
//!
//! **This is why `mint`'s failed-descent path no longer leaks either.** The same `region` this
//! function used to abandon on a refused descent (the caretaker had already `exit()`ed, so nothing
//! was running in it) is now reclaimed right there, with the same bounded retry, before this
//! function returns `None`. That removes the one case this program's own BUGS used to note as
//! unaffected by the cspace fix.
//!
//! **A full logout needs no fifth capability, because the third one already carried enough right.**
//! [`CLIENT_BUDGET_PAGES`] is delegated with `WRITE | GRANT` (every principal's own spending money),
//! and `WRITE` is the one right `Untyped::DESTROY` needs. Nothing before this fix had a reason to
//! call it, so this program's BUGS never named it, but any client holding `budget` could always
//! reclaim it the same way the logout ticket reclaims `region`. `user/src/login_test_client.rs`'s
//! `ROLE_LOGOUT` does both, so a full logout gives back everything a session spent:
//! [`CARETAKER_REGION_PAGES`] through the fourth capability and [`CLIENT_BUDGET_PAGES`] through the
//! third, both returning to [`CONSTRUCTION_UT`].
//!
//! **The order the two are destroyed in is load-bearing, and getting it wrong does not fail loudly.**
//! `mint` splits `region` first and `budget` second, both off [`CONSTRUCTION_UT`], so `budget` sits
//! at the top of its watermark. `crates/regions`' own LIFO reclaim (the same rule §16's object
//! revocation and `job_undertaker`'s pool already live under, and the one DECISIONS §92 already named
//! for a caretaker's own region) only returns a freed child's pages to reusable capacity when it is
//! the current top; destroying `region` while `budget` is still alive still tears down the caretaker
//! correctly (`DESTROY` returns success either way) but strands `region`'s pages until
//! `CONSTRUCTION_UT` itself goes away. **This was found, not merely reasoned about**: the first
//! version of this fix's own test destroyed them in the wrong order, every one of its own assertions
//! passed, and it silently starved a later, unrelated test in the same suite of real login attempts
//! by leaving thirteen logins' worth of stranded pages behind (see
//! `kernel::user::login_tests::caretaker_teardown_reclaims_a_full_session_worth_of_memory`'s own doc
//! comment). The fix is ordering, not a capability change: destroy `budget` (the third capability)
//! before `region` (the fourth). See `crates/login_proto`'s own module docs for the client-facing
//! version of this note, including why it holds regardless of what other clients do (nothing else is
//! ever split from `CONSTRUCTION_UT` between one login's two capabilities) but does not generalize to
//! reclaiming two different logins' memory out of the order they were minted in.
//!
//! # Capability contract
//!
//! - slot [`REQUEST`]: `RECV`. A client sends one [`login_proto::LOGIN`] request here, identity and
//!   secret staged at [`LOGIN_VA`] the way [`login_proto::place`] writes them.
//! - slot [`RESULT`]: `WRITE | GRANT`. The verdict, and on [`login_proto::OK`] four delegated
//!   capabilities, in the order `login_proto`'s module docs give.
//! - slot [`VERIFY`]: `WRITE` on the credential service's verify endpoint (milestone 56). This
//!   process never provisions it and never could: the provision endpoint is deleted at both ends
//!   before any client of the credential service exists (`user/src/credentialer.rs`).
//! - slot [`FS_EP`]: `WRITE | GRANT` on the file service's root directory capability. What every
//!   minted caretaker attenuates.
//! - slot [`FS_FRAME`]: a `Frame`, `READ | WRITE`, the page the file service shares with its
//!   clients. Delegated on to each authenticated principal (see the module docs on why one frame
//!   serves every hop).
//! - slot [`CONSTRUCTION_UT`]: `WRITE | GRANT`. Everything a caretaker and a client budget are
//!   built from. Never given away, unlike `root_supervisor`'s: this process keeps serving logins
//!   for its whole life, so unlike an init that hands its authority away once, it must keep some.
//! - slot [`AUDIT`]: `WRITE`. One [`login_proto::ATTRIBUTED`] message per successful login, so the
//!   property DECISIONS §109 names (a server logging which channel it just established, and for
//!   whom) is checkable rather than merely claimed. See this program's BUGS on the scope of what
//!   this endpoint proves.
//! - mapped [`LOGIN_VA`]: the page shared with whichever client is calling right now.
//! - mapped [`CRED_VA`]: the page shared with the credential service, for the relayed `VERIFY`.
//! - mapped [`INITRD_VA`]: the archive, read-only, so this process can find `fs_subtree_caretaker`'s
//!   own bytes and [`measured_boot::PROGRAM_MEASUREMENTS`], the exact way `crates/system_initializer`
//!   does, and verify the first against the second before ever building a caretaker from it.
//!
//! Name: unrecorded. Provisional, minted 2026-08-22 for milestone 49 and not yet put to calef.
//! `login` is the plain noun for what this program answers a request to do, on the pattern
//! `clock`/`entropy`/`credentialer` already set, which is the reasoning a ratification would test.
//!
//! # BUGS
//!
//! **One client at a time, structurally.** [`REQUEST`] and [`RESULT`] are each a single endpoint, so
//! two concurrent callers would interleave their words on one page, exactly the limit
//! `credentialer.rs` already documents for its own verify page and for the same reason: this
//! process has one thread and no wait-any primitive, so it cannot serve two rendezvous at once.
//! `filesystem_proto`'s answer (a channel per client) is the shape to copy when a second concurrent caller
//! exists; today this program's only callers are `login_test_client` roles the test suite runs one
//! at a time.
//!
//! **No terminal.** The roadmap's own text names three things a login hands back: a root directory,
//! a budget, a terminal. This program hands back the first two. A terminal in this system is a
//! singleton hardware-backed resource wired once at interactive boot
//! (`crates/system_initializer::boot`); minting a second one, or multiplexing the one that exists
//! across logins, is real work this slice does not attempt and does not want to guess the shape of.
//! It is unscoped follow-on, not an oversight.
//!
//! **Resolved, 2026-08-23 (DECISIONS §117).** Every successful login used to be attenuated to the
//! same fixed subtree, with the same rights, for every identity. It is now attenuated to a subtree
//! named by the identity string itself; see the module docs above for what that does and does not
//! cover, and the two bounds this brought with it, named honestly rather than left implicit:
//!
//! - **An identity longer than [`filesystem_proto::grant::MAX_NAME`] (16 bytes) cannot get a per-identity
//!   subtree in this slice at all**, even though [`login_proto::MAX_IDENTITY`] (64 bytes) would
//!   otherwise accept it. The grant name travels in two `START` argument words to the caretaker,
//!   not a frame (`filesystem_proto::grant`'s own doc explains why: a per-file or per-subtree grant this way
//!   costs no extra page and no extra mapping), and that encoding is the 16-byte one, not
//!   `login_proto`'s wider one. `mint` refuses (folded into [`login_proto::DENIED`], next bullet)
//!   rather than silently truncating the name `filesystem_proto::grant::pack_name` would otherwise produce,
//!   which would attenuate the caretaker to a *different* subtree than the one
//!   `identity_provisioner` created (a name collision hazard, not merely a usability one: two
//!   identities that agree on their first 16 bytes would silently share a subtree). Lifting this
//!   bound to `login_proto`'s own 64 means giving the caretaker a frame for its grant instead of two
//!   argument words, which is a change to `filesystem_proto::grant`'s contract and every caretaker built
//!   against it, not a one-line fix here.
//! - **An authenticated identity with no provisioned subtree is refused, indistinguishably from a
//!   wrong password.** `mint`'s caretaker construction reaches the same `OPENDIR`-against-a-missing-
//!   name refusal an unprovisioned identity's descent gets, and this program's existing fold (below,
//!   "an otherwise-authenticated principal") already answers it with [`login_proto::DENIED`] rather
//!   than a distinguishable code. **This is a considered answer, not the accident of reusing the
//!   fold**: the same reasoning [`login_proto::DENIED`]'s own doc gives for a wrong password applies
//!   just as much here (a caller must not be able to tell "your identity has no home" from "your
//!   password is wrong" by comparing outcomes across attempts, which would let a caller probe which
//!   identities are provisioned without ever presenting a right password for one). The honest cost is
//!   that an operator who forgot to run `identity_provisioner` for a real identity sees the same
//!   denial a typo would produce; the audit trail (see below) does not help here either, since it
//!   only records a *successful* login. Distinguishing the two would need a new, deliberately-weaker
//!   channel than the login result itself (an operator-facing log the login result is not), which is
//!   real work this slice does not build.
//!
//! **Not wired into the interactive boot, and the blocker is a missing device grant, not a wiring
//! exercise** (investigated 2026-08-23, milestone/49-login-boot-prompt). This process is spawned
//! directly by the kernel's guest test harness (`kernel/src/user/login_service.rs`), the same way
//! `credentialer` is, and is not reachable from `crates/system_initializer::boot`'s real prompt.
//! The chain that is missing: `login` needs `credentialer`'s `VERIFY`; `credentialer` refuses to
//! start without the entropy service (it draws its decoy record's salt from entropy at start-up
//! and will not fall back to a predictable one, DECISIONS §42); the entropy service needs a
//! `Virtio` capability onto a real virtio-rng device (`user/src/entropy.rs`'s slot 2); and neither
//! `kernel::user::spawn_init` (aarch64) nor `kernel::user::riscv_shell_boot` (riscv64), the two
//! sites that build a [`system_initializer::BootEndowment`] (`user/src/hello.rs`'s
//! `init_boot`, `user/src/system_initializer.rs`), discovers or grants one. Nor does the interactive
//! boot's own QEMU invocation attach a virtio-rng device at all: `NIFE_RNG` (like `NIFE_GPU`,
//! `NIFE_KBD`, `NIFE_NVME`) is set only inside `cargo xtask test`, never by `cargo xtask shell-check`
//! or any interactive/demo boot, which is a deliberate, existing pattern (a minimal device surface
//! for the boot a person actually meets) and not an oversight this file's own code could fix.
//!
//! **What is already proven, so the remaining gap is precisely this and nothing more**: the whole
//! chain (a real virtio-rng-backed entropy service, `credentialer` provisioned with milestone 56's
//! own family fixture, and this program relaying to it) already runs and is tested end to end
//! whenever a virtio-rng device is present (`kernel::user::login_tests::wired`, which reuses
//! `kernel::user::credential_tests::provisioned`). No code in this program, `credentialer.rs`, or
//! `entropy.rs` needs to change for interactive-boot reachability. What is missing is a capability
//! the kernel does not yet construct, on a boot whose device set does not yet include the device
//! that capability would name.
//!
//! **Why this is not this lane's decision to make.** Whether the interactive boot should carry a
//! virtio-rng device at all is a question about what that boot models, not a plumbing gap: milestone
//! 55's actual target is real Raspberry-Pi-class hardware (`design/roadmap/55-*`), which has no
//! virtio-rng and would need a different entropy source entirely, so "attach one in QEMU" answers
//! the demonstrator's boot and not the target's. It also touches `BootEndowment`, which is what the
//! kernel promises an init at spawn, on a cspace this file's own comments already call "one slot
//! from the wall" at peak (`crates/system_initializer::DIR_JOB_REGION_PAGES`'s neighboring comment).
//! Getting either wrong costs a design decision to unwind, not a line of code, which is the "move
//! fast on what can be undone" tenet's own test for when a fork is calef's rather than a lane's.
//! Replacing the shell's own build-time endowment with a real login prompt remains the multi-lane
//! remainder of this milestone, and its first lane is now this decision rather than a
//! `build_child` exercise.
//!
//! **Resolved, 2026-08-24.** This process used to load `fs_subtree_caretaker` by name with no check
//! at all, inconsistent with `crates/system_initializer`'s own discipline (milestone 104: refuse a
//! program whose bytes do not match the archive's measurement table). Investigating "how a non-init
//! loader joins that chain" (the open question this BUGS entry used to leave unanswered) found the
//! premise did not hold: this process maps the *same physical archive* the kernel already maps for
//! `system_initializer`, the same read-only way, at the same address (`kernel::user::spawn_init` for
//! aarch64's init, `kernel::user::login_service`'s own `start` for this process, both taking the
//! physical range from `memory::initrd_region()`), so the kernel's boot already vouches for this
//! process's copy exactly as much as it vouches for init's. There is no new trust boundary to cross:
//! `_start` now reads
//! [`measured_boot::PROGRAM_MEASUREMENTS`] out of that same archive and calls
//! [`measured_boot::verify_in_manifest`] against `fs_subtree_caretaker`'s bytes, the identical
//! function `system_initializer::measured` calls for its own six components.
//!
//! **Not folded into a boot failure.** `system_initializer` treats this exact program as optional
//! (its own doc: an unvouched `fs_subtree_caretaker` "costs `rm` and nothing else"), and this process
//! mirrors that instead of refusing to start: the check runs once, at `_start`, before any client
//! exists, and on refusal `care_elf` becomes `None` rather than a call to [`fail`]. [`mint`] then
//! returns `None` immediately for every future login (`care?`, its very first line), which its
//! caller already folds into [`login_proto::DENIED`], the same code "the construction budget is
//! spent" and "the caretaker's descent was refused" already share.
//!
//! **This fold is not the anti-oracle reasoning the other two folded cases get, and should not be
//! read as one.** A wrong password and a missing subtree both vary with what a caller presents, so
//! folding them prevents a caller from learning something about a specific identity by comparing
//! outcomes across attempts. A failed caretaker measurement varies with *nothing* a caller controls:
//! the archive is immutable RAM fixed for the whole boot, so every identity, on every attempt, for
//! the rest of this process's life, gets the identical answer. There is nothing to probe. The honest
//! reason for the fold is narrower and more mundane: [`login_proto`] has no separate wire code for
//! "this service's core dependency failed to verify," and DENIED's own doc already covers "the
//! service could not mint a capability set for an otherwise-authenticated principal," which this is
//! one more instance of. The cost this leaves unaddressed is operational rather than a security gap:
//! an operator whose build produced a tampered or unmeasured `fs_subtree_caretaker` sees every real
//! login denied with no signal that the *cause* is the caretaker rather than, say, a misconfigured
//! credential store, and the audit endpoint does not help (it only records a *successful* login, the
//! same limitation already named below for the no-subtree case). A deployment that wants that
//! distinguished needs an operator-facing log distinct from the login result, which this slice does
//! not build.
//!
//! Proven by
//! `kernel::user::login_tests::logins_caretaker_measurement_matches_the_real_table_and_a_tampered_one_would_be_refused`:
//! the real archive's `fs_subtree_caretaker` bytes verify against the real measurement table (so
//! `wired()`'s own instance, and every other test in that file, depends on this check passing), and
//! the identical [`measured_boot::verify_in_manifest`] call `_start` now makes refuses a tampered
//! copy and a name the table does not mention. That test cannot spawn a second login instance against
//! a deliberately corrupted archive to prove the wire-level `DENIED` end to end: the initrd is one
//! physical region the whole kernel test binary shares, set up once at boot, and `cargo xtask` always
//! packs a table that agrees with the bytes it just packed, so no test in this suite can make the real
//! archive disagree with itself. Proving the exact check `_start` performs, against the exact name and
//! table it uses, is the strongest proof available without a second, deliberately-tampered kernel
//! image, which is out of scope for this fix.
//!
//! **Resolved, 2026-08-23.** A caretaker's construction memory used to never come back: every
//! successful login spent [`CARETAKER_REGION_PAGES`] and [`CLIENT_BUDGET_PAGES`] out of
//! [`CONSTRUCTION_UT`] for the rest of this process's life, with no logout that gave the memory
//! back. `mint` now returns its own copy of the caretaker's construction region as a fourth
//! delegated capability (narrowed to `WRITE`, the one right `Untyped::DESTROY` needs) instead of
//! dropping it, and the authenticated client holds it as its own logout ticket: an `Untyped` with
//! nothing left to `SPLIT` or `RETYPE` (the region's whole budget already went into building the
//! caretaker), whose only remaining use is `DESTROY`. Calling it reclaims the caretaker's TCB,
//! address space and endpoints, and the pages come home to [`CONSTRUCTION_UT`] under §13 region
//! ownership (the region's builder, not its destroyer). See this program's module docs, "Reclaiming
//! a session", for the two candidate shapes this replaces (a supervision endpoint reaching this
//! process, or a caretaker `DESTROY`ed by name) and why the second, refined this way, needed neither
//! a new supervision mechanism nor overlap with milestone 152's durable-session scope.
//!
//! **The client's own budget is the other half, and needed no new capability at all**: it was
//! always delegated with `WRITE | GRANT`, and `WRITE` is what `DESTROY` needs, so a client that
//! wants to give back everything a session spent (both [`CARETAKER_REGION_PAGES`] and
//! [`CLIENT_BUDGET_PAGES`]) calls `DESTROY` on both the fourth capability and the third, **in that
//! order** (budget, then region): the module docs, "Reclaiming a session", explain why the order is
//! load-bearing rather than a preference (a `crates/regions` LIFO rule, the same one DECISIONS §92
//! already names) and record that this was caught empirically, not merely reasoned about.
//!
//! [`CONSTRUCTION_UT`] is still sized by whoever spawns this process, and running out (a client that
//! never logs out) still answers every further login with [`login_proto::DENIED`] rather than a
//! distinguishable error, for `login_proto`'s own stated reason (a caller must not learn "the
//! service is out of resources" by comparing outcomes across two attempts with the same identity);
//! a deployment that wants that not to happen relies on clients actually calling `DESTROY`, which
//! this program cannot compel and does not police (see "one client at a time" above: policing would
//! need to know when a client is genuinely done, which is exactly the session concept this fix
//! avoided building).
//!
//! **The cspace ceiling this shares history with is unaffected and stays fixed.** This process's own
//! capability table has sixteen slots (`kernel::cap::CSPACE_SLOTS`) and eight are spent at rest;
//! `mint` used to leak one of the remaining eight per successful login by keeping `region`'s
//! capability past a confirmed descent, which left room for exactly eight logins ever before the
//! cspace itself (not `CONSTRUCTION_UT`) answered every further attempt with `DENIED`. That was
//! fixed separately, by dropping `mint`'s own copy of `region` once the caretaker confirmed descent
//! (a `cap_delete`, not a `DESTROY`). This slice's fix keeps that shape: `region` is delegated and
//! then this process's own copy is deleted, the same "delegate, then drop our own copy" pattern
//! already used for the directory and the budget, so a live login costs this process's cspace
//! nothing beyond the width of one `mint` call, regardless of how many clients are logged in at
//! once. See `kernel::user::login_tests::the_login_service_serves_past_the_old_cspace_ceiling`.
//!
//! **The audit endpoint proves establishment, not per-request attribution.** [`login_proto::ATTRIBUTED`]
//! records which identity established which channel at the moment this process minted it. It does
//! not prove that a *downstream* server, later, can say which channel one of its own requests
//! arrived on; DECISIONS §109's own text describes both halves and this program is only the first.
//! No server in this tree today needs the second: `fs_subtree_caretaker` already serves exactly one
//! principal by construction (there is nothing to distinguish), and the credential service is
//! anonymous by design (DECISIONS §109 predates this and neither wants nor needs to know who is
//! asking). Wiring the second half into a real multi-tenant consumer is follow-on for whenever such
//! a consumer exists.

#![no_std]
// Program entry points, not the crates/ library surface milestone 68's ratchet tracks
// (DECISIONS §107): each `[[bin]]` is its own crate root with one `_start`, and 58 of them
// documenting an OS-facing ABI entry point is not what the lint is for.
#![allow(missing_docs)]
#![no_main]

use supervision_proto::{
    ChildEndowment, build_child, retype_obj_from as retype_obj, tcb_start, untyped_destroy,
    untyped_split,
};
use user_rt::{call, cap_delete, invoke, recv, send, yield_now};

/// A client's login request, `RECV` (milestone 49).
const REQUEST: u64 = 0;
/// The verdict and, on success, four delegated capabilities, `WRITE | GRANT`.
const RESULT: u64 = 1;
/// The credential service's verify endpoint (milestone 56), `WRITE`.
const VERIFY: u64 = 2;
/// The file service's root directory capability, `WRITE | GRANT`.
const FS_EP: u64 = 3;
/// The file service's shared page, a `Frame`, `READ | WRITE`.
const FS_FRAME: u64 = 4;
/// Everything a caretaker and a client budget are built from, `WRITE | GRANT`.
const CONSTRUCTION_UT: u64 = 5;
/// One [`login_proto::ATTRIBUTED`] message per successful login, `WRITE`.
const AUDIT: u64 = 6;

/// The page shared with the current client. Distinct from `credentialer.rs`'s own request pages
/// (a different process, so no collision is possible), but numbered in the same family so a reader
/// who knows one contract's addresses recognises the shape of the other's.
const LOGIN_VA: u64 = 0x0000_0000_00e2_0000;
/// The page shared with the credential service, for the relayed `VERIFY`.
const CRED_VA: u64 = 0x0000_0000_00e3_0000;
/// Where the archive is mapped read-only. Must match `kernel::user::INITRD_VA`.
const INITRD_VA: u64 = 0x2000_0000;
/// Where a built caretaker and the file service's shared page meet. Must match
/// `user/src/fs_subtree_caretaker.rs`'s `PAGE_VA` (the same address every caretaker in this tree
/// uses, since the caretaker itself hardcodes it and this process copies its ELF, not its address).
const CARETAKER_FS_VA: u64 = 0x0000_0000_0060_0000;

/// This process's own scratch mappings for `build_child` (its page tables, never a child's).
/// Small: one caretaker at a time is ever mid-construction, so this never needs to hold more than
/// one build's worth of intermediate page tables. Sized generously against `INIT_OWN_PAGES` (128
/// in `crates/system_initializer`, which builds six boot components against the same budget).
const OWN_UT_PAGES: u64 = 128;

/// One caretaker's whole construction: its address space, TCB, and stack.
/// `crates/system_initializer::DIR_JOB_REGION_PAGES` (96) covers a caretaker **and** the program
/// behind it; this process builds only the caretaker, so a smaller region should hold it, with
/// margin rather than a tight fit (a region too small fails as `Err(())` mid-login, which this
/// process can only answer with the one code `login_proto::DENIED` already carries for "could not
/// mint", see this program's BUGS).
const CARETAKER_REGION_PAGES: u64 = 64;

/// Stack pages beyond the one `build_child` maps, matching
/// `crates/system_initializer::CARETAKER_STACK_PAGES`: measured for that program rather than
/// guessed, and this is the same program.
const CARETAKER_STACK_PAGES: u64 = 4;

/// What this process hands each authenticated principal as its own budget. Arbitrary and modest,
/// for the demonstration this slice is; a real deployment sizes it against what a session actually
/// needs, which is not yet a question this program has enough callers to answer.
const CLIENT_BUDGET_PAGES: u64 = 64;

#[unsafe(no_mangle)]
pub extern "C" fn _start(_a0: u64, initrd_len: u64, _a2: u64) -> ! {
    // SAFETY: the wiring mapped `initrd_len` bytes of reserved RAM read-only at INITRD_VA.
    let archive =
        unsafe { core::slice::from_raw_parts(INITRD_VA as *const u8, initrd_len as usize) };
    let Ok(fs) = nifefs::Fs::parse(archive) else {
        fail(1)
    };
    let Some(care_bytes) = fs.read("fs_subtree_caretaker") else {
        fail(2)
    };
    let Ok(care_elf) = elf::Elf::parse(care_bytes) else {
        fail(3)
    };

    // **The table `crates/system_initializer` already consults before loading anything it did not
    // build itself** (milestone 104), read the identical way: bytes that are not UTF-8 become the
    // empty table rather than a fault (`measured_boot`'s own reasoning for why that is the safe
    // direction to fail), and an empty table vouches for nothing. Checked once, here, rather than
    // per login: the archive is immutable RAM this process shares with the whole boot, so every
    // future request would see exactly the same verdict, and there is nothing to gain by re-hashing
    // the same bytes on every request.
    //
    // **Not a boot failure**, unlike the two checks above. `crates/system_initializer` treats this
    // exact program as optional (its own doc: "without one, a directory grant cannot be delivered,
    // ... which costs `rm` and nothing else"), and this process mirrors that rather than refusing to
    // come up at all: an unvouched caretaker costs this service the one thing it exists to hand out,
    // so every login is answered `DENIED` below (via `mint`) instead. See this program's BUGS for why
    // that fold is not the same anti-oracle reasoning a wrong password gets, and is the considered
    // answer anyway.
    let table = fs
        .read(measured_boot::PROGRAM_MEASUREMENTS)
        .and_then(|b| core::str::from_utf8(b).ok())
        .unwrap_or("");
    let care_elf =
        if measured_boot::verify_in_manifest(table, "fs_subtree_caretaker", care_bytes).is_ok() {
            Some(care_elf)
        } else {
            None
        };

    let Ok(own_ut) = untyped_split(CONSTRUCTION_UT, OWN_UT_PAGES) else {
        fail(4)
    };

    // How many channels this process has established, in order. The audit trail's sequence number,
    // not a capacity: `CONSTRUCTION_UT` is what actually bounds how many logins this process can
    // serve (see BUGS).
    let mut seq: u64 = 0;

    loop {
        let (w0, _w1, _w2) = recv(REQUEST);
        // SAFETY: the wiring mapped one page read/write at LOGIN_VA before this process ran.
        let page = unsafe { core::slice::from_raw_parts(LOGIN_VA as *const u8, login_proto::PAGE) };
        let Some((identity, secret)) = login_proto::read(page, w0) else {
            wipe_login_page();
            send(RESULT, login_proto::MALFORMED, 0, 0);
            continue;
        };
        // Computed before the page is wiped: `identity` borrows LOGIN_VA and must not be read after.
        let hint = login_proto::identity_hint(identity);
        // **Copy the identity out before it is gone.** `identity` borrows LOGIN_VA, and the page is
        // wiped a few lines below (`wipe_login_page`, right after the credential relay); `mint`
        // needs the identity's own bytes to name the subtree to attenuate to (DECISIONS §117), which
        // happens after that wipe, on success. An owned, fixed-size copy (bounded by
        // `login_proto::MAX_IDENTITY`, which `login_proto::read` has already checked `identity` fits
        // within) is the only way to carry it that far without reading freed/zeroed memory.
        let mut identity_buf = [0u8; login_proto::MAX_IDENTITY];
        let identity_len = identity.len();
        identity_buf[..identity_len].copy_from_slice(identity);

        // SAFETY: the wiring mapped one page read/write at CRED_VA before this process ran, shared
        // with the credential service and with nothing else.
        let cred_page =
            unsafe { core::slice::from_raw_parts_mut(CRED_VA as *mut u8, credential_proto::PAGE) };
        let placed = credential_proto::place(
            cred_page,
            identity,
            secret,
            credential_proto::verify::VERIFY,
        );
        // The presented secret has now been copied to CRED_VA (or the placement failed and never
        // will be); either way LOGIN_VA's copy is done being read.
        wipe_login_page();
        let Some(cw0) = placed else {
            send(RESULT, login_proto::MALFORMED, 0, 0);
            continue;
        };
        let (cr0, _) = call(VERIFY, cw0, 0);
        credential_proto::wipe(cred_page);

        if !credential_proto::authenticated(cr0) {
            send(RESULT, login_proto::DENIED, 0, 0);
            continue;
        }

        match mint(own_ut, care_elf.as_ref(), &identity_buf[..identity_len]) {
            Some((dir_ep, budget, region)) => {
                send(RESULT, login_proto::OK, 0, 0);
                delegate(dir_ep, abi::rights::WRITE);
                delegate(FS_FRAME, abi::rights::READ | abi::rights::WRITE);
                delegate(budget, abi::rights::WRITE | abi::rights::GRANT);
                // The logout ticket: `WRITE` is the one right `Untyped::DESTROY` needs (this
                // program's own module docs, "Reclaiming a session"). Not `GRANT`: a client that
                // could delegate its own logout ticket onward could hand another principal the
                // means to end this one's session, which is authority narrower to withhold than to
                // grant back.
                delegate(region, abi::rights::WRITE);
                cap_delete(dir_ep);
                cap_delete(budget);
                cap_delete(region);
                send(AUDIT, login_proto::ATTRIBUTED, seq, hint);
                seq += 1;
            }
            // Authenticated, and the service still could not serve it (the construction budget is
            // spent, or the caretaker's descent was refused). Answered identically to a wrong
            // secret; see login_proto::DENIED's own doc on why that fold is deliberate rather than
            // a missed distinction.
            None => {
                send(RESULT, login_proto::DENIED, 0, 0);
            }
        }
    }
}

/// **Mint one principal's capability set**: a fresh `fs_subtree_caretaker` attenuated to
/// `identity`'s own home subtree (DECISIONS §117: the identity string, used directly, with no
/// separate lookup table), a fresh budget, and the construction region itself (the caretaker's own
/// logout ticket; see this program's module docs, "Reclaiming a session"), all three held with full
/// rights so [`delegate`] can narrow them on the way out. `None` on any failure, which this
/// process's caller answers with [`login_proto::DENIED`] (see this program's BUGS on why that is the
/// honest fold rather than a missing distinction, and on the two failures this now folds in
/// alongside "the construction budget is spent": an identity too long for the grant mechanism, and
/// an authenticated identity with no provisioned subtree).
///
/// `identity` must already name a subtree `identity_provisioner` created; this process never
/// creates one (DECISIONS §117: provision-time creation, not auto-vivified at login).
///
/// `care` is `None` when `_start`'s own measurement check refused `fs_subtree_caretaker`'s bytes
/// (this program's BUGS, "does not consult `measured_boot::PROGRAM_MEASUREMENTS`", resolved): this
/// function has nothing to build from and returns `None` immediately, the same as every other
/// reason it cannot serve a login.
fn mint(own_ut: u64, care: Option<&elf::Elf>, identity: &[u8]) -> Option<(u64, u64, u64)> {
    let care = care?;

    // **The grant name travels in two `START` argument words, not a frame** (`filesystem_proto::grant`'s own
    // doc), so it is capped at `grant::MAX_NAME` (16 bytes): smaller than `login_proto::MAX_IDENTITY`
    // (64), the bound `identity` already satisfies by construction (`login_proto::read` checked it).
    // `pack_name` does not itself refuse an oversized name; it silently stops copying at the 16th
    // byte, which would otherwise mint a caretaker attenuated to a *different, truncated* name than
    // the one `identity_provisioner` created. Refusing here, before anything is built, is what keeps
    // that silent truncation from ever happening. See this program's BUGS: identities over 16 bytes
    // cannot get a per-identity subtree in this slice at all.
    if !filesystem_proto::grant::fits(identity) {
        return None;
    }

    let region = untyped_split(CONSTRUCTION_UT, CARETAKER_REGION_PAGES).ok()?;
    let narrow_ep = retype_obj(region, abi::objtype::RENDEZVOUS).ok()?;
    let ready = retype_obj(region, abi::objtype::RENDEZVOUS).ok()?;

    let (lo, hi) = filesystem_proto::grant::pack_name(identity);
    let spec = filesystem_proto::grant::spec(identity.len(), filesystem_proto::dir::ALL);

    // Its whole authority: the file service to attenuate, the endpoint it will serve, one place to
    // say it is ready, and the frame it shares with the file service. No untyped of its own, no
    // clock, nothing that could name another process. See `crates/system_initializer::build_caretaker`,
    // which this mirrors.
    let built = build_child(
        own_ut,
        region,
        care,
        &ChildEndowment {
            caps: &[
                (FS_EP, abi::rights::WRITE),
                (narrow_ep, abi::rights::READ),
                (ready, abi::rights::WRITE),
            ],
            maps: &[(CARETAKER_FS_VA, FS_FRAME, abi::aspace::MAP_RW)],
            stack_pages: CARETAKER_STACK_PAGES,
            ..ChildEndowment::new()
        },
    );
    let tcb = built.ok()?;
    let started = tcb_start(tcb, lo, hi, spec);
    cap_delete(tcb);
    if !started {
        cap_delete(ready);
        cap_delete(narrow_ep);
        // `region` is not reclaimed here: a caretaker that failed to start left nothing running in
        // it, but this process has no `DESTROY` capability on its own construction budget's
        // children today. See BUGS.
        return None;
    }
    // The one bounded wait: the caretaker's descent against the file service, exactly the
    // handshake `crates/system_initializer::build_caretaker` performs.
    //
    // **This is also where "the credential is real but nobody ever provisioned this identity's
    // subtree" is answered**, and deliberately with no special case: `identity_provisioner` didn't
    // run, or its `MKDIR` never reached this file service's disk, so the caretaker's `OPENDIR`
    // against `identity` comes back `ENOENT` and it reports [`filesystem_proto::fixture::DESCENT_REFUSED`]
    // here instead of `READY`, which this function already turns into `None` and this program's
    // caller already folds into `login_proto::DENIED`, indistinguishable from a wrong password. See
    // this program's BUGS for why that fold is the considered answer for this case too, not merely
    // an accident of reusing the same code path.
    let (verdict, _, _) = recv(ready);
    cap_delete(ready);
    if verdict != filesystem_proto::fixture::READY {
        cap_delete(narrow_ep);
        // **`region` used to be abandoned here.** The caretaker's `OPENDIR` was refused, so it has
        // already called `exit()` (`fs_subtree_caretaker.rs`'s own descent handshake): nothing is
        // running in `region` anymore, and `narrow_ep` (retyped from it) was just deleted above, so
        // this is case (a) or a plain corpse, never case (c) (`notes/hung-component.md`'s
        // taxonomy; see this program's module docs, "Reclaiming a session", for why that
        // distinction is what makes `DESTROY` usable at all rather than merely desired). Reclaiming
        // it here, rather than leaving it for a client that will never receive this `region` (a
        // failed mint hands back nothing), is what keeps this failure path from being a second,
        // silent leak alongside the one this function's caller already answers with `DENIED`.
        reclaim(region);
        return None;
    }

    // **`region` is not dropped here anymore.** It is returned to the caller, which delegates it to
    // the authenticated client as the fourth capability (the caretaker's own logout ticket; see
    // this program's module docs, "Reclaiming a session") and only then deletes its own copy, the
    // same "delegate, then drop our own copy" pattern already used for the directory and the
    // budget below. Dropping it here, the way an earlier version of this function did, was what
    // made the caretaker's construction memory permanently unreclaimable: nobody downstream ever
    // held a capability that could `DESTROY` it.
    let budget = untyped_split(CONSTRUCTION_UT, CLIENT_BUDGET_PAGES).ok()?;
    Some((narrow_ep, budget, region))
}

/// **Reclaim a construction region, retrying while something in it can still run.**
/// `crates/system_initializer::reclaim`'s own idiom, reused rather than re-derived: a `DESTROY` of a
/// region holding a live thread is refused with §16's kill armed, and one preemption later the retry
/// succeeds. Bounded for the same reason that one is: the only resident this ever waits on is a
/// caretaker that has already exited or is parked on its own endpoint (never on a foreign one; see
/// this program's module docs), so one preemption is enough, and a caller stuck here past a few
/// dozen attempts has a different problem than this loop can fix.
fn reclaim(region: u64) {
    for _ in 0..RECLAIM_ATTEMPTS {
        if untyped_destroy(region) {
            return;
        }
        yield_now();
    }
}

/// How many times [`reclaim`] retries, matching `crates/system_initializer::RECLAIM_ATTEMPTS`.
const RECLAIM_ATTEMPTS: usize = 64;

/// Delegate our own copy of `slot`, narrowed to `rights`, over [`RESULT`]. `GRANT` must already be
/// on our own copy for the kernel to allow this at all (`abi::rendezvous::SEND_CAP`'s contract);
/// every capability this process delegates was retyped or split by this process, so it always is.
fn delegate(slot: u64, rights: u64) {
    // SAFETY: `svc`/`ecall`; the kernel checks WRITE on RESULT and GRANT on the delegated capability.
    unsafe {
        invoke(RESULT, abi::rendezvous::SEND_CAP, slot, rights, 0);
    }
}

/// Zero the identity/secret staged at [`LOGIN_VA`], on every path: a malformed request, a denial,
/// and a success all leave a presented secret sitting in a page two processes share until this
/// runs.
fn wipe_login_page() {
    // SAFETY: the wiring mapped one page read/write here, and this process is the only writer
    // between a request arriving and its reply going out.
    let page = unsafe { core::slice::from_raw_parts_mut(LOGIN_VA as *mut u8, login_proto::PAGE) };
    login_proto::wipe(page);
}

fn fail(step: u64) -> ! {
    send(AUDIT, 0xDEAD_0000_0000_0000 | step, 0, 0);
    supervision_proto::fail()
}

user_rt::panic_handler!();
