# 165. x86_64 PCI enumeration: wire `kernel/src/pci.rs` to ACPI's MCFG

**Status: PARTIAL.** Minted 2026-08-24, provisional number pending the integrator (mint against the
current index at merge; two other numbers in this neighborhood were already taken by open pull
requests when this was written, one of them proposing a milestone for the JH7110's PLDA PCIe root
complex driver, unmerged as of this writing so not cited here by number). Scoped the same day as
that other proposal, as a parallel check of whether x86_64 can reach a real-hardware data point for
[DECISIONS §86](../decisions/86-el0-nvme-driver.md) faster and cheaper than a from-scratch JH7110
driver would, given that x86's PCIe discovery is ACPI-described and architecturally identical
between QEMU's `q35` and real x86 hardware (unlike RISC-V's QEMU-only fake ECAM device).

**Gate: NONE.** What is built here needs no syscall surface, no new dependency, and no
`DECISIONS.md` section of its own. What it unblocks (real NVMe-behind-IOMMU on x86) is gated on
VT-d, roadmap item 6 of milestone 161, unbuilt.

## The gap, as `notes/x86-port.md` already named it

Milestone 161 (the x86_64 kernel port) landed real ACPI table parsing (the RSDP scan, the root
table walk, the MADT, and the MCFG) against real hardware evidence (the Dell OptiPlex 7050 Micro,
milestone 87, x86_64's real target machine). `notes/x86-port.md`'s "discovery seam" section recorded
the gap this milestone closes: "The PCIe ECAM window the MCFG describes is exactly the constant
`arch::mmu::PCI_ECAM_PHYS` hardcodes... The constant should come from here rather than being checked
against here, and doing that is a line of code once something consumes it." `kernel/src/pci.rs`
itself said the same thing at its own probe: "A machine whose tree has no such node... answers every
probe here with nobody home", true of x86_64 unconditionally, since it has no device tree at all.

## What was built

1. **The seam.** `memory::record_pci_regions` fills `memory::pci_regions()` directly (the same
   static every probe in `pci.rs` already reads), the x86_64 counterpart of how `memory::init` fills
   it from a `pci-host-ecam-generic` device-tree node on the other two architectures. Called once
   from `main.rs`'s boot tour, right after `read_acpi`, with the ECAM window from ACPI's MCFG and a
   hardcoded BAR window (see the BAR paragraph below).

2. **The real complication, found by measurement rather than assumed away.** Reading ACPI's MCFG is
   not sufficient by itself. QEMU's monitor was used to check directly (`xp` on the physical address
   the MCFG names, before any kernel code ran): the address **faults** ("Cannot access memory"),
   not the all-ones an absent PCI *device* would read. The chipset's ECAM decode is off until
   something programs the host bridge's `PCIEXBAR` register (offset `0x60` in bus 0, device 0,
   function 0's configuration space): a real BIOS or UEFI does this before ever handing control to
   an OS, and PVH (the protocol this port boots through) is a hypervisor entry point, not firmware,
   so nothing has. `arch::x86_64::machine::enable_pcie_ecam` programs it through the legacy
   `CONFIG_ADDRESS`/`CONFIG_DATA` ports (`0xcf8`/`0xcfc`, already-existing primitives in
   `arch::x86_64::port`, written for exactly this and unused until now), and the same address was
   then confirmed, still via the monitor, to read the host bridge's own vendor/device id
   (`8086:29c0`) once the register was enabled. This is very likely a PVH-only step: real firmware
   (what milestone 87's OptiPlex will boot through) does this already, and rewriting the same base to
   the same value there should be a no-op. It is written unconditionally rather than assumed so this
   port does not depend on that being true.

3. **The BAR window is hardcoded, not discovered, and this is a real, permanent limitation rather
   than a temporary gap.** ACPI's MCFG names only the ECAM window; the 32-bit MMIO window BARs are
   placed in lives in the PCI host bridge's `_CRS` object on a real ACPI machine, which is AML, and
   this kernel has no AML interpreter (`notes/x86-port.md`'s own "What is deliberately not decoded").
   `arch::x86_64::mmu::PCI_BAR_PHYS` is a hardcoded constant (`0xc000_0000`, q35's conventional PCI
   hole), confirmed disjoint from RAM, ECAM, the HPET and both APICs by reading QEMU's `info mtree`,
   but **not exercised by an actual BAR placement**: no PCI function that needs one is on the bus
   under the current runner (see below).

4. **Proof, under QEMU's `q35`.** `kernel::pci::tests::acpi_mcfg_wires_a_real_ecam_window_that_finds_the_host_bridge`
   (new, x86_64-only) enumerates the bus through the ACPI-sourced, now-enabled ECAM window and
   asserts it finds q35's own host bridge (bus 0, device 0, function 0, vendor `8086`, device
   `29c0`), a chipset function present regardless of what `-device` flags the runner does or does
   not pass. `script/test` on all three architectures: aarch64 295/1, riscv64 298/1, x86_64 99/6 (one
   skip converted to one pass versus the pre-existing baseline; no regressions). `script/lint` green.

## What was deliberately not built

- **`scripts/qemu-runner-x86_64.sh` was not touched.** q35 already presents six PCI functions with
  zero `-device` flags (host bridge, VGA, a default NIC, ISA bridge, SATA/AHCI, SMBus; measured via
  QEMU's monitor `info pci`), which is what made proof possible without changing the runner's default
  device set at all, the exact fork the brief for this lane flagged as needing to stop and report
  rather than silently changing. It did not arise.

- **An actual `-device nvme` attach was investigated and is NOT safe today**, which is worth
  recording precisely because it looked free. `xtask` already sets `NIFE_NVME` unconditionally for
  every architecture's leg, and both aarch64 and riscv64 runners already wire it; only x86_64's
  does not. But `kernel::nvme::tests::the_nvme_disk_serves_the_block_interface_end_to_end` hard-
  asserts `crate::iommu::active()`, and `arch::x86_64::iommu::active()` is a permanent `false` (no
  VT-d built; roadmap item 6 of milestone 161). `nvme::bring_up()` itself tolerates an inactive
  IOMMU (it runs the controller unconfined, printing nothing), so attaching the device would not
  skip the existing test, it would turn it into a **hard failure** on the confinement assertion.
  Getting a real NVMe device working on x86, confined, is therefore gated on VT-d/DMAR parsing, not
  on anything this milestone touches. This was established by reading `nvme.rs` and
  `arch/x86_64/iommu.rs` directly rather than by running the experiment and breaking the suite.

## What this does and does not establish for DECISIONS §86

**What it establishes**: the *discovery* half of what a real hardware data point needs. x86's PCIe
enumeration mechanism (ACPI's MCFG naming an ECAM window, a chipset register enabling its decode,
then ordinary config-space reads) is now proven under QEMU using the same mechanism real x86
hardware uses; nothing about the mechanism itself is QEMU-specific the way the device is on RISC-V.
Milestone 87's OptiPlex, once it boots this kernel, should need no code change here to also
enumerate its own real PCI bus; only the values (its own MCFG base, its own BAR hole) will differ,
and if real UEFI leaves ECAM already enabled, `enable_pcie_ecam`'s rewrite should be a no-op.

**What it does not establish**: §86 is about a *confined userspace NVMe driver*, and confinement is
exactly the piece this milestone cannot touch. x86 has no IOMMU driver at all (VT-d, item 6), so
there is no path from what is built here to "NVMe behind a real IOMMU" without first building VT-d,
which is a materially larger piece of work than this one (a DMAR table walk, translation table
setup, and fault handling, the same shape §16b's SMMUv3/RISC-V IOMMU drivers took) and is out of
this lane's scope by the brief that started it. The proposed JH7110 driver (see the note about it
above) has the same gap in reverse shape: it would reach real NVMe hardware over a real
(non-QEMU-fake) discovery mechanism, but RISC-V's IOMMU driver already exists (milestone 16b), so
its path to a *confined* data point is shorter once its hardware bring-up lands. **Read plainly: x86
got the cheaper half of §86's evidence (discovery) at low cost; the JH7110 path still owns the
harder half (confinement over a real, non-fake root complex) for its own architecture, and x86 needs
VT-d built first to catch up on that half here. Neither path alone gives §86 the full data point it
is holding out for.**

## What is owed

- **VT-d** (roadmap item 6 of milestone 161): DMAR table parsing (the root-table walk is already
  generic; finding the DMAR is adding a signature arm the same way `read_mcfg`/`read_madt` are),
  translation domain setup, and fault handling. Without it, x86 cannot supply §86's confinement half
  at all, on any device.
- **PCI interrupt routing** (INTx via `_PRT`, or MSI): named as owed by milestone 161 already,
  unaffected by this milestone. `PCI_IRQ_BASE = 0` stays honest.
- **An actual BAR placement**, proven rather than merely mapped-and-disjoint: needs either a real
  device attached (which the NVMe/VT-d gap above blocks for NVMe specifically) or a smaller device
  that needs no confinement to test against.
