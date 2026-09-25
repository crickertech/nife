# Every VT-d unit translates its own devices

**Status: PROPOSED 2026-09-24.** Raised by milestone 261's bench rehearsal
(`notes/risk-6-bench-evening.md`), which found the kernel brought up the DMAR's first unit, very
likely the integrated GPU's on xenon, and changed it to bring up the catch-all instead. Name
provisional.

**Gate: NONE.** Buildable and host-testable now; QEMU presents one unit, so the multi-unit half is
proved on host tables and on xenon.

## What it is

`kernel/src/arch/x86_64/iommu.rs` drives exactly one DRHD. Since 2026-09-24 it is the segment's
`INCLUDE_PCI_ALL` unit (`machine_discovery::acpi::DmarUnits::translating`), and
`iommu::scope_of(rid)` reports whether a device belongs to it. That is right for fatal risk 6's
experiment if xenon's DMAR matches the Skylake OptiPlex 7040's (graphics unit first, catch-all
second), and wrong for any machine where a device this kernel drives sits under a non-catch-all
unit.

The work: bring up every unit `DmarUnits` records, keep one root table per unit, and route each
`attach` to the unit `DmarUnits::owner` names. Map every RMRR as an identity region in the owning
unit before translation turns on, because firmware declares those for DMA it expects to keep
doing (USB legacy emulation, the GPU's stolen memory) and default-deny faults it today.

## Why it is not in milestone 261 (the NVMe driver leaves the kernel)

It is not needed for the bench evening if the first preflight passes, and whether it passes is
exactly what the evening reads. If it fails with `drhd X owns it, but this kernel translates Y`,
this becomes the next piece of risk 6's critical path.

## What proves it

A host test over a two-unit DMAR asserting each device's context entry lands in its owner's root
table; under OVMF, the existing suite unchanged; on xenon, the tour listing both units up and the
bench boot's first preflight passing with the NVMe owned by whichever unit the DMAR names.
