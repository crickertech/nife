# 320. Every PCI bus the machine has, not just bus zero

**Status: NOT-STARTED.** Minted 2026-09-17 by the maintainer, from
`design/roadmap/proposals/a-kernel-that-maps-one-pci-bus.md`, which this block replaces and which
carries the full argument. *(Number provisional until the merge queue lands it.)*

**Gate: NONE.** Writable and testable without hardware. Only xenon can confirm it, because the
topology it is about does not exist under QEMU.

**This is the one thing standing between `design/fatal-risks.md` risk 6 and its decisive
experiment.** Everything else that experiment needs is built, tested and on `main`.

## What is wrong

```rust
// kernel/src/arch/x86_64/mmu.rs:246
pub const PCI_ECAM_BUSES: u16 = 1;
```

The kernel maps one megabyte of configuration space and enumerates one bus. ACPI's MCFG described
`buses 0..=127` and the boot printed exactly that, two lines apart, and the discrepancy has been
sitting in plain sight for a year because both sentences are true:

```
pcie ecam 0xf0000000, buses 0..=127 (mmu::PCI_ECAM_PHYS says 0xb0000000)
pci : 15 function(s) on the bus, 0 with a BAR this kernel can neither use nor adopt
```

On `q35` an NVMe controller hangs directly off bus 0, so one bus has always been enough. **On
xenon the M.2 slot is behind a PCIe root port**, so the controller sits on a secondary bus and
`pci::find_nvme_device()` searched a bus the disk was never on. The confinement test did not fail,
it **skipped**, reporting `NIFE_NVME not set on this leg?`, which is QEMU's explanation for an
absence with an entirely different cause.

Transcript: `bench/xenon-2026-09-17/nvme-attempt-2-no-controller-found.log`.

## The first commit is a census, and it is not optional

**The claim that the Micron is behind a root port is inference, not a reading.** It comes from the
machine's form factor and from an absence. Before building anything, map the buses MCFG describes
and print every function found: bus, device, function, vendor, device, class, and for each bridge
its secondary and subordinate bus numbers. One boot then either confirms the diagnosis or replaces
it with a better one, and the census is worth keeping afterwards either way.

**If the census finds the NVMe on bus 0 after all, stop and report.** The whole premise of this
block is wrong in that case and the real cause is elsewhere.

## The design fork, and which half is the effort argument

Three shapes, and the block does not decide between them; the lane recommends after the census:

1. **Map all 128 and brute-force scan.** `pci::enumerate` already loops `0..buses`, so the
   enumeration needs no change at all. Costs 128 MB of mapping where there is now 1 MB, on a boot
   already spending 32,840 KiB on page tables, and spends most of it on buses with nothing on them.
2. **Walk the bridges.** Read each bridge's secondary and subordinate bus numbers (header type 1)
   and map only what firmware populated. What every real OS does, and the answer that survives a
   machine with more than one root port. `crates/pci` has no notion of header type 1 today.
3. **Map lazily**, a bus at a time as the walk reaches it. Cheapest in address space, most
   machinery.

**The recommendation is 2**, and by AGENTS.md's own test rather than by cost. Option 1 is less work
and would answer risk 6 sooner; if it is taken, it must be taken *as the cheaper thing* and said so
in those words, because a flat scan over a range ACPI happens to describe is the same species of
assumption that produced this bug. **Would we still choose the bridge walk if both cost the same?**
Yes. That is the tell.

Option 1 is not wasted work if taken first: the bridge walk subsumes it.

## What it must not break

- **The two device-tree architectures.** They fill the same `memory::pci_regions()` seam from a
  `pci-host-ecam-generic` node and their BARs always arrive at zero. Parity is a gate (§19), so
  both must still pass `script/test`.
- **`place_bars`' adoption arm** (milestone 256), which exists because xenon's firmware places BARs
  this kernel cannot honour. More buses means more functions carrying firmware addresses, and the
  2 MiB BAR window is already the binding constraint: the same boot reported 13 of 15 functions
  outside the kernel's window. **Expect the census to make this worse and report what it finds**;
  `design/roadmap/proposals/a-bar-window-wide-enough-to-see-into.md` is the existing proposal there
  and this may be what promotes it.

## Done when

- The census prints every function on every bus the machine describes, on xenon.
- `pci::find_nvme_device()` finds the Micron, or the census proves it is not there.
- `script/test` green on all three architectures; host tests for any new bridge arithmetic in
  `crates/pci`, which is where it belongs and where Kani can reach it.
- The roadmap block carries what the census actually found.

## Index row

The kernel maps one megabyte of PCI configuration space and enumerates one bus, so a
controller behind a root port is invisible to it. That is why xenon's NVMe was never found, and it
is the last thing standing between fatal risk 6 and its decisive experiment.

## Follow-on

- **None.** This block is the identified work; it has not run yet.
