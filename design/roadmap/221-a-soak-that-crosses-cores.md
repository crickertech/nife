# 221. The soak never crosses cores, so build the hook that makes it

**Status: BUILT** (2026-09-02). Minted 2026-09-02 by the maintainer, the moment calef approved
option D in DECISIONS 138 (how a saturated workload is made to hand threads across cores). *(Number
provisional until the merge queue lands it.)*

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

## What was built

`sched::on_tick` gained one `#[cfg(feature = "soak")]` call. `kernel/src/soak.rs` gained the tick
route (a rendezvous, a soak-only intid, and the signal the tick sends it), `user/src/soaker.rs`
gained a waiter role that loops on `user_rt::irq_wait`, `crates/soak_page` gained a third counter
array so a wake is never mistaken for a round trip, and `crates/board_console` and the `script/soak`
summary carry the new field and the sentence that says what the crossings are.

No syscall, no architecture-specific code, and no new feature: it is all behind the `soak` feature
milestone 219 already had.

**Measured on patagonia under QEMU, 2026-09-02, `script/soak --for 30s`, each pair back to back
against the commit this branched from:**

| Architecture | Cores | Round trips/s before | after | Crossings in 25s before | after |
|---|---|---|---|---|---|
| aarch64 | 4 | 47,864 | 43,031 | 14, frozen from beat 1 | 2,252, rising linearly |
| riscv64 | 4 | 26,469 | 27,933 | 20, frozen from beat 1 | 5,476, rising linearly |
| x86_64 | 1 | not run | 2,508 | n/a | 0, and one core is the whole reason |

**The round-trip rate was expected to fall and did not measurably**, which is recorded as a
difference from the spike rather than smoothed over: the spike saw about 30% on aarch64 and 55% on
riscv64. The cumulative totals are the cleaner comparison, since the waiters complete none of them
and the set of workers that does is identical: aarch64 did 1,153,348 round trips in 25 seconds
before and 1,151,772 after. A 25-second pair on a laptop cannot resolve a few per cent, so this
supports "no large cost" and not "no cost"; notes/soak.md carries the caveat in full.

**A production build is unchanged**, checked rather than asserted. Without the feature, on all three
architectures, every symbol and every loaded section has the same size and `ipc_fastpath` and
`syscall_entry` are unchanged at 6,687 and 1,637 bytes. The loadable image differs by 45 bytes on
aarch64, all of them panic-location line numbers below the insertion point, and rebuilding the base
commit with ten comment lines at the same place gives a byte-identical image on all three.

## BUGS

- **This does not run the experiment**, it makes it runnable. The run needs a bench evening on radon,
  argon or xenon, and that is milestone 219's other proposed follow-on rather than this one.
- **It says nothing about what a crossing rate should be.** The spike's numbers are a shape, not a
  target, and there is no baseline to compare a board against until a board has run one.
- **The hook fires on a timer, which is the one thing the workload cannot starve.** That is why it
  works, and it is also why it is not evidence about what happens without it.
