# 161. The x86_64 kernel port: bring up the HAL's third architecture

**Status: PARTIAL.** Minted 2026-08-23, splitting real work out of milestone 20's stale text; early
boot built and gated the same day. **The kernel boots on QEMU's `q35`, reaches Rust in the high half
of a 4-level address space, prints over a 16550, installs a GDT/TSS and an IDT, catches a breakpoint
and steps over it, takes a calibrated timer interrupt, brings up the frame allocator, replaces the
boot map with fine-grained W^X page tables of its own, and halts.** What is built and what is still
open are spelled out at the bottom of this block; `notes/x86-port.md` is the working record. The
scope note below (milestone 20's "enough of each ISA to boot, confine a ring-3/U process, and run
the test suite") is unchanged and is not met: ring 3 does not exist here yet.
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


## What was built (2026-08-23, plus step 9 on 2026-08-24)

Ordered as it was built, because each step is what made the next one debuggable.

1. **The boot path.** `kernel/link-x86_64.ld` and `kernel/src/arch/x86_64/boot.s`: a 32-bit
   trampoline into long mode and the high half. The boot protocol is the surprise and is written up
   at length in both files and in `notes/x86-port.md`: Multiboot 1 **cannot** boot a 64-bit kernel
   (QEMU refuses the image rather than ignoring the header) and QEMU 11 has no Multiboot 2, so the
   image carries a **PVH** ELF note instead. PVH also hands over the ACPI RSDP address, which is the
   nearest thing x86 has to the single device-tree pointer the other two architectures pass.
2. **The console.** The existing NS16550 driver, unchanged in substance: x86 puts the same part at an
   I/O **port** rather than a memory address, so `drivers/ns16550.rs` gained a `RegisterSpace` type
   parameter (defaulting to MMIO, so nothing else changed) and `arch/x86_64/port.rs` supplies the
   `in`/`out` half. One driver, two address spaces, which is also what milestone 87's real machine
   will want.
3. **The trap path.** `segments.rs` (GDT + TSS, with the double fault on its own IST) and
   `exceptions.rs` + `trap.s` (an IDT, 256 generated stubs, one trap frame). Proven by taking an
   `int3` and returning from it. Before this line a fault is a triple fault and a silent machine
   reset.
4. **The page-table format**, behind milestone 20's existing seam: `crates/paging/src/x86_64.rs`,
   sixty lines of entry codec plus seven host tests and six Kani harnesses, with **no change to the
   generic walk**. That is the milestone-20 claim tested and holding.

5. **The boot handoff, parsed.** `crates/machine_discovery/src/x86_64.rs` decodes
   `hvm_start_info`, host-tested (seven tests), beside the two ISA records already in that crate; it
   is a parser, so it lives where a parser can be proved in milliseconds rather than in an emulator.
   The tour prints the memory map, which is what proves the handoff carries real data: nine regions,
   255 MiB usable, and the PCIe ECAM window reported as reserved at exactly the address
   `arch::mmu::PCI_ECAM_PHYS` hardcodes. **QEMU's PVH loader leaves the ACPI RSDP field zero**, so
   the root pointer has to be found by scanning the BIOS area, which is a prerequisite for the APIC
   step rather than a detail.

6. **ACPI**, which is what x86 has instead of a device tree.
   `crates/machine_discovery/src/acpi.rs` decodes the RSDP, the root-table walk, the SDT header, the
   MADT and the MCFG, host-tested (thirteen tests), beside the arch records rather than inside the
   x86_64 one because ACPI is not an x86 standard. The kernel side scans for the RSDP, since QEMU's
   PVH loader leaves the field zero, and every table's checksum is verified before it is believed.
   **Verified against real firmware**: five tables found on q35, the local APIC at `0xfee00000`, one
   enabled CPU, the 8259s reported present, the IO APIC at `0xfec00000`, and the ECAM window at
   `0xb0000000`, which is exactly what `arch::mmu::PCI_ECAM_PHYS` hardcodes and what the PVH memory
   map independently reports as reserved.

7. **The local APIC and a calibrated periodic timer.** `irq.rs` masks the 8259s (an obligation the
   MADT states, and their power-on vectors overlap the CPU's own exception vectors), enables the
   local APIC at the address ACPI gave, and owns the timer's LVT; `timer.rs` measures both the TSC
   and the APIC timer against the PIT in one ten-millisecond window, because x86 has four clocks and
   no architected way to ask any of them its rate. **Verified: 20 ticks in ~0.2 s at 100 Hz, 20
   routed, 0 spurious**, which proves the whole interrupt path rather than only the clock. A missed
   EOI is a hang on this architecture, so exactly one tick would have been the failure to expect.

8. **The frame allocator.** `memory::init` was split into a device-tree front end and
   `memory::bring_up_frames`, which takes a RAM slice and a forbidden slice and does not care where
   they came from; the x86 front end builds those from the PVH map. **Verified: 254 MiB total, 254
   MiB free, first frame 0x16f000.** The whole first megabyte is clipped rather than reserved (it
   holds the IVT, the BDA, the EBDA, the boot info, and the page a STARTUP IPI's vector can name),
   and the kernel image is the one entry on the forbidden list, which covers the boot page tables
   because the linker script puts `.boot_scratch` inside the image bounds.

9. **Fine-grained page tables, on a direct map with a base of its own** (2026-08-24, the open
   list's item 1 below). `mmu::init` builds a W^X four-level map and switches `CR3` to it:
   `.text` executable and read-only, `.rodata` neither writable nor executable, everything else
   non-executable, every guard page a hole, device registers device-typed, and **no identity map**.
   The direct map now covers all of physical memory rather than the low gigabyte.

   The address-space decision item 1 recorded was taken as recorded: **two bases**, Linux's, the
   image at `KERNEL_VA_BASE` where the code model pins it and the direct map at
   `0xffff888000000000` (PML4[273]), with `virt_to_phys` inverting both, which is Linux's `__pa()`
   shape. `crates/paging` again needed nothing.

   **The sequencing hazard was removed rather than sequenced around**, and that is the part worth
   carrying forward. Item 1 prescribed carrying the old alias across the `CR3` switch and dropping
   it afterwards, because `phys_to_virt` changes meaning at that instant and the frame bitmap is a
   stored `&'static mut [u8]` derived through it. Instead `boot.s` installs PML4[273] itself,
   pointing at the same low PDPT the identity map already used: eight bytes, no memory, and
   `phys_to_virt` means one thing for the machine's whole life. `mmu::device_va`, the foot gun that
   existed only because the direct map could not reach a device, was deleted outright rather than
   migrated.

   **Verified on q35 with `-m 256M`**: the tour keeps printing across the `mov cr3`, the timer
   keeps ticking, and the map costs **556 KiB of page tables for 254 MiB of RAM**. That 0.2% is the
   price of 4 KiB leaves and is recorded in `arch/x86_64/mmu.rs`'s `BUGS`, with the two other honest
   limits: `mmu::init` must draw its tables from below 4 GiB (all the boot map reaches), and
   `CR4.PGE` is still off, so the `G` bit the kernel's `Flags` set is ignored and every `mov cr3`
   flushes everything.

Plus the build wiring it all needs: `kernel/build.rs`, `.cargo/config.toml` (including the static
relocation model, which this target does not default to), `rust-toolchain.toml`,
`scripts/qemu-runner-x86_64.sh`, and an x86_64 pass in `script/lint`.

**The evidence that the `arch/` split is real**: making the *entire* kernel compile for a third
architecture took 42 compiler errors, every one of them a missing `arch::` name, and four `cfg` arms
in files that already had two. `crates/paging` needed nothing.

## What is still open

In the order it should be done, because each is a prerequisite for the next.

0. **The device-discovery seam**, milestone 20's other promised abstraction. Its **narrow half is
   built** (`memory::bring_up_frames`), because without it the frame allocator could not come up on
   a machine with no device tree and the port stopped there; `memory::init` is now explicitly a
   device-tree front end and `arch::x86_64::machine::bring_up_memory` is the x86 one. **The wide
   half is still owed and should be its own milestone**: the device *windows* (interrupt controller,
   RTC, UART interrupt line, PCIe ECAM) are still tree-only and stay `None` on x86, so ACPI answers
   for the ECAM window and PCI still reports nothing. notes/x86-port.md has the table of which fact
   has which source.
1. **Fine-grained page tables, and the address-space layout they force a decision about. BUILT
   2026-08-24**; see step 9 of "What was built" for what landed and what it cost. The number is kept
   in place rather than struck out because the items below cite each other by it.

   The decision this item existed to record (two bases, Linux's) was taken as recorded, and the
   hazard it flagged was removed rather than sequenced around: `boot.s` installs the direct map's
   PML4 entry itself, so `phys_to_virt` never changes meaning. `mmu::device_va` is gone. What is
   still open here is not the map but its **leaves**: `crates/paging` maps 4 KiB pages and nothing
   else, so the direct map costs 0.2% of RAM in page tables. 2 MiB and 1 GiB leaves are a change to
   the shared format trait that all three architectures would want, and they want their own
   milestone rather than an x86 patch.
2. **The IO APIC.** The *local* APIC and its timer are built and taking interrupts; what is left is
   routing a **device** line, which is the IO APIC's redirection table. The MADT already gives its
   address, its global-interrupt base, and the interrupt source overrides, and that last one is the
   trap: a legacy IRQ number is not an IO APIC input, because on essentially every PC the timer's
   IRQ 0 arrives as GSI 2. PCI *interrupt* routing is blocked beyond that, on AML rather than on
   tables, which is why MSI (which bypasses the routing entirely by writing a vector straight to the
   local APIC) is the path worth building. **Its register page is already mapped device-typed** by
   `mmu::init` (step 9 above), so this is redirection-table code rather than a page-table detour.
3. **Ring 3**: the `syscall`/`sysret` MSR setup, the `swapgs` pair in `trap.s` (whose location is
   marked), and `enter_user`. The syscall ABI is written down in `exceptions.rs`
   (`rax` + `rdi`/`rsi`/`rdx`/`r10`/`r8`/`r9`) and is **provisional**: it is a boundary rather than a
   habit (DECISIONS §10, §16) and nothing speaks it yet.

   **Item 1 cleared the ground for this and left three things named rather than done.** The low half
   of the kernel's tables is now genuinely empty (asserted, not assumed), so a process root has
   somewhere to put user pages. What remains in `arch/x86_64/mmu.rs` is the user block, still a wall
   of `unimplemented!()`: `share_kernel_half` (x86 is RISC-V's single-root model, so a process root
   must carry the kernel's high-half PML4 entries), `ttbr0_value` (here `root | pcid`), and the
   `CR3` switch. Two decisions come with them, both already written up in that module: **`CR4.PCIDE`
   is off**, so `crates/asid`'s tags mean nothing to the hardware and every switch flushes the whole
   TLB, and **`CR4.PGE` is off**, so the kernel's global mappings are not pinned across those
   flushes. Turning both on belongs with this item rather than before it, because until a context
   switch happens often enough to measure, neither can be shown to be worth its risk.
4. **The kernel test suite**, and therefore a `script/test` leg. Blocked on 3: the suite's
   userspace-exec tests hand-assemble machine code and its supervision fixtures are per-ISA stubs.
5. **SMP**, via INIT-SIPI-SIPI. The local APIC is up, so what remains is the Interrupt Command
   Register sequence and a real-mode trampoline copied below 1 MiB. Also needs a per-CPU TSS and a
   per-CPU GDT, since `TSS.RSP0` names a per-core stack.
6. **VT-d.** No longer blocked on table parsing: `machine_discovery::acpi` walks the root table
   generically, so finding the DMAR is adding a signature arm. What remains is the device itself.

Two things that are not steps but are owed:

- **A capability shape for port I/O.** On the other two architectures a device is a page, so a device
  capability is a mapping and the MMU enforces it. x86's legacy devices, the console UART included,
  are in an I/O space with no page tables; the only mechanism with the right granularity is the TSS
  I/O permission bitmap, which is per-task rather than per-page. `user::UART_PHYS` is zero on this
  architecture and that zero is the marker. Written up as **§121 (PROPOSED)**, since it is a change
  to what a capability *is* rather than an implementation choice. The number is provisional: a lane
  minted it against the current index, and the integrator owns it at merge.
- **Two arch-contract names that do not stretch to a third architecture**: `arch::psci_cpu_on` (an
  ARM firmware interface's name for an operation x86 performs by sending an interrupt) and
  `kernel_main`'s `dtb` argument (which carries `hvm_start_info` here). Both are naming decisions and
  so calef's; recorded in `notes/x86-port.md` and in `arch/x86_64/mod.rs`.
