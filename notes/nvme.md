# NVMe: the first non-virtio disk

Milestone 53's storage half, decided by calef on 2026-08-15: **NVMe first**, because the backup
workload (milestone 55) measures sustained sequential write and endurance, which SD media fails,
and because a real PCIe device driver compounds toward the machines this project actually wants to
run on. Everything this project drove before this was a QEMU paravirtual device; an NVMe
controller is a real device family with a real specification, the same one in the M.2 drive on the
desk, and QEMU emulates it faithfully enough that the driver written here is the driver the
board-side work will reuse.

## The shape

Three pieces, in the tree's usual split:

- **`crates/nvme`**: the pure logic. Register field decode (CAP), command building (the 64-byte
  submission entries), the submission/completion ring arithmetic with the phase tag, doorbell
  addressing, PRP construction, and IDENTIFY parsing. Host-tested in milliseconds, five Kani
  harnesses (`script/verify`), no MMIO anywhere in it.
- **`kernel/src/nvme.rs`**: the volatile half. Reads and writes into BAR0, copies commands into
  the queue pages, rings doorbells, polls completions. `Nvme` takes the register window and one
  DMA region passed in (rule 2); `bring_up()` is the policy that finds the controller, allocates
  the region, and confines the device.
- **`kernel/src/pci.rs::find_nvme_device`**: enumeration and transport bring-up over the §18 PCIe
  machinery, matching the NVMe **class code** (`01:08:02`) rather than a vendor id, because the
  class triple is the one identity the spec requires of every controller, QEMU's included.

## How the protocol works, in one sitting

NVMe is a queue machine. The driver owns rings in its own DMA memory; the controller owns nothing
in the driver's space except what commands point it at.

1. **Commands** are 64-byte entries the driver writes into a **submission queue** (SQ), then
   announces by writing the ring's new tail index to that queue's **doorbell register** in BAR0.
2. The controller fetches the command (a DMA read of the SQ), does the work (DMA to or from the
   addresses in the command's **PRP** fields), and posts a 16-byte entry to the paired
   **completion queue** (CQ).
3. Every completion carries a **phase tag** that the controller flips each time it laps the ring.
   The ring starts zeroed, so the first lap writes tag 1, the second tag 0, and so on; the driver
   knows which tag means "fresh" and needs no shared index and no interrupt to spot a completion.
   `crates/nvme`'s `CqState` owns that discipline and a Kani harness proves the flip happens
   exactly at the wrap.
4. Queue pair 0 (the **admin queue**) is created by plain register writes (`AQA`/`ASQ`/`ACQ`)
   while the controller is disabled; every other pair is created by admin commands (CQ first,
   because an SQ names its CQ at creation). Admin commands and I/O commands are the same wire
   shape through the same rings, so bring-up itself exercises the transfer machinery.

The driver serves whole 4096-byte filesystem blocks, `fs_proto::blk`'s unit, and exposes exactly
the blk-IPC verbs as methods: `read_block`, `write_block`, `size_bytes`. A block is one to eight
logical blocks depending on the namespace's LBA format (QEMU's default is 512-byte LBAs, so eight);
IDENTIFY tells the driver the format and `IdentifyNamespace::blocks_per` does the arithmetic.

## The confinement story, which is the interesting part

This is the first DMA device in the tree whose addresses the kernel cannot validate before the
device uses them. The virtio drivers run at EL0 behind a `Virtio` capability: the kernel owns the
queue addresses and checks every descriptor against the driver's DMA region before ringing the
device. NVMe's equivalent of a descriptor is the PRP inside a command the controller fetches from
driver-written memory, and nothing kernel-side parses commands on their way past.

What bounds the device instead is the **IOMMU alone** (milestone 16b): `bring_up` confines the
controller's requester id to its six-page DMA region *before* the controller is enabled, so there
is no instant at which an enabled controller could reach other memory. All three test boots have
an IOMMU denying unlisted requester ids by default (aarch64's SMMUv3, riscv64's ratified RISC-V
IOMMU, x86_64's VT-d, confirmed behind this driver 2026-08-25 per decisions §86's evidence
section), which cuts two ways: the confinement is real (an address outside the region faults in
hardware), and it is *mandatory* (an unconfined NVMe controller on these machines cannot fetch its
first command, so a boot where someone forgets the confine call fails loudly rather than running
unconfined).

Note the flag difference in the runners: the virtio PCI devices need `iommu_platform=on` or QEMU
silently routes their DMA around the IOMMU; the NVMe device model needs **no flag**, because a real
PCI device's DMA always goes through the PCI address space. One less thing to forget, and the
reason the runner comment says so at the attach line.

### What x86_64 needed that the other two did not

Wiring the same driver onto `q35` (2026-08-25, decisions §86's evidence section) surfaced two bugs
that only a real (non-virtio) DMA device with a real PVH boot could have found, both fixed rather
than worked around:

- **`kernel/src/pci.rs::place_bars`** used to trust any nonzero BAR as "already placed, already
  mapped." True on the two device-tree architectures, where nothing runs before this kernel to
  place one; false on `x86_64`'s PVH boot, where QEMU resets `-device nvme`'s BAR0 to a live
  address of its own choosing (attached directly to the root complex, no firmware in between to
  reprogram it), unrelated to `PCI_BAR_PHYS`, the window `mmu::map_everything` actually mapped.
  `place_bars` now checks the existing address against that window rather than against zero.
- **`kernel/src/memory.rs::bring_up_page_frames`** and **`kernel/src/arch/x86_64/mmu.rs`'s
  `map_firmware_regions`** both sized themselves from `ram_regions()` alone (the e820 map's
  `usable` entries). Attaching VT-d and NVMe together grows the ACPI tables QEMU parks just above
  the top of guest memory enough that the `reserved` entry above them swallows the initrd's last
  few hundred bytes, which the PVH loader placed at a fixed offset below the top of memory sized
  for a smaller device set. The frame allocator's bitmap now widens to cover whatever `forbidden`
  reaches past RAM's own end, and the direct map now covers the initrd's recorded bounds
  explicitly, regardless of how the memmap classified the bytes.

Neither is `x86_64`-only in principle (a real UEFI machine, milestone 87, picks its own BAR
addresses too), which is why both fixes check against what is actually true rather than against
which architecture is running.

## EXAMPLES

Bring the disk up and move a block, from kernel context (this is the boot test, abridged; the full
version is `kernel/src/nvme.rs::tests`):

```rust
let mut disk = nvme::bring_up().expect("an NVMe controller is attached");
assert_eq!(disk.size_bytes(), 8 * 1024 * 1024);

disk.buffer().fill(0x5a);
disk.write_block(37)?;      // eight 512-byte LBAs, one command, one PRP
disk.buffer().fill(0);
disk.read_block(37)?;
assert!(disk.buffer().iter().all(|b| *b == 0x5a));
```

Run the proof of all of it on all three architectures:

```sh
script/test            # the boot test runs in every leg; xtask attaches the controller
cargo test -p nvme     # the queue mechanics alone, on the host, in milliseconds
cargo kani -p nvme     # the five harnesses, ~seconds
```

Poke at the controller interactively:

```sh
cargo xtask build && NIFE_NVME=target/nife-nvme.img cargo xtask run
# (write the image first: an 8 MiB zero file, or keep one a test run made)
```

## What the test proves, and where

`kernel/src/nvme.rs::tests::the_nvme_disk_serves_the_block_interface_end_to_end`, on **all three**
architectures (§19; x86_64 joined 2026-08-25, decisions §86's evidence section): the controller
enumerates over ECAM, comes up confined behind the SMMU (aarch64), the RISC-V IOMMU (riscv64), or
VT-d (x86_64), answers IDENTIFY with the attached image's exact size, and serves WRITE, READ-back
with byte-exact verification, and a read of an untouched block that must still be zeros (the write
landed where it said, not everywhere).

**The parity note milestone 53 requires**: what ships on all three architectures is this driver
against QEMU's `-device nvme`, on the two `virt` machines and on `q35`. The VisionFive 2's PLDA
XpressRICH root complex is a different host bridge (no `pci-host-ecam-generic` node, so
`find_nvme_device` truthfully reports nobody home on the board today); driving it is the
board-side follow-up, and this driver is written to need only a working enumeration and a BAR from
it.

## BUGS

- **The driver is kernel-resident**, unlike every virtio driver, and that is a recorded limitation
  rather than the design. A confined EL0 NVMe driver needs a kernel-owned transport capability in
  the `Virtio` capability's mold (or command parsing at the doorbell, which is the same decision
  wearing worse clothes), and that is new syscall surface: a §10/§16 design fork deliberately not
  taken in this milestone. Until it is, the IOMMU confines the *device* and nothing additionally
  confines the *driver*, because the driver is the kernel.

  **Two corrections to that sentence, 2026-09-03**, from §86's research pass, and they are recorded
  here because this is where a reader meets the limitation. **"Command parsing at the doorbell" is
  not the same decision wearing worse clothes.** It is a different one, and it is the only shape in
  §86's list that confines an EL0 driver on a machine with no IOMMU, which is every board this
  project owns. And **it may need no new syscall surface at all**: NVMe puts the controller and
  admin registers below offset 1000h and the first doorbell at 1000h, a page boundary, so
  "kernel keeps the admin plane, EL0 gets the data path" is expressible with the `DeviceFrame`
  capability that already exists. Read §86 before quoting this bullet.
- **Nothing serves it over blk IPC yet.** The driver speaks the blk verbs as an API, so an
  NVMe-backed block server is a wiring exercise plus the fork above; the FS server's default
  remains virtio-blk, untouched.
- **Polling only.** Completions are spotted by phase-tag polls with a spin bound, not by the
  interrupt-as-message path the virtio drivers use. Right for QEMU (which completes synchronously
  inside the doorbell write) and for a boot test; wrong for a shared machine under load. The
  controller is created with IEN=0 and no MSI-X table is touched, so interrupts are additive later.
- **One command in flight per queue.** `SqState` asserts rather than manages a full ring; queue
  depth is 16 to keep wraps exercised, not for parallelism. Milestone 55's storage bench will want
  real queue depth, and the ring arithmetic already supports it; the driver's completion loop does
  not.
- **One namespace, PRP-only, no SGLs, no PRP lists.** Transfers are exactly one filesystem block.
  All fine for the blk unit; a future bulk path (or a 8 KiB+ LBA format) needs PRP lists.
- **`bring_up` is not idempotent.** A second call re-confines the requester id (leaking the first
  domain's tables, as `iommu::confine` documents) and re-creates queues against a live controller,
  which the controller will refuse. Call it once; the boot test does everything in one case for
  exactly this reason.
