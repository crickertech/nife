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

**And the stick boots any architecture, made from any machine** (§157, amended again at 17:19 UTC):
a person on a Mac, a Linux PC or a Windows PC, whatever its CPU, can make a boot for an x86_64 PC,
an aarch64 board or a riscv64 board. calef left open whether that is one universal stick or a target
the person picks.

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

## The universal stick, recommended

UEFI already defines one: each architecture's firmware looks for its own file at a fixed path on a
FAT32 volume, and ignores the others.

| Architecture | Removable-media path | nife today |
|---|---|---|
| x86_64 | `\EFI\BOOT\BOOTX64.EFI` | **Works** (xenon, milestone 87) |
| aarch64 | `\EFI\BOOT\BOOTAA64.EFI` | Not built: `uefi_loader` builds only for `x86_64-unknown-uefi` |
| riscv64 | `\EFI\BOOT\BOOTRISCV64.EFI` | Not built: radon boots through U-Boot's `booti`, three files and a boot script (`script/board-image`) |

**So one stick carrying all three files boots every UEFI machine, and nobody chooses a target.** The
program embeds the three payloads and writes all of them; which host it runs on stops mattering,
because it only writes files.

| | Shape | Kept or lost |
|---|---|---|
| **U1. UEFI everywhere** | Port `uefi_loader` to aarch64 and riscv64; the boards boot it through U-Boot's UEFI support | **Recommended.** One layout, no target selection, no per-board boot scripts; chosen on elegance, and it would be chosen at equal cost |
| U2. The person picks a target | The program writes `BOOTX64.EFI` for a PC, or `script/board-image`'s set for radon | Kept as the fallback for any board where U1's premise fails. Two layouts to maintain, and a third per new board |

**U1's premise is recalled, not read, and it is the first thing to check**: that U-Boot as shipped
on radon (VisionFive 2) and argon (Jetson TX1) implements enough of UEFI (`bootefi`, and distro boot
scanning removable media for the fallback paths) to start a nife loader, and hands it the device
tree. If it does not on a board, that board takes U2 until it does. Rust's UEFI targets for aarch64
and riscv64 are also recalled, not checked: confirm each exists and at what support tier before
relying on it.

**The check is five commands at each board's U-Boot prompt with a FAT32 stick in**, written into the
bench runbooks on 2026-09-19: radon's is notes/visionfive2.md, "To measure at the bench", item 10,
and argon's is milestone 127's bench list. What the tree already shows for radon: its U-Boot
(2021.10, StarFive's vendor build) does not initialise USB by itself at boot, and its BootROM cannot
boot USB at all, so a USB boot there is always U-Boot reading the stick.

**Two axes, and they are independent.** The *target* axis above is what the stick boots. The *host*
axis is where the program runs: macOS (one universal binary covers Apple Silicon and Intel), Linux
and Windows, each on x86_64 and arm64. The host axis is ordinary Rust cross-compilation.

## What a lane has to answer, with the options known today

1. **Finding the stick safely.** It must offer only removable volumes (USB sticks, and SD cards,
   since radon and argon boot from microSD) and confirm by name and size before writing. Formatting
   the wrong disk destroys someone's data, which makes this the part to over-test. Each host has its
   own enumeration (macOS Disk Arbitration or `diskutil`, Linux `/sys/block/*/removable` and
   `lsblk`, Windows volume and drive-type APIs); **that list is recalled, not read**, and the lane
   should read each before relying on it.
2. **Formatting: shell out, or write GPT and FAT32 ourselves.** The OS's own tools
   (`diskutil eraseDisk`, `mkfs.vfat`, `format`/`diskpart`) are proven and handle every quirk of
   real sticks, but differ per OS and are not always installed on Linux. The tree has a GPT crate
   (`crates/globally_unique_identifier_partition_table`) and milestone 57's `disk_partitioner`, and
   no FAT32 writer. Reversible either way; say which and why, measured.
3. **Building the host binaries** from this tree (the host axis above), each embedding every
   architecture's payload, reproducibly (`script/build-is-reproducible` exists for the x86_64
   image). Cross-compiling from the dev Mac, or CI runners per OS, is the lane's call.
4. **Testing without destroying a real disk**: a file-backed or RAM disk per host (macOS `hdiutil`,
   Linux loop devices, Windows VHD), and a boot of the resulting stick image as the end-to-end proof
   on each architecture QEMU has UEFI firmware for (OVMF for x86_64, the EDK2 builds for aarch64 and
   riscv64).
5. **What it says when it finishes**: the next step for the stranger (boot the PC from the stick,
   Secure Boot off; see `a-stick-that-boots-with-secure-boot-on.md`).

## What this does not include

- **Signing.** Decided against for now (§157). Adding it later changes the release process, not the
  program.
- **The web page and its per-OS instructions for stepping past the warnings.** Rung 4, calef's to
  publish.
- **Installing onto the PC's disk.** Rung 2, the installer proposal.

## BUGS

- **An unsigned program is refused by default on macOS**, and the workaround moves between macOS
  releases. The page's instructions will need re-checking on each major release, and nothing gates
  that.
- **A new dependency may be tempting** for USB enumeration or FAT32; §46 applies, and the lane
  should treat one as calef's call rather than a convenience.
