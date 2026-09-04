# Nothing here confines an interrupt target, and no boot has ever exercised one

**Status: PROPOSED 2026-09-03.** Written by the §86 research lane
(`maintainer/research-86-el0-nvme`), which went looking for what a userspace NVMe driver would need
and found a hole underneath every option it was pricing.

**Gate: NONE.** The first half is reading and two QEMU flags. Silicon is not required to find out
whether the claim holds, only to find out whether it holds on hardware.

**In brief.** An MSI or MSI-X message is a memory write to an architecturally special address, so an
IOMMU doing DMA remapping alone does not confine it. A component that can write a device's MSI-X
table can aim an interrupt at a vector it was never given. Linux refuses to hand a device to an
untrusted userspace driver on a machine without interrupt remapping for exactly this reason, and
names its escape hatch `allow_unsafe_interrupts`.

**This tree has never asked the question**, in either direction:

- `scripts/qemu-runner-x86_64.sh` attaches `-device intel-iommu` with no `intremap=on`, so
  interrupt remapping is off in every x86_64 boot this project runs.
- `scripts/qemu-runner-aarch64.sh` uses `gic-version=2`, which has no ITS, so there is no MSI
  translation path to exercise on that machine at all.
- No driver touches an MSI-X table. notes/nvme.md's `BUGS` records that the NVMe controller is
  brought up with `IEN=0` and no MSI-X table is touched, and the virtio drivers take their
  interrupts through `Object::Irq`.

So the gap is latent, and it stays latent exactly as long as every driver that can reach a BAR is
the kernel. It goes live the first time a driver leaves the kernel and wants interrupts instead of
polling, which is what DECISIONS §86 (whether an NVMe driver can leave the kernel, and what
capability would let it) is deciding.

**What the work is, cheapest first.**

1. **Say what is claimed.** notes/confinement-claims.md enumerates 26 claims and records which have a
   replayable falsification. Interrupt targets appear in none of them. Adding the claim, even as one
   that is deliberately *not* made today, is most of the value and costs an afternoon; milestone 202
   found three claims that were stated nowhere and that was its most useful output.
2. **Turn the flags on and see what breaks.** `-device intel-iommu,intremap=on` on x86_64 needs
   `kernel-irqchip=split`, and `gic-version=3` on aarch64 brings an ITS. Both are one line in a
   runner, and both would tell us whether this tree's boot path even survives the machine having the
   hardware.
3. **Then decide who owns the MSI-X table**, which is a §86-shaped question and probably belongs in
   whatever §86 settles: if any part of a device's BAR is mapped to EL0, the page holding the MSI-X
   table is either kept by the kernel or the confinement claim has a hole in it that the IOMMU does
   not cover.

**The hazard to name in whatever does this.** Turning on interrupt remapping and watching the tests
stay green proves nothing on its own, which is the failure milestone 202 caught in DECISIONS §31's
headline assertion: a test that passes for a reason unrelated to the claim is worse than no test.
The falsification has to be a driver deliberately aiming an interrupt somewhere it was not given,
and it has to come back red.
