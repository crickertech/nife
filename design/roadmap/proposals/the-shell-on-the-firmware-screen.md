# The shell on the firmware's screen, because on a PC with no serial port the prompt goes nowhere

**Status: PROPOSED 2026-09-19.** Found by milestone 198's rungs lane
(`milestone/198-rungs-to-a-trivial-install`) while mapping rung 1 of the trivial install DECISIONS
§157 defines: a USB stick that reaches a prompt on a PC. §157's table names the keyboard (milestones
242 and 192) and the interactive boot (milestone 182) as the missing pieces; this one it did not name.

**Gate: NONE.** Everything it builds on exists and is testable under OVMF.

## The gap, read from the code

On x86_64 the **kernel's** output reaches the screen: milestone 243 carries the firmware's linear
framebuffer across the handoff and tees `print!` into it (`console::attach_screen`), so the boot tour
is legible on any UEFI machine's monitor. **The shell's output does not.** Since milestone 299 the
console server is a userspace process whose x86 arm writes COM1 by port I/O and nothing else
(`components/src/console.rs`, the `(0x3F8, 8)` port range). So on a PC with no serial port, which is
every laptop and most desktops sold today, the tour scrolls past and **the prompt is never seen.**
xenon hides this, because milestone 87 bought it a serial module.

Nothing owns it. Milestone 243's follow-ons cover a gate on real silicon, early boot and the other
two architectures; milestone 177's graphical boot drives virtio-gpu, which no PC has; milestone 157
is U-Boot's `simple-framebuffer` on the boards.

## What the tree already has

- **The discovery**: `machine_discovery::framebuffer`, the aperture's address, geometry and pixel
  order, written by `uefi_loader` and read by the kernel (milestone 243).
- **A terminal engine**: `display_terminal` and `crates/video_terminal` behind the framebuffer
  contract (`crates/gfx_proto`, DECISIONS §29), proven in QEMU under milestone 177.
- **A simpler painter**: `crates/screen_console` (name provisional there), sharing `bitmap_font`.
- **The keystroke seam**: milestone 192's `boot_graphical_terminal` picks the keystroke source, and
  the serial `input` driver is already one arm, so a screen with serial input needs no new input
  code on xenon.

## Options (reversible, recommended)

| | Shape | Cost | Kept or lost |
|---|---|---|---|
| **S1. A pre-set-buffer backend behind the framebuffer contract** | A small driver holding the aperture as a device mapping serves `gfx_proto` to `display_terminal`; no mode setting, no queues | One driver. **It is the driver milestone 157 describes** ("a new backend behind that same contract, read a pre-set buffer instead of negotiating virtio queues"), fed by UEFI's handoff here and U-Boot's there | **Recommended.** One terminal engine for every screen, and one driver serving two milestones |
| S2. The console server paints the screen itself | Its x86 arm maps the aperture and calls `screen_console` beside its COM1 writes | Smallest | Lost: a second terminal engine with no escape-sequence handling, and the console server gains a device it was not built to hold |
| S3. The kernel paints for the shell | A kernel path for userspace text | New syscall surface | Refused: DECISIONS §121's reversal (milestone 299) took the console out of the kernel on purpose |

**The §92 test.** S2 is cheaper; S1 would still be chosen at equal cost, for the one-engine and
shared-driver reasons, so this recommendation is not about effort.

**One hazard S1 must answer.** Milestone 177's recorded blocker is a second `FLUSH` that does not
return through `components/src/gpu_driver.rs`'s real boot path (`notes/framebuffer-contract.md`'s
`BUGS`). A pre-set buffer needs no device flush, so the hang may not apply, but whether it lives in
the virtio driver or in the contract was **not checked**. And the kernel keeps teeing its own
`print!` into the same aperture, so the handover (the kernel stops painting when the driver starts)
is part of this work, or two painters interleave the way milestone 230 found two UART writers did.

## Exit criteria

1. **Under OVMF**: `cargo xtask uefi-boot`'s screen reader (`board_console::screen`) reads `$` back
   off the framebuffer, and a command typed on the serial line echoes on the screen.
2. **On xenon, with a monitor**: the prompt on the monitor and a serial keystroke echoing there,
   which is also milestone 192's option A on x86 (192's bench procedure is written for radon).

## Relation to other milestones

It is **rung 1's output half**; milestone 242 (USB host and HID) is its input half. Together they
are "a PC with a monitor and a USB keyboard, no serial cable". It also serves milestone 157: whoever
builds the pre-set-buffer driver first builds it for both.

## BUGS

- **x86_64 and UEFI only**, like milestone 243's mechanism; the boards need 157's discovery.
- **The aperture is mapped uncacheable** (243's `BUGS`: no PAT programming), so scrolling a full
  screen is slow on real silicon. Measure it on xenon before calling the prompt usable.
- **Whether a firmware left the screen in a mode the monitor is showing** is the firmware's choice
  (`uefi_loader` never calls `SetMode`), and a stranger's machine may differ from xenon.
