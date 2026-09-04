//! **The slice of UEFI this loader speaks**, hand-written rather than taken as a dependency.
//!
//! # Why there is no `uefi` crate here
//!
//! DECISIONS §46: a dependency is a decision, and the tree's shape is thin architectural
//! primitives or whole subsystems nobody would write, with nothing in between. The `uefi` crate is
//! squarely in between. What this loader needs is **six function pointers and two GUIDs**: allocate
//! pages, get the memory map, exit boot services, print a line, and the two ACPI configuration-table
//! identifiers. That is the whole of it, and it is written below in about two hundred lines that a
//! reader can check against the specification without leaving the repository.
//!
//! The cost of getting it wrong is also unusually visible: a mis-numbered field in
//! [`BootServices`] is a call to the wrong function pointer, which faults immediately and loudly at
//! the very start of boot, rather than being the sort of subtle wrongness a dependency protects
//! against.
//!
//! # The one rule to remember when editing this file
//!
//! **[`BootServices`]'s field order is the ABI.** The firmware hands over a pointer to a table it
//! filled in; every entry is found by its offset and nothing checks the name. Adding a field in the
//! middle, or getting the count of unused ones wrong, silently re-points every call after it. The
//! fields are therefore listed in specification order, in full, with the unused ones spelled
//! `usize` and *counted* in comments rather than collapsed into an array.
//!
//! # BUGS
//!
//! - **The struct definitions stop where this loader's needs stop.** [`SystemTable`] ends after
//!   `configuration_table` and [`BootServices`] ends after `exit_boot_services`; the real tables
//!   are longer. That is safe (the firmware's allocation is larger than ours, and we only read),
//!   but a field added below the last one listed will not be found by name and has to be counted
//!   in from the specification the same way these were.
//! - **Nothing validates the table headers' CRC.** The firmware is the trust root at this point in
//!   boot; there is nothing to check it against that is not also the firmware.

use core::ffi::c_void;

/// An opaque firmware object. Only ever passed back to the firmware.
pub type Handle = *mut c_void;

/// A UEFI return code. Zero is success; the high bit marks an error.
pub type Status = usize;

/// `EFI_SUCCESS`.
pub const SUCCESS: Status = 0;

/// A UEFI GUID, in the mixed-endian form the specification prints and stores.
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct Guid {
    /// First 32 bits, stored little-endian.
    pub a: u32,
    /// Next 16 bits, stored little-endian.
    pub b: u16,
    /// Next 16 bits, stored little-endian.
    pub c: u16,
    /// The last eight bytes, stored big-endian, which is why they are a byte array and not a `u64`.
    pub d: [u8; 8],
}

/// `EFI_ACPI_20_TABLE_GUID`, whose configuration-table entry is the **XSDT-capable** RSDP.
///
/// This is the one to prefer: an RSDP found here has revision 2 or later, so it carries the 64-bit
/// `xsdt_address` as well as the 32-bit `rsdt_address`, and `machine_discovery::acpi` reads
/// whichever the revision says is there.
pub const ACPI_20_TABLE_GUID: Guid = Guid {
    a: 0x8868_e871,
    b: 0xe4f1,
    c: 0x11d3,
    d: [0xbc, 0x22, 0x00, 0x80, 0xc7, 0x3c, 0x88, 0x81],
};

/// `ACPI_TABLE_GUID`, the ACPI 1.0 RSDP, taken only when the 2.0 entry is absent.
pub const ACPI_10_TABLE_GUID: Guid = Guid {
    a: 0xeb9d_2d30,
    b: 0x2d88,
    c: 0x11d3,
    d: [0x9a, 0x16, 0x00, 0x90, 0x27, 0x3f, 0xc1, 0x4d],
};

/// The header every UEFI table begins with. Read for nothing here; present so the fields after it
/// are at their specified offsets.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct TableHeader {
    /// Identifies which table this is.
    pub signature: u64,
    /// The UEFI revision the table conforms to.
    pub revision: u32,
    /// Bytes in the header plus the table.
    pub header_size: u32,
    /// A CRC-32 over the table with this field zeroed.
    pub crc32: u32,
    /// Must be zero.
    pub reserved: u32,
}

/// `EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL`, cut off after the one method this loader calls.
///
/// It is worth having at all for one reason: **it is the only way a person standing at the machine
/// learns that anything ran before `ExitBootServices`.** On the `OptiPlex` the serial console does
/// not carry firmware output, so a stub that failed silently would be indistinguishable from a
/// stick the firmware never looked at.
#[allow(
    dead_code,
    reason = "the leading `reset` is a placeholder whose only job is to put `output_string` at its \
              specified offset; it is read by the firmware's own layout, never by this crate"
)]
#[repr(C)]
pub struct SimpleTextOutput {
    reset: usize,
    /// Print a NUL-terminated UTF-16 string.
    pub output_string: extern "efiapi" fn(*mut SimpleTextOutput, *const u16) -> Status,
}

/// `EFI_MEMORY_DESCRIPTOR`.
///
/// **Never stride an array of these by `size_of::<MemoryDescriptor>()`.** `GetMemoryMap` reports
/// its own `descriptor_size`, which the specification permits to be larger than the structure and
/// which real firmware does make larger. Striding by the Rust size is the classic UEFI loader bug:
/// it works on the firmware you tested and produces a garbage memory map on the next one.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct MemoryDescriptor {
    /// One of the `MEMORY_TYPE_*` constants below.
    pub kind: u32,
    /// Present so `physical_start` lands at offset 8, which is where the specification puts it.
    pub padding: u32,
    /// The first byte of the range.
    pub physical_start: u64,
    /// Where the range will be after `SetVirtualAddressMap`, which this loader never calls.
    pub virtual_start: u64,
    /// The length, in 4 KiB pages.
    pub page_count: u64,
    /// Cacheability and runtime attributes.
    pub attribute: u64,
}

/// `EFI_MEMORY_TYPE`. Only the ones [`crate::handoff`] classifies are named.
pub mod memory_type {
    /// Not usable by anyone.
    pub const RESERVED: u32 = 0;
    /// The loader's own image.
    pub const LOADER_CODE: u32 = 1;
    /// Memory the loader allocated, which is where this loader puts everything it hands over.
    pub const LOADER_DATA: u32 = 2;
    /// Firmware boot-services code, free once `ExitBootServices` returns.
    pub const BOOT_SERVICES_CODE: u32 = 3;
    /// Firmware boot-services data, free once `ExitBootServices` returns.
    pub const BOOT_SERVICES_DATA: u32 = 4;
    /// Firmware runtime code. Live forever.
    pub const RUNTIME_SERVICES_CODE: u32 = 5;
    /// Firmware runtime data. Live forever.
    pub const RUNTIME_SERVICES_DATA: u32 = 6;
    /// Free RAM.
    pub const CONVENTIONAL: u32 = 7;
    /// RAM the firmware found faulty.
    pub const UNUSABLE: u32 = 8;
    /// Holds ACPI tables; reclaimable once they have been read.
    pub const ACPI_RECLAIM: u32 = 9;
    /// ACPI non-volatile storage.
    pub const ACPI_NVS: u32 = 10;
    /// Memory-mapped IO.
    pub const MMIO: u32 = 11;
    /// Memory-mapped IO port space.
    pub const MMIO_PORT_SPACE: u32 = 12;
    /// Processor abstraction-layer code.
    pub const PAL_CODE: u32 = 13;
    /// Byte-addressable persistent memory.
    pub const PERSISTENT: u32 = 14;
}

/// `EFI_ALLOCATE_TYPE::AllocateMaxAddress`: give me pages **at or below** the address I pass in.
///
/// This is the one that matters here. Everything the kernel is handed has to be nameable by a
/// 32-bit trampoline running with paging off, so every allocation this loader makes asks for it
/// below 4 GiB rather than taking whatever the firmware felt like.
pub const ALLOCATE_MAX_ADDRESS: u32 = 1;

/// `EFI_ALLOCATE_TYPE::AllocateAddress`: give me exactly this address or fail.
pub const ALLOCATE_ADDRESS: u32 = 2;

/// `EFI_BOOT_SERVICES`, in specification order, truncated after `ExitBootServices`.
///
/// The unused entries are `usize` and are **counted in the comments**, because their number is the
/// only thing keeping the used ones at the right offsets.
#[allow(
    dead_code,
    reason = "the private fields are the unused entries of the firmware's table. They exist to \
              hold the used ones at their specified offsets and are deliberately spelled out one \
              by one rather than collapsed, because their COUNT is the ABI"
)]
#[repr(C)]
pub struct BootServices {
    /// The table header.
    pub hdr: TableHeader,

    // --- Task priority services (2) ---
    raise_tpl: usize,
    restore_tpl: usize,

    // --- Memory services (5) ---
    /// `AllocatePages(type, memory_type, pages, &mut physical_address)`.
    ///
    /// **`memory` is in/out**: with [`ALLOCATE_MAX_ADDRESS`] it carries the ceiling on the way in
    /// and the allocated base on the way out.
    pub allocate_pages: extern "efiapi" fn(u32, u32, usize, *mut u64) -> Status,
    /// `FreePages(physical_address, pages)`.
    pub free_pages: extern "efiapi" fn(u64, usize) -> Status,
    /// `GetMemoryMap(&mut size, buffer, &mut key, &mut descriptor_size, &mut descriptor_version)`.
    pub get_memory_map:
        extern "efiapi" fn(*mut usize, *mut u8, *mut usize, *mut usize, *mut u32) -> Status,
    /// `AllocatePool(memory_type, size, &mut buffer)`.
    pub allocate_pool: extern "efiapi" fn(u32, usize, *mut *mut u8) -> Status,
    /// `FreePool(buffer)`.
    pub free_pool: extern "efiapi" fn(*mut u8) -> Status,

    // --- Event and timer services (6) ---
    create_event: usize,
    set_timer: usize,
    wait_for_event: usize,
    signal_event: usize,
    close_event: usize,
    check_event: usize,

    // --- Protocol handler services (9) ---
    install_protocol_interface: usize,
    reinstall_protocol_interface: usize,
    uninstall_protocol_interface: usize,
    handle_protocol: usize,
    reserved: usize,
    register_protocol_notify: usize,
    locate_handle: usize,
    locate_device_path: usize,
    install_configuration_table: usize,

    // --- Image services (4 before the one we want) ---
    load_image: usize,
    start_image: usize,
    exit: usize,
    unload_image: usize,
    /// `ExitBootServices(image_handle, map_key)`.
    ///
    /// **The `map_key` must come from a `GetMemoryMap` with nothing allocated since**, which is
    /// what makes the call order in the loader binary (`src/main.rs`) rigid rather than stylistic.
    pub exit_boot_services: extern "efiapi" fn(Handle, usize) -> Status,

    // --- Miscellaneous services (3) ---
    get_next_monotonic_count: usize,
    stall: usize,
    set_watchdog_timer: usize,

    // --- Driver support services (2) ---
    connect_controller: usize,
    disconnect_controller: usize,

    // --- Open and close protocol services (3) ---
    open_protocol: usize,
    close_protocol: usize,
    open_protocol_information: usize,

    // --- Library services (2 before the one we want) ---
    protocols_per_handle: usize,
    locate_handle_buffer: usize,
    /// `LocateProtocol(&guid, registration, &mut interface)`.
    ///
    /// The one call milestone 243 added, and it is the cheapest possible form of the question:
    /// **is there a linear framebuffer on this machine, and where.** `registration` is always null
    /// here, which asks for the first handle carrying the protocol rather than the next one since
    /// some notification. On a machine with two display adapters that is the firmware's choice of
    /// console rather than ours, and this loader has no basis for a better one.
    pub locate_protocol: extern "efiapi" fn(*const Guid, *mut c_void, *mut *mut c_void) -> Status,
}

/// `EFI_GRAPHICS_OUTPUT_PROTOCOL_GUID`.
pub const GRAPHICS_OUTPUT_PROTOCOL_GUID: Guid = Guid {
    a: 0x9042_a9de,
    b: 0x23dc,
    c: 0x4a38,
    d: [0x96, 0xfb, 0x7a, 0xde, 0xd0, 0x80, 0x51, 0x6a],
};

/// `EFI_GRAPHICS_OUTPUT_MODE_INFORMATION`: what one video mode looks like.
///
/// **`pixels_per_scan_line` is not `horizontal_resolution`** and treating them as one is the
/// classic framebuffer bug. Firmware is free to pad each row out to a convenient stride, so a
/// writer that multiplies by the width paints a picture that shears progressively down the screen.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct GraphicsModeInformation {
    /// The structure's own version. 0 for everything this loader has met.
    pub version: u32,
    /// Width, in pixels.
    pub horizontal_resolution: u32,
    /// Height, in pixels.
    pub vertical_resolution: u32,
    /// One of [`pixel_format`]'s constants.
    pub pixel_format: u32,
    /// The channel masks, meaningful only for [`pixel_format::BIT_MASK`].
    pub pixel_information: [u32; 4],
    /// **The stride, in pixels**, which is the row pitch and not the width. See the note above.
    pub pixels_per_scan_line: u32,
}

/// `EFI_GRAPHICS_PIXEL_FORMAT`.
pub mod pixel_format {
    /// Bytes in memory are R, G, B, unused. A little-endian `u32` is therefore `0xXXBBGGRR`.
    pub const RGBX: u32 = 0;
    /// Bytes in memory are B, G, R, unused. A little-endian `u32` is therefore `0xXXRRGGBB`, which
    /// is the order every colour constant in this tree is already written in.
    pub const BGRX: u32 = 1;
    /// Channels described by masks rather than named. Not supported here; see `uefi_loader`'s BUGS.
    pub const BIT_MASK: u32 = 2;
    /// **There is no linear framebuffer at all**: this adapter can only be drawn on with `Blt`,
    /// which is a boot-services call and therefore gone by the time the kernel runs.
    pub const BLT_ONLY: u32 = 3;
}

/// `EFI_GRAPHICS_OUTPUT_PROTOCOL_MODE`: the mode the adapter is *in*, and where its pixels live.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct GraphicsOutputMode {
    /// How many modes [`GraphicsOutput::query_mode`] would accept.
    pub max_mode: u32,
    /// Which one is current.
    pub mode: u32,
    /// The current mode's geometry.
    pub info: *const GraphicsModeInformation,
    /// How many bytes of [`Self::info`] the firmware filled in.
    pub size_of_info: usize,
    /// **The physical address of the linear framebuffer, and it survives `ExitBootServices`.**
    ///
    /// That is the whole reason milestone 243 can use this: it is a bar aperture on the display
    /// adapter, not firmware memory, so nothing about ending the boot phase moves it or takes it
    /// away. What ends is the firmware's *console*, not the display.
    pub framebuffer_base: u64,
    /// How many bytes of it there are.
    pub framebuffer_size: usize,
}

/// `EFI_GRAPHICS_OUTPUT_PROTOCOL`, truncated after the one field this loader reads.
///
/// The three function pointers ahead of it are placeholders holding [`Self::mode`] at its specified
/// offset, exactly as in [`BootServices`]. This loader never changes the video mode: it takes
/// whatever the firmware left on the screen, because a mode set here is a mode the kernel would
/// have to be told about through a channel that does not exist yet, and because the firmware has
/// already picked one that works on this monitor.
#[allow(
    dead_code,
    reason = "the three leading entries are the firmware's, present only to put `mode` at its \
              specified offset"
)]
#[repr(C)]
pub struct GraphicsOutput {
    query_mode: usize,
    set_mode: usize,
    blt: usize,
    /// The current mode, and the framebuffer's address.
    pub mode: *const GraphicsOutputMode,
}

/// One entry of the UEFI configuration table: a GUID and a pointer.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct ConfigurationTable {
    /// What the pointer points at.
    pub vendor_guid: Guid,
    /// The table itself, physical (this loader never calls `SetVirtualAddressMap`).
    pub vendor_table: *const c_void,
}

/// `EFI_SYSTEM_TABLE`, truncated after `configuration_table`.
#[allow(
    dead_code,
    reason = "as in `BootServices` above: the private fields are the table entries this loader \
              does not call, and their presence is what keeps the ones it does call at the right \
              offsets"
)]
#[repr(C)]
pub struct SystemTable {
    /// The table header.
    pub hdr: TableHeader,
    firmware_vendor: *const u16,
    firmware_revision: u32,
    console_in_handle: Handle,
    con_in: usize,
    console_out_handle: Handle,
    /// The text console, or null when the firmware has none.
    pub con_out: *mut SimpleTextOutput,
    standard_error_handle: Handle,
    std_err: *mut SimpleTextOutput,
    runtime_services: usize,
    /// Everything this loader calls before it hands over.
    pub boot_services: *mut BootServices,
    /// How many entries [`Self::configuration_table`] has.
    pub number_of_table_entries: usize,
    /// **Where the ACPI RSDP is found**, and the reason a UEFI boot needs no `"RSD PTR "` scan.
    pub configuration_table: *const ConfigurationTable,
}
