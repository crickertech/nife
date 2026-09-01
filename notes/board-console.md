# Reading a board, without a person watching it

Milestone 216. `script/console` boots a shell in QEMU. `script/board-image` builds the VisionFive
2 payload and prints the `dd` commands for a card. Between those two there was nothing, so every
milestone gated on real hardware needed somebody at a terminal emulator reading with their eyes.
`script/board-console` is the missing middle: it opens the port, logs every byte, recognises how
far the boot got, and **stops on a deadline**.

The last clause is the only hard part. Opening a serial port is a `screen` invocation. Knowing
when to stop reading is what kept this a milestone.

## The commands

```
script/board-console                                # watch until the kernel banner, 120s cap
script/board-console --for 30m --until none         # sustained watching, for a stress run
script/board-console --port /dev/cu.usbmodemXXXX    # when two adapters are plugged in
script/board-console --replay target/board-console-1756744000.log   # re-read a capture
```

| flag | default | what it does |
|---|---|---|
| `--port <dev>` | the only USB serial adapter in `/dev` | Which device to open. `NIFE_BOARD_PORT` sets it too. Two adapters with no `--port` is an error, not a guess. |
| `--log <file>` | `target/board-console-<epoch>.log` | Where the bytes go. There is no way to turn it off. |
| `--for <duration>` | `120s` | The hard cap. `90`, `90s`, `30m`, `2h`. |
| `--until <stage>` | `banner` | Stop early at `spl`, `opensbi`, `uboot`, `handoff`, `banner`, or `none` to watch the whole duration. |
| `--quiet-after <duration>` | `15s` | Give up if the board speaks and then stops. `0` disables it. |

## The exit statuses, which are the point

A bench script needs to tell a hang from a refusal, and a tool with two exit codes cannot.

| status | meaning |
|---|---|
| 0 | Reached the stage asked for. With `--until none`, watched the whole duration and the board announced nothing. |
| 1 | The board announced a failure: a bad image header, a measured-boot refusal, or a kernel panic. |
| 2 | It spoke and then went quiet. **This is what a hang looks like from this end**, and it is the one the multicore risk cares about. |
| 3 | The time ran out with the requested stage unreached. |
| 4 | The port could not be opened, or the arguments were wrong. No session happened. |

## What it recognises, and where each marker came from

Every marker is quoted from `notes/visionfive2.md`'s bench runbook ("What appears, in order, on a
good day" and the failure-triage ladder) or from this tree's own source. **None was taken from a
capture**, because no VisionFive 2 console capture exists in this repository; see BUGS.

| stage | marker | source |
|---|---|---|
| `spl` | `U-Boot SPL` | runbook |
| `opensbi` | `OpenSBI v` | runbook ("record the version line") |
| `uboot` | `U-Boot ` followed by a word that is not `SPL`/`TPL`, or `StarFive #` | runbook |
| `handoff` | `Starting kernel ...` | runbook |
| `banner` | `nife on ` | `kernel/src/main.rs` |

And four things that end a session early rather than waiting the clock out:

| what | marker | source |
|---|---|---|
| a stale or wrong card | `Bad Linux RISCV Image magic!` | triage ladder row 3 |
| the trust boundary refusing | `MEASURED BOOT REFUSED` | `notes/trusted-init.md`, boot 12 |
| our own panic, with its message | `[PANIC] ` | `kernel/src/panic.rs` |
| U-Boot relocating (a note, not a stage) | `Moving Image from` | triage ladder row 4 |

## Two things in the design that are not obvious

**A partial line is weaker evidence than a complete one.** The recogniser is offered the
incomplete tail of the stream as well as the finished lines, because U-Boot's `StarFive #` prompt
has no newline after it and a tool that waited for one would sit there while the board sat waiting
for it. But a tail is ambiguous in two ways that a byte-at-a-time test found and reasoning did
not. `U-Boot ` reads as U-Boot proper right up until the next three bytes turn out to be `SPL`, so
a board dying in SPL was being reported two stages further along than it got. And a marker
carrying a payload captures a truncated one: the tail `nife on ` recorded the banner as the empty
string, and `[PANIC] ` recorded a panic with no message, both latched before the rest arrived. So
a tail may ratchet a stage, because a substring match is monotone and more bytes cannot unmake it;
it may not settle a word boundary and it may not capture text.

**The read happens on its own thread, so the deadline holds whatever the reader does.** The port
is configured with `min 0 time 1`, which makes a `read` return after a tenth of a second with
whatever arrived, including nothing. When that works, a single-threaded loop would be fine. There
are several ordinary ways for it not to work: an `stty` that failed and was only warned about, a
driver that ignores the setting, a file or a pipe standing in for a port. So the read runs on a
worker and the loop waits on a channel with a timeout. The worker may still be parked in `read`
when the tool decides to stop, and that is fine: deciding to stop means returning, the process
exits, and the descriptor goes with it. This is `CLAUDE.md`'s *Never leave QEMU running* rule
wearing different clothes. An emulator that never exits and a board that never speaks are the same
bug seen from the tool's side.

**Open the device first, then run `stty` on it.** Opening a tty nobody currently holds resets its
termios to the driver's initial state, so an `stty` before the open is undone by the open; and
holding the descriptor is what keeps the settings alive, because they revert when the last user
closes. Measured on the rig: the CH343 sits at 9600 when nobody holds it and reads 115200 while
this tool does. Getting the order backwards would produce a session at the wrong baud logging
plausible garbage, which the triage ladder's second row would then send somebody chasing a bad
adapter.

**And `cu.`, never `tty.`** On macOS the `tty.*` name is the dial-in side and blocks in `open`
until carrier detect asserts, which a three-wire console cable never does. That hangs *before* any
deadline of ours has started. The tool warns if it is handed one.

## Why there is no dependency

Configuring a UART is one `tcsetattr`, and the two ordinary ways to reach it from Rust are the
`serialport` crate or `libc` and the struct by hand. Both are new dependencies in the shipping
graph, and §46 (thin primitives or whole subsystems; we write everything in between) makes taking
one a decision rather than a convenience. A lane does not take that decision. A serial
configuration call is squarely the "in between" that section refuses: not a thin architectural
primitive, and nothing like a whole subsystem. `stty(1)` makes the
same call, is in every base system, and costs a process spawn per session. If this ever needs to
*write* to the board with flow control, or a non-standard baud, the trade changes and the
dependency is worth proposing.

## Testing it with no board

The board was powered off for all of this and its power strip was unreachable, so nothing below
involved a booting machine.

- **The recogniser** runs against four fixtures under `crates/board_console/tests/fixtures/`, fed
  one byte at a time, which is the worst case a real UART delivers and the case that catches a
  recogniser depending on chunk boundaries. Read that directory's README first: **the fixtures are
  synthetic**, constructed from documented markers rather than captured.
- **The deadline** runs against sources that block forever, which is what a powered-off board
  looks like through a port whose read timeout did not take.
- **A real descriptor**: a FIFO standing in for a port covers `stty` failing (a warning, not
  fatal) and a source that speaks and then stops (caught as silence, exit 2).
- **The real adapter**, with the board off, covers everything except the board: discovery finds
  `/dev/cu.usbmodem*`, the `stty` moves it to 115200 and it reverts on exit, and the deadline
  returns with zero bytes and exit 3.

## EXAMPLES

A boot check, the ordinary case:

```
$ script/board-console --for 90s
--- /dev/cu.usbmodem5C7B0104661 at 115200 baud, logging to target/board-console-1756744000.log, up to 90s ---
U-Boot SPL 2021.10 (Feb 12 2023 - 20:24:34 +0800)
...
Starting kernel ...

nife on RISC-V (rv64, S-mode, Sv39)

board-console: reached kernel banner (1437 bytes in 6.2s)
board-console: banner: nife on RISC-V (rv64, S-mode, Sv39)
board-console: log at target/board-console-1756744000.log
$ echo $?
0
```

A hang, which is what a multicore defect looks like:

```
$ script/board-console --for 90s --quiet-after 10s
...
Starting kernel ...

board-console: went quiet after kernel handoff (491 bytes in 14.1s)
board-console: log at target/board-console-1756744100.log
$ echo $?
2
```

Nothing at all, with the triage ladder's first row said out loud:

```
board-console: time ran out after nothing recognisable (0 bytes in 90.0s)
board-console: not one byte arrived. Check TX/RX are crossed, that the board has power, that the
DIP switches are on QSPI, and that this is the cu.* device.
```

Re-reading a capture, which needs no board and no adapter:

```
$ script/board-console --replay target/board-console-1756744100.log
```

## BUGS

**The markers have never been checked against a board.** Every one is quoted from the runbook or
from this tree's source, and none from a capture, because no capture exists here: the bench
sessions of 2026-08-14 and 2026-08-15 are recorded in `notes/visionfive2.md` as prose and quoted
fragments, and the transcripts were never committed. So a marker whose real text differs by a word
will be missed, and a missed marker reports a healthy board as having got less far than it did.
**The first real capture should be committed and the fixtures replaced.**

**A missed marker fails toward pessimism; a matched one does not.** Matching is `contains`
anywhere in a line, so a console that echoed `Starting kernel ...` back would be read as having
handed over. Nothing guards against that, and today the only way it happens is a person typing
into the same session, because this tool never writes to the port.

**It does not recognise an OpenSBI trap dump**, which the triage ladder lists as a real and
specific outcome (the kernel started the S7 and vendor firmware died in its own handler). The
dump's exact first line is written down nowhere in this tree, and guessing it would put text in a
recogniser that no machine has ever printed. Such a boot is caught as silence or as time running
out, with the dump sitting in the log, which is one step worse than being named.

**A captured log is not a test result.** The roadmap block says this first and it is worth
repeating where the tool is: this reports how far a boot got, and deciding a milestone passed on
the strength of a vendor's boot message is a line nobody has agreed to cross. `Reached` is named
for what was observed rather than for a verdict.

**It reads and never writes.** A first boot still needs the four `StarFive #` commands typed by
hand, which `script/board-image` prints. What this removes is the watching, not the typing.

**It does not touch power.** The board's Kasa strip was not reachable from either machine when
this was written, and the roadmap block declines to decide whether this tool should ever drive it.
A tool that power-cycles is a different and more dangerous object than one that reads.

**One board's vocabulary.** These stages are the VisionFive 2's boot chain. An aarch64 or x86_64
board wants the same shape with different banners, and whether that is one tool with a board
profile or three tools is an open question that milestone 216's block names and does not answer.

**`--until banner` returns the moment the banner appears**, so a failure printed one line later is
not seen. The exception is a failure in the *same* read as the banner, which does win, because
that is what boot 12's measured-boot refusal looked like. For sustained watching use `--until
none`, which has no early exit and sees everything up to the cap.

**Neither the tool nor this note knows whether a session was interrupted.** Ctrl-C kills the
process, the descriptor closes, and the log holds every byte that had arrived, but no summary line
is written and no exit status distinguishes it from a crash. The log is the record; the summary is
a convenience.
