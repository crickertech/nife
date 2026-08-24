//! **Reading what the loader said**, `x86_64`: the kernel side of
//! `machine_discovery::x86_64`.
//!
//! Deliberately thin. Everything that decides what a byte means is in the crate, host-tested; what
//! is here is the part that cannot be: turning the physical address the loader put in `ebx` into
//! bytes this kernel can look at, through the direct map. That is the same division `console.rs`
//! and `memory.rs` make with `crates/dtb` on the other two architectures.
//!
//! # BUGS
//!
//! - **Nothing consumes the memory map yet.** The boot tour prints it, which proves the handoff and
//!   the parse, and the frame allocator is still `memory::init`'s, which reads a device tree and so
//!   cannot run here. Closing that is the fine-map step; see
//!   design/roadmap/161-x86-64-kernel-port.md and the discovery-seam note in notes/x86-port.md.

use machine_discovery::x86_64::{BootInfo, MEMMAP_ENTRY_LEN, MemoryEntry, memory_entry};

use super::mmu::phys_to_virt;

/// The most memory-map entries the boot print will walk. QEMU's `q35` produces four with `-m 256M`;
/// a real machine with an assortment of firmware reservations produces a couple of dozen. The cap
/// exists because this walks a structure a loader wrote, and a corrupt entry count must bound the
/// loop rather than hang the boot before the console has said anything useful.
const MAX_ENTRIES: usize = 64;

/// Read the `hvm_start_info` at physical address `at`.
///
/// `None` when the magic does not match or the structure is unreadable, which covers the case that
/// matters: this kernel booted some other way and `ebx` held something else. Reporting `None` lets
/// the caller say so; reading on would produce a plausible memory map out of unrelated bytes.
pub fn boot_info(at: usize) -> Option<BootInfo> {
    if at == 0 {
        return None;
    }
    // SAFETY: `at` is the physical address the PVH loader put in `ebx`, reached through the direct
    // map, which covers the low gigabyte and so covers the low megabyte the loader places this in.
    // 56 bytes is the whole version-1 structure; the parser checks the magic before believing any
    // of them, and refuses a truncated read.
    let bytes = unsafe { core::slice::from_raw_parts(phys_to_virt(at as u64) as *const u8, 56) };
    BootInfo::parse(bytes).ok()
}

/// Read memory-map entry `index` of the map `info` describes.
pub fn memory_map_entry(info: &BootInfo, index: usize) -> Option<MemoryEntry> {
    if info.memmap == 0 || index >= info.memmap_entries as usize {
        return None;
    }
    let at = phys_to_virt(info.memmap) + (index * MEMMAP_ENTRY_LEN) as u64;
    // SAFETY: a physical address the loader gave us, reached through the direct map, for exactly
    // one entry's worth of bytes. The index is bounded by the count the loader stated above.
    let bytes = unsafe { core::slice::from_raw_parts(at as *const u8, MEMMAP_ENTRY_LEN) };
    memory_entry(bytes, 0)
}

/// Print the memory map, one line per entry plus a usable total. The x86 stand-in for the device
/// tree's RAM regions, and for now the only thing that reads the map at all.
pub fn print_memory_map(info: &BootInfo) {
    if info.memmap_entries == 0 {
        crate::println!(
            "  memory      : the loader described no memory map (version {})",
            info.version
        );
        return;
    }
    let shown = (info.memmap_entries as usize).min(MAX_ENTRIES);
    let mut usable = 0u64;
    crate::println!(
        "  memory      : {} regions from the PVH handoff, rsdp {:#x}",
        info.memmap_entries,
        info.rsdp,
    );
    for i in 0..shown {
        let Some(e) = memory_map_entry(info, i) else {
            break;
        };
        if e.is_usable_ram() {
            usable += e.size;
        }
        crate::println!(
            "                {:#014x}..{:#014x}  {}",
            e.addr,
            e.end(),
            e.kind.name(),
        );
    }
    if shown < info.memmap_entries as usize {
        crate::println!(
            "                ... and {} more",
            info.memmap_entries as usize - shown
        );
    }
    crate::println!("                usable ram: {} KiB", usable / 1024);
}
