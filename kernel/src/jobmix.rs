//! **The multi-tasking workload benchmark, and the thing that times it** (milestone 168).
//!
//! `design/decisions/96-process-kernel-or-event-kernel.md` has four inputs and three of them are
//! settled. The live one is performance, and the retrospective it rests on says exactly where the
//! difference lives: *"generally within 1% on micro-benchmarks but a 20% performance advantage of
//! the event kernel on a multitasking workload (AIM7)"* (Elphinstone and Heiser, *L4 Microkernels:
//! The Lessons from 20 Years of Research and Deployment*, ACM TOCS 34(1), April 2016, section 4.1,
//! read on 2026-09-04). Every instrument this tree owns is on the left of that sentence. This
//! module is the right of it.
//!
//! `crates/job_mix` is the workload's definition, and its header carries what AIM7 actually is,
//! which of its properties are kept and which of its 53 jobs are deliberately not.
//! `user/src/job_mixer.rs` is one task. This file is the supervisor: it builds the pool, releases a
//! subrun's worth of tasks, owns the wall clock, and prints.
//!
//! # The division of labour, and why it is the soak's and not the bench's
//!
//! `kernel/src/bench.rs` would have been the obvious home and is the wrong one, for a reason the
//! gate makes concrete: **this milestone's number has to be taken on radon**, and the bench boot
//! has never run on a board. The path a board takes is `script/board-image` plus
//! `script/board-console`, which is a kernel feature that prints to the serial console and a watcher
//! that reads it. So this is shaped like `kernel/src/soak.rs`: `--features jobmix`, the tour ends
//! here instead of halting, output is machine-readable lines on the console, and `script/job-mix` is
//! the QEMU rehearsal that proves the mechanism before anybody carries a card to a desk.
//!
//! It differs from the soak in the one way that matters: **a soak never ends and this one does.**
//! The sweep is finite, and when it is finished this prints [`DONE_MARKER`] and halts.
//!
//! # The pool is built once and released in slices, which is not how AIM7 does it
//!
//! AIM7 forks a fresh set of tasks per subrun. This spawns [`job_mix::MAX_TASKS`] processes once and
//! releases the first N of them per subrun, leaving the rest blocked on their own go endpoints.
//!
//! **Two reasons, and the second is a limitation rather than a design.** The honest one: a process
//! here costs an address space and a loaded image, and `spawn_el0` in `kernel/src/bench.rs` exists
//! to reclaim exactly one of those per iteration; spawning 63 across a sweep would measure the
//! memory-region allocator's fragmentation as much as the scheduler. The one to know: it means the
//! parked tasks' kernel stacks exist during every subrun even though nothing touches them. For the
//! quantity §96 asks about that is the right choice (the cost is in the stacks the machine *cycles
//! through*, and E1 measured the same way), but it is not a faithful reproduction of AIM7's own
//! subrun, and a reader comparing the two should know which is which.
//!
//! # Release skew, priced rather than hidden
//!
//! The supervisor hands out the go-ahead one rendezvous at a time, and a rendezvous is synchronous,
//! so task N starts N round trips after task 0. At the top of the sweep that is 32 sends of roughly
//! a microsecond against a subrun of tens of milliseconds, so it is under a part in a thousand. It
//! is not zero, and the alternative (a shared flag every parked task spins on) was refused: it would
//! have the tasks *not* in this subrun burning cores while the subrun ran, which would make the
//! sweep measure the pool size rather than N.
//!
//! # BUGS
//!
//! - **A number from one boot is a draw from a distribution, and on radon that distribution spans
//!   fifteenfold.** `notes/soak.md` records four runs on that board whose rates differ by that much,
//!   and milestone 240's census explains them: the rate tracks how the boot-time placement lottery
//!   landed. Nothing in this module fixes placement, because nothing in this kernel can. What it
//!   does instead is **print the census** ([`print_census`]) so a run's number is never quotable
//!   without the arrangement that produced it, and the bench procedure in `notes/job-mix.md` asks
//!   for repeated boots rather than one. **A single boot's jobs-per-minute figure is not a result.**
//! - **The best of [`job_mix::REPEATS`] is kept, which is a decision about host noise and not about
//!   this kernel.** It is `kernel/src/bench.rs`'s own methodology (`tp_best`), and it is right on a
//!   shared dev Mac. On a board with nothing else running, the spread between repeats is itself
//!   information, and this throws it away. The per-repeat lines are printed for that reason, so the
//!   spread is in the log even though the summary line is not.
//! - **This does not compare a process kernel against an event kernel**, and it cannot: there is no
//!   event kernel to compare against. It measures what this kernel does under multi-tasking load,
//!   which is the input §96 says it is missing. §96 stays open either way.
//! - **The supervisor is a thread on the machine under test.** It is blocked in `RECV` for the whole
//!   of every timed window rather than spinning, so it is much less load than the soak's yielding
//!   watcher; it is not zero, and it is one more thread the scheduler holds.
//! - **The subrun's wall clock includes the tasks' own report sends.** The last of N tasks finishing
//!   is followed by N rendezvous the supervisor drains, and the clock stops after the last of them.
//!   That is deliberate (AIM7's subrun ends when every task has finished, and reporting completion
//!   is part of finishing) and it is a fixed additive cost that grows with N, so it flatters the
//!   small subruns by a few microseconds.

use job_mix::{ECHO_SERVERS, MAX_TASKS, REPEATS, TASK_SWEEP};

use crate::cap::{Rights, rendezvous_cap};
use crate::user::{self, Spawn};
use crate::{arch, println, sched, smp};

/// The line that tells a reader, and a log, that a sweep has begun.
const START_MARKER: &str = "jobmix: started";

/// The line that says the sweep is finished and the machine is about to park.
///
/// `script/job-mix` watches for this the way `cargo xtask bench` watches for `bench: done`: the
/// kernel halts rather than exiting, so the host side is what tears QEMU down.
const DONE_MARKER: &str = "jobmix: done";

/// The prefix every placement-census line carries.
///
/// Its own word rather than `jobmix:`, for the reason `kernel/src/soak.rs`'s own census marker
/// gives: a census is neither the start of a run nor a result line, and a watcher matching on the
/// result prefix should not have to be proven harmless against it.
const CENSUS_MARKER: &str = "jobmix-census:";

/// Run the sweep and never come back. The caller is the boot thread at the end of the tour, and
/// this replaces its `arch::halt()`.
pub fn run() -> ! {
    let cores = smp::online_count();

    let Some(image) = user::program("job_mixer") else {
        println!("jobmix: FAILED: no 'job_mixer' program in the initrd archive; nothing to run");
        arch::halt();
    };

    let report = sched::create_rendezvous();
    let mut go = [0u64; MAX_TASKS];
    for slot in &mut go {
        *slot = sched::create_rendezvous();
    }
    let mut echo = [0u64; ECHO_SERVERS];
    for slot in &mut echo {
        *slot = sched::create_rendezvous();
    }

    // Where the kernel put each task at spawn, kept rather than forgotten: milestone 240's finding
    // is that this decides throughput on real silicon by up to fifteenfold, so a result printed
    // without it is a number nobody can interpret. Servers first, then tasks, which is the order
    // `census_letter` reads them back in.
    let mut placed = [u8::MAX; ECHO_SERVERS + MAX_TASKS];

    // **The servers before the tasks.** A mixer whose server is not yet receiving simply parks on
    // the endpoint, so the order is not load-bearing for correctness; starting the answering half
    // first keeps a fresh boot's first subrun from timing a server's spawn.
    for (i, &ep) in echo.iter().enumerate() {
        let grant = rendezvous_cap(ep, Rights::READ);
        let started = sched::spawn_reporting_placement(move || {
            user::run(
                image,
                Spawn {
                    arg0: job_mix::ROLE_ECHO,
                    arg1: i as u64,
                    arg2: 0,
                    grants: &[grant],
                    maps: &[],
                },
            )
        });
        let Some((_tid, cpu)) = started else {
            println!("jobmix: FAILED: could not spawn echo server {i} of {ECHO_SERVERS}");
            arch::halt();
        };
        placed[i] = u8::try_from(cpu).unwrap_or(u8::MAX);
    }

    for i in 0..MAX_TASKS {
        // Slot order is `job_mix`'s, and both sides read it from there rather than from a second
        // copy of the numbering. The rights are the least each use needs: a task may write its
        // report and may not read other tasks' reports; it may read its own go endpoint and may not
        // release anybody, including itself; it may call a server and may not answer as one.
        let grants = [
            rendezvous_cap(report, Rights::WRITE),
            rendezvous_cap(go[i], Rights::READ),
            rendezvous_cap(echo[i % ECHO_SERVERS], Rights::WRITE),
        ];
        let started = sched::spawn_reporting_placement(move || {
            user::run(
                image,
                Spawn {
                    arg0: job_mix::ROLE_MIXER,
                    arg1: i as u64,
                    // A distinct, odd, well-spread seed per task, so no two walk the mix in the
                    // same order. Two tasks handed the same seed would be in lockstep, which is
                    // the one property `job_mix::order` exists to prevent.
                    arg2: (i as u64)
                        .wrapping_mul(0x9E37_79B9_7F4A_7C15)
                        .wrapping_add(1),
                    grants: &grants,
                    maps: &[],
                },
            )
        });
        let Some((_tid, cpu)) = started else {
            println!("jobmix: FAILED: could not spawn task {i} of {MAX_TASKS}");
            arch::halt();
        };
        placed[ECHO_SERVERS + i] = u8::try_from(cpu).unwrap_or(u8::MAX);
    }

    let hz = arch::timer::frequency();
    println!();
    println!(
        "{START_MARKER} {MAX_TASKS} tasks and {ECHO_SERVERS} servers on {cores} online core(s), \
         cntfrq={hz} Hz"
    );
    println!(
        "jobmix: one job is one of {} kinds ({} per round, {} rounds per task per subrun); see \
         crates/job_mix for the mix and what it deliberately leaves out",
        job_mix::JOB_KINDS,
        job_mix::MIX_LEN,
        job_mix::ROUNDS_PER_TASK
    );
    println!(
        "jobmix: this measures THIS kernel under multi-tasking load; it compares nothing against an \
         event kernel and does not decide design/decisions/96-process-kernel-or-event-kernel.md"
    );
    print_census(&placed);
    println!(
        "{CENSUS_MARKER} placement decides throughput on real silicon by up to fifteenfold \
         (notes/soak.md, milestone 240), so a jobs-per-minute figure from ONE boot is a draw and \
         not a result; notes/job-mix.md's procedure asks for repeated boots"
    );

    for &tasks in &TASK_SWEEP {
        let mut best = u64::MAX;
        for repeat in 0..REPEATS {
            let ticks = subrun(report, &go[..tasks]);
            println!("jobmix-repeat: tasks={tasks} repeat={repeat} ticks={ticks}");
            best = best.min(ticks);
        }
        let jobs = tasks as u64 * job_mix::JOBS_PER_TASK;
        println!(
            "jobmix: tasks={tasks} jobs={jobs} ticks={best} jpm={}",
            job_mix::jobs_per_minute(jobs, best, hz)
        );
    }

    println!("{DONE_MARKER}");
    // Parked, not exited: the watcher saw the marker and tears the run down, and a forgotten QEMU
    // costs nothing in `wfi` (AGENTS.md's rule).
    arch::halt();
}

/// One subrun: release `go.len()` tasks, wait for all of them, and return the wall-clock ticks.
///
/// The clock starts before the first release and stops after the last report, which is AIM7's own
/// definition of a subrun (it ends when every one of its tasks has completed its jobs) and which
/// this module's `BUGS` prices.
fn subrun(report: sched::RendezvousId, go: &[sched::RendezvousId]) -> u64 {
    let t0 = arch::timer::now();
    for &ep in go {
        sched::ipc_send(ep, [0, 0, 0]);
    }
    for _ in go {
        let _ = sched::ipc_recv(report);
    }
    arch::timer::now() - t0
}

/// **Print which of the pool is on which core, one line per core** (the shape milestone 240 gave
/// `kernel/src/soak.rs`, for the same reason).
///
/// `placed[i]` is where thread `i` was put at spawn, servers first and then tasks, and `u8::MAX`
/// means nothing knows yet. Tokens are `S<n>` for a server and `T<n>` for a task.
///
/// A core with no threads gets a line too, because that is the informative case: four cores online
/// and one of them empty is an explanation for a slow run, and a census that printed only the
/// occupied cores would hide it. This states where the threads are and draws no conclusion, which
/// is the constraint milestone 240 wrote for itself: if a series of boots shows the rate does not
/// follow the placement, that is the result, and this must not have prejudged it.
fn print_census(placed: &[u8]) {
    println!("{CENSUS_MARKER} where the kernel placed each thread at spawn: S=echo server, T=task");
    let mut accounted = 0usize;
    for core in smp::online_cpus() {
        let here = u8::try_from(core).unwrap_or(u8::MAX);
        let n = placed.iter().filter(|&&c| c == here).count();
        accounted += n;
        crate::print!("{CENSUS_MARKER} core={core} threads={n}");
        for (i, &c) in placed.iter().enumerate() {
            if c == here {
                if i < ECHO_SERVERS {
                    crate::print!(" S{i}");
                } else {
                    crate::print!(" T{}", i - ECHO_SERVERS);
                }
            }
        }
        println!();
    }
    // Everything the per-core loop did not name, which has two causes and both deserve a line
    // rather than a silent shortfall: a thread that has not been switched to yet (its `last_cpu`
    // is unset), and a core id outside the online set, which would be a placement bug rather than
    // a census one.
    let missing = placed.len() - accounted;
    if missing != 0 {
        let unknown = placed.iter().filter(|&&c| c == u8::MAX).count();
        println!(
            "{CENSUS_MARKER} unplaced={missing} of which not-yet-run={unknown} (online={})",
            smp::online_count()
        );
    }
}
