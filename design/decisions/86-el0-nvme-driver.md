# 86. Whether an NVMe driver can leave the kernel, and what capability would let it

**Status: PROPOSED.** Raised by milestone 53's storage lane (2026-08-15, pull request #193), which
built the NVMe driver kernel-resident and stopped exactly here, correctly: the alternative needs
new syscall surface, and that is a boundary a lane does not cross (§10, §16).

## What is being decided

Every other DMA driver in this tree runs at EL0 behind a `Virtio` capability: the kernel owns the
queue addresses, parses each descriptor on its way to the device, and refuses any address outside
the driver's DMA region. That checkpoint exists because virtio's descriptors pass through a
doorbell path the kernel mediates.

NVMe has no such point to stand on. The controller fetches 64-byte commands directly out of
driver-written memory, and the DMA targets (the PRP fields) ride inside those commands. Nothing
kernel-side sees a command on its way past, so the `Virtio` capability's design does not transfer;
a confined EL0 NVMe driver needs a different contract. The question: does the NVMe driver stay in
the kernel, or does the syscall surface grow whatever lets it leave?

## The options

1. **Stay kernel-resident, with the IOMMU as the whole confinement story.** This is what is built.
   `bring_up` confines the controller's requester id to its six-page DMA region before enabling it,
   and on both `virt` machines an unconfined controller cannot fetch its first command, so
   forgetting the confinement fails loudly. The cost is architectural: the driver joins the TCB,
   against the microkernel thesis that drivers are user programs (§21, §23 spent real work getting
   virtio out).
2. **A kernel-owned admin plane.** The kernel keeps the admin queues (queue creation names physical
   addresses; that is the dangerous authority) and hands EL0 a capability to a pre-built I/O queue
   pair whose rings and data buffers all live inside the driver's own confined DMA region. The
   IOMMU bounds every fetch and every transfer to that region, so the kernel never parses commands;
   it only guarantees the geometry inside which any command is harmless. This is the `Virtio`
   capability's *shape* (kernel owns setup, EL0 owns the data path) with the IOMMU replacing
   per-descriptor validation.
3. **Delegate the whole controller.** Map BAR0 into the driver and let the IOMMU alone bound it,
   admin queues included. Simplest surface, but the driver can then re-point queues anywhere inside
   its region and the kernel cannot name which pages are rings versus data; and BAR0 contains every
   doorbell including the admin pair, so revocation semantics get murky. Recorded because someone
   will propose it, not because it is recommended.

## Recommendation

Option 2, **but not yet**. The recommendation is to hold this PROPOSED until the board-side work
(the JH7110's PLDA XpressRICH root complex, tracked as milestone 163, NOT-STARTED) exists, for the
same reason §23 built multi-queue confinement only after single-queue worked: the second data point
tells you which parts of the contract are QEMU artifacts. A second, possibly faster path to the same
data point may exist through milestone 87's x86 machine instead; see milestone 163's own text for
why that does not make 163 unnecessary. Option 1 is honest in the meantime because the limitation is recorded
in notes/nvme.md's BUGS section rather than implied away.

**A scoping lane has now reported on x86_64 (milestone 165, PARTIAL), and it is half an answer
rather than a whole one.** Milestone 165 wired real PCI enumeration on x86_64 through ACPI's MCFG
and proved it under QEMU's `q35`, which is stronger evidence than an equivalent RISC-V QEMU proof
would be, because x86's discovery mechanism (ACPI tables naming an ECAM window) is the same one
real x86 hardware uses, unlike RISC-V's QEMU-only fake ECAM device. That closes the *discovery*
half of what a real data point needs. It does not close the *confinement* half this decision turns
on: x86 has no IOMMU driver at all (VT-d, roadmap item 6 of milestone 161, unbuilt), so a real
NVMe-behind-IOMMU controller cannot yet be brought up confined on either machine this tree targets.
This stays PROPOSED, pending either the JH7110 board bring-up or VT-d, whichever lands first. (A
separate lane was scoping a dedicated milestone for the JH7110 driver as this report was being
written; check the roadmap index for its number before citing it.)

**The VT-d half has now landed and been exercised against this driver (2026-08-25), and the
*confinement* gap this section named is closed on x86_64.** VT-d itself landed earlier this
session (milestone 161 item 6, `kernel/src/arch/x86_64/iommu.rs`) but had confined no real PCI
device; this is that exercise. `scripts/qemu-runner-x86_64.sh` now attaches `-device nvme` the same
way the aarch64 and riscv64 runners do (no `iommu_platform` flag, same as the other two, since a
real PCI device's DMA is not virtio's opt-in), and
`kernel/src/nvme.rs::tests::the_nvme_disk_serves_the_block_interface_end_to_end` now runs for real
on all three architectures instead of skipping on x86_64. The result: **the confinement claim
holds under VT-d exactly as it does under SMMUv3 and the RISC-V IOMMU.** The existing driver,
unmodified in shape, enumerates the controller over ACPI's real MCFG, confines its requester id to
its six-page DMA region before enabling it, and serves SIZE/WRITE/READ over the blk-IPC verbs, on
QEMU's `q35` with `-device intel-iommu`.

Getting there took two fixes, neither of which is a confinement-contract difference between VT-d
and the other two IOMMUs, and both recorded in full in `notes/nvme.md`'s new "What x86_64 needed
that the other two did not" section:

- `kernel/src/pci.rs::place_bars` trusted any nonzero BAR as already placed and already mapped,
  true on the two device-tree architectures (nothing runs before this kernel to place one) and
  false on x86_64's PVH boot, where QEMU resets the NVMe device's BAR0 to a live address of its own
  choosing, unrelated to the kernel's own mapped BAR window. Fixed to check the existing address
  against the window this kernel actually mapped rather than against zero.
- The frame allocator (`memory::bring_up_page_frames`) and the direct map
  (`arch::x86_64::mmu::map_firmware_regions`) both sized themselves from the e820 map's `usable`
  RAM entries alone. Attaching VT-d and NVMe together grows the ACPI tables QEMU parks above the
  top of guest memory enough that the adjacent `reserved` entry swallows the last few hundred bytes
  of the initrd (placed by the PVH loader at a fixed offset below the top of memory, sized for a
  smaller device set). Fixed to size the bitmap from whatever `forbidden` reaches past RAM's own
  end, and to map the initrd's recorded bounds explicitly regardless of how the memmap classified
  them.

Both are the kind of gap only a real, non-virtio DMA device on a real PVH boot could have found
(virtio's BARs and a bare boot's initrd never got close to either edge), and both would in
principle also bite a real UEFI x86 machine (milestone 87), which is why neither fix is
architecture-conditional: they check against what is actually true (which window is mapped, which
bytes the kernel has claimed) rather than against which architecture is running. Neither changed
the driver's shape, the syscall surface, or the confinement contract itself.

**With this data point in, the two data points §86 was waiting on are the JH7110 board and the
VT-d/NVMe exercise; the second is now done.** This section still does not decide option 1 versus
option 2; it reports that the confinement claim itself has now been checked, not merely built, on
all three of this tree's targeted architectures.

If the answer is option 1 permanently, nothing is blocked, and notes/nvme.md's BUGS entry becomes
the standing record. If it is never decided, the driver silently becomes load-bearing kernel code,
which is how a microkernel stops being one; that is the failure this entry exists to prevent.

## What is blocked until it is answered

- An EL0 `nvme_server` program (the block-server shape `fs_proto::blk` already speaks).
- `block_roster` growing an NVMe transport kind (the surveyor cannot list the NVMe disk today);
  small, but its wire shape depends on who owns the controller.
- Milestone 55's storage stack picking its final backend: it can benchmark against the
  kernel-resident driver meanwhile, and those numbers stay honest if they are labeled with it.
