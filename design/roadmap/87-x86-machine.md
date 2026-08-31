# 87. The x86_64 bare-metal machine

**Status: PARTIAL.** Raised 2026-08-03. The selection is made and recorded here, and as of
2026-08-30 **the software side is done and proved under real firmware in QEMU**; the milestone
completes when the machine on the desk has printed a byte over serial, which only calef can do.

**Gate: HARDWARE.** It is now the only gate. The hardware side finished 2026-08-23 (the
OptiPlex arrived 2026-08-15; the Dell C4PDJ serial module and the dev-side RS-232 chain arrived and
are installed). The *software* blocker closed 2026-08-30: the kernel could not be started by any
real firmware at all until then, because the x86_64 port boots by PVH, a hypervisor direct-boot
protocol no machine speaks. That is what "What was built" below fixed.

**What remains is one person, one USB stick and a serial console**, and the procedure is written
out step by step, with a failure-triage table, in notes/x86-uefi-boot.md's "The bench" section. It
is deliberately one file to copy: `target/esp/EFI/BOOT/BOOTX64.EFI`, which `cargo xtask uefi-image`
writes and which carries the kernel and the userspace archive inside itself.

## What was built (2026-08-30)

**A UEFI entry, chosen over GRUB Multiboot 2 on a fork this lane priced rather than argued.** Both
were real and both could coexist; two commands decided it. OVMF, the open-source UEFI
implementation, **ships with the QEMU this project already pins**
(`/opt/homebrew/share/qemu/edk2-x86_64-code.fd`), and QEMU's `vvfat` driver synthesises the FAT
filesystem out of a host directory, so the whole path is testable today with nothing installed.
GRUB is not installable on the development machine at all (`brew info grub`: no formula), so that
path could have been written on patagonia but not *proved* there. The OptiPlex is also UEFI-native,
so UEFI is the shorter path at both ends. GRUB stays cheap to add for a BIOS-only machine.

**The kernel is not modified**, and that is the design rather than an economy. `uefi_loader` places
the kernel at its `p_paddr`, synthesises an `hvm_start_info` out of what the firmware knows, leaves
long mode, and enters **the same `_start`** with the same register contract QEMU's PVH loader
delivers. One entry point, one handoff structure, one decoder, one set of tests; two of each would
have diverged, and the divergence would first show up on hardware nobody can attach a debugger to.
It also means `script/test --arch x86_64` cannot regress under it, which was this milestone's
sharpest hazard.

**Proved under OVMF, and it exercised four paths that had never run**, because a hypervisor never
takes them: the ACPI root pointer arrives non-zero (so the BIOS-area `"RSD PTR "` scan is skipped),
it is revision 2 with an **XSDT** root rather than revision 0 with an RSDT, the MCFG's ECAM window
is `0xe0000000` where the hardcoded constant says `0xb0000000` (so "read the table" is finally
distinguishable from "used the constant", which milestone 165 could not show), and the memory map is
**118 regions** against PVH's nine. The userspace archive arrives too, through the module list the
loader writes.

`cargo xtask uefi-boot` gates it and runs inside `script/test --arch x86_64`. See
notes/x86-uefi-boot.md for the whole account, the measured numbers, and the honest limitations
(the bench procedure itself is untested, the suite has not been run under firmware, and SMP under
UEFI has never been exercised).

**Purchased 2026-08-15 (calef), all arrived and installed as of 2026-08-23**: the OptiPlex 7050
Micro (i5-7500T, 16GB, 256GB NVMe, with its AC adapter, $139), the Dell C4PDJ serial module with
its cable ($18.88, the with-cable check the earlier draft flagged, answered by the listing's own
title), and the dev-side RS-232 chain (FTDI USB adapter at 1.5 ft, $15.96, plus a StarTech NM9FF
null-modem barrel, $7.98, chosen over a cable so the desk carries eighteen inches of serial, not
three feet). About $182 all-in against the $194 estimate, twelve days after selection; the $129
machine tier had aged out and nothing else moved. This milestone completes when the machine has
printed a byte over serial; the x86_64 port itself is milestone 161's scope and is not gated on
the purchase, because it starts under QEMU TCG the way riscv64 did. One bench note for arrival,
recorded here because both kits share the desk: this port is real RS-232 and the boards' adapters
are 3.3 V TTL, and the two chains must never swap; label them.

DECISIONS §19 names x86_64 as the third ISA target, and the second ISA's lesson (milestone 16, the
VisionFive 2) is that the board should be chosen and ordered before the port needs it, from
requirements the port derives rather than from specs. Bare-metal bring-up is a loop of hang,
power-cycle, retry, so the machine must be dedicated and consequence-free; cordoba is disqualified
for bare metal on exactly those grounds (it is the production server, and a 2013 desktop board has
no BMC, no serial-over-LAN, no remote power), though it remains the KVM and VT-d *virtualized* test
host for the same port.

The requirements, each traced to something this tree already does:

- **A real 16550 COM port.** Early bring-up output exists before anything else works, and QEMU's
  q35 machine emulates the same legacy UART, so one driver spans emulator and silicon. This is the
  NS16550/PL011 pattern both existing ISAs follow, and it eliminates most modern consumer hardware.
- **VT-d**, because IOMMU-backed driver isolation (milestone 16) is a parity theme (§19), and the
  x86 side of the DMA-confinement story needs real hardware eventually.
- **A NIC QEMU can stand in for.** QEMU 11.0.2 (checked against the pinned binary, not the docs)
  emulates two modern Intel families: `e1000e` (I217/I218/I219) and `igb` (82576, whose driver
  family covers i210/i211/i350). It does **not** emulate `igc` (i225/i226), and upstream has
  nothing in flight. An i226 machine is therefore acceptable but taxed: the driver core gets
  written against QEMU's `igb` (igc is igb's descendant, so rings and descriptors carry over) and
  the igc deltas are ported on hardware. A minimal driver is 1,500-3,000 lines against Intel's
  public datasheet; the plumbing around it (PCI decode, DMA confinement, the userspace net server)
  already exists.
- **Four real cores** for the per-CPU scheduler, and any Intel core has the PMU that milestone 25's
  `sel4bench` comparison was deferred to real hardware for.
- **Remote power cycling** by smart plug, not by management firmware. A plug is $15 and works on
  anything.

**The selection: a used Dell OptiPlex 7050 Micro plus the Dell C4PDJ serial module** (calef,
2026-08-03, settled after a full pass over the new market): i5-7500T with 16GB was $129 with the
module at $35, ~$194 all-in with the dev-side serial gear and the smart plug. The used-hardware
risk was weighed deliberately and priced: eBay's money-back guarantee bounds "does it work" to
return friction, and at real configured prices every new machine cost $150-350 more. The 7050
keeps the fastest cores in the field and the I219 NIC in QEMU's `e1000e` family, so the
one-driver-spans-emulator-and-silicon property holds with no caveats. The module is Dell P/N
**C4PDJ** (fits 3050/7040/7050 MFF, snaps into the rear punch-out, cables to a motherboard
header; check the listing includes the cable); used units essentially never ship with it, so buy
it separately rather than hunting for a factory-configured unit.

The market at selection time, so the next reader knows what was weighed. The closest contender
was a **new Protectli VP2430** ($300 configured with coreboot): a real vendor with published
datasheets, open-source firmware aligned with `measured_boot`'s future on x86, console cable
included, but i226-V NICs in the `igc` family QEMU does not emulate, and $150 over the used
route; it stays **the recorded alternative** if the used machine disappoints or when open
firmware becomes the point. Configured industrial N100 boxes on Amazon ran $500-730 and are
dominated by the VP2430 at every point. A used PC Engines apu2 deserves a correction from the
first draft of this entry: its i210 NICs are `igb` family, so QEMU's igb model gives it the
one-driver property this entry originally credited only to the 7050; it stays a runner-up for its
EOL status and slow Jaguar cores, not its NIC. If netboot iteration becomes worth it, cordoba
hosts the PXE/TFTP end.

## Scope note

This milestone is the machine, the serial link proven, and nothing else; the port itself is
milestone 161's scope and is not gated on this purchase, because the port starts under
QEMU TCG the way riscv64 did. Buying early was cheap insurance against the VisionFive 2 pattern
(ordered 2026-07, arrived 2026-08-21) of the board being the long pole, and it paid off: the
hardware side finished before the code side needed it.
