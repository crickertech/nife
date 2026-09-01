# 216. Nothing in this tree can read a board, so every hardware milestone waits on a person

**Status: NOT-STARTED.** Minted 2026-09-01 by the maintainer, from calef wiring the VisionFive 2
as a remote target and the gap becoming concrete the same hour. *(Number provisional until the
merge queue lands it.)*

**Gate: NONE.** No board is needed to build this, which is the point: the tool is what lets a lane
use a board it cannot see.

**In brief.** `script/console` runs `cargo xtask shell`, which is QEMU. `script/board-image` builds
the VisionFive 2 payload and prints the `dd` commands for a card, deliberately running nothing
destructive itself. Between those two there is nothing. **A serial console on real hardware is not
a thing this repository knows how to open**, so every `Gate: HARDWARE` milestone in the sense that
means "the board is on the desk" needs a person at a terminal emulator, reading with their eyes.

That is the binding constraint on the fatal risks rather than a convenience. Four of the five
unrun entries in `design/fatal-risks.md` are hardware-gated, and risk 5 (it cannot be made reliable
on multicore, and the bugs appear only on silicon) is the one this hurts most: its own decisive
experiment is *sustained* stress with the load-sensitive assertions live, and its text is honest
that the class "produces a confidence rather than a verdict". Sustained is exactly what a person at
a bench is worst at.

## What exists as of today, so nobody re-derives it

calef wired the rig on 2026-09-01. The UART is a **WCH CH343** dongle, presenting as CDC-ACM, at
`/dev/cu.usbmodem*` on the development Mac; it enumerates whether or not the board is powered.
Board power is on a Kasa smart strip, and `~/.local/bin/vf2-power` on that machine drives it, with
the outlet addressed by alias and no way for a caller to name a different one, because another
outlet on that strip feeds an external drive that must never be switched off.

**The plug was not reachable from either machine when this was written** (discovery found nothing
on two subnets, by broadcast and by unicast to every host), so treat remote power as unproven and
do not build anything that assumes it. This milestone's deliverable stands without it: a console
that reads and logs is useful the moment somebody presses power by hand, and gains reset later.

## What it needs

**A way to open the board's serial port, log what comes back, and know when to stop.** The last
clause is the whole difficulty and is why this is a milestone rather than a one-line `screen`
invocation:

- **Knowing a boot succeeded** means recognising the sequence `notes/visionfive2.md`'s bench
  runbook already writes down: the SPL banner, OpenSBI's, U-Boot's, then `Starting kernel ...`,
  then ours. That note is the specification; read it rather than inventing markers.
- **Knowing a boot hung** is the case that matters, because a hang is what a multicore defect looks
  like. A timeout with the captured log is the answer, not a hang of the tool itself.
- **Never leaving the port open**, which is this tree's `Never leave QEMU running` rule wearing
  different clothes. A held serial port locks out the next run and the failure surfaces far from
  the cause.

## BUGS

- **This block does not decide whether the tool drives power**, and it should not until the plug is
  reachable. A tool that power-cycles is a different and more dangerous object than one that reads.
- **It says nothing about the aarch64 or x86_64 boards**, which will want the same thing with
  different banners. Whether that is one tool with a board profile or three is a real design
  question and is not answered here.
- **A captured log is not a test result.** Deciding pass or fail from console text is how a harness
  ends up asserting on a vendor's boot message; where that line sits is worth naming before it is
  crossed.
