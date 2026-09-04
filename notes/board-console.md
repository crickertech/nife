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
| `--until <stage>` | `banner` | Stop early at `spl`, `opensbi`, `uboot`, `handoff`, `banner`, `tour`, or `none` to watch the whole duration. |
| `--quiet-after <duration>` | `15s` | Give up if the board speaks and then stops. `0` disables it. Suppressed once the tour completes, always. |

**And one mode that opens no port at all** (milestone 249): `--tally <log>` reads a capture of many
boots and reports what the thread-placement lottery drew on each. It is `board_console::lottery`
rather than the recogniser, it answers a question no single-boot reader can be asked (how often does
each arrangement come up), and its exit statuses are only `0` and `4`, because an analysis of a
finished log has no board left to have gone quiet. See notes/soak.md.

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

Every marker was first quoted from `notes/visionfive2.md`'s bench runbook ("What appears, in order,
on a good day" and the failure-triage ladder) or from this tree's own source. **They were then
checked against the board**, on 2026-09-01, against a captured success and a captured failure that
now live in `crates/board_console/tests/fixtures/captured/` and are asserted on by the tests.

| stage | marker | source |
|---|---|---|
| `spl` | `U-Boot SPL` | runbook, confirmed on the board |
| `opensbi` | `OpenSBI v` | runbook ("record the version line"), confirmed: `OpenSBI v1.2` |
| `uboot` | `U-Boot ` followed by a word that is not `SPL`/`TPL`, or `StarFive #` | runbook, confirmed both ways |
| `handoff` | `Starting kernel ...` | runbook, confirmed |
| `banner` | `nife on ` | `kernel/src/main.rs`, confirmed |
| `tour` | `nife: the capability core runs on ` | `kernel/src/main.rs`, confirmed |

One more thing is reported and is deliberately **not** a stage: `init/build`, meaning userspace init
built its child. It cannot be a stage without breaking the ladder, because a card with no archive
runs the whole tour and never reaches it, so putting it below `tour` would make reaching the tour
imply something that did not happen. It is a detail of a successful boot, like `Moving Image from`,
and it is the only difference between the two successful captures.

And five things that end a session early rather than waiting the clock out:

| what | marker | source |
|---|---|---|
| **U-Boot giving up**, with the reason on the line before | `### ERROR ### Please RESET the board ###` | **captured 2026-09-01**; no documentation in this tree named it |
| a stale or wrong card | `Bad Linux RISCV Image magic!` | triage ladder row 3, never seen at a bench |
| the trust boundary refusing | `MEASURED BOOT REFUSED` | `kernel/src/trust.rs`, **captured 2026-09-01** |
| our own panic, with its message | `[PANIC] ` | `kernel/src/panic.rs` |
| U-Boot relocating (a note, not a stage) | `Moving Image from` | triage ladder row 4, confirmed |

### Four outcomes, three of them real

The captures of 2026-09-01 turned this from a two-way question into a four-way one, and three of
the four are things a board actually did.

| outcome | what it looks like | exit |
|---|---|---|
| **Success** | the tour reaches `nife: the capability core runs on RISC-V.`, then the board goes quiet because the kernel halts in `wfi` | 0 |
| **U-Boot refused before the kernel ran** | the image loads and relocates, then `### ERROR ### Please RESET the board ###` | 1 |
| **The kernel booted and then halted on purpose** | `MEASURED BOOT REFUSED`, after the banner and most of a tour | 1 |
| **A genuine hang** | it starts, says a few lines, and stops before the tour | 2 |

Three of those four are traps for a naive recogniser, and each got a fix:

**Both successes and the measured-boot refusal contain the banner.** The refusal prints
`Starting kernel ...`, the whole nife banner, and most of a tour before halting, so a watcher that
returned the moment `--until banner` was satisfied would report it as a success, in the case a
bench script most needs to be right about. So reaching the wanted stage starts a **settle window**
(`settle`, two seconds) rather than ending the session, and a failure arriving inside it wins.

**Silence after the tour is how a good boot ends.** The kernel halts in `wfi`, so the board goes
quiet and stays quiet. Treating silence alone as a hang would fail every successful boot, so the
quiet timer is suppressed once the tour completes, whatever `--quiet-after` says. The synthetic hang
fixture stops *before* the tour, which is exactly the difference.

**A measured-boot refusal is the gate working, not a crash.** It exits 1, because the board did not
boot and a script asking "did it boot" needs a no. But the message says what actually happened and
what to do, because a report that read like a crash would send somebody debugging the boot mechanism
when what is wrong is that two files on the card came from different builds.

### The extlinux outcome, which documentation did not have

The capture that mattered most is the one that failed. From power-on, the extlinux path ends:

```
Moving Image from 0x40200000 to 0x80200000, end=802ff000
Device tree not found or missing FDT support
### ERROR ### Please RESET the board ###
```

That is exactly the caveat `notes/visionfive2.md` records about U-Boot's fallback DTB addresses,
and it arrives **after** the image has loaded and relocated and **before** the kernel has run a
single instruction. A recogniser that knew only the stages would have watched the image load, seen
nothing more, and called the silence a hang, which is the worst available answer: it sends somebody
hunting a multicore bug in a kernel that never started. **Booted, hung, and refused-before-the-
kernel are three outcomes, not two.** The tool exits 1 for the refusal and 2 for a hang, and the
difference is the difference between resetting the board and opening a debugger.

### And `tour` is a stronger claim than `banner`

`nife on RISC-V (rv64, S-mode, Sv39)` is printed before the device tree is touched, so it says the
console works and nothing else. `nife: the capability core runs on RISC-V.` is the last line of the
boot tour, so it says paging, traps, the timer, the frame allocator, SMP and the scheduler all came
up. `--until tour` is the one to want when the question is "did it work"; the default stays
`banner` because only the milestone-tour build prints the other, and a shell build would wait for a
line that is never coming.

### The card's U-Boot environment is degraded, and that is not ours

Both captures carry `*** Warning - bad CRC, using default environment`, several
`** Invalid partition 3 **` / `Couldn't find partition mmc 1:3` / `Can't set block device`
complaints, and `## Error: "boot2" not defined`, before U-Boot finds `mmc 1:1` and gets on with it.
The board boots through all of it. Nobody should read those lines as a defect in our payload, and
whether the environment is worth repairing is somebody else's milestone.

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

**Open the device first, then run `stty` on it, then check that the speed took.** Opening a macOS
`cu` device resets its termios towards the driver's default, so a configuration made before the
read descriptor exists is undone by the open that follows it; and holding the descriptor is what
keeps the setting alive, because it reverts when the last user closes. Measured on the rig: the
CH343 sits at 9600 when nobody holds it and reads 115200 while this tool does.

**This is not a theory, and somebody already paid for it.** calef's first capture of the board on
2026-09-01 was pure garbage, because it configured the port with `stty` and then ran `cat`, and
`cat`'s open put the device back to the default rate. It looks exactly like a wiring fault, which
is the triage ladder's second row, and it cost a power cycle to diagnose. So the tool now reads the
speed back after configuring and says loudly when it is not 115200: an invisible wrong baud sends
the next person chasing hardware, and a stated one takes a second to fix.

The residual hazard, stated rather than hidden: because the configuration is a separate process
rather than a `tcsetattr` on our own descriptor, any *other* process that opens the device
mid-session can put it back, and nothing here would notice. That is the honest cost of having no
dependency, and it is the case that would justify taking one.

**`O_NONBLOCK` is not used, and that is a decision rather than an oversight.** Its job would be to
stop a silent board hanging the tool in `read`, and the worker thread above already guarantees
that. The thread's guarantee is the stronger one: it holds even when the `stty` fails outright,
where a non-blocking read on its own would not, because a non-blocking read returns immediately but
only a deadline decides when to stop asking.

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
  recogniser depending on chunk boundaries. Two of them are in `captured/` and are **raw bytes off
  the wire on 2026-09-01**, control characters and all; two are in `synthetic/` and are cases
  nobody has yet seen at a bench. The directory split is the provenance, deliberately, because a
  claim in a README is a weaker record than a path.
- **The deadline** runs against sources that block forever, which is what a powered-off board
  looks like through a port whose read timeout did not take.
- **A real descriptor**: a FIFO standing in for a port covers `stty` failing (a warning, not
  fatal) and a source that speaks and then stops (caught as silence, exit 2).
- **The port layer**, which is the part that looks untestable and mostly is not. The argument list
  `stty` is given, the complaint it produces when it fails, the speed read back afterwards, the
  dial-in warning, and the choice between zero, one and several adapters are all pure functions
  with the IO lifted off them, so a host test asserts on the exact words a person meets at a bench.
  `open` itself is exercised against a temporary file holding the real capture, so the path from
  `port::open` to a recognised boot runs in a host test.
- **The real adapter**, with the board off, covers everything except the board: discovery finds
  `/dev/cu.usbmodem*`, the `stty` moves it to 115200 and it reverts on exit, and the deadline
  returns with zero bytes and exit 3.

**The residue no host test reaches is one claim: that `tcsetattr` actually took.** Nothing on a
host is a tty, so a test can prove the right arguments were sent and cannot prove the device
listened. That was checked by hand against the CH343 (9600 before, 115200 while held, reverting on
exit), and it is what `confirm_speed` gates at runtime, which is the better answer anyway: the
check runs on every real session rather than only when somebody runs the tests.

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

**The markers are checked against one board, on one day, in four states.** That is much better than
where this started, which was documentation only, and it is not the same as proven. Not covered:
every other way this board can behave, a different vendor firmware build with differently worded
banners, the two synthetic cases nobody has yet seen at a bench, and the aarch64 and x86_64 boards,
which have not been looked at at all.

**There is no real sample of a hang**, which is the outcome this tool exists for, since a hang is
what a multicore defect looks like from the far end of a serial cable. The synthetic fixture is a
real capture truncated before the tour. If risk 5 ever produces a genuine one at a bench, capture
it; it would be worth more than every other fixture here. A marker whose real text differs by a word is missed, and a
missed marker reports a healthy board as having got less far than it did.

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

**It reads and never writes, and on this board that is not enough to boot.** The captured failure
is the proof: the extlinux path from power-on ends at `### ERROR ###`, so reaching nife means
interrupting autoboot and typing the four `StarFive #` commands. So a console that only reads
cannot, on its own, get this board into the state the hardware-gated milestones need. Driving
U-Boot is proven to work and is deliberately absent here, because whether it belongs in this tool
or a second one is a scope question for calef; see milestone 216's block for the proposal.

**Because it never writes, it cannot stop a rebooting soak, and that is now something a board does**
(milestone 249). `--features reboot_soak` makes a board cold-reboot every two minutes, and its
escape is a byte on the console UART: any byte, checked every five seconds. This tool holds the
port and cannot send one, so the escape is reached by a **person typing**, either into this
session's terminal or by detaching it first, and detaching a console is not free (notes/soak.md
records a 6% rate change from doing it mid-run). Two things a writing mode would buy: `--stop`, the
whole escape from a script; and `--stop-after <n>`, ending a series with exactly the sample it was
asked for.

**This is a proposed milestone rather than a change made here**, because it overturns the invariant
this note's own heading states and that a person reads before pointing this at hardware. Whether it
is a mode of this tool or a second entry point is a naming and boundary question, which makes it
calef's. The reason it is worth raising rather than leaving: it is the only host-testable thing that
would raise milestone 249's escape above rung four.

**It does not touch power.** The board's Kasa strip was not reachable from either machine when
this was written, and the roadmap block declines to decide whether this tool should ever drive it.
A tool that power-cycles is a different and more dangerous object than one that reads.

**One board's vocabulary.** These stages are the VisionFive 2's boot chain. An aarch64 or x86_64
board wants the same shape with different banners, and whether that is one tool with a board
profile or three tools is an open question that milestone 216's block names and does not answer.

**The settle window is two seconds, and two seconds is a guess.** It is long enough for the
captured measured-boot refusal, which follows the banner within a tour's worth of printing, and
there is no principle behind it beyond that. A failure that a board announces three seconds after
the awaited stage would still be reported as a success. `--until none` has no early exit at all and
sees everything up to the cap, which is the answer when being right matters more than being quick.

**No test opens a real serial device.** A pseudo-terminal pair would be the honest stand-in, and
making one needs `posix_openpt` and its `ioctl`s, which is `libc`, which is §46's decision and not
a lane's. So the port layer is tested as pure logic plus a regular file, and the one claim that
leaves unproven is named in the testing section above. If this crate ever takes a serial
dependency, a pty test should arrive with it.

**Neither the tool nor this note knows whether a session was interrupted.** Ctrl-C kills the
process, the descriptor closes, and the log holds every byte that had arrived, but no summary line
is written and no exit status distinguishes it from a crash. The log is the record; the summary is
a convenience.
