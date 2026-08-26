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

/// **One TSS per core** (milestone 161's SMP item, fixing exactly the limitation this static's own
/// doc used to name: "every core needs its own, since `rsp0` names a per-core stack"). Indexed by
/// `cpu::id()`, the same way every other per-core array in this kernel is; `TSS.rsp0` is what makes
/// this matter, since sharing one across cores would let two cores race on the same ring-0 stack
/// pointer the instant both took a trap from ring 3.
static mut TSS: [Tss; crate::cpu::MAX_CPUS] = [const { Tss::new() }; crate::cpu::MAX_CPUS];

/// The GDT: seven 8-byte slots, the last two of which are one 16-byte TSS descriptor. **One per
/// core**, for the same reason as [`TSS`]: the TSS descriptor's base address is this core's own
/// `TSS[cpu::id()]`, so sharing one GDT would mean every core's `ltr` loaded the SAME task
/// register, aliasing every core's ring-0 stack onto whichever one initialized last.
static mut GDT: [[u64; 7]; crate::cpu::MAX_CPUS] = [BASE_GDT; crate::cpu::MAX_CPUS];

/// Every core's GDT starts identical: only the TSS descriptor (slots 5 and 6) differs per core, and
/// [`init`] fills those in from this core's own [`TSS`] entry.
const BASE_GDT: [u64; 7] = [
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
/// Must be called once per CPU, with a valid stack and `cpu::init_this_cpu` already run on it
/// (this fills `TSS`/`GDT` at `cpu::id()`'s own slot, and restores this core's per-CPU pointer
/// afterward; see below), before anything depends on the GDT: it replaces the boot GDT that
/// `boot.s` installed, and the old one goes away.
pub unsafe fn init() {
    let id = crate::cpu::id();

    // The TSS descriptor cannot be a constant: it holds the TSS's own address, which is only known
    // once the image is linked and loaded. A 64-bit system descriptor spreads that address across
    // three disjoint fields, which is a layout inherited from the 16-bit 286 and is why this looks
    // the way it does rather than being a single store.
    // SAFETY: indexing this core's own slot with `id` in bounds (`cpu::id()` cannot exceed
    // `MAX_CPUS`, which is exactly how big `TSS` is); reading only the address, not the (possibly
    // concurrently-written-by-another-core-at-a-different-index) contents.
    let base = unsafe { (&raw const TSS[id]) as u64 };
    let limit = (size_of::<Tss>() - 1) as u64;

    // Hand trap.s the address of the one field it writes: this core's OWN `TSS[id].rsp0`, through
    // the per-CPU block `gs` already names, so there is nowhere a second core's write could land.
    // Before the IDT exists, so no trap can have taken the path that reads it. See
    // `cpu::PerCpu::x86_trap` / `cpu::X86TrapPerCpu::tss_rsp0_ptr`.
    // SAFETY: writes this core's own `PerCpu` slot, reached the same way every per-CPU write on
    // this architecture is, before any trap can be taken on this core.
    unsafe {
        *crate::cpu::current().x86_trap.tss_rsp0_ptr.get() = (&raw const TSS[id].rsp0) as u64;
    }
    let low = (limit & 0xffff)
        | ((base & 0x00ff_ffff) << 16)
        | (0x89 << 40)                    // present, type 9 = available 64-bit TSS
        | (((limit >> 16) & 0xf) << 48)
        | (((base >> 24) & 0xff) << 56);
    let high = base >> 32;

    // SAFETY: this core writes only its OWN slot (`id`), before anything else on this core reads
    // its GDT; a different core's `init` writes a different slot.
    unsafe {
        GDT[id][5] = low;
        GDT[id][6] = high;
    }

    // SAFETY: as `base` above, this core's own slot only.
    let gdt_base = unsafe { (&raw const GDT[id]) as u64 };
    let gdtr = DescriptorTablePointer {
        limit: (size_of::<[u64; 7]>() - 1) as u16,
        base: gdt_base,
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
///
/// **It sets two things, and that is the point.** x86 has two doors into the kernel from ring 3 and
/// they find their stack differently: a trap reads `TSS.RSP0`, and `syscall` reads nothing at all,
/// so the syscall entry path has to be told separately (`exceptions::set_syscall_kernel_stack`).
/// Two writes behind one function is what stops the two ever naming different stacks; the wrong
/// state is not representable rather than merely documented.
pub fn set_kernel_stack(top: u64) {
    let id = crate::cpu::id();
    // SAFETY: writes this CPU's own TSS field, indexed by its own `cpu::id()`. The CPU reads it
    // only on a privilege transition, which cannot happen while this runs with interrupts masked by
    // the caller. `write_unaligned` because the TSS is `packed`: rsp0 sits at byte 4 and a plain
    // store would assume alignment the layout does not promise.
    unsafe { (&raw mut TSS[id].rsp0).write_unaligned(top) };
    super::exceptions::set_syscall_kernel_stack(top);
}

/// Point an IST slot (1-based, as the IDT encodes it) at `top`, on this core.
///
/// # Safety
/// `top` must be the top of a stack no other IST vector shares. Two vectors on one stack means a
/// fault taken while handling the other overwrites its frame, which is precisely the situation IST
/// exists to survive.
pub unsafe fn set_interrupt_stack(slot: u8, top: u64) {
    assert!((1..=7).contains(&slot), "IST slots are 1..=7, not {slot}");
    let id = crate::cpu::id();
    // SAFETY: writes this CPU's own TSS `ist[slot - 1]`, in bounds by the assertion above. Through
    // a raw pointer because the TSS is `packed` and a reference to a field would be misaligned.
    unsafe {
        (&raw mut TSS[id].ist)
            .cast::<u64>()
            .add(slot as usize - 1)
            .write_unaligned(top);
    }
}

/// **Bench-only** (DECISIONS §121's amendment, 2026-08-24): the I/O permission bitmap option 1
/// would write into the current CPU's TSS on every switch-in, sized for the whole port space,
/// **not installed as live**.
///
/// The x86 port space is 16 bits (`in`/`out` address exactly 64 Ki ports), one bit of "may this
/// ring-3 thread touch this port" each, so the bitmap is `65536 / 8 == 8192` bytes exactly (the
/// module doc's "8 KiB" is not a round number picked for convenience; it is what the architecture
/// requires). Some real implementations (Linux's, e.g.) append one further guard byte set to all
/// ones, because the CPU may read two bytes when checking the highest port and a bitmap that ends
/// exactly at the limit would read past it; that byte is not needed here because nothing ever
/// checks this array against a real port access (see below).
///
/// **What this measures, and what it does not.** §121's option 1 would extend the live [`Tss`]
/// with this array, point `iomap_base` at it, and have the scheduler overwrite it on every
/// switch-in. This does the write (the dominant cost the amendment names) without any of the
/// rest: `iomap_base` above still points past the end of `TSS`, `ltr` is never re-issued, and no
/// `in`/`out` from ring 3 ever runs in this benchmark boot (there is no ring-3 program on this
/// port yet; see `user::x86_programs`). So the number this produces is the cost of an 8 KiB
/// per-CPU memory write on the switch path, not a proof that the bitmap enforces anything; that
/// second half is option 1's real implementation, out of scope here (`design/decisions/121-port-io-capability.md`).
#[cfg(feature = "bench")]
const IOMAP_BYTES: usize = 65536 / 8;

#[cfg(feature = "bench")]
#[repr(C, align(8))]
struct BenchIoBitmap([u8; IOMAP_BYTES]);

/// A second CPU-owned 8 KiB region, separate from the live `TSS` static above so this benchmark
/// can never be mistaken for having wired the real one in. One instance because this port has one
/// CPU (`smp::bring_up_secondaries` is a refusal on `x86_64` today); a real per-CPU version would
/// need `crate::cpu::PerCpu`, which option 1 would also need and this benchmark does not.
#[cfg(feature = "bench")]
static mut BENCH_IOMAP: BenchIoBitmap = BenchIoBitmap([0; IOMAP_BYTES]);

/// Write a full I/O permission bitmap's worth of bytes into this CPU's bench-only scratch region,
/// the way `schedule()` would write the incoming thread's bitmap into the TSS under option 1. The
/// pattern argument (varied per call by the caller) and the touch after are both there so nothing
/// about this write is provably dead code to the optimizer.
///
/// Safe to call from anywhere: the internal `unsafe` is this CPU's own static, single-hart, no
/// concurrent access, the same fact `bench.rs`'s single-threaded caller already relies on.
#[cfg(feature = "bench")]
pub fn bench_write_io_bitmap(pattern: u8) -> u8 {
    // SAFETY: single-hart bench boot; writes and reads this CPU's own static, never aliased.
    unsafe {
        let base = (&raw mut BENCH_IOMAP.0).cast::<u8>();
        base.write_bytes(pattern, IOMAP_BYTES);
        base.add(IOMAP_BYTES - 1).read()
    }
}
