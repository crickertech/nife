# 261. The NVMe driver leaves the kernel, on the machine that can finally confine it

**Status: NOT-STARTED.** Minted 2026-09-05 by the maintainer. [§86](../decisions/86-el0-nvme-driver.md)
was **DECIDED on 2026-09-03** and the work it authorises has had no milestone since, which is
milestone 247's failure class (work identified by a finished piece goes nowhere) applied to a
decision rather than to a block. *(Number provisional until the merge queue lands it.)*

**Gate: HARDWARE.** Of the second kind: the machine is on the desk and one thing on it is calef's,
below. The driver itself is buildable and testable under QEMU and does not wait for any of it.

## What §86 decided, so this block does not reopen it

**Option 2a**, in its own words: an EL0 NVMe server that **adds no syscall surface at all**, so *"the
only durable commitment is what an `nvme_server`'s spawn contract says, and this tree changes spawn
contracts routinely."* Designed so option 4 can be added later without reshaping the driver, and the
choice between them settled by measurement rather than argument.

**Not reopened here**: whether a new `Object` variant is minted, and whether a validator ships with
it. §86 says plainly those are calef's, and they are option 4's questions, not 2a's.

## One premise of §86 has changed, and it matters to this milestone

§86's reading, written 2026-09-03:

> fatal risk 6's decisive experiment is "one real, non-virtio device on real silicon, confined, at
> throughput", **the silicon this project owns has no IOMMU**, and option 4 is the only entry here
> that confines without one.

**That is no longer true, and nobody had established it when §86 was written.** xenon has VT-d and an
NVMe behind it. `notes/xenon-firmware.md` records the machine's own setup UI: a `Micron 2450 NVMe
256GB` on M.2 PCIe SSD-0 with SATA in AHCI rather than RAID, *"so the NVMe is a plain PCIe function
rather than hidden behind Intel RST"*, on a machine milestone 87 selected partly for VT-d. The tour
has already reported `vt-d: drhd 0xfed90000 up, translation enabled (gsts.tes confirmed)` under OVMF.

**This does not overturn §86's choice; it strengthens it.** Option 4 exists to confine a device on
silicon with no IOMMU. On xenon the IOMMU is present, so **option 2a plus VT-d is a confined real
device at throughput**, which is risk 6's decisive experiment without the validator option 4 was
reaching for. Option 4 remains what a board without an IOMMU would need, and that is a different
machine's problem.

## What calef has to do, and it is one thing

**Wipe xenon's internal NVMe.** The disk currently holds a Windows installation, and **a disk this
project must not write to is not a disk it can drive**, which is what has actually been standing
between this tree and risk 6's decisive experiment rather than any missing hardware.

- **The machine's own firmware does it**: Setup, Maintenance, **Data Wipe**, `Wipe on Next Boot`
  (`notes/xenon-firmware.md`, IMG_4091). It covers internal SATA HDD/SSD, M.2 SATA SSD, M.2 PCIe SSD
  and eMMC; on a 7050 Micro that is this NVMe and nothing else, and it does not touch removable
  media.
- **It is not recoverable and cannot be terminated once started**, which is the page's own warning.
- **Nothing on it is wanted.** calef, 2026-09-05: *"The Windows image is freshly wiped. Don't worry
  about it. The system listing told me it would be there and I wouldn't want somebody else's data
  anyways."* So this is a seller's fresh image rather than anyone's data.
- **Cost: nothing, and one boot.** No purchase, unlike milestone 87's requirements list.

**Status: not done as of 2026-09-05.** This block should carry the date it happens, the way 87 carries
its purchase dates, and its `## Follow-on` should say `Outstanding.` with a checked date until then.

## What a lane builds without any of that

An EL0 `nvme_server` under §86's option 2a, against QEMU's NVMe, which the runner already attaches.
`crates/nvme` exists (716 lines, host-tested, 25 Kani sites) and `kernel/src/nvme.rs` is the
in-kernel driver milestone 53 built; the work is moving the queue mechanics out to a confined process
that is handed a doorbell page and a DMA window and nothing else.

**The name is calef's** and a lane should ship a provisional one and say so. `nvme_server` is what
§86 calls it in passing and that is not a ratification.

## The proof that this milestone worked

**A confined EL0 process drives xenon's real NVMe at a measured throughput, with VT-d translating
its DMA**, photographed, since xenon has no serial console this project can read.

Under QEMU the same driver working is the prerequisite and not the proof: milestone 16b already
proved IOMMU-backed isolation against emulated silicon, and risk 6's open clause is specifically
about a real device at real speed.

## BUGS

- **This block assumes VT-d in front of the NVMe and has not proved it.** The tour reports a DRHD up
  under OVMF; nobody has confirmed the NVMe's StreamID is behind that unit on this machine, and a
  DRHD covering only some functions is a normal x86 arrangement.
- **Nothing here is measured.** "At throughput" has no number attached, and risk 6's clause needs one
  to be answered rather than merely attempted.
- **xenon halts at POST without a keyboard**, so every boot here is attended until the two settings
  milestone 260 names are changed. That makes an iteration loop expensive in exactly the way the
  netboot work was meant to fix.
- **`crates/nvme`'s Kani harnesses cover queue mechanics, not confinement.** Moving the driver to EL0
  does not move the proofs, and this block does not say what the confinement claim's own test would
  be. Milestone 202's convention (a claim, a test, a replayable falsification) is what it should meet.
