# 144. The fastpath footprint gate gets a delta and a ceiling, and the ceiling is 16 KiB

**Status: DECIDED.** calef, 2026-09-04, after three failures of the stored-baseline design in three
days. *(Number provisional until the merge queue lands it.)*

## What is being decided

`script/fastpath-footprint` compares the IPC fastpath's code size against a **stored baseline file**
per architecture and fails at a 5% tolerance. The proposal
`design/roadmap/proposals/fastpath-footprint-against-main.md` asked whether it should instead compare
against `main` at pull-request time. **The answer is both, and the reason each is needed is that they
catch different failures.**

## The shape

**A delta check against `main`.** No stored state, so nothing can go stale and nobody has to remember
to re-record. This replaces the baseline files.

**Plus an absolute ceiling of 16 KiB**, per architecture, on the sum of `ipc_fastpath` and
`syscall_entry`. Only calef raises it.

## Why a delta alone is not enough

**The gate is named for an absolute property**, and its own header says so: *"the IPC fastpath must
stay small enough to live in L1i"*, from Liedtke's *On micro-Kernel Construction* (SOSP 1995), which
argued Mach's IPC was slow because of the cache footprint of its hot path rather than anything
inherent to microkernels. **A delta cannot express that claim.** It answers "did this change grow it",
never "is it still small enough".

Its failure mode is also silent and is the mirror of the stored baseline's: every individual change
is within bound while the absolute size grows without limit, because the reference moves with the
tree.

## Why a stored baseline alone is not enough, and this is now measured three times

- **Milestone 237.** Two lanes each measured "within bound" against the same stale baseline and
  neither re-saved it, so aarch64 headroom fell from 3.9 points to 1.5 **with the gate green
  throughout**.
- **2026-09-04, the nightly bump.** `nightly-2026-09-04` inlined `drivers::plic::disable` into
  `riscv_trap_body` and the entry set grew 194 bytes with **no source change anywhere to attribute
  them to**. A delta against `main` would have made that a non-event, because both sides compile with
  the same toolchain.
- **2026-09-04, milestone 220.** Adding a dependency to the `kernel` crate flipped an inlining
  decision the other way and `syscall::dispatch` (1,160 bytes) vanished from the symbol table. The
  gate failed at **35.1% shrinking**, and the script's own advice for a shrink ("just an explicit
  acknowledgment") would have re-recorded an under-measurement as the new truth.

And the drift is not one architecture's: measured with the compiler held fixed, `ipc_fastpath` sits at
**+0.6% aarch64, +0.3% riscv64, +1.9% x86_64** against stored baselines, so **aarch64 has re-drifted
within a week of milestone 237 attributing it and re-recording 5852**.

## Where 16 KiB comes from, because a number without a derivation is a guess

notes/target-hardware.md's L1i requirement gives it. The binding constraint is **32 KB**, which is
radon's SiFive U74 and xenon's Core i5-7500T, and is where frontier x86 has sat for two decades
because a virtually-indexed L1 is capped at page size times associativity and the ecosystem's page is
4 KB. **16 KiB is half of that**, the point that note names as where Liedtke's argument genuinely
weakens: at a quarter of L1i the hot path leaves the application three quarters, and at half it does
not.

Measured today, the totals are **aarch64 9,156 B, x86_64 8,404 B, riscv64 7,174 B**, so the ceiling
sits about 75% above the largest and fires rarely. **A tighter number was rejected for a specific
reason**: 8 KiB, a quarter of 32 KB, was floated and would have been **red on the day it shipped**,
since two architectures already exceed it. A ceiling that fails on arrival is a ceiling nobody trusts.

## What each half is for

| | catches | misses |
|---|---|---|
| delta against `main` | a change that grows the path | slow accumulation, since the reference moves |
| 16 KiB ceiling | accumulation, whatever caused it | a single change that is large but still under |

Neither is redundant, and 2026-09-04 demonstrated both failure modes within one day.

## BUGS

- **The measurement is an upper bound and a loose one.** Whole symbol sizes, so a cold tail inside a
  hot function counts. The bytes an IPC actually touches are fewer and nobody knows by how much, so
  16 KiB is a ceiling on a pessimistic figure.
- **`syscall_entry` is not comparable across ISAs.** x86_64's `syscall` goes through `IA32_LSTAR` and
  never touches the IDT, so its entry figure legitimately excludes about 3,330 bytes the other two
  include. The ceiling is applied per architecture for that reason and must not be read as a ranking.
- **Nothing has observed the effect either half protects against.** No cache is modelled by icount and
  the development host's L1i is several times the boards'. This is a 1995 argument plus a measured
  code size, not a measured miss rate, and the experiment that would settle it wants the performance
  counters milestone 74 landed on radon on 2026-09-04.
- **A delta against `main` costs a second build per run.** Whether that is material on the merge
  queue's critical path is measurable and has not been measured; the prover is already the long pole.
