# 221. The soak never crosses cores, so build the hook that makes it

**Status: NOT-STARTED.** Minted 2026-09-02 by the maintainer, the moment calef approved option D in
DECISIONS 138 (how a saturated workload is made to hand threads across cores). *(Number provisional
until the merge queue lands it.)*

**Gate: NONE.** The decision is made, the mechanism is measured, and nothing here needs a board.

**In brief.** Milestone 219 (the boot tour ends and the kernel halts, so there is nothing to soak)
built a workload that lasts and then found something nobody expected: **a saturated workload never
migrates between cores.** Measured on both multicore architectures, the crossing count freezes
within the first second while the machine does tens of thousands of IPC round trips a second.

DECISIONS 138 established why, and it is three structural facts rather than a bug: rendezvous wakes
are local by design (DECISIONS §28.2), `wake_load_aware` is reachable only from a device interrupt,
and a work steal needs an idle core **and** a queued thread elsewhere, which a saturated machine
does not have. It also found that nife has counterparts for three of Linux v6.12's four balancing
moments, all event-driven, and that a saturated rendezvous workload starves all three of their
triggers. That is the whole explanation.

**So the second half of fatal risk 5's decisive experiment has never been runnable**, and it is the
half that matters: the one defect this risk has produced, on radon, was a receiver made Ready with
nothing delivered, on the `irq_notify` to `wake_load_aware` path. The existing soak exercises
contention on shared kernel state. It does not exercise the path the defect was on.

## What it needs

**A `--features soak` hook that signals a routed rendezvous from `sched::on_tick`**, so soak workers
block on it through the `Irq::WAIT` that already exists. `sched::on_tick` is called by all three
timer dispatchers in real interrupt context on every core, so the hook runs the identical sequence a
device interrupt runs, down through `wake_load_aware`, `pick_wake_target`, `place_on` and the
reschedule IPI.

**The research lane spiked this and threw the spike away**, so the shape is known rather than
guessed: about 53 lines across three files, no syscall, no architecture-specific code. Measured:

| | today | with the spike |
|---|---|---|
| aarch64, 4 cores | 15 crossings, frozen | 3,785 and rising |
| riscv64, 4 cores | 21 crossings, frozen | 1,948 and rising |

Round-trip rate falls (aarch64 64,000 to 44,500 a second), which is expected and must be recorded
rather than hidden: the machine is doing more work per round trip.

**Being architecture-neutral is load-bearing, not incidental.** riscv64 has no software-raisable
line reaching `irq_route`, so an aarch64 `send_sgi` or an x86 self-IPI would have left **radon** out,
and radon is the machine that produced the defect this milestone exists to hunt.

## What it must not claim

**The threads that cross are the waiters, not the rendezvous pairs.** This sustains the wake protocol
across cores under load. It does not make the IPC workload itself migrate, and only a rebalancer
would, which DECISIONS 138 declines. Anything this milestone prints or records must not let a reader
conclude otherwise, in the same way milestone 219's own output refuses to let a green soak be quoted
as proof the concurrency is correct.

**A soak build is already not a production build** (milestone 219 measured the fastpath at about
1.05x with instrumentation compiled in), and this widens that gap. A soak number compares only with
another soak number.

## BUGS

- **This does not run the experiment**, it makes it runnable. The run needs a bench evening on radon,
  argon or xenon, and that is milestone 219's other proposed follow-on rather than this one.
- **It says nothing about what a crossing rate should be.** The spike's numbers are a shape, not a
  target, and there is no baseline to compare a board against until a board has run one.
- **The hook fires on a timer, which is the one thing the workload cannot starve.** That is why it
  works, and it is also why it is not evidence about what happens without it.
