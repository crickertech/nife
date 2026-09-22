//! **aarch64: the firmware's device tree, and the Linux arm64 boot state.**
//!
//! The kernel's entry contract here is the one U-Boot's `booti` meets (Linux's
//! Documentation/arch/arm64/booting.rst, which `kernel/src/arch/aarch64/boot.s` cites): entered at
//! the image's first byte with **the MMU off**, `x0` = the physical address of the device tree and
//! `x1`..`x3` zero, at EL2 or EL1 (the kernel drops itself from EL2). UEFI hands over the opposite
//! state: MMU on with an identity map, caches on. So the whole delta is:
//!
//! 1. copy the firmware's device tree, adding `/chosen/linux,initrd-*` for the archive
//!    (`uefi_loader::device_tree_patch`);
//! 2. exit boot services;
//! 3. clean and invalidate the data cache to the point of coherency over everything the kernel will
//!    read with its caches off, so it reads what was written rather than stale RAM;
//! 4. turn the MMU and the data cache off at whichever exception level we are running at, and
//!    branch (`leave_uefi.s`).
//!
//! # BUGS
//!
//! - **The device tree must be offered by the firmware.** EDK2 on QEMU `virt` offers it only with
//!   `acpi=off`; an aarch64 machine that describes itself only with ACPI (most servers) cannot boot
//!   this kernel at all, which is a kernel limit rather than a loader one.
//! - **The kernel is linked for `0x4008_0000`** (`kernel/link-aarch64.ld`), which is QEMU `virt`'s
//!   RAM. argon's RAM starts at `0x8000_0000`, so this payload cannot place its kernel there and
//!   says so with the firmware's memory map printed: the same limit milestone 127's port has to
//!   lift for `booti`, not one the stick adds.
//! - **Everything the kernel reads early must be below 2 GiB** ([`ALLOCATION_CEILING`]), because
//!   the boot map in `boot.s` is one 1 GiB block of RAM at `0x4000_0000`. A firmware that keeps its
//!   device tree higher is fine (it is copied down); one with no free RAM below 2 GiB is not.

use core::ptr;

use uefi_loader::efi::{self, BootServices, Handle, SystemTable};

use crate::{Placed, configuration_table, copy_device_tree, exit_boot_services, say};

core::arch::global_asm!(include_str!("leave_uefi.s"));

unsafe extern "C" {
    /// `leave_uefi.s`: MMU and data cache off at the current EL, then `x0` = `device_tree`, branch.
    fn aarch64_leave_uefi(entry: u64, device_tree: u64) -> !;
}

/// **The ceiling on everything the kernel reads before its own page tables are up**: the device
/// tree and the archive. `boot.s`'s boot map covers RAM with a single 1 GiB block at
/// `0x4000_0000`, so the last byte it reaches is `0x7fff_ffff`.
pub const ALLOCATION_CEILING: u64 = 0x7fff_ffff;

/// The firmware's device tree, found before anything moves.
pub struct Found {
    device_tree: *const u8,
}

/// Find the device tree in the configuration table.
pub fn discover(table: &SystemTable, _services: &BootServices) -> Result<Found, &'static str> {
    configuration_table(table)
        .iter()
        .find(|entry| entry.vendor_guid == efi::DEVICE_TREE_GUID)
        .map(|entry| Found {
            device_tree: entry.vendor_table.cast(),
        })
        .ok_or(
            "the firmware offers no device tree (on QEMU virt, boot with acpi=off; \
             nife on aarch64 reads a device tree, not ACPI)",
        )
}

/// Clean and invalidate the data cache to the point of coherency over `[start, end)`, by virtual
/// address, which under UEFI's identity map is the physical one.
fn clean_to_coherency(start: u64, end: u64) {
    let ctr: u64;
    // SAFETY: reading the cache type register has no side effects.
    unsafe { core::arch::asm!("mrs {}, ctr_el0", out(reg) ctr, options(nomem, nostack)) };
    // DminLine, bits 19:16: log2 of the smallest data cache line, in words.
    let line = 4u64 << ((ctr >> 16) & 0xf);
    let mut at = start & !(line - 1);
    while at < end {
        // SAFETY: `dc civac` on an address the firmware mapped for us; it moves no data this
        // program can observe, only where the current copy lives.
        unsafe { core::arch::asm!("dc civac, {}", in(reg) at, options(nostack)) };
        at += line;
    }
    // SAFETY: a barrier.
    unsafe { core::arch::asm!("dsb sy", options(nostack)) };
}

/// Exit boot services and enter the kernel. Returns only on failure, with a console.
pub fn hand_over(
    handle: Handle,
    table: &SystemTable,
    services: &BootServices,
    found: Found,
    kernel: &Placed,
    module: Option<(u64, u64)>,
    // **Always `None` here**, and that is a recorded gap rather than an unused parameter.
    // `main.rs`'s `place_boot_file` reads this loader's own file so the running system can install
    // itself, and hands it over as a second PVH module. A device-tree handoff has `/chosen` with
    // one initrd and no second slot, so there is nowhere to put it; see that function's BUGS and
    // milestone 198 (a package manager, and the trivial install that makes a second customer
    // possible)'s rung 2a, which is an x86_64 claim for this reason.
    _boot_file: Option<(u64, u64)>,
    /// **Always `None` here**, and that is a scope note (DECISIONS §19 (architectural parity is a
    /// gate, not an aspiration)) rather than an unused parameter. The chooser that would set it is
    /// `uefi_loader::chooser`, which is x86_64 only for the same reason the install offer is: an
    /// installed machine on this architecture is what rung 2b of milestone 198 built, and the
    /// device-tree architectures have no installed disk to choose slots on yet. When they do, this
    /// parameter is where the number arrives.
    _from_slot: Option<u8>,
) -> Result<(), &'static str> {
    if kernel.end > ALLOCATION_CEILING + 1 {
        return Err("the kernel is linked above what its own boot map reaches");
    }
    let device_tree = copy_device_tree(services, found.device_tree, module)?;
    say(
        table,
        "uefi_loader: kernel placed, exiting boot services\r\n",
    );
    exit_boot_services(handle, services)?;

    // --- The firmware is gone. No console from here. ---
    clean_to_coherency(kernel.start, kernel.end);
    if let Some((start, len)) = module {
        clean_to_coherency(start, start + len);
    }
    // The tree's own header says how long it is; the copy was written with that length.
    // SAFETY: `device_tree` is the copy made above and its header was written by us.
    let len = unsafe { u32::from_be(ptr::read_unaligned((device_tree + 4) as *const u32)) } as u64;
    clean_to_coherency(device_tree, device_tree + len);

    // SAFETY: everything the kernel reads is placed, cleaned to coherency, and below the ceiling;
    // `leave_uefi.s` documents the state it leaves, which is the arm64 boot protocol's.
    unsafe { aarch64_leave_uefi(kernel.entry, device_tree) }
}

/// Park the core with `wfi`, which QEMU turns into a real host-thread sleep (AGENTS.md).
pub fn park() -> ! {
    loop {
        // SAFETY: waits for an interrupt; no other effect.
        unsafe { core::arch::asm!("wfi", options(nomem, nostack)) };
    }
}
