# 494. A driver for the network card a PC actually has

**Status: NOT-STARTED.** *(Number provisional until the merge queue lands it.)* Promoted from the
proposal `a-driver-for-the-network-card-a-pc-actually-has`, filed 2026-09-19, on calef's instruction
of 2026-09-20 to give every proposal on `main` a number. The text below is the proposal's own,
unedited except for this paragraph: the argument is its author's and promotion is not the moment to
improve it. Written by milestone 198 (a package manager, and the trivial install)'s rungs lane (`milestone/198-rungs-to-a-trivial-install`) for
rung 3 of the trivial install DECISIONS §157 (a trivial install is a web page,) defines: packages over the internet need a network
card, and the only one nife can drive is virtio-net, which no physical machine has.

**Gate: NONE.** The driver is written and tested under QEMU, which emulates the family xenon has.
The bench half needs xenon powered, which is the same attended boot milestone 261 needs.

## Which card xenon has, and the tree disagreed with itself

**xenon's network card is an Intel I219-LM.** Dell's own specification sheet for the OptiPlex 7050
says, for the Micro: *"Integrated Intel® i219-LM Ethernet LAN 10/100/1000"* (read on 2026-09-19 from
`i.dell.com/.../OptiPlex-7050-Towers-Technical-Specifications.pdf`, which covers the Tower, Small
Form Factor and Micro). The machine's own System Information page records only `LOM MAC Address
D8-9E-F3-74-B2-A2` and `Wi-Fi Device: Intel Wireless` (`notes/xenon-firmware.md`), so the firmware
names no model.

**Two records in the tree say otherwise, and neither cites anything.** Milestone 260's `BUGS` and
`script/netboot-rehearsal`'s header both say xenon has *"a Broadcom LOM rather than QEMU's e1000"*.
Both arrived in one lane's commits on 2026-09-05 (`55702345`, `962d0ba9`) with no photograph, log or
specification behind them, and milestone 87, which chose the machine partly for its NIC, says I219.
**The Broadcom sentence is false by Dell's specification**, and this lane cannot edit either file;
it is listed for the maintainer.

**What one boot would capture, and it costs nothing new.** The PCI survey has printed every
function's `vendor:device` and class since commit `672d3b97` (2026-09-18 06:48 UTC,
`kernel/src/pci.rs`, `survey`). xenon's last recorded boot predates it and printed only `15
function(s) on the bus` (`bench/xenon-2026-09-17/first-light-095500.log`). **The next xenon boot of
any current build prints the network card's device id on a line with class `020000`**, and the
Intel 8265 wireless card on class `028000`. Photograph the survey; that settles the id. The I219's
PCI device ids vary with the chipset generation and are not recorded here, because they would be
recalled rather than read.

## What a stranger's PC most plausibly has

A judgement, with the parts recalled rather than read marked. **Desktop boards mostly carry an Intel
or a Realtek gigabit or 2.5-gigabit controller** (Intel I219, I225 or I226; Realtek RTL8111 or
RTL8125, recalled). **Many laptops have no Ethernet port at all**, and Wi-Fi is a different order of
work (firmware blobs, 802.11 management, WPA), so it is out of scope and recorded in `BUGS`. A USB
Ethernet adapter is the plausible bridge for such a laptop, and it rides on milestone 242's host
controller, so it is a follow-on to 242 rather than to this.

## Options for the first card (reversible, recommended)

| | Card | QEMU model | Who has it | Kept or lost |
|---|---|---|---|---|
| **N1. Intel I219, `e1000e` family** | xenon's | Yes: QEMU emulates `e1000e` (milestone 87, checked against the pinned QEMU binary) | xenon; a large share of business desktops (recalled) | **Recommended.** Developed under QEMU, proved on the bench machine, one driver spanning both, which is the property milestone 87 bought xenon for |
| N2. Intel I225/I226, `igc` | Protectli VP2430, newer boards | **No** (milestone 87: *"It does not emulate `igc`"*) | Newer desktops | Lost as the first: no emulator, and nobody here owns one. Milestone 87 says `igc` is `igb`'s descendant, so N1's shape carries |
| N3. Realtek | Common on consumer boards | QEMU has an old Realtek model, not the RTL8111 family (recalled) | Consumer desktops | Lost as the first: no bench machine and no emulator for the part |

## How it is confined, and which precedent it follows

Two drivers in this tree are the analogues, and they differ in the way that matters:

- **virtio-net** (milestone 30) runs **inside `net_stack`**: `components/src/net_transport.rs` is a
  `#[path]` module of that binary presenting smoltcp's `phy::Device`, and the kernel owns the
  device's registers and mediates it through a `Virtio` capability with the shadow-ring validator.
  None of that mediation exists for a non-virtio device.
- **The EL0 NVMe server** (milestone 261, DECISIONS §86 option 2a) is the precedent for a real
  device: the kernel keeps the admin plane, the process holds a page of BAR0 and a confined DMA
  window, and on xenon VT-d is the confinement. **It added no syscall surface.**

So the I219 driver takes 261's shape, and one difference is worth checking before the design is
drawn. NVMe's specification put a page boundary exactly where the authority boundary belongs
(controller registers below `0x1000`, doorbells above). **For the `e1000e` family the ring base
registers and the ring tail registers sit in the same 4 KiB page** (recalled from the older 8254x
datasheets, not read from the I219's), so the page split that let 261's server ring doorbells
without being able to repoint its rings may not exist here. If it does not, the driver can aim its
rings anywhere inside its IOMMU domain, and **VT-d is the whole of the confinement**, which is
milestone 261's own load-bearing unknown (whether xenon's DMAR scope covers the function) applied to
a second device. Read the datasheet first.

**Where the driver runs** is a second, smaller question: a second `phy::Device` inside `net_stack`
beside `net_transport`, as virtio-net does today, or its own process with frames crossing an
endpoint. Recommendation: **inside `net_stack` first**, matching the tree; a separate process is a
frame protocol two programs agree on, which is the expensive category and wants its own reason.
The §92 test: at equal cost the separate process would be preferred for confinement (a NIC parser
and a TCP stack in separate address spaces), so **this recommendation is about effort** and is
recorded as such.

## Costs, from the tree rather than adjectives

- **Milestone 87's estimate**: *"A minimal driver is 1,500-3,000 lines against Intel's public
  datasheet; the plumbing around it (PCI decode, DMA confinement, the userspace net server) already
  exists."* An estimate written at purchase time, not a measurement.
- **The measured neighbours**: `net_transport.rs` is 390 lines (virtio, with the kernel doing the
  register work); `components/src/non_volatile_memory_express.rs` is 417 lines and
  `crates/non_volatile_memory_express` 1,086, host-tested with Kani harnesses, which is the split
  this driver should copy.
- **Interrupts**: x86 does not yet route a device line to a userspace waiter (milestone 299's scope
  note), and 261's server polls. The first NIC driver polls too.
- **The MTU**: `net_transport.rs` fixes `MTU = 576` because its whole DMA region is one page. A 10 MB
  package at 576 bytes a frame is slow for no good reason; this driver gets a multi-page region the
  way 261's server got sixteen pages of transfer buffer, and full 1,500-byte frames.

## Exit criteria

1. **Under QEMU**, `-device e1000e` behind `intel-iommu`: `net_stack` completes a DHCP round trip
   and a TCP transfer through the new driver, the same gates milestone 30's virtio-net passes, on
   x86_64. aarch64 and riscv64 carry the driver too, since QEMU's `e1000e` is a PCIe device on
   every bus the runners attach, or a scope note says why not (DECISIONS §19).
2. **On xenon**, a DHCP lease from the house router and a measured transfer from a host on the LAN,
   photographed, with the survey line naming the card.

## BUGS

- **The page layout argument above is recalled**, and it decides whether the confinement story is
  "like NVMe" or "the IOMMU alone". Read the I219 datasheet before designing the split.
- **Wi-Fi is out of scope**, and on a laptop it is the only network there is. A stranger with a
  laptop and no USB Ethernet adapter cannot reach rung 3.
- **One family.** `igc` and Realtek are follow-ons with no emulator for either.
- **xenon's DMAR scope for the NIC is unread**, the same unknown milestone 261 carries for the NVMe.

## Index row

xenon's network card is an Intel I219-LM.
