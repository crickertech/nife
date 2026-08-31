//! **Building the handoff the kernel already knows how to read.**
//!
//! This is the half of milestone 87 that decided the shape of the rest of it. The kernel's `x86_64`
//! boot handoff is PVH's `hvm_start_info`: one physical pointer to a structure carrying the memory
//! map, the ACPI root pointer, and any loaded modules. `machine_discovery::x86_64` decodes it and
//! `arch::x86_64::machine` consumes it, both host-tested, both already working.
//!
//! So this loader does not invent a UEFI handoff. **It synthesises an `hvm_start_info`**, and the
//! kernel cannot tell which loader started it. That is not a shortcut, it is the answer to the
//! hazard milestone 87 was briefed on: two boot paths that produce two different internal states
//! diverge, and the divergence shows up on hardware nobody can attach a debugger to. There is one
//! structure, one decoder, and one set of tests.
//!
//! # This module is the *writer* for `machine_discovery::x86_64`'s *reader*
//!
//! And the tests below say so literally: every encoder here is checked by decoding its output with
//! that crate. Neither side carries its own copy of the layout, which is the same rule
//! `byte_sink_proto`, `grant_plan` and `clock_proto` are held to.
//!
//! # BUGS
//!
//! - **Firmware-owned memory is reported as reserved rather than reclaimed.** UEFI's
//!   `EfiBootServicesCode` and `EfiBootServicesData` become free RAM the moment `ExitBootServices`
//!   returns, and Linux reclaims them. [`e820_kind`] does not: it reports them, and the loader's
//!   own `EfiLoaderCode`/`EfiLoaderData`, as [`E820_RESERVED`]. The reason is that this loader's
//!   handoff structure, memory map, module list and mode-switch trampoline are all in
//!   `EfiLoaderData` and are read *by the kernel* after the frame allocator comes up, so
//!   reclaiming that type would let the allocator hand out the memory map it is reading. Splitting
//!   the loader's own allocations back out (they are known, and there are four of them) would
//!   recover the rest, and is the fix if the lost RAM ever matters. It is measured rather than
//!   guessed: see notes/x86-uefi-boot.md.
//! - **The map is passed through in firmware order and is neither sorted nor coalesced.** UEFI
//!   promises neither, and `machine_discovery::x86_64`'s own `BUGS` already records that its
//!   consumer is where such a check belongs.

/// `hvm_start_info`, version 1: the size this loader always writes.
pub const START_INFO_LEN: usize = 56;

/// One `hvm_memmap_table_entry`.
pub const MEMMAP_ENTRY_LEN: usize = 24;

/// One `hvm_modlist_entry`.
pub const MODULE_ENTRY_LEN: usize = 32;

/// The magic at offset 0, and the value the kernel is handed in `eax`. "xEn3" little-endian.
pub const MAGIC: u32 = 0x336e_c578;

/// The structure version this loader writes. **1, not 0**, because version 0 stops before the
/// memory-map fields and a version-0 handoff means "this kernel cannot find its RAM here".
pub const VERSION: u32 = 1;

/// E820 type 1: ordinary RAM, the only kind a frame allocator may hand out.
pub const E820_RAM: u32 = 1;
/// E820 type 2: somebody else owns it.
pub const E820_RESERVED: u32 = 2;
/// E820 type 3: ACPI tables, reclaimable after they have been read.
pub const E820_ACPI_RECLAIMABLE: u32 = 3;
/// E820 type 4: ACPI non-volatile storage.
pub const E820_ACPI_NVS: u32 = 4;
/// E820 type 5: RAM the firmware found faulty.
pub const E820_UNUSABLE: u32 = 5;
/// E820 type 7: byte-addressable persistent memory.
pub const E820_PERSISTENT: u32 = 7;

/// **Translate one UEFI memory type into the E820 type PVH's map carries.**
///
/// The direction of every judgement call here is the same one
/// `machine_discovery::x86_64::MemoryKind::from_raw` documents for unknown types: **claiming less
/// RAM than exists costs a few megabytes, and claiming more corrupts something.** A UEFI type this
/// function has never heard of is reserved, not RAM.
pub const fn e820_kind(efi_type: u32) -> u32 {
    use crate::efi::memory_type as t;
    match efi_type {
        t::CONVENTIONAL => E820_RAM,
        t::ACPI_RECLAIM => E820_ACPI_RECLAIMABLE,
        t::ACPI_NVS => E820_ACPI_NVS,
        t::UNUSABLE => E820_UNUSABLE,
        t::PERSISTENT => E820_PERSISTENT,
        // Everything else, boot-services memory and this loader's own allocations included. See
        // this module's BUGS section for what that costs and how to get it back.
        _ => E820_RESERVED,
    }
}

/// The fields of `hvm_start_info` this loader fills in. Everything else is zero.
#[derive(Clone, Copy, Default)]
pub struct StartInfo {
    /// **The ACPI RSDP**, taken from the UEFI configuration table.
    ///
    /// This is the field that makes a UEFI boot different from a PVH one in a way the kernel can
    /// see. QEMU's PVH loader leaves it zero, so `arch::x86_64::machine::find_rsdp` falls back to
    /// scanning the BIOS area for `"RSD PTR "`; real firmware hands it over, so the non-zero path
    /// runs. Milestone 87's brief called that path out by name: it had never executed.
    pub rsdp: u64,
    /// Where the module list is, or 0 when there is no initrd.
    pub modules: u64,
    /// How many modules. 0 or 1 here.
    pub module_count: u32,
    /// Where the memory map is.
    pub memmap: u64,
    /// How many entries it has.
    pub memmap_entries: u32,
}

impl StartInfo {
    /// Lay the structure out, little-endian, exactly as Xen's `start_info.h` specifies it.
    pub fn encode(&self) -> [u8; START_INFO_LEN] {
        let mut out = [0u8; START_INFO_LEN];
        put_u32(&mut out, 0, MAGIC);
        put_u32(&mut out, 4, VERSION);
        // offset 8: flags. Xen defines SecureBoot (bit 0) and SecureBootEnabled (bit 1); this
        // loader does not report them, because it does not verify anything and a flag claiming
        // otherwise would be a lie the kernel has no way to check.
        put_u32(&mut out, 12, self.module_count);
        put_u64(&mut out, 16, self.modules);
        // offset 24: cmdline. Nothing here has a command line yet.
        put_u64(&mut out, 32, self.rsdp);
        put_u64(&mut out, 40, self.memmap);
        put_u32(&mut out, 48, self.memmap_entries);
        out
    }
}

/// Lay out one `hvm_memmap_table_entry`.
pub fn encode_memmap_entry(addr: u64, size: u64, kind: u32) -> [u8; MEMMAP_ENTRY_LEN] {
    let mut out = [0u8; MEMMAP_ENTRY_LEN];
    put_u64(&mut out, 0, addr);
    put_u64(&mut out, 8, size);
    put_u32(&mut out, 16, kind);
    out
}

/// Lay out one `hvm_modlist_entry`.
pub fn encode_module(addr: u64, size: u64) -> [u8; MODULE_ENTRY_LEN] {
    let mut out = [0u8; MODULE_ENTRY_LEN];
    put_u64(&mut out, 0, addr);
    put_u64(&mut out, 8, size);
    // offset 16: this module's own command line, and offset 24 is reserved. Neither is used.
    out
}

fn put_u32(bytes: &mut [u8], at: usize, value: u32) {
    bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
}

fn put_u64(bytes: &mut [u8], at: usize, value: u64) {
    bytes[at..at + 8].copy_from_slice(&value.to_le_bytes());
}

#[cfg(test)]
mod tests {
    use machine_discovery::x86_64 as pvh;

    use super::*;

    /// The point of every test in this file: what this loader **writes** is decoded by the crate
    /// the kernel **reads** with, so the two cannot drift apart without a host test failing in
    /// milliseconds.
    #[test]
    fn what_the_loader_writes_is_what_the_kernel_reads() {
        let info = StartInfo {
            rsdp: 0x7fee_0014,
            modules: 0x0100_0000,
            module_count: 1,
            memmap: 0x0100_1000,
            memmap_entries: 42,
        };
        let decoded =
            pvh::BootInfo::parse(&info.encode()).expect("the kernel's decoder accepts it");

        assert_eq!(
            decoded.version, 1,
            "version 0 has no memory-map fields at all"
        );
        assert_eq!(decoded.rsdp, 0x7fee_0014);
        assert_eq!(decoded.modules, 0x0100_0000);
        assert_eq!(decoded.module_count, 1);
        assert_eq!(decoded.memmap, 0x0100_1000);
        assert_eq!(decoded.memmap_entries, 42);
    }

    /// The two crates agree on the structure's length, which is the fact that would silently break
    /// if either side gained a field.
    #[test]
    fn the_two_sides_agree_on_the_entry_sizes() {
        assert_eq!(MEMMAP_ENTRY_LEN, pvh::MEMMAP_ENTRY_LEN);
        assert_eq!(MODULE_ENTRY_LEN, pvh::MODULE_ENTRY_LEN);
        assert_eq!(MAGIC, pvh::MAGIC);
    }

    #[test]
    fn a_memory_map_entry_round_trips_through_the_kernels_decoder() {
        let bytes = encode_memmap_entry(0x10_0000, 0x0fe0_0000, E820_RAM);
        let entry = pvh::memory_entry(&bytes, 0).expect("one entry, in bounds");
        assert_eq!(entry.addr, 0x10_0000);
        assert_eq!(entry.size, 0x0fe0_0000);
        assert_eq!(entry.kind, pvh::MemoryKind::Ram);
        assert!(entry.is_usable_ram());
    }

    /// Indexing works across a *pair* of entries, which is what catches a stride mistake that a
    /// one-entry map cannot: the same trap `machine_discovery`'s own module tests call out.
    #[test]
    fn a_two_entry_map_strides_correctly() {
        let mut bytes = [0u8; MEMMAP_ENTRY_LEN * 2];
        bytes[..MEMMAP_ENTRY_LEN].copy_from_slice(&encode_memmap_entry(0, 0x1000, E820_RESERVED));
        bytes[MEMMAP_ENTRY_LEN..]
            .copy_from_slice(&encode_memmap_entry(0x10_0000, 0x2000, E820_RAM));

        assert_eq!(pvh::memory_entry(&bytes, 0).unwrap().addr, 0);
        assert_eq!(pvh::memory_entry(&bytes, 1).unwrap().addr, 0x10_0000);
        assert!(!pvh::memory_entry(&bytes, 0).unwrap().is_usable_ram());
        assert!(pvh::memory_entry(&bytes, 1).unwrap().is_usable_ram());
        assert!(
            pvh::memory_entry(&bytes, 2).is_none(),
            "past the end is None"
        );
    }

    #[test]
    fn a_module_entry_round_trips_through_the_kernels_decoder() {
        let bytes = encode_module(0x0200_0000, 4_400_000);
        let module = pvh::module(&bytes, 0).expect("one module, in bounds");
        assert_eq!(module.addr, 0x0200_0000);
        assert_eq!(module.size, 4_400_000);
    }

    /// The classification, stated as a table so a reader can check it against the UEFI
    /// specification without reading the `match`.
    #[test]
    fn every_uefi_memory_type_lands_where_it_should() {
        use crate::efi::memory_type as t;
        assert_eq!(e820_kind(t::CONVENTIONAL), E820_RAM);
        assert_eq!(e820_kind(t::ACPI_RECLAIM), E820_ACPI_RECLAIMABLE);
        assert_eq!(e820_kind(t::ACPI_NVS), E820_ACPI_NVS);
        assert_eq!(e820_kind(t::UNUSABLE), E820_UNUSABLE);
        assert_eq!(e820_kind(t::PERSISTENT), E820_PERSISTENT);

        for reserved in [
            t::RESERVED,
            t::LOADER_CODE,
            t::LOADER_DATA,
            t::BOOT_SERVICES_CODE,
            t::BOOT_SERVICES_DATA,
            t::RUNTIME_SERVICES_CODE,
            t::RUNTIME_SERVICES_DATA,
            t::MMIO,
            t::MMIO_PORT_SPACE,
            t::PAL_CODE,
        ] {
            assert_eq!(e820_kind(reserved), E820_RESERVED, "type {reserved}");
        }
    }

    /// The direction of the unknown-type default, asserted rather than left to the reader: a type
    /// UEFI adds after this was written must not become RAM by accident.
    #[test]
    fn a_memory_type_this_loader_has_never_heard_of_is_not_ram() {
        assert_eq!(e820_kind(0xdead_beef), E820_RESERVED);
        assert_eq!(e820_kind(u32::MAX), E820_RESERVED);
    }
}
