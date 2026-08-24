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
    /// Note what is **not** set: `IOPL` stays 0, so a ring-3 program may not touch the I/O space at
    /// all, and the TSS's I/O permission bitmap is empty (segments.rs). That is the only correct
    /// answer while there is no capability shape for a port (DECISIONS §121).
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

/// **Diagnostic: the ring-3 PC of a thread, from the [`TrapFrame`] at the top of its kernel
/// stack.** The twin of the other two architectures' `user_pc`, and the watchdog dump's
/// user-PC column.
///
/// The frame lives at `stack_top - size_of::<TrapFrame>()` on all three architectures, which is
/// what makes this readable at all; see `user::enter_frame` for why that placement is a rule rather
/// than a convenience. Meaningless for a pure kernel thread, which never builds one.
pub fn user_pc(stack_top: u64) -> u64 {
    let frame = (stack_top - size_of::<TrapFrame>() as u64) as *const TrapFrame;
    // SAFETY: a diagnostic read of the frame the entry path writes at the stack top. Volatile
    // because the owning thread may be running on another CPU while we read, and this must not be
    // hoisted.
    unsafe { core::ptr::read_volatile(&raw const (*frame).rip) }
}

unsafe extern "C" {
    /// Load `frame` as the register state and `iretq` into it: the first entry to ring 3. Defined
    /// in trap.s, sharing the restore path with every trap return. Its first instructions are `cli`
    /// and `mov rsp, rdi`, so it does not touch the caller's stack.
    fn user_return(frame: *mut TrapFrame) -> !;
}

/// Drop to ring 3 by loading `frame` and `iretq`ing into it. The x86 side of the userspace-entry
/// seam (the counterpart of aarch64's `enter_user` and RISC-V's).
///
/// **The privilege change is carried entirely by the frame**, which is the piece of x86 worth
/// stating plainly: `iretq` returns to the ring named by the low two bits of the CS it pops, and
/// pops SS:RSP as well because it is changing ring. There is no "return to user" instruction and no
/// mode bit to set; there is one instruction that pops a context, and the context says which ring.
///
/// **`#[inline(always)]` is load-bearing**, exactly as on the other two architectures: the frame
/// sits at the top of the caller's own kernel stack, so a real call frame pushed here could land on
/// top of it. Inlining makes the caller tail-jump to `user_return` with no push.
///
/// # Safety
/// `frame` must be a correctly-built, writable [`TrapFrame`] at the top of the current thread's
/// kernel stack, with the user address space installed and `TSS.RSP0` naming a kernel stack the
/// next trap can be pushed onto (`segments::set_kernel_stack`).
#[inline(always)]
pub unsafe fn enter_user(frame: *mut TrapFrame) -> ! {
    // **Refuse to enter ring 3 with no entry point**, for the reason the RISC-V twin carries at
    // length: a thread dispatched with `rip == 0` fetches its first instruction from address 0,
    // page-faults, and dies, and whatever it was supposed to serve then never answers. That is a
    // lost-wakeup hang arbitrarily far from the cause; this converts it into a loud failure that
    // carries its own evidence. The comparison is a load and a branch on a frame this function
    // already touches, so nothing is pushed over the frame at the top of this stack.
    //
    // SAFETY: the caller's contract says `frame` is a valid, writable `TrapFrame`.
    let rip = unsafe { (*frame).rip };
    if rip == 0 {
        // SAFETY: as above; read before the cold call below is allowed to use this stack.
        let user_sp = unsafe { (*frame).rsp };
        entered_user_with_no_entry_point(user_sp);
    }

    // SAFETY: the caller's contract; `user_return` never returns.
    unsafe { user_return(frame) }
}

/// The `rip == 0` case, out of line so [`enter_user`] stays a tail jump.
///
/// Its argument is read from the frame **before** it is called, because the frame lives at the top
/// of this very stack and this call is entitled to overwrite it.
#[cold]
#[inline(never)]
fn entered_user_with_no_entry_point(user_sp: u64) -> ! {
    panic!(
        "a thread on cpu {} was dispatched to ring 3 with rip = 0 (user rsp {user_sp:#018x}). \
         Its context was never built, or was built and not seen by this cpu.",
        crate::cpu::id(),
    )
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

// ---------------------------------------------------------------------------------------------
// The `syscall` instruction pair (milestone 161, roadmap item 3).
//
// x86 has TWO ways into the kernel and this is the second one. The IDT above is how a *fault* or an
// *interrupt* arrives; `syscall` is how a program asks on purpose, and it shares almost nothing with
// the IDT path: no gate, no descriptor, no stack switch, no pushes. Four MSRs are the whole of its
// configuration, and none of them has a default worth having.
// ---------------------------------------------------------------------------------------------

/// `IA32_EFER`. Bit 0, `SCE`, is what makes `syscall` a legal instruction rather than `#UD`. The
/// same register carries `LME` and `NXE`, which `boot.s` set before any table was built.
const IA32_EFER: u32 = 0xC000_0080;
/// `IA32_STAR`: the selectors. `[47:32]` is what `syscall` loads into CS (and, plus 8, into SS);
/// `[63:48]` is the base `sysret` derives the user pair from.
const IA32_STAR: u32 = 0xC000_0081;
/// `IA32_LSTAR`: where a 64-bit `syscall` jumps. There is no default and no descriptor: this is a
/// raw address, so a zero here is a jump to zero.
const IA32_LSTAR: u32 = 0xC000_0082;
/// `IA32_FMASK`: the `RFLAGS` bits a `syscall` clears on the way in.
const IA32_FMASK: u32 = 0xC000_0084;

/// `IA32_EFER.SCE`.
const EFER_SCE: u64 = 1 << 0;

/// The selector base `sysret` derives the user pair from, and `syscall`'s kernel pair sits at
/// [`KERNEL_CODE`].
///
/// **The GDT's order is this arithmetic, not a style choice** (segments.rs says so at its top). The
/// three assertions below are the mechanism that keeps the two files agreeing: change a selector in
/// segments.rs without changing this and the build stops, rather than `sysret` landing a user
/// program on the kernel's data segment.
const SYSRET_SELECTOR_BASE: u16 = 0x10;

const _: () = {
    assert!(
        SYSRET_SELECTOR_BASE + 8 == super::segments::USER_DATA & !3,
        "sysret computes the user SS as IA32_STAR[63:48] + 8"
    );
    assert!(
        SYSRET_SELECTOR_BASE + 16 == super::segments::USER_CODE & !3,
        "sysret computes the user CS as IA32_STAR[63:48] + 16"
    );
    assert!(
        KERNEL_CODE + 8 == super::segments::KERNEL_DATA,
        "syscall computes the kernel SS as IA32_STAR[47:32] + 8"
    );
};

/// The `RFLAGS` bits cleared on every `syscall`, named one at a time because each is a hazard
/// rather than tidiness.
///
/// `IF` (9) is the important one: clearing it makes `syscall` arrive with interrupts masked exactly
/// as an interrupt gate does, so the handler cannot be re-entered before it has a stack. `TF` (8)
/// would single-step the kernel on behalf of a user debugger. `DF` (10) must be clear on entry to
/// any System V function, and a user program is free to leave it set. `IOPL` (13:12) is the
/// all-or-nothing I/O gate, and a value inherited from ring 3 is not one the kernel chose. `NT` (14)
/// changes what `iret` means. `RF` (16) suppresses instruction breakpoints. `AC` (18) turns
/// unaligned kernel accesses into faults, which is a user-settable bit that has been a real
/// privilege-escalation lever elsewhere.
const SYSCALL_FLAG_MASK: u64 = (1 << 8)      // TF
    | (1 << 9)                               // IF
    | (1 << 10)                              // DF
    | (0b11 << 12)                           // IOPL
    | (1 << 14)                              // NT
    | (1 << 16)                              // RF
    | (1 << 18); // AC

/// The pseudo-vector a `syscall` frame carries, so a fault report or a dump can say where the frame
/// came from. **256 is out of the IDT's range by construction**, which is what keeps it from
/// colliding with a real vector; the assertion says so rather than leaving it to be noticed.
pub const SYSCALL_VECTOR: u64 = 0x100;
const _: () = assert!(
    SYSCALL_VECTOR > 255,
    "a real IDT vector would be ambiguous here"
);

/// **The kernel stack a `syscall` from ring 3 lands on.** Written by
/// `segments::set_kernel_stack` in the same breath as `TSS.RSP0`, so the two mechanisms cannot name
/// different stacks; read by `x86_syscall_entry` in trap.s, which is why it is `no_mangle`.
///
/// # BUGS
/// **One CPU's, not one per CPU.** A second CPU running a `syscall` would take this one's stack.
/// `TSS.RSP0` has the same problem for the same reason (segments.rs holds a single static TSS), and
/// both are fixed by the same work: a per-CPU GDT and TSS, which SMP bring-up needs anyway
/// (milestone 161, roadmap item 5).
#[unsafe(no_mangle)]
static mut X86_SYSCALL_KERNEL_RSP: u64 = 0;

/// Where `x86_syscall_entry` parks the caller's `rsp` between the `swapgs` and the point where it
/// can be pushed into the frame. `syscall` does not switch stacks, so there is nowhere else to put
/// it: every register still holds a user value that has to be saved. Same single-CPU caveat as
/// [`X86_SYSCALL_KERNEL_RSP`].
#[unsafe(no_mangle)]
static mut X86_SYSCALL_USER_RSP: u64 = 0;

/// Point the `syscall` path's kernel stack at `top`. Called only by `segments::set_kernel_stack`,
/// which owns the other half of the same fact.
pub(super) fn set_syscall_kernel_stack(top: u64) {
    // SAFETY: a single-CPU kernel with no preemption of this write; see the BUGS on the static.
    unsafe { X86_SYSCALL_KERNEL_RSP = top };
}

/// **Program the four MSRs that make `syscall` work**, once per CPU, at boot.
///
/// Order matters in one place: `SCE` is enabled last, so the instruction becomes legal only after
/// the address it jumps to and the flags it clears are already in place. The reverse order leaves a
/// window in which a `syscall` would jump to whatever `IA32_LSTAR` happened to hold, which on a cold
/// machine is zero.
///
/// # Safety
/// Must be called with the GDT installed (the selectors this writes index it) and before anything
/// enters ring 3.
pub unsafe fn init_syscall() {
    let star = ((SYSRET_SELECTOR_BASE as u64) << 48) | ((KERNEL_CODE as u64) << 32);
    // SAFETY: these are the architectural `syscall` MSRs, present on every long-mode CPU (they are
    // part of the mode itself, not an optional feature). The values are checked against the GDT by
    // the const assertions above.
    unsafe {
        super::write_msr(IA32_STAR, star);
        super::write_msr(IA32_LSTAR, x86_syscall_entry as *const () as u64);
        super::write_msr(IA32_FMASK, SYSCALL_FLAG_MASK);
        let efer = super::read_msr(IA32_EFER);
        super::write_msr(IA32_EFER, efer | EFER_SCE);
    }
}

unsafe extern "C" {
    /// The `IA32_LSTAR` target: where a ring-3 `syscall` lands. Defined in trap.s.
    fn x86_syscall_entry();

    /// Resume whoever called `x86_enter_user_and_wait`, handing back `value`. Defined in trap.s;
    /// see [`ring3_self_test`] for the one caller of the pair.
    fn x86_leave_user(resume_rsp: u64, value: u64) -> !;

    /// Enter ring 3 with `frame`, leaving a way back in `*resume_slot`. Defined in trap.s.
    fn x86_enter_user_and_wait(frame: *mut TrapFrame, resume_slot: *mut u64) -> u64;
}

/// **The one Rust function every `syscall` reaches**, called by `x86_syscall_entry` with a pointer
/// to the [`TrapFrame`] it built. The `syscall` twin of [`x86_trap_handler`].
///
/// It hands straight to the portable dispatcher, which reads the number and the arguments through
/// `TrapFrame::{syscall_nr, arg, set_arg}` and so never names an x86 register. That is the whole
/// point of those accessors, and this function is where the third architecture cashes them in.
///
/// # BUGS
/// **The return is an `iretq`, not a `sysretq`.** `sysret` is the faster half of the instruction
/// pair and this port does not use it yet, deliberately: it returns to whatever `rcx` holds without
/// checking that the address is canonical, and a non-canonical `rcx` faults **in ring 0 on the
/// user's stack**, which is the shape of CVE-2012-0217 and needs an explicit canonicality check and
/// an `iretq` fallback to use safely. Sharing `isr_restore` costs some tens of cycles per syscall
/// and buys one return path with one `swapgs` rule. Worth revisiting with a benchmark rather than an
/// argument, once there is a syscall-heavy workload on this architecture to measure.
///
/// # Safety
/// Called only from `x86_syscall_entry`, which has just constructed a complete [`TrapFrame`] at the
/// address it passes. Nothing else may call it.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn x86_syscall_handler(frame: *mut TrapFrame) {
    // SAFETY: `x86_syscall_entry` built the frame directly below the pointer it passed, and no
    // other caller exists (the symbol is only referenced from trap.s).
    let frame = unsafe { &mut *frame };
    SVC_COUNT.fetch_add(1, Ordering::Relaxed);
    ring3_probe_syscall(frame); // diverges if this is the probe's last call
    crate::syscall::dispatch(frame);
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
        // An architectural exception taken **in ring 3**: the user program's fault, not the
        // kernel's. Recorded rather than fatal, which is the difference the whole privilege
        // boundary exists to make. Only vectors below 32 reach here; a device interrupt that
        // happened to land while a user program was running was already handled above, because an
        // interrupt is not the interrupted program's fault.
        vector if frame.cs & 3 == 3 => {
            USER_FAULTS.fetch_add(1, Ordering::Relaxed);
            let (fault, address) = classify_user_fault(vector, frame.error_code);
            LAST_USER_FAULT.store(fault.encode(), Ordering::Relaxed);
            LAST_USER_FAULT_ADDRESS.store(address, Ordering::Relaxed);

            // The boot tour's ring-3 probe, if one is running: this is how it ends, and the
            // function does not return in that case.
            ring3_probe_faulted();

            // And otherwise there is nothing to do with the thread, because there is no scheduler
            // on this architecture yet to kill it with (milestone 161, roadmap item 4). Fall
            // through to the report below rather than returning to a program whose fault was not
            // resolved, which would fault again forever.
            crate::println!();
            crate::println!("=== x86_64: a ring-3 program faulted and there is no scheduler ===");
            crate::println!("  fault      : {fault:?} at {address:#018x}");
            crate::println!(
                "  rip        : {:#018x}   cs : {:#06x}",
                frame.rip,
                frame.cs
            );
            panic!("user fault with no thread to kill: {fault:?} at {address:#018x}");
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

/// **Turn an x86 exception into the portable [`UserFault`] both other architectures already
/// state.**
///
/// The page fault's error code is the interesting half, and x86 is the *most* forthcoming of the
/// three about it: aarch64 is told the fault class in `ESR_EL1`, RISC-V has to derive it by walking
/// the tables (see its `user_fault`), and here the CPU pushes a word saying so outright.
///
/// | Bit | Set means |
/// |---|---|
/// | 0 (`P`) | the page **was** present, so this is a permission refusal rather than a missing map |
/// | 1 (`W/R`) | the access was a write |
/// | 2 (`U/S`) | the access came from ring 3 |
/// | 3 (`RSVD`) | a reserved bit was set in some entry of the walk |
/// | 4 (`I/D`) | the access was an instruction fetch |
///
/// The `P` bit is what separates [`UserFault::Permission`] from [`UserFault::Translation`], and that
/// distinction is the one worth having: a permission fault means the hardware **found** the page,
/// read its bits, and said no.
///
/// Anything that is not a page fault is [`UserFault::Other`] at the faulting instruction, matching
/// what the other two report for an illegal instruction or a misaligned access.
fn classify_user_fault(vector: u64, error_code: u64) -> (UserFault, u64) {
    use crate::arch::UserFaultAccess;

    if vector != 14 {
        return (UserFault::Other, 0);
    }
    let access = if error_code & (1 << 4) != 0 {
        UserFaultAccess::Fetch
    } else if error_code & (1 << 1) != 0 {
        UserFaultAccess::Write
    } else {
        UserFaultAccess::Read
    };
    let fault = if error_code & 1 != 0 {
        UserFault::Permission(access)
    } else {
        UserFault::Translation(access)
    };
    (fault, faulting_address())
}

// ---------------------------------------------------------------------------------------------
// The ring-3 self test (milestone 161, roadmap item 3).
//
// **This is bring-up scaffolding and it is written to be deleted.** The other two architectures
// prove ring 3 by running a compiled ELF out of the initrd under the scheduler; neither exists here
// yet, so the boot tour proves the privilege boundary with a hand-assembled probe
// (ring3_probe.s) entered directly from the boot thread. What it establishes is exactly the arch
// layer: an `iretq` into CPL 3, the `syscall` ABI reaching the portable dispatcher, the return to
// ring 3, and the page tables refusing a supervisor page. What it does NOT establish is the loader,
// argument passing, capability grants, or anything about a process, all of which arrive with the
// scheduler (roadmap item 4).
//
// It is the sibling of `self_test` above, which proves the trap path round-trips by taking an
// `int3`, and it earns its place for the same reason: there is no other cheap moment to find out
// that the privilege boundary does not work.
// ---------------------------------------------------------------------------------------------

/// The syscall number the probe uses to ask the **portable** dispatcher something it will refuse.
/// Deliberately not a real one: `Error::BadSyscall` is the only answer `crate::syscall::dispatch`
/// can give on a kernel with no scheduler, so it is the only round trip through the real dispatcher
/// available here. Substituted into `ring3_probe.s` by `global_asm!`, so the number lives in one
/// place.
pub const PROBE_ABI_NR: u64 = 0x1610_0001;
/// The probe reporting its own CS, SS, and the dispatcher's answer. Intercepted, then returned from.
pub const PROBE_REPORT_NR: u64 = 0x1610_0002;
/// The probe saying it **read kernel memory from ring 3**, which is the failure this test exists to
/// catch. Reaching it at all means the confinement did not hold.
pub const PROBE_ESCAPED_NR: u64 = 0x1610_0003;

/// The value `x86_enter_user_and_wait` returns when the probe faulted, which is the expected end.
const LEAVE_FAULTED: u64 = 1;
/// The value it returns when the probe read kernel memory and lived: a failure, loudly.
const LEAVE_ESCAPED: u64 = 2;

/// Where to resume when the probe is finished; zero when no probe is running, which is also what
/// makes the two interception points below no-ops in an ordinary kernel.
static mut RING3_RESUME: u64 = 0;

/// What the probe reported: its CS, its SS, and the dispatcher's answer to [`PROBE_ABI_NR`].
static PROBE_CS: AtomicU64 = AtomicU64::new(0);
static PROBE_SS: AtomicU64 = AtomicU64::new(0);
static PROBE_ANSWER: AtomicU64 = AtomicU64::new(0);

/// Is a ring-3 probe waiting to be resumed?
fn ring3_resume_slot() -> u64 {
    // SAFETY: single-CPU boot code; the probe is entered and left on this one thread.
    unsafe { RING3_RESUME }
}

/// The probe's syscalls. Returns normally for anything else, including the probe's own
/// [`PROBE_ABI_NR`], which is meant to reach the portable dispatcher.
fn ring3_probe_syscall(frame: &mut TrapFrame) {
    let resume = ring3_resume_slot();
    if resume == 0 {
        return;
    }
    match frame.syscall_nr() {
        PROBE_REPORT_NR => {
            PROBE_CS.store(frame.arg(0), Ordering::Relaxed);
            PROBE_SS.store(frame.arg(1), Ordering::Relaxed);
            PROBE_ANSWER.store(frame.arg(2), Ordering::Relaxed);
            // Return to ring 3, which is the point: the probe's next instruction is the one that
            // tests the page tables, and it only runs if this return path works.
        }
        PROBE_ESCAPED_NR => {
            // SAFETY: `resume` is the block `x86_enter_user_and_wait` parked on the boot stack,
            // which is still live: its frame is above ours and nothing has returned through it.
            unsafe { x86_leave_user(resume, LEAVE_ESCAPED) }
        }
        _ => {}
    }
}

/// The probe's fault, which is how it is supposed to end. Returns normally if no probe is running.
fn ring3_probe_faulted() {
    let resume = ring3_resume_slot();
    if resume == 0 {
        return;
    }
    // SAFETY: as in `ring3_probe_syscall`; the parked block is still live.
    unsafe { x86_leave_user(resume, LEAVE_FAULTED) }
}

/// What a ring-3 round trip found. Every field is something the **hardware** or the **portable
/// dispatcher** said, rather than something this kernel assumed.
#[derive(Debug, Clone, Copy)]
pub struct Ring3Report {
    /// The probe's own CS, read out of the segment register in ring 3. Its low two bits are the
    /// CPL, so `0x23` is the whole proof that this ran at ring 3 and not ring 0.
    pub cs: u64,
    /// The probe's own SS, likewise.
    pub ss: u64,
    /// What `crate::syscall::dispatch` answered [`PROBE_ABI_NR`] with, as the user program saw it.
    pub dispatcher_answer: i64,
    /// How many syscalls the probe made.
    pub syscalls: usize,
    /// The fault that ended it, and where. `None` means it did not fault, which is a failure.
    pub fault: Option<(UserFault, u64)>,
    /// The kernel address the probe was told to try to read.
    pub forbidden_address: u64,
    /// **True is a failure**: the probe read kernel memory from ring 3 and came back to say so.
    pub escaped: bool,
}

unsafe extern "C" {
    /// The hand-assembled probe, in `.rodata`. See `ring3_probe.s`.
    static x86_ring3_probe_start: u8;
    static x86_ring3_probe_end: u8;
}

/// Where the probe's code and stack are mapped in its own address space. Any two low-half pages
/// would do; 4 MiB is where the other two architectures' first hand-assembled programs went, so a
/// reader who has seen one of those recognises it.
const PROBE_CODE_VA: u64 = 0x40_0000;
const PROBE_STACK_VA: u64 = 0x41_0000;

/// How far below this function's own live frames the probe's trap stack starts, so the two cannot
/// overlap. A syscall or fault from ring 3 builds a 176-byte frame at the top of it and the handler
/// runs beneath; the boot stack is 64 KiB and the tour is a handful of frames deep, so 8 KiB of
/// separation is generous on both sides rather than tuned. It is not a measurement.
const PROBE_KERNEL_STACK_GAP: u64 = 8 * 1024;

/// **Prove ring 3 round-trips**, by running the hand-assembled probe at CPL 3 and reporting what it
/// found. The privilege-boundary counterpart of [`self_test`].
///
/// Frames are drawn from the allocator and **not returned**, which is the boot tour's existing
/// habit (the RISC-V tour's user-address-space step does the same) and is five frames on a machine
/// with sixty thousand. It is called once.
///
/// # Safety
/// Must be called after `mmu::init`, with the GDT and IDT installed, from the boot thread, with no
/// other thread running. It installs a page-table root of its own and points `TSS.RSP0` into the
/// current stack.
pub unsafe fn ring3_self_test() -> Result<Ring3Report, &'static str> {
    use paging::{Flags, PAGE_SIZE};

    use super::mmu;
    use crate::memory;

    let kernel_root = mmu::current_root();

    // A process root of its own, carrying the kernel's high half. Without the share, the `mov cr3`
    // below would unmap the instruction after it.
    let root = memory::alloc()
        .ok_or("no frame for the probe's page-table root")?
        .addr();
    // SAFETY: a fresh frame from the allocator, reachable through the direct map. Zeroed before the
    // hardware could ever walk it.
    unsafe { (*mmu::phys_to_ptr(root)).entries = [0; paging::ENTRIES] };
    mmu::share_kernel_half(root);

    // Its two pages. The code frame is written through the direct map, which is the same "two names
    // for one frame" the address-space builder uses: the kernel cannot address `PROBE_CODE_VA`,
    // because that is a low address and means something else entirely from ring 0.
    let code = memory::alloc()
        .ok_or("no frame for the probe's code")?
        .addr();
    let stack = memory::alloc()
        .ok_or("no frame for the probe's stack")?
        .addr();

    let from = &raw const x86_ring3_probe_start;
    let len = (&raw const x86_ring3_probe_end as usize) - (from as usize);
    if len == 0 || len as u64 > PAGE_SIZE {
        return Err("the ring-3 probe does not fit in one page");
    }
    // SAFETY: `from` is the probe in this image's `.rodata` and `len` is the distance to the label
    // after it; the destination is a whole frame the allocator just handed us, addressable through
    // the direct map. The two cannot overlap: one is the image, the other is a free frame, and the
    // image is on the allocator's forbidden list.
    unsafe {
        core::ptr::copy_nonoverlapping(from, mmu::phys_to_virt(code) as *mut u8, len);
    }

    // Install it, then map into it. The mapping helpers work on the *current* space, which is what
    // makes the order matter; `share_kernel_half` above is what makes the switch survivable.
    //
    // SAFETY: `root` is zeroed, page-aligned, and carries the kernel's high half, so this code, its
    // stack and the direct map all still resolve on the next instruction.
    unsafe { mmu::switch_user_root(mmu::ttbr0_value(root, 0)) };

    let result = (|| -> Result<Ring3Report, &'static str> {
        let alloc = || memory::alloc().map(|f| f.addr());
        mmu::map_current_user_frame(PROBE_CODE_VA, code, Flags::user_code(), alloc)
            .map_err(|_| "could not map the probe's code")?;
        mmu::map_current_user_frame(PROBE_STACK_VA, stack, Flags::user_data(), alloc)
            .map_err(|_| "could not map the probe's stack")?;

        // The kernel stack a ring-3 trap lands on, below this function's own live frames so the two
        // cannot overlap. `set_kernel_stack` writes `TSS.RSP0` and the `syscall` path's own stash
        // together, which is why there is one call here and not two.
        let kernel_sp = (super::current_sp() - PROBE_KERNEL_STACK_GAP) & !0xf;
        super::segments::set_kernel_stack(kernel_sp);

        // The entry frame goes at the top of that region, where `enter_user` expects it.
        let slot = kernel_sp - size_of::<TrapFrame>() as u64;
        let forbidden = mmu::text_start();
        // SAFETY: `slot` is 16-byte-aligned writable kernel stack well below our own frames, and
        // `TrapFrame` is a multiple of 16.
        unsafe {
            (slot as *mut TrapFrame).write(TrapFrame::for_user_entry(
                PROBE_CODE_VA,
                PROBE_STACK_VA + PAGE_SIZE,
                [forbidden, 0, 0],
            ));
        }

        let before = SVC_COUNT.load(Ordering::Relaxed);
        LAST_USER_FAULT.store(0, Ordering::Relaxed);

        // And go. Everything from here until this call returns runs in ring 3 or in a trap handler
        // serving it.
        //
        // SAFETY: the frame is complete and writable, the probe's address space is installed, and
        // `TSS.RSP0` names the stack the trap will be pushed onto. The resume slot is a static this
        // function is the only writer of, and the block it will name is parked on this stack, which
        // stays live for exactly as long as this call does.
        let outcome =
            unsafe { x86_enter_user_and_wait(slot as *mut TrapFrame, &raw mut RING3_RESUME) };

        Ok(Ring3Report {
            cs: PROBE_CS.load(Ordering::Relaxed),
            ss: PROBE_SS.load(Ordering::Relaxed),
            dispatcher_answer: PROBE_ANSWER.load(Ordering::Relaxed) as i64,
            syscalls: SVC_COUNT.load(Ordering::Relaxed) - before,
            fault: last_user_fault(),
            forbidden_address: forbidden,
            escaped: outcome == LEAVE_ESCAPED,
        })
    })();

    // Whatever happened, stop running on the probe's tables and stop pointing the trap path at a
    // stack that is about to be reused.
    //
    // SAFETY: the resume slot is cleared first, so the interception points above are inert again
    // before anything else can trap.
    unsafe { RING3_RESUME = 0 };
    mmu::deactivate_user();
    super::segments::set_kernel_stack(0);
    debug_assert_eq!(mmu::current_root(), kernel_root);

    let report = result?;
    if report.escaped {
        return Err("the probe read kernel memory from ring 3: confinement did not hold");
    }
    // The dispatcher's answer is checked here rather than printed and left to a reader, because the
    // interesting failure is not "no answer" but "an answer that came from somewhere else": a
    // return register the restore path never wrote would most likely still hold the zero
    // `for_user_entry` put there.
    if report.dispatcher_answer != abi::Error::BadSyscall as i64 {
        return Err("the portable dispatcher's answer did not reach the program in rdi");
    }
    Ok(report)
}
