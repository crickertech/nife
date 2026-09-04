//! **The job mix a multi-tasking benchmark runs, defined once** (milestone 168).
//!
//! `design/decisions/96-process-kernel-or-event-kernel.md` asks whether this kernel should keep a
//! kernel stack per thread. Three of its four inputs are settled; the live one is performance, and
//! the retrospective it rests on is explicit that the difference **does not appear where this
//! project measures**: Warton's event kernel was "generally within 1% on micro-benchmarks but a 20%
//! performance advantage of the event kernel on a multitasking workload (AIM7)" (Elphinstone and
//! Heiser, *L4 Microkernels: The Lessons from 20 Years of Research and Deployment*, ACM TOCS 34(1),
//! April 2016, section 4.1; read from
//! <https://trustworthy.systems/publications/nicta_full_text/8988.pdf> on 2026-09-04).
//!
//! Every instrument this tree owns is on the wrong side of that sentence. This crate is the shape
//! of the one that is not: the **workload definition** both halves of the instrument agree on, so
//! the kernel-side supervisor and the EL0 task read one description of what a job is rather than
//! two copies that drift. AGENTS.md rule 7 is why it is a crate and not a `#[path]` module.
//!
//! # What AIM7 actually is, since the name is not self-explanatory
//!
//! Read rather than recalled, on 2026-09-04, from the benchmark's own README
//! (<https://github.com/davidlohr/areaim/blob/master/osdl-aim-7/_NOTICES/README.aim7>) and the
//! encyclopaedia entry that summarises it: AIM Multiuser Benchmark Suite VII forks many processes
//! called **tasks**, each of which runs, **in random order**, a set of subtests called **jobs**.
//! There are 53 job kinds covering disk-file operations, process creation, user virtual-memory
//! operations, pipe I/O and compute-bound arithmetic, and a **workfile** sets the proportions. A run
//! is a sequence of **subruns** with the task count incremented between them; each subrun ends when
//! every one of its tasks has finished its jobs, and reports **jobs completed per minute**. The
//! final report is that throughput against task count.
//!
//! Four properties do the work there, and this crate keeps all four:
//!
//! 1. **Heterogeneity.** A task alternates between unrelated kernel paths rather than hammering
//!    one. That is the property a micro-benchmark cannot have by construction.
//! 2. **Random order per task.** Tasks are not in lockstep, so the machine sees a mixed arrival
//!    stream instead of a phase-aligned one.
//! 3. **A task-count sweep.** The result is a curve, not a number.
//! 4. **Throughput, not latency.** Jobs per minute, timed to the completion of the slowest task.
//!
//! **What is deliberately not kept is AIM7's own 53 jobs**, and the reason is not effort. Most of
//! them name a Unix service this system does not have and should not grow one to be measured:
//! `fork`, `link`, `sync`, `signal` handlers, `sbrk`. Porting the names would produce a benchmark
//! measuring a shim. The mix below keeps AIM7's *categories* instead, expressed in this kernel's own
//! primitives, and [`BUGS`](self#bugs) records which categories are still missing.
//!
//! # The five jobs
//!
//! | job | AIM7 category | what it costs here |
//! |---|---|---|
//! | [`COMPUTE`] | compute-bound arithmetic loops | no syscall at all; the control arm |
//! | [`TOUCH`] | user virtual-memory operations | a working set walked in user mode, no syscall |
//! | [`NULL_SYSCALL`] | (the trap itself) | EL0 to EL1 and back, the cheapest kernel entry |
//! | [`YIELD`] | (scheduling pressure) | a full context switch through the ready queue |
//! | [`ROUND_TRIP`] | pipe I/O | `CALL` to a shared server and its `REPLY`: two rendezvous |
//!
//! **[`ROUND_TRIP`] is the one that answers §96 and the other four are what make it a workload.**
//! A process kernel's cost is the kernel stack a blocking thread leaves behind, so the quantity that
//! matters is how many *distinct* stacks the machine cycles through and how much of the cache each
//! displaces between visits. [`COMPUTE`] and [`TOUCH`] are what displace it: they are the
//! "application" that runs between two kernel entries, which is the thing
//! `kernel/src/bench.rs`'s `app_displacement` (milestone 134's E4) measures in isolation and which a
//! ping-pong micro-benchmark leaves out entirely.
//!
//! # BUGS
//!
//! - **Three of AIM7's categories are absent: disk-file operations, process creation, and page
//!   mapping.** Each was refused for a stated reason rather than overlooked. A filesystem job needs
//!   a disk attached and would make the instrument's availability depend on the runner's storage,
//!   which is the thing that keeps `script/bench`'s `fs_*` rows out of the gated set. A spawn job
//!   costs an address space per iteration and the tree's own `spawn_el0` benchmark exists to
//!   reclaim them one at a time; at 32 concurrent tasks it would measure the memory-region
//!   allocator rather than the scheduler. A map job needs a per-task address-space capability that
//!   the spawn path does not currently hand out. **All three are real gaps in fidelity**, and the
//!   honest reading of a result from this mix is that it covers the compute, memory, trap,
//!   scheduling and IPC categories and no others.
//! - **The mix proportions are chosen, not derived.** AIM7 ships workfiles for four machine roles
//!   (multiuser, compute server, large database, file server) and nobody here has one for a
//!   capability microkernel. [`MIX`] is a flat-ish spread with the IPC job weighted up, on the
//!   argument that IPC is what this kernel is for. A different mix would give a different number,
//!   and no result from this instrument should be quoted without saying which mix produced it.
//! - **The per-job work constants are sized for a real machine, not for TCG.** A QEMU icount run of
//!   the full sweep takes minutes and its magnitudes are fiction, which is the same caveat every
//!   `--real`-only benchmark in this tree carries. The rehearsal exists to prove the mechanism.
//! - **[`order`] is a shuffle, not a random sequence.** Every task runs exactly the same multiset of
//!   jobs, in a per-task order. That is what makes two tasks' work comparable, and it is a
//!   simplification against AIM7, whose tasks draw independently.
//!
//! Name: provisional, this lane's coinage (2026-09-04, milestone 168), and calef's call. A noun
//! pair naming the thing the crate defines, in the `snake_case` this tree's crates use, and it is
//! the phrase the source itself uses: AIM7's workfile is a *mix* of *jobs*. `aim7` was refused for
//! claiming somebody else's benchmark, which this is not (see the BUGS above: none of AIM7's 53
//! jobs is here, and no number from this is comparable with an AIM7 number). `workload` was refused
//! as too general for a tree that already has a soak workload and a compute workload. `benchmark`
//! was refused because this crate is the workload's *definition* and produces no measurement; the
//! same distinction `os_primitives_benchmarker`'s own header draws between the agent and the
//! output.

#![no_std]
#![deny(missing_docs)]

/// A compute-bound arithmetic loop, [`COMPUTE_ITERS`] iterations. No syscall.
pub const COMPUTE: u8 = 0;
/// A walk over [`TOUCH_WORDS`] words of the task's own memory, read and written. No syscall.
pub const TOUCH: u8 = 1;
/// [`NULL_SYSCALL_CALLS`] trips through the cheapest syscall this ABI has.
pub const NULL_SYSCALL: u8 = 2;
/// [`YIELD_CALLS`] voluntary yields, each a trip through the ready queue.
pub const YIELD: u8 = 3;
/// [`ROUND_TRIP_CALLS`] `CALL`/`REPLY` round trips against a shared server.
pub const ROUND_TRIP: u8 = 4;

/// How many job kinds there are. A counted claim: the table in this crate's header has one row per
/// kind, and [`MIX`] must contain each of them at least once.
pub const JOB_KINDS: usize = 5;

/// **The workfile, in AIM7's sense**: the multiset of jobs one task runs per round, and therefore
/// the proportions. Sixteen entries so a round is long enough to time and short enough that a task
/// visits every kind several times inside one subrun.
///
/// The weighting is stated rather than derived (see this crate's `BUGS`): [`ROUND_TRIP`] gets a
/// quarter of the mix because IPC is the primitive this kernel exists to be fast at, and the other
/// four split the rest evenly.
pub const MIX: [u8; 16] = [
    COMPUTE,
    TOUCH,
    NULL_SYSCALL,
    YIELD,
    ROUND_TRIP,
    COMPUTE,
    TOUCH,
    NULL_SYSCALL,
    YIELD,
    ROUND_TRIP,
    COMPUTE,
    TOUCH,
    ROUND_TRIP,
    NULL_SYSCALL,
    YIELD,
    ROUND_TRIP,
];

/// Jobs in one round, which is [`MIX`]'s length.
pub const MIX_LEN: usize = MIX.len();

/// Rounds of [`MIX`] one task runs per subrun. `JOBS_PER_TASK` is this times [`MIX_LEN`].
///
/// Sized so that a single task's subrun runs for a few tens of milliseconds on real silicon, which
/// is two things at once: long against the release skew the supervisor cannot avoid (it hands out
/// the go-ahead one rendezvous at a time, so the last task starts N round trips after the first),
/// and long against a timer whose grain is tens of nanoseconds.
pub const ROUNDS_PER_TASK: u64 = 8;

/// Jobs one task completes in one subrun. The numerator of the AIM7 metric.
pub const JOBS_PER_TASK: u64 = ROUNDS_PER_TASK * MIX_LEN as u64;

/// Iterations of the arithmetic grind in one [`COMPUTE`] job.
pub const COMPUTE_ITERS: u64 = 20_000;

/// `u64` words touched in one [`TOUCH`] job. 4,096 words is 32 KiB, which is the smallest L1d this
/// project targets (the `SiFive` U74's), so one job's working set is exactly the size that evicts
/// it. Chosen against that cache rather than against the dev Mac's, for the reason milestone 134's
/// E1 records: a large-cache machine hides this whole effect.
pub const TOUCH_WORDS: usize = 4_096;

/// Syscalls in one [`NULL_SYSCALL`] job.
pub const NULL_SYSCALL_CALLS: u64 = 64;

/// Yields in one [`YIELD`] job.
pub const YIELD_CALLS: u64 = 64;

/// `CALL`/`REPLY` round trips in one [`ROUND_TRIP`] job.
pub const ROUND_TRIP_CALLS: u64 = 32;

/// The largest task pool the supervisor builds, and therefore the length of its go-endpoint array.
///
/// 32, against `sched::MAX_THREADS`'s 256 and against the 64 user threads the soak already builds
/// on this machine. The sweep tops out here rather than at milestone 134's E1 ceiling of 96 because
/// every task in this pool is a **process**, with an address space and a loaded image behind it,
/// where E1's were kernel threads; the memory, not the thread table, is what binds.
pub const MAX_TASKS: usize = 32;

/// The subruns, in AIM7's sense: how many tasks are released for each measurement.
///
/// AIM7 increments by one and runs until throughput collapses. This doubles, because a run on a
/// board is an evening of somebody's time and the interesting feature (a knee, or its absence) is
/// visible on a log axis. 1 is the control: a single task with the whole machine.
pub const TASK_SWEEP: [usize; 6] = [1, 2, 4, 8, 16, MAX_TASKS];

/// Measured repeats per subrun. The **best** is kept, for `kernel/src/bench.rs`'s stated reason:
/// the minimum is the least host-contended sample, and everything above it is somebody else's load.
pub const REPEATS: usize = 3;

/// Shared servers the [`ROUND_TRIP`] job calls. More than one so that the sweep's larger subruns are
/// not measuring a single server's serialization; fewer than the task count so that the endpoint is
/// genuinely contended, which is what a multi-tasking workload does to a microkernel.
pub const ECHO_SERVERS: usize = 2;

/// `arg0` for a task that runs the mix.
pub const ROLE_MIXER: u64 = 0;
/// `arg0` for a shared server the mixers call.
pub const ROLE_ECHO: u64 = 1;

/// Mixer slot 0: the endpoint it `SEND`s its result on. Slot 0 is "the endpoint I report on"
/// throughout this tree's benchmark programs, and this keeps that true.
pub const SLOT_REPORT: u64 = 0;
/// Mixer slot 1: the endpoint it `RECV`s its go-ahead on, one per task.
pub const SLOT_GO: u64 = 1;
/// Mixer slot 2: the endpoint it `CALL`s for a [`ROUND_TRIP`] job.
pub const SLOT_ECHO: u64 = 2;
/// Echo-server slot 0: the endpoint it serves. It holds nothing else, and cannot report, spawn or
/// call: a compromised echo server can only answer wrongly, which is the least authority that does
/// the job.
pub const SLOT_SERVE: u64 = 0;

// **More tasks than servers, checked by the compiler rather than by a test.** The [`ROUND_TRIP`]
// job's endpoint has to be contended at the top of the sweep or the instrument is measuring an idle
// machine, and an instrument with no server at all cannot run that job at all. This is a relation
// between two constants in one file, which is the case where AGENTS.md's ladder says to make the
// wrong state unrepresentable rather than to write a check that runs later.
const _: () = assert!(ECHO_SERVERS >= 1 && ECHO_SERVERS < MAX_TASKS);

/// The order one task runs [`MIX`] in: a permutation of [`MIX`] chosen by `seed`.
///
/// Fisher-Yates driven by a 64-bit LCG, which is enough randomness for "these tasks are not in
/// lockstep" and is deterministic given the seed, so a run is reproducible. It is **not** a source
/// of randomness for anything else and must not be used as one.
#[must_use]
pub fn order(seed: u64) -> [u8; MIX_LEN] {
    let mut out = MIX;
    let mut x = seed | 1;
    let mut i = MIX_LEN;
    while i > 1 {
        i -= 1;
        x = x
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        // The high bits of an LCG are the good ones; the low bits cycle short.
        let j = ((x >> 33) as usize) % (i + 1);
        out.swap(i, j);
    }
    out
}

/// **The AIM7 metric**: jobs completed per minute, from a job count, a tick delta and the counter's
/// frequency in hertz.
///
/// Saturating rather than wrapping, and zero on a zero-length or zero-frequency measurement, so a
/// nonsense input produces a number a reader will disbelieve rather than one they will quote. The
/// intermediate is `u128` because `jobs * 60 * hz` overflows `u64` at unremarkable inputs: 4,096
/// jobs on radon's 4 MHz `rdtime` is already 9.8e11, and a 1 GHz counter would be there in one
/// subrun.
#[must_use]
pub fn jobs_per_minute(jobs: u64, ticks: u64, hz: u64) -> u64 {
    if ticks == 0 || hz == 0 {
        return 0;
    }
    let n = u128::from(jobs) * 60 * u128::from(hz) / u128::from(ticks);
    u64::try_from(n).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every job kind named in the header's table appears in the mix. A kind that is defined and
    /// never run is a row of documentation describing a workload that does not exist.
    #[test]
    fn the_mix_runs_every_job_kind() {
        for kind in 0..JOB_KINDS as u8 {
            assert!(MIX.contains(&kind), "job kind {kind} is never run");
        }
    }

    /// The mix is exactly [`JOB_KINDS`] kinds and nothing else, so a kind added to the constants
    /// without a row in the table cannot ride along unnoticed.
    #[test]
    fn the_mix_contains_no_kind_that_is_not_defined() {
        for job in MIX {
            assert!(job < JOB_KINDS as u8, "mix holds undefined job kind {job}");
        }
    }

    /// **The property that makes two tasks comparable**: an order is a permutation of the mix, so
    /// every task does exactly the same work and only the sequence differs. If this were merely a
    /// random draw, a subrun's slowest task might simply have drawn more expensive jobs, and the
    /// throughput number would be measuring the dice.
    #[test]
    fn an_order_is_a_permutation_of_the_mix() {
        for seed in [0u64, 1, 7, 0x9E37_79B9_7F4A_7C15, u64::MAX] {
            let got = order(seed);
            let mut a = MIX;
            let mut b = got;
            a.sort_unstable();
            b.sort_unstable();
            assert_eq!(a, b, "seed {seed} did not produce a permutation");
        }
    }

    /// Different seeds produce different orders, which is the whole reason the seed exists: tasks
    /// released together must not walk the mix in lockstep. Not every pair need differ, but the
    /// spread over a hundred seeds must not collapse to one sequence.
    #[test]
    fn distinct_seeds_do_not_all_walk_the_mix_in_lockstep() {
        let first = order(1);
        let mut differing = 0;
        for seed in 2..102u64 {
            if order(seed) != first {
                differing += 1;
            }
        }
        assert!(
            differing > 90,
            "only {differing} of 100 seeds differed from the first; the shuffle is degenerate"
        );
    }

    /// The same seed twice is the same order. A run has to be reproducible for a second run on the
    /// same board to be a comparison rather than a new experiment.
    #[test]
    fn an_order_is_deterministic_in_its_seed() {
        assert_eq!(order(12_345), order(12_345));
    }

    /// The metric, against a case worked by hand: 600 jobs in one second of a 4 MHz counter is
    /// 36,000 jobs per minute.
    #[test]
    fn the_metric_is_jobs_per_minute() {
        assert_eq!(jobs_per_minute(600, 4_000_000, 4_000_000), 36_000);
    }

    /// A zero-length or unmeasurable window reports zero rather than dividing by zero or reporting
    /// an enormous rate a reader might believe.
    #[test]
    fn an_unmeasurable_window_reports_zero() {
        assert_eq!(jobs_per_minute(600, 0, 4_000_000), 0);
        assert_eq!(jobs_per_minute(600, 4_000_000, 0), 0);
    }

    /// **The overflow this function exists to hold.** `jobs * 60 * hz` at a plausible silicon
    /// frequency passes `u64::MAX` while the answer is small, so a `u64` intermediate would have
    /// reported a wrapped rate as though it were a measurement.
    #[test]
    fn a_plausible_silicon_measurement_does_not_overflow() {
        // 32 tasks * 128 jobs at 1 GHz over one second: the numerator is 2.5e11 * 1e9.
        let jobs = MAX_TASKS as u64 * JOBS_PER_TASK;
        let hz = 1_000_000_000;
        assert_eq!(jobs_per_minute(jobs, hz, hz), jobs * 60);
    }

    /// The sweep is increasing and ends at the pool size, so the supervisor never releases more
    /// tasks than it built and a partial run's printed points are still a prefix of the curve.
    #[test]
    fn the_sweep_climbs_to_the_pool_size() {
        assert_eq!(*TASK_SWEEP.last().expect("a sweep has points"), MAX_TASKS);
        for w in TASK_SWEEP.windows(2) {
            assert!(w[0] < w[1], "the sweep is not increasing at {w:?}");
        }
        assert!(TASK_SWEEP[0] >= 1);
    }

}
