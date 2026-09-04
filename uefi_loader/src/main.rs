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
//! - **AP bring-up under UEFI is proved on OVMF and nowhere else** (milestone 195). This loader now
//!   asks the firmware for physical `0x8000` by name, and `cargo xtask uefi-test` boots two cores
//!   through OVMF and asserts both come online. What that does not establish is any *other*
//!   firmware's low-memory habits: on a machine that refuses the page this loader says so and boots
//!   one core, which is a report rather than a fix.
//! - **The `hvm_start_info` command line is empty.** PVH carries one and the kernel ignores it, so
//!   there is nowhere yet for a boot argument to come from or go to.

#![no_std]
#![no_main]

use core::ptr;

use machine_discovery::framebuffer::{Framebuffer, PixelOrder};
use uefi_loader::efi::{
    self, ALLOCATE_ADDRESS, ALLOCATE_MAX_ADDRESS, BootServices, Handle, SUCCESS, Status,
    SystemTable, memory_type,
};
use uefi_loader::handoff::{
    MEMMAP_ENTRY_LEN, MODULE_ENTRY_LEN, START_INFO_LEN, StartInfo, e820_kind, encode_memmap_entry,
    encode_module,
};
use uefi_loader::image;

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

/// **The page a STARTUP IPI can name**, which the kernel's AP bring-up copies its real-mode
/// trampoline into.
///
/// It is `AP_TRAMPOLINE_PHYS` in `kernel/link-x86_64.ld` and `.ap_trampoline`'s link address in the
/// kernel image, so the two files name one number. It is not part of the kernel's `p_paddr` span
/// (that section is linked low and *loaded* beside `.rodata`), which is why the span allocation
/// above does not cover it and this loader has to ask for it separately.
const AP_TRAMPOLINE_PHYS: u64 = 0x8000;

/// Where the boot command line sits inside page 0 of the handoff block.
///
/// 128 rather than 88 (the `hvm_start_info` plus the one-entry module list) so that a field added
/// to either does not silently start overwriting the command line; the page has four kilobytes and
/// there is nothing else to spend them on.
const CMDLINE_OFFSET: u64 = 128;

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

    // --- 1b. The screen, which is milestone 243's whole reason for touching this file ---
    //
    // Asked here for the same reason the RSDP is: it is a read of something that already exists,
    // nothing later can invalidate it, and the answer has to be in hand before the console this
    // sentence would be printed on goes away. **A machine with no screen is not an error**: the
    // OptiPlex with its serial module is that machine, and it boots exactly as it did before.
    let screen = find_screen(services);
    match screen {
        Some(found) => {
            say_span(
                table,
                "uefi_loader: screen at ",
                found.base,
                found.base + found.span().unwrap_or(0) as u64,
            );
            say(table, "uefi_loader:   ");
            say_decimal(table, found.width);
            say(table, "x");
            say_decimal(table, found.height);
            say(table, ", stride ");
            say_decimal(table, found.stride);
            say(table, ", ");
            say(table, found.order.token());
            say(table, "\r\n");
        }
        None => say(
            table,
            "uefi_loader: no linear framebuffer; the kernel will have only a UART\r\n",
        ),
    }

    // --- 2. Place the kernel at the physical addresses its linker script chose ---
    // `elf::Elf::parse` validates everything before this loader moves a byte: bounds, overlap,
    // W^X, the entry point inside an executable segment. Until milestone 208 the kernel image could
    // not pass it (an RWX boot section), which is why this module used to carry its own reader.
    let kernel = image::parse(embedded::KERNEL)?;
    let (span_start, span_end) = image::physical_span(kernel.segments(), PAGE)
        .ok_or("the embedded kernel has no loadable segments")?;

    // `AllocateAddress`, not `AllocateMaxAddress`: the kernel is linked for exactly this range
    // (`kernel/link-x86_64.ld`'s `PHYS_START`, 32 MiB since milestone 195) and there is nowhere
    // else to put it. A firmware that has something of its own there fails HERE, with the range and
    // the offending descriptors printed, rather than by silently overwriting whatever it was.
    let mut kernel_base = span_start;
    let kernel_pages = ((span_end - span_start) / PAGE) as usize;
    if (services.allocate_pages)(
        ALLOCATE_ADDRESS,
        memory_type::LOADER_DATA,
        kernel_pages,
        &mut kernel_base,
    ) != SUCCESS
    {
        // **Name what is in the way before giving up** (milestone 195). "The firmware said no" is
        // the least actionable sentence a person standing at a machine can be handed, and this is
        // the one failure here whose cause is entirely inside the firmware's own bookkeeping: the
        // kernel's physical span is fixed by `kernel/link-x86_64.ld`, so what changes between one
        // build and the next is only how far past 1 MiB it reaches. Under OVMF the kernel's test
        // build reaches into the firmware volumes at 8 MiB and the tour build does not, which is a
        // fact no amount of staring at the loader would have produced.
        say_span(table, "uefi_loader: wanted ", span_start, span_end);
        say_conflict(table, services, span_start, span_end);
        return Err("the firmware would not give up the kernel's load range");
    }

    // Zero the whole span before copying anything, which is what makes every `.bss` and every
    // `NOLOAD` section (the boot page tables among them) arrive zeroed. `boot.s` zeroes its own
    // tables anyway; the two agreeing is cheaper than either one being the only guard.
    // SAFETY: the firmware just granted this whole range exclusively.
    unsafe { ptr::write_bytes(span_start as *mut u8, 0, (span_end - span_start) as usize) };

    // `Segment::data` is `p_filesz` bytes, which is empty for a `NOLOAD` section: `.boot_scratch`
    // (the boot page tables) and the two per-CPU stack areas arrive that way, as address space to
    // reserve rather than bytes to copy. The zeroing above is what makes their memory correct.
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

    // --- 2b. The page the kernel's AP bring-up needs, asked for by name ---
    //
    // A STARTUP IPI names a physical PAGE below 1 MiB (`vector << 12`), so `arch::x86_64::ap_boot`
    // copies its real-mode trampoline to a fixed low address that `kernel/link-x86_64.ld` picks at
    // link time (`AP_TRAMPOLINE_PHYS`). Until milestone 195 this loader never mentioned that page,
    // and secondary cores under firmware therefore worked or did not by luck: OVMF happens to leave
    // the first 640 KiB conventional, and a firmware that did not would have been discovered by a
    // core that started executing something else's bytes in real mode.
    //
    // **A refusal is not fatal, and that asymmetry is deliberate.** A single-core boot on a machine
    // whose firmware wants this page is far more useful than no boot at all, and the kernel brings
    // up secondaries only when it is asked to. So this says what it found and carries on; the
    // person at the bench gets the one line that explains why `smp: 1 core(s) online` on a machine
    // with eight.
    let mut ap_page = AP_TRAMPOLINE_PHYS;
    if (services.allocate_pages)(ALLOCATE_ADDRESS, memory_type::LOADER_DATA, 1, &mut ap_page)
        != SUCCESS
    {
        say_span(
            table,
            "uefi_loader: WARNING no AP trampoline page at ",
            AP_TRAMPOLINE_PHYS,
            AP_TRAMPOLINE_PHYS + PAGE,
        );
        say_conflict(
            table,
            services,
            AP_TRAMPOLINE_PHYS,
            AP_TRAMPOLINE_PHYS + PAGE,
        );
        say(
            table,
            "uefi_loader: secondary cores will not be brought up\r\n",
        );
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
    // **The boot command line lives in the same page**, after the 56-byte structure and the 32-byte
    // module list, which together end at 88. It is at most `Framebuffer::MAX_LEN` plus a NUL and
    // there are four kilobytes here, so it costs no allocation and adds no failure path: one more
    // `AllocatePages` for sixty bytes would be one more thing that can be refused on somebody's
    // firmware, in the middle of the one sequence in this loader whose order is rigid.
    let cmdline_at = handoff_base + CMDLINE_OFFSET;
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
    if kernel.entry() >= BELOW_4G || start_info_at >= BELOW_4G {
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

    // The command line, NUL-terminated, in the page reserved for it above. Written after
    // `ExitBootServices` like everything else in this section: it names memory this loader already
    // owns and needs no firmware call, so there is nothing here that could invalidate the map key.
    let cmdline = screen.map_or(0, |found| {
        let mut token = [0u8; Framebuffer::MAX_LEN + 1];
        let n = found.encode(&mut token);
        // SAFETY: `CMDLINE_OFFSET + MAX_LEN + 1` is inside page 0 of the handoff block, which was
        // allocated above and whose first 88 bytes are the structure and the module list.
        unsafe { ptr::copy_nonoverlapping(token.as_ptr(), cmdline_at as *mut u8, n + 1) };
        cmdline_at
    });

    let info = StartInfo {
        rsdp,
        modules: if module.is_some() { module_list_at } else { 0 },
        module_count: u32::from(module.is_some()),
        memmap: memmap_at,
        memmap_entries: written as u32,
        cmdline,
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
        kernel.entry(),
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

/// **Ask the firmware where the screen is** (milestone 243).
///
/// One `LocateProtocol` call and three field reads. It deliberately does **not** set a video mode:
/// the firmware has already chosen one that works on this monitor, and a mode set here would be a
/// mode the kernel has to be told about through the same channel anyway.
///
/// `None` on every unhappy answer, and they are all the same answer to the caller: no protocol, a
/// null `mode` or `info`, a `PixelBltOnly` adapter (which has no linear framebuffer at all, only a
/// boot-services `Blt` call that is gone by the time the kernel runs), a `PixelBitMask` adapter
/// (whose channels are described by masks `machine_discovery::framebuffer` cannot express), or a
/// geometry whose arithmetic does not close. A machine with no screen is the OptiPlex, and it is
/// not an error.
fn find_screen(services: &BootServices) -> Option<Framebuffer> {
    let mut interface: *mut core::ffi::c_void = ptr::null_mut();
    if (services.locate_protocol)(
        &efi::GRAPHICS_OUTPUT_PROTOCOL_GUID,
        ptr::null_mut(),
        &mut interface,
    ) != SUCCESS
        || interface.is_null()
    {
        return None;
    }
    // SAFETY: `LocateProtocol` returned success, so the firmware asserts this is an
    // `EFI_GRAPHICS_OUTPUT_PROTOCOL` and that it outlives boot services.
    let gop = unsafe { &*interface.cast::<efi::GraphicsOutput>() };
    if gop.mode.is_null() {
        return None;
    }
    // SAFETY: as above; `Mode` is a required field of the protocol and non-null was just checked.
    let mode = unsafe { &*gop.mode };
    if mode.info.is_null() {
        return None;
    }
    // SAFETY: as above. Read unaligned is not needed: the firmware allocates this structure and the
    // specification gives it natural alignment.
    let info = unsafe { &*mode.info };
    let order = match info.pixel_format {
        efi::pixel_format::BGRX => PixelOrder::Bgrx,
        efi::pixel_format::RGBX => PixelOrder::Rgbx,
        _ => return None,
    };
    let found = Framebuffer {
        base: mode.framebuffer_base,
        width: info.horizontal_resolution,
        height: info.vertical_resolution,
        // The firmware reports the stride in PIXELS and every consumer of it wants bytes. This is
        // the one multiplication in the whole path and getting it wrong shears the picture.
        stride: info.pixels_per_scan_line.checked_mul(4)?,
        order,
    };
    // The kernel's console indexes this with a `usize` computed from the geometry, so a geometry
    // whose span does not close is refused here rather than trusted there.
    found.span()?;
    // And the aperture the firmware reports has to actually hold what the geometry claims. A
    // firmware that reported a stride larger than its own framebuffer would have this loader
    // handing the kernel a licence to write past the end of a BAR.
    if found.span()? > mode.framebuffer_size {
        return None;
    }
    Some(found)
}

/// Print one unsigned value in decimal, without an allocator and without `core::fmt`.
///
/// Its own function for the same reason [`say_hex`] is: a screen's geometry is the one thing here a
/// person reads as a number rather than as an address, and `800x600` in hex helps nobody.
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
