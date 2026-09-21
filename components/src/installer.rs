//! **`installer`**: the program a booted stick runs to put this system on the machine's own disk
//! (milestone 198 (a package manager, and the trivial install that makes a second customer
//! possible), rung 2a).
//!
//! The proposal this follows is
//! [milestone 515 (a stick that puts itself on the machine's disk)](../../design/roadmap/515-the-installer-a-stick-runs-to-put-itself-on-the-disk.md),
//! under DECISIONS §157 (a trivial install is a web page, a USB drive, and packages).
//!
//! Name: provisional, minted 2026-09-21 by the rung 2a lane. `crates/stick_maker`'s own header
//! set the word aside for exactly this program when it refused it for itself: *"DECISIONS §157's
//! rung 2 installs nife onto a PC's own disk, and that is a different program."* calef names
//! programs; expect this to change.
//!
//! # This is the most destructive program in this tree, and its shape is the answer to that
//!
//! It replaces a disk's partition table and everything the table described. What stands between a
//! stranger and a wiped disk is **not** this program's judgement, and that is the point:
//!
//! - It holds **one disk**, as a `filesystem_protocol::blk` endpoint, and there is no path to type.
//!   `parted /dev/sda` as root reaches every disk in the machine; this reaches the one it was
//!   handed. That is `disk_partitioner`'s claim, from milestone 57 (partitioning and formatting a
//!   real drive), and this program inherits it whole.
//! - It holds an **entropy endpoint** and nothing else besides, so a GPT's unique ids are drawn
//!   rather than invented, and a run with no entropy writes nothing at all.
//! - **It is not spawned until a person has answered a question naming the disk.** The asking is
//!   `kernel/src/user/install_service.rs`'s, deliberately: a confirmation this program printed to
//!   itself would be a confirmation it could also decide to skip, and the authority to wipe the
//!   disk arrives here only after the answer.
//!
//! # Two roles, and the second is what keeps the first honest
//!
//! | `a0` | |
//! |---|---|
//! | [`ROLE_SURVEY`] | read the table back and say whether this disk already carries nife |
//! | [`ROLE_INSTALL`] | draw the ids, write the table, lay out the EFI system partition, copy the boot file |
//!
//! `ROLE_SURVEY` is **a separate process with no entropy endpoint**, exactly as
//! `disk_partitioner`'s verify role is, and for the same reason: a program that cannot draw a
//! unique id cannot write a partition table anything would read back. It is what lets the boot ask
//! "is nife already on this disk" without the kernel parsing a partition table and without granting
//! the authority to write one.
//!
//! # What it writes, in order
//!
//! | | |
//! |---|---|
//! | 1 | A GPT: a **nife data partition first**, then an EFI system partition at the end of the disk |
//! | 2 | A FAT32 volume in the EFI system partition (`crates/file_allocation_table`) |
//! | 3 | The boot file, into that volume, at `\EFI\BOOT\BOOTX64.EFI` |
//!
//! It does **not** create the filesystem in the data partition. `redoxfs_server`'s `mkfs` already
//! does exactly that, finding the partition by its type GUID, and it is run next by the same
//! service. Two programs with one capability each beats one program with two.
//!
//! ## Why the data partition comes first, which is the unusual half
//!
//! A GPT does not care about order, and every installer a reader has met puts the EFI system
//! partition first. This one does not, and the reason is a property of the filesystem engine rather
//! than a preference. `redoxfs`'s `FileSystem::open` scans blocks `0..65536` of whatever disk it is
//! handed for its header, and adopts the block it finds it at as the filesystem's origin. So a
//! RedoxFS that begins inside the first 256 MiB of a disk is mounted correctly **by a server that
//! was given the whole disk**, and the installed system's boot needs no partition-aware mount, no
//! base-block field on the `blk` wire, and no program in the middle.
//!
//! That is a real simplification and it is also a real authority loss, recorded in `BUGS` below:
//! the filesystem server on an installed machine can address the EFI system partition and the
//! partition table, which a partition-bounded mount would make impossible. `mkfs` shows what the
//! bounded version looks like (its `PartitionDisk`), and closing the gap is a lane of its own.
//!
//! # The boot file, and why it is not in the archive
//!
//! The bytes written at step 3 are a copy of the file this machine was booted from, mapped
//! read-only into this program by the kernel. It cannot come from the archive, because the file
//! *contains* the archive; `uefi_loader`'s `place_boot_file` reads it back off the boot volume
//! while the firmware is still up, and the whole argument is there.
//!
//! **The kernel and the archive therefore move as a set**, which is the failure this design had to
//! make unexpressible: a disk carrying a new kernel beside an archive it does not vouch for halts
//! at `MEASURED BOOT REFUSED`. They are sealed together inside the one file at build time, and this
//! program copies the file.
//!
//! # EXAMPLES
//!
//! ```text
//!   install: disk 1073741824 bytes, 2097152 blocks of 512
//!   install: nife data at 2048..1046527, EFI system at 1046528..2095103
//!   install: EFI system partition laid out, 2099 metadata sectors
//!   install: boot file 10392064 bytes written
//!   report: R_INSTALLED, data at 2048
//! ```
//!
//! # BUGS
//!
//! - **Whole disk only.** The table is replaced wholesale. Installing beside another operating
//!   system means resizing a filesystem nife cannot read, which is milestone 140 (mount a drive
//!   this system did not create)'s territory and a different order of risk to somebody's data.
//! - **The installed filesystem is not bounded by its partition**, for the reason the section above
//!   gives. A filesystem server on the installed machine holds the whole disk. What keeps it inside
//!   the partition today is that `mkfs` created it bounded, so its own allocator never learns about
//!   the blocks past the end; that is a property of the filesystem rather than a capability, which
//!   is the wrong rung of AGENTS.md's ladder and is recorded here rather than papered over.
//! - **The logical block size is assumed to be 512**, the same assumption and the same reason as
//!   `disk_surveyor` and `disk_partitioner`: nothing in `filesystem_protocol::blk` carries the
//!   device's. An NVMe namespace formatted with 4096-byte logical blocks would get a table no other
//!   operating system can read. The fix is a field on the wire.
//! - **Nothing here is crash-atomic.** A power cut partway through leaves a disk that boots nothing.
//!   Real installers share the property; nobody has measured this one.
//! - **There is no progress report during the copy.** Ten megabytes at one 4096-byte request per
//!   round trip is a few thousand calls, and a person watching a serial console sees nothing until
//!   it is done.
//! - **It writes no firmware boot entry.** The file goes at the removable-media fallback path
//!   `\EFI\BOOT\BOOTX64.EFI` on the new disk, which OVMF starts and which most firmware starts.
//!   Milestone 515 records this as B1 and calls it unmeasured on anything but OVMF; a firmware that
//!   refuses it needs `SetVariable`, which is a runtime service this kernel does not map.

#![no_std]
// Program entry points, not the crates/ library surface tracked by milestone 68 (code-quality
// gates: one lint policy)'s ratchet
// (DECISIONS §107 (`missing_docs` moves to `workspace.lints.rust`)).
#![allow(missing_docs)]
#![no_main]

use entropy_protocol as entropy;
use file_allocation_table as fat;
use filesystem_protocol::fixture::blank;
use filesystem_protocol::{blk, req};
use globally_unique_identifier_partition_table::entry::Entry;
use globally_unique_identifier_partition_table::guid::{Guid, types};
use globally_unique_identifier_partition_table::{
    ENTRY_ARRAY_BYTES, GloballyUniqueIdentifierPartitionTable, PRIMARY_HEADER_LBA,
};
use user_mode_runtime::{call, send};

/// Slot 0: where the verdict goes. An endpoint with `WRITE`.
const REPORT: u64 = 0;
/// Slot 1: the block service for the one disk this program was handed, `WRITE`.
const BLK: u64 = 1;
/// Slot 2: the entropy service, `WRITE`.
const ENTROPY: u64 = 2;
/// Slot 3: the untyped this program spends on the page tables its own mapping needs.
const BUDGET: u64 = 3;
/// Slot 4: the page shared with the block server, `READ|WRITE`.
const BLK_PAGE_FRAME: u64 = 4;

/// Where this program puts the page it shares with the block server. Its own choice
/// (milestone 108 (the drivers move onto frame capabilities)), so nothing on the kernel side
/// names this address.
const BLK_PAGE: u64 = 0x5000_0000;

/// **Where the kernel maps the boot file, read-only.** This one *is* named on the kernel side
/// (`kernel/src/user/install_service.rs`'s `BOOT_FILE_VA`) and the two must agree; the bytes are a
/// run of physical frames the kernel owns, so this program cannot map them itself.
const BOOT_FILE_VA: u64 = 0x1000_0000;

// The roles, in `a0`. Must match kernel/src/user/install_service.rs.
/// Write the table, the EFI system partition and the boot file.
pub const ROLE_INSTALL: u64 = 0;
/// **Read the table back and report what is on the disk.** No entropy endpoint, no writes.
pub const ROLE_SURVEY: u64 = 1;

/// The transfer unit of the block service: one filesystem block per request.
const TRANSFER: u64 = blk::BLOCK_SIZE as u64;

/// The logical block size a GPT counts in. See BUGS.
const LBA: u64 = blank::LBA;

/// Logical blocks per transfer block: eight 512-byte sectors in one 4096-byte request.
const SECTORS_PER_TRANSFER: u64 = TRANSFER / LBA;

/// **The alignment every partition starts on**, in logical blocks: 1 MiB, which is what every real
/// partitioning tool uses and what keeps a partition's first byte on an erase block of anything
/// this will ever be written to.
const ALIGN: u64 = 2048;

/// **How big the EFI system partition is**, in logical blocks: 512 MiB.
///
/// It is not the 100 MiB a reader expects, and the reason is arithmetic rather than generosity.
/// `crates/file_allocation_table` refuses to build a volume with fewer than 65525 data clusters,
/// because below that the volume is FAT16 whatever its boot sector claims; at 4096-byte clusters
/// that floor is a little over 256 MiB. 512 MiB clears it with room for a second boot file when
/// somebody builds the two-slot layout milestone 515 lists as L2.
const ESP_SECTORS: u64 = 512 * 1024 * 1024 / 512;

/// **The smallest nife data partition this program will leave behind.** A disk that cannot spare
/// this after the EFI system partition is refused, rather than installed onto and then found to be
/// full.
const DATA_MIN_SECTORS: u64 = 64 * 1024 * 1024 / 512;

/// How many logical blocks the backup table needs at the far end of the disk: the entry array plus
/// its header.
const BACKUP_BLOCKS: u64 = 33;

/// The names written into the two partition entries. Read back by nothing; a person running
/// `disk_surveyor` on the installed machine meets them.
const NAMES: [&str; 2] = ["nife data", "nife boot"];

/// The removable-media boot file's name on the EFI system partition. 8.3, which is what
/// `file_allocation_table` will write; see that crate's BUGS for the riscv64 name that is not.
const BOOT_FILE_NAME: &str = "BOOTX64.EFI";

// The report's first word. Must match kernel/src/user/install_service.rs.
/// The disk was partitioned, the EFI system partition laid out, and the boot file written. ASCII
/// `INSTD`.
pub const R_INSTALLED: u64 = 0x_49_4E_53_54_44;
/// **No entropy endpoint, so nothing was written.** The same refusal `disk_partitioner` makes, for
/// the same reason: an id that is not random is not unique, and a made-up one is worse than none
/// because it looks right (DECISIONS §42 (a filesystem declares what it offers and must be
/// truthful)).
pub const R_NO_ENTROPY: u64 = 0x_4E_4F_52_4E_47;
/// The disk is too small to hold the layout. The second word is its size in bytes.
pub const R_TOO_SMALL: u64 = 0x_53_4D_41_4C_4C;
/// The disk refused a read or a write, or a layout step failed. The second word says which step.
pub const R_DISK_FAILED: u64 = 0x_44_49_53_4B_45;
/// There is no boot file to copy, so an install would produce a disk that boots nothing.
pub const R_NO_BOOT_FILE: u64 = 0x_4E_4F_42_4F_54;

// [`ROLE_SURVEY`]'s verdicts.
/// **The disk already carries a nife data partition**, so this machine is installed rather than
/// installable. ASCII `HAVIT`.
pub const R_ALREADY: u64 = 0x_48_41_56_49_54;
/// The disk carries no nife data partition: no table at all, or somebody else's.
pub const R_EMPTY: u64 = 0x_45_4D_50_54_59;

/// The entry array under construction. 16 KiB, so it goes in `.bss` rather than on the stack.
static mut ARRAY: [u8; ENTRY_ARRAY_BYTES] = [0; ENTRY_ARRAY_BYTES];
/// The primary table as read back by [`survey`]: LBA 0..33 is 34 logical blocks, five transfer
/// blocks. Also `.bss`, for the same reason.
static mut PRIMARY: [u8; 5 * blk::BLOCK_SIZE] = [0; 5 * blk::BLOCK_SIZE];

#[unsafe(no_mangle)]
pub extern "C" fn _start(role: u64, boot_file_len: u64, _a2: u64) -> ! {
    // The page shared with the block server, mapped out of this program's own budget
    // (milestone 108). Before anything else, because every `blk` call goes through it.
    if !user_mode_runtime::map_page_frame(BLK_PAGE_FRAME, BLK_PAGE, true, BUDGET) {
        user_mode_runtime::exit()
    }
    if role == ROLE_SURVEY {
        survey()
    }
    debug_assert!(role == ROLE_INSTALL);
    if boot_file_len == 0 {
        send(REPORT, R_NO_BOOT_FILE, 0, 0);
        user_mode_runtime::exit()
    }
    install(boot_file_len)
}

/// **Is nife already on this disk?** Read the primary table and look for a partition of the nife
/// data type (DECISIONS §45 (a nife partition is `EC5CC08B-D749-4434-AC38-A274C50385BA`)). Never by
/// name: on any disk a Mac has touched there are no partition
/// names at all.
///
/// A disk this cannot read, or one with no table, is [`R_EMPTY`]: "there is no nife here" is the
/// honest answer to both, and the caller's next step after either is to ask a person.
fn survey() -> ! {
    let size = call(BLK, req(blk::SIZE), 0).0 as i64;
    if size <= 0 {
        send(REPORT, R_EMPTY, 0, 0);
        user_mode_runtime::exit()
    }

    // LBA 0 through 33: the protective MBR, the primary header and the whole entry array, which is
    // five transfer blocks.
    let head = primary();
    for i in 0..5u64 {
        if (call(BLK, req(blk::READ), i).0 as i64) < 0 {
            send(REPORT, R_EMPTY, 0, 0);
            user_mode_runtime::exit()
        }
        // SAFETY: `BLK_PAGE` is a mapped page of exactly one transfer block, and the destination
        // window is inside `head`, which is five of them.
        unsafe {
            core::ptr::copy_nonoverlapping(
                BLK_PAGE as *const u8,
                head.as_mut_ptr().add(i as usize * blk::BLOCK_SIZE),
                blk::BLOCK_SIZE,
            );
        }
    }
    let head: &[u8] = head;
    let Ok(table) = GloballyUniqueIdentifierPartitionTable::parse(
        &head[LBA as usize..2 * LBA as usize],
        &head[2 * LBA as usize..],
    ) else {
        send(REPORT, R_EMPTY, 0, 0);
        user_mode_runtime::exit()
    };
    for (_, part) in table.partitions() {
        if part.type_guid == types::NIFE_DATA {
            send(REPORT, R_ALREADY, part.first_lba, 0);
            user_mode_runtime::exit()
        }
    }
    send(REPORT, R_EMPTY, 0, 0);
    user_mode_runtime::exit()
}

/// The whole of it, in the order that makes a refusal cheap: ask the disk its size, lay the
/// arithmetic out, draw the randomness, and only then write.
fn install(boot_file_len: u64) -> ! {
    let size = call(BLK, req(blk::SIZE), 0).0 as i64;
    if size <= 0 || !(size as u64).is_multiple_of(LBA) {
        send(REPORT, R_DISK_FAILED, 1, size as u64);
        user_mode_runtime::exit()
    }
    let block_count = size as u64 / LBA;

    let Some(layout) = Layout::fit(block_count) else {
        send(REPORT, R_TOO_SMALL, size as u64, 0);
        user_mode_runtime::exit()
    };

    // **Every random byte this install needs, drawn together and before the first write**, so that
    // a process with no entropy endpoint finds out while the disk still holds whatever it held.
    // Three ids for the table and one serial for the FAT volume.
    let mut guids = [Guid::ZERO; 3];
    for g in guids.iter_mut() {
        let Some(bytes) = random16() else {
            send(REPORT, R_NO_ENTROPY, 0, 0);
            user_mode_runtime::exit()
        };
        *g = Guid::v4_from_random(bytes);
    }
    let Some(serial) = random16() else {
        send(REPORT, R_NO_ENTROPY, 0, 0);
        user_mode_runtime::exit()
    };
    let volume_id = u32::from_le_bytes([serial[0], serial[1], serial[2], serial[3]]);

    let Ok(volume) = fat::Volume::new(
        ESP_SECTORS,
        layout.esp_first,
        boot_file_len,
        volume_id,
        BOOT_FILE_NAME,
    ) else {
        send(REPORT, R_DISK_FAILED, 2, boot_file_len);
        user_mode_runtime::exit()
    };

    if let Err(step) = write_table(&layout, block_count, &guids) {
        send(REPORT, R_DISK_FAILED, step, 0);
        user_mode_runtime::exit()
    }
    if let Err(step) = write_efi_system_partition(&layout, &volume, boot_file_len) {
        send(REPORT, R_DISK_FAILED, step, 0);
        user_mode_runtime::exit()
    }

    // One flush, so that what a reboot reads is what this program wrote rather than what a
    // controller still has in a cache.
    let _ = call(BLK, req(blk::FLUSH), 0);

    send(REPORT, R_INSTALLED, layout.data_first, layout.esp_first);
    user_mode_runtime::exit()
}

/// **Where the two partitions go**, in logical blocks, inclusive at both ends the way a GPT entry
/// records them.
struct Layout {
    data_first: u64,
    data_last: u64,
    esp_first: u64,
    esp_last: u64,
}

impl Layout {
    /// Fit the layout onto a disk of `block_count` logical blocks, or answer `None` if it will not
    /// fit. **All of the arithmetic, in one place**, so the writes below have nothing to decide.
    ///
    /// The EFI system partition goes at the end and the data partition takes everything before it,
    /// for the reason this file's header gives.
    fn fit(block_count: u64) -> Option<Layout> {
        // The last block a partition may use: the backup entry array and its header sit above it.
        let last_usable = block_count.checked_sub(BACKUP_BLOCKS + 1)?;

        // The EFI system partition, pushed down to an alignment boundary. Its length then grows by
        // whatever the rounding left over, which is harmless: FAT is told how many sectors it has.
        let esp_first = (last_usable + 1).checked_sub(ESP_SECTORS)? / ALIGN * ALIGN;
        if esp_first < ALIGN + DATA_MIN_SECTORS {
            return None;
        }
        let data_first = ALIGN;
        let data_last = esp_first - 1;

        // `mkfs` refuses a partition that is not 4096-aligned at both ends rather than rounding one
        // in, so this must not hand it one. Both bounds are multiples of `ALIGN`, which is a
        // multiple of eight sectors, so this cannot fail; it is checked anyway, because the day it
        // can is the day somebody changes `ALIGN`.
        if !data_first.is_multiple_of(SECTORS_PER_TRANSFER)
            || !(data_last + 1).is_multiple_of(SECTORS_PER_TRANSFER)
        {
            return None;
        }

        Some(Layout {
            data_first,
            data_last,
            esp_first,
            esp_last: last_usable,
        })
    }
}

/// Write both copies of the partition table. `Err(step)` names which write failed.
fn write_table(layout: &Layout, block_count: u64, guids: &[Guid; 3]) -> Result<(), u64> {
    let entries = [
        (
            types::NIFE_DATA,
            layout.data_first,
            layout.data_last,
            NAMES[0],
        ),
        (
            types::EFI_SYSTEM,
            layout.esp_first,
            layout.esp_last,
            NAMES[1],
        ),
    ];
    let mut parts = [Entry::UNUSED; 2];
    for (i, (type_guid, first, last, name)) in entries.iter().enumerate() {
        parts[i] = Entry::new(*type_guid, guids[i + 1], *first, *last)
            .with_name(name)
            .map_err(|_| 10 + i as u64)?;
    }

    let array = array();
    let table = GloballyUniqueIdentifierPartitionTable::create(
        guids[0],
        LBA as usize,
        block_count,
        &parts,
        array,
    )
    .map_err(|_| 12u64)?;

    // One logical block of scratch. Built and written one at a time: the protective MBR, the two
    // headers and the entry array are each at a known LBA.
    let mut block = [0u8; LBA as usize];
    let entry_array: &[u8] = table.entry_array();
    let steps: [(u64, Option<&[u8]>); 5] = [
        (0, None),
        (PRIMARY_HEADER_LBA, None),
        (table.primary_entry_lba(), Some(entry_array)),
        (table.backup_entry_lba(), Some(entry_array)),
        (table.backup_header_lba(), None),
    ];
    for (n, (lba, bytes)) in steps.iter().enumerate() {
        let data: &[u8] = match bytes {
            Some(b) => b,
            None => {
                block.fill(0);
                let built = match n {
                    0 => table.write_protective_mbr(&mut block),
                    1 => table.write_primary_header(&mut block),
                    _ => table.write_backup_header(&mut block),
                };
                built.map_err(|_| 20 + n as u64)?;
                &block
            }
        };
        if !write_at(*lba, data) {
            return Err(30 + n as u64);
        }
    }
    Ok(())
}

/// **Lay the FAT32 volume down and copy the boot file into it.**
///
/// Written in whole transfer blocks throughout, which is what the alignment above buys: the EFI
/// system partition begins on a multiple of [`ALIGN`], so a partition-relative sector `i` and the
/// device's own transfer blocks line up with no read-modify-write anywhere.
fn write_efi_system_partition(
    layout: &Layout,
    volume: &fat::Volume,
    boot_file_len: u64,
) -> Result<(), u64> {
    // The metadata: the boot sector, `FSInfo`, their backups, both copies of the table, and the
    // three directory clusters. Eight sectors are assembled in the shared page and written as one
    // request.
    let metadata = volume.metadata_sectors();
    let mut sector = 0u64;
    while sector < metadata {
        for slot in 0..SECTORS_PER_TRANSFER {
            // SAFETY: `BLK_PAGE` is a mapped, writable page of exactly one transfer block, and
            // `slot` is below the number of sectors in one.
            let out = unsafe {
                core::slice::from_raw_parts_mut(
                    (BLK_PAGE as *mut u8).add(slot as usize * LBA as usize),
                    LBA as usize,
                )
            };
            // Past the end of the metadata the volume answers `false` and the sector stays zero,
            // which is what belongs there: it is the first of the file's own clusters and is about
            // to be overwritten.
            if !volume.sector(sector + slot, out) {
                out.fill(0);
            }
        }
        let block = (layout.esp_first + sector) / SECTORS_PER_TRANSFER;
        if (call(BLK, req(blk::WRITE), block).0 as i64) < 0 {
            return Err(40);
        }
        sector += SECTORS_PER_TRANSFER;
    }

    // The boot file itself, contiguous from the cluster the volume's directory entry names.
    //
    // **Refuse a volume whose clusters do not land on transfer blocks**, rather than dividing and
    // silently writing somewhere else. `file_allocation_table` aligns its data area for exactly
    // this reason, and the run that made it do so wrote ten megabytes six sectors early onto a real
    // disk and produced a file the firmware would not start. This is the check that would have said
    // so in one line.
    let first = layout.esp_first + volume.file_first_sector();
    if !first.is_multiple_of(SECTORS_PER_TRANSFER) {
        return Err(42);
    }
    let blocks = boot_file_len.div_ceil(TRANSFER);
    for i in 0..blocks {
        let at = i * TRANSFER;
        let n = core::cmp::min(TRANSFER, boot_file_len - at) as usize;
        // SAFETY: the kernel mapped `boot_file_len` bytes read-only at `BOOT_FILE_VA`, and
        // `at + n` is bounded by that length; `BLK_PAGE` is one mapped writable transfer block.
        unsafe {
            core::ptr::copy_nonoverlapping(
                (BOOT_FILE_VA as *const u8).add(at as usize),
                BLK_PAGE as *mut u8,
                n,
            );
            if n < TRANSFER as usize {
                // The tail of the last cluster is never read, but leaving the previous block's
                // bytes there would put somebody else's data on the disk.
                core::ptr::write_bytes((BLK_PAGE as *mut u8).add(n), 0, TRANSFER as usize - n);
            }
        }
        if (call(BLK, req(blk::WRITE), first / SECTORS_PER_TRANSFER + i).0 as i64) < 0 {
            return Err(41);
        }
    }
    Ok(())
}

/// Sixteen random bytes from the entropy service, or `None` if this process holds no entropy
/// endpoint. Two round trips, because a reply carries one word; `entropy_protocol::delivered` is
/// what separates "the service answered with n bytes" from "the kernel refused the call".
fn random16() -> Option<[u8; 16]> {
    let mut out = [0u8; 16];
    for half in 0..2 {
        let (r0, r1) = call(ENTROPY, entropy::req(entropy::GET, entropy::MAX_BYTES), 0);
        let n = entropy::delivered(r0)?;
        if n != entropy::MAX_BYTES as usize {
            return None;
        }
        entropy::take(n, r1, &mut out[half * 8..half * 8 + 8]);
    }
    Some(out)
}

/// Lay `data` over the disk starting at logical block `first_lba`, **preserving everything else in
/// the transfer blocks it touches**: each is read, patched in the shared page, and written back.
///
/// The partition table is the one thing here written this way, and it has to be: the primary table
/// is 34 logical blocks and the transfer unit is eight of them, so the tail of the last transfer
/// block is the first three kilobytes of the partition the table is only supposed to be
/// *describing*.
fn write_at(first_lba: u64, data: &[u8]) -> bool {
    let mut done = 0usize;
    while done < data.len() {
        let byte_at = first_lba * LBA + done as u64;
        let block = byte_at / TRANSFER;
        let offset = (byte_at % TRANSFER) as usize;
        let n = core::cmp::min(blk::BLOCK_SIZE - offset, data.len() - done);
        if (call(BLK, req(blk::READ), block).0 as i64) < 0 {
            return false;
        }
        // SAFETY: `BLK_PAGE` is a mapped, writable page of exactly one transfer block, and
        // `offset + n` is bounded by that block above.
        unsafe {
            core::ptr::copy_nonoverlapping(
                data.as_ptr().add(done),
                (BLK_PAGE as *mut u8).add(offset),
                n,
            );
        }
        if (call(BLK, req(blk::WRITE), block).0 as i64) < 0 {
            return false;
        }
        done += n;
    }
    true
}

/// The entry-array buffer.
///
/// # Safety
/// One thread per address space here (DECISIONS §33 (the compositor's authority is memory, not
/// messages)), so there is no second reference. Taking the
/// raw pointer first is what `static_mut_refs` asks for.
fn array() -> &'static mut [u8] {
    let p = &raw mut ARRAY;
    // SAFETY: see above.
    unsafe { &mut *p }
}

/// [`survey`]'s read buffer. Same reasoning as [`array()`].
fn primary() -> &'static mut [u8] {
    let p = &raw mut PRIMARY;
    // SAFETY: see above.
    unsafe { &mut *p }
}

user_mode_runtime::panic_handler!();
