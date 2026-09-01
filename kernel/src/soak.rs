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
//! - **Nothing here sets a duration, and nothing here should.** The kernel soaks until the power
//!   goes away. The watcher decides when enough is enough, which is what makes the QEMU run and the
//!   bench run the same experiment with a different deadline.

use core::sync::atomic::Ordering;

use paging::Flags;
use soak_page::MAX_WORKERS;

use crate::cap::{Rights, rendezvous_cap};
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
    // Two workers per core, so every core has a runnable thread and every run queue still goes
    // empty often enough for the work-steal path to fire. One pair per core is the smallest
    // configuration that puts a rendezvous *between* cores rather than inside one; more pairs than
    // that buys queue depth rather than crossings, and queue depth is not what the risk is about.
    //
    // At least two pairs even on a single-core boot, because the QEMU legs run with one vCPU on
    // some architectures and a soak that cannot start there is a soak nobody runs before a bench
    // trip.
    let cores = smp::online_count();
    let pairs = cores.max(2).min(MAX_WORKERS / 2);

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

    for pair in 0..pairs {
        let endpoint = sched::create_rendezvous();
        // The responder first. A caller whose responder is not yet receiving simply parks on the
        // rendezvous, so the order is not load-bearing; starting the answering half first keeps a
        // fresh boot's first few beats from reading as a stall.
        for role in [ROLE_RESPONDER, ROLE_CALLER] {
            let index = worker_index(pair, role);
            let rights = if role == ROLE_RESPONDER {
                Rights::READ
            } else {
                Rights::WRITE
            };
            let started = sched::spawn(move || {
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
                        grants: &[rendezvous_cap(endpoint, rights)],
                        maps: &[Mapping {
                            va: soak_page::VA,
                            phys: shared,
                            flags: Flags::user_data(),
                        }],
                    },
                )
            });
            if started.is_none() {
                println!(
                    "soak: FAILED: could not spawn worker {index} of {}",
                    pairs * 2
                );
                arch::halt();
            }
        }
    }

    let workers = pairs * 2;
    println!();
    println!(
        "{START_MARKER} {pairs} caller/responder pairs ({workers} user threads) on {cores} online \
         core(s), beating every {BEAT_SECONDS}s"
    );
    println!(
        "soak: silence longer than a few beats is a hang, not a slow run; the beat is on the wall \
         clock and does not depend on the workload making progress"
    );
    println!(
        "soak: a clean run is a number to compare against, NOT evidence that the concurrency is \
         correct (design/roadmap/219-a-workload-that-does-not-stop.md)"
    );

    watch(shared, workers)
}

/// `arg0` for the half that answers, matching `user/src/soaker.rs`.
const ROLE_RESPONDER: u64 = 0;
/// `arg0` for the half that calls.
const ROLE_CALLER: u64 = 1;

/// Which slot of the shared page belongs to this pair's half.
///
/// Pairs are laid out consecutively (`0` and `1` are the first pair) so that a stalled index in a
/// report names its partner by arithmetic a reader can do in their head.
fn worker_index(pair: usize, role: u64) -> usize {
    pair * 2 + if role == ROLE_RESPONDER { 0 } else { 1 }
}

/// Read one `u64` out of the shared page through the direct map.
fn read_counter(shared: u64, offset: u64) -> u64 {
    // SAFETY: `shared` is a frame this module allocated and never freed, the direct map names it,
    // and `offset` comes from `soak_page`, whose host tests prove every offset it produces lands
    // inside one page.
    unsafe { core::ptr::read_volatile((arch::mmu::phys_to_virt(shared) + offset) as *const u64) }
}

/// Beat, sample, judge; forever, or until something is wrong.
fn watch(shared: u64, workers: usize) -> ! {
    let hz = arch::timer::frequency();
    let started = arch::timer::now();
    let mut previous = [0u64; MAX_WORKERS];
    let mut last_total = 0u64;
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
        let mut mismatches = 0u64;
        let mut stalled = 0usize;
        let mut first_stalled = usize::MAX;
        for i in 0..workers {
            let rounds = read_counter(shared, soak_page::rounds(i));
            mismatches = mismatches.wrapping_add(read_counter(shared, soak_page::mismatches(i)));
            total = total.wrapping_add(rounds);
            if rounds == previous[i] {
                stalled += 1;
                if first_stalled == usize::MAX {
                    first_stalled = i;
                }
            }
            previous[i] = rounds;
        }

        let refused = sched::wake_refusals();
        let rate = total.wrapping_sub(last_total) / window;
        // One line, one beat, every field named. A log a person greps six weeks later is worth more
        // than a table that lines up on a terminal nobody kept.
        println!(
            "soak: t={elapsed}s beat={beat} rounds={total} rate={rate}/s workers={workers} \
             refused={refused} mismatch={mismatches} stalled={stalled} remote={} steals={} \
             deferred={}",
            sched::remote_placements(),
            sched::steals_served(),
            sched::wakes_deferred(),
        );
        last_total = total;
        last_at = now;

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
                println!(
                    "soak: first stalled worker is {first_stalled} (its partner is {})",
                    first_stalled ^ 1
                );
            }
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
