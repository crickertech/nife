# Two cores is not a different number for the x86_64 bench, it is a different instrument

**Status: PROPOSED 2026-09-23.** Raised by the lane that explained CI's `bench (icount regression
tripwire)` failure on milestone 315 (a port revoke that reaches every core). The evidence is in
notes/benchmarks.md, the 2026-09-23 section; this block is the work that evidence leaves behind.

**Gate: NONE.** No syscall surface, no dependency, no hardware.

## What happened

Milestone 315 flipped `scripts/qemu-runner-x86_64.sh`'s `NIFE_SMP` default to 2, per DECISIONS §153
(how a two-core x86_64 test earns its place).
`bench_x86` was the one architecture arm that never pinned its own core count, so the icount bench
silently started measuring a two-core machine while both its `eprintln!`s still said "single hart",
and five counters left the 10% tripwire, four of them faster. That arm is pinned to 1 now, in the
same shape `bench()` and `bench_riscv` have always used, and the baseline did not move.

## Why a two-core baseline cannot simply be saved

At two cores the counts stay **deterministic** (three runs of one binary, byte-identical on every
row) and stop being a **function of the code**:

- Moving one `#[cfg]`-gated call of roughly fifteen instructions inside a lock, alone, on `main`,
  moved `tss_iomap_lazy_switch` +89% and `yield_switch` +15%.
- Adding two `fetch_add`s for a measurement probe, which changes no semantics at all, moved
  `spawn_reap` -42%.

A 10% tolerance over that fires on unrelated changes forever, which is the failure the tripwire
exists to prevent rather than to cause.

The cause is not the instrument's determinism but the benchmarks' shape. `bench::spawn_reap`'s
parent **busy-yields** until the reaper runs: 22 spins over 64 iterations at one core, 4,205 at
two, because the child is now placed on the other core. What that measures is who won a race, not
what a spawn costs.

## What this asks for

1. **Benchmarks whose wait loops are not races.** `spawn_reap` is the worked example; every bench
   that spins on another thread's progress has the same defect at more than one core.
2. **A separate baseline file per core count**, not a second set of rows in
   `bench/baseline-x86_64.txt`, since `--check` has no way to say which configuration a row is for.
3. **A tolerance chosen from measured spread**, not inherited from the single-core 10%.
4. Until then, the two-core numbers are available by hand (`NIFE_SMP=2 script/bench --x86`) and are
   not gated. That is a loss of nothing: x86_64 was single-core in this bench from the day the arm
   was written.

## What else was considered

- **Re-save the baseline at two cores.** Refused: it would bless numbers that a semantically empty
  change moves by 42%, and the next lane to touch the scheduler would inherit a red tripwire with
  no defect behind it.
- **Widen the tolerance to cover the spread.** Refused: the measured spread is ±89% on one row, and
  a tripwire that loose detects nothing.
- **Leave `bench_x86` inheriting the runner default and accept the flip.** Refused: it is the only
  arm of three that did so, its own output already claimed otherwise, and the inheritance is how a
  runner change with nothing to do with benchmarking became a benchmark failure.
