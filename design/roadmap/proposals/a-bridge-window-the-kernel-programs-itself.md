# A bridge memory window the kernel programs itself

**Status: PROPOSED 2026-09-17.** Written by milestone 320's lane. Provisional slug; the integrator
mints the number.

**Gate: NONE.** `NIFE_PCIE_ROOT_PORT=1` on `scripts/qemu-runner-x86_64.sh` reaches the machine this
is about in one boot, with no hardware: QEMU's PVH boot leaves a root port's window unprogrammed,
which is exactly the case this describes.

## What

A PCI-to-PCI bridge forwards a memory transaction downstream only when the address falls inside the
window its own **Memory Base** and **Memory Limit** registers describe (config 0x20 and 0x22 of a
type-1 header; there is a prefetchable pair at 0x24/0x26 too). Those registers are written by
whatever enumerates the bus. This kernel does not write them, and until milestone 320 it never met a
bridge, so there was nothing to write.

`crates/pci` now decodes a bridge's bus numbers ([`bridge_buses`]) and `kernel/src/pci.rs` walks
them. It does not decode or program the memory windows. So on a machine where firmware did not
program them either, a device behind a bridge **enumerates correctly and does not work**: its
configuration space answers, its BARs size, `place_bars` assigns an address, and no read of that
address reaches the device, because the bridge in front of it drops the transaction.

## Why it is not a milestone yet, and the honest reason

**It has never been observed to matter on a machine anyone runs.** QEMU's `q35` boots PVH with no
firmware at all, so `NIFE_PCIE_ROOT_PORT=1` produces exactly this: a controller on bus 1 that
enumerates and whose BAR does not decode. That is the only machine where it has been seen, and it is
a machine configuration built by this milestone to test a walk, not one any workload uses.

On real firmware the opposite is true and is the reason this can wait. xenon's UEFI enumerated its
own bus, numbered it, and programmed every bridge window before this kernel started; milestone 256's
adoption arm exists precisely so that the kernel honours what firmware placed rather than moving it.
A kernel that adopts a firmware-placed BAR is by construction using an address the bridge in front of
it already forwards, because firmware chose both.

So the shape of the risk is narrow: **a machine with real firmware that leaves a slot unconfigured**,
which is what an empty hot-plug slot or a device powered up after boot looks like.

## What would force it

- A boot where the census prints a function behind a bridge and the driver for it then fails on its
  first register read. On xenon that would read as an NVMe controller found at `01:00.0` whose
  controller capabilities register reads all-ones.
- `place_bars` assigning (rather than adopting) a BAR on a function whose bus is not 0. Today that
  combination cannot be distinguished in the boot line from any other placement, which is itself
  worth fixing first and is the cheap half of this proposal: **print the bus a placed BAR is on**.
- Hot plug, which is not on the roadmap.

## What it would take

Three things, in order of how much they are worth:

1. **Decode the windows** in `crates/pci`, beside `bridge_buses`. Memory base and limit are the top
   twelve bits of a 32-bit address with the low twenty implied, which is a two-line decode and a
   host test. Reading them is enough to *report* a BAR that falls outside its bridge's window, which
   turns the failure above from a mystery register read into a boot line.
2. **Constrain `place_bars`** so that a BAR on a bus behind a bridge is drawn from that bridge's
   window rather than from the machine-wide cursor. This is the correctness half and it needs the
   walk to carry each function's bridge with it, which `pci::Function` does not do today.
3. **Program the windows** where firmware left them zero, which means the kernel is doing a
   bridge-resource assignment pass and is the largest piece. It is also the one nothing has asked
   for: the first two make the failure visible and confine the damage, and a machine that needs the
   third is a machine this port has not met.

**Steps 1 and 2 without 3 is the useful stopping point**, and saying so is the point of writing this
down: a reader arriving from a failed boot should not assume the whole of a resource allocator is
owed.
