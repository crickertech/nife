# Read the DMAR, because a firmware checkbox is not an IOMMU

**Status: PROPOSED 2026-09-04.** Written by the lane that transcribed xenon's firmware into
`notes/xenon-firmware.md`, which answered one of the two questions it was sent to answer and
converted the other into this.

**Gate: NONE.** The DMAR is an ACPI table like every other one, `arch::x86_64` already walks the
XSDT and prints the table list on every boot, and QEMU with `-device intel-iommu` synthesises a
DMAR, so the parser can be written and tested with no board and nothing is reserved. Only the
*result on xenon* needs the bench, and that is one boot of a stick that already exists.

## What is being proposed

Parse the DMAR (DMA Remapping Reporting table) at boot and print what it says: the remapping units,
their register base addresses, the `INTR_REMAP` flag in the table's own flags field, and each
unit's extended capability register, whose `IR` bit is the one that says the hardware can remap
interrupts at all.

## Why

Two in-tree records have been waiting on a fact that turns out not to live where everyone assumed.

`notes/x86-uefi-boot.md` says the third of its open questions is xenon's alone: *"whether the
OptiPlex's firmware leaves interrupt remapping off. Nothing under QEMU can answer that, because the
answer is a setting in somebody else's firmware."* `notes/confinement-claims.md` carries the same
thing as a fifth claim: a confined component's MSI-X write is a memory write, so DMA remapping does
not cover it, and VFIO refuses to hand a device to an untrusted userspace driver on a machine
without interrupt remapping for exactly that reason. `design/decisions/86-el0-nvme-driver.md` names
it as the axis its options had not priced.

**The premise turned out to be half wrong, which is why this is a proposal rather than a bench
task.** Seventy photographs of the 7050's entire setup UI contain no interrupt-remapping control,
and the whole Virtualization Support menu is three pages: Virtualization, VT for Direct I/O,
Trusted Execution. So it is not a setting in somebody else's firmware in the sense that phrase
implies; there is no switch to photograph. What there is instead is a **table** the firmware
publishes, which any kernel can read, and which this kernel is already three lines from touching.

That relocates the work from the most expensive place in this project to the cheapest. AGENTS.md
notes that a null modem is where discovery costs most; a DMAR parser is host-testable logic in a
crate, exercised under QEMU, and the bench trip is reduced to reading one line off a boot tour.

## What it would settle

- **`notes/confinement-claims.md`'s fifth claim** stops being latent-with-no-way-to-check and
  becomes a measured yes or no on the one machine that has the hardware.
- **DECISIONS §86's option set** gets the number it was priced without: whether a userspace NVMe
  driver on xenon could be given interrupts at all, or would have to poll the way the current one
  does.
- **`design/roadmap/195-uefi-boot-finish.md`'s remaining open question** closes, or is shown to
  need something else.

## What it does not do

It does not enable interrupt remapping, program a remapping unit, or change any driver. It reads
and reports. Turning the unit on is a separate and much larger piece of work, and it should not be
started until somebody knows whether the hardware here supports it.

## The honest cost

The DMAR's structure is a short table with a variable-length list of remapping-structure entries,
and this tree has parsed several ACPI tables already (`notes/x86-port.md` records the XSDT, MADT
and MCFG work). The expensive part is not the parse, it is deciding what the kernel should *do*
when it finds a DMAR it did not expect, and the answer for this proposal is nothing: print it.
