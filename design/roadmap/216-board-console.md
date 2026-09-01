# 216. Nothing in this tree can read a board, so every hardware milestone waits on a person

**Status: BUILT** 2026-09-01. Minted the same day by the maintainer, from calef wiring the
VisionFive 2 as a remote target and the gap becoming concrete the same hour.

**What was built.** `script/board-console` (name provisional), over `cargo xtask board-console`
and `crates/board_console` (both names provisional). It opens the port at 115200 8N1, tees every
byte to a log file that is never optional, recognises the runbook's boot sequence, and returns a
different exit status for each way a session can end: reached the stage asked for, the board
announced a failure, it spoke and then went quiet, the time ran out, the port would not open. That
five-way answer is the deliverable rather than a nicety, because a bench script's whole reason to
exist is telling a hang from a refusal. See notes/board-console.md, which carries the runbook, the
marker table with each marker's source, and the BUGS.

**Tested with the board powered off**, which is the condition this milestone was built under.
Fixtures fed one byte at a time cover the recogniser; sources that block forever cover the
deadline; a FIFO standing in for a port covers a failing `stty` and a source that speaks and then
stops; and the real CH343 dongle covers everything except the board itself, including that `stty`
on the already-open descriptor moves it from 9600 to 115200 and that it reverts on exit.

**The one finding worth carrying out of the lane**, because reasoning did not produce it and a
byte-at-a-time test did: a partial line is weaker evidence than a complete one. `U-Boot ` reads as
U-Boot proper right up until the next three bytes turn out to be `SPL`, and a marker carrying a
payload captures a truncated one. So a tail may ratchet a stage and may not settle a word boundary
or capture text.

**Nothing gated this**, which was the point when it was minted and is now a fact rather than a
plan: no board was needed to build the thing that lets a lane use a board it cannot see, and none
was available while it was built.

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
  **The built tool reads and never writes**, to the port or to the outlet, so this stayed undecided
  rather than being settled by an implementation.
- **It says nothing about the aarch64 or x86_64 boards**, which will want the same thing with
  different banners. Whether that is one tool with a board profile or three is a real design
  question and is not answered here.
- **A captured log is not a test result.** Deciding pass or fail from console text is how a harness
  ends up asserting on a vendor's boot message; where that line sits is worth naming before it is
  crossed. The tool names its outcome `Reached`, for what it observed, rather than `Passed`; that
  is a wording, not a mechanism, and the line is still uncrossed rather than defended.
- **Every marker is quoted from documentation, and none from a board.** No VisionFive 2 console
  capture exists in this repository, so the fixtures are synthetic and say so in their own README.
  A marker whose real text differs by a word will be missed. The first bench session should commit
  its capture and replace them.
