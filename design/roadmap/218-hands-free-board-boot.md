# 218. Every boot of the VisionFive 2 needs a human typing four commands into U-Boot

**Status: NOT-STARTED.** Minted 2026-09-01 by the maintainer, after driving the board from a script
made the cost of the manual path concrete. *(Number provisional until the merge queue lands it.)*

**Gate: NONE.** The fix is a boot-path change and needs the board only to confirm it.

**In brief.** The board's own autoboot fails. Captured 2026-09-01 on real hardware:

```
Found /extlinux/extlinux.conf
Retrieving file: /nife-vf2.img
225280 bytes read in 31 ms (6.9 MiB/s)
Moving Image from 0x40200000 to 0x80200000, end=802ff000
Device tree not found or missing FDT support
### ERROR ### Please RESET the board ###
```

So the only path that reaches nife is the manual one: interrupt the two-second countdown and type
four commands (five with the archive). `script/board-image` prints them and
`notes/visionfive2.md` explains why: with no `fdt` line in the extlinux label, U-Boot falls back to
`fdt_addr_r`, then `fdt_addr`, then `$fdtcontroladdr`, and those addresses land outside the kernel's
boot page table. The manual path exists to control the DTB address exactly.

## Why this is worth a milestone rather than a note

**It is the difference between a bench session and a test target.** A boot that needs a person at a
keyboard cannot be repeated overnight, and `design/fatal-risks.md` risk 5 (it cannot be made
reliable on multicore, and the bugs appear only on silicon) names *sustained* stress as its decisive
experiment. Sustained is exactly what the manual path forecloses.

It was proved on 2026-09-01 that a script can drive the manual path over the serial console, so this
is not a blocker to automation today. But a rig that works by typing at U-Boot's prompt is
recreating in a script what the boot loader is supposed to do, and it fails differently and worse:
it depends on catching a two-second window.

## The routes, none chosen

- **An `fdt` line in `extlinux.conf`**, if U-Boot will then load the DTB somewhere the kernel's boot
  map covers. Cheapest if it works; needs checking against what `fdt_addr_r` actually is on this
  board rather than assumed.
- **Repair the U-Boot environment and set `fdt_addr_r`.** The board reports
  `*** Warning - bad CRC, using default environment` on every boot, so the environment in SPI flash
  is already degraded. This route fixes that as a side effect and is persistent, but it writes to
  the board's flash, which is the least reversible thing on this list.
- **Widen the kernel's boot page table** so the fallback address is inside it.
  `notes/visionfive2.md` refers to this as the gigapage-1 fix, which makes it the route the tree has
  already been contemplating; it is also the only one that fixes the class rather than this board.

## BUGS

- **The `Invalid partition 3` complaints are not explained here.** Every boot prints
  `** Invalid partition 3 ** / Couldn't find partition mmc 1:3` several times before finding
  `mmc 1:1`. Harmless so far, unexamined, and recorded so nobody reads them as caused by whatever
  this milestone changes.
- **This block does not price the routes**, which is what its own tenet asks for. Doing so needs a
  board and one boot per route.
- **A persistent fix is harder to un-fix.** The environment route writes to SPI flash on a board
  this project owns exactly one of.
