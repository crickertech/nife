//! **The MMU, `x86_64`.** The third implementation of the seam milestone 20 built: `crates/paging`
//! owns the level walk and `paging::x86_64::Ia32e` owns the entry format, and this module is the
//! glue that names `CR3`, the direct map, and where this machine's devices are.
//!
//! # Two bases, which is the one structural difference of the three architectures
//!
//! aarch64 and RISC-V put the kernel image and the direct map at a single base, so one constant and
//! one subtraction serve both. `x86_64` cannot: `x86_64-unknown-none` uses `code-model: kernel`,
//! which promises LLVM that every symbol is in the **top 2 GiB**, so the image is pinned at
//! [`KERNEL_VA_BASE`] (`0xffffffff80000000`) and there is no room above it to name more than 2 GiB
//! of physical memory. `VA = PA | KERNEL_VA_BASE` was therefore never going to reach a real
//! machine's RAM, and it is not even invertible above the line: the local APIC at `0xfee00000` maps
//! to a distinct address whose inverse is not `0xfee00000`.
//!
//! So the two jobs that constant was doing are split, exactly as Linux splits them (milestone 161,
//! item 1 of the roadmap's open list):
//!
//! | What | Base | Who needs it there |
//! |---|---|---|
//! | The kernel **image** | [`KERNEL_VA_BASE`] `0xffffffff80000000` | the code model, non-negotiably |
//! | The **direct map** of physical memory | [`DIRECT_MAP_BASE`] `0xffff888000000000` | [`phys_to_virt`], and it has room for 64 TiB |
//!
//! They are separate PML4 entries (511 and 273), so nothing about them interferes, and both are
//! canonically high, so `Ia32e::in_half(Half::High, ..)` admits both by the same bit-47 test. The
//! `paging` crate needed no change for either, which is milestone 20's claim holding a second time.
//!
//! [`phys_to_virt`] is the direct map's arithmetic and [`virt_to_phys`] inverts **both** bases,
//! because the kernel asks it about image addresses (`memory::image_start`, linker symbols) as well
//! as about direct-map ones. That two-branch shape is Linux's `__pa()`, for the same reason.
//!
//! # Why `boot.s` installs the direct map, and not this module
//!
//! The roadmap flagged one hazard here and it is real: `phys_to_virt` must not change meaning under
//! anything that already used it. The frame allocator's bitmap is the sharp case, because
//! `memory::bring_up_frames` turns a physical address into a `&'static mut [u8]` and **stores it**,
//! long before there is a fine map; the PVH structure and the ACPI tables are read the same way.
//!
//! Rather than sequence around that (map the old alias too, switch, then drop it), `boot.s` gives
//! the boot tables a PML4[273] entry pointing at the same low PDPT the identity map uses. The
//! direct map therefore exists from before the first Rust instruction, covering the low 4 GiB, and
//! [`init`] widens it rather than introducing it. **`phys_to_virt` means one thing for the
//! machine's whole life**, so the hazard stops existing instead of being documented for the next
//! reader to remember. The cost is one PML4 entry and no memory at all.
//!
//! What [`init`] *does* drop is the identity map, which was a complete alias of physical memory
//! sitting in the half user programs will get.
//!
//! # BUGS
//!
//! - **The direct map is built out of 4 KiB leaves**, because the shared `Mapper` maps 4 KiB pages
//!   and nothing else. That is 8 bytes of page table per 4 KiB of physical memory, or 0.2% of RAM:
//!   512 KiB of tables for QEMU's 256 MiB, and ~64 MiB on a 32 GiB machine. Linux uses 2 MiB and
//!   1 GiB leaves for exactly this map. Fixing it is a change to `crates/paging` (a leaf size in
//!   the format trait) that all three architectures would want, not an x86 patch.
//!
//! - **`mmu::init` must allocate its page tables from the low 4 GiB**, because that is all the boot
//!   tables' direct map reaches and the mapper writes every table through it. The frame allocator
//!   hands out the lowest free frame first, so this holds today on any machine; a machine whose RAM
//!   *starts* above 4 GiB would fault inside `map_everything`. Widening the boot map to 1 GiB
//!   leaves (one PDPT covers 512 GiB) is the fix, and it needs a `CPUID` check for `PDPE1GB`.
//!
//! - **`CR4.PGE` is off, so the `G` bit the kernel's `Flags` set is ignored.** Every `mov cr3`
//!   therefore flushes the whole TLB, which is correct and slow. Turning PGE on is worth doing with
//!   ring 3 (roadmap item 3), where a context switch starts happening often enough to measure, and
//!   it has to be turned on *after* the fine map is installed or the boot map's non-global entries
//!   are the ones that get pinned.
//!
//! - **User address spaces are still `unimplemented!()`.** Nothing runs in ring 3 yet (roadmap item
//!   3), so `CR3` never names anything but the kernel root; those functions stay loud stubs.

use core::sync::atomic::{AtomicU64, Ordering};

use paging::x86_64::Ia32e;
use paging::{Flags, Half, MapError, Mapper, PAGE_SIZE, PageTable};

use crate::memory;

/// This architecture's page-table format. Portable code names it as `arch::mmu::Format`.
pub type Format = Ia32e;

/// The format used to build IOMMU (VT-d) translation domains. The same four-level format the CPU
/// uses, which is not a coincidence: VT-d's second-level tables were designed to be walked by the
/// same hardware logic. Kept as a separate name because the two are separate decisions on other
/// architectures and one day may be here.
#[cfg_attr(not(test), allow(dead_code))]
pub type DmaFormat = Ia32e;

/// The base of the kernel **image**'s virtual addresses.
///
/// **Fixed at 0xffffffff80000000 by the target's code model**, not chosen for taste: see the module
/// header and link-x86_64.ld. It decomposes as PML4[511], PDPT[510], PD[0], which is why `boot.s`
/// can alias the first gigabyte into the high half with a single PDPT entry.
///
/// This is deliberately **not** where physical memory is mapped; see [`DIRECT_MAP_BASE`].
pub const KERNEL_VA_BASE: u64 = 0xffff_ffff_8000_0000;

/// **The base of the direct map: where physical address `pa` is readable as `pa + this`.**
///
/// `0xffff888000000000`, which is Linux's `page_offset_base`, taken rather than invented because a
/// reader who has met one `x86_64` kernel has met this number. What the value has to satisfy is
/// short: canonically high (bit 47 set and sign-extended, so the same `Ia32e::in_half` test that
/// admits the kernel image admits this), clear of PML4[511] where the image lives, clear of the low
/// half that ring 3 will get, and with room above it for all of physical memory. This has 64 TiB of
/// room before it would reach anything else, which is 2^46 and not a limit worth thinking about.
///
/// **`boot.s` installs the PML4 entry for this before any Rust runs**, so the arithmetic below is
/// valid from the first instruction rather than from [`init`]; the module header says why that
/// matters more than it looks like it should.
///
/// **Name provisional** (milestone 161): calef names the constants, and this one was minted by a
/// lane. The tree's analogous name is [`KERNEL_VA_BASE`], which is why this is `_BASE` too.
pub const DIRECT_MAP_BASE: u64 = 0xffff_8880_0000_0000;

/// The top-level (PML4) index the direct map occupies, **duplicated in `boot.s`** because a 32-bit
/// assembler cannot compute it from the constant above. This is the gate that keeps the two in
/// agreement: change [`DIRECT_MAP_BASE`] without changing `boot.s` and the kernel does not build.
const DIRECT_MAP_PML4_INDEX: u64 = 273;

const _: () = {
    assert!(
        DIRECT_MAP_BASE >> 47 == 0x1_ffff,
        "the direct map base must be canonically high, or every address in it faults"
    );
    assert!(
        (DIRECT_MAP_BASE >> 39) & 0x1ff == DIRECT_MAP_PML4_INDEX,
        "boot.s writes PML4[273]: change both or neither"
    );
    assert!(
        (KERNEL_VA_BASE >> 39) & 0x1ff != DIRECT_MAP_PML4_INDEX,
        "the image and the direct map must not share a PML4 entry"
    );
};

/// The kernel's view of physical address `pa`: its address in the direct map.
pub const fn phys_to_virt(pa: u64) -> u64 {
    pa + DIRECT_MAP_BASE
}

/// The physical address behind kernel virtual address `va`.
///
/// **Two branches, because this architecture has two kernel bases** (see the module header). An
/// address at or above [`KERNEL_VA_BASE`] is part of the kernel image, linked there by
/// link-x86_64.ld; anything else in the high half is a direct-map address. Both callers exist:
/// `memory::image_start` hands this a linker symbol, and `sched`/`kmem` hand it pointers that came
/// out of [`phys_to_virt`]. Linux's `__pa()` makes the same distinction for the same reason.
///
/// It is the exact inverse of [`phys_to_virt`] over the direct map, now for **all** of physical
/// memory rather than the low 2 GiB the single-base arithmetic could reach.
pub const fn virt_to_phys(va: u64) -> u64 {
    if va >= KERNEL_VA_BASE {
        va - KERNEL_VA_BASE
    } else {
        va - DIRECT_MAP_BASE
    }
}

/// COM1's I/O port on every x86 machine since the PC/AT, and on QEMU's `q35`. Not an address in the
/// memory space at all; see `arch/x86_64/port.rs` for what that means and why the console reaches it
/// through a different accessor than the other two architectures' UARTs.
pub const COM1_PORT: usize = 0x3f8;

/// The local APIC's default physical base. Relocatable through `IA32_APIC_BASE`, and nothing
/// relocates it; the constant is where the machine puts it at reset.
#[cfg_attr(not(test), allow(dead_code))]
pub const LOCAL_APIC_PHYS: u64 = 0xfee0_0000;

/// The IO APIC's default physical base on a PC-compatible machine. The real answer is in the ACPI
/// MADT, which this port does not parse yet; this is the value every machine so far has put there.
#[cfg_attr(not(test), allow(dead_code))]
pub const IO_APIC_PHYS: u64 = 0xfec0_0000;

/// **Where q35 puts the PCIe ECAM window.** The same `pci-host-ecam-generic` shape both `virt`
/// boards present through their device trees, discovered on x86 through ACPI's MCFG table instead.
/// Hardcoded until that table is parsed, and this is QEMU's q35 default.
#[cfg_attr(not(test), allow(dead_code))]
pub const PCI_ECAM_PHYS: u64 = 0xb000_0000;

/// How many PCI buses of the ECAM window the kernel maps. One bus is 1 MiB of configuration space;
/// the other two architectures map one for the same reason (everything QEMU puts on the bus is on
/// bus 0) and pay 1 MiB rather than the 256 MiB the full window would cost.
#[cfg_attr(not(test), allow(dead_code))]
pub const PCI_ECAM_BUSES: u16 = 1;
/// Bytes of ECAM actually mapped.
#[cfg_attr(not(test), allow(dead_code))]
pub const PCI_ECAM_MAPPED: u64 = PCI_ECAM_BUSES as u64 * 0x10_0000;

/// Is paging on? True from the moment `boot.s` set `CR0.PG`, which is before any Rust runs, so this
/// is a constant `true` in practice and is read back from the hardware anyway: the one thing worth
/// knowing here is what the machine says, not what we believe.
pub fn is_enabled() -> bool {
    let cr0: u64;
    // SAFETY: reads a control register. No side effects.
    unsafe {
        core::arch::asm!("mov {}, cr0", out(reg) cr0, options(nomem, nostack, preserves_flags));
    }
    cr0 & (1 << 31) != 0
}

/// The physical address of the page-table root the CPU is currently walking (`CR3`, with the
/// PCID/flag bits masked off).
pub fn current_root() -> u64 {
    let cr3: u64;
    // SAFETY: reads a control register. No side effects.
    unsafe {
        core::arch::asm!("mov {}, cr3", out(reg) cr3, options(nomem, nostack, preserves_flags));
    }
    cr3 & 0x000f_ffff_ffff_f000
}

/// Print what the MMU is doing, one line, on every boot. The x86 twin of the other two
/// architectures' summaries.
pub fn print_summary() {
    let root = KERNEL_ROOT.load(Ordering::Relaxed);
    if root == 0 {
        crate::println!(
            "  mmu         : 4-level paging on (cr3 {:#x}), boot map: 4 GiB identity + high alias",
            current_root(),
        );
    } else {
        crate::println!(
            "  mmu         : fine W^X 4-level map installed (cr3 {:#x}), image {:#x}, direct map {:#x}",
            current_root(),
            KERNEL_VA_BASE,
            DIRECT_MAP_BASE,
        );
        // The cost, measured rather than asserted, because this map is built out of 4 KiB leaves
        // and the module's BUGS section says what that is worth on a bigger machine. On QEMU's
        // 256 MiB it is the number to watch if the direct map ever grows a second consumer.
        crate::println!(
            "                {} KiB of page tables, no identity map, guard pages are holes",
            TABLE_FRAMES.load(Ordering::Relaxed) * PAGE_SIZE / 1024,
        );
    }
}

// ---------------------------------------------------------------------------------------------
// The fine-grained kernel map (milestone 161, roadmap item 1). The third instance of the same
// shape aarch64 and riscv64 already have: build a root, walk it with `paging::Mapper`, check the
// things that would kill us in software, then hand the root to the hardware.
// ---------------------------------------------------------------------------------------------

/// The kernel's fine-map root, saved by [`init`] so a secondary CPU can adopt it and so the kernel
/// mapper does not have to read `CR3` back (which will name a *process* root once ring 3 exists).
static KERNEL_ROOT: AtomicU64 = AtomicU64::new(0);

/// How many frames the fine map cost, root and intermediate tables together. Reported on every
/// boot rather than left to be estimated: see this module's BUGS on 4 KiB leaves.
static TABLE_FRAMES: AtomicU64 = AtomicU64::new(0);

/// Serializes edits to the kernel's live tables: two CPUs must not mutate them at once. Same role
/// and lock rank as the other two architectures' `KERNEL_MMU`.
static KERNEL_MMU: crate::sync::IrqSafeMutex<()> =
    crate::sync::IrqSafeMutex::new(crate::sync::rank::KERNEL_MMU, ());

/// **Replace the boot map with a fine-grained one**: W^X kernel sections, the guard pages actually
/// unmapped, device registers device-typed, a direct map that covers all of physical memory, and
/// **no identity map** (which was a complete alias of physical memory in the half user programs
/// get).
///
/// We are already executing in the high half on the boot table, and the fine table maps the same
/// kernel VAs (image and direct map alike) to the same frames, so the `mov cr3` is seamless: the
/// next instruction fetch resolves identically. That is what [`verify`] checks before the hardware
/// bets the machine on it.
///
/// **The console does not enter into it here, unlike on the other two architectures.** x86's COM1 is
/// an I/O port, not a mapping, so nothing this function does can make the machine go silent. That is
/// a real convenience while debugging and it is also the reason a port capability is an open
/// question (DECISIONS §121).
pub fn init() {
    let free_before = memory::free_frames();
    let root = memory::alloc()
        .expect("no frame for the root page table")
        .addr();
    // SAFETY: a fresh frame from the allocator, reachable through the boot tables' direct map. Zero
    // it before the hardware could ever walk it.
    unsafe {
        (*phys_to_ptr(root)).entries = [0; paging::ENTRIES];
    }

    // SAFETY: `root` is zeroed and page-aligned; `phys_to_ptr` is valid because `boot.s` installed
    // the direct map over the low 4 GiB before any Rust ran, so every frame the mapper allocates is
    // addressable through it (see this module's BUGS for the >4 GiB case).
    let mut mapper = unsafe {
        Mapper::<_, _, Ia32e>::new(
            root,
            Half::High,
            || memory::alloc().map(|f| f.addr()),
            phys_to_ptr,
        )
    };

    map_everything(&mut mapper).expect("failed to build the kernel page tables");
    verify(&mapper);
    TABLE_FRAMES.store(
        (free_before - memory::free_frames()) as u64,
        Ordering::Relaxed,
    );

    // SAFETY: `verify` just walked these tables and asserted they cover this function's own code,
    // this CPU's stack, and the direct map every pointer already in flight was derived through.
    unsafe { install(root) };

    KERNEL_ROOT.store(root, Ordering::Relaxed);
}

/// Install the tables rooted at physical `root` by writing `CR3`.
///
/// Writing `CR3` invalidates every non-global TLB entry on this CPU, and `CR4.PGE` is clear, so in
/// practice it invalidates everything: there is no stale boot-table translation left afterwards.
///
/// # Safety
/// `root` must be a complete kernel map covering the currently-executing code, the current stack,
/// and any memory touched before the next barrier; otherwise the instruction after the write is
/// fetched through a table that does not describe it, which on this architecture is a page fault
/// escalating to a triple fault and a silent machine reset.
unsafe fn install(root: u64) {
    // SAFETY: the caller's contract. This is a control-register write with no memory operand.
    unsafe {
        core::arch::asm!("mov cr3, {}", in(reg) root, options(nostack, preserves_flags));
    }
}

/// Adopt the kernel map on a secondary CPU. Every CPU shares one kernel root, so there is nothing
/// per-CPU to build.
///
/// Unreachable today: `arch::cpu_start` returns an error on this architecture, so no secondary is
/// ever started (roadmap item 5). It is written because the shape is fixed by the other two ports
/// and guessing at it later is how a bring-up sequence acquires an ordering bug.
#[allow(dead_code)]
pub fn init_secondary() {
    let root = KERNEL_ROOT.load(Ordering::Relaxed);
    assert_ne!(
        root, 0,
        "a secondary CPU reached init_secondary before init"
    );
    // SAFETY: the primary built this root, verified it, and is running on it.
    unsafe { install(root) };
}

/// The kernel's live page tables, as a `Mapper`. **Call only while holding [`KERNEL_MMU`].**
#[allow(clippy::type_complexity)]
fn kernel_mapper() -> Mapper<impl FnMut() -> Option<u64>, fn(u64) -> *mut PageTable, Ia32e> {
    let root = KERNEL_ROOT.load(Ordering::Relaxed);
    // SAFETY: `root` is the fine kernel table `init` built; the direct map makes `phys_to_ptr` valid
    // for every table frame.
    unsafe {
        Mapper::new(
            root,
            Half::High,
            || memory::alloc().map(|f| f.addr()),
            phys_to_ptr,
        )
    }
}

/// Map one page in the kernel's address space.
pub fn map_page(va: u64, pa: u64, flags: Flags) -> Result<(), MapError> {
    let _guard = KERNEL_MMU.lock(); // exclusive: two CPUs must not mutate the tables at once
    kernel_mapper().map(va, pa, flags)
}

/// Unmap one page in the kernel's address space, invalidate the TLB, and return the physical frame
/// (the caller's to free; the mapper never owned it).
#[allow(dead_code)]
pub fn unmap_page(va: u64) -> Result<u64, MapError> {
    let _guard = KERNEL_MMU.lock(); // exclusive: see map_page
    let (pa, flush) = kernel_mapper().unmap(va)?;
    flush.flush(flush_tlb);
    Ok(pa)
}

/// Invalidate the TLB entry for `va`. The x86 instruction is `invlpg`, which unlike aarch64's
/// `tlbi ..., is` is **local to this CPU**: a multi-CPU kernel needs a software shootdown protocol
/// (an IPI), the same problem RISC-V solves with SBI RFENCE. There is one CPU here (roadmap item 5),
/// so the local invalidate is the whole of it, and this is the line that will need company.
pub fn flush_tlb(va: u64) {
    // SAFETY: TLB maintenance is always sound. Getting it wrong means a stale translation, which is
    // the memory-unsafety that matters here rather than Rust's.
    unsafe {
        core::arch::asm!("invlpg [{}]", in(reg) va, options(nostack, preserves_flags));
    }
}

/// Translate a kernel virtual address through the live kernel tables.
pub fn translate(va: u64) -> Option<(u64, Flags)> {
    let _guard = KERNEL_MMU.lock();
    kernel_mapper().translate(va)
}

/// **Is this kernel address mapped, asked without taking a lock?**
///
/// For fault handlers, which is why it does not go through [`translate`]: a handler that has already
/// lost the machine must not block on a lock whose holder might be the thread that just died. Same
/// disposition and same caller (`stack::print_text_words`) as the RISC-V twin.
pub fn is_mapped(va: u64) -> bool {
    let root = KERNEL_ROOT.load(Ordering::Relaxed);
    if root == 0 {
        // Before `init`, the boot map covers the low 4 GiB three ways over and nothing is a hole.
        return true;
    }
    // SAFETY: `root` is the live kernel root; the direct map makes `phys_to_ptr` valid; a translate
    // allocates nothing, so the `|| None` allocator is never called.
    let mapper = unsafe { Mapper::<_, _, Ia32e>::new(root, Half::High, || None, phys_to_ptr) };
    mapper.translate(va).is_some()
}

/// Build every mapping the kernel needs. Mirrors the aarch64 and RISC-V `map_everything`, and the
/// order is the same: physical memory first, then the image's own sections over the top of it.
fn map_everything<A, P>(m: &mut Mapper<A, P, Ia32e>) -> Result<(), MapError>
where
    A: FnMut() -> Option<u64>,
    P: Fn(u64) -> *mut PageTable,
{
    // 1. The direct map. **Every RAM region**, not just the low gigabyte the old single-base
    //    arithmetic could reach, which is the whole point of this milestone's step.
    //
    //    The kernel image's own frames are skipped: on this architecture the image lives at a
    //    *different* base, so a direct-map entry for those frames would not collide with the
    //    section mappings below (as it would on the other two ISAs), it would be a second,
    //    WRITABLE alias of `.text`. W^X that a second mapping undoes is not W^X. Nothing needs
    //    those frames by physical address anyway: the image is on the frame allocator's forbidden
    //    list, so they are never handed out.
    let image_lo = memory::image_start();
    let image_hi = memory::image_end();
    for (start, size) in memory::ram_regions() {
        let end = start + size;
        direct_map(m, start, image_lo.min(end), Flags::kernel_data())?;
        direct_map(m, image_hi.max(start), end, Flags::kernel_data())?;
    }

    // 2. Memory the loader did not call usable RAM but which is still *memory*: the first megabyte
    //    (IVT, BDA, EBDA, the BIOS area the ACPI RSDP is scanned out of, and the page below 1 MiB a
    //    STARTUP IPI's vector will have to name), and the block of ACPI tables QEMU parks just
    //    above the top of RAM. Those are read through `phys_to_virt` and are in no RAM region, so
    //    without this they become arithmetic with no mapping the moment the boot map goes.
    map_firmware_regions(m)?;

    // 3. The kernel image, section by section, at its linked VAs. W^X: text executable and
    //    read-only, rodata neither writable nor executable, everything else writable and never
    //    executable. Until this map is installed the boot table had all of it present, writable AND
    //    executable, in both halves.
    map_range(m, text_start(), text_end(), Flags::kernel_code())?;
    map_range(m, rodata_start(), rodata_end(), Flags::kernel_rodata())?;
    map_range(m, data_start(), bss_end(), Flags::kernel_data())?;

    // 4. The stacks. The guard page below each is deliberately NOT mapped, which is the entire
    //    stack-overflow mechanism and is asserted in `verify`: a mapped guard page is protection
    //    that is silently off.
    map_range(m, stack_bottom(), stack_top(), Flags::kernel_data())?;
    for id in 0..crate::cpu::MAX_CPUS {
        let (bottom, top) = crate::smp::secondary_stack_span(id);
        map_range(m, bottom, top, Flags::kernel_data())?;
    }
    for id in 0..crate::cpu::MAX_CPUS {
        let (bottom, top) = crate::interrupt_stack::span(id);
        map_range(m, bottom, top, Flags::kernel_data())?;
    }

    // 5. The device registers, device-typed (uncacheable), one window each.
    //
    //    The local APIC's address comes from the machine's own MADT when ACPI answered, and falls
    //    back to the architectural reset default when it did not: `IA32_APIC_BASE` is relocatable,
    //    and mapping the constant while the hardware is somewhere else would be a page of nothing
    //    that reads as all-ones.
    let apic = super::irq::local_apic_phys().unwrap_or(LOCAL_APIC_PHYS);
    direct_map(m, apic, apic + PAGE_SIZE, Flags::device())?;

    // The IO APIC's own page. Nothing routes a device line through it yet (roadmap item 2); the
    // page is mapped here because the mapping is this module's job either way, and item 2 should be
    // redirection-table code rather than a page-table detour.
    direct_map(m, IO_APIC_PHYS, IO_APIC_PHYS + PAGE_SIZE, Flags::device())?;

    // Bus 0 of the PCIe ECAM window, the same one-bus cap the other two architectures apply and for
    // the same reason: every device QEMU puts on this machine is on bus 0, and all 256 buses would
    // be 64K leaves describing space that reads all-ones.
    direct_map(
        m,
        PCI_ECAM_PHYS,
        PCI_ECAM_PHYS + PCI_ECAM_MAPPED,
        Flags::device(),
    )?;

    Ok(())
}

/// The first megabyte: real-mode legacy that is nonetheless memory, and the only part of the address
/// space this kernel reads by absolute physical address (the BIOS Data Area's EBDA pointer at
/// `0x40e`, the BIOS area the RSDP is scanned out of). The frame allocator clips it rather than
/// managing it, so it is in no RAM region and would otherwise be unmapped.
const LOW_MEGABYTE: u64 = 0x10_0000;

/// **Direct-map memory the frame allocator does not manage but the kernel still reads**: the first
/// megabyte, and every loader-reserved entry below the top of RAM (ACPI tables, firmware
/// scratch).
///
/// The bound is the top of RAM, and it is the load-bearing part rather than a tidiness rule. The
/// reserved entries *above* the top of RAM are MMIO windows (the PCIe ECAM range, the LAPIC/HPET
/// block, the flash at 4 GiB), which must be device-typed rather than cacheable and are mapped by
/// name in [`map_everything`]'s step 5. Mapping "everything below the highest reserved address"
/// instead would be correct on QEMU's 256 MiB and would swallow a real machine's whole 3-4 GiB MMIO
/// hole into a cacheable direct map.
///
/// Physical page 0 is deliberately left out, so that `phys_to_virt(0)` faults instead of quietly
/// naming the interrupt vector table.
///
/// Maps only the low megabyte when the PVH structure cannot be re-read. That is the right failure:
/// this kernel got its RAM regions from the same structure, so a machine that reaches here without
/// one had no ACPI tables to lose access to either.
///
/// **Name provisional** (milestone 161).
fn map_firmware_regions<A, P>(m: &mut Mapper<A, P, Ia32e>) -> Result<(), MapError>
where
    A: FnMut() -> Option<u64>,
    P: Fn(u64) -> *mut PageTable,
{
    // A reserved entry can abut or overlap something step 1 already mapped, and the mapper refuses
    // to overwrite (break-before-make). Ask before mapping rather than treating `AlreadyMapped` as
    // success, which would also swallow a genuine collision.
    let fill = |m: &mut Mapper<A, P, Ia32e>, from: u64, to: u64| -> Result<(), MapError> {
        for pa in (from..to).step_by(PAGE_SIZE as usize) {
            if m.translate(phys_to_virt(pa)).is_none() {
                m.map(phys_to_virt(pa), pa, Flags::kernel_data())?;
            }
        }
        Ok(())
    };

    fill(m, PAGE_SIZE, LOW_MEGABYTE)?;

    let Some(top_of_ram) = memory::ram_regions()
        .map(|(start, size)| start + size)
        .max()
    else {
        return Ok(());
    };
    let Some(info) = super::machine::boot_info(crate::DTB.load(Ordering::Relaxed)) else {
        return Ok(());
    };

    for i in 0..info.memmap_entries as usize {
        let Some(e) = super::machine::memory_map_entry(&info, i) else {
            break;
        };
        if e.is_usable_ram() {
            continue; // step 1 already did these, and did them minus the kernel image
        }
        let start = (e.addr & !(PAGE_SIZE - 1)).max(LOW_MEGABYTE);
        let end = e.end().next_multiple_of(PAGE_SIZE).min(top_of_ram);
        if end <= start {
            continue;
        }
        fill(m, start, end)?;
    }
    Ok(())
}

/// Map a range of *virtual* addresses to the physical ones they were linked against.
fn map_range<A, P>(
    m: &mut Mapper<A, P, Ia32e>,
    va_start: u64,
    va_end: u64,
    flags: Flags,
) -> Result<(), MapError>
where
    A: FnMut() -> Option<u64>,
    P: Fn(u64) -> *mut PageTable,
{
    if va_end <= va_start {
        return Ok(());
    }
    let pages = (va_end - va_start).div_ceil(PAGE_SIZE);
    m.map_range(va_start, virt_to_phys(va_start), pages, flags)
}

/// Map a range of *physical* addresses into the direct map at [`phys_to_virt`].
fn direct_map<A, P>(
    m: &mut Mapper<A, P, Ia32e>,
    pa_start: u64,
    pa_end: u64,
    flags: Flags,
) -> Result<(), MapError>
where
    A: FnMut() -> Option<u64>,
    P: Fn(u64) -> *mut PageTable,
{
    if pa_end <= pa_start {
        return Ok(());
    }
    let pages = (pa_end - pa_start).div_ceil(PAGE_SIZE);
    m.map_range(phys_to_virt(pa_start), pa_start, pages, flags)
}

/// Walk the tables in software and check the things that would kill us, **before** the hardware bets
/// the machine on them. The x86 counterpart of the other two architectures' `verify`, and it earns
/// its keep here more than anywhere: a mistake in these tables is not a fault report, it is a page
/// fault taken with a page-table root that cannot describe the fault handler, which escalates to a
/// double fault, then to a triple fault, which QEMU shows as a silent machine reset.
fn verify<A, P>(m: &Mapper<A, P, Ia32e>)
where
    A: FnMut() -> Option<u64>,
    P: Fn(u64) -> *mut PageTable,
{
    // The code we are executing right now. If this is wrong, the instruction after `mov cr3` never
    // gets fetched.
    let here = init as *const () as u64;
    let (pa, flags) = m
        .translate(here)
        .expect("the code switching tables is not mapped: we would die on the next fetch");
    assert_eq!(pa, virt_to_phys(here), "our .text maps to the wrong frame");
    assert!(
        flags.is_kernel_executable(),
        "our own .text is not executable"
    );
    assert!(
        !flags.is_writable(),
        "our own .text is writable (W^X violated)"
    );

    // This CPU's stack, which the `mov cr3` does not push to but the return from `install` does.
    let sp = super::current_sp();
    assert!(
        m.translate(sp).is_some(),
        "the current stack is not mapped: the return from `install` would fault"
    );

    // The direct map, asked about a frame the allocator actually owns rather than about the
    // arithmetic. Everything already in flight (the frame bitmap, the ACPI tables, every page table
    // this walk just wrote) is reached this way, so a direct map that came out short is the failure
    // that would look like memory corruption rather than like a page fault.
    let bitmap = memory::bitmap_region().0;
    assert!(
        m.translate(phys_to_virt(bitmap)).is_some(),
        "the frame allocator's own bitmap is not in the direct map"
    );

    // And the identity map is gone, which is half of what this step is for: an identity map is a
    // complete alias of physical memory sitting in the half ring 3 will get. Asked of the PML4
    // entry rather than through `translate`, which would answer `None` for any low address simply
    // because this mapper serves the high half, and so would pass whatever the tables said.
    //
    // SAFETY: `m.root()` is the table this function has been walking, reachable through the direct
    // map like every other table frame.
    let low_half_root = unsafe { (*phys_to_ptr(m.root())).entries[0] };
    assert_eq!(
        low_half_root, 0,
        "the low half of the fine tables is not empty: an identity map survived"
    );

    // The guard page must NOT be mapped, or stack-overflow protection is silently off and the next
    // time anyone finds out is during an overflow.
    assert!(
        m.translate(stack_guard()).is_none(),
        "the guard page IS mapped: stack overflow protection is off"
    );
    for id in 0..crate::cpu::MAX_CPUS {
        assert!(
            m.translate(crate::smp::secondary_stack_guard(id)).is_none(),
            "a secondary's guard page IS mapped: its stack overflow protection is off"
        );
        assert!(
            m.translate(crate::interrupt_stack::guard(id)).is_none(),
            "an interrupt stack's guard page IS mapped: its overflow protection is off"
        );
    }
}

// ---------------------------------------------------------------------------------------------
// The image bounds, read back out of the linker script. These are real: the addresses exist the
// moment the image is linked, and `stack.rs` (the overflow canary and the backtrace's is-this-code
// test) needs them before anything else here works.
// ---------------------------------------------------------------------------------------------

macro_rules! linker_symbol {
    ($(#[$doc:meta])* $name:ident, $sym:ident) => {
        $(#[$doc])*
        pub fn $name() -> u64 {
            unsafe extern "C" {
                static $sym: core::ffi::c_void;
            }
            (&raw const $sym) as u64
        }
    };
}

linker_symbol!(
    /// The boot stack's guard page: one page of address space beneath the stack, and a **hole** in
    /// the map [`init`] builds. `verify` asserts it is one; before `init` the boot map covers it
    /// like everything else, which is worth knowing while reading a stack-overflow report from the
    /// first few steps of the boot tour.
    stack_guard,
    __stack_guard
);
linker_symbol!(
    /// The low end of the boot stack (it grows down, so this is the limit, not the start).
    stack_bottom,
    __stack_bottom
);
linker_symbol!(
    /// One past the high end of the boot stack: where `rsp` started.
    stack_top,
    __stack_top
);
linker_symbol!(
    /// The first byte of the kernel's `.text`.
    text_start,
    __text_start
);
linker_symbol!(
    /// One past the last byte of the kernel's `.text`.
    text_end,
    __text_end
);
linker_symbol!(
    /// The first byte of the kernel's `.rodata`: readable, and executable by nobody.
    rodata_start,
    __rodata_start
);
linker_symbol!(
    /// One past the last byte of `.rodata`.
    rodata_end,
    __rodata_end
);
linker_symbol!(
    /// The first byte of `.data`. `.data`, `.bss` and the boot stack are contiguous in
    /// link-x86_64.ld, so one range from here to [`bss_end`] covers the writable image.
    data_start,
    __data_start
);
linker_symbol!(
    /// One past the last byte of `.bss`.
    bss_end,
    __bss_end
);

/// A page table at physical address `pa`, reached through the direct map.
pub(crate) fn phys_to_ptr(pa: u64) -> *mut paging::PageTable {
    phys_to_virt(pa) as *mut paging::PageTable
}

// ---------------------------------------------------------------------------------------------
// The user address spaces. None of this is built; see the module header.
//
// Each stub names itself and the reason, so a caller that reaches one gets a sentence rather than a
// hang. It is the scaffold the RISC-V port shipped on its first day, and what is left of the one
// this module shipped on its own: the kernel half above is now real.
//
// The x86 shape is aarch64's rather than RISC-V's, which is worth recording before anyone writes
// it: there are two roots' worth of behaviour in one register. `CR3` names the whole address space
// like RISC-V's `satp`, so a process's own root must carry the kernel's high-half entries
// (`share_kernel_half`), but the ASID equivalent (PCID, `CR3[11:0]`) is only honoured when
// `CR4.PCIDE` is set, and until it is, EVERY `mov cr3` flushes the entire non-global TLB. That is
// why `crates/asid`'s reuse contract will need a different answer here than on either of the others.
// ---------------------------------------------------------------------------------------------

macro_rules! not_yet {
    ($name:literal) => {
        unimplemented!(concat!(
            "x86_64 mmu::",
            $name,
            ": ring 3 does not exist on this architecture yet (milestone 161, roadmap item 3)"
        ))
    };
}

/// The physical root of the currently installed user address space.
pub fn current_user_root() -> u64 {
    not_yet!("current_user_root")
}

/// The root a thread with no user address space runs on.
#[allow(dead_code)]
pub fn reserved_root() -> u64 {
    not_yet!("reserved_root")
}

/// Compose the value `CR3` should hold for a root and an address-space id. Named for aarch64's
/// register because that is the arch contract's word; here it is `root | pcid`.
#[allow(dead_code)]
pub fn ttbr0_value(root: u64, asid: u16) -> u64 {
    let _ = (root, asid);
    not_yet!("ttbr0_value")
}

/// Give `root` the kernel's high-half entries, so a process's own tables map the kernel.
#[allow(dead_code)]
pub fn share_kernel_half(root: u64) {
    let _ = root;
    not_yet!("share_kernel_half")
}

/// Install the address space named by `cr3`.
///
/// # Safety
/// Not implemented; see the module header.
#[allow(dead_code)]
pub unsafe fn switch_user_root(cr3: u64) {
    let _ = cr3;
    not_yet!("switch_user_root")
}

/// Stop using any user address space.
#[allow(dead_code)]
pub fn deactivate_user() {
    not_yet!("deactivate_user")
}

/// Discharge every translation tagged with `asid`.
#[allow(dead_code)]
pub fn flush_asid(asid: u16) {
    let _ = asid;
    not_yet!("flush_asid")
}

/// Map one user page at `va`, allocating the leaf and any intermediate tables from `alloc`.
#[allow(dead_code)]
pub fn map_current_user_page(
    va: u64,
    flags: paging::Flags,
    alloc: impl FnMut() -> Option<u64>,
) -> Result<u64, paging::MapError> {
    let _ = (va, flags, alloc);
    not_yet!("map_current_user_page")
}

/// Map one user page at `va` onto the already-owned physical frame `phys`.
#[allow(dead_code)]
pub fn map_current_user_frame(
    va: u64,
    phys: u64,
    flags: paging::Flags,
    alloc: impl FnMut() -> Option<u64>,
) -> Result<(), paging::MapError> {
    let _ = (va, phys, flags, alloc);
    not_yet!("map_current_user_frame")
}

/// Unmap one user page at `va` in the space rooted at `root`.
#[allow(dead_code)]
pub fn unmap_user_at(root: u64, va: u64) -> Option<u64> {
    let _ = (root, va);
    not_yet!("unmap_user_at")
}

/// Translate `va` in the space rooted at physical `root`.
#[allow(dead_code)]
pub fn translate_at(root: u64, va: u64) -> Option<(u64, paging::Flags)> {
    let _ = (root, va);
    not_yet!("translate_at")
}

/// Translate `va` as a user address in the current space.
#[allow(dead_code)]
pub fn translate_user(va: u64) -> Option<(u64, paging::Flags)> {
    let _ = va;
    not_yet!("translate_user")
}

// ---------------------------------------------------------------------------------------------
// The device windows. Both `virt` machines describe theirs in a device tree; q35 describes its PCIe
// window in ACPI's MCFG and has no virtio-mmio bus at all.
// ---------------------------------------------------------------------------------------------

/// **There is no virtio-mmio transport on q35.** Both `virt` machines put eight virtio devices on a
/// memory-mapped bus the kernel finds by walking fixed slots; a PC has never had such a thing, and
/// every virtio device on q35 is a PCI function. So this base is zero and [`VIRTIO_SLOTS`] is zero,
/// which is what makes `virtio::find_block_device` find nothing rather than probe an address that
/// answers with bus noise. The PCIe transport (DECISIONS §18) is the one that carries here, which is
/// the arrangement that transport was built for.
#[cfg_attr(not(test), allow(dead_code))]
pub const VIRTIO_MMIO_BASE: u64 = 0;
/// Bytes between consecutive virtio-mmio transports. Meaningless with no bus; see
/// [`VIRTIO_MMIO_BASE`].
#[cfg_attr(not(test), allow(dead_code))]
pub const VIRTIO_SLOT_STRIDE: u64 = 0x1000;
/// **Zero virtio-mmio slots**, which is the fact, not a placeholder. See [`VIRTIO_MMIO_BASE`].
#[cfg_attr(not(test), allow(dead_code))]
pub const VIRTIO_SLOTS: u64 = 0;
/// The interrupt the first virtio-mmio slot would raise. Unreachable with no slots.
#[cfg_attr(not(test), allow(dead_code))]
pub const VIRTIO_IRQ_BASE: u32 = 0;

/// Bytes of PCI BAR space the kernel maps for device registers, matching what the other two
/// architectures reserve. 2 MiB covers every BAR QEMU hands out on this machine.
#[cfg_attr(not(test), allow(dead_code))]
pub const PCI_BAR_MAPPED: u64 = 0x20_0000;

/// The first interrupt a PCI function raises.
///
/// **Zero, and honestly so.** On the other two machines this is a fixed base the device tree states
/// and legacy INTx lines land at base + pin. On q35 a PCI function's legacy interrupt goes through
/// the PIRQ router to an IO APIC input the ACPI `_PRT` names, and MSI/MSI-X bypasses the routing
/// entirely by writing a vector straight to the local APIC. Neither answer is a constant, and this
/// port reads neither table; zero is the marker for that rather than a value to trust.
#[cfg_attr(not(test), allow(dead_code))]
pub const PCI_IRQ_BASE: u32 = 0;
