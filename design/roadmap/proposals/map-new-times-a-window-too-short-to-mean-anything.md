# `map_new` times a window too short to mean anything

**Status: PROPOSED 2026-09-21.** Found by the lane attributing the x86_64 `map_new` regression,
which established that the whole of a 26.4% benchmark failure was a fixed 47,752-tick lump landing
inside the timed window, and nothing at all on the path the row claims to measure. The measurements
are in notes/benchmarks.md under this date; this is the fix they hand off.

**Gate: `script/bench --x86 --check`, and the two legs beside it.** Changing `MAP_ITERS` moves the
row on all three architectures, so this re-saves three baselines and is a naming-and-numbers change
calef has to approve rather than one a lane lands quietly.

## The defect, in one table

| `MAP_ITERS` | `schedule()` spelled one way | spelled the other | delta | reads as |
|---|---|---|---|---|
| 64 (what ships) | 180,604 | 228,356 | 47,752 | **+26.4%, red** |
| 192 | 529,532 | 577,284 | 47,752 | +9.0%, green |

The two spellings are semantically identical and the map path's object code is byte-identical
between them. The marginal cost of one map is 2,726 ticks in both. The delta is a constant, so what
`map_new` reports is dominated by a term that has nothing to do with mapping, and the 10% tripwire
fires or does not depending only on how many iterations the row happens to run.

`map_new` is 180,604 ticks. The next smallest row that is not `spawn_reap` is `ipc_rtt` at
17,252,344, ninety-five times larger. This row is the only one short enough for a few preemptions
to be a quarter of it.

## What to do, and the order to consider it in

1. **Take the lump out of the window.** The right fix if it can be had: a benchmark that times a
   map loop should not be timing the scheduler. Whether the window can exclude preemption at all is
   the open question, and answering it is most of this work. `-icount` gives every vCPU one shared
   virtual clock, so a timed window that disables interrupts is a different measurement rather than
   a cleaner one, and that trade has to be made explicitly.
2. **Failing that, make the lump small relative to the row.** Raising `MAP_ITERS` from 64 to 192
   takes the same disturbance from 26.4% to 9.0%; 512 would take it under 3%. This is cheap, and it
   is the option that is honest about being a mitigation: the lump is still in there.
3. **Whatever is chosen, print the marginal cost.** `(ticks(2n) - ticks(n)) / n` is 2,726 on both
   builds where the total differs by a quarter. A row that reported the marginal number would have
   been flat through this entire episode. That is the measurement the benchmark was always trying
   to make.

Option 1 is the elegant one and option 2 is the cheap one, and this proposal deliberately does not
recommend between them, because the answer depends on whether a preemption-free window is
achievable under `-icount` and nobody has established that.

## Why this is not urgent and should still happen

Nothing is currently mismeasured on `main`: the committed baseline of 180,604 reproduces exactly.
The cost is paid by the next person, who will read a red `map_new` as a regression in the mapping
path and go looking for it in the mapping path, which is where this proposal's lane spent most of
its time before the iteration sweep ruled it out.

## BUGS

- **The composition of the lump is inferred, not counted.** `47,752 / 9,594` is 4.98 `yield_switch`
  iterations, which is why "about five preemptions" is the reading. Nothing counts them directly,
  and a lane taking this should instrument rather than inherit the arithmetic.
- **Only the x86_64 leg has been swept this way.** `map_new` is 64 iterations on all three
  architectures, so the same fragility is likely on aarch64 and riscv64 and is unmeasured here.
