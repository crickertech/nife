//! **The offer a booted stick makes, and the two programs that carry it out** (milestone 198 (a
//! package manager, and the trivial install that makes a second customer possible), rung 2a).
//!
//! The proposal it follows is milestone 515 (a stick that puts itself on the machine's disk).
//!
//! # What this module is, and what it deliberately is not
//!
//! It is the **boot policy**: decide whether this boot could install at all, say what would be
//! destroyed, ask, and then wire the two confined programs that do the work. It is not the
//! installer. Nothing here writes a byte to a disk.
//!
//! The split is the whole argument, and it is the one `disk_service` already makes for milestone
//! 57's partitioner:
//!
//! | | holds | can destroy |
//! |---|---|---|
//! | this module | the boot path, the console | nothing; it has no block endpoint of its own to spend |
//! | `components/src/installer.rs` | one disk, one entropy endpoint, a read-only copy of the boot file | that disk |
//! | `redoxfs_server`'s `mkfs` | the same disk, the same entropy endpoint | that disk |
//!
//! **The question is asked here rather than in the installer**, and that is not an accident of
//! where the console is. A confirmation a program prints to itself is a confirmation that program
//! could also decide to skip; asking before the capability is granted means the authority to wipe
//! the disk does not exist until the answer does.
//!
//! # When the offer is made
//!
//! Three things must be true, and all three are facts about the machine rather than preferences:
//!
//! 1. **This boot came from a file the loader could read back** (`memory::boot_file_region`), so
//!    there is something to install. A `-kernel` boot has no such file and is silently skipped.
//! 2. **There is an NVMe controller**, which is the disk an install would go to.
//! 3. **That disk does not already carry nife.** An installed machine boots from a file too, so
//!    without this check every boot of every installed machine would pause to offer to wipe itself.
//!    The check is a survey process, not a parse in the kernel: `installer`'s `ROLE_SURVEY`, spawned
//!    with the disk and **no entropy endpoint**, so it cannot write a table anything would read
//!    back. That is `disk_partitioner`'s verify role, one milestone along.
//! 4. **Somebody answers.** The wait is bounded, so a stick booted on a machine with no console
//!    reaches the prompt instead of hanging.
//!
//! Anything else is a refusal, and every refusal is silent-or-explained and then continues to the
//! ordinary boot. **Refusing by default is the whole safety property**: the only path to a write is
//! a person typing [`CONFIRMATION`].
//!
//! # EXAMPLES
//!
//! ```text
//!   install     : this system was booted from a file and can install itself.
//!   install     :   TARGET: the NVMe disk attached to this machine, 1073741824 bytes.
//!   install     :   EVERYTHING ON THAT DISK WILL BE DESTROYED.
//!   install     :   Type INSTALL and press return to proceed; anything else continues the boot.
//!   install     : > INSTALL
//!   install     : partitioning and copying the boot file...
//!   install     : installed. nife data at LBA 2048, EFI system at LBA 1046528.
//!   install     : filesystem created.
//!   install     : DONE. Remove the installation medium and reboot.
//! ```
//!
//! # BUGS
//!
//! - **x86_64 only**, because `memory::boot_file_region` is: a device-tree handoff has one initrd
//!   slot in `/chosen` and no second one for the loader's own file. `uefi_loader`'s
//!   `place_boot_file` carries the argument and the gap.
//! - **It asks on the console, which means the kernel reads the UART** (`console::read_line`).
//!   That is only sound because this runs before the progenitor hands the device to the input
//!   driver, and it is the reason the offer is made at this exact point in the boot and not later.
//! - **The disk is named by what it is, not by model and serial.** Milestone 515's `BUGS` asks for
//!   model and size, and only the size is here: reading a controller's model is a second IDENTIFY
//!   (`CNS_CONTROLLER`) in the admin plane and there is nowhere in
//!   `non_volatile_memory_express::Handoff` to put forty ASCII bytes, whose three scalars are all
//!   spent. On a machine with two disks this sentence would not say which, and that is why the
//!   offer says "the NVMe disk attached to this machine" rather than naming an ordinal it cannot
//!   back up. **Naming the disk properly wants a lane**; it is the single most load-bearing
//!   sentence a stranger reads.
//! - **Nothing here is crash-atomic**, and a power cut between the partitioner and `mkfs` leaves a
//!   partitioned disk with no filesystem, which the next boot reports as a disk it cannot mount.
//! - **It checks for nife and for nothing else.** A disk carrying Windows, or a Linux root, looks
//!   exactly like an empty one from here: the survey asks "is there a nife data partition" and any
//!   other answer is "installable". A person who says yes to the question loses that disk. Naming
//!   what is already on it is `disk_surveyor`'s job and it would be a good next step.
//! - **A machine that already carries nife is never offered an install again**, which is what stops
//!   an installed system offering to wipe itself on every boot and also means there is no way to
//!   reinstall from the stick. The honest fix is a second question ("nife is already on this disk;
//!   replace it?") rather than a second mechanism, and it is not written.

use super::*;
use crate::cap::{Rights, memory_region_cap, page_frame_cap, rendezvous_cap};
use crate::sched::RendezvousId;

/// **The word a person types to destroy a disk.** Upper case and unabbreviated, so that a stray
/// newline, a `y`, or a terminal echoing something cannot be it.
pub const CONFIRMATION: &str = "INSTALL";

/// How long the offer waits for an answer before continuing the boot.
///
/// Long enough that somebody who has just watched a machine come up can read three lines and type
/// seven characters; short enough that a stick left in a machine in a cupboard still reaches a
/// prompt. It is a duration measured against the counter, not a spin count.
const PATIENCE: core::time::Duration = core::time::Duration::from_secs(30);

/// Where the kernel maps the boot file, read-only, into the installer. **Must match
/// `components/src/installer.rs`'s `BOOT_FILE_VA`**; the bytes are a run of physical frames the
/// kernel owns, so the program cannot map them itself.
const BOOT_FILE_VA: u64 = 0x1000_0000;

/// The installer's capability table. Must match `components/src/installer.rs`.
const SLOT_REPORT: u64 = 0;
const SLOT_BLK: u64 = 1;
const SLOT_ENTROPY: u64 = 2;
const SLOT_BUDGET: u64 = 3;
const SLOT_BLK_PAGE: u64 = 4;

/// The untyped the installer spends on the page tables for the one page it maps itself. The same
/// figure `disk_service` gives the partitioner, and for the same reason: it maps one page.
const INSTALLER_BUDGET_PAGES: u64 = 64;

/// Extra stack pages for the installer, below the single page `run` maps.
///
/// It keeps a 16 KiB entry array in `.bss` rather than on the stack and recurses nowhere, so this
/// is the partitioner's number rather than a measurement of its own; the deepest thing it does is
/// `GloballyUniqueIdentifierPartitionTable::create`.
const INSTALLER_STACK_PAGES: usize = 8;

/// The report's first word, from `components/src/installer.rs`. Duplicated here rather than shared
/// through a crate because it is the kernel's own convention with one program it spawns, the same
/// terms `disk_service` and `disk_tests` already state for `R_PARTITIONED` and `R_MADE`.
const R_INSTALLED: u64 = 0x_49_4E_53_54_44;

/// `mkfs`'s create role, from `redoxfs_server/src/bin/mkfs.rs`. Spelled here for the same reason
/// `disk_tests` spells it: the kernel's convention with a program it spawns.
const ROLE_MAKE: u64 = 0;
/// `mkfs`'s "a filesystem was created and a file written into it" verdict, ASCII `MKFSD`.
const R_MADE: u64 = 0x_4D_4B_46_53_44;

// The installer's roles, in `a0`. Must match `components/src/installer.rs`.
const ROLE_INSTALL: u64 = 0;
const ROLE_SURVEY: u64 = 1;
/// The survey's "this disk already carries a nife data partition" verdict, ASCII `HAVIT`.
const R_ALREADY: u64 = 0x_48_41_56_49_54;

/// **Offer to install, and carry it out if a person says so.** Returns after either, so the boot
/// continues in every case.
///
/// The three programs it needs are named rather than passed, because a caller on the boot path
/// has no business knowing which binaries an install is made of.
pub fn offer() {
    let Some((_, boot_file_len)) = crate::memory::boot_file_region() else {
        return; // Not booted from a file: nothing to install.
    };
    let Some(installer) = program("installer") else {
        crate::println!(
            "  install     : this boot came from a file, but the archive has no installer."
        );
        return;
    };
    let Some(nvme) = program("non_volatile_memory_express") else {
        return;
    };
    let Some(disk) = non_volatile_memory_express_service::ensure(nvme) else {
        return; // No NVMe controller: nothing to install onto.
    };
    disk.wait_for_ready();

    // **Before the question, and by a process that cannot write.** An installed machine boots from
    // a file too, so an offer that did not look at the disk first would ask every installed machine
    // to wipe itself, once per boot.
    let report = spawn_installer(installer, &disk, None, ROLE_SURVEY, 0);
    if crate::sched::ipc_recv(report)[0] == R_ALREADY {
        return; // nife is already on this disk. See BUGS: there is no reinstall.
    }

    if !asked(disk.size_bytes) {
        crate::println!("  install     : not confirmed; continuing the boot.");
        return;
    }

    // Entropy from the instruction rather than from a device: `RDSEED` is on every x86_64 part a
    // stranger would install this on, and needing no virtio-rng is what lets the offer work on a
    // machine that has only a disk. A machine without it gets `None` here and the installer's own
    // refusal below, which is the honest answer rather than a GPT full of invented ids.
    let entropy = program("entropy")
        .and_then(|image| entropy_service::ensure(image, entropy_service::Bus::Instruction))
        .map(|w| {
            w.wait_for_ready();
            w.request
        });
    let Some(entropy) = entropy else {
        crate::println!(
            "  install     : this machine has no source of randomness, so a partition table \
             written here could not carry unique ids. Nothing was written."
        );
        return;
    };

    crate::println!("  install     : partitioning and copying the boot file...");
    let report = spawn_installer(installer, &disk, Some(entropy), ROLE_INSTALL, boot_file_len);
    let answer = crate::sched::ipc_recv(report);
    if answer[0] != R_INSTALLED {
        crate::println!(
            "  install     : FAILED ({:#x}, {}, {}). The disk may be in any state.",
            answer[0],
            answer[1],
            answer[2]
        );
        return;
    }
    crate::println!(
        "  install     : installed. nife data at LBA {}, EFI system at LBA {}.",
        answer[1],
        answer[2]
    );

    // The filesystem, by the program that already knows how to make one inside a partition it finds
    // by type GUID. It is `mkfs` unmodified, on the same disk and the same entropy endpoint.
    let Some(maker) = program("mkfs") else {
        crate::println!("  install     : the archive has no mkfs, so the data partition is empty.");
        return;
    };
    let target = disk_service::BlankDisk {
        blk_ep: disk.request,
        blk_shared: disk.transfer_phys,
        blk_ready: None,
    };
    let report = disk_service::start_maker(maker, &target, ROLE_MAKE, Some(entropy), true);
    let answer = crate::sched::ipc_recv(report);
    if answer[0] != R_MADE {
        crate::println!(
            "  install     : the partition table is written but mkfs refused ({:#x}).",
            answer[0]
        );
        return;
    }
    crate::println!("  install     : filesystem created.");
    crate::println!("  install     : DONE. Remove the installation medium and reboot.");
}

/// Print what is about to be destroyed and read the answer. `true` only for exactly
/// [`CONFIRMATION`].
fn asked(size_bytes: u64) -> bool {
    crate::println!("  install     : this system was booted from a file and can install itself.");
    crate::println!(
        "  install     :   TARGET: the NVMe disk attached to this machine, {size_bytes} bytes."
    );
    crate::println!("  install     :   EVERYTHING ON THAT DISK WILL BE DESTROYED.");
    crate::println!(
        "  install     :   Type {CONFIRMATION} and press return to proceed; anything else \
         continues the boot."
    );
    crate::print!("  install     : > ");

    let mut typed = [0u8; 32];
    let Some(n) = crate::console::read_line(&mut typed, PATIENCE) else {
        crate::println!();
        return false;
    };
    &typed[..n] == CONFIRMATION.as_bytes()
}

/// Spawn the installer with the four things it needs and nothing else: the disk, the randomness,
/// the page it shares with the disk's server, and a read-only copy of the file this machine booted
/// from.
fn spawn_installer(
    image: &'static [u8],
    disk: &non_volatile_memory_express_service::Wiring,
    entropy: Option<RendezvousId>,
    role: u64,
    boot_file_len: u64,
) -> RendezvousId {
    let report = crate::sched::create_rendezvous();
    let budget =
        crate::memory_region::create(INSTALLER_BUDGET_PAGES).expect("no budget for the installer");
    let blk_ep = disk.request;
    let blk_shared = disk.transfer_phys;
    // Read only when `entropy` is `Some`; copied out here so the closure captures a plain `u64`,
    // the same shape `disk_service::start_partitioner` uses for the same reason.
    let ep = entropy.unwrap_or_default();

    crate::sched::spawn(move || {
        let maps = installer_maps(boot_file_len);
        crate::sched::grant_at(SLOT_REPORT, rendezvous_cap(report, Rights::WRITE))
            .expect("installer slot 0 was occupied");
        crate::sched::grant_at(SLOT_BLK, rendezvous_cap(blk_ep, Rights::WRITE))
            .expect("installer slot 1 was occupied");
        // **Slot 2 is the difference between the two roles and nothing else is.** A survey holds
        // the same disk and cannot write a table anything would read back, because a partition
        // table carries unique ids and this process has no way to draw one.
        if entropy.is_some() {
            crate::sched::grant_at(SLOT_ENTROPY, rendezvous_cap(ep, Rights::WRITE))
                .expect("installer slot 2 was occupied");
        }
        crate::sched::grant_at(SLOT_BUDGET, memory_region_cap(budget))
            .expect("installer slot 3 was occupied");
        crate::sched::grant_at(
            SLOT_BLK_PAGE,
            page_frame_cap(blk_shared, Rights::READ.union(Rights::WRITE)),
        )
        .expect("installer slot 4 was occupied");
        run(
            image,
            Spawn {
                arg0: role,
                arg1: boot_file_len,
                arg2: 0,
                grants: &[], // every one of them is placed above, at its own slot
                maps,
            },
        )
    })
    .expect("could not spawn the installer");

    report
}

/// **Build the installer's extra mappings**: its stack, then the boot file read-only, one entry per
/// page.
///
/// The boot file is about ten megabytes, so this is a few thousand entries and they do not fit on a
/// stack or in `.bss` worth spending on a feature used once in the life of a machine. They go in
/// frames allocated for the purpose and are never freed, which costs about sixty kilobytes for the
/// rest of this boot and is the cheapest honest answer: the alternative is a `pages` field on
/// [`Mapping`], and that struct has ninety-six literals in this tree.
fn installer_maps(boot_file_len: u64) -> &'static [Mapping] {
    let boot_file_phys = crate::memory::boot_file_region().map_or(0, |(at, _)| at);
    let pages = boot_file_len.div_ceil(FRAME_SIZE) as usize;
    let total = INSTALLER_STACK_PAGES + pages;

    let bytes = total * core::mem::size_of::<Mapping>();
    let scratch = crate::memory::alloc_contiguous_zeroed(bytes.div_ceil(FRAME_SIZE as usize))
        .expect("no scratch for the installer's mappings")
        .addr();
    // SAFETY: `total` `Mapping`s fit in the run of frames just allocated, which nothing else holds;
    // the direct map reaches them, and `Mapping` is a plain `Copy` struct of integers, so the
    // zeroed frames are already a valid initialised slice of them.
    let maps: &'static mut [Mapping] = unsafe {
        core::slice::from_raw_parts_mut(mmu::phys_to_virt(scratch) as *mut Mapping, total)
    };

    for (k, m) in maps[..INSTALLER_STACK_PAGES].iter_mut().enumerate() {
        m.va = USER_STACK_VA - (k as u64 + 1) * FRAME_SIZE;
        m.phys = crate::memory::alloc()
            .expect("no frame for the installer's stack")
            .addr();
        m.flags = Flags::user_data();
    }
    for (i, m) in maps[INSTALLER_STACK_PAGES..].iter_mut().enumerate() {
        m.va = BOOT_FILE_VA + i as u64 * FRAME_SIZE;
        m.phys = boot_file_phys + i as u64 * FRAME_SIZE;
        // Read-only, and it is the whole of what keeps a program holding a disk from also being
        // able to rewrite the image it is about to install.
        m.flags = Flags::user_rodata();
    }
    maps
}
