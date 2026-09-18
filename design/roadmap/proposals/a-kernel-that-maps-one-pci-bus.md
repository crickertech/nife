# A kernel that maps one PCI bus cannot see a disk behind a root port

**Status: PROPOSED 2026-09-17.** Found on xenon, by booting the milestone 318 image on the machine
fatal risk 6's decisive experiment is waiting on. Transcript:
`bench/xenon-2026-09-17/nvme-attempt-2-no-controller-found.log`.

**Gate: NONE.** It is a kernel change and needs no hardware to write, though only hardware can
confirm it, since the topology this is about does not exist under QEMU.

## In brief

**This is the one finding standing between risk 6 and its answer**, and it is four characters:

```rust
// kernel/src/arch/x86_64/mmu.rs:246
pub const PCI_ECAM_BUSES: u16 = 1;
```

ACPI's MCFG described the window as `buses 0..=127` and the boot printed exactly that, but the
kernel maps one bus and enumerates one bus. Its own census says so in the singular, and nobody
noticed for a year because the sentence is true either way:

```
pcie ecam 0xf0000000, buses 0..=127 (mmu::PCI_ECAM_PHYS says 0xb0000000)
pci : 15 function(s) on the bus, 0 with a BAR this kernel can neither use nor adopt
```

On `q35` an NVMe controller hangs directly off bus 0, so one bus has always been enough and
`pci::find_nvme_device()` has always found it. **On a real machine the M.2 slot is behind a PCIe
root port**, so the controller sits on a secondary bus, and the search ran over a bus the disk was
never on. The test did not fail; it skipped, reporting `NIFE_NVME not set on this leg?`, which is
QEMU's explanation for an absence with a different cause.

**So the risk 6 entry is wrong again, in a new way.** It now says the remaining distance is a bench
evening. The bench evening happened. What it found is that the driver cannot be handed a controller
this kernel cannot see.

## Why it is a milestone rather than a one-line change

**Raising the constant is not free, and the cost is page tables rather than code.**
`PCI_ECAM_MAPPED` is `PCI_ECAM_BUSES * 0x10_0000`, one megabyte of configuration space per bus, so
128 buses is 128 MB of mapping where there is now one. That is a decision about the kernel's
address space, and this boot already spends 32,840 KiB on page tables.

Three shapes, and they are not equivalent:

1. **Map all 128 and brute-force scan**, matching what MCFG describes. Simplest, and the existing
   `pci::enumerate` already loops `0..buses` so the enumeration needs no change at all. Costs the
   mapping, and spends most of it on buses with nothing on them.
2. **Walk the bridges.** Read each bridge's secondary and subordinate bus numbers and map only the
   buses firmware actually populated. This is what every real OS does and it is the answer that
   scales to a machine with more than one root port. Costs a bridge walk `crates/pci` does not have:
   `enumerate` is a flat scan over a bus range and knows nothing about header type 1.
3. **Map lazily**, a bus at a time as the walk reaches it. Cheapest in address space and the most
   machinery.

**Recommendation: 2**, and by AGENTS.md's own test rather than by effort. Option 1 is less work and
would answer risk 6 this week, so if it is chosen it should be chosen *as* the cheaper thing and
said so out loud. But a flat scan of a range ACPI happens to describe is the same shape of
assumption that produced this bug: it works until a machine describes something bigger. A bridge
walk asks the machine what is there. **Would we still pick the bridge walk if both cost the same?**
Yes, which is the tell that option 1 is an effort argument.

Worth noting option 1 is not wasted if taken first: the bridge walk subsumes it, and a measured
answer to risk 6 has been waiting since 2026-08-30.

## What it unblocks

- **Fatal risk 6's decisive experiment**, which is the reason this is worth a lane now rather than
  in its turn. Everything else that experiment needs exists and is on `main`.
- **Milestone 261's own gate**, which says the driver is done and the machine is what is left.
- Anything else on a real x86_64 machine that is not on bus 0, which on this machine is most of it:
  the census found 15 functions and a 7050 has considerably more than 15 PCI functions.

## What this does not claim

**It is not established that the Micron is the only thing hidden.** The census counted 15 functions
on bus 0 and nobody has yet enumerated what a full walk finds, so "the NVMe is behind a root port"
is inference from the machine's form factor and from the absence, not from a reading. **The first
thing a lane should do is print the full census**, because that either confirms this in one boot or
replaces it with a better finding.
