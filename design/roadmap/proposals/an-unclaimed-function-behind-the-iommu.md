# An unclaimed PCI function behind the IOMMU is refused one request at a time, not confined once

**Status: PROPOSED 2026-09-16.** Written by the milestone 303 lane (x86_64's FS disk), which put a
second `virtio-blk-pci` function on `q35`'s bus, watched the first one fault, and spent an hour
proving the fault was correct rather than a confinement gap.

**Gate: NONE.** QEMU's `q35` with `-device intel-iommu` reproduces all of it, and both `virt` boards
have the same shape with their own units.

**What the work is.** This kernel enters a function into an IOMMU domain when it *claims* it:
`virtio::register` is the only caller of `iommu::confine`, and it runs when a driver is being wired.
A function nobody claims therefore has no context entry at all, so the unit refuses each DMA it
attempts, individually, with "context entry not present". That is the right outcome reached by
absence rather than by decision. The work is to enter every enumerated function into an **empty**
domain as part of bus enumeration, so default-deny is a thing the kernel did rather than a thing it
omitted, and a later `confine` replaces the empty domain rather than creating the first one.

**What it would settle.** Whether "an unconfined device can reach RAM" is false because the kernel
says so or because nothing got round to saying anything. Today the evidence is a QEMU log line
(milestone 303 recorded `rid 0x18, code 0x2` on x86_64) rather than a test: with empty domains the
claim is assertable on all three architectures by pointing an unclaimed function at RAM and reading
the kernel's own fault register, which is the shape
`virtio::tests::the_iommu_faults_a_dma_that_escapes_the_domain` already has for a claimed one.

**Why it was not done in 303.** It changes the confinement posture on three architectures and two
IOMMU drivers (VT-d and the RISC-V unit) for a milestone whose subject was a disk. It is also not
free: a domain per idle function costs page-table frames on a real machine's fifteen-function bus,
so the sizing is part of the work rather than a detail.

**Recorded in the meantime** where a reader meets the gap:
`design/roadmap/303-x86-64-fs-disk.md`'s `BUGS`.
