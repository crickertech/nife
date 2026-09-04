# x86_64 places PCI BARs in a hardcoded window that no machine has agreed to

**Status: PROPOSED 2026-09-03.** Written by the milestone 247 sweep, from milestone 165's block.

**Gate: NONE.** Both sources of truth are reachable from code that already runs: the firmware
memory map is parsed at boot, and Intel's host bridge reports `TOLUD` in its own configuration
space, which this kernel can already read. Confirming the result on xenon is `HARDWARE` of the
second kind, but the work does not start there, and `pci::bar_census` already prints the number
that would show it working under QEMU.

**In brief.** When a PCI function's BAR needs placing, this kernel places it inside
`arch::x86_64::mmu::PCI_BAR_PHYS`, a constant. Take the window from the machine instead. `_CRS` is
AML and stays refused for the reason milestone 165 gave, but the firmware memory map already names
the gaps between regions of RAM, and Intel's host bridge reports the top of low usable DRAM in
configuration space. The measure of success is `pci::bar_census`'s second number reaching zero, or
a window the machine itself agreed to.

## Why this matters

This one is not tidiness and the block says why in a sentence worth repeating: a machine whose RAM
reaches above `0xc000_0000` would have this kernel relocate most of its bus on top of memory. That
is not a device that fails to enumerate, it is a device writing into RAM the allocator believes it
owns, discovered as corruption somewhere unrelated. The constant is correct on QEMU's `q35` and
correct by luck anywhere else.

Recording it is not sufficient, which is the distinction that puts this in a proposal rather than a
`BUGS` entry. A recorded limitation is the right answer when the cost of hitting it is a clear
failure a reader can recognise. Here the cost is silent memory corruption on the first real machine
with enough RAM, and the first real machine is xenon, where AGENTS.md already notes that a null
modem is this project's most expensive place to discover anything.

## Where it came from

Milestone 165's Follow-on: *"Take the 32-bit MMIO window that BARs are placed in from the machine
rather than from the hardcoded `arch::x86_64::mmu::PCI_BAR_PHYS`. `_CRS` is AML and stays refused,
but the firmware memory map already names the gaps and Intel's host bridge reports `TOLUD` in its
own configuration space. The measure is `pci::bar_census`'s second number reaching zero, or a
window the machine agreed to. Recording it is not enough: a machine whose RAM reaches above
`0xc000_0000` would have this kernel relocate most of its bus on top of memory."*

The same block records what only xenon can confirm, and this is the first item on that list:
whether `PCI_BAR_PHYS` is free on that machine. It also refused an AML interpreter for `_CRS`, and
that refusal stands here. This is the cheaper source for the same fact.

## What it would take

Read `TOLUD` from the host bridge, cross-check it against the gaps in the firmware memory map, and
place BARs in whichever window both agree is free. Fail loudly when they do not agree rather than
falling back to the constant, which is the posture milestone 215 took for the analogous case: a
silent fallback to the old behaviour is the original bug wearing the clothes of a graceful
degradation.
