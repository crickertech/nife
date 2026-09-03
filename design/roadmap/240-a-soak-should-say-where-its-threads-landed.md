# 240. The soak reports what happened and not where, so an eightfold difference cannot be explained

**Status: NOT-STARTED.** Minted 2026-09-03 by calef, during the first bench session that ran the soak
on real silicon. *(Number provisional until the merge queue lands it.)*

**Gate: NONE.** The kernel knows the answer at spawn; nothing prints it.

**In brief.** Two soak runs on **radon**, from the same card and the same build, twenty minutes apart:

| | first beat | rate | crossings at beat 12 |
|---|---|---|---|
| 13:04 | `rounds=918313` | **183,662/s** | ~3,000 |
| 13:24 | `rounds=112960` | **22,592/s** | 47 |

**Eightfold, and the machine was identical.** The boot tour's own pure-compute check ran 6,904,828
and 7,271,375 iterations in the first run against 6,831,327 and 7,288,574 in the second, over the
same fixed window, with 82 preemptions both times. Same clock, same four cores online, same timer,
same firmware. So the CPU is not throttled and the difference is in the workload, not the machine.

**The explanation is almost certainly placement**, and milestone 221 (the soak never crosses cores,
so build the hook that makes it) already predicted its shape: its `BUGS` records that the crossing
count varies by more than 2x between identical runs and names the boot-time placement lottery as the
cause. Twenty-four threads (four groups of a responder, three callers, a grinder and a tick waiter)
are placed across four cores at spawn, **and nothing rebalances**, which is milestone 219's (the boot
tour ends and the kernel halts, so there is nothing to soak) central finding and a deliberate design
following DECISIONS 138 (how a saturated workload is made to hand threads across cores). A core that
draws two grinders starves its IPC threads, because a grinder is pure compute and never yields.

**Almost certainly is not good enough**, and that is this milestone. The soak prints round trips,
wakes, crossings, remote placements, steals and deferrals. **It does not print where any thread is.**

## What it needs

**A per-core census at soak start**, printed once: which roles landed on which core. Enough that a
reader of a log alone can say whether that boot drew two grinders on one core, without having the
board.

That turns a series of boots from *"the rate varies"* into *"the rate varies with this placement"*,
which is the difference between an observation and a measurement.

## Why it is worth building before the next bench session rather than after

**Tonight's plan is a series of boots rather than one long run**, on two independent arguments that
arrived the same afternoon: this eightfold spread, and the literature that
`notes/soak.md` now records, where stress coverage saturates and independent runs multiply while one
long run does not.

**Without the census those boots produce a distribution nobody can attribute.** With it, every log
explains itself, and the question of whether thread affinity is worth its syscall surface becomes
answerable from data rather than from argument.

## What this is not

**It is not a rebalancer.** DECISIONS 138 declined one, on prior art that no capability microkernel
read for it rebalances at all, and nothing here reopens that.

**It is not an argument for thread affinity yet.** Two boots are two boots. The census is what would
let a series of them make that argument, or refuse it.

## BUGS

- **The placement explanation is an inference.** It fits the numbers, it fits milestone 221's own
  prediction, and it has not been confirmed, which is precisely why this milestone exists.
- **A census at start says nothing about what happens after**, and threads can move: milestone 221
  added `Thread::last_cpu` and a migration counter for exactly that reason. Whether the census should
  also be reported at the end is a real question this block does not settle.
- **It adds output to a run whose output is already dense**, and `script/board-console` has to keep
  recognising the boot; a census long enough to be useful may want its own line format.
