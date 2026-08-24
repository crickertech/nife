//! **The `x86_64` architecture layer.** The third implementation of the `arch/` contract (milestone
//! 161, notes/x86-port.md), and the one milestone 20 said would be the real test of it: the first
//! two architectures are both RISC machines with a device tree, weak memory and a similar MMU, and
//! this one is none of those things.
//!
//! **This is a scaffold, and it says so in every stub.** What is real: the boot path (a 32-bit
//! multiboot-style trampoline into long mode and the high half), the GDT and TSS, the IDT and the
//! trap frame, the console UART over port I/O, the page-table format, the address arithmetic, the
//! interrupt-masking primitives, the context switch, and the test exit. What is not: the
//! fine-grained page tables, the APIC, any clock, VT-d, SMP bring-up, and ring 3. Each of those is
//! an `unimplemented!()` that names itself and the reason, exactly as the RISC-V port shipped on
//! its first day, so that nobody mistakes a stub for a working port.
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

// The 256 trap stubs and the common save/restore path (the asm half of exceptions.rs).
global_asm!(include_str!("trap.s"));

/// `IA32_GS_BASE`: the MSR holding the base of the `gs` segment, and this architecture's answer to
/// aarch64's `TPIDR_EL1` and RISC-V's `tp`. A per-CPU register the kernel owns.
///
/// **There is a second one, and knowing why matters before user mode exists.** `IA32_KERNEL_GS_BASE`
/// (0xC0000102) holds the *other* value, and the `swapgs` instruction exchanges the two. That pair
/// is how a trap from ring 3 recovers the kernel's per-CPU pointer without trusting anything the
/// user program could have set, which is exactly the problem RISC-V solves with `sscratch`. Nothing
/// uses it yet because nothing runs in ring 3; trap.s records where the `swapgs` goes.
const IA32_GS_BASE: u32 = 0xC000_0101;

/// The logical id of the CPU the kernel boots on. Always 0 on x86: unlike RISC-V, where firmware
/// picks a boot hart and the spec does not require it to be hart 0, the x86 boot processor is
/// selected by hardware and is the one running when the kernel is entered. Its *APIC* id need not be
/// 0, and that is a separate number this port does not yet read (it is in the MADT).
pub fn boot_cpu_id() -> usize {
    0
}

/// Set this CPU's per-CPU pointer, by writing the `gs` segment base.
pub fn set_percpu(ptr: usize) {
    // SAFETY: writes a per-CPU MSR the kernel reserves for its own per-CPU block. `wrmsr` takes the
    // value split across edx:eax and the MSR number in ecx.
    unsafe {
        asm!(
            "wrmsr",
            in("ecx") IA32_GS_BASE,
            in("eax") ptr as u32,
            in("edx") (ptr >> 32) as u32,
            options(nomem, nostack, preserves_flags),
        );
    }
}

/// Read this CPU's per-CPU pointer (the value last handed to [`set_percpu`]).
pub fn percpu() -> usize {
    let low: u32;
    let high: u32;
    // SAFETY: reads an MSR. No side effects.
    unsafe {
        asm!(
            "rdmsr",
            in("ecx") IA32_GS_BASE,
            out("eax") low,
            out("edx") high,
            options(nomem, nostack, preserves_flags),
        );
    }
    ((high as usize) << 32) | low as usize
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

/// Start a secondary CPU.
///
/// # BUGS
/// **Unimplemented, and the name is wrong.** See this module's header: the mechanism here is
/// INIT-SIPI-SIPI through the local APIC, which is neither PSCI nor anything resembling it, and the
/// arch contract's name for this operation is aarch64's firmware interface. Returns a nonzero error
/// rather than panicking, because that is what `smp::bring_up_secondaries` already expects from a
/// CPU that will not start, and this port genuinely cannot start one.
pub fn cpu_start(target_cpu: u64, entry: u64, context: u64) -> i64 {
    let _ = (target_cpu, entry, context);
    -1
}

/// Can this machine start a secondary CPU at all? **No**, and saying so plainly is what keeps
/// `smp::bring_up_secondaries` from trying and hanging.
pub fn can_start_secondaries() -> bool {
    false
}

/// Print how this machine starts a CPU. One line, on every boot, beside the SMP count.
pub fn print_bring_up_mechanism() {
    crate::println!("  smp: none yet (x86 uses INIT-SIPI-SIPI via the local APIC; milestone 161)");
}

/// Bring this CPU's architecture state up: the GDT and TSS, then the IDT. The order is forced, and
/// not obviously: an IDT entry names a code **selector**, so the GDT that selector indexes has to be
/// the one installed before any trap can be delivered through it.
pub fn init() {
    // SAFETY: called once per CPU during boot, with a valid stack, before interrupts are unmasked.
    unsafe { segments::init() };
    exceptions::init();
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
