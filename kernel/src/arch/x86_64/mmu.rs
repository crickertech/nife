//! **The MMU, x86_64.** The third implementation of the seam milestone 20 built: `crates/paging`
//! owns the level walk and `paging::x86_64::Ia32e` owns the entry format, and this module is the
//! glue that names `CR3`, the direct map, and where this machine's devices are.
//!
//! # What is real here and what is not
//!
//! Real: the address arithmetic (the direct map, `phys_to_virt`/`virt_to_phys`), the format
//! selection, the device-window constants, and reading back whether paging is on. Those are what
//! the boot trampoline already established and what the console needs before anything else runs.
//!
//! **Not real: everything that builds or switches a page table.** `boot.s` left the machine on a
//! boot map of 2 MiB pages with no permission separation at all (every page present, writable and
//! executable, both halves), which is enough to run and not enough to be a kernel. Replacing it
//! with a fine-grained map, and then giving user address spaces their own roots, is the next step
//! of this milestone. Every function that would do that is a loud `unimplemented!()` naming itself,
//! the same scaffold the RISC-V port shipped on its first day, so that nothing mistakes a stub for
//! a working MMU. See design/roadmap/161-x86-64-kernel-port.md.
//!
//! # The direct map is exactly the other two architectures'
//!
//! `VA = PA | KERNEL_VA_BASE`, reversible, and a kernel virtual address has the same page-table
//! indices as its physical address. What differs is the constant, and it is not a free choice: the
//! `x86_64-unknown-none` target uses the **kernel code model**, which promises LLVM every symbol is
//! in the top 2 GiB. See link-x86_64.ld.
//!
//! # BUGS
//!
//! - **The direct map only covers the low 1 GiB**, because that is what `boot.s`'s high PDPT entry
//!   aliases. A physical address above 1 GiB has no high-half alias, so `phys_to_virt` computes an
//!   address that is arithmetically right and not mapped. QEMU's 256 MiB default keeps every real
//!   address well below the line; a machine with more memory than that would fault somewhere far
//!   from here. Fixing it is part of building the fine map.

use paging::x86_64::Ia32e;

/// This architecture's page-table format. Portable code names it as `arch::mmu::Format`.
pub type Format = Ia32e;

/// The format used to build IOMMU (VT-d) translation domains. The same four-level format the CPU
/// uses, which is not a coincidence: VT-d's second-level tables were designed to be walked by the
/// same hardware logic. Kept as a separate name because the two are separate decisions on other
/// architectures and one day may be here.
#[cfg_attr(not(test), allow(dead_code))]
pub type DmaFormat = Ia32e;

/// The base of the kernel's half of the address space, and of the direct map.
///
/// **Fixed at 0xffffffff80000000 by the target's code model**, not chosen for taste: see the module
/// header and link-x86_64.ld. It decomposes as PML4[511], PDPT[510], PD[0], which is why `boot.s`
/// can alias the first gigabyte into the high half with a single PDPT entry.
pub const KERNEL_VA_BASE: u64 = 0xffff_ffff_8000_0000;

/// The kernel's view of physical address `pa`.
pub const fn phys_to_virt(pa: u64) -> u64 {
    pa | KERNEL_VA_BASE
}

/// The physical address behind kernel virtual address `va`. The exact inverse of
/// [`phys_to_virt`] for any address in the direct map.
pub const fn virt_to_phys(va: u64) -> u64 {
    va & !KERNEL_VA_BASE
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
    crate::println!(
        "  mmu         : 4-level paging on (cr3 {:#x}), boot map: 4 GiB identity + 1 GiB high",
        current_root()
    );
}

// ---------------------------------------------------------------------------------------------
// Everything below is the fine-grained map and the user address spaces, which this milestone has
// not built. Each names itself and the reason, so a caller that reaches one gets a sentence rather
// than a hang. See the module header.
// ---------------------------------------------------------------------------------------------

macro_rules! not_yet {
    ($name:literal) => {
        unimplemented!(concat!(
            "x86_64 mmu::",
            $name,
            ": the fine-grained page tables are not built (milestone 161)"
        ))
    };
}

/// Replace the boot map with a fine-grained one: per-section kernel permissions, the guard page
/// actually unmapped, and devices mapped device-typed.
pub fn init() {
    not_yet!("init")
}

/// Adopt the kernel map on a secondary CPU.
#[allow(dead_code)]
pub fn init_secondary() {
    not_yet!("init_secondary")
}

/// Map one page in the kernel's address space.
#[allow(dead_code)]
pub fn map_page(va: u64, pa: u64, flags: paging::Flags) -> Result<(), paging::MapError> {
    let _ = (va, pa, flags);
    not_yet!("map_page")
}

/// Unmap one page in the kernel's address space.
#[allow(dead_code)]
pub fn unmap_page(va: u64) -> Result<u64, paging::MapError> {
    let _ = va;
    not_yet!("unmap_page")
}

/// Invalidate the TLB entry for `va`. The x86 instruction is `invlpg`, which unlike aarch64's
/// `tlbi ..., is` is **local to this CPU**: a multi-CPU kernel needs a software shootdown protocol
/// (an IPI), the same problem RISC-V solves with SBI RFENCE.
#[allow(dead_code)]
pub fn flush_tlb(va: u64) {
    let _ = va;
    not_yet!("flush_tlb")
}

/// Translate a kernel virtual address through the current tables.
#[allow(dead_code)]
pub fn translate(va: u64) -> Option<(u64, paging::Flags)> {
    let _ = va;
    not_yet!("translate")
}

/// Is `va` mapped in the kernel's address space?
#[allow(dead_code)]
pub fn is_mapped(va: u64) -> bool {
    let _ = va;
    not_yet!("is_mapped")
}

// ---------------------------------------------------------------------------------------------
// The image bounds, read back out of the linker script. These are real: the addresses exist the
// moment the image is linked, and `stack.rs` (the overflow canary and the backtrace's is-this-code
// test) needs them before anything else here works.
// ---------------------------------------------------------------------------------------------

/// The boot stack's guard page: one page of address space beneath the stack, left unmapped once the
/// fine tables are built. Until then it is reserved and mapped like everything else, which is a
/// difference from the other two architectures worth knowing while reading a stack-overflow report.
pub fn stack_guard() -> u64 {
    unsafe extern "C" {
        static __stack_guard: core::ffi::c_void;
    }
    (&raw const __stack_guard) as u64
}

/// The low end of the boot stack (it grows down, so this is the limit, not the start).
pub fn stack_bottom() -> u64 {
    unsafe extern "C" {
        static __stack_bottom: core::ffi::c_void;
    }
    (&raw const __stack_bottom) as u64
}

/// The first byte of the kernel's `.text`.
pub fn text_start() -> u64 {
    unsafe extern "C" {
        static __text_start: core::ffi::c_void;
    }
    (&raw const __text_start) as u64
}

/// One past the last byte of the kernel's `.text`.
pub fn text_end() -> u64 {
    unsafe extern "C" {
        static __text_end: core::ffi::c_void;
    }
    (&raw const __text_end) as u64
}

/// A page table at physical address `pa`, reached through the direct map.
pub(crate) fn phys_to_ptr(pa: u64) -> *mut paging::PageTable {
    phys_to_virt(pa) as *mut paging::PageTable
}

// ---------------------------------------------------------------------------------------------
// The user address spaces. None of this is built; see the module header.
//
// The x86 shape is aarch64's rather than RISC-V's, which is worth recording before anyone writes
// it: there are two roots' worth of behaviour in one register. `CR3` names the whole address space
// like RISC-V's `satp`, so a process's own root must carry the kernel's high-half entries
// (`share_kernel_half`), but the ASID equivalent (PCID, `CR3[11:0]`) is only honoured when
// `CR4.PCIDE` is set, and until it is, EVERY `mov cr3` flushes the entire non-global TLB. That is
// why `crates/asid`'s reuse contract will need a different answer here than on either of the others.
// ---------------------------------------------------------------------------------------------

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
