//! **The `x86_64` architecture layer.** The third implementation of the `arch/` contract (milestone
//! 161, notes/x86-port.md), and the one milestone 20 said would be the real test of it: the first
//! two architectures are both RISC machines with a device tree, weak memory and a similar MMU, and
//! this one is none of those things.
//!
//! **This is a partial port, and it says so in every remaining stub.** What is real: the boot path
//! (a 32-bit multiboot-style trampoline into long mode and the high half), the GDT and TSS, the IDT
//! and the trap frame, the console UART over port I/O, the page-table format, the fine-grained W^X
//! kernel map, the local APIC and a calibrated timer, the IO APIC and a routed device line, user
//! address spaces, the `syscall` pair and ring 3, the address arithmetic, the interrupt-masking
//! primitives, the context switch, and the test exit. What is not: VT-d and SMP bring-up, each an
//! `unimplemented!()` that names itself and the reason, so that nobody mistakes a stub for a working
//! port.
//!
//! **And one thing that is real only at this layer**, which is worth saying here because this module
//! is where a reader looks first: a program runs at ring 3, but there is no *process* behind it. The
//! scheduler, the kernel heap and the untyped budget have never been brought up on this
//! architecture, so `user::run` and `KernelStack::new` have nothing to stand on. See
//! design/roadmap/161-x86-64-kernel-port.md, item 4.
//!
//! # What this port has already shown about the seam
//!
//! Two things, both worth more than the code. The `paging` crate's split into a generic level walk
//! plus a per-architecture entry codec **held**: `paging::x86_64::Ia32e` is 60 lines and nothing
//! above it changed. And the 16550 driver spans a *different address space* (x86 port I/O rather
//! than MMIO) as a type parameter rather than a second driver, which is the strongest evidence so
//! far that "a new ISA is a new directory" is true rather than aspirational.
//!
//! # And two places it did not hold, until calef ratified the rename
//!
//! **`arch::psci_cpu_on` was an aarch64 name that leaked**, and RISC-V already had to implement it
//! as an SBI call under an ARM firmware interface's name. x86 has no third mechanism to hide behind
//! it: SMP bring-up here is INIT-SIPI-SIPI through the local APIC, sent by the interrupt controller,
//! at a physical page below 1 MiB, in 16-bit real mode. Ratified as `arch::cpu_start`.
//!
//! **The single pointer `kernel_main` took was called `dtb`.** x86 has no device tree; what arrives
//! there is PVH's `hvm_start_info`, which carries the memory map and the ACPI RSDP address, so the
//! *shape* was right and only the name was wrong. Ratified as `boot_info_pointer`.

use core::arch::{asm, global_asm};

// The AP real-mode trampoline's copy-and-prepare step (milestone 161's SMP item). See its own
// header, and `boot.s`'s `secondary_boot` for what it prepares.
pub mod ap_boot;
pub mod context;
pub mod exceptions;
pub mod interrupts;
pub mod iommu;
pub mod irq;
pub mod isa;
// What the loader said (milestone 161): the kernel side of `machine_discovery::x86_64`.
pub mod machine;
pub mod mmu;
pub mod port;
pub mod segments;
pub mod semihosting;
pub mod timer;

// The saved thread context and how a new one is faked (the Rust half of context.s). Re-exported
// flat so `crate::arch::{Context, switch_to}` names them regardless of architecture.
pub use context::{Context, switch_to};
// How the console reaches its UART's registers on this architecture. Named flat through `arch`
// because `console.rs` picks it by `target_arch` and must not reach into `arch::x86_64::` directly.
pub use port::PortIo;

// The 32-bit entry (_start), the long-mode transition, the .bss zeroing, and the stack handoff to
// `kernel_main`.
global_asm!(include_str!("boot.s"));

// The context switch and the two first-run trampolines (the asm half of context.rs).
global_asm!(include_str!("context.s"));

// The 256 trap stubs, the shared restore path, the `syscall` entry, and the door into ring 3
// (the asm half of exceptions.rs). The constants are substituted rather than duplicated: a 64-bit
// assembler cannot read a Rust `const`, and the alternative is two files that can drift. The three
// `*_OFF` ones are `gs`-relative byte offsets into `cpu::PerCpu::x86_trap` (milestone 161's SMP
// item), computed by `core::mem::offset_of!` rather than hand-counted, so a field added or reordered
// in `cpu.rs` cannot silently desynchronize the assembly that reaches through them.
global_asm!(
    include_str!("trap.s"),
    USER_CODE = const segments::USER_CODE as u64,
    USER_DATA = const segments::USER_DATA as u64,
    SYSCALL_VECTOR = const exceptions::SYSCALL_VECTOR,
    TSS_RSP0_PTR_OFF = const core::mem::offset_of!(crate::cpu::PerCpu, x86_trap.tss_rsp0_ptr),
    SYSCALL_KERNEL_RSP_OFF =
        const core::mem::offset_of!(crate::cpu::PerCpu, x86_trap.syscall_kernel_rsp),
    SYSCALL_USER_RSP_OFF =
        const core::mem::offset_of!(crate::cpu::PerCpu, x86_trap.syscall_user_rsp),
);

/// `IA32_GS_BASE`: the MSR holding the base of the `gs` segment, and this architecture's answer to
/// aarch64's `TPIDR_EL1` and RISC-V's `tp`. A per-CPU register the kernel owns.
///
/// **There is a second one, and it is now load-bearing.** `IA32_KERNEL_GS_BASE` (0xC0000102) holds
/// the *other* value, and `swapgs` exchanges the two. That pair is how a trap from ring 3 recovers
/// the kernel's per-CPU pointer without trusting anything the user program could have set, which is
/// exactly the problem RISC-V solves with `sscratch`. The convention this kernel keeps, stated once
/// here because it is the sort of thing that is otherwise only true by accident: **while executing
/// in ring 0, `IA32_GS_BASE` names the per-CPU block and `IA32_KERNEL_GS_BASE` holds the user's
/// value; in ring 3 they are the other way round.** trap.s does the swapping, guarded on the
/// interrupted CPL, and its header says why the guard rather than a bare instruction.
const IA32_GS_BASE: u32 = 0xC000_0101;

/// Read a model-specific register. `rdmsr` returns the value split across `edx:eax`, with the
/// register number in `ecx`.
///
/// **Name provisional** (milestone 161): calef names public functions (AGENTS.md, milestone 160),
/// and this one was minted by a lane.
///
/// # Safety
/// `msr` must be a register this CPU implements. Reading one it does not is a general protection
/// fault, and `CPUID` is the only way to know for the optional ones.
pub unsafe fn read_msr(msr: u32) -> u64 {
    let low: u32;
    let high: u32;
    // SAFETY: the caller's contract. A read has no architectural side effects.
    unsafe {
        asm!(
            "rdmsr",
            in("ecx") msr,
            out("eax") low,
            out("edx") high,
            options(nomem, nostack, preserves_flags),
        );
    }
    ((high as u64) << 32) | low as u64
}

/// Write a model-specific register. The counterpart of [`read_msr`], with the same split.
///
/// **Name provisional** (milestone 161).
///
/// # Safety
/// `msr` must be one this CPU implements, and `value` must be legal for it. An MSR write is one of
/// the few instructions that can change what mode the machine is in (`IA32_EFER` alone controls
/// long mode, `NXE` and `syscall`), so this is exactly as dangerous as what the caller names.
pub unsafe fn write_msr(msr: u32, value: u64) {
    // SAFETY: the caller's contract.
    unsafe {
        asm!(
            "wrmsr",
            in("ecx") msr,
            in("eax") value as u32,
            in("edx") (value >> 32) as u32,
            options(nomem, nostack, preserves_flags),
        );
    }
}

/// The logical id of the CPU the kernel boots on.
///
/// **Read from `CPUID`, not hardcoded** (milestone 161's SMP item). This used to return the
/// constant 0, on the reasoning that the boot processor is selected by hardware rather than by
/// firmware choice the way RISC-V's boot hart is. That is true and beside the point: the number
/// this returns has to agree with the *roster's* seating (`smp::seat_cpus_from_acpi`, which seats
/// every core, boot core included, at the slot its own local APIC id names, the same
/// logical-id-equals-hardware-id invariant `read_cpu_list` gives the other two architectures), and
/// nothing guarantees the boot CPU's local APIC id is 0 in general, only that it usually is on
/// QEMU.
///
/// `CPUID` leaf 1, `EBX[31:24]` ("Initial APIC ID") is the same local APIC id the MADT's
/// `LocalApic` entries report, and unlike the local APIC's own ID register it needs no MMIO and no
/// prior bring-up: it is available from the very first instruction, which is exactly why
/// `cpu::init_this_cpu(arch::boot_cpu_id())` can call this before the console, the GDT, or ACPI
/// exist.
pub fn boot_cpu_id() -> usize {
    // `__cpuid` is a safe function (see `isa::init`'s own comment); leaf 1 is architected on every
    // CPU this kernel runs on, so no maximum-leaf check is needed the way leaf 7 wants one.
    let leaf1 = core::arch::x86_64::__cpuid(1);
    ((leaf1.ebx >> 24) & 0xff) as usize
}

/// Set this CPU's per-CPU pointer, by writing the `gs` segment base.
pub fn set_percpu(ptr: usize) {
    // SAFETY: `IA32_GS_BASE` exists on every long-mode CPU, and this value is the per-CPU block
    // this kernel reserves the register for.
    unsafe { write_msr(IA32_GS_BASE, ptr as u64) };
}

/// Read this CPU's per-CPU pointer (the value last handed to [`set_percpu`]).
pub fn percpu() -> usize {
    // SAFETY: `IA32_GS_BASE` exists on every long-mode CPU; a read has no side effects.
    unsafe { read_msr(IA32_GS_BASE) as usize }
}

/// **Test-only: does the per-CPU pointer name the CPU we are physically running on?**
///
/// A constant `true` here, and for aarch64's reason rather than RISC-V's. RISC-V keeps the pointer
/// in `tp`, an ordinary register a trap frame carries, so it can go stale when a preempted thread
/// resumes on a different hart. `IA32_GS_BASE` is an MSR: it is not saved or restored by any context
/// switch and does not travel with a thread, so there is nothing that could make it stale. The
/// independent ground truth (the local APIC id) is not readable yet either way.
#[cfg(test)]
pub fn percpu_matches_hart() -> bool {
    true
}

/// Start a secondary CPU via INIT-SIPI-SIPI (milestone 161's SMP item). `target_cpu` is a local
/// APIC id (the arch contract's "hardware id", `smp::bring_up_secondaries` reads it out of the
/// roster `smp::seat_cpus_from_acpi` built), `entry` is the trampoline's own physical page
/// (`ap_boot::trampoline_phys()`, handed back through `secondary_boot_entry`, see below), and
/// `context` is the stack top the trampoline hands to `secondary_main`.
///
/// **Blocks until the core it started is fully online, or has been given up on**, which is not
/// `smp::bring_up_secondaries`'s usual contract (aarch64 and RISC-V return as soon as the firmware
/// call is accepted) but is required here: there is exactly one trampoline scratch page, shared by
/// every `STARTUP` IPI this kernel ever sends, and `bring_up_secondaries` starts one core per call
/// with nothing else serializing them. Waiting for `online_count()` to move is what makes the next
/// call's `ap_boot::prepare` safe to overwrite that page.
///
/// # BUGS
/// **`entry` must equal `ap_boot::trampoline_phys()`.** The arch contract passes whatever
/// `smp::bring_up_secondaries` computed for `secondary_boot`'s address, which on this architecture
/// *is* that trampoline page (see `secondary_boot_entry`'s own doc), so this holds by construction
/// today; it is not re-derived here because there is nowhere else it could sensibly come from.
pub fn cpu_start(target_cpu: u64, entry: u64, context: u64) -> i64 {
    if !irq::local_apic_ready() {
        return -1;
    }
    debug_assert_eq!(
        entry,
        ap_boot::trampoline_phys(),
        "cpu_start's entry is not the AP trampoline's own page"
    );
    // SAFETY: this function does not return until the core it is about to start has either come
    // fully online or been given up on (see the wait loop below), which is what keeps two calls
    // from ever overwriting the shared trampoline page while an earlier one is still in use.
    unsafe { ap_boot::prepare(context) };

    let dest = target_cpu as u8;
    let before = crate::smp::online_count();

    // The universal INIT-SIPI-SIPI startup algorithm (Intel MP spec, appendix B.4): INIT, a settle
    // delay, then a STARTUP IPI.
    irq::send_init(dest);
    busy_wait_us(10_000);
    let vector = (entry >> 12) as u8;
    irq::send_startup(dest, vector);
    busy_wait_us(200);

    // **The second STARTUP IPI is conditional, not unconditional.** The MP spec calls for two, "the
    // second is a no-op on a core that already started", meaning a core that has already left the
    // wait-for-SIPI state is defined to ignore a further one. This port does not fully trust that on
    // QEMU TCG (see `ap_boot`'s own `BUGS`: a real, unexplained intermittent failure exists when a
    // third or later secondary starts while an earlier one is already running, and an accepted
    // second SIPI re-vectoring an already-running core back to the trampoline mid-execution was one
    // hypothesis for it). Sending the second IPI only when the first evidently has not worked yet
    // (`online_count` has not moved in the 200 us the spec allows for it to) costs nothing when the
    // first succeeds and is strictly more conservative than sending it unconditionally, so it is
    // kept even though a direct test did not show it fixing the deeper issue on its own.
    if crate::smp::online_count() == before {
        irq::send_startup(dest, vector);
        busy_wait_us(200);
    }

    // Bounded: a core the firmware silently declines to start must not hang bring-up.
    //
    // **Ten seconds, not one.** The delays above are the real thing's timing, sub-millisecond on
    // real hardware; the budget below is not that, it is how long CPU 0 waits for the SIPI'd core to
    // run its own trampoline, adopt the fine map, and reach `secondary_main`'s online mark, all of
    // it QEMU TCG instructions on whatever host thread the emulator's own scheduler gets around to
    // next. Measured too short once already, at one second: the target core's own vCPU thread had
    // simply not been scheduled by the host yet, and `cpu_start` gave up and reported "did not
    // start" for a core that came up perfectly well a moment later, arriving too late to be counted
    // and permanently invisible to the roster. This is exactly the host-contention shape
    // `smp::tests::wait_for`'s own sixty-second budget exists for, one level earlier: bring-up
    // itself can be starved the same way a test's own wait can.
    //
    // **`hlt` between checks, not a tight spin.** TCG runs each vCPU as one host thread, and
    // `spin_loop`'s `pause` hint does nothing for *host* scheduling; a CPU 0 that never stops
    // consuming its host thread's time slice can only make it harder for the vCPU thread whose
    // progress this loop is waiting on to get scheduled, on a busy host (this project's own recorded
    // condition; AGENTS.md, "other lanes running in parallel"). `wait_for_interrupt` parks CPU 0 on
    // `hlt` until its own local APIC timer (armed at `TICK_HZ`, already ticking: interrupts are
    // enabled before `bring_up_secondaries` runs) wakes it, which costs at most one tick of latency
    // per check and, unlike the spin, actually yields host CPU time. Measured to turn an occasional
    // full hang (waiting past even a sixty-second budget) into a reliable, clean give-up within the
    // stated budget when a core does not come up; it did not, on its own, fix the deeper reason a
    // core sometimes does not come up (`ap_boot`'s own `BUGS`), so it is kept for the failure mode it
    // does fix rather than claimed as a fix for the one it does not.
    let before = crate::smp::online_count();
    let budget = 10 * crate::arch::timer::frequency();
    let start = crate::arch::timer::now();
    while crate::smp::online_count() == before {
        if crate::arch::timer::now().wrapping_sub(start) >= budget {
            return -1;
        }
        wait_for_interrupt();
    }
    0
}

/// Busy-wait roughly `us` microseconds, against the calibrated TSC. No `wfi`/`hlt` equivalent
/// here: INIT-SIPI-SIPI's delays are a handful of instructions on real hardware, and parking this
/// core for them would need an interrupt to wake it that nothing is going to send.
fn busy_wait_us(us: u64) {
    let ticks = crate::arch::timer::frequency() / 1_000_000 * us;
    let start = crate::arch::timer::now();
    while crate::arch::timer::now().wrapping_sub(start) < ticks {
        core::hint::spin_loop();
    }
}

/// Can this machine start a secondary CPU at all? Yes, once the local APIC is up: INIT-SIPI-SIPI is
/// sent *through* it, unlike PSCI or SBI, which need no device at all before the first call.
pub fn can_start_secondaries() -> bool {
    irq::local_apic_ready()
}

/// Print how this machine starts a CPU. One line, on every boot, beside the SMP count.
pub fn print_bring_up_mechanism() {
    if irq::local_apic_ready() {
        crate::println!(
            "  smp: init-sipi-sipi via the local apic, trampoline at {:#x}",
            ap_boot::trampoline_phys()
        );
    } else {
        crate::println!(
            "  smp: the local apic is not up, so no core can be started here (init-sipi-sipi needs it)"
        );
    }
}

/// **The physical address `smp::bring_up_secondaries` hands `cpu_start` as `entry`.**
///
/// On aarch64 and RISC-V this is `virt_to_phys(secondary_boot)`: their `secondary_boot` is an
/// ordinary high-linked label, and firmware wants its physical address. Here `secondary_boot` is
/// **already** physical: link-x86_64.ld gives it the fixed low virtual address
/// (`AP_TRAMPOLINE_PHYS`) it has to execute from, because a `STARTUP` IPI can only name a page
/// below 1 MiB, so its own address *is* that page and converting it again would be wrong (this was
/// milestone 161's first named bug: `smp::bring_up_secondaries` used to call `virt_to_phys` on it
/// unconditionally, computing `secondary_boot's address - DIRECT_MAP_BASE`, an underflow, since
/// `secondary_boot`'s address is nowhere near the direct map).
pub fn secondary_boot_entry(secondary_boot_addr: u64) -> u64 {
    secondary_boot_addr
}

/// Bring this CPU's architecture state up: the GDT and TSS, then the IDT. The order is forced, and
/// not obviously: an IDT entry names a code **selector**, so the GDT that selector indexes has to be
/// the one installed before any trap can be delivered through it.
pub fn init() {
    // SAFETY: called once per CPU during boot, with a valid stack, before interrupts are unmasked.
    unsafe { segments::init() };
    exceptions::init();
    // And the second door into the kernel, which shares none of the first one's machinery: four
    // MSRs, no gate and no descriptor. It goes here rather than with ring 3 because it is per-CPU
    // state like the GDT and the IDT, and because a `syscall` before it is programmed is a jump to
    // whatever `IA32_LSTAR` holds, which on a cold machine is zero.
    //
    // SAFETY: `segments::init` above installed the GDT these selectors index, and nothing has
    // entered ring 3 (nothing can: this is the boot CPU's own bring-up).
    unsafe { exceptions::init_syscall() };
}

/// Stop this CPU forever, cheaply. `hlt` parks it until an interrupt; with interrupts masked and
/// nothing left to wake it, that is the rest of time at zero host CPU. The same discipline as the
/// other two architectures' `wfi`. See CLAUDE.md, "Never leave QEMU running".
pub fn halt() -> ! {
    loop {
        // SAFETY: halting until the next interrupt is always safe; it only affects when the next
        // instruction runs.
        unsafe { asm!("hlt", options(nomem, nostack)) };
    }
}

/// Park until the next interrupt (the scheduler's idle primitive).
pub fn wait_for_interrupt() {
    // SAFETY: as `halt`, but returns when an interrupt arrives.
    unsafe { asm!("hlt", options(nomem, nostack)) };
}

/// This CPU's current stack pointer, for the stack-overflow canary check (stack.rs).
pub fn current_sp() -> u64 {
    let rsp: u64;
    // SAFETY: reads a register. No side effects.
    unsafe { asm!("mov {}, rsp", out(reg) rsp, options(nomem, nostack, preserves_flags)) };
    rsp
}

/// A DMA write memory barrier: order all prior stores before any device sees a later one.
///
/// **`sfence`, and the reason it is nearly free is the whole reason x86 is worth porting to.** This
/// machine is TSO: stores are already globally ordered with respect to each other, so the ordinary
/// case needs no instruction at all. `sfence` is here because non-temporal stores and
/// write-combining memory are the exceptions TSO does not cover, and a DMA buffer can legitimately
/// be either.
///
/// This is where rule #4's bet pays out in the direction it was made. Code proven correct under
/// ARM's weak model and RISC-V's RVWMO is correct here by construction; nothing about developing on
/// x86 first could have said the reverse, and the tree would have accumulated invisible
/// strong-ordering assumptions that only a real port would have found.
pub fn dma_wmb() {
    // SAFETY: a fence has no memory effect of its own; it only constrains ordering.
    unsafe { asm!("sfence", options(nostack, preserves_flags)) };
}

/// Make the instruction fetcher aware of code just written as data.
///
/// **Nothing to do, and this is the one place x86's complexity buys something.** The instruction
/// cache on x86 is architecturally coherent with the data caches: a store to an address, followed
/// by a fetch of that address, sees the store, on this core and on every other, with no
/// software action at all. aarch64 needs a clean/invalidate loop over cache lines and a broadcast;
/// RISC-V needs `fence.i` locally and an SBI RFENCE remotely (and getting that wrong is what hung
/// init on first silicon). Here the guarantee is the hardware's.
///
/// A serialising instruction is still required if the *modifying* store and the fetch are separated
/// by a jump the CPU may have already speculated past, which the callers here are not doing.
pub fn sync_icache(va: u64, len: usize) {
    let _ = (va, len);
}
