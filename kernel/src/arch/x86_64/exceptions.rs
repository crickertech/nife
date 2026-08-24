//! **The IDT and the trap path, `x86_64`.** The third implementation of the arch contract aarch64
//! states with `VBAR_EL1` and RISC-V with `stvec`.
//!
//! The shape of the difference is worth naming, because it is what the assembly beside this file
//! exists to paper over. aarch64 has one vector base and sixteen 128-byte slots the hardware picks
//! between; RISC-V has one entry point and a cause register. x86 has 256 independent entry points
//! and tells the handler nothing, so the vector number has to be manufactured by having 256
//! different stubs (trap.s), and ten of the 32 architectural exceptions push an extra word the other
//! 246 vectors do not.
//!
//! What the rest of the kernel sees through this module is the same four things it sees on the
//! other two: a [`TrapFrame`], counters it can assert on, the last user fault, and an `init` that
//! makes faults reportable instead of fatal.

use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use super::segments::{IST_DOUBLE_FAULT, KERNEL_CODE};
use crate::arch::UserFault;

/// **The registers a trap saves, in exactly the order trap.s pushes them.**
///
/// The first fifteen fields are the general registers, lowest address first, which is the *reverse*
/// of the push order because the stack grows down. Then the vector number and error code the stub
/// manufactured, then the five words the CPU itself pushed.
///
/// `rsp` and `ss` are in the frame unconditionally, which is an `x86_64` change from 32-bit and is
/// load-bearing here: a long-mode interrupt pushes SS:RSP even when the ring does not change, so
/// there is exactly one frame layout rather than two, and `iretq` always pops five words.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct TrapFrame {
    pub rax: u64,
    pub rbx: u64,
    pub rcx: u64,
    pub rdx: u64,
    pub rsi: u64,
    pub rdi: u64,
    pub rbp: u64,
    pub r8: u64,
    pub r9: u64,
    pub r10: u64,
    pub r11: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,
    /// Which of the 256 entries was taken. Pushed by the per-vector stub, because the CPU does not
    /// say.
    pub vector: u64,
    /// The hardware error code, or the zero the stub substituted for a vector that has none.
    pub error_code: u64,
    pub rip: u64,
    pub cs: u64,
    pub rflags: u64,
    pub rsp: u64,
    pub ss: u64,
}

// trap.s pushes 22 quadwords and `add rsp, 16` at the end assumes the vector and error code are the
// two words directly above the register block. Both facts are checked here rather than trusted.
const _: () = {
    assert!(size_of::<TrapFrame>() == 22 * 8);
    assert!(core::mem::offset_of!(TrapFrame, vector) == 15 * 8);
    assert!(core::mem::offset_of!(TrapFrame, error_code) == 16 * 8);
    assert!(core::mem::offset_of!(TrapFrame, rip) == 17 * 8);
};

impl TrapFrame {
    /// The syscall number the caller passed.
    ///
    /// **`rax`, following Linux**, and that choice is worth stating because it is the one place a
    /// third architecture could gratuitously invent a third convention. The tree's shape is one
    /// number register plus six argument registers, with argument 0 doubling as the return
    /// (aarch64: `x8` + `x0`..`x5`; RISC-V: `a7` + `a0`..`a5`). Applying that shape to `x86_64`'s own
    /// `syscall` convention gives `rax` + `rdi`, `rsi`, `rdx`, `r10`, `r8`, `r9`.
    ///
    /// **`r10` rather than `rcx` is not a preference**, it is the instruction: `syscall` overwrites
    /// `rcx` with the return address and `r11` with the caller's RFLAGS, so the fourth argument
    /// cannot ride in the C ABI's fourth register. Every `x86_64` kernel makes the same substitution
    /// for the same reason.
    ///
    /// **Provisional.** Nothing speaks this ABI yet: the x86 boot halts before user mode, and no
    /// user program is built for the target. A syscall ABI is a boundary rather than a habit
    /// (DECISIONS §10, §16) and is exactly the kind of decision the "move fast on what can be
    /// undone" tenet calls expensive, so it is written here to be argued with rather than merged as
    /// settled. See design/roadmap/161-x86-64-kernel-port.md.
    pub fn syscall_nr(&self) -> u64 {
        self.rax
    }

    /// Syscall argument register `i`. See [`syscall_nr`](Self::syscall_nr) for the register list
    /// and why `r10` is in it.
    pub fn arg(&self, i: usize) -> u64 {
        match i {
            0 => self.rdi,
            1 => self.rsi,
            2 => self.rdx,
            3 => self.r10,
            4 => self.r8,
            5 => self.r9,
            _ => panic!("x86_64 syscall argument {i} does not exist (there are six)"),
        }
    }

    /// Set syscall argument/return register `i`. The return value and IPC message words ride back
    /// in the first three, exactly as they do on the other two architectures.
    pub fn set_arg(&mut self, i: usize, v: u64) {
        match i {
            0 => self.rdi = v,
            1 => self.rsi = v,
            2 => self.rdx = v,
            3 => self.r10 = v,
            4 => self.r8 = v,
            5 => self.r9 = v,
            _ => panic!("x86_64 syscall argument {i} does not exist (there are six)"),
        }
    }

    /// Build the frame that drops a brand-new thread to ring 3 at `entry` on `user_sp`, with `args`
    /// in the first three argument registers.
    ///
    /// The privilege change is carried entirely by the selectors: `iretq` returns to the ring named
    /// by the low two bits of the CS it pops, and pops SS:RSP too because it is changing ring. That
    /// is the whole mechanism, and it is why the GDT's layout in `segments.rs` is load-bearing.
    ///
    /// `rflags` is `IF | 0x2`. Bit 1 reads as one on every x86 since the 8086 and a frame without
    /// it is malformed; `IF` set is what keeps a tight-loop user thread preemptible, the same
    /// intent as RISC-V's SPIE and aarch64's DAIF = 0.
    ///
    /// # BUGS
    /// **Untested**: nothing enters ring 3 yet, and `enter_user` below is unimplemented, so this
    /// builds a frame nothing loads.
    #[allow(dead_code)]
    pub fn for_user_entry(entry: u64, user_sp: u64, args: [u64; 3]) -> Self {
        const RFLAGS_IF: u64 = 1 << 9;
        const RFLAGS_ALWAYS_ONE: u64 = 1 << 1;
        TrapFrame {
            rax: 0,
            rbx: 0,
            rcx: 0,
            rdx: args[2],
            rsi: args[1],
            rdi: args[0],
            rbp: 0,
            r8: 0,
            r9: 0,
            r10: 0,
            r11: 0,
            r12: 0,
            r13: 0,
            r14: 0,
            r15: 0,
            vector: 0,
            error_code: 0,
            rip: entry,
            cs: super::segments::USER_CODE as u64,
            rflags: RFLAGS_IF | RFLAGS_ALWAYS_ONE,
            rsp: user_sp,
            ss: super::segments::USER_DATA as u64,
        }
    }
}

/// One IDT entry: a 64-bit interrupt/trap gate.
///
/// The handler address is split across three fields for the same reason the TSS descriptor's is:
/// the layout grew from the 16-bit 286 outward and the new bits were appended rather than the old
/// ones widened.
#[repr(C)]
#[derive(Clone, Copy)]
struct IdtEntry {
    offset_low: u16,
    selector: u16,
    /// Bits 2:0 select an IST slot (0 = do not switch stacks); the rest is reserved.
    ist: u8,
    /// Present, DPL, and gate type. 0x8E is "present, DPL 0, 64-bit interrupt gate".
    type_attr: u8,
    offset_mid: u16,
    offset_high: u32,
    _reserved: u32,
}

impl IdtEntry {
    const fn empty() -> Self {
        Self {
            offset_low: 0,
            selector: 0,
            ist: 0,
            type_attr: 0,
            offset_mid: 0,
            offset_high: 0,
            _reserved: 0,
        }
    }

    /// An interrupt gate at `handler`. **Interrupt gate, not trap gate**, and the difference is one
    /// bit that decides whether the CPU clears `RFLAGS.IF` on entry. An interrupt gate does, so a
    /// handler starts with interrupts masked and cannot be re-entered by the same source before it
    /// has established anything. A trap gate does not, which is what you want for a deliberate
    /// software interrupt and not for anything else.
    fn interrupt_gate(handler: u64, ist: u8) -> Self {
        Self {
            offset_low: handler as u16,
            selector: KERNEL_CODE,
            ist: ist & 0b111,
            type_attr: 0x8E,
            offset_mid: (handler >> 16) as u16,
            offset_high: (handler >> 32) as u32,
            _reserved: 0,
        }
    }
}

const _: () = assert!(size_of::<IdtEntry>() == 16);

/// The IDT. 256 entries, 4 KiB, filled by [`init`] from the stub table trap.s exports.
static mut IDT: [IdtEntry; 256] = [IdtEntry::empty(); 256];

unsafe extern "C" {
    /// 256 stub addresses, one per vector, in vector order. Generated by trap.s.
    static ISR_STUBS: [u64; 256];
}

/// **How many interrupts were routed to a handler.** The portable counter every architecture's
/// interrupt tests read.
pub static ROUTED_IRQS: AtomicUsize = AtomicUsize::new(0);

/// **How many interrupts arrived with nothing to route them to.** A spurious interrupt is normal on
/// x86 in a way it is not elsewhere: the 8259 PIC manufactures vector 7 (or 15) when a line drops
/// between assertion and acknowledgement, and the local APIC has a dedicated spurious vector.
pub static SPURIOUS_IRQS: AtomicUsize = AtomicUsize::new(0);

/// **How many *device* interrupts arrived through the IO APIC**, as distinct from the local APIC's
/// own timer. Counted separately because the two prove different things: the timer proves the local
/// APIC delivers and the trap path returns, and this proves a line outside the CPU reached it.
pub static DEVICE_IRQS: AtomicUsize = AtomicUsize::new(0);

/// **How many system calls were taken.** Named for aarch64's `svc` instruction because that is the
/// arch contract's word; on x86 the mechanism will be `syscall`, which does not go through the IDT
/// at all.
pub static SVC_COUNT: AtomicUsize = AtomicUsize::new(0);

/// **How many user-mode faults were taken.**
pub static USER_FAULTS: AtomicUsize = AtomicUsize::new(0);

/// **How many breakpoints were taken.** `int3`, vector 3, which is what [`self_test`] uses to prove
/// the trap path round-trips.
pub static BRK_COUNT: AtomicUsize = AtomicUsize::new(0);

/// The last user fault, packed by [`UserFault::encode`]. 0 means no user thread has faulted.
static LAST_USER_FAULT: AtomicU64 = AtomicU64::new(0);
/// The faulting address of the fault [`LAST_USER_FAULT`] describes.
static LAST_USER_FAULT_ADDRESS: AtomicU64 = AtomicU64::new(0);

/// What the last user-mode fault was and where. `None` until one has happened.
pub fn last_user_fault() -> Option<(UserFault, u64)> {
    let code = LAST_USER_FAULT.load(Ordering::Relaxed);
    UserFault::decode(code).map(|f| (f, LAST_USER_FAULT_ADDRESS.load(Ordering::Relaxed)))
}

/// The user PC saved in the trap frame at `stack_top`.
///
/// # BUGS
/// **Unimplemented.** Its callers are the user-mode exec tests, which this port has not reached;
/// what it will read is the `rip` field of the [`TrapFrame`] the kernel stack top holds, which is
/// not yet built by anything. It panics rather than returning a plausible zero, because a zero here
/// would be read as "the program faulted at address 0".
pub fn user_pc(stack_top: u64) -> u64 {
    unimplemented!("x86_64 user_pc({stack_top:#x}): user mode is not built (milestone 161)")
}

/// Drop to ring 3 with the register state in `frame`.
///
/// # Safety
/// See the BUGS note: nothing calls this yet, and it is not implemented.
///
/// # BUGS
/// **Unimplemented.** The mechanism is an `iretq` through a frame whose CS names the DPL-3 code
/// selector, plus the `swapgs` pair trap.s does not have yet. See notes/x86-port.md.
pub unsafe fn enter_user(frame: *mut TrapFrame) -> ! {
    unimplemented!("x86_64 enter_user({frame:p}): user mode is not built (milestone 161)")
}

/// Unmask external interrupts at the controller.
///
/// **Nothing to do on this architecture, and that is a real difference rather than a stub.** RISC-V
/// has a second gate, `sie.SEIE`, that has to be opened before an external interrupt can be
/// delivered even with `sstatus.SIE` set. x86 has one gate, `RFLAGS.IF`, which
/// `arch::interrupts::enable` owns. What is *per source* here is the IO APIC's mask bit, which
/// `irq::enable` clears when it arms a line, and the local APIC's Task Priority Register, which
/// `irq::init_local_apic` sets to zero so no priority class is dropped.
///
/// # Panics
/// If the local APIC is not up. An empty function that silently succeeded before the controller
/// exists would let a caller believe interrupts are unmasked when nothing can deliver one.
pub fn enable_external() {
    assert!(
        super::irq::local_apic_ready(),
        "external interrupts are the local APIC's to deliver, and it is not up yet",
    );
}

/// Install the IDT on this CPU.
///
/// Every vector gets a gate, including the 224 that no hardware currently drives: an IDT entry that
/// is not present makes an unexpected vector a **general protection fault** whose error code is the
/// vector, which is a second puzzle stacked on the first. A present gate that prints "vector 87,
/// unexpected" is strictly more informative and costs 4 KiB.
///
/// The double fault is the one entry with an IST. It has to be: the reason a double fault happens
/// is that the CPU could not deliver a first exception, and the commonest cause of that is a stack
/// that cannot be pushed to. A handler that tries to push its frame onto the same broken stack
/// triple-faults, which QEMU reports as a silent machine reset and a real machine as a reboot.
pub fn init() {
    // SAFETY: single-threaded boot code on the boot CPU, before interrupts are unmasked. The stub
    // table is 256 entries and so is the IDT, so every index below is in bounds.
    unsafe {
        for vector in 0..256 {
            let ist = if vector == 8 { IST_DOUBLE_FAULT } else { 0 };
            IDT[vector] = IdtEntry::interrupt_gate(ISR_STUBS[vector], ist);
        }
    }

    let idtr = super::segments::DescriptorTablePointer {
        limit: (size_of::<[IdtEntry; 256]>() - 1) as u16,
        base: (&raw const IDT) as u64,
    };
    // SAFETY: `idtr` describes the table just filled, whose every entry is a well-formed gate
    // pointing at a stub in this image.
    unsafe {
        core::arch::asm!("lidt [{}]", in(reg) &idtr, options(readonly, nostack, preserves_flags));
    }
}

/// **Prove the trap path round-trips**, by taking a breakpoint and returning from it.
///
/// Returns the breakpoint count after the round trip, which the boot tour prints. The same shape as
/// the RISC-V twin, and it earns its place for the same reason: an IDT that is installed but whose
/// return path is wrong produces a kernel that runs fine until the first real fault and then
/// vanishes, and there is no other cheap moment to find that out.
pub fn self_test() -> usize {
    // SAFETY: `int3` raises vector 3, which the IDT above routes to the common handler. The handler
    // treats a breakpoint as an event to count and step over, so this returns to the next
    // instruction.
    unsafe { core::arch::asm!("int3", options(nomem, nostack)) };
    BRK_COUNT.load(Ordering::Relaxed)
}

/// Read `CR2`, which holds the faulting address after a page fault (vector 14) and nothing
/// meaningful otherwise. The x86 analog of `FAR_EL1` and of RISC-V's `stval`.
fn faulting_address() -> u64 {
    let cr2: u64;
    // SAFETY: reads a control register. No side effects.
    unsafe {
        core::arch::asm!("mov {}, cr2", out(reg) cr2, options(nomem, nostack, preserves_flags));
    };
    cr2
}

/// The names of the 32 architecturally defined exceptions, so a fault report says what happened
/// rather than only which number it was.
const EXCEPTION_NAMES: [&str; 32] = [
    "divide error",
    "debug",
    "non-maskable interrupt",
    "breakpoint",
    "overflow",
    "bound range exceeded",
    "invalid opcode",
    "device not available",
    "double fault",
    "coprocessor segment overrun",
    "invalid TSS",
    "segment not present",
    "stack-segment fault",
    "general protection fault",
    "page fault",
    "reserved (15)",
    "x87 floating-point exception",
    "alignment check",
    "machine check",
    "SIMD floating-point exception",
    "virtualization exception",
    "control protection exception",
    "reserved (22)",
    "reserved (23)",
    "reserved (24)",
    "reserved (25)",
    "reserved (26)",
    "reserved (27)",
    "hypervisor injection exception",
    "VMM communication exception",
    "security exception",
    "reserved (31)",
];

/// **The one Rust function every trap reaches**, called by trap.s with a pointer to the frame it
/// just built on the current stack.
///
/// # Safety
/// Called only from `isr_common`, which has just constructed a complete [`TrapFrame`] at the
/// address it passes. Nothing else may call it.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn x86_trap_handler(frame: *mut TrapFrame) {
    // SAFETY: `isr_common` built the frame directly below the pointer it passed, and no other
    // caller exists (the symbol is only referenced from trap.s).
    let frame = unsafe { &mut *frame };

    match frame.vector {
        // A breakpoint is an event to count and step over: `int3` is one byte and the CPU already
        // advanced rip past it, so returning is enough. This is what `self_test` proves.
        3 => {
            BRK_COUNT.fetch_add(1, Ordering::Relaxed);
        }
        // The local APIC timer. Counted, then acknowledged below with every other interrupt.
        v if v == super::irq::TIMER_VECTOR as u64 => {
            super::timer::tick();
            ROUTED_IRQS.fetch_add(1, Ordering::Relaxed);
            super::irq::end_of_interrupt();
        }
        // A device line, routed by the IO APIC's redirection table onto a vector this kernel chose
        // (irq::GSI_VECTOR_BASE plus the GSI). No device driver claims one yet, so this counts and
        // acknowledges; when one does, the GSI is recoverable from the vector by subtraction, which
        // is why the map is flat.
        v if super::irq::is_device_vector(v) => {
            DEVICE_IRQS.fetch_add(1, Ordering::Relaxed);
            ROUTED_IRQS.fetch_add(1, Ordering::Relaxed);
            super::irq::end_of_interrupt();
        }
        // A spurious interrupt: the local APIC had to deliver something and had nothing real. It
        // is **not** acknowledged, and that is architecture rather than an oversight: the APIC does
        // not set an in-service bit for a spurious interrupt, so an EOI here would acknowledge
        // whatever genuinely is in service and lose it.
        v if v == super::irq::SPURIOUS_VECTOR as u64 => {
            SPURIOUS_IRQS.fetch_add(1, Ordering::Relaxed);
        }
        // Any other vector at or above 32 is a device interrupt nothing has claimed. Counted and
        // acknowledged rather than fatal: an unowned interrupt that is not acknowledged wedges the
        // local APIC, which turns a stray line into a dead machine.
        v if v >= 32 => {
            SPURIOUS_IRQS.fetch_add(1, Ordering::Relaxed);
            super::irq::end_of_interrupt();
        }
        // Everything below 32 is an architectural exception, and every one of them is fatal here.
        vector => {
            let name = EXCEPTION_NAMES
                .get(vector as usize)
                .copied()
                .unwrap_or("reserved");
            crate::println!();
            crate::println!("=== x86_64 trap: vector {vector} ({name}) ===");
            crate::println!("  error code : {:#018x}", frame.error_code);
            crate::println!(
                "  rip        : {:#018x}   cs : {:#06x}",
                frame.rip,
                frame.cs
            );
            crate::println!(
                "  rsp        : {:#018x}   ss : {:#06x}",
                frame.rsp,
                frame.ss
            );
            crate::println!("  rflags     : {:#018x}", frame.rflags);
            if vector == 14 {
                // Only meaningful for a page fault, so only printed for one: CR2 is stale
                // otherwise, and a stale address printed as if it were current is worse than no
                // address at all.
                crate::println!("  cr2        : {:#018x}", faulting_address());
            }
            panic!("unhandled x86_64 exception: vector {vector} ({name})");
        }
    }
}
