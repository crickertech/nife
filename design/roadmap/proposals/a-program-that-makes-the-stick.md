# A program that makes the stick: one download per operating system, the image inside it

**Status: PROPOSED 2026-09-19.** Written by the maintainer from calef's amendment to DECISIONS §157
the same day, which settles the shape and leaves the build to a lane.

**Gate: NONE.** The shape is decided (§157's amendment) and nothing it needs is missing from the
tree. It belongs to rung 1 of the trivial install (milestone 198's rescope): the stick is what every
later rung starts from.

## What calef decided, and what is left

Decided (§157, amended 2026-09-19): **one program per host operating system (macOS, Linux, Windows),
downloaded as one file, with the system image embedded, which writes a bootable stick when run.**
And **not signed or notarized for now**: the stranger steps past macOS Gatekeeper and Windows
SmartScreen by hand, and the web page says how.

Left to a lane, all reversible: how the program finds the stick, how it writes it, how it is built
and tested, and what it is called (provisionally; names are calef's).

## Why the common case is a file copy

The x86_64 system is already one file, `\EFI\BOOT\BOOTX64.EFI` (10,158,080 bytes on 2026-09-17),
holding the loader, the kernel and the archive (notes/x86-uefi-boot.md). Every UEFI firmware boots
that path from a FAT32 stick with no boot entry. Most sticks are sold FAT32. So:

| Path | When | What the program does | Administrator rights |
|---|---|---|---|
| **Copy** | The stick is already FAT32 | Creates `EFI/BOOT/` on the mounted volume and writes the file | **No** |
| **Format** | It is not | Partitions (GPT) and formats (FAT32), then copies | Yes: `sudo`, or a UAC prompt |

Copy first, format only when needed. The common case then asks for no password and never writes a
raw disk.

## What a lane has to answer, with the options known today

1. **Finding the stick safely.** It must offer only removable USB volumes and confirm by name and
   size before writing. Formatting the wrong disk destroys someone's data, which makes this the part
   to over-test. Each host has its own enumeration (macOS Disk Arbitration or `diskutil`, Linux
   `/sys/block/*/removable` and `lsblk`, Windows volume and drive-type APIs); **that list is
   recalled, not read**, and the lane should read each before relying on it.
2. **Formatting: shell out, or write GPT and FAT32 ourselves.** The OS's own tools (`diskutil
   eraseDisk`, `mkfs.vfat`, `format`/`diskpart`) are proven and handle every quirk of real sticks,
   but differ per OS and are not always installed on Linux. The tree has a GPT crate
   (`crates/globally_unique_identifier_partition_table`) and milestone 57's `disk_partitioner`, and
   no FAT32 writer. Reversible either way; say which and why, measured.
3. **Building three host binaries** from this tree, each embedding the image produced by
   `cargo xtask uefi-image`, reproducibly (`script/build-is-reproducible` exists for the image itself).
   Cross-compiling for Windows and Linux from the dev Mac, or CI runners per OS, is the lane's call.
4. **Testing without destroying a real disk**: a file-backed or RAM disk per host (macOS `hdiutil`,
   Linux loop devices, Windows VHD), and an OVMF boot of the resulting stick image as the end-to-end
   proof.
5. **What it says when it finishes**: the next step for the stranger (boot the PC from the stick,
   Secure Boot off; see `a-stick-that-boots-with-secure-boot-on.md`).

## What this does not include

- **Signing.** Decided against for now (§157). Adding it later changes the release process, not the
  program.
- **The web page and its per-OS instructions for stepping past the warnings.** Rung 4, calef's to
  publish.
- **aarch64 and riscv64 boards.** They boot from SD cards through U-Boot with more than one file
  (`script/board-image`); the stick is x86_64 UEFI until a board boots the same way.
- **Installing onto the PC's disk.** Rung 2, the installer proposal.

## BUGS

- **An unsigned program is refused by default on macOS**, and the workaround moves between macOS
  releases. The page's instructions will need re-checking on each major release, and nothing gates
  that.
- **A new dependency may be tempting** for USB enumeration or FAT32; §46 applies, and the lane
  should treat one as calef's call rather than a convenience.
