# RISC-V `map_new` and the RFENCE probe

*An appendix to [`notes/benchmarks.md`](../benchmarks.md), which carries the current numbers and is written so a reader can act without opening this file. This one holds the 2026-08-15 `map_new` +15.6% and the 2026-08-17 probe that refuted its first reading, with the dates, tables and corrections behind them. Name provisional (`notes/benchmarks/` and this stem), minted 2026-09-24 by the lane that split the note; naming is calef's.*

## 2026-08-15: `map_new` moved 15.6% on RISC-V, and the movement was a bug rather than a cost

Every section above this one is about the instrument measuring a *cost*: a correctness fix that had
to be paid for, or a win it could not see. This is the first time it caught a **defect**, and it did
so on a benchmark nobody was looking at, in a change whose entire test suite was green.

Four pull requests from the VisionFive 2 lane family failed `script/bench --riscv --check` on the
same benchmark by the same amount, and a fifth from the same stack did not:

| tree | riscv64 `map_new` | note |
|---|---|---|
| `main` | within `2362 ± 236` | its push run executed the riscv leg rather than skipping it |
| #172, #173, #175, #176 | **2731 to 2732** (+15.6%) | tolerance is `±236`, so this fails by a wide margin |
| #178 | **2362, exactly** | `kernel/src/smp.rs` byte-identical to #176's |

The stack's change routes every per-cpu loop through `smp::online_cpus()` and masks the RISC-V
RFENCE calls with `online_harts_mask()`, so a machine whose online set is `{1,2,3}` stops being
indexed as `0..count` (see notes/visionfive2.md and `crates/cpu_set`). **The bench boots a single
hart.** On one hart the online mask names one cpu, the shootdown has nobody to send to, and the
correct instruction count for the masked version is the count for the unmasked one. 2362 is the
right answer and 2731 is not.

### The first reading was wrong, and it is kept because it is the plausible one

+370 ticks over 64 iterations is **5.8 instructions per map**, which is exactly the shape of "consult
a mask instead of a count on every TLB flush". That reading says the regression is the price of the
correctness fix, and it leads directly to the action `script/bench` itself recommends on failure:
rerun with `--save` and commit the new baseline with the change that moved it.

What refuted it is the third row of the table. **#178's `kernel/src/smp.rs` is byte-identical to
#176's**, and #178 measures 2362. A cost carried by the code cannot be absent from a tree that
contains the same code. So the extra instructions are not the mask being consulted; they are work the
mask should have prevented and did not, on the trees that lack the later registration fix. The best
available reading is remote RFENCEs being issued on a single-hart machine, from a mask that
over-reports which harts are online.

**Rebaselining would have been the expensive mistake, and it was one command away.** It would have
written 2731 into `bench/baseline-riscv64.txt`, and every future run of the tripwire would then have
been silent on precisely the defect it had just caught. That is worse than never having had the
check: a green tripwire is read as evidence, and this one would have been evidence of nothing.

### What is proven, what is inferred, and what would settle it

**Proven.** The four trees measure 2731 to 2732 and #178 measures 2362, against a baseline `main`
still satisfies. The `smp.rs` files are identical. All of it is from CI's own runs rather than
computed or scaled, per the discipline the 2026-08-04 riscv64 re-save established.

**Inferred.** That the delta is remote RFENCEs fired against an over-reporting mask. Nothing here
counted a fence. The evidence for it is that the number returns to the baseline **exactly** rather
than approximately, which is what a path that stops executing looks like and not what a cheaper path
looks like.

**What would settle it**: count RFENCE issues on a single-hart boot, or print `online_harts_mask()`
at bench time on both trees and compare. Neither is built, and this is recorded as a reading rather
than a mechanism until one of them is.

### Why this is the strongest argument yet for the job's wall-clock

Nothing else noticed. `build + test (host + QEMU)` passed on #176. So did `cpu matrix (riscv64
across QEMU CPU models)`, which exists specifically to boot RISC-V across CPU models. The kernel
worked; it just did more than it needed to, on the one configuration where the extra work is
provably unnecessary. A correctness bug that leaves behaviour correct is invisible to every test in
the tree by construction, and an instruction counter is the only instrument here that can see it.

Milestone 21's stated purpose for the tripwire is catching "the *introduction* of performance
problems proximate to the changes that introduce them". This is the same mechanism catching
something better, and the case is worth citing the next time the job's five minutes come up for
debate.

## 2026-08-17: the RFENCE probe was built, and it refuted the reading above

The section above ends by naming what would settle its inference: "count RFENCE issues on a
single-hart boot, or print `online_harts_mask()` at bench time on both trees and compare. Neither is
built, and this is recorded as a reading rather than a mechanism until one of them is." Both are
built now (`arch::riscv64::remote_fence_count`, the `bench-probe:` lines beside `map_new`, and
`rfence_self`), and the answer is not the one the reading predicted.

### What the probe measures

| tree | `map_new` | remote fences in the timed window | `online_harts_mask` | `rfence_self` |
|---|---|---|---|---|
| `main` (53ca491) | 2362 | **0** | `0x1` | 9.81 ticks/call |
| pre-fix (593c00e) | 2361 | **0** | `0x1` | 8.91 ticks/call |
| **PR #176's head** (f601c6b) | **2362** | **0** | `0x1` | 8.91 ticks/call |

The third row is the one that matters. **That is the tree CI measured at 2731**, fetched from
`pull/176/head` and run with the probe cherry-picked onto it, and here it measures the baseline
exactly, issues **zero** remote RFENCEs across `map_new`, and reports a mask with one bit set. The
mask does not over-report and there are no fences to be the cost.

### The arithmetic error that made the wrong reading plausible

The section above computes "+370 ticks over 64 iterations is **5.8 instructions per map**". **A tick
is not an instruction.** The bench counter is `rdtime` at the machine's timebase, which on QEMU's
`virt` is ~10 MHz, and `-icount shift=0` makes one instruction one nanosecond of virtual time. So
one tick is **~100 instructions**, and the delta was ~577 instructions per map, not 5.8.

That kills the reading the section called "the plausible one" from the other direction than it
thought. Consulting a mask instead of a count is a handful of instructions, which is ~0.05 ticks and
invisible at this resolution; it was never a candidate for a 5.8-tick move. The section rejected it
for the right reason (a cost cannot be absent from a tree containing the same code) while its stated
arithmetic was two orders of magnitude out.

### What one RFENCE actually costs, and why it does not fit either

`rfence_self` prices the firmware path by naming this hart in the SBI hart mask, a legal call the
firmware serves with a local fence. It costs **8.9 to 9.8 ticks**, so one extra remote RFENCE per
map would have moved `map_new` by ~570 ticks over 64 iterations. The observed move was 369, or
**0.65 fences per map**, which is not a whole number of anything.

### What is proven, what is not, and what to run next

**Proven.** Three trees, including the accused one, issue zero remote RFENCEs during `map_new` and
carry a correct single-bit mask. One RFENCE costs ~9 ticks. The 2026-08-15 inference, that the delta
was remote RFENCEs fired against an over-reporting mask, **is refuted for PR #176's tree.**

**The confound, and why it turned out to be small.** The three runs above are on **QEMU 8.2.2 with
`-device riscv-iommu-pci` removed**, because the QEMU available in that container is not the pinned
11.0.2 and does not implement the device. CI measured 2731 on 11.0.2 with the IOMMU present, so the
worry was that the regression is a property of the machine rather than of the branch.

**CI has since run this probe on the pinned machine and the worry mostly dissolves.** On QEMU 11.0.2
with the IOMMU present, `main` reports `map_new_remote_fences 0`, `online_harts_mask 0x1`, and
`map_new 2362`.

The interesting part is the pair of numbers either side of that machine change:

| | container (8.2.2, no IOMMU) | CI (11.0.2, IOMMU) |
|---|---|---|
| `map_new` | 2362 | **2362** |
| `rfence_self` | 8.91 ticks/call | **11.70 ticks/call (+31%)** |

**`map_new` is bit-identical across the machine change while the RFENCE benchmark moves 31%.** That
is a cross-check nobody designed and it is the strongest single piece of evidence here: a benchmark
made entirely of SBI calls is visibly sensitive to the firmware and emulator, and `map_new` is
completely insensitive to them, which is what a path that makes **no SBI calls at all** looks like.
The fence counter says zero and the machine-sensitivity says zero independently.

So the container's reading of `pull/176/head` at 2362 is very unlikely to be an artifact of the
machine, and the refutation stands rather than being provisional on it.

**What is now genuinely unexplained.** Not the mechanism, but the observation: what CI measured at
2731 to 2732 on four branches, reproducibly, in August. Nothing in this section explains it, and
there is no live hypothesis left. The remaining experiment that would speak to it is the probe run
against `pull/176/head` **in CI**, on the pinned machine, which is a thing a lane can do deliberately
and nothing does by accident. Until then the 2026-08-15 section's diagnosis should be read as
withdrawn rather than replaced.

### The part of the old section that survives intact

**Rebaselining would still have been the expensive mistake.** Everything above changes what the
delta *was*; nothing changes that writing 2731 into the baseline would have silenced a real
difference nobody had explained. The tripwire's value here was never the diagnosis, which was wrong.
It was that the number moved, refused to be quiet about it, and stayed unexplained until somebody
measured. That is what a tripwire is for, and it is the reading in the section above, not the
instrument, that this correction lands on.
