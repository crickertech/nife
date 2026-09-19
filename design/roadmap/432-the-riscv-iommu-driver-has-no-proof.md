# 432. The RISC-V IOMMU driver has no counterpart to the SMMU's proofs

**Status: NOT-STARTED.** Promoted from the proposal `the-riscv-iommu-driver-has-no-proof`, filed
2026-09-18 from milestone 322's confirmation pass, which went looking for an unexercised IOMMU
configuration, found that all three architectures already boot behind one, and turned up this
instead. *(Number provisional until the merge queue lands it.)*

**Gate: NONE.** The harnesses would sit beside `arch/riscv64/iommu.rs` the way
`arch/aarch64/iommu.rs`'s already do, and `script/verify` already reaches that tree since milestone
193. Nothing has to be bought, decided or ported first.

**Premise re-checked 2026-09-19 and still true, by count.**
`kernel/src/arch/aarch64/iommu.rs` carries two `kani::proof` harnesses and
`kernel/src/arch/riscv64/iommu.rs` carries **zero**. `notes/iommu.md`'s "What is proved, as against
tested" still says the RISC-V IOMMU has no counterpart, and still records that the register offsets
and bit constants are not proved and cannot be. This is the newest block in milestone 433's promotion
and the only one of the twenty-five whose premise needed no qualification at all.

**In brief.** `notes/iommu.md`, "What is proved, as against tested", names it plainly: **the RISC-V
IOMMU has no counterpart** to the aarch64 proofs. `arch/aarch64/iommu.rs` carries two Kani harnesses
over its entry-building arithmetic, both falsified before they were believed
(`the_smmu_is_handed_exactly_the_tables_the_kernel_built` and
`no_stream_can_reach_another_streams_tables`, the latter replayable in
`kernel/falsifications/`). The RISC-V side writes its device context in 64-bit stores with no split,
so that first property does not apply to it, **and nothing was written in its place**.

## Why the asymmetry is not benign

The boot-time confinement test is what stands in for a proof on both sides, and the same note says
what it cannot do: it *"proves the hardware stops an escaping device. It cannot prove the kernel
wrote the **right** entry, because a wrong entry that still confines this device on this board is
invisible to it, and this board is the only one either driver has ever run on."*

So on riscv64 that test is the **whole** of the assurance, with no proof beside it, on exactly one
board. §19 makes architectural parity a gate rather than an aspiration, and this is a parity gap in
the isolation boundary that milestone 35 calls *"the one isolation boundary we test instead of
prove"*.

## What it is not

**Not "port the aarch64 harnesses".** The property they prove is about a 64-bit physical address
surviving a split across two 32-bit words without colliding with the control bits sharing the low
word. The RISC-V device context has no such split, so a transliteration would prove a hazard that
does not exist there and miss whatever the real one is. **Finding the right property is most of this
work**, and writing it down is the deliverable even if the harness that follows is short.

**Not a claim that the riscv64 driver is wrong.** Nothing here reports a defect. It reports that one
of two rhyming drivers is checked in a way the other is not, which is the asymmetry §19 exists to
catch and the kind this tree prefers to find by looking rather than by failing.

## One limit to carry over rather than discover again

`notes/iommu.md` records that the register offsets and bit constants **are not proved and cannot
be**: nothing in this tree can check a constant against Arm IHI 0070, or against the RISC-V IOMMU
specification, so a misreading of the document makes the code and the proof wrong together. Whatever
is written here inherits that, and should say so where a reader meets it rather than implying a
completeness it cannot have.

## Index row

`arch/aarch64/iommu.rs` carries two Kani harnesses over its entry-building arithmetic, both
falsified before they were believed, one of them replayable in `kernel/falsifications/`. The RISC-V
driver writes its device context in 64-bit stores with no split, so the property those prove does
not apply to it, and **nothing was written in its place**: the boot-time confinement test is the
whole of the assurance on that side, on exactly one board, and the same note says what that test
cannot do, which is prove the kernel wrote the right entry, since a wrong entry that still confines
this device on this board is invisible to it. §19 makes architectural parity a gate rather than an
aspiration and this is a parity gap in the boundary milestone 35 calls the one isolation boundary we
test instead of prove. The work is not porting the aarch64 harnesses, which would prove a hazard
RISC-V does not have; finding the right property is most of it, and writing it down is the
deliverable even if the harness that follows is short.
