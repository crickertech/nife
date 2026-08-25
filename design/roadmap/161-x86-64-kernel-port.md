# 161. The x86_64 kernel port: bring up the HAL's third architecture

**Status: PARTIAL.** Minted 2026-08-23, splitting real work out of milestone 20's stale text; early
boot built and gated the same day. **The kernel boots on QEMU's `q35`, reaches Rust in the high half
of a 4-level address space, prints over a 16550, installs a GDT/TSS and an IDT, catches a breakpoint
and steps over it, takes a calibrated timer interrupt, brings up the frame allocator, replaces the
boot map with fine-grained W^X page tables of its own, routes a real device line through the IO
APIC's redirection table, brings up the scheduler and preempts, runs a kernel thread, builds two
processes out of untyped memory and runs them at ring 3 (one invokes a capability it was granted and
exits, one is refused by the page tables and its death is delivered to its supervisor), gives both
regions back leaving the frame count where it found it, and halts. `script/test --arch x86_64` runs
97 of the kernel's own tests and skips 7.** What is built and what is still open are spelled out at
the bottom of this block; `notes/x86-port.md` is the working record. The scope note below (milestone
20's "enough of each ISA to boot, confine a ring-3/U process, and run the test suite") is **met** as
of 2026-08-24, on QEMU. What keeps this milestone PARTIAL rather than BUILT is items 0, 5 and 6, and
one thing none of them names: **no user program is compiled for `x86_64-unknown-none`**, so the
processes above are hand-assembled and thirty of the suite's test modules are behind `cfg(initrd)`.
That is item 4's own hand-off and is written up in its block.
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


## What was built (2026-08-23, plus steps 9 through 12 on 2026-08-24)

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

10. **The IO APIC, and a real device line routed through it** (2026-08-24, the open list's item 2
    below). `irq.rs` gained the redirection table: the `IOREGSEL`/`IOWIN` index-and-data register
    pair, an entry count read from the version register, every entry masked on the way in, and
    `route_gsi`/`mask_gsi`/`enable` writing one. The arch contract's `enable(intid)` now means what
    it means on the other two architectures, and `exceptions::enable_external` stopped being an
    `unimplemented!()` and became a documented no-op, because x86 has one interrupt gate
    (`RFLAGS.IF`) where RISC-V has two.

    **The trap the item flagged is real, and the fix is in the crate rather than the kernel.** A
    legacy IRQ number is not an IO APIC input: q35's MADT rewires ISA IRQ 0 to GSI 2 (the PIT is on
    pin 2; pin 0 carries the 8259 cascade) and IRQs 5, 9, 10 and 11 to level-triggered. Resolving
    that is pure logic over table bytes, so it is `machine_discovery::acpi::isa_irq_table`, with
    six host tests, one of which is the IRQ-0 case by name. The flags word is **two two-bit
    fields**, not two bits, and a test exists for exactly that misreading: `0x000d` is active
    *high*, level, and a decoder testing bit 1 for polarity would answer active low and arm a line
    that never fires.

    **Verified on q35**: `pit irq 0 -> gsi 2 on vector 0x32: 20 interrupts in ~0.2 s at 100 Hz`,
    with the local APIC timer masked for that window so the count is the PIT's alone. Twenty is the
    same number the local APIC timer produces in the same window against the same TSC, which is
    what makes it a measurement rather than a nonzero. The vector map is flat
    (`GSI_VECTOR_BASE + gsi`, base 0x30), so a stray vector in a fault report names its line by
    subtraction; it costs the ability to express a priority policy, and there is not one to express.

11. **Ring 3, the `syscall` pair, and a program the page tables refuse** (2026-08-24, the open
    list's item 3 below). Four pieces, and the order below is the order they have to work in.

    **The user address spaces.** `arch/x86_64/mmu.rs`'s user block stopped being a wall of
    `unimplemented!()`. x86 is **RISC-V's single-root model**, not aarch64's: `CR3` names the whole
    address space, so `share_kernel_half` copies PML4 entries 256..512 out of the kernel root into
    every process root, which shares the image (PML4[511]), the direct map (PML4[273]) and every
    device window in one `copy_from_slice`. Without it the `mov cr3` unmaps the instruction after
    it.

    **The `swapgs` pair, guarded on the interrupted CPL.** The guard is the whole design: `swapgs`
    is an *exchange*, so a trap from ring 0 that swapped would install the user's GS base while
    running kernel code, and the next nested trap would hand the kernel's back to the user. Both
    sites test the saved `CS`'s low two bits, which are the interrupted privilege level. It does
    **not** trip the hazard `segments.rs` documents (loading a segment register in long mode zeroes
    that segment's base MSR), because it writes the MSR pair and loads no selector; that was checked
    against the instruction's definition rather than assumed, since it is exactly the shape of bug
    that cost this port an afternoon once.

    **The `syscall` entry.** Four MSRs (`IA32_STAR`, `IA32_LSTAR`, `IA32_FMASK`, and `EFER.SCE`
    last, so the instruction becomes legal only after the address it jumps to exists), programmed in
    `arch::init` beside the GDT and the IDT because they are per-CPU state of the same kind. The
    entry path is the part with no counterpart on the other two architectures: `syscall` **does not
    switch stacks** and pushes nothing, so the first three instructions are `swapgs`, park the
    user's `rsp`, and load the kernel's. `segments::set_kernel_stack` writes `TSS.RSP0` and the
    `syscall` path's stack together, so the two doors into the kernel cannot come to name different
    stacks.

    **The return is an `iretq`, not a `sysretq`, and that is a decision rather than an omission.**
    `sysret` returns to `rcx` without checking it is canonical, and a non-canonical `rcx` faults *in
    ring 0 on the user's stack* (the shape of CVE-2012-0217); using it safely needs an explicit
    canonicality check and an `iretq` fallback. Sharing one restore path costs some tens of cycles
    per syscall and buys one `swapgs` rule with one place to get it wrong. Recorded in that
    function's `BUGS` as worth revisiting **with a benchmark**, once there is a syscall-heavy
    workload here to measure.

    **What it was proved with, and the honest size of the claim.** There is no ELF built for
    `x86_64-unknown-none` and no scheduler on this architecture, so the proof is a hand-assembled
    probe (`arch/x86_64/ring3_probe.s`) entered from the boot thread, the same shape the other two
    ports shipped on their first day. Verified on q35:

    ```
      ring 3      : a program ran at cpl 3 (cs 0x0023, ss 0x001b) and made 2 syscalls
                    the portable dispatcher answered BadSyscall (-6) to an unknown number
                    reading the kernel's .text at 0xffffffff80109000 from ring 3: Permission(Read)
    ```

    Three separate facts, and the third is the strongest. `cs 0x23` is the **hardware's** answer to
    what ring this is, since CPL is literally `CS[1:0]`. `BadSyscall (-6)` came out of the
    *portable* `crate::syscall::dispatch` through `TrapFrame::{syscall_nr, arg, set_arg}` and
    reached the program in `rdi`, which means the ABI accessors, the entry and the return all work;
    asking for a refusal is deliberate, because it is the one answer that dispatcher can give on a
    kernel with no scheduler. And `Permission(Read)` rather than `Translation(Read)` is the page
    tables saying the walk **found** the kernel's `.text` and refused it, which is the privilege
    boundary doing its job rather than an accident of an incomplete map.

    **Two latent bugs on the untested user-thread path were found and fixed while reading it.**
    `Context::for_user_thread` wrote the child's three arguments into `r13`/`r14`/`r15` while
    `user_entry_trampoline` read them from `r12`/`r13`/`r14`, so a child would have received
    `(0, arg0, arg1)`; and the trampoline did not reserve a trap frame's worth of stack, which
    milestone 71 established both other architectures need. Neither could have been caught before
    this item, because nothing had ever entered ring 3 here. Both are still unexecuted: that
    trampoline is the *scheduler's* entry path, and the self test enters through `enter_user`
    directly.

12. **The scheduler, real processes, and the kernel's own test suite** (2026-08-24, the open list's
    item 4). The distance between item 11's "a program ran at CPL 3" and "a process", closed.

    **Almost none of it was new x86 code, which is the milestone-20 claim holding a third time.**
    `kmem`, `untyped`, `sched` and `user::AddressSpace` are portable and came up here by being
    compiled for this target. What had to be written was the arch layer underneath them; what had to
    be *found* was four places where portable code encoded an assumption true of two architectures.

    **The trap path grew the split both other ports already had.** `x86_trap_handler` became
    `x86_trap_dispatch` (outer, on the interrupted thread's stack, where the deferred `schedule()`
    runs) plus `x86_trap_body` (inner, which may run on this CPU's interrupt stack), with
    `dispatch_on_interrupt_stack` in trap.s between them. Not an optimisation: `schedule()` parks the
    running `rsp` in the outgoing thread's `Context`, so calling it from a per-CPU stack would park a
    per-CPU address in a thread.

    **`TSS.RSP0` and the `syscall` path's stack are recomputed from the frame on every return to
    ring 3.** x86 has two doors from ring 3 and they find their stack differently; with a scheduler
    there is one kernel stack per thread, so the pair has to be re-pointed at every switch. The
    frame's own address is the answer (`rsp + 176` at the top of `isr_restore`, because milestone 71
    put every thread's `TrapFrame` at `stack_top - 176`), which makes the wrong state unrepresentable
    rather than a duty to remember. RISC-V does exactly this at exactly this point.

    **`send_reschedule` stopped being `unimplemented!()`**: the local APIC's ICR is an IPI send, and
    its "self" shorthand is also how this ISA raises an interrupt by hand, which puts x86 on
    **aarch64's** side of `sched`'s test-fixture split rather than RISC-V's. The intid for a local
    APIC source is its vector, because there is no controller input to name; legacy IRQs (0..15) and
    these vectors (0x20..0x2f) cannot collide.

    **The ring-3 fault arm became a real teardown**, and `ring3_probe.s` plus the
    `x86_enter_user_and_wait`/`x86_leave_user` pair were deleted, exactly as item 3 said they would
    be.

    **Four bugs in portable code, all one shape**: a default arm or an expression naming one of two
    answers, silently wrong at three. `thread.rs`'s stack area was `KERNEL_VA_BASE | 0x10_0000_0000`,
    which is the **identity** on x86 (that base already carries the bit), so every kernel thread
    stack would have been mapped over `.text`. `crates/elf`'s `EXPECTED_MACHINE` was
    `not(riscv64) => EM_AARCH64`, so the x86 kernel was compiled to accept **aarch64** binaries and
    refuse its own. `xtask`'s `ArchLegs` predicates were `self != the_other_one`. And
    `smp::bring_up_secondaries` computed the secondary entry with `virt_to_phys` before checking
    whether any core can be started, which panics here because `secondary_boot` is in `.boot`, linked
    at its physical address.

    **Verified on q35**, and the frame line is the one that means the most:

    ```
      scheduler   : up on 1 cpu, preempting at 100 Hz (idle thread registered)
      kernel task : a spawned thread ran and carried its captured state (0x16100004)
      userspace   : a process built from untyped ran at cpl 3 and sent 0x1610004 on a granted cap
                    thread 8589934594 died at pc 0x400005 on addr 0xa50000, delivered to its supervisor
                    two children cost 0 frames the first round and 0 the second (steady state)
    ```

    Two children, because the two halves of "a process" fail differently: one is built out of a
    single untyped region (address space, code page, stack page, TCB, two endpoints), granted a
    capability, dispatched to ring 3 **by the scheduler**, invokes that capability and exits; one
    faults and its death arrives on its supervision endpoint naming the thread, the pc and the
    address. The round runs **twice** because a first round pays first-use carves that are not leaks,
    and a system in steady state charges the second one zero. The first version of it reported
    sixteen frames a round going missing, which was true and was the demo's own fault.

    **And the suite: `script/test --arch x86_64` runs 97 tests, skips 7, in 2.7 seconds.**
    `--arch` takes a third value and the default runs all three legs, which is what DECISIONS §19
    means by parity being a gate. The seven skips each print a reason: two are fixtures the runner
    does not attach, four are SMP or roster facts a single-core machine with no device tree cannot
    have, and one is the device-tree magic, which PVH's `hvm_start_info` genuinely is not.

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
   has which source. **Item 2 made this sharper rather than smaller**: the boot tour now hands the
   MADT's answers to `arch::x86_64::irq` *directly*, so the interrupt controller has a working x86
   path that goes around `memory.rs` instead of through it, and COM1's interrupt line is discovered
   (`Acpi::isa_irqs[4]`) and unused.
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
2. **The IO APIC. BUILT 2026-08-24**; see step 10 of "What was built". The number is kept in place
   rather than struck out because the items below cite each other by it.

   The trap this item existed to flag turned out to be exactly as stated and is now proven rather
   than asserted: q35's MADT says ISA IRQ 0 arrives as GSI 2, and the resolution lives in
   `machine_discovery::acpi::isa_irq_table` where a host test holds it. What is still open here is
   **PCI interrupt routing**, which is blocked on AML rather than on tables, and **MSI**, which
   bypasses the redirection table entirely by writing a vector straight to the local APIC and is
   the path worth building for that reason. Neither is this item, and neither was.
3. **Ring 3. BUILT 2026-08-24**; see step 11 of "What was built" for what landed, what it proved,
   and the two things it deliberately did not. The number is kept in place rather than struck out
   because the items below cite each other by it.

   **What is proven is the arch layer, and the boundary between that and a process is where this
   item now stops.** A program runs at CPL 3 (the hardware's own `cs` reads `0x23`), makes syscalls
   that reach the *portable* `crate::syscall::dispatch` and get its answer back in `rdi`, returns to
   ring 3, and is refused by the page tables when it reads the kernel's `.text`. What is **not**
   proven is anything above the arch layer: there is no ELF loader path here, no capability grant,
   no argument passing beyond the three registers `TrapFrame::for_user_entry` sets, and no
   scheduler, so the program is a hand-assembled probe entered from the boot thread rather than a
   process. Item 4 is where that gap closes, and it is the reason item 4 is the next thing.

   The syscall ABI (`rax` + `rdi`/`rsi`/`rdx`/`r10`/`r8`/`r9`) is now **spoken and ratified**: it was
   a boundary rather than a habit (DECISIONS §10, §16), and one probe program agreeing with the
   kernel was not the same as a decision having been made, so it went up as
   [DECISIONS §124](../decisions/124-x86-64-syscall-abi.md), which calef ratified 2026-08-24.

   **The two `CR4` bits stay off, and that is now a recorded choice rather than a deferral.**
   `CR4.PCIDE` off means `crates/asid`'s tags have nowhere to live: PCID is `CR3[11:0]`, and with
   PCIDE clear those bits are reserved-zero, so writing a tag into them faults rather than tagging
   anything. `arch/x86_64/mmu.rs`'s `ttbr0_value` therefore drops the number and `flush_asid`
   flushes the whole TLB, each saying so in its own words rather than implying a selectivity the
   hardware does not have. `CR4.PGE` off means every `mov cr3` discards the kernel's mappings too.
   Both are worth turning on and neither can be *shown* to be worth it yet: nothing on this
   architecture switches address spaces often enough to measure, so turning them on now would buy a
   number nobody could check. That measurement arrives with the scheduler (item 4), which is where
   the decision belongs.

   **Item 4 landed and the measurement is now available but was not taken, deliberately.** There is
   a workload to measure against (the suite context-switches throughout, and `mmu::switch_user_root`
   already skips a `CR3` write that would change nothing, which is worth more here than on either
   other architecture precisely because PGE is off). Turning either bit on is calef's call and wants
   a number rather than an argument: `script/icount` has no x86 leg, so producing one is its own
   small piece of work rather than a line in item 4's diff.
4. **The scheduler, real processes, and the kernel test suite. BUILT 2026-08-24**; see step 12 of
   "What was built" for what landed, what it cost, and the four portable-code bugs it found. The
   number is kept in place rather than struck out because the items around it cite each other by it.

   All three hand-overs it named were taken: the `x86_enter_user_and_wait`/`x86_leave_user` pair and
   `ring3_probe.s` are deleted, and the ring-3 fault arm is a real teardown through `sched::fault`.
   The bring-up was indeed mostly portable code meeting a third architecture for the first time.

   **What is still open here is not the suite but its fixtures**, and it is the thing that now bounds
   this whole milestone rather than an item within it: **no user program is compiled for
   `x86_64-unknown-none`.** Consequences, each a concrete piece of work:

   - `crates/user_rt` has **seven places with an aarch64/RISC-V pair each and no fallback**, so
     nothing in `user/` compiles: `invoke5`, `yield_now`, `cap_delete`, `now`, `cntfrq`, and the
     `cfg`s inside `exit` and `trap`. It was thirteen until the `invoke5` collapse landed on main
     the same day, which is most of the work already done. Five of the seven are transliteration
     (`syscall` for `svc`/`ecall`; the ABI is written down as §124 and now spoken). **Two are a
     design fork**: `now()` and `cntfrq()` read `CNTVCT_EL0`/`CNTFRQ_EL0` and RISC-V's `time` CSR,
     and x86 has neither. `rdtsc` is the obvious answer and is a decision rather than a
     transliteration, because its rate is not architected and this kernel already measures it
     against the PIT.
   - `user/build.rs` cannot compile its C components for this target, so `c_shim` and `c_swappable`
     would not link even once `user_rt` is done.
   - `xtask` packs no x86 archive. Adding one needs a third arm in `read_stripped`'s cache tag (x86
     would collide with aarch64 under `"host"` today, silently, producing wrong measurements), a
     third arm in `boot_programs` (whose `_` catches x86 now), and a `target/init-measure-x86_64.txt`
     without which the trust root is empty and any boot program is refused as `Unmeasured`.
   - `scripts/qemu-runner-x86_64.sh` passes no `-initrd`.
   - `crates/elf` **is** ready: it accepts `EM_X86_64` as of this item.

   Until then, thirty test modules under `kernel/src/user/` are behind **`cfg(initrd)`**, a cfg
   `kernel/build.rs` emits for every target that has user programs. The cfg names the reason rather
   than the architecture on purpose: `#[cfg(not(target_arch = "x86_64"))]` would have said the same
   thing thirty times and been wrong the day this port can build them, in thirty places nobody would
   look. Six modules under `user/` do run here, on the hand-assembled programs in
   `kernel/src/user/x86_programs.rs`.

   **This wants its own milestone**, and it is the largest single piece of x86 work left: it is what
   stands between this port and the shell, the drivers and the services, all of which are userspace.

   **A second, narrower gap found once userspace could compile: `fs_server` cannot compile for
   this target at all**, because its vendored RedoxFS engine needs `aes`, and `aes` needs SSE this
   target disables. Tracked separately as **milestone 164**, since it is a toolchain problem rather
   than an item in this port's own sequence.
5. **SMP**, via INIT-SIPI-SIPI. The local APIC is up, so what remains is the Interrupt Command
   Register sequence and a real-mode trampoline copied below 1 MiB. Also needs a per-CPU TSS and a
   per-CPU GDT, since `TSS.RSP0` names a per-core stack.

   **Item 4 did three of these without meaning to**, which is worth knowing before this is scoped:
   the ICR is written and tested (`irq::send_ipi` and `irq::raise_self_interrupt`, the latter driving
   the scheduler's interrupt-delivery tests on every x86 run), `irq::send_reschedule` is real, and
   the reschedule vector has a handler arm that drains the inbox and serves a steal request. So the
   *cross-CPU scheduler plumbing* is built and only its second CPU is missing. What item 4 also
   found, and what this item has to fix rather than inherit: `smp::bring_up_secondaries` converts the
   secondary entry with `arch::mmu::virt_to_phys`, and that is the wrong conversion here, because
   `secondary_boot` is in `.boot` and the linker already places it at its physical address. And
   `send_reschedule` uses the logical cpu id as the destination local APIC id, which is true on one
   CPU and not in general; the MADT states the mapping and nothing reads it into a roster yet, which
   is also why three `smp` tests skip on this architecture.
6. **VT-d.** No longer blocked on table parsing: `machine_discovery::acpi` walks the root table
   generically, so finding the DMAR is adding a signature arm. What remains is the device itself.

One thing that is not a step but is owed:

- **A capability shape for port I/O.** On the other two architectures a device is a page, so a device
  capability is a mapping and the MMU enforces it. x86's legacy devices, the console UART included,
  are in an I/O space with no page tables; the only mechanism with the right granularity is the TSS
  I/O permission bitmap, which is per-task rather than per-page. `user::UART_PHYS` is zero on this
  architecture and that zero is the marker. Written up as
  [DECISIONS §121](../decisions/121-port-io-capability.md) (PROPOSED), since it is a change to what a
  capability *is* rather than an implementation choice. The number is provisional: a lane minted it
  against the current index, and the integrator owns it at merge.

**Resolved**: the two arch-contract names that did not stretch to a third architecture,
`arch::psci_cpu_on` and `kernel_main`'s `dtb` argument, were renamed `arch::cpu_start` and
`boot_info_pointer`; see `notes/x86-port.md` and `arch/x86_64/mod.rs`.
