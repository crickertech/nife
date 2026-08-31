//! **The boot entry real firmware can start** (milestone 87).
//!
//! The x86_64 kernel boots under QEMU by PVH, a *hypervisor* direct-boot protocol. No machine's
//! firmware speaks it, so up to this milestone there was no way to start nife on the Dell `OptiPlex`
//! sitting on calef's desk. This is that way: a UEFI application the firmware loads from
//! `\EFI\BOOT\BOOTX64.EFI` on a FAT32 partition, carrying the kernel and the userspace archive
//! inside itself, which asks the firmware the three questions a kernel cannot ask once the firmware
//! is gone (where is RAM, where is ACPI, may I have this memory) and hands over.
//!
//! **Read notes/x86-uefi-boot.md before changing anything here.** It carries the fork this
//! milestone decided (UEFI rather than GRUB, and why), the exact procedure for the bench, and what
//! could not be proved without the hardware.
//!
//! # The shape, and the one decision everything follows from
//!
//! **The kernel is entered through its existing `_start`, in 32-bit protected mode, with PVH's
//! register contract.** The kernel is not modified and cannot tell which loader started it.
//! Everything else here follows:
//!
//! - The handoff is a synthesised `hvm_start_info` (`uefi_loader::handoff`), so
//!   `machine_discovery::x86_64` and `arch::x86_64::machine` are unchanged and already host-tested.
//! - The last thing this loader does is leave long mode (`src/leave_long_mode.s`), because that is
//!   the only difference between the state UEFI hands us and the state QEMU's PVH loader hands the
//!   kernel.
//! - `script/test --arch x86_64` cannot regress, because the path it rides is untouched.
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
//!   loader whenever the kernel changes. `cargo xtask uefi-image` does both in one command, so the
//!   cost lands on the build; but a `.efi` copied to a stick and left there goes stale silently.
//! - **Nothing here verifies what it hands over.** The kernel and the archive are bytes this binary
//!   was compiled with, so the trust boundary is the build. `measured_boot`'s manifest is not
//!   consulted.
//! - **AP bring-up is untested under UEFI.** `arch::x86_64::ap_boot` copies its real-mode trampoline
//!   to physical `0x8000`, which is memory this loader never asked the firmware for. The runner and
//!   the bench procedure both boot one core, so nothing has exercised it; asking the firmware for
//!   that page here is the fix if it turns out to matter.
//! - **The `hvm_start_info` command line is empty.** PVH carries one and the kernel ignores it, so
//!   there is nowhere yet for a boot argument to come from or go to.

#![no_std]
#![no_main]

use core::ptr;

use uefi_loader::efi::{
    self, ALLOCATE_ADDRESS, ALLOCATE_MAX_ADDRESS, BootServices, Handle, SUCCESS, Status,
    SystemTable, memory_type,
};
use uefi_loader::handoff::{
    MEMMAP_ENTRY_LEN, MODULE_ENTRY_LEN, START_INFO_LEN, StartInfo, e820_kind, encode_memmap_entry,
    encode_module,
};
use uefi_loader::image::Image;

/// The kernel ELF and the userspace archive, embedded by `build.rs`.
mod embedded {
    include!(concat!(env!("OUT_DIR"), "/embedded.rs"));
}

// The mode-switch trampoline. See src/leave_long_mode.s, which is where the interesting half of
// this file's job actually happens.
core::arch::global_asm!(include_str!("leave_long_mode.s"));

unsafe extern "C" {
    /// The trampoline's first instruction. Copied, not called where it is linked.
    static x86_leave_long_mode: [u8; 0];
    /// The 32-bit continuation, whose address in the *copy* is an argument to the trampoline.
    static x86_leave_long_mode_pmode32: [u8; 0];
    /// The three descriptors the trampoline loads.
    static x86_leave_long_mode_gdt: [u8; 0];
    /// The `lgdt` operand, whose 8-byte base this loader patches after the copy.
    static x86_leave_long_mode_gdtr: [u8; 0];
    /// One past the last byte to copy.
    static x86_leave_long_mode_end: [u8; 0];
}

/// UEFI's page size, and the granularity of every `AllocatePages` call.
const PAGE: u64 = 4096;

/// **The ceiling on every allocation this loader makes.**
///
/// Everything the kernel is handed has to be nameable by a 32-bit instruction stream running with
/// paging off, which is the state `_start` is entered in. A `hvm_start_info` at 5 GiB would be a
/// pointer the kernel's trampoline literally cannot load.
const BELOW_4G: u64 = 0xffff_ffff;

/// Slack, in descriptors, added to the memory map buffer between the sizing call and the real one.
///
/// The UEFI specification says so outright: allocating memory to hold the map can itself change the
/// map. This loader allocates exactly once between the two calls, and that allocation can split one
/// free range into as many as three; thirty-two is far more than that and costs 4 KiB.
const MAP_SLACK_DESCRIPTORS: usize = 32;

/// Where the firmware starts us. The name is UEFI's default entry point for the
/// `x86_64-unknown-uefi` target.
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
    const LOAD_ERROR: Status = 0x8000_0000_0000_0000 | 1;

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
    // --- 1. The ACPI root pointer, which is the whole reason a kernel wants UEFI's help ---
    //
    // Taken FIRST, because it is a read of a table that is already there: nothing later can
    // invalidate it, and if it is missing this loader should say so before it has moved anything.
    let rsdp = find_rsdp(table).ok_or("the firmware's configuration table has no ACPI RSDP")?;

    // --- 2. Place the kernel at the physical addresses its linker script chose ---
    let kernel = Image::parse(embedded::KERNEL).map_err(|e| e.text())?;
    let (span_start, span_end) = kernel.physical_span(PAGE).map_err(|e| e.text())?;

    // `AllocateAddress`, not `AllocateMaxAddress`: the kernel is linked for exactly this range
    // (`kernel/link-x86_64.ld`'s `PHYS_START`) and there is nowhere else to put it. A firmware that
    // has something of its own at 1 MiB fails HERE, with an address printed, rather than by
    // silently overwriting whatever it was.
    let mut kernel_base = span_start;
    let kernel_pages = ((span_end - span_start) / PAGE) as usize;
    if (services.allocate_pages)(
        ALLOCATE_ADDRESS,
        memory_type::LOADER_DATA,
        kernel_pages,
        &mut kernel_base,
    ) != SUCCESS
    {
        return Err("the firmware would not give up the kernel's load range at 1 MiB");
    }

    // Zero the whole span before copying anything, which is what makes every `.bss` and every
    // `NOLOAD` section (the boot page tables among them) arrive zeroed. `boot.s` zeroes its own
    // tables anyway; the two agreeing is cheaper than either one being the only guard.
    // SAFETY: the firmware just granted this whole range exclusively.
    unsafe { ptr::write_bytes(span_start as *mut u8, 0, (span_end - span_start) as usize) };

    for segment in kernel.load_segments() {
        let segment = segment.map_err(|e| e.text())?;
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

    // --- 3. The userspace archive, if this build has one ---
    let module = if embedded::INITRD.is_empty() {
        None
    } else {
        let pages = (embedded::INITRD.len() as u64).div_ceil(PAGE) as usize;
        let mut base = BELOW_4G;
        if (services.allocate_pages)(
            ALLOCATE_MAX_ADDRESS,
            memory_type::LOADER_DATA,
            pages,
            &mut base,
        ) != SUCCESS
        {
            return Err("no memory below 4 GiB for the userspace archive");
        }
        // SAFETY: `pages` covers the archive and the range was just granted exclusively.
        unsafe {
            ptr::copy_nonoverlapping(
                embedded::INITRD.as_ptr(),
                base as *mut u8,
                embedded::INITRD.len(),
            );
        }
        Some((base, embedded::INITRD.len() as u64))
    };

    // --- 4. Size the memory map, then allocate everything the handoff needs in one block ---
    //
    // The sizing call is expected to fail with EFI_BUFFER_TOO_SMALL and to report the size it
    // wanted; a firmware that returned success for a zero-length buffer would be reporting an empty
    // machine, which is why the size is checked rather than the status.
    let mut map_bytes = 0usize;
    let mut map_key = 0usize;
    let mut descriptor_size = 0usize;
    let mut descriptor_version = 0u32;
    (services.get_memory_map)(
        &mut map_bytes,
        ptr::null_mut(),
        &mut map_key,
        &mut descriptor_size,
        &mut descriptor_version,
    );
    if map_bytes == 0 || descriptor_size < size_of::<efi::MemoryDescriptor>() {
        return Err("the firmware reported no memory map");
    }

    let map_capacity = map_bytes + MAP_SLACK_DESCRIPTORS * descriptor_size;
    let entry_capacity = map_capacity / descriptor_size;

    // One allocation, three regions, so there is one failure to report and one address to print:
    //   page 0                    the `hvm_start_info` and the one-entry module list
    //   [entries)                 the PVH memory map this loader writes
    //   [raw)                     the firmware's own memory map, read once and never handed on
    let entries_bytes = (entry_capacity * MEMMAP_ENTRY_LEN) as u64;
    let handoff_pages = (PAGE
        + entries_bytes.next_multiple_of(PAGE)
        + (map_capacity as u64).next_multiple_of(PAGE))
        / PAGE;
    let mut handoff_base = BELOW_4G;
    if (services.allocate_pages)(
        ALLOCATE_MAX_ADDRESS,
        memory_type::LOADER_DATA,
        handoff_pages as usize,
        &mut handoff_base,
    ) != SUCCESS
    {
        return Err("no memory below 4 GiB for the boot handoff");
    }
    let start_info_at = handoff_base;
    let module_list_at = handoff_base + START_INFO_LEN as u64;
    let memmap_at = handoff_base + PAGE;
    let raw_map_at = memmap_at + entries_bytes.next_multiple_of(PAGE);

    // --- 5. The mode-switch trampoline, in a page the firmware will let us execute ---
    //
    // `LOADER_CODE` rather than `LOADER_DATA`, and that is not tidiness: firmware with a memory
    // protection policy (OVMF has one, and so does every recent vendor firmware) sets the
    // execute-disable bit on data allocations, and the first instruction of the copy would fault
    // with boot services still live and nothing watching.
    //
    // Below 4 GiB for the same reason everything else is: the second half of this blob executes
    // with paging off, where a linear address is a physical one.
    let mut trampoline_base = BELOW_4G;
    if (services.allocate_pages)(
        ALLOCATE_MAX_ADDRESS,
        memory_type::LOADER_CODE,
        1,
        &mut trampoline_base,
    ) != SUCCESS
    {
        return Err("no executable page below 4 GiB for the mode-switch trampoline");
    }
    let trampoline = copy_trampoline(trampoline_base)?;

    // Everything the kernel is handed has to be reachable from 32 bits. Checked here, with a
    // console still available, rather than discovered as a triple fault on somebody's desk.
    if kernel.entry >= BELOW_4G || start_info_at >= BELOW_4G {
        return Err("the kernel entry or the handoff landed above 4 GiB");
    }

    say(
        table,
        "uefi_loader: kernel placed, exiting boot services\r\n",
    );

    // --- 6. The final map, then the point of no return ---
    //
    // NOTHING may allocate between these two calls. `ExitBootServices` compares the key against the
    // map's current generation and refuses if anything has changed, which is the specification
    // protecting the kernel from being handed a map that no longer describes the machine.
    let mut final_bytes = map_capacity;
    if (services.get_memory_map)(
        &mut final_bytes,
        raw_map_at as *mut u8,
        &mut map_key,
        &mut descriptor_size,
        &mut descriptor_version,
    ) != SUCCESS
    {
        return Err("the firmware's memory map grew between the two GetMemoryMap calls");
    }
    if (services.exit_boot_services)(handle, map_key) != SUCCESS {
        return Err("ExitBootServices refused the map key");
    }

    // --- 7. From here the firmware is gone. No console, no allocation, no going back. ---
    let descriptors = final_bytes / descriptor_size;
    let written = descriptors.min(entry_capacity);
    for i in 0..written {
        // SAFETY: `raw_map_at` holds `final_bytes` of descriptors and `i < final_bytes /
        // descriptor_size`. Read unaligned because the firmware chooses `descriptor_size` and
        // nothing promises the stride keeps 8-byte alignment.
        let descriptor = unsafe {
            ptr::read_unaligned(
                (raw_map_at as usize + i * descriptor_size) as *const efi::MemoryDescriptor,
            )
        };
        let bytes = encode_memmap_entry(
            descriptor.physical_start,
            descriptor.page_count * PAGE,
            e820_kind(descriptor.kind),
        );
        // SAFETY: `written <= entry_capacity`, which is what the region was sized for.
        unsafe {
            ptr::copy_nonoverlapping(
                bytes.as_ptr(),
                (memmap_at as usize + i * MEMMAP_ENTRY_LEN) as *mut u8,
                MEMMAP_ENTRY_LEN,
            );
        }
    }

    let info = StartInfo {
        rsdp,
        modules: if module.is_some() { module_list_at } else { 0 },
        module_count: u32::from(module.is_some()),
        memmap: memmap_at,
        memmap_entries: written as u32,
    };
    if let Some((addr, size)) = module {
        let bytes = encode_module(addr, size);
        // SAFETY: the module list sits inside page 0 of the handoff block, after the 56-byte
        // structure, and is 32 bytes long.
        unsafe {
            ptr::copy_nonoverlapping(bytes.as_ptr(), module_list_at as *mut u8, MODULE_ENTRY_LEN)
        };
    }
    let bytes = info.encode();
    // SAFETY: page 0 of the handoff block, 56 bytes.
    unsafe { ptr::copy_nonoverlapping(bytes.as_ptr(), start_info_at as *mut u8, START_INFO_LEN) };

    // The trampoline was copied to an executable page below 4 GiB and its GDT pointer patched;
    // every argument is a physical address below 4 GiB, which is the contract stated at the top of
    // leave_long_mode.s. It does not return, so this expression's type is `!` and the `Ok` arm of
    // this function's signature is unreachable.
    (trampoline.enter)(
        kernel.entry,
        start_info_at,
        trampoline.gdtr,
        trampoline.pmode32,
    )
}

/// Where the copied trampoline's three interesting addresses ended up.
struct Trampoline {
    /// Its first instruction, called with the System V convention.
    enter: extern "sysv64" fn(u64, u64, u64, u64) -> !,
    /// The `lgdt` operand, argument three.
    gdtr: u64,
    /// The 32-bit continuation, argument four.
    pmode32: u64,
}

/// Copy `leave_long_mode.s` into `base` and patch the one address it cannot know at link time.
fn copy_trampoline(base: u64) -> Result<Trampoline, &'static str> {
    let start = (&raw const x86_leave_long_mode).cast::<u8>();
    let end = (&raw const x86_leave_long_mode_end).cast::<u8>();
    let len = end as usize - start as usize;
    if len > PAGE as usize {
        return Err("the mode-switch trampoline no longer fits in one page");
    }

    let offset =
        |symbol: *const [u8; 0]| base + (symbol.cast::<u8>() as usize - start as usize) as u64;
    let gdt = offset(&raw const x86_leave_long_mode_gdt);
    let gdtr = offset(&raw const x86_leave_long_mode_gdtr);
    let pmode32 = offset(&raw const x86_leave_long_mode_pmode32);

    // SAFETY: the source is this image's own `.text`, the destination is a page the firmware just
    // granted exclusively, and the length was checked against the page above.
    unsafe { ptr::copy_nonoverlapping(start, base as *mut u8, len) };

    // The GDT pointer's base is the only field in the blob whose correct value depends on where the
    // copy landed. It sits two bytes into the descriptor, after the 16-bit limit, and is therefore
    // unaligned by construction: `lgdt` does not care, and `write_unaligned` is why this is not a
    // fault on the first boot.
    // SAFETY: `gdtr` is inside the page just written, and the 8 bytes at `gdtr + 2` are the base
    // field `leave_long_mode.s` reserved with `.quad 0`.
    unsafe { ptr::write_unaligned((gdtr as usize + 2) as *mut u64, gdt) };

    Ok(Trampoline {
        // SAFETY: `base` now holds the trampoline's first instruction, and the signature is the
        // register contract leave_long_mode.s documents.
        enter: unsafe {
            core::mem::transmute::<u64, extern "sysv64" fn(u64, u64, u64, u64) -> !>(base)
        },
        gdtr,
        pmode32,
    })
}

/// **Where the ACPI root pointer comes from on a real machine.**
///
/// The 2.0 entry is preferred because its RSDP is revision 2 or later and carries the 64-bit
/// `xsdt_address`; the 1.0 entry is the fallback, and `machine_discovery::acpi` reads whichever
/// root the revision says is there. This is what makes a UEFI boot hand the kernel a **non-zero**
/// `rsdp`, where QEMU's PVH loader leaves it zero and the kernel falls back to scanning the BIOS
/// area for `"RSD PTR "`.
fn find_rsdp(table: &SystemTable) -> Option<u64> {
    let mut fallback = None;
    for i in 0..table.number_of_table_entries {
        // SAFETY: the firmware promises `number_of_table_entries` valid entries at
        // `configuration_table`.
        let entry = unsafe { *table.configuration_table.add(i) };
        if entry.vendor_guid == efi::ACPI_20_TABLE_GUID {
            return Some(entry.vendor_table as u64);
        }
        if entry.vendor_guid == efi::ACPI_10_TABLE_GUID {
            fallback = Some(entry.vendor_table as u64);
        }
    }
    fallback
}

/// Print one ASCII line on the firmware console, if there is one.
///
/// It exists for the person standing at the `OptiPlex`: that machine's serial port carries the
/// kernel's output but not the firmware's, so without this a loader that failed before
/// `ExitBootServices` would look exactly like a stick the firmware never read. Non-ASCII input is
/// not this loader's problem; every string it prints is a literal in this file.
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
///
/// `hlt` rather than a bare spin for the reason CLAUDE.md gives about `wfi`: a halted core should
/// not burn a real one. Under QEMU this is the difference between 0% and 100% of a host thread.
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {
        // SAFETY: `hlt` at CPL 0, with no effect other than parking this core until an interrupt.
        unsafe { core::arch::asm!("hlt", options(nomem, nostack)) };
    }
}
