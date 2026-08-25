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
//! - **Only RAM and reservations cross the seam.** `memory::bring_up_frames` takes those two, and
//!   the device *windows* the device-tree front end also discovers (the interrupt controller, the
//!   RTC, the UART's interrupt line, the PCIe ECAM range) are still read from a tree by the front
//!   end and stay `None` here. What ACPI answers is handed to `arch::x86_64::irq` **directly** by
//!   the boot tour rather than through `memory.rs`'s statics, so `memory::pci_regions()` and
//!   friends still report nothing on x86 even though the MCFG answered a few lines earlier.
//!   Widening the seam is its own milestone; see notes/x86-port.md.
//! - **COM1's interrupt is discovered and not used.** [`Acpi::isa_irqs`] resolves all sixteen
//!   legacy IRQs, so `isa_irqs[4]` is the console UART's line and could be routed the way the PIT's
//!   is; the x86 console is polled, so nothing asks.

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
    self, ISA_IRQ_COUNT, IsaIrqRouting, MADT_PCAT_COMPAT, MadtEntry, Rsdp, SdtHeader,
    isa_irq_table, madt_entries, mcfg_entry, parse_madt, parse_rsdp, parse_sdt_header, root_entry,
    root_entry_count,
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
#[derive(Debug, Clone, Copy)]
pub struct Acpi {
    /// The local APIC's physical address, from the MADT (with any address-override applied).
    pub local_apic: Option<u64>,
    /// The first IO APIC's id, physical address and global-interrupt base.
    pub io_apic: Option<(u8, u32, u32)>,
    /// **The sixteen legacy ISA IRQs, resolved through the MADT's interrupt source overrides.**
    ///
    /// Not an `Option`, unlike everything around it, and for a reason worth stating: a machine with
    /// no overrides at all is not a machine with no answer. The ISA bus's own convention (identity
    /// onto the global interrupt space, active high, edge triggered) *is* the answer in that case,
    /// which is what `IsaIrqRouting::isa_default` encodes.
    pub isa_irqs: [IsaIrqRouting; ISA_IRQ_COUNT],
    /// How many processors the MADT lists as enabled.
    pub enabled_cpus: usize,
    /// How many it lists as present but not enabled.
    pub disabled_cpus: usize,
    /// True when the machine also has 8259 PICs, which must be masked before the APICs are used.
    pub has_8259: bool,
    /// The PCIe ECAM window, from the MCFG: base, first bus, last bus.
    pub ecam: Option<(u64, u8, u8)>,
}

impl Default for Acpi {
    fn default() -> Self {
        Self {
            local_apic: None,
            io_apic: None,
            isa_irqs: core::array::from_fn(|irq| IsaIrqRouting::isa_default(irq as u8)),
            enabled_cpus: 0,
            disabled_cpus: 0,
            has_8259: false,
            ecam: None,
        }
    }
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
    let bda = unsafe { phys_slice(EBDA_SEGMENT_PTR, 2) };
    // A SEGMENT, not an address: the BDA holds it in real-mode form, so it is shifted left four to
    // become the physical address. Reading it as an address directly would land in the first 64 KiB
    // and find nothing, quietly.
    let ebda = (u16::from_le_bytes([bda[0], bda[1]]) as u64) << 4;
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
    // The overrides in one pass of their own, in the crate, because resolving them is pure logic
    // over bytes and belongs where a host test can prove it. See its `the_timers_irq_0_resolves_to
    // _gsi_2` test for the case this whole table exists for.
    into.isa_irqs = isa_irq_table(body);

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

// ---------------------------------------------------------------------------------------------
// Turning the MCFG's ECAM window on. Reading it is not enough: nobody has told the chipset to
// route it yet.
// ---------------------------------------------------------------------------------------------

/// The legacy PCI configuration-mechanism ports. Present on every PC-compatible machine, reached
/// only by `in`/`out` (see `arch::x86_64::port`), and independent of whether ECAM decode is on,
/// which is exactly why they are the way to turn it on rather than a chicken-and-egg problem.
const CONFIG_ADDRESS: u16 = 0xcf8;
const CONFIG_DATA: u16 = 0xcfc;

/// `CONFIG_ADDRESS` naming bus 0, device 0, function 0 (the host bridge, always present), register
/// `0x60`: the enable bit is set (bit 31), bus/device/function are all zero, and the register
/// offset is `0x60` (`PCIEXBAR`, the host bridge's memory-mapped-config-space base-address
/// register on every Intel-compatible chipset this kernel has run on, q35 included).
const PCIEXBAR_CONFIG_ADDRESS: u32 = 0x8000_0060;

/// **Turn the MCFG's ECAM window on.**
///
/// A real BIOS or UEFI programs the host bridge's `PCIEXBAR` register (base address, enable bit)
/// before it ever hands control to an OS, which is why a device-tree machine's port of this
/// kernel never has to think about this: firmware already turned the decode on, `virt`'s
/// `pci-host-ecam-generic` binding just states where. PVH is a hypervisor entry protocol, not
/// firmware, so nothing has written this register by the time this runs, and it is not a
/// hypothetical: measured on QEMU 2026-08-24, a read of the physical address ACPI's MCFG names,
/// before this function runs, **faults** (the monitor's `xp` answers "Cannot access memory")
/// rather than reading the all-ones an absent *device*'s config space would, because the address
/// is not routed to the ECAM window at all yet. After writing `base | 1` here the same address
/// reads the host bridge's own vendor and device id (`8086:29c0` on q35), confirmed the same way,
/// which is the evidence this is the right register and the right bit rather than a guess that
/// happens not to crash.
///
/// **This is very likely a PVH-only step**, not a general x86 fact. Milestone 87's `OptiPlex` boots
/// through real UEFI (notes/x86-port.md's BUGS already names the gap), and real firmware
/// programs this register as a matter of course while bringing PCIe up; calling this there should
/// find the register already carrying the base MCFG itself reports, and rewriting it to the same
/// value is a no-op rather than a correction. It is written here rather than assumed so this port
/// does not depend on that being true.
///
/// Uses the legacy configuration mechanism rather than the ECAM window itself, which is the only
/// way to bootstrap: nothing can read the ECAM window to turn the ECAM window on.
pub fn enable_pcie_ecam(base: u64) {
    // SAFETY: 0xcf8/0xcfc are the legacy PCI configuration ports, present on every PC-compatible
    // machine independent of ECAM, and this is exactly the register sequence firmware runs before
    // handing control to an OS. `base` is the MCFG's own base address; this port has never seen
    // one above 4 GiB and truncating to 32 bits is the same assumption `PCI_ECAM_PHYS` already
    // makes. The enable bit (bit 0) is set and the length field (bits 2:1) is left at 0 for
    // "256 MiB, buses 0..255", which is what the MCFG entry itself already states.
    unsafe {
        super::port::out32(CONFIG_ADDRESS, PCIEXBAR_CONFIG_ADDRESS);
        super::port::out32(CONFIG_DATA, (base as u32) | 1);
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
    print_isa_overrides(found);
    match found.ecam {
        Some((base, lo, hi)) => crate::println!(
            "                pcie ecam {base:#x}, buses {lo}..={hi} (mmu::PCI_ECAM_PHYS says {:#x})",
            super::mmu::PCI_ECAM_PHYS,
        ),
        None => crate::println!("                no MCFG: the PCIe window is not described"),
    }
}

/// **Print every legacy IRQ the MADT rewired**, and only those.
///
/// Sixteen lines of identity mapping would be noise; the ones that moved are the whole reason this
/// table is read, and the first of them is always the timer. A boot that printed nothing here on a
/// PC would itself be the finding.
fn print_isa_overrides(found: &Acpi) {
    let moved = found
        .isa_irqs
        .iter()
        .enumerate()
        .filter(|(irq, r)| **r != IsaIrqRouting::isa_default(*irq as u8));
    for (irq, routing) in moved {
        crate::println!(
            "                isa irq {irq} -> gsi {} ({}, {})",
            routing.gsi,
            if routing.active_low {
                "active low"
            } else {
                "active high"
            },
            if routing.level_triggered {
                "level"
            } else {
                "edge"
            },
        );
    }
}

// ---------------------------------------------------------------------------------------------
// Bringing the frame allocator up from the PVH memory map.
// ---------------------------------------------------------------------------------------------

/// The most RAM regions handed to the allocator. `memory::bring_up_frames` indexes a fixed map of
/// this size, so more than this cannot be described; q35 produces three.
const MAX_RAM_REGIONS: usize = 16;

/// **The whole first megabyte is never allocatable**, and this is the one x86 reservation with no
/// counterpart on the other two architectures.
///
/// It holds the real-mode interrupt vector table, the BIOS data area, the extended BIOS data area,
/// the VGA window, option ROM shadow, and (on this boot) the `hvm_start_info` structure and the
/// memory map the loader wrote. Some of that is described as RAM by the memory map, because it
/// physically is; none of it is memory the kernel may hand out. It is also where an SMP real-mode
/// trampoline has to be copied, since a STARTUP IPI's vector can only name a page below this line.
const LOW_MEGABYTE: u64 = 0x0010_0000;

/// **Bring the frame allocator up from what the loader described.**
///
/// The x86 counterpart of `memory::init`'s device-tree front end: it turns the PVH memory map into
/// the two slices `memory::bring_up_frames` takes, and nothing else.
///
/// Returns how many RAM regions were used, which the boot print reports; a machine describing more
/// than [`MAX_RAM_REGIONS`] gets the first that many, and saying how many were taken is what makes
/// that visible rather than silent.
pub fn bring_up_memory(info: &BootInfo) -> usize {
    let mut ram = [dtb::Region { start: 0, size: 0 }; MAX_RAM_REGIONS];
    let mut count = 0;

    for i in 0..info.memmap_entries as usize {
        if count == MAX_RAM_REGIONS {
            break;
        }
        let Some(e) = memory_map_entry(info, i) else {
            break;
        };
        if !e.is_usable_ram() {
            continue;
        }
        // Clip the low megabyte off rather than reserving it afterwards. Either works; clipping is
        // the one that cannot be undone by a later `mark_free`, and the allocator's own rule is
        // that reserving has to win, which means order would otherwise matter.
        let start = e.addr.max(LOW_MEGABYTE);
        let end = e.end();
        if end <= start {
            continue;
        }
        ram[count] = dtb::Region {
            start,
            size: end - start,
        };
        count += 1;
    }

    // Two reservations, and both would be catastrophic to miss. The kernel image is the code
    // running right now, plus the boot page tables the CPU is walking (they live in `.boot_scratch`,
    // which the linker script puts inside the image bounds precisely so that this one entry covers
    // them). The low megabyte is clipped above rather than listed here.
    let forbidden = [dtb::Region {
        start: crate::memory::image_start(),
        size: crate::memory::image_end() - crate::memory::image_start(),
    }];

    crate::memory::bring_up_frames(&ram[..count], &forbidden);
    count
}
