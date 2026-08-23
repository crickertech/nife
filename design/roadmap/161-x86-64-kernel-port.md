# 161. The x86_64 kernel port: bring up the HAL's third architecture

**Status: NOT-STARTED.** Minted 2026-08-23, splitting real work out of milestone 20's stale text.
DECISIONS §19 declared the target set (aarch64, riscv64, x86_64) and recorded honestly that
*"x86_64 is a declared target that does not exist yet."* Milestone 20's own "Deliverable, in two
parts" already named "bring up a second ISA, then a third: RISC-V first, x86_64 second," but 20 is
marked **BUILT** for the HAL split plus RISC-V alone (its own title says "proven on **a second**
architecture," singular) -- checked directly, `kernel/src/arch/` holds only `aarch64/` and
`riscv64/`, no `x86_64/` exists anywhere in the tree. The x86_64 half of 20's own deliverable was
never actually tracked as open work; this milestone is that tracking.

**Gate: NONE.** DECISIONS §19 already settled that x86_64 is a target; nothing here needs deciding.
Milestone 87's own text is explicit that this can start now, under QEMU TCG, the way RISC-V's port
did -- it is not gated on the physical machine.

## What this carries forward from milestone 20's original scope

**Why x86_64 second** (unchanged from 20's own reasoning, restated here since it now lives with the
work rather than beside it): the hard proof that the HAL abstraction is real rather than an
accident of two similar RISC ISAs. RISC-V is structurally close to aarch64 (device tree, weak
memory, a similar MMU shape); x86_64 is a genuinely different model: CISC, strong TSO memory
ordering, GDT/TSS, ACPI + PCI instead of a device tree, port I/O, the `syscall` + `swapgs`
trampoline, and INIT-SIPI-SIPI SMP bring-up instead of PSCI/SBI. If the `arch/` split survives x86,
it is real. Also the reach: x86_64 is what most machines are.

**What x86_64 will stress** (DECISIONS §19's own list): a different boot world (UEFI/ACPI, not
device tree; no OpenSBI/PSCI analog), the APIC instead of GIC/PLIC, a third page-table format
behind the `paging` seam, and TSO memory ordering -- where rule #4's weak-first discipline finally
pays out in the direction it was bet on: code proven correct on a weak machine (ARM, RISC-V) is
correct on TSO, and nothing about x86-first development could have said the reverse. The PCIe
transport (§18) is already x86's native bus, and the ECAM bridge both `virt` boards already use is
the same `pci-host-ecam-generic` shape x86 presents through ACPI.

## Sequencing against the physical machine (milestone 87)

**Starts under QEMU now; does not wait for the real machine.** Milestone 87 tracks the physical
OptiPlex 7050's bring-up (a real 16550 COM port, VT-d, an `igb`/`e1000e`-family NIC QEMU can stand
in for, four real cores, remote power cycling) and is the eventual parity-proof hardware, the same
role the VisionFive 2 played for RISC-V (milestone 16). QEMU's `q35` machine emulates the same
16550 UART the real hardware carries, so the boot/serial driver spans both from the start, the
NS16550/PL011 pattern both existing ISAs already follow.

**As of 2026-08-23: the Dell C4PDJ serial module and the dev-side RS-232 chain have arrived and
are installed.** Milestone 87's remaining blocker (recorded 2026-08-18 as *"the machine is here
and the serial module is not"*) is closed on the hardware side; what remains for 87 to complete is
the actual bring-up (boot code, the UART driver, printing a byte over serial on real silicon),
which is downstream of this milestone's own boot/console work reaching a point worth trying on the
box. See milestone 87 for the machine's own status.

## What this does not decide

The exact boot path (a minimal UEFI stub vs. a bootloader) and the SMP bring-up sequence's precise
shape are implementation judgment for whoever builds this, not decided here -- DECISIONS §19 named
the stress points, not the mechanism.

## What it unblocks

Architectural parity (DECISIONS §19) reaching all three declared targets rather than two. Milestone
25's `sel4bench` cross-OS comparison, once real x86 hardware (87) is behind it, gets a third
comparison point.
