# The console fixtures, and which of them a board actually printed

Two directories, and the split is the point: **`captured/` is bytes off the wire, `synthetic/` is
text written by hand.** The distinction was a paragraph in this file for the first half of a day
and is now a directory, because a fixture that looks like a capture and is not one is the shape of
the fabricated block quote this tree carried for twelve days, and a claim in a README is rung four
where a path is rung three.

## `captured/`

**Raw bytes from the VisionFive 2, 115200 8N1, on 2026-09-01**, taken by calef with the board on
his desk. CRLF and control bytes are preserved exactly as they arrived, including the NUL run just
before our banner and the `\b` backspaces U-Boot's autoboot countdown overwrites itself with.
Nothing has been cleaned up; cleaning it would remove the parts a recogniser has to survive.

- **`vf2-2026-09-01-manual-boot.log`** is a **successful** boot, all the way from the SPL banner to
  `nife: the capability core runs on RISC-V.` Autoboot was interrupted and the four manual commands
  from `notes/visionfive2.md` were typed, which is the only way this board reaches nife today.
- **`vf2-2026-09-01-extlinux-refused.log`** is the **extlinux** path from power-on, and it fails:

  ```
  Moving Image from 0x40200000 to 0x80200000, end=802ff000
  Device tree not found or missing FDT support
  ### ERROR ### Please RESET the board ###
  ```

  That is exactly the caveat `notes/visionfive2.md` records about U-Boot's fallback DTB addresses.
  It is the third outcome, and the reason the recogniser has one: U-Boot refusing before the kernel
  ever ran looks nothing like a hang and must not be reported as one.

**Both captures show a degraded U-Boot environment on this card, and that is not a defect in our
payload.** `*** Warning - bad CRC, using default environment`, then several
`** Invalid partition 3 **` / `Couldn't find partition mmc 1:3` / `Can't set block device`
complaints, and `## Error: "boot2" not defined`, before it finds `mmc 1:1` and gets on with it. The
board boots through all of it. Nobody should read those lines as something we caused, and whether
the environment is worth repairing is somebody else's milestone.

**What the captures settled.** Every marker in the recogniser had been quoted from documentation
and none from a machine, and this is where that got checked. `U-Boot SPL`, `OpenSBI v`, the U-Boot
banner with a version where `SPL` would be, `StarFive #`, `Starting kernel ...`, `Moving Image
from` and `nife on ` were all correct as written. Two things were missing and are now in: the
`### ERROR ### Please RESET the board ###` refusal, and `nife: the capability core runs on `, which
is a stronger success signal than the banner because it means the whole boot tour ran.

## `synthetic/`

**Written by hand from documented markers**, because no capture of either case exists. Each line is
either a marker quoted from `notes/visionfive2.md`'s failure-triage ladder, a line quoted from this
tree's own source (`notes/trusted-init.md` for the measured-boot refusal), or filler in the shape
vendor firmware prints so that a chunk boundary lands somewhere other than on a marker. No test
asserts on the filler.

- **`vf2-bad-magic.log`**: U-Boot rejecting the payload's header (triage ladder row three).
- **`vf2-measured-refusal.log`**: the kernel refusing the archive at the trust boundary, which
  really happened on 2026-08-15 as boot 12, but was not captured.

These prove the recogniser finds the markers **it was told about**, in a stream that behaves like a
stream. They prove nothing about whether those markers are the text the board prints. If either
case ever occurs at the bench, capture it and move the file.
