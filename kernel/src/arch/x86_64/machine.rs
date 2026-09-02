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
//! - **Only RAM and reservations cross the seam** through [`crate::memory::bring_up_page_frames`].
//!   The device *windows* the device-tree front end also discovers on the other two architectures
//!   have their own, narrower seams instead of that one: the interrupt controller (IO APIC) is
//!   reached directly by `arch::x86_64::irq` rather than through `memory.rs`'s statics (milestone
//!   161 item 2), and PCI and the UART's interrupt line are wired into `memory.rs`'s own statics
//!   from `main.rs`'s boot tour (`memory::record_pci_regions`, `memory::record_uart_irq`;
//!   milestones 165 and 176). **The only device window with no seam at all is the CMOS RTC**: it
//!   is not memory-mapped (two fixed I/O ports, not a page), so `memory::RTC_REGION`'s
//!   `Option<(u64, u64, u64)>` shape has nowhere to put it. See notes/x86-port.md and
//!   `kernel/src/arch/x86_64/port.rs`'s own doc comment.

use machine_discovery::x86_64::{
    BootInfo, MEMMAP_ENTRY_LEN, MODULE_ENTRY_LEN, MemoryEntry, Module, memory_entry, module,
};

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

/// **The initrd, if the loader put one in RAM** (milestone 161): x86's answer to the device tree's
/// `/chosen/linux,initrd-start`, which is what both other architectures read.
///
/// QEMU's PVH loader turns `-initrd FILE` into one entry in the module list `hvm_start_info` points
/// at. **The first module is taken and any others ignored**, which is a decision rather than an
/// oversight: PVH permits several, this kernel wants exactly one archive, and inventing a policy
/// for a case the machine never produces would be code nobody could test. A second module would be
/// left in RAM, unreserved and unread; if one ever appears, this is where it has to be handled.
pub fn initrd(info: &BootInfo) -> Option<Module> {
    if info.modules == 0 || info.module_count == 0 {
        return None;
    }
    let at = phys_to_virt(info.modules);
    // SAFETY: a physical address the loader gave us, reached through the direct map, for exactly one
    // entry's worth of bytes. The count was checked above, so entry 0 exists.
    let bytes = unsafe { core::slice::from_raw_parts(at as *const u8, MODULE_ENTRY_LEN) };
    // A module with no bytes is not an archive, and reporting it as one would have the kernel parse
    // an empty slice and report a corrupt filesystem rather than an absent one.
    module(bytes, 0).filter(|m| m.size > 0)
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
    self, ISA_IRQ_COUNT, IsaIrqRouting, MADT_PCAT_COMPAT, MadtEntry, Rsdp, SdtHeader, first_drhd,
    isa_irq_table, madt_entries, mcfg_entry, parse_dmar, parse_madt, parse_rsdp, parse_sdt_header,
    root_entry, root_entry_count,
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

/// The most `LocalApic` entries [`Acpi::cpus`] records (milestone 161's SMP item). Bounded well
/// past `cpu::MAX_CPUS` on purpose: a MADT can list more processor entries than this kernel can
/// seat (disabled sockets, a bigger machine than this build is sized for), and `smp::
/// seat_cpus_from_acpi` needs the true count to report "N described, only M startable" honestly,
/// the same distinction `read_cpu_list` draws on the device-tree architectures.
const MAX_ACPI_CPUS: usize = 32;

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
    /// **Every `LocalApic` entry the MADT listed**: `(local_apic_id, enabled)`, in the table's own
    /// order, past `cpu_count`. Fed to `smp::seat_cpus_from_acpi` (milestone 161's SMP item), the
    /// ACPI analog of `read_cpu_list`'s device-tree walk.
    pub cpus: [(u8, bool); MAX_ACPI_CPUS],
    /// How many entries of [`Acpi::cpus`] are real. May exceed `MAX_ACPI_CPUS` conceptually (the
    /// table can list more than that), in which case this saturates at `MAX_ACPI_CPUS` and
    /// `enabled_cpus`/`disabled_cpus` above are still the true totals.
    pub cpu_count: usize,
    /// True when the machine also has 8259 PICs, which must be masked before the APICs are used.
    pub has_8259: bool,
    /// The PCIe ECAM window, from the MCFG: base, first bus, last bus.
    pub ecam: Option<(u64, u8, u8)>,
    /// **VT-d's register base, from the DMAR's first DRHD** (milestone 161, roadmap item 6).
    /// `machine_discovery::acpi::first_drhd`'s own doc says why "first" rather than "every": one
    /// DRHD is what QEMU's `-device intel-iommu` presents, and this driver does not yet route a
    /// device to one of several.
    pub vtd_base: Option<u64>,
}

impl Default for Acpi {
    fn default() -> Self {
        Self {
            local_apic: None,
            io_apic: None,
            isa_irqs: core::array::from_fn(|irq| IsaIrqRouting::isa_default(irq as u8)),
            enabled_cpus: 0,
            disabled_cpus: 0,
            cpus: [(0, false); MAX_ACPI_CPUS],
            cpu_count: 0,
            has_8259: false,
            ecam: None,
            vtd_base: None,
        }
    }
}

/// Read `len` bytes at physical address `at` through the direct map.
///
/// # Safety
/// `at..at + len` must be inside the boot map's direct map (the low [`BOOT_DIRECT_MAP_LIMIT`],
/// which is what [`reachable`] checks) and must be memory rather than a device register block,
/// since this is an ordinary read.
unsafe fn phys_slice<'a>(at: u64, len: usize) -> &'a [u8] {
    // SAFETY: the caller's obligation, restated: an in-range physical address naming memory.
    unsafe { core::slice::from_raw_parts(phys_to_virt(at) as *const u8, len) }
}

/// **How far the boot page tables' direct map reaches: the low 4 GiB.**
///
/// `boot.s` builds 2048 page-directory entries of 2 MiB each, hangs them off a four-entry PDPT,
/// and points both PML4[0] (identity) and PML4[273] (the direct map `phys_to_virt` names) at that
/// same PDPT. Everything the ACPI walk reads happens before `mmu::init` installs the fine tables,
/// so this, and not the fine map, is the bound that applies.
///
/// **This constant was `0x4000_0000` (1 GiB) until 2026-09-02, and the 4x was not a rounding
/// error, it was a machine that could not boot.** The comment claimed the boot map "aliases only
/// the low gigabyte", which `boot.s` has not done since it was written: its own comment says 4 GiB
/// "because everything x86 talks to early is above 1 GiB and below 4". Nothing caught the
/// disagreement because both QEMU runners pass `-m 256M`, and firmware puts its tables just under
/// the top of RAM: at 256 MiB the RSDP lands at `0x0fb7e014` and passes a 1 GiB test. Booted with
/// `-m 2048` under OVMF it lands at `0x7fb7e014`, every table was refused as unreachable, and the
/// kernel came up with no MADT, no MCFG and no DMAR: no APIC, no timer, no PCI, no VT-d, on a
/// machine that had described all four. Every real x86 machine has more than 1 GiB of RAM, so this
/// was unconditional on hardware and invisible in the suite. `cargo xtask uefi-boot` now boots at
/// [`crate::arch::x86_64::machine`]'s witness size instead; see `uefi_boot`'s own gate.
const BOOT_DIRECT_MAP_LIMIT: u64 = 0x1_0000_0000;

/// Is the direct map able to reach this physical address? Anything above
/// [`BOOT_DIRECT_MAP_LIMIT`] would compute an address that is arithmetically right and not mapped,
/// which faults a long way from here. Checked rather than assumed because these addresses come
/// from firmware.
///
/// **A refusal here is not a table this kernel may ignore**, which is what the bug above cost: an
/// unreachable ACPI table is a machine whose APICs, PCIe window and IOMMU are all undiscoverable,
/// so `read_acpi` says so per table rather than skipping quietly. A machine that puts its tables
/// above 4 GiB (none seen; firmware keeps ACPI in low memory precisely so 32-bit loaders can read
/// it) needs the boot map widened, not this loosened.
fn reachable(at: u64, len: usize) -> bool {
    at != 0 && at.saturating_add(len as u64) <= BOOT_DIRECT_MAP_LIMIT
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
    // Out of the boot map's reach is said out loud, and separately from a bad checksum, because
    // the two ask for opposite things from whoever reads the line: a checksum failure is a table
    // to distrust, an unreachable address is a boot map to widen. Conflating them is what let the
    // 1 GiB bound above sit unnoticed.
    if !reachable(at, acpi::SDT_HEADER_LEN) {
        crate::println!(
            "                {at:#012x}  outside the boot map's low {} GiB, cannot be read",
            BOOT_DIRECT_MAP_LIMIT / (1024 * 1024 * 1024),
        );
        return None;
    }
    // SAFETY: `reachable` checked the header's range.
    let header = parse_sdt_header(unsafe { phys_slice(at, acpi::SDT_HEADER_LEN) }).ok()?;
    if !reachable(at, header.length as usize) {
        crate::println!(
            "                {at:#012x}  {} bytes long, which runs past the boot map",
            header.length,
        );
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
            b"DMAR" => read_dmar(body, &mut found),
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
            MadtEntry::LocalApic {
                apic_id, enabled, ..
            } => {
                if enabled {
                    into.enabled_cpus += 1;
                } else {
                    into.disabled_cpus += 1;
                }
                if into.cpu_count < MAX_ACPI_CPUS {
                    into.cpus[into.cpu_count] = (apic_id, enabled);
                    into.cpu_count += 1;
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

fn read_dmar(body: &[u8], into: &mut Acpi) {
    // The fixed part (host address width, interrupt-remapping flags) is decoded and not kept:
    // nothing here reads either yet, and `arch::x86_64::iommu::init` reads `CAP_REG` itself for
    // the one fact it needs (48-bit/4-level second-level translation support) rather than trusting
    // a value carried this far. Parsing it anyway is what proves the table is well-formed before
    // the structure list is trusted.
    if parse_dmar(body).is_err() {
        return;
    }
    into.vtd_base = first_drhd(body).map(|d| d.register_base);
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

/// What [`enable_pcie_ecam`] found, so the boot line can say which machine it is on rather than
/// claiming credit for work firmware had already done.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EcamDecode {
    /// The register already carried the MCFG's own base with the enable bit set, and nothing was
    /// written. Measured on both paths this kernel boots: OVMF programs it while bringing PCIe up,
    /// and QEMU's `q35` arrives at reset with it already set (see [`enable_pcie_ecam`]).
    AlreadyOn,
    /// The register was off, or named somewhere else, and was programmed from the MCFG.
    Programmed,
    /// The MCFG describes a bus count this chipset register cannot encode, so nothing was written
    /// and whatever decoded before still decodes. Loud rather than silent: a wrong length field
    /// redirects a range of physical addresses that belongs to something else.
    Unencodable,
}

/// The `PCIEXBAR` length field (bits 2:1) for a bus count, or `None` for a count the register
/// cannot express. Three sizes are defined and every Intel-compatible chipset this kernel targets
/// encodes them the same way: 256 MiB for 256 buses, 128 MiB for 128, 64 MiB for 64.
fn pciexbar_length_bits(bus_count: u32) -> Option<u32> {
    match bus_count {
        256 => Some(0b00),
        128 => Some(0b01),
        64 => Some(0b10),
        _ => None,
    }
}

/// **Turn the MCFG's ECAM window on, unless something already has.**
///
/// A real BIOS or UEFI programs the host bridge's `PCIEXBAR` register (base address, length,
/// enable bit) before it ever hands control to an OS, which is why a device-tree machine's port of
/// this kernel never has to think about this: firmware already turned the decode on, `virt`'s
/// `pci-host-ecam-generic` binding just states where. PVH is a hypervisor entry protocol rather
/// than firmware, so this port cannot assume the same and asks.
///
/// **It reads before it writes, and that is the difference between the two machines this has to
/// work on.** The version written on 2026-08-24 wrote unconditionally, reasoning that rewriting
/// the same base under real firmware "should be a no-op". It is a no-op only if the length field
/// agrees too, and firmware is free to choose 128 or 64 MiB where that wrote 256. On a machine
/// that had chosen a smaller window an unconditional write **widens the chipset's decode over
/// whatever physical addresses sit above it**, which on xenon (milestone 87's `OptiPlex`) would be
/// discovered at a null modem. Some chipsets also lock the register once firmware has written it,
/// in which case the write is dropped and the value read back is the only thing that would say so.
///
/// **And the complication that write was introduced for does not reproduce**, which is worth
/// stating precisely because the original measurement is quoted in milestone 165's block. That
/// block records QEMU's monitor answering "Cannot access memory" at the MCFG's base before the
/// kernel ran, read as the chipset's ECAM decode being off under PVH. Re-measured 2026-09-02 on
/// QEMU 11.1.1, from inside the guest, which is the side that matters: this register reads
/// `0xb0000001` on the PVH path **before anything writes it**, so the decode arrives on, and the
/// suite's PCI tests (the MCFG witness, NVMe, both userspace PCIe driver tests) all pass with this
/// function writing nothing at all. The monitor still answers "Cannot access memory" on the same
/// boot, so the monitor and the guest disagree and the guest is the one being served. Either QEMU
/// changed under the original measurement or the monitor was never evidence about the guest's
/// decode; nothing here needs to know which, because the register is asked rather than assumed.
///
/// The write stays for the machine that genuinely arrives with the decode off. It is therefore
/// **unexercised on both paths this kernel boots today**, which is recorded rather than hidden:
/// see this module's BUGS in notes/x86-port.md.
///
/// Uses the legacy configuration mechanism rather than the ECAM window itself, which is the only
/// way to bootstrap: nothing can read the ECAM window to turn the ECAM window on.
pub fn enable_pcie_ecam(base: u64, bus_count: u32) -> EcamDecode {
    // SAFETY: 0xcf8/0xcfc are the legacy PCI configuration ports, present on every PC-compatible
    // machine independent of ECAM, and reading the host bridge's own configuration space through
    // them is what firmware does before handing control to an OS.
    let current = unsafe {
        super::port::out32(CONFIG_ADDRESS, PCIEXBAR_CONFIG_ADDRESS);
        super::port::in32(CONFIG_DATA)
    };
    // Bit 0 is the enable bit, bits 2:1 are the length, and the rest is the base. This port has
    // never seen an MCFG base above 4 GiB, and truncating to 32 bits is the same assumption
    // `PCI_ECAM_PHYS` already makes.
    let want_base = base as u32 & !0xf;
    if current & 1 != 0 && current & !0xf == want_base {
        return EcamDecode::AlreadyOn;
    }
    let Some(length) = pciexbar_length_bits(bus_count) else {
        return EcamDecode::Unencodable;
    };
    // SAFETY: as above, and this is the register sequence firmware itself runs. The length comes
    // from the MCFG's own bus range rather than being assumed, which is what stops this widening a
    // window firmware sized deliberately.
    unsafe {
        super::port::out32(CONFIG_ADDRESS, PCIEXBAR_CONFIG_ADDRESS);
        super::port::out32(CONFIG_DATA, want_base | (length << 1) | 1);
    }
    EcamDecode::Programmed
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
    match found.vtd_base {
        Some(base) => crate::println!("                vt-d drhd at {base:#x}"),
        None => crate::println!("                no DMAR: no VT-d unit described"),
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

/// The most RAM regions handed to the allocator. `memory::bring_up_page_frames` indexes a fixed map of
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
/// the two slices `memory::bring_up_page_frames` takes, and nothing else.
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

    // Three reservations now, and all three would be catastrophic to miss. The kernel image is the
    // code running right now, plus the boot page tables the CPU is walking (they live in
    // `.boot_scratch`, which the linker script puts inside the image bounds precisely so that this
    // one entry covers them). The initrd is the archive every user program is read out of, sitting
    // in ordinary RAM the loader did not mark reserved, so without this entry the first process
    // built would very likely be built on top of its own ELF. The low megabyte is clipped above
    // rather than listed here.
    //
    // **The count is what the array is sliced to**, rather than the array being sized to the worst
    // case and passed whole: an all-zero `Region` is a reservation of nothing at address zero, and
    // `bring_up_page_frames` would dutifully take it.
    let mut forbidden = [dtb::Region { start: 0, size: 0 }; 2];
    forbidden[0] = dtb::Region {
        start: crate::memory::image_start(),
        size: crate::memory::image_end() - crate::memory::image_start(),
    };
    let mut forbidden_count = 1;
    if let Some(m) = initrd(info) {
        forbidden[forbidden_count] = dtb::Region {
            start: m.addr,
            size: m.size,
        };
        forbidden_count += 1;
        crate::memory::record_initrd(m.addr, m.size);
    }

    crate::memory::bring_up_page_frames(&ram[..count], &forbidden[..forbidden_count]);
    count
}
