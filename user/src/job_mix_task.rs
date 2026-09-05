//! **One task of the multi-tasking workload** (milestone 168), at EL0, where a real workload runs.
//!
//! `crates/job_mix` is the workload's definition and its header carries the argument for *what* the
//! mix is and why AIM7's own categories were kept while its 53 job names were not. This file is one
//! task: it runs the mix when the supervisor releases it, self-times, and reports.
//! `kernel/src/jobmix.rs` is the supervisor that releases it and owns the wall clock.
//!
//! # Why the tasks are processes and not kernel threads
//!
//! Milestone 134's E1 (`ipc_thread_scaling`) already sweeps IPC round-trip latency against thread
//! count, and its threads are **kernel** threads calling `sched::` directly. That is the right shape
//! for a micro-benchmark and the wrong one here. DECISIONS §96 asks what a process kernel costs a
//! *workload*, and a workload is EL0 processes: every job below crosses the trap boundary the way a
//! real program does, and the two jobs that do not ([`job_mix::COMPUTE`] and [`job_mix::TOUCH`]) are
//! there precisely to be the application-mode time between two kernel entries, which is what
//! displaces the cache a returning thread wanted.
//!
//! # The two roles
//!
//! - [`job_mix::ROLE_MIXER`]: block on the go endpoint, run [`job_mix::ROUNDS_PER_TASK`] rounds of
//!   the mix in this task's own order, report `[jobs, ticks, index]`, block again. It never exits;
//!   the supervisor halts the machine when the sweep is done.
//! - [`job_mix::ROLE_ECHO`]: `RECV_CAP` and `REPLY`, forever. The [`job_mix::ROUND_TRIP`] job's other
//!   half.
//!
//! # BUGS
//!
//! - **A mixer's self-timed ticks are not the benchmark's number.** They include whatever the
//!   scheduler did to this task while it was runnable, which is the point of a multi-tasking
//!   benchmark, so they are per-task latency rather than throughput. The number is the supervisor's
//!   wall clock over the whole subrun; these are printed beside it so a subrun with one very slow
//!   task can be told from one with many evenly slow ones.
//! - **The compute grind can be recognised by a future compiler.** It is an LCG folded into a value
//!   that leaves through a `#[inline(never)]` return and is accumulated across rounds, which is the
//!   same defence `kernel/src/bench.rs`'s `busy` uses; nothing checks that it still holds. A
//!   compute job optimised to nothing would show as an implausible jump in the whole sweep rather
//!   than as a subtle skew, which is the failure mode to hope for and not a guarantee.
//! - **The touch job walks this task's own `.bss`.** It is not a fresh mapping per iteration, so it
//!   measures cache displacement and not the page-table work AIM7's virtual-memory jobs also do.
//!   `crates/job_mix`'s own `BUGS` records the missing map job and why.
//! - **A mixer whose `CALL` is refused keeps counting the job as done.** `user_rt::call` returns two
//!   words and no status this program can distinguish from a legitimate reply, so a wedged echo
//!   server shows up as a subrun that never completes rather than as an error. The supervisor's
//!   own stall is what says so.
//!
//! Name: provisional, this lane's coinage (2026-09-04, milestone 168), and calef's call. An agent
//! noun in the family this directory already has (`soaker`, `spinner`, `painter`,
//! `os_primitives_benchmarker`): the thing that runs the job mix. `aim` and `aim7` were refused for
//! naming somebody else's benchmark, which this is not: it keeps AIM7's four methodological
//! properties and none of its 53 jobs, and a name claiming the original would be a claim about
//! comparability that `crates/job_mix`'s `BUGS` explicitly denies. `worker` was refused because
//! `user/src/worker.rs` already exists.
//!
//! **Name: ratified 2026-09-05 (calef, milestone 168).** Shipped provisionally as `job_mixer`, which
//! was wrong rather than merely inconsistent: **this program does not mix anything.** It is one task
//! *inside* the mix, and the `-er` suffix claimed an agent role it does not have, which is the
//! failure `dwarden` is cited for in AGENTS.md's own evidence, a name for the wrong relationship so
//! a reader who correctly infers the scheme gets it wrong.
//!
//! `job_mix_task` was calef's, over the maintainer's `mix_task`, and it is better for a reason the
//! maintainer had not made: **the family stays greppable as one string**, so `job_mix` finds the
//! crate, the script and this program. `job` alone was refused because this tree already uses the
//! word in the shell sense (milestone 48 is job control, and `job_undertaker` collects them).
//!
//! **`task` is this crate's own word**, not a new one: 26 of the tree's 70 uses of it are inside
//! `crates/job_mix`, and `abi` never uses it, so it competes with no kernel concept.

#![no_std]
// Program entry points, not the crates/ library surface milestone 68's ratchet tracks
// (DECISIONS §107): each `[[bin]]` is its own crate root with one `_start`.
#![allow(missing_docs)]
#![no_main]

use user_rt::{call, now, recv, recv_cap, reply, send, yield_now};

/// The working set the [`job_mix::TOUCH`] job walks: this task's own memory, sized in
/// `crates/job_mix` against the smallest L1d this project targets.
///
/// A `static mut` rather than a stack array because the user stack is one page and this is 32 KiB,
/// and because every task having its own copy is the point: two tasks sharing one buffer would be
/// measuring a cache line's worth of sharing instead of displacement.
static mut WORKING_SET: [u64; job_mix::TOUCH_WORDS] = [0; job_mix::TOUCH_WORDS];

/// A capability slot this task was given nothing in, for the [`job_mix::NULL_SYSCALL`] job to be
/// refused on. Past every slot `kernel/src/jobmix.rs` grants, and it must stay that way: a slot
/// that ever held an object would make the job invoke it rather than bounce off the kernel's
/// slot check.
const EMPTY_SLOT: u64 = 63;

/// A non-elidable integer grind, the same shape as `kernel/src/bench.rs`'s `busy`: an LCG mixed
/// with an xorshift, folded into a returned value the caller accumulates so the optimizer cannot
/// delete the loop.
#[inline(never)]
fn grind(iters: u64, seed: u64) -> u64 {
    let mut x = seed | 1;
    let mut i = 0;
    while i < iters {
        x = x
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407 ^ i);
        x ^= x >> 29;
        i += 1;
    }
    x
}

/// Read and write every word of [`WORKING_SET`] once, returning a value the caller accumulates so
/// the walk cannot be deleted.
#[inline(never)]
fn touch(seed: u64) -> u64 {
    let mut acc = seed;
    let mut i = 0;
    while i < job_mix::TOUCH_WORDS {
        // SAFETY: `WORKING_SET` is this process's own `.bss` and this program is single-threaded
        // within its address space, so there is no other reference to it anywhere. The index is
        // bounded by the array's own length.
        unsafe {
            let p = &raw mut WORKING_SET[i];
            acc = acc.wrapping_add(p.read()).rotate_left(7);
            p.write(acc);
        }
        i += 1;
    }
    acc
}

/// Run one job and return a value the caller accumulates.
fn run_job(job: u8, seed: u64) -> u64 {
    match job {
        job_mix::COMPUTE => grind(job_mix::COMPUTE_ITERS, seed),
        job_mix::TOUCH => touch(seed),
        job_mix::NULL_SYSCALL => {
            let mut i = 0;
            while i < job_mix::NULL_SYSCALL_CALLS {
                // **A real trap, and the cheapest one this ABI has.** `granted` invokes a method
                // number no object type defines on a slot this task was given nothing in, so the
                // kernel validates the slot, refuses, and returns: an entry and an exit with no
                // object work between them. `now()` was refused for this job because it is *not* a
                // syscall on two of the three architectures (an EL0 counter read on aarch64 and
                // riscv64), so a job named for the trap would have measured a loop.
                core::hint::black_box(user_rt::granted(EMPTY_SLOT));
                i += 1;
            }
            seed
        }
        job_mix::YIELD => {
            let mut i = 0;
            while i < job_mix::YIELD_CALLS {
                yield_now();
                i += 1;
            }
            seed
        }
        job_mix::ROUND_TRIP => {
            let mut acc = seed;
            let mut i = 0;
            while i < job_mix::ROUND_TRIP_CALLS {
                let (r0, _r1) = call(job_mix::SLOT_ECHO, acc, i);
                acc = acc.wrapping_add(r0).rotate_left(11);
                i += 1;
            }
            acc
        }
        // Unreachable while `crates/job_mix`'s own test holds (the mix contains no undefined kind),
        // and a spin rather than a panic if it ever does not: a task that cannot say anything is
        // better reported by the supervisor's stall than by a fault report interleaved with the
        // console of a board nobody is watching.
        _ => seed,
    }
}

/// The task's entry.
///
/// `x0` is the role, `x1` this task's index, and `x2` its seed. The supervisor gives every task a
/// distinct index and a distinct seed, so no two tasks walk the mix in the same order.
#[unsafe(no_mangle)]
pub extern "C" fn _start(role: u64, index: u64, seed: u64) -> ! {
    if role == job_mix::ROLE_ECHO {
        loop {
            let (_op, reply_slot, arg) = recv_cap(job_mix::SLOT_SERVE);
            reply(reply_slot, arg, 0);
        }
    }

    let order = job_mix::order(seed);
    let mut acc = seed | 1;

    loop {
        // The go-ahead. The supervisor hands these out one rendezvous at a time, so a task's own
        // clock starts when it is released rather than when the subrun does; the difference between
        // the two is the release skew the supervisor's own doc comment prices.
        let _ = recv(job_mix::SLOT_GO);

        let t0 = now();
        let mut round = 0;
        while round < job_mix::ROUNDS_PER_TASK {
            for &job in &order {
                acc = run_job(job, acc);
            }
            round += 1;
        }
        let ticks = now() - t0;

        // `acc` is kept alive across the report rather than folded into it: the supervisor reads
        // `index` and would print a corrupted one, and a benchmark that mangles its own labels to
        // defeat the optimizer has traded a readable result for a defence `black_box` already
        // gives. It carries into the next subrun, so nothing above it is dead.
        core::hint::black_box(acc);
        send(job_mix::SLOT_REPORT, job_mix::JOBS_PER_TASK, ticks, index);
    }
}

user_rt::panic_handler!();
