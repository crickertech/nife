# Run the footprint-perturbation experiment on radon, where a cycle counter can see it

**Status: PROPOSED 2026-09-04.** Found by the milestone 188 lane (phases 1 to 3), which measured the
IPC fastpath's footprint three ways and could not observe a single cycle of consequence.

**Gate: MILESTONE 134.** E3 is milestone 134's experiment and that block owns the register of
measures; this proposes running it on hardware rather than minting a new measure. It also wants
milestone 74's riscv64 half, which landed 2026-09-04.

## In brief

`script/fastpath-footprint` bounds a **quantity**, not a harm. Liedtke's argument is that a kernel
touching a lot of memory per IPC evicts the *application's* working set, so the bill arrives as
capacity misses spread through the workload. Nothing in this tree has ever observed that: icount
models no cache, and the HVF development host's L1i is several times the boards'.

Milestone 134's E3 was the experiment built to test it without a PMU. It runs the kernel with
`--features fastpath_pad`, which pads the fastpath with `nop` to roughly double its footprint, and
measures what happens. Run under icount on 2026-08-22 it reported **2 to 3% latency effect for a 2x
footprint growth**, which is what a tripwire that cannot see a cache can see.

**Radon can now see cycles.** Milestone 74's riscv64 half landed 2026-09-04
(`kernel/src/arch/riscv64/pmu.rs`, the SBI PMU extension), on a SiFive U74 with a **32 KB L1i**,
which is the binding constraint DECISIONS §144's 16 KiB ceiling was derived from. E3 on that machine
is the same experiment with an instrument that can answer.

## Why it is worth a lane rather than a paragraph

**It is the measurement that decides milestone 188 phase 4**, a hand-written IPC fastpath, which is
a standing verification obligation and a permanent maintenance cost. That block's own recommendation
is to wait for exactly this. Phases 1 to 3 established that the cheap methods leave the shape the
system runs 48 to 103% over the 4 KiB target, so the arithmetic case for phase 4 is as strong as it
will get and the empirical case does not exist.

Two outcomes, both useful. If a 2x padded fastpath costs nothing measurable in cycles on a 32 KB
L1i, **fatal risk 4 gets its best evidence yet** and phase 4 should be refused in writing. If it
costs something, phase 4 has a number to be measured against for the first time.

## What it is not

**It is not the full Liedtke experiment.** A cycle count says whether the round trip got slower; it
does not attribute the cost to instruction-cache displacement, and it says nothing about the
*application's* working set, which is Liedtke's actual claim. That wants cache-miss events
attributed across a workload, which is milestone 134's tier B and milestone 127's silicon. This
proposal is the cheap half that can run today, and it should say so wherever it reports.

## What it needs first

- `kernel/src/arch/riscv64/fastpath_pad.rs` exists, so `--features fastpath_pad` works on radon.
  (x86_64 has no such module, which holds xenon out; recorded in `script/fastpath-footprint`'s
  BUGS.)
- The board rig: radon's UART into cordoba, smart plug 2. See notes/target-hardware.md.

## Where it came from

`design/roadmap/188-ipc-fastpath.md`'s Follow-on. `design/roadmap/134-the-measurements-that-decide.md`
owns E3; `design/roadmap/132-the-fastpath-footprint.md` owns the gate.
