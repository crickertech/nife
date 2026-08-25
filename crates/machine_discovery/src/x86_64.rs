//! **What the machine told us on the way in, `x86_64`**: the PVH `hvm_start_info` structure.
//!
//! Milestone 161. This is the third answer to the question the other two modules in this crate
//! answer, and it is a *fourth* tier the crate's header does not list, because x86 has all three of
//! the others and needs a different one for this particular fact.
//!
//! The other two architectures learn where RAM is from the device tree, which the firmware hands
//! them as a single pointer. x86 has no device tree. What it has is a boot protocol, and the
//! protocol this kernel uses (PVH, see `kernel/src/arch/x86_64/boot.s` for why not multiboot) hands
//! over one pointer to a structure carrying the memory map, the ACPI root pointer, the command line
//! and any loaded modules. So the *shape* is identical to the device-tree handoff, one pointer to
//! everything, and only the format differs.
//!
//! # Why the decoding is here and not in `arch/x86_64/`
//!
//! Because it is a parser, and a parser proved only inside a booting kernel is a parser proved by
//! nothing that runs in milliseconds. Same reason `crates/dtb` exists rather than a device-tree
//! reader living in `arch/aarch64/`: this file compiles for the host, its tests run without an
//! emulator, and the kernel side is reduced to reading a pointer through the direct map.
//!
//! # The structure, from Xen's `start_info.h`
//!
//! ```text
//! struct hvm_start_info {          offset  size
//!     uint32_t magic;                   0     4   0x336ec578, "xEn3" little-endian
//!     uint32_t version;                 4     4
//!     uint32_t flags;                   8     4
//!     uint32_t nr_modules;             12     4
//!     uint64_t modlist_paddr;          16     8
//!     uint64_t cmdline_paddr;          24     8
//!     uint64_t rsdp_paddr;             32     8   the ACPI root pointer
//!     /* version >= 1 only: */
//!     uint64_t memmap_paddr;           40     8
//!     uint32_t memmap_entries;         48     4
//!     uint32_t reserved;               52     4
//! };
//! ```
//!
//! Version 0 stops at offset 40, which is why [`BootInfo::parse`] reads the memory-map fields only
//! when the version says they are there and reports zero entries otherwise. A caller that finds no
//! entries has to fall back to the legacy E820 call, which this kernel cannot make (it needs real
//! mode), so in practice a version-0 handoff means "this kernel cannot find its RAM here"; the
//! caller decides what to do about it and this module only reports what is true.
//!
//! # BUGS
//!
//! - **Nothing checks that the map is sorted or non-overlapping.** The specification does not
//!   promise either, and every implementation this has been run against produces a sorted map. A
//!   frame allocator built over an unsorted or overlapping map would double-count, so the consumer
//!   is where that check belongs.

/// The magic word at offset 0 of `hvm_start_info`, and the same value the boot CPU is handed in
/// `eax`. "xEn3" read little-endian.
pub const MAGIC: u32 = 0x336e_c578;

/// The size of the version-0 structure, which is where the memory-map fields begin.
const V0_LEN: usize = 40;
/// The size of the version-1 structure.
const V1_LEN: usize = 56;

/// The size of one `hvm_memmap_table_entry`.
pub const MEMMAP_ENTRY_LEN: usize = 24;

/// Why a `hvm_start_info` could not be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootInfoError {
    /// The magic word is not [`MAGIC`]. Either the pointer is wrong or the loader is not speaking
    /// PVH, and in both cases nothing after offset 4 means anything.
    BadMagic(u32),
    /// The bytes end before the structure does.
    Truncated,
}

/// What the loader said, decoded.
///
/// Every address in here is **physical**, because that is what a loader running with paging off can
/// mean. The kernel reaches them through its direct map.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BootInfo {
    /// The structure version. 0 has no memory map; 1 and later do.
    pub version: u32,
    /// Loader-defined flags. Xen defines bit 0 (`SecureBoot`) and bit 1 (`SecureBootEnabled`); QEMU
    /// sets neither.
    pub flags: u32,
    /// How many modules (initrd images) were loaded.
    pub module_count: u32,
    /// Where the module list is, or 0.
    pub modules: u64,
    /// Where the NUL-terminated command line is, or 0.
    pub cmdline: u64,
    /// **The ACPI RSDP**, the root of every table x86 uses in place of a device tree: the MADT (the
    /// APICs), the MCFG (the PCIe ECAM window) and the DMAR (the IOMMU) are all reached from here.
    /// 0 when the loader did not say, which on a machine with ACPI means it must be found by
    /// scanning the BIOS area instead.
    pub rsdp: u64,
    /// Where the memory map is, or 0 on a version-0 handoff.
    pub memmap: u64,
    /// How many entries the memory map has. Zero on a version-0 handoff.
    pub memmap_entries: u32,
}

impl BootInfo {
    /// Decode `bytes`, which must begin at the structure.
    ///
    /// The magic is checked first and everything else is refused until it passes, which is the same
    /// discipline `dtb::Dtb::from_ptr` follows and for the same reason: this is the first thing the
    /// kernel does with a pointer somebody else chose, so a wrong pointer must produce an error
    /// rather than a plausible-looking memory map.
    pub fn parse(bytes: &[u8]) -> Result<Self, BootInfoError> {
        if bytes.len() < V0_LEN {
            return Err(BootInfoError::Truncated);
        }
        let magic = u32(bytes, 0);
        if magic != MAGIC {
            return Err(BootInfoError::BadMagic(magic));
        }
        let version = u32(bytes, 4);

        // The memory-map fields exist only from version 1. Reading them from a version-0 structure
        // would be reading whatever the loader left after its own last field, which is exactly the
        // kind of plausible garbage this crate exists to keep out of the kernel.
        let (memmap, memmap_entries) = if version >= 1 {
            if bytes.len() < V1_LEN {
                return Err(BootInfoError::Truncated);
            }
            (u64(bytes, 40), u32(bytes, 48))
        } else {
            (0, 0)
        };

        Ok(BootInfo {
            version,
            flags: u32(bytes, 8),
            module_count: u32(bytes, 12),
            modules: u64(bytes, 16),
            cmdline: u64(bytes, 24),
            rsdp: u64(bytes, 32),
            memmap,
            memmap_entries,
        })
    }
}

/// What a memory-map entry says the range is for. The numbers are E820's, which PVH reuses rather
/// than inventing its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryKind {
    /// Ordinary RAM. **The only kind a frame allocator may hand out.**
    Ram,
    /// Firmware or hardware owns it. Never allocatable.
    Reserved,
    /// ACPI tables live here. Not allocatable until they have been read, and this kernel has not
    /// read them, so it is not allocatable at all yet.
    AcpiReclaimable,
    /// ACPI non-volatile storage. Never allocatable.
    AcpiNvs,
    /// RAM the firmware found faulty.
    Unusable,
    /// Present but disabled.
    Disabled,
    /// Persistent memory.
    Persistent,
    /// A type this decoder does not know. **Treated as not-RAM by [`MemoryEntry::is_usable_ram`]**,
    /// which is the safe direction: a new E820 type has never meant "more RAM than you thought".
    Unknown(u32),
}

impl MemoryKind {
    /// Decode the 32-bit type word.
    pub const fn from_raw(raw: u32) -> Self {
        match raw {
            1 => MemoryKind::Ram,
            2 => MemoryKind::Reserved,
            3 => MemoryKind::AcpiReclaimable,
            4 => MemoryKind::AcpiNvs,
            5 => MemoryKind::Unusable,
            6 => MemoryKind::Disabled,
            7 => MemoryKind::Persistent,
            other => MemoryKind::Unknown(other),
        }
    }

    /// A short word for the boot print.
    pub const fn name(self) -> &'static str {
        match self {
            MemoryKind::Ram => "ram",
            MemoryKind::Reserved => "reserved",
            MemoryKind::AcpiReclaimable => "acpi",
            MemoryKind::AcpiNvs => "acpi-nvs",
            MemoryKind::Unusable => "unusable",
            MemoryKind::Disabled => "disabled",
            MemoryKind::Persistent => "pmem",
            MemoryKind::Unknown(_) => "unknown",
        }
    }
}

/// One range of physical memory and what it is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryEntry {
    pub addr: u64,
    pub size: u64,
    pub kind: MemoryKind,
}

impl MemoryEntry {
    /// May a frame allocator hand this range out?
    ///
    /// Only [`MemoryKind::Ram`], and deliberately **not** [`MemoryKind::AcpiReclaimable`] even
    /// though the name promises it can be reclaimed. It can be, *after* the tables in it have been
    /// read, and this kernel has not read them; reclaiming it now would hand out the MADT that the
    /// APIC bring-up is going to need.
    pub const fn is_usable_ram(&self) -> bool {
        matches!(self.kind, MemoryKind::Ram)
    }

    /// One past the last byte, saturating rather than wrapping so a malformed entry claiming a size
    /// near `u64::MAX` cannot make a range look empty.
    pub const fn end(&self) -> u64 {
        self.addr.saturating_add(self.size)
    }
}

/// Decode memory-map entry `index` out of `bytes`, which must begin at the map.
///
/// Indexed rather than iterated because the kernel side reads the map through its direct map at a
/// physical address, one entry at a time, and never has a slice of the whole thing. `None` when the
/// bytes end before that entry does.
pub fn memory_entry(bytes: &[u8], index: usize) -> Option<MemoryEntry> {
    let at = index.checked_mul(MEMMAP_ENTRY_LEN)?;
    if bytes.len() < at.checked_add(MEMMAP_ENTRY_LEN)? {
        return None;
    }
    Some(MemoryEntry {
        addr: u64(bytes, at),
        size: u64(bytes, at + 8),
        kind: MemoryKind::from_raw(u32(bytes, at + 16)),
    })
}

/// The size of one `hvm_modlist_entry`.
pub const MODULE_ENTRY_LEN: usize = 32;

/// **A module the loader put in RAM for the kernel**, which on this system means the initrd.
///
/// This is x86's answer to `/chosen/linux,initrd-start`, and the analogy is close enough to be
/// worth stating: on both other architectures the loader writes the archive's bounds into the
/// device tree and `memory::init` reads them out; here it writes them into a list of these, and
/// [`BootInfo::modules`] says where the list is. The only real difference is that a module carries
/// a command line of its own, which nothing here uses.
///
/// PVH allows several. QEMU's loader produces exactly one per `-initrd`, and one is what this
/// kernel wants; a caller taking the first is not making an assumption so much as declining to
/// invent a policy for a case the machine does not produce.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Module {
    /// Where the module's bytes are, physically.
    pub addr: u64,
    /// How many bytes there are.
    pub size: u64,
    /// Where this module's own NUL-terminated command line is, or 0. Unused here; decoded because
    /// leaving a field out of a decoder is how a reader comes to believe the structure is smaller
    /// than it is.
    pub cmdline: u64,
}

impl Module {
    /// One past the last byte, saturating for [`MemoryEntry::end`]'s reason: a malformed size near
    /// `u64::MAX` must not make the range look empty to a caller that reserves it.
    pub const fn end(&self) -> u64 {
        self.addr.saturating_add(self.size)
    }
}

/// Decode module `index` out of `bytes`, which must begin at the module list.
///
/// Indexed rather than iterated for [`memory_entry`]'s reason: the kernel reads the list through
/// its direct map, one entry at a time, and never holds a slice of the whole thing. `None` when the
/// bytes end before that entry does.
pub fn module(bytes: &[u8], index: usize) -> Option<Module> {
    let at = index.checked_mul(MODULE_ENTRY_LEN)?;
    if bytes.len() < at.checked_add(MODULE_ENTRY_LEN)? {
        return None;
    }
    Some(Module {
        addr: u64(bytes, at),
        size: u64(bytes, at + 8),
        cmdline: u64(bytes, at + 16),
    })
}

fn u32(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
}

fn u64(bytes: &[u8], at: usize) -> u64 {
    let mut w = [0u8; 8];
    w.copy_from_slice(&bytes[at..at + 8]);
    u64::from_le_bytes(w)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A version-1 structure with the values QEMU's `q35` actually produced on 2026-08-23, read
    /// back out of the guest. Built by hand rather than captured as a blob so a reader can see
    /// which field is which.
    fn qemu_q35() -> [u8; V1_LEN] {
        let mut b = [0u8; V1_LEN];
        b[0..4].copy_from_slice(&MAGIC.to_le_bytes());
        b[4..8].copy_from_slice(&1u32.to_le_bytes()); // version
        b[8..12].copy_from_slice(&0u32.to_le_bytes()); // flags
        b[12..16].copy_from_slice(&0u32.to_le_bytes()); // nr_modules
        b[16..24].copy_from_slice(&0u64.to_le_bytes()); // modlist
        b[24..32].copy_from_slice(&0u64.to_le_bytes()); // cmdline
        b[32..40].copy_from_slice(&0x000f_5a30u64.to_le_bytes()); // rsdp, in the BIOS area
        b[40..48].copy_from_slice(&0x0000_15b0u64.to_le_bytes()); // memmap
        b[48..52].copy_from_slice(&4u32.to_le_bytes()); // memmap_entries
        b
    }

    /// **The magic is checked before anything else is believed.** This is the first thing the
    /// kernel does with a pointer the loader chose, so a wrong pointer has to be an error rather
    /// than a memory map that looks reasonable.
    #[test]
    fn a_wrong_magic_is_refused_and_reports_what_it_saw() {
        let mut b = qemu_q35();
        b[0..4].copy_from_slice(&0xdead_beefu32.to_le_bytes());
        assert_eq!(
            BootInfo::parse(&b),
            Err(BootInfoError::BadMagic(0xdead_beef))
        );
    }

    /// Bytes that end inside the structure are refused rather than read past.
    #[test]
    fn a_truncated_structure_is_refused() {
        let b = qemu_q35();
        assert_eq!(BootInfo::parse(&b[..20]), Err(BootInfoError::Truncated));
        // Long enough for version 0, but the version says 1, so the memory-map fields must be
        // there and are not.
        assert_eq!(BootInfo::parse(&b[..V0_LEN]), Err(BootInfoError::Truncated));
    }

    /// **A version-0 handoff reports no memory map rather than reading past its own end.** The
    /// fields simply do not exist there, and inventing them from whatever follows is the failure
    /// this crate is written to prevent.
    #[test]
    fn a_version_0_structure_reports_no_memory_map() {
        let mut b = qemu_q35();
        b[4..8].copy_from_slice(&0u32.to_le_bytes());
        // Leave plausible garbage where the memory-map fields would be.
        b[40..48].copy_from_slice(&0xdead_beef_dead_beefu64.to_le_bytes());
        b[48..52].copy_from_slice(&99u32.to_le_bytes());
        let info = BootInfo::parse(&b).expect("version 0 is still a valid structure");
        assert_eq!(info.version, 0);
        assert_eq!(info.memmap, 0);
        assert_eq!(info.memmap_entries, 0);
        // The fields that DO exist in version 0 are still read.
        assert_eq!(info.rsdp, 0x000f_5a30);
    }

    /// Every field lands where the structure says it does.
    #[test]
    fn the_fields_are_read_from_the_offsets_the_specification_gives() {
        let info = BootInfo::parse(&qemu_q35()).expect("well-formed");
        assert_eq!(info.version, 1);
        assert_eq!(info.flags, 0);
        assert_eq!(info.module_count, 0);
        assert_eq!(info.rsdp, 0x000f_5a30);
        assert_eq!(info.memmap, 0x0000_15b0);
        assert_eq!(info.memmap_entries, 4);
    }

    /// Lay `entries` out as a memory map. A fixed-size buffer rather than a `Vec` because this
    /// crate is `no_std` in its test build too, which is the same constraint the kernel side has.
    fn entry_bytes<const N: usize>(entries: &[(u64, u64, u32)]) -> [u8; N] {
        assert_eq!(N, entries.len() * MEMMAP_ENTRY_LEN);
        let mut b = [0u8; N];
        for (i, (addr, size, kind)) in entries.iter().enumerate() {
            let at = i * MEMMAP_ENTRY_LEN;
            b[at..at + 8].copy_from_slice(&addr.to_le_bytes());
            b[at + 8..at + 16].copy_from_slice(&size.to_le_bytes());
            b[at + 16..at + 20].copy_from_slice(&kind.to_le_bytes());
        }
        b
    }

    /// The map q35 produces with `-m 256M`: low RAM, the BIOS hole, high RAM, and the reserved
    /// block below 4 GiB.
    #[test]
    fn a_real_memory_map_decodes_entry_by_entry() {
        let b = entry_bytes::<{ 4 * MEMMAP_ENTRY_LEN }>(&[
            (0x0000_0000, 0x0009_fc00, 1),
            (0x0009_fc00, 0x0000_0400, 2),
            (0x0010_0000, 0x0ff0_0000, 1),
            (0xfffc_0000, 0x0004_0000, 2),
        ]);
        assert_eq!(
            memory_entry(&b, 0),
            Some(MemoryEntry {
                addr: 0,
                size: 0x0009_fc00,
                kind: MemoryKind::Ram
            })
        );
        assert_eq!(memory_entry(&b, 1).unwrap().kind, MemoryKind::Reserved);
        assert_eq!(memory_entry(&b, 2).unwrap().end(), 0x1000_0000);
        assert_eq!(memory_entry(&b, 3).unwrap().addr, 0xfffc_0000);
        assert_eq!(
            memory_entry(&b, 4),
            None,
            "past the end is None, not garbage"
        );
    }

    /// **Only type 1 is allocatable.** `AcpiReclaimable` is the trap: the name says it can be
    /// reclaimed and it cannot be until the tables in it have been read, which this kernel has not
    /// done. Handing it out would hand out the MADT the APIC bring-up needs.
    #[test]
    fn only_plain_ram_is_allocatable_and_acpi_memory_is_not() {
        let b = entry_bytes::<{ 5 * MEMMAP_ENTRY_LEN }>(&[
            (0x1000, 0x1000, 1),
            (0x2000, 0x1000, 3),
            (0x3000, 0x1000, 4),
            (0x4000, 0x1000, 5),
            (0x5000, 0x1000, 4242),
        ]);
        assert!(memory_entry(&b, 0).unwrap().is_usable_ram());
        assert!(
            !memory_entry(&b, 1).unwrap().is_usable_ram(),
            "acpi-reclaimable holds the tables the APIC bring-up has not read yet"
        );
        assert!(!memory_entry(&b, 2).unwrap().is_usable_ram());
        assert!(!memory_entry(&b, 3).unwrap().is_usable_ram());
        assert_eq!(memory_entry(&b, 4).unwrap().kind, MemoryKind::Unknown(4242));
        assert!(
            !memory_entry(&b, 4).unwrap().is_usable_ram(),
            "an unknown type has never meant more RAM than you thought"
        );
    }

    /// A size near `u64::MAX` saturates rather than wrapping, so a malformed entry cannot make a
    /// huge range look like an empty one.
    #[test]
    fn an_absurd_size_saturates_rather_than_wrapping() {
        let b = entry_bytes::<MEMMAP_ENTRY_LEN>(&[(0x1000, u64::MAX, 1)]);
        assert_eq!(memory_entry(&b, 0).unwrap().end(), u64::MAX);
    }

    /// Build a module list of `entries`, each `(paddr, size, cmdline_paddr)`.
    fn module_bytes<const N: usize>(entries: &[(u64, u64, u64)]) -> [u8; N] {
        let mut b = [0u8; N];
        for (i, &(addr, size, cmdline)) in entries.iter().enumerate() {
            let at = i * MODULE_ENTRY_LEN;
            b[at..at + 8].copy_from_slice(&addr.to_le_bytes());
            b[at + 8..at + 16].copy_from_slice(&size.to_le_bytes());
            b[at + 16..at + 24].copy_from_slice(&cmdline.to_le_bytes());
            // The last eight bytes are `reserved` and stay zero, which is what makes an entry 32
            // bytes rather than 24. A decoder that stepped by 24 would read the second module's
            // address out of the first one's tail.
        }
        b
    }

    /// **A module list decodes entry by entry, at the 32-byte stride the structure gives.** The
    /// second entry is what makes this a test rather than a field read: the reserved word at the
    /// end of the first is invisible in a one-module list, and QEMU only ever produces one.
    #[test]
    fn a_module_list_decodes_entry_by_entry() {
        let b = module_bytes::<{ 2 * MODULE_ENTRY_LEN }>(&[
            (0x0100_0000, 0x0002_2000, 0),
            (0x0200_0000, 0x1000, 0x0f00),
        ]);
        assert_eq!(
            module(&b, 0),
            Some(Module {
                addr: 0x0100_0000,
                size: 0x0002_2000,
                cmdline: 0,
            })
        );
        assert_eq!(
            module(&b, 1),
            Some(Module {
                addr: 0x0200_0000,
                size: 0x1000,
                cmdline: 0x0f00,
            })
        );
        assert_eq!(module(&b, 2), None, "past the end is None, not garbage");
    }

    /// Bytes ending inside an entry are refused rather than read past, exactly as a truncated
    /// memory map is.
    #[test]
    fn a_truncated_module_entry_is_refused() {
        let b = module_bytes::<MODULE_ENTRY_LEN>(&[(0x0100_0000, 0x1000, 0)]);
        assert_eq!(module(&b[..MODULE_ENTRY_LEN - 1], 0), None);
    }

    /// A module claiming a size near `u64::MAX` saturates, for the memory map's reason: the kernel
    /// reserves `addr..end()` so the allocator cannot hand out the archive it is about to read, and
    /// a wrapped `end` would reserve nothing at all.
    #[test]
    fn an_absurd_module_size_saturates() {
        let b = module_bytes::<MODULE_ENTRY_LEN>(&[(0x1000, u64::MAX, 0)]);
        assert_eq!(module(&b, 0).unwrap().end(), u64::MAX);
    }
}
