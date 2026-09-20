//! **The boot entry real firmware can start** (milestone 87; aarch64 and riscv64 since the stick
//! milestone, 2026-09-19).
//!
//! A UEFI application the firmware loads from the removable-media path of a FAT volume
//! (`\EFI\BOOT\BOOTX64.EFI`, `BOOTAA64.EFI`, `BOOTRISCV64.EFI`), carrying the kernel and the
//! userspace archive inside itself, which asks the firmware the questions a kernel cannot ask once
//! the firmware is gone and hands over. One source, three architectures; what differs is how the
//! kernel is told about the machine, and that lives under `src/arch/`:
//!
//! | Architecture | The kernel's entry contract | Told about the machine by |
//! |---|---|---|
//! | `x86_64` | PVH: 32-bit protected mode, `ebx` = `hvm_start_info` | a synthesised `hvm_start_info` (`arch/x86_64`) |
//! | aarch64 | Linux arm64 `booti`: MMU off, `x0` = device tree | the firmware's device tree, with `/chosen` naming the archive (`arch/aarch64`) |
//! | riscv64 | Linux riscv `booti`: paging off, `a0` = hart, `a1` = device tree | the same, plus the boot hart from the firmware (`arch/riscv64`) |
//!
//! In every case **the kernel is not modified and cannot tell which loader started it**: the state
//! it is entered in is the state QEMU's `-kernel` or U-Boot's `booti` would have entered it in.
//!
//! **Read notes/x86-uefi-boot.md and notes/boot-stick.md before changing anything here.**
//!
//! # The order of operations is rigid, and two rules are why
//!
//! `ExitBootServices` takes a `map_key` from a `GetMemoryMap` with **no allocation in between**, so
//! every buffer is allocated before that call and filled in after it. And **after
//! `ExitBootServices` there is no console**: a failure past that point can only halt, so everything
//! checkable is checked while there is still something to report it on.
//!
//! # BUGS
//!
//! - **Secure Boot must be off.** This image is unsigned and nothing here signs it. On the `OptiPlex`
//!   that is a firmware setting; under QEMU it is the difference between `edk2-x86_64-code.fd` and
//!   `edk2-x86_64-secure-code.fd`. Signing is milestone 22's neighbourhood (`measured_boot`), not
//!   this one.
//! - **The kernel is embedded rather than loaded from the filesystem.** One file on the stick
//!   instead of three, and no `SimpleFileSystem` protocol to speak, at the cost of rebuilding this
//!   loader whenever the kernel changes. `cargo xtask uefi-image` and `cargo xtask stick` do both
//!   in one command, and the stick carries a `NIFE.TXT` naming its build, so a stale file is no
//!   longer silent; it is still stale.
//! - **Nothing here verifies what it hands over at boot.** The kernel and the archive are bytes this
//!   binary was compiled with, so the trust boundary is the build. `build.rs` does refuse to embed
//!   an archive the kernel does not vouch for, which is the one check that matters at build time.
//! - **AP bring-up under UEFI is proved on OVMF and nowhere else** (milestone 195); see
//!   `arch/x86_64`.
//! - **The `hvm_start_info` command line carries exactly one thing** (milestone 243): where the
//!   screen is. Nothing reads a configuration file off the stick, so anything on that line is
//!   something this binary decided.
//! - **The screen is taken as the firmware left it**, and on aarch64 and riscv64 it is not handed
//!   over at all: those kernels learn about a display from the device tree, and the firmware's GOP
//!   framebuffer is not in it.
//! - **A `PixelBitMask` or `PixelBltOnly` adapter gets no screen** on `x86_64`, and the loader says
//!   so before it hands over.

#![no_std]
#![no_main]

use core::ptr;

#[cfg(not(target_arch = "x86_64"))]
use uefi_loader::device_tree_patch;
use uefi_loader::efi::{
    self, ALLOCATE_ADDRESS, ALLOCATE_MAX_ADDRESS, BootServices, ConfigurationTable, Handle,
    SUCCESS, Status, SystemTable, memory_type,
};
use uefi_loader::image;

mod arch;

/// The kernel ELF and the userspace archive, embedded by `build.rs`.
mod embedded {
    include!(concat!(env!("OUT_DIR"), "/embedded.rs"));
}

/// UEFI's page size, and the granularity of every `AllocatePages` call.
const PAGE: u64 = 4096;

/// Slack, in descriptors, added to the memory map buffer between the sizing call and the real one.
///
/// The UEFI specification says so outright: allocating memory to hold the map can itself change the
/// map. This loader allocates exactly once between the two calls, and that allocation can split one
/// free range into as many as three; thirty-two is far more than that and costs 4 KiB.
const MAP_SLACK_DESCRIPTORS: usize = 32;

/// The kernel, placed: where to enter it and the physical range it occupies.
pub struct Placed {
    /// The physical address of `_start` ([`image::physical_entry`]).
    pub entry: u64,
    /// The first byte of the placed image, page-aligned.
    pub start: u64,
    /// One past the last, page-aligned.
    pub end: u64,
}

/// Where the firmware starts us. The name is UEFI's default entry point for the UEFI targets, and
/// the one `cargo xtask`'s riscv64 build names as the image's entry.
#[unsafe(no_mangle)]
pub extern "efiapi" fn efi_main(handle: Handle, system_table: *mut SystemTable) -> Status {
    // SAFETY: the firmware's own contract for an EFI application entry point: it passes a valid
    // system table that outlives the call, and `boot_services` is non-null until
    // `ExitBootServices`.
    let table = unsafe { &*system_table };
    // SAFETY: as above.
    let services = unsafe { &*table.boot_services };

    say(table, "nife uefi_loader: milestone 87\r\n");

    // A UEFI error status (the high bit is what makes it one), so the firmware reports the failure
    // and moves to the next boot option rather than presenting a blank screen.
    const LOAD_ERROR: Status = (1 << (usize::BITS - 1)) | 1;

    let Err(reason) = load(handle, table, services) else {
        // `load` hands the machine to the kernel and never comes back, so this cannot run.
        return LOAD_ERROR;
    };
    say(table, "uefi_loader: ");
    say(table, reason);
    say(table, "\r\n");
    LOAD_ERROR
}

/// Everything between "the firmware started us" and "the kernel is running", in the one order that
/// works. Returns only on failure, and only while there is still a console to say so on.
fn load(handle: Handle, table: &SystemTable, services: &BootServices) -> Result<(), &'static str> {
    // --- 1. What this architecture needs from the firmware, read before anything moves ---
    let found = arch::discover(table, services)?;

    // --- 2. The kernel, at the physical addresses its linker script chose ---
    let kernel = place_kernel(table, services)?;

    // --- 3. The userspace archive, if this build has one ---
    let module = place_archive(services)?;

    // --- 4. The architecture's handover, which ends in the kernel or in an error ---
    arch::hand_over(handle, table, services, found, &kernel, module)
}

/// Place the embedded kernel by its physical addresses, zeroed first so every `.bss` and `NOLOAD`
/// section arrives zeroed.
fn place_kernel(table: &SystemTable, services: &BootServices) -> Result<Placed, &'static str> {
    // `elf::Elf::parse` validates everything before this loader moves a byte: bounds, overlap,
    // W^X, the entry point inside an executable segment.
    let kernel = image::parse(embedded::KERNEL)?;
    let (start, end) = image::physical_span(kernel.segments(), PAGE)
        .ok_or("the embedded kernel has no loadable segments")?;
    let entry = image::physical_entry(kernel.entry(), kernel.segments())
        .ok_or("the embedded kernel's entry point is in no segment")?;

    // `AllocateAddress`, not `AllocateMaxAddress`: the kernel is linked for exactly this range
    // (`kernel/link-<arch>.ld`'s `PHYS_START`) and there is nowhere else to put it. A firmware that
    // has something of its own there fails HERE, with the range and the offending descriptors
    // printed, rather than by silently overwriting whatever it was.
    let mut base = start;
    if (services.allocate_pages)(
        ALLOCATE_ADDRESS,
        memory_type::LOADER_DATA,
        ((end - start) / PAGE) as usize,
        &mut base,
    ) != SUCCESS
    {
        // **Name what is in the way before giving up** (milestone 195). "The firmware said no" is
        // the least actionable sentence a person standing at a machine can be handed.
        say_span(table, "uefi_loader: wanted ", start, end);
        say_conflict(table, services, start, end);
        return Err("the firmware would not give up the kernel's load range");
    }

    // SAFETY: the firmware just granted this whole range exclusively.
    unsafe { ptr::write_bytes(start as *mut u8, 0, (end - start) as usize) };

    // `Segment::data` is `p_filesz` bytes, which is empty for a `NOLOAD` section: they arrive as
    // address space to reserve rather than bytes to copy, and the zeroing above is what makes their
    // memory correct.
    for segment in kernel.segments() {
        // SAFETY: `physical_span` covers every segment, and the range it covers was just allocated;
        // the source is a slice of this binary's own `.rodata`. They cannot overlap: the source is
        // in the firmware's allocation for this image and the destination is a range the firmware
        // handed out separately.
        unsafe {
            ptr::copy_nonoverlapping(
                segment.data.as_ptr(),
                segment.paddr as *mut u8,
                segment.data.len(),
            );
        }
    }
    Ok(Placed { entry, start, end })
}

/// Copy the archive to memory below the architecture's ceiling, if this build has one.
fn place_archive(services: &BootServices) -> Result<Option<(u64, u64)>, &'static str> {
    if embedded::INITRD.is_empty() {
        return Ok(None);
    }
    let pages = (embedded::INITRD.len() as u64).div_ceil(PAGE) as usize;
    let base = allocate_below(services, pages, memory_type::LOADER_DATA)
        .ok_or("no memory below the kernel's reach for the userspace archive")?;
    // SAFETY: `pages` covers the archive and the range was just granted exclusively.
    unsafe {
        ptr::copy_nonoverlapping(
            embedded::INITRD.as_ptr(),
            base as *mut u8,
            embedded::INITRD.len(),
        );
    }
    Ok(Some((base, embedded::INITRD.len() as u64)))
}

/// `AllocatePages(AllocateMaxAddress)` under [`arch::ALLOCATION_CEILING`], which is the highest
/// physical address the kernel can reach before its own page tables are up.
fn allocate_below(services: &BootServices, pages: usize, kind: u32) -> Option<u64> {
    let mut base = arch::ALLOCATION_CEILING;
    ((services.allocate_pages)(ALLOCATE_MAX_ADDRESS, kind, pages, &mut base) == SUCCESS)
        .then_some(base)
}

/// The firmware's configuration table as a slice.
fn configuration_table(table: &SystemTable) -> &[ConfigurationTable] {
    if table.configuration_table.is_null() {
        return &[];
    }
    // SAFETY: the firmware promises `number_of_table_entries` valid entries at
    // `configuration_table`, and they outlive boot services.
    unsafe { core::slice::from_raw_parts(table.configuration_table, table.number_of_table_entries) }
}

/// **Copy the firmware's device tree below the ceiling, naming the archive in `/chosen`.**
/// aarch64 and riscv64 both do this, identically.
#[cfg(not(target_arch = "x86_64"))]
fn copy_device_tree(
    services: &BootServices,
    source: *const u8,
    module: Option<(u64, u64)>,
) -> Result<u64, &'static str> {
    // SAFETY: the firmware installed this table and it outlives boot services; the header's first
    // eight bytes are magic and total size, and the full length is read from them before the rest.
    let header = unsafe { core::slice::from_raw_parts(source, 8) };
    let total = u32::from_be_bytes([header[4], header[5], header[6], header[7]]) as usize;
    // SAFETY: as above, now for the length the header states.
    let src = unsafe { core::slice::from_raw_parts(source, total) };
    let capacity = device_tree_patch::output_len(src).map_err(device_tree_patch::Error::reason)?;
    let pages = (capacity as u64).div_ceil(PAGE) as usize;
    let base = allocate_below(services, pages, memory_type::LOADER_DATA)
        .ok_or("no memory below the kernel's reach for the device tree")?;
    // SAFETY: `pages` pages at `base` were just granted exclusively.
    let out = unsafe { core::slice::from_raw_parts_mut(base as *mut u8, pages * PAGE as usize) };
    match module {
        Some((start, len)) => {
            device_tree_patch::with_initrd(src, start, start + len, out)
                .map_err(device_tree_patch::Error::reason)?;
        }
        None => out[..total].copy_from_slice(src),
    }
    Ok(base)
}

/// **Exit boot services when the caller has nothing to read from the final map.**
///
/// The device-tree architectures hand the kernel the firmware's tree, not its memory map, so the
/// map is fetched only because `ExitBootServices` needs its key. One pool allocation, sized with
/// slack, then `GetMemoryMap` and `ExitBootServices` back to back. The specification allows one
/// retry after a refused key (the firmware may have changed the map by timer activity), and this
/// takes it.
#[cfg(not(target_arch = "x86_64"))]
fn exit_boot_services(handle: Handle, services: &BootServices) -> Result<(), &'static str> {
    let mut bytes = 0usize;
    let mut key = 0usize;
    let mut descriptor_size = 0usize;
    let mut version = 0u32;
    (services.get_memory_map)(
        &mut bytes,
        ptr::null_mut(),
        &mut key,
        &mut descriptor_size,
        &mut version,
    );
    if bytes == 0 || descriptor_size < size_of::<efi::MemoryDescriptor>() {
        return Err("the firmware reported no memory map");
    }
    let capacity = bytes + MAP_SLACK_DESCRIPTORS * descriptor_size;
    let mut buffer: *mut u8 = ptr::null_mut();
    if (services.allocate_pool)(memory_type::LOADER_DATA, capacity, &mut buffer) != SUCCESS
        || buffer.is_null()
    {
        return Err("no memory for the firmware's memory map");
    }
    for _ in 0..2 {
        let mut got = capacity;
        if (services.get_memory_map)(
            &mut got,
            buffer,
            &mut key,
            &mut descriptor_size,
            &mut version,
        ) != SUCCESS
        {
            return Err("the firmware's memory map grew between the two GetMemoryMap calls");
        }
        if (services.exit_boot_services)(handle, key) == SUCCESS {
            return Ok(());
        }
    }
    Err("ExitBootServices refused the map key twice")
}

/// Print one unsigned value in decimal, without an allocator and without `core::fmt`.
#[cfg_attr(target_arch = "aarch64", allow(dead_code))]
fn say_decimal(table: &SystemTable, value: u32) {
    let mut buffer = [0u8; 10];
    let mut n = 0;
    let mut digits = [0u8; 10];
    let mut left = value;
    loop {
        digits[n] = b'0' + (left % 10) as u8;
        left /= 10;
        n += 1;
        if left == 0 {
            break;
        }
    }
    for i in 0..n {
        buffer[i] = digits[n - 1 - i];
    }
    // SAFETY: every byte written above is an ASCII digit.
    say(table, unsafe {
        core::str::from_utf8_unchecked(&buffer[..n])
    });
}

/// Print a `[start, end)` physical range after a caller-supplied phrase.
///
/// Its own function rather than a format string because there is no allocator here and
/// `core::fmt` on the firmware console would pull in machinery this binary otherwise does not have.
fn say_span(table: &SystemTable, what: &str, start: u64, end: u64) {
    say(table, what);
    say_hex(table, start);
    say(table, "..");
    say_hex(table, end);
    say(table, "\r\n");
}

/// Print one 64-bit value as `0x...`, without an allocator and without `core::fmt`.
fn say_hex(table: &SystemTable, value: u64) {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut buffer = [0u8; 18];
    buffer[0] = b'0';
    buffer[1] = b'x';
    for i in 0..16 {
        buffer[2 + i] = DIGITS[((value >> (60 - 4 * i)) & 0xf) as usize];
    }
    // SAFETY: every byte written above is ASCII.
    say(table, unsafe { core::str::from_utf8_unchecked(&buffer) });
}

/// **Say which memory the firmware is already using inside a range it refused** (milestone 195).
///
/// `AllocatePages(AllocateAddress)` reports one status and no address, so on its own it cannot
/// distinguish "your kernel is too big for this machine" from "your kernel overlaps a firmware
/// volume". This walks the map the firmware would have handed over anyway and prints every
/// descriptor in the way that is not free RAM, with its type number as the UEFI specification
/// numbers them (`efi::memory_type`).
///
/// It runs only on the failure path, so the allocation it makes cannot disturb the map key
/// `ExitBootServices` later checks: there is no later on this path.
///
/// # BUGS
///
/// - **A firmware too broken to report a memory map prints nothing here**, and the caller's own
///   sentence is then all the reader gets. That is the honest floor: there is no second source for
///   this information.
fn say_conflict(table: &SystemTable, services: &BootServices, start: u64, end: u64) {
    let mut bytes = 0usize;
    let mut key = 0usize;
    let mut descriptor_size = 0usize;
    let mut version = 0u32;
    (services.get_memory_map)(
        &mut bytes,
        ptr::null_mut(),
        &mut key,
        &mut descriptor_size,
        &mut version,
    );
    if bytes == 0 || descriptor_size < size_of::<efi::MemoryDescriptor>() {
        return;
    }
    // Slack for the pool allocation itself, the same reason `MAP_SLACK_DESCRIPTORS` exists.
    let capacity = bytes + MAP_SLACK_DESCRIPTORS * descriptor_size;
    let mut buffer: *mut u8 = ptr::null_mut();
    if (services.allocate_pool)(memory_type::LOADER_DATA, capacity, &mut buffer) != SUCCESS
        || buffer.is_null()
    {
        return;
    }
    let mut got = capacity;
    if (services.get_memory_map)(
        &mut got,
        buffer,
        &mut key,
        &mut descriptor_size,
        &mut version,
    ) == SUCCESS
    {
        for i in 0..got / descriptor_size {
            // SAFETY: `buffer` holds `got` bytes of descriptors and `i` is inside that count. Read
            // unaligned because the firmware chooses `descriptor_size` and nothing promises the
            // stride keeps 8-byte alignment.
            let descriptor = unsafe {
                ptr::read_unaligned(
                    (buffer as usize + i * descriptor_size) as *const efi::MemoryDescriptor,
                )
            };
            let first = descriptor.physical_start;
            let last = first + descriptor.page_count * PAGE;
            if descriptor.kind == memory_type::CONVENTIONAL || last <= start || first >= end {
                continue;
            }
            say_span(table, "uefi_loader:   in the way: ", first, last);
            say(table, "uefi_loader:   memory type ");
            say_hex(table, u64::from(descriptor.kind));
            say(table, "\r\n");
        }
    }
    (services.free_pool)(buffer);
}

/// Print one ASCII line on the firmware console, if there is one.
///
/// It exists for the person standing at the machine: the `OptiPlex`'s serial port carries the
/// kernel's output but not the firmware's, so without this a loader that failed before
/// `ExitBootServices` would look exactly like a stick the firmware never read. Non-ASCII input is
/// not this loader's problem; every string it prints is a literal in this binary.
fn say(table: &SystemTable, text: &str) {
    if table.con_out.is_null() {
        return;
    }
    // 128 is longer than any message here and keeps this off the heap, of which there is none.
    let mut utf16 = [0u16; 128];
    let mut n = 0;
    for byte in text.bytes() {
        if n + 1 >= utf16.len() {
            break;
        }
        utf16[n] = u16::from(byte);
        n += 1;
    }
    // SAFETY: `con_out` is non-null and the firmware owns it until `ExitBootServices`; the buffer
    // is NUL-terminated because `n + 1 < len` leaves at least one zero behind.
    unsafe { ((*table.con_out).output_string)(table.con_out, utf16.as_ptr()) };
}

/// There is nothing to unwind to, and by the time most failures here can happen there is no console
/// left to say anything on, so a panic parks the core.
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    arch::park()
}
