//! **The check that is worth more than every other test in this crate**, and it is `#[ignore]`d
//! because it needs a person and a host filesystem.
//!
//! A writer tested against its own reader proves almost nothing, and this crate has no reader at
//! all (see its `BUGS`). What proves it is somebody else's FAT driver reading what it wrote. Two do:
//! OVMF, in `cargo xtask install-boot`, which is the gate; and the host's own, which is this file,
//! and which is the one a person can run in three seconds while changing the layout.
//!
//! # EXAMPLES
//!
//! ```console
//! $ cargo test -p file_allocation_table -- --ignored --nocapture
//! wrote target/file-allocation-table-check.img
//!
//! $ hdiutil attach -imagekey diskimage-class=CRawDiskImage -nobrowse \
//!       target/file-allocation-table-check.img
//! /dev/disk7                                     /Volumes/NIFE
//!
//! $ diskutil info /dev/disk7 | grep Personality
//!    File System Personality:   MS-DOS FAT32
//!
//! $ ls -l /Volumes/NIFE/EFI/BOOT/BOOTX64.EFI
//! -rwx------  1 calef  staff  3000000 ... /Volumes/NIFE/EFI/BOOT/BOOTX64.EFI
//!
//! $ hdiutil detach /dev/disk7
//! ```
//!
//! On Linux the same two commands are `sudo mount -o loop,ro` and `umount`.
//!
//! **Run and passed on macOS 15 (Darwin 25.6.0) on 2026-09-21**, which is the record that matters:
//! Apple's `msdos` driver mounted the volume, reported it as **FAT32** (so the cluster count clears
//! the 65525 floor the boot sector's claim depends on), walked `\EFI\BOOT\BOOTX64.EFI` through both
//! directory levels, and returned all 3,000,000 bytes byte-exact.

use std::io::Write;

use file_allocation_table::{SECTOR, Volume};

/// The file the volume carries. Not a round number, so a length the writer got wrong by a cluster
/// cannot pass by accident.
const FILE_LEN: u64 = 3_000_000;

/// 700 MiB, in 512-byte sectors: comfortably over the FAT32 cluster floor and small enough that a
/// sparse file costs nothing.
const TOTAL_SECTORS: u64 = 700 * 1024 * 2;

#[test]
#[ignore = "writes an image for a host FAT driver to mount; see this file's header"]
fn write_an_image_for_the_hosts_own_fat_driver() {
    let volume = Volume::new(TOTAL_SECTORS, 0, FILE_LEN, 0x1234_5678, "BOOTX64.EFI")
        .expect("700 MiB is a FAT32 volume");

    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../target/file-allocation-table-check.img"
    );
    let mut image = std::fs::File::create(path).expect("create the image");

    let mut sector = [0u8; SECTOR];
    for index in 0..volume.metadata_sectors() {
        assert!(
            volume.sector(index, &mut sector),
            "sector {index} is metadata"
        );
        image.write_all(&sector).expect("write a metadata sector");
    }

    // A pattern with a prime stride, so a copy that is off by a cluster or that duplicates one
    // reads back wrong everywhere rather than in one place.
    let body: Vec<u8> = (0..FILE_LEN).map(|i| (i % 251) as u8).collect();
    image.write_all(&body).expect("write the file");
    let tail = volume.file_sectors() * SECTOR as u64 - FILE_LEN;
    image
        .write_all(&vec![0u8; tail as usize])
        .expect("pad the last cluster");

    // Everything past the file is never read, and a sparse tail is what keeps this cheap.
    image
        .set_len(TOTAL_SECTORS * SECTOR as u64)
        .expect("size the image");
    eprintln!("wrote {path}");
}
