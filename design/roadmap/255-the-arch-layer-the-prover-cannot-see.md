# 255. A quarter of `kernel/src/arch/` has no assembly in it, and none of it is proved

**Status: NOT-STARTED.** Minted 2026-09-04 by calef, from `design/fatal-risks.md` risk 2's remaining
amber. *(Number provisional until the merge queue lands it.)*

**Gate: NONE.** Milestone 193 (put `kernel/src` within reach of the prover) built the machinery and
`script/verify` already runs a `kernel` row.

**In brief.** Fatal risk 2 (the proofs prove trivia, and the real bugs live where Kani cannot reach)
is **AMBER**, and its own text names what keeps it there:

> `kernel/src/arch/`, `user/` and `xtask` are still out of reach, so the amber stands

Milestone 197 closed `user/` and `xtask` on 2026-08-31. **`kernel/src/arch/` is what remains**, and
milestone 193's `BUGS` says why it matters rather than merely being large:

> `kernel/src/arch/` stays unreachable under every option here, which means the architecture layer,
> **where the VisionFive 2's undelivered-wake defect actually lived**, is not what this fixes.

## The measurement that makes this a milestone rather than a wish

Counted 2026-09-04 across `kernel/src/arch/*/*.rs`:

| | lines |
|---|---|
| `arch/` total | **16,225** |
| files containing **no** `asm!` at all | **4,004 (24%)** |

**A quarter of the architecture layer is ordinary Rust.** The blanket claim that `arch/` is
unreachable because Kani cannot model `asm!` is true of `arch/` as a directory and false of a
quarter of its lines. The largest asm-free files:

| file | lines | what it is |
|---|---|---|
| `x86_64/irq.rs` | 903 | interrupt plumbing |
| `x86_64/machine.rs` | 762 | ACPI table walking, the RSDP and XSDT decode |
| `x86_64/iommu.rs` | 368 | VT-d |
| `aarch64/iommu.rs` | 347 | SMMUv3 |
| `riscv64/iommu.rs` | 314 | the ratified RISC-V IOMMU |
| `aarch64/isa.rs` | 299 | feature and granule discovery |

## Why these lines specifically

**The three IOMMU drivers are 1,029 lines of confinement**, and `design/fatal-risks.md` risk 7 (the
confinement claim is false) rests on what they do. They build device tables and page tables, and they
are the code that decides which physical addresses a device may touch. That is table arithmetic over
integers, which is exactly what a bounded model checker is good at and what a boot test is bad at:
a wrong entry that still confines *this* device on *this* machine is invisible to every test in the
tree.


**`x86_64/machine.rs` walks somebody else's tables.** ACPI's RSDP, XSDT and MCFG arrive from firmware
and are untrusted input in the sense that matters: a length field, a checksum, a count. notes/x86-uefi-boot.md
records that under OVMF the kernel read a 118-descriptor memory map and a revision-2 RSDP where PVH
gave 9 and revision 0, so both branches are live and one of them had never executed until milestone
195.

## What this is not

**It is not a proof over the architecture layer**, and a block that promised one would be promising
what milestone 193 already refused: *"nobody should imagine a proof over the kernel"*. It is
**reachability** — harnesses can be written against these lines, one property at a time, the way they
already are for the crates.

**And it is not a licence to move code.** Milestone 193 chose option A (make the crate reachable with
the arch layer stubbed) over option B (lift pure logic into crates), and milestone 244 later declined
option B for `system_initializer` on measurement. The same discipline applies: if a property can be
proved where the code lives, prove it there.

## The proof that this milestone worked

**One property over code in `kernel/src/arch/`, proved by a harness `script/verify` runs, and
falsified before it is believed** — a real defect re-introduced and the harness turning red. That is
milestone 193's own bar and the reason its result was credible: milestone 191 found no harness in
this tree had ever caught a defect, and 193 answered by reintroducing milestone 142's real
`(count - 1).wrapping_mul(PAGE_SIZE)` and watching two harnesses fail.

Not a survey of what could be proved, and not a harness over a function moved out of `arch/` first.

## BUGS

- **A stub is a hole in a proof, and a proof with an unexamined stub reads as coverage.** Whatever
  boundary this lands on needs its stubs enumerated where a reader meets the harness, which is
  milestone 193's own recorded limitation and applies here with more force, since `arch/` is where the
  stubs are.
- **24% is a line count, not a difficulty estimate.** Some of those 4,004 lines are `const` tables and
  some are the hardest reasoning in the tree; the fraction says where to look, not how long it takes.
- **`script/verify` already costs about 42 minutes** and anything added lands on that budget. The
  `--affected-since` machinery exists because it is expensive.
- **The three IOMMU drivers are structural siblings but not identical**, so a property proved of one
  is not thereby proved of the other two, and a harness that appears to cover all three by symmetry
  would be the overclaim this milestone is meant to remove rather than add.
