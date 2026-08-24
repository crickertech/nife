//! **The GDT and the TSS**, which have no counterpart at all on the other two architectures.
//!
//! aarch64 and RISC-V have no segmentation. `x86_64` has *almost* none: in 64-bit mode a segment's
//! base and limit are ignored for everything except `fs` and `gs`, so the descriptors below carry
//! essentially no information. What they still carry is the three bits that decide **what mode the
//! CPU is in**: the code segment's `L` bit (64-bit), `DPL` (which ring), and `D/B`. So the GDT
//! cannot be dispensed with even though almost every field in it is dead.
//!
//! The TSS is the part that does real work, and its job is the one x86 gives no other mechanism for:
//! **where does the stack pointer come from when the CPU changes ring?** On aarch64 an exception
//! from EL0 lands on `SP_EL1`, a separate banked register the kernel has already set. On RISC-V the
//! trap handler recovers its stack from `sscratch`. x86 reads `TSS.RSP0` out of memory, so a task
//! that traps in from ring 3 gets whatever `RSP0` says, and if it says something wrong the very
//! first user-mode fault is unrecoverable. The IST entries are the same mechanism with a stronger
//! promise: an IST vector switches stacks **unconditionally**, even for a ring-0 trap, which is the
//! only way to survive a fault whose cause is that the current stack is unusable.
//!
//! # The selector layout is not free
//!
//! `syscall`/`sysret` derive four selectors from two 16-bit fields of `IA32_STAR`, by arithmetic:
//! entering the kernel takes CS = `STAR[47:32]` and SS = that + 8, and returning to user takes
//! CS = `STAR[63:48]` + 16 and SS = `STAR[63:48]` + 8. So the order below (kernel code, kernel data,
//! user **data**, user **code**) is forced by the instruction, and the apparently-backwards user
//! pair is the tell. Reordering these to look tidier breaks `sysret` in a way that shows up as a
//! general protection fault on the way back to a program that did nothing wrong.

use core::arch::asm;

/// Selector for the kernel code segment. Also `IA32_STAR[47:32]`, from which `syscall` derives the
/// kernel SS as this + 8.
pub const KERNEL_CODE: u16 = 0x08;
/// Selector for the kernel data segment (ss/ds/es while in ring 0).
pub const KERNEL_DATA: u16 = 0x10;
/// Selector for the user data segment. `sysret` computes this as `IA32_STAR[63:48] + 8`.
#[allow(dead_code)] // Referenced once user mode exists; see the module header for why it is here now.
pub const USER_DATA: u16 = 0x18 | 3;
/// Selector for the user code segment. `sysret` computes this as `IA32_STAR[63:48] + 16`.
#[allow(dead_code)]
pub const USER_CODE: u16 = 0x20 | 3;
/// Selector for the TSS descriptor, which occupies **two** GDT slots (0x28 and 0x30) because a
/// 64-bit system descriptor is 16 bytes rather than 8.
const TSS_SELECTOR: u16 = 0x28;

/// The IST slot the double-fault handler runs on. **One-based**: the IDT encodes 0 as "do not
/// switch stacks", so slot 1 is `TSS.ist[0]`.
pub const IST_DOUBLE_FAULT: u8 = 1;

/// A 64-bit Task State Segment.
///
/// Almost every field of the 32-bit TSS is gone in long mode; what remains is three ring stacks,
/// seven interrupt stacks, and the I/O permission bitmap offset. `#[repr(C, packed)]` because the
/// layout is the CPU's, not Rust's, and the reserved words are load-bearing padding rather than
/// slack.
#[repr(C, packed)]
struct Tss {
    _reserved0: u32,
    /// The stack pointer the CPU loads on a trap that raises privilege to ring 0. This is the field
    /// that makes user mode survivable.
    rsp0: u64,
    rsp1: u64,
    rsp2: u64,
    _reserved1: u64,
    /// Seven unconditional stacks, selectable per IDT entry. `ist[0]` is what
    /// [`IST_DOUBLE_FAULT`] names.
    ist: [u64; 7],
    _reserved2: u64,
    _reserved3: u16,
    /// Offset from the base of this TSS to the I/O permission bitmap. Set past the end of the
    /// structure, which means "no ports are permitted to ring 3", the only correct answer while no
    /// program has been granted one. See `port.rs` for why this is where such a grant would go.
    iomap_base: u16,
}

impl Tss {
    const fn new() -> Self {
        Self {
            _reserved0: 0,
            rsp0: 0,
            rsp1: 0,
            rsp2: 0,
            _reserved1: 0,
            ist: [0; 7],
            _reserved2: 0,
            _reserved3: 0,
            // sizeof(Tss) == 104. Anything >= the limit in the descriptor means "empty bitmap".
            iomap_base: 104,
        }
    }
}

const _: () = assert!(size_of::<Tss>() == 104);

/// The boot CPU's TSS. One for now, because SMP bring-up (INIT-SIPI-SIPI) is not built; every core
/// needs its own, since `rsp0` names a per-core stack.
static mut TSS: Tss = Tss::new();

/// The GDT: seven 8-byte slots, the last two of which are one 16-byte TSS descriptor.
static mut GDT: [u64; 7] = [
    0,                     // 0x00: the mandatory null descriptor
    0x00AF_9A00_0000_FFFF, // 0x08: kernel code, DPL 0, L=1
    0x00CF_9200_0000_FFFF, // 0x10: kernel data, DPL 0
    0x00CF_F200_0000_FFFF, // 0x18: user data, DPL 3
    0x00AF_FA00_0000_FFFF, // 0x20: user code, DPL 3, L=1
    0,                     // 0x28: TSS descriptor, low half, filled in by `init`
    0,                     // 0x30: TSS descriptor, high half
];

/// The operand `lgdt` and `lidt` take: a 16-bit limit (size minus one) and a 64-bit base.
/// `#[repr(C, packed)]` because the CPU reads exactly ten bytes and a Rust-chosen alignment hole
/// between the two fields would make it read the base from the wrong offset.
#[repr(C, packed)]
pub struct DescriptorTablePointer {
    pub limit: u16,
    pub base: u64,
}

/// Install the GDT and the TSS on this CPU, and reload every segment register from it.
///
/// # Why the code-segment reload is a return and not a jump
///
/// `lgdt` changes the table but not the selectors already loaded, and `mov cs, ax` does not exist:
/// CS can only be changed by a control transfer. The idiomatic 64-bit way is a far return: push the
/// new selector and a target address, then `retfq`, which pops both. A far *jump* would work too but
/// has to go through memory (`ljmp [mem]`) in 64-bit mode, which needs a scratch location; the
/// stack is already there.
///
/// # Safety
/// Must be called once per CPU, with a valid stack, before anything depends on the GDT: it replaces
/// the boot GDT that `boot.s` installed, and the old one goes away.
pub unsafe fn init() {
    // The TSS descriptor cannot be a constant: it holds the TSS's own address, which is only known
    // once the image is linked and loaded. A 64-bit system descriptor spreads that address across
    // three disjoint fields, which is a layout inherited from the 16-bit 286 and is why this looks
    // the way it does rather than being a single store.
    let base = (&raw const TSS) as u64;
    let limit = (size_of::<Tss>() - 1) as u64;
    let low = (limit & 0xffff)
        | ((base & 0x00ff_ffff) << 16)
        | (0x89 << 40)                    // present, type 9 = available 64-bit TSS
        | (((limit >> 16) & 0xf) << 48)
        | (((base >> 24) & 0xff) << 56);
    let high = base >> 32;

    // SAFETY: single-threaded boot code, before any other CPU exists and before anything else reads
    // the GDT. The two slots written are the ones `TSS_SELECTOR` names.
    unsafe {
        GDT[5] = low;
        GDT[6] = high;
    }

    let gdtr = DescriptorTablePointer {
        limit: (size_of::<[u64; 7]>() - 1) as u16,
        base: (&raw const GDT) as u64,
    };

    // **Loading a segment register in long mode DESTROYS that segment's base MSR**, and `gs`'s base
    // is where this kernel keeps its per-CPU pointer (the analog of aarch64's `TPIDR_EL1` and
    // RISC-V's `tp`). `cpu::init_this_cpu` has already run by the time anything calls this, because
    // the console lock reads the per-CPU block, so the `mov gs, ax` below would silently zero the
    // pointer and the very next `println!` would dereference null.
    //
    // That is exactly what happened during bring-up, and it is worth recording how it presented:
    // not as a null dereference but as an instruction fetch from the middle of a static, several
    // frames away, with the register dump showing a perfectly correct GDT, TSS and IDT. Nothing
    // about the symptom pointed here.
    //
    // So the base is saved and put back around the reload. Doing it here rather than making the
    // caller re-arm the pointer keeps the ordering constraint from existing at all, which is the
    // ladder's first rung: the wrong state is not representable rather than merely documented.
    // `fs` gets no such treatment because nothing uses its base; if anything ever does, it needs the
    // same two lines.
    let gs_base = super::percpu();

    // SAFETY: `gdtr` describes the table above, which is well-formed by construction. The far
    // return reloads CS from a descriptor whose L bit is set, so the CPU stays in 64-bit mode; the
    // data selectors are reloaded before anything can use a stale one.
    unsafe {
        asm!(
            "lgdt [{gdtr}]",
            // Push the new CS and the address to continue at, then far-return into it.
            "push {code:r}",
            "lea {tmp}, [rip + 2f]",
            "push {tmp}",
            "retfq",
            "2:",
            "mov ds, {data:e}",
            "mov es, {data:e}",
            "mov ss, {data:e}",
            "mov fs, {data:e}",
            "mov gs, {data:e}",
            "ltr {tss:x}",
            gdtr = in(reg) &gdtr,
            code = in(reg) KERNEL_CODE as u64,
            data = in(reg) KERNEL_DATA as u32,
            tss = in(reg) TSS_SELECTOR,
            tmp = lateout(reg) _,
            options(preserves_flags),
        );
    }

    // Put the per-CPU pointer back; see the comment above the `asm!`.
    super::set_percpu(gs_base);
}

/// Point this CPU's ring-0 trap stack at `top`. Called whenever the thread that would take a trap
/// from user mode changes, which on the other two architectures is `sscratch.kernel_sp` (RISC-V) or
/// nothing at all (aarch64 banks `SP_EL1`).
#[allow(dead_code)] // Its caller is the user-mode entry path, which milestone 161 has not reached.
pub fn set_kernel_stack(top: u64) {
    // SAFETY: writes this CPU's own TSS field. The CPU reads it only on a privilege transition,
    // which cannot happen while this runs with interrupts masked by the caller. `write_unaligned`
    // because the TSS is `packed`: rsp0 sits at byte 4 and a plain store would assume alignment the
    // layout does not promise.
    unsafe { (&raw mut TSS.rsp0).write_unaligned(top) };
}

/// Point an IST slot (1-based, as the IDT encodes it) at `top`.
///
/// # Safety
/// `top` must be the top of a stack no other IST vector shares. Two vectors on one stack means a
/// fault taken while handling the other overwrites its frame, which is precisely the situation IST
/// exists to survive.
pub unsafe fn set_interrupt_stack(slot: u8, top: u64) {
    assert!((1..=7).contains(&slot), "IST slots are 1..=7, not {slot}");
    // SAFETY: writes this CPU's own TSS `ist[slot - 1]`, in bounds by the assertion above. Through
    // a raw pointer because the TSS is `packed` and a reference to a field would be misaligned.
    unsafe {
        (&raw mut TSS.ist)
            .cast::<u64>()
            .add(slot as usize - 1)
            .write_unaligned(top);
    }
}
