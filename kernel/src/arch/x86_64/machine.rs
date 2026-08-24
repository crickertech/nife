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

// ---------------------------------------------------------------------------------------------
// ACPI: finding the tables, and reading the two the kernel needs first.
//
// Everything that decides what a byte means is in `machine_discovery::acpi`, host-tested. What is
// here is finding the root pointer and turning physical addresses into slices, neither of which a
// host test can do.
// ---------------------------------------------------------------------------------------------

use machine_discovery::acpi::{
    self, MADT_PCAT_COMPAT, MadtEntry, Rsdp, SdtHeader, madt_entries, mcfg_entry, parse_madt,
    parse_rsdp, parse_sdt_header, root_entry, root_entry_count,
};

/// The BIOS area the RSDP is required to be in when it is not in the EBDA: `0xe0000..0x100000`,
/// scanned on 16-byte boundaries.
const BIOS_AREA: core::ops::Range<u64> = 0x000e_0000..0x0010_0000;

/// Where the BIOS Data Area records the Extended BIOS Data Area's segment. The EBDA's first
/// kilobyte is the other place an RSDP is allowed to be.
const EBDA_SEGMENT_PTR: u64 = 0x0000_040e;

/// The most root-table entries the walk will follow. A machine has a dozen or two tables; the cap
/// exists because the count comes from a length field firmware wrote.
const MAX_TABLES: usize = 64;

/// The most MADT entries the walk will report, for the same reason.
const MAX_MADT_ENTRIES: usize = 256;

/// What the tables said, kept so the APIC and PCI bring-up can read it rather than re-walking.
///
/// Every field is `Option` because a machine is allowed not to have the table it comes from, and
/// "the firmware did not say" has to be distinguishable from "the firmware said zero". That is the
/// same posture `memory::uart_irq` takes on the other two architectures.
#[derive(Debug, Clone, Copy, Default)]
pub struct Acpi {
    /// The local APIC's physical address, from the MADT (with any address-override applied).
    pub local_apic: Option<u64>,
    /// The first IO APIC's id, physical address and global-interrupt base.
    pub io_apic: Option<(u8, u32, u32)>,
    /// How many processors the MADT lists as enabled.
    pub enabled_cpus: usize,
    /// How many it lists as present but not enabled.
    pub disabled_cpus: usize,
    /// True when the machine also has 8259 PICs, which must be masked before the APICs are used.
    pub has_8259: bool,
    /// The PCIe ECAM window, from the MCFG: base, first bus, last bus.
    pub ecam: Option<(u64, u8, u8)>,
}

/// Read `len` bytes at physical address `at` through the direct map.
///
/// # Safety
/// `at..at + len` must be inside the direct map (the low gigabyte) and must be memory rather than a
/// device register block, since this is an ordinary read.
unsafe fn phys_slice<'a>(at: u64, len: usize) -> &'a [u8] {
    // SAFETY: the caller's obligation, restated: an in-range physical address naming memory.
    unsafe { core::slice::from_raw_parts(phys_to_virt(at) as *const u8, len) }
}

/// Is the direct map able to reach this physical address? The boot map aliases only the low
/// gigabyte, so anything above it would compute an address that is arithmetically right and not
/// mapped, which faults a long way from here. Checked rather than assumed because these addresses
/// come from firmware.
fn reachable(at: u64, len: usize) -> bool {
    at != 0 && at.saturating_add(len as u64) <= 0x4000_0000
}

/// **Find the ACPI root pointer.**
///
/// Three places, in the order the specification and reality dictate:
///
/// 1. **What the loader said**, if anything. PVH has a field for it. QEMU leaves that field **zero**
///    (measured, 2026-08-23), which is exactly why the other two are not optional.
/// 2. **The first kilobyte of the EBDA**, whose segment is a 16-bit word in the BIOS Data Area.
/// 3. **`0xe0000..0x100000`**, the BIOS area.
///
/// 2 and 3 are a scan for an eight-byte string, so the checksum inside `parse_rsdp` is the only
/// thing separating a real hit from a coincidence. That is why nothing here does its own comparison
/// and calls it found.
pub fn find_rsdp(hint: u64) -> Option<(u64, Rsdp)> {
    if reachable(hint, 36)
        // SAFETY: `reachable` checked the range is in the direct map, and firmware places this
        // structure in ordinary memory.
        && let Ok(rsdp) = parse_rsdp(unsafe { phys_slice(hint, 36) })
    {
        return Some((hint, rsdp));
    }

    // SAFETY: the BIOS Data Area is at a fixed low physical address on every PC and is memory.
    let ebda_segment = u16::from_le_bytes([
        unsafe { phys_slice(EBDA_SEGMENT_PTR, 2) }[0],
        unsafe { phys_slice(EBDA_SEGMENT_PTR, 2) }[1],
    ]);
    let ebda = (ebda_segment as u64) << 4;
    if ebda >= 0x400
        && let Some(found) = scan_for_rsdp(ebda..ebda + 0x400)
    {
        return Some(found);
    }

    scan_for_rsdp(BIOS_AREA)
}

/// Scan `range` on 16-byte boundaries for a well-formed RSDP.
fn scan_for_rsdp(range: core::ops::Range<u64>) -> Option<(u64, Rsdp)> {
    let mut at = range.start & !0xf;
    while at + 36 <= range.end {
        if reachable(at, 36) {
            // SAFETY: `reachable` checked the range; the low megabyte is memory.
            let bytes = unsafe { phys_slice(at, 36) };
            if let Ok(rsdp) = parse_rsdp(bytes) {
                return Some((at, rsdp));
            }
        }
        at += 16;
    }
    None
}

/// Read the header of the table at physical `at`, verifying its checksum over the whole table.
///
/// A table whose checksum fails is reported as absent rather than used, which is the only safe
/// reading: a corrupt MADT would hand out an APIC address, and there is nothing downstream that
/// could notice it was wrong.
fn table_at(at: u64) -> Option<(SdtHeader, &'static [u8])> {
    if !reachable(at, acpi::SDT_HEADER_LEN) {
        return None;
    }
    // SAFETY: `reachable` checked the header's range.
    let header = parse_sdt_header(unsafe { phys_slice(at, acpi::SDT_HEADER_LEN) }).ok()?;
    if !reachable(at, header.length as usize) {
        return None;
    }
    // SAFETY: `reachable` checked the whole table's range, using the length the header states.
    let whole = unsafe { phys_slice(at, header.length as usize) };
    if !acpi::checksum_ok(whole) {
        return None;
    }
    Some((header, &whole[acpi::SDT_HEADER_LEN..]))
}

/// Walk the tables and collect what the kernel will need, printing a line per table as it goes.
///
/// Printing during the walk rather than after is deliberate: this is the first code in the boot that
/// follows pointers firmware wrote, so if one of them is wrong the last line printed says which
/// table was being read.
pub fn read_acpi(hint: u64) -> Acpi {
    let mut found = Acpi::default();

    let Some((at, rsdp)) = find_rsdp(hint) else {
        crate::println!("  acpi        : no RSDP found (loader said {hint:#x}, and no scan hit)");
        return found;
    };
    let (root, wide) = rsdp.root_table();
    crate::println!(
        "  acpi        : rsdp at {at:#x} (revision {}), root table {root:#x} ({})",
        rsdp.revision,
        if wide { "xsdt" } else { "rsdt" },
    );

    let Some((root_header, root_body)) = table_at(root) else {
        crate::println!("                root table unreadable or failed its checksum");
        return found;
    };
    let count = root_entry_count(root_header.length, wide).min(MAX_TABLES);

    for i in 0..count {
        let Some(pa) = root_entry(root_body, i, wide) else {
            break;
        };
        let Some((header, body)) = table_at(pa) else {
            crate::println!("                {pa:#012x}  unreadable or bad checksum");
            continue;
        };
        let name = header.signature_str().unwrap_or("????");
        crate::println!(
            "                {pa:#012x}  {name} ({} bytes)",
            header.length
        );
        match &header.signature {
            b"APIC" => read_madt(body, &mut found),
            b"MCFG" => read_mcfg(body, &mut found),
            _ => {}
        }
    }

    found
}

fn read_madt(body: &[u8], into: &mut Acpi) {
    let Ok(madt) = parse_madt(body) else {
        return;
    };
    into.local_apic = Some(madt.local_apic as u64);
    into.has_8259 = madt.flags & MADT_PCAT_COMPAT != 0;

    for entry in madt_entries(body).take(MAX_MADT_ENTRIES) {
        match entry {
            MadtEntry::LocalApic { enabled, .. } => {
                if enabled {
                    into.enabled_cpus += 1;
                } else {
                    into.disabled_cpus += 1;
                }
            }
            MadtEntry::IoApic {
                id,
                address,
                gsi_base,
            } => {
                if into.io_apic.is_none() {
                    into.io_apic = Some((id, address, gsi_base));
                }
            }
            // Overrides the 32-bit address in the fixed part, and comes after it in the list, which
            // is why this is applied here rather than being read first.
            MadtEntry::LocalApicAddressOverride(pa) => into.local_apic = Some(pa),
            _ => {}
        }
    }
}

fn read_mcfg(body: &[u8], into: &mut Acpi) {
    if let Some(e) = mcfg_entry(body, 0) {
        into.ecam = Some((e.base, e.start_bus, e.end_bus));
    }
}

/// Print what the tables said, after [`read_acpi`] has walked them.
pub fn print_acpi_summary(found: &Acpi) {
    match found.local_apic {
        Some(pa) => crate::println!(
            "                local apic {pa:#x}, {} cpu(s) enabled, {} disabled, 8259s {}",
            found.enabled_cpus,
            found.disabled_cpus,
            if found.has_8259 {
                "present (must be masked)"
            } else {
                "absent"
            },
        ),
        None => crate::println!("                no MADT: nothing knows where the APICs are"),
    }
    if let Some((id, address, gsi_base)) = found.io_apic {
        crate::println!("                io apic {id} at {address:#x}, gsi base {gsi_base}");
    }
    match found.ecam {
        Some((base, lo, hi)) => crate::println!(
            "                pcie ecam {base:#x}, buses {lo}..={hi} (mmu::PCI_ECAM_PHYS says {:#x})",
            super::mmu::PCI_ECAM_PHYS,
        ),
        None => crate::println!("                no MCFG: the PCIe window is not described"),
    }
}
