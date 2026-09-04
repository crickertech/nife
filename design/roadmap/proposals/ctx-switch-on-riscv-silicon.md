# The ctx_switch number on real RISC-V silicon

**Status: PROPOSED 2026-09-03.** Written by the milestone 247 sweep, from milestone 58's block.

**Gate: HARDWARE, DECISION.** HARDWARE because only a core with a genuinely ASID-tagged TLB can
charge the right price, which means radon, the VisionFive 2, with a person at the bench rig.
DECISION because nobody has said which board leg owns a RISC-V number: milestone 58's block points
the measurement at milestone 24, which is an aarch64 VMM board and cannot take it.

**In brief.** Milestone 58 removed the unconditional `sfence.vma` from `write_satp` behind a probe,
which is the whole reason ASIDs exist on RISC-V. The number that would show the win has never been
taken. QEMU's softmmu TLB is not ASID-tagged and flushes wholesale whenever `satp.ASID` changes, so
it charges for the added probe gate and credits nothing for the removed flush; icount came back
**+1.2% on `ctx_switch`**, which is the measurement reading backwards. Re-run `ctx_switch` on radon,
with and without the flush, and record what an ASID-tagged TLB actually saves.

## Why this matters

A milestone exists whose entire justification is a performance claim that has never been measured,
and the one measurement in the tree points the wrong way. `bench/baseline-riscv64.txt` carries
`ctx_switch 492350 5000` from QEMU, which is not evidence for or against the change. This project's
own standard is *measure, do not argue*, and an honest tie or loss recorded plainly is worth more
than a win nobody checked. Right now there is neither.

The second cost is the record. Milestone 58's block sends a future reader to milestone 24 for this
number, and milestone 24 is an aarch64 board that can never produce it. So the one pointer that
exists is wrong, and a reader who follows it wastes the trip. That miscitation only gets fixed by
somebody deciding where the number really lives.

`notes/riscv-tlb-shootdown.md` also says the board "has not arrived", which stopped being true on
2026-08-14. The blocker is no longer the hardware; it is that nobody owns the run.

## What it would take

The mechanism is built and gated: `asid_tagging_is_trusted()` in
`kernel/src/arch/riscv64/mmu.rs` is what decides whether the flush is issued, so both arms of the
comparison are already reachable from one build. What is missing is a run on radon over the bench
rig (UART into cordoba, smart plug 2), a `ctx_switch` number from each arm, and a home for the
result beside the existing baselines.

## Where it came from

Milestone 58's block: *"Take the `ctx_switch` number on real RISC-V silicon, where the TLB is
genuinely ASID-tagged. QEMU charges the added gate and credits nothing for the removed flush, so the
win this milestone exists for has never been measured, and the block's citation of milestone 24 is
wrong (that is an aarch64 board). Somebody still has to say which board leg owns the measurement."*

`notes/riscv-tlb-shootdown.md`'s BUGS section states the constraint: *"QEMU cannot exercise the case
the probe measures for the reason a real core would."*
