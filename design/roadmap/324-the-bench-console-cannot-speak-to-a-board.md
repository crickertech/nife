# 324. The bench console cannot speak to any of the three boards

**Status: PARTIAL.** Minted 2026-09-18 by calef, promoting a cluster rather than its members: four
proposals from three lanes, all about the same tool. *(Number provisional until the merge queue
lands it.)* **Parts 1 and 4 landed 2026-09-19** (`milestone/324-board-console-write-mode`); parts 2
and 3 are untouched and each is startable on its own.

**Gate: NONE.** It was `DECISION` on parts 3 and 4 when this block was minted; calef answered both on
2026-09-19, in the order this block asked for. A lane can take every remaining part today.

**Part 4, decided: yes, it writes, under a mode that names its purpose.** The invariant in
`script/board-console`'s header changes from *"it reads and never writes to the board"* to **it
writes only what a named mode sends, and every byte it sends is printed into the log**. That second
clause is not decoration: the log is the artifact a bench run is judged from, and a byte the board
received that the capture does not show makes the capture a lie about the run.

The reason for *narrow* rather than a general typing mode is an incident rather than a preference.
Milestone 249's lane sent the escape byte by detaching the console, and its block records what that
cost: *"Sending it at the wrong moment also stopped U-Boot's autoboot countdown and cost a power
cycle."* The hazard is not writing; it is an open keyboard beside a countdown a stray byte consumes.
So the shape is 249's own proposal, `--stop` and `--stop-after <n>`, each a command with a purpose.

**The reason this block offered for answering "no" had expired, and that is worth recording.** Part
4 was filed on the possibility that *"Milestone 218 may remove the need by fixing autoboot"*. 218 is
BUILT and was confirmed on a real boot on 2026-09-16, with the countdown expiring and nobody typing
(`bench/radon-2026-09-16/tour-083200.log`). It removed the four `StarFive #` commands and did not
touch 249's escape, which had created a second reason to write thirteen days earlier.

**Part 3, decided: one tool with a board profile, and the profile is the firmware prologue only.**
A tool each would fork 3,545 lines whose portable majority is already proven shared: `script/soak-test
--arch <a>` runs on all three architectures and its own header says the judging is literally
`script/board-console`'s code, exit statuses included.

**And the split the profile needs is already in the source, undocumented as a design.** `Stage` in
`crates/board_console/src/progress.rs` is two things in one enum. `Spl`, `OpenSbi`, `UBoot` and
`Handoff` are radon's firmware chain. `Banner`, `Machine`, `SelfTest`, `Tour` and `Soak` are the
kernel's own ladder, and that half's doc comments already record it as reachable on all three
architectures since milestone 268. A board profile is a firmware prologue and nothing else; the rest
is shared and always was.

**Argon is deferred rather than profiled, and the scope note is the point.** It has never booted
nife and sits behind milestone 127, which is NOT-STARTED. A profile written for it today would be a
Jetson boot chain read out of vendor documentation and never watched on a wire, which is the
assertion-shaped-as-measurement failure this tree keeps catching. Its prologue stays unwritten until
a board prints something.

**Why a cluster.** `script/board-console` and `crates/board_console` are one tool, and four lanes
filed four limitations against it from four directions: milestone 247's sweep of 216's block (twice),
milestone 249's boot-lottery lane, and milestone 168's job-mix lane. Each is small. Together they are
one statement: **the bench console is a reader, for one board, that cannot tell every finished run
from a wedged one**, and every one of the three boards is a machine somebody currently has to walk
to.

That matters beyond tidiness because `design/fatal-risks.md`'s remaining experiments are
hardware-gated in the sense that means *a person has to sit at the board*: milestones 16, 168 and 201
all wait on hands rather than on hardware. A console that can drive a board is the difference between
those being a bench session and being a job.

## The four parts

1. **BUILT 2026-09-19. It cannot stop a reboot loop.** Milestone 249's self-rebooting soak is
   stopped by pressing a key, so nine draws is a whole evening at the board. A mode on a script that
   already holds the port. Found by milestone 249's lane, which built the escape this would
   automate. **Part 4's answer makes this the smallest part rather than the blocked one**: `--stop`
   sends the byte and logs that it sent it, and `--stop-after <n>` ends a series with exactly the
   sample it was asked for, which is 249's own wording for what a writing mode buys. See *What
   landed* below, and read its second paragraph before extending this: the obvious trigger is the
   wrong one.
2. **Outstanding. It cannot tell a finished job-mix sweep from a wedged one.** `crates/board_console` recognises
   the boot sequence and, since milestone 219, a soak's stages and heartbeat; the job-mix sweep has
   no recogniser, so its exit status cannot distinguish the two. Found by milestone 168's lane.
3. **Outstanding. It serves two boards of three, not one, and the third is not ready to be served.** The part was
   filed on 2026-09-03 saying *"it serves radon only"*, and that was true then. On **2026-09-17** it
   captured xenon's boot over `/dev/cu.usbserial-A28FR8LZ`; the capture is in the tree at
   `bench/xenon-2026-09-17/first-light-095500.log` and a `--replay` of it on 2026-09-19 reports the
   banner, the machine line, the five-of-five self-test and the measured-boot refusal with the right
   diagnosis. Nothing was added for xenon, because xenon boots through PVH straight into our banner
   and the portable half of `Stage` is its whole boot. So what is left of this part is radon's
   firmware prologue being hard-coded rather than declared, which is refactoring against a working
   second case rather than a port. Found by milestone 247's sweep, from milestone 216's block.
4. **BUILT 2026-09-19. It cannot type at a board, and it should be able to.** Decided above: a
   named mode, its bytes logged, not a keyboard. What a lane builds is `--stop`, `--stop-after <n>`,
   and the rewritten invariant in `script/board-console`'s header. `notes/board-console.md` already
   carried the ruling beside each limitation it supersedes, and said plainly that until this lane
   landed the tool still behaved as the note described; **those sentences are now true rather than
   pending.** Found by milestone 247's sweep, from milestone 216's block.

## What landed, 2026-09-19: parts 1 and 4

`--stop` and `--stop-after <n>` on `script/board-console`, the rewritten invariant in its header,
and the same invariant in `crates/board_console/src/lib.rs` and `notes/board-console.md`. The engine
is a new module, `crates/board_console/src/stop.rs`, whose header carries the argument; the reading
loop was not duplicated, it took one parameter (`watch::watch_with`).

**The invariant now reads**: *it writes only what a named mode sends, and every byte it sends is
printed into the log.* That is a mechanism rather than a promise. `Escape::observe_line` is the only
thing in the crate that writes to a port, and it writes the announcement into the log and flushes it
**before** the byte goes out, so the two failure orders are the readable one: a log that says a byte
is going and then says the write failed is honest, and there is no path that puts a byte on a wire
the log does not name. The byte is rendered as `0x0d` and never written raw, so a capture stays
greppable and a replay of it cannot be fooled; a host test asserts that no line this mode writes
reads as a boot marker.

**The finding worth carrying out of this lane is that the obvious trigger is the wrong one, and
silently so.** The natural place to send the escape is `soak-test: started`, the line that reaches
`Stage::Soak`. That would lose. `kernel/src/soak.rs` prints `START_MARKER`, then four more lines,
and *then* calls `arm_reboot`, whose first statement is `console::discard_rx`: about a kilobyte
between the two, roughly ninety milliseconds at 115200 baud, and a host that reads a line and
answers it is comfortably inside that. The byte would be thrown away by the kernel's own drain, on
almost every run, and the failure would look exactly like a board whose receive path is miswired.

So the trigger is the **arming announcement** (`THIS BUILD REBOOTS THE BOARD`), which `arm_reboot`
prints after the drain, and nothing else in a session can cause a byte to go out. That one gate
answers every state the ruling did not cover, in the same way and with the same answer, **nothing is
sent**: a board powered off, a board at a U-Boot prompt, a board at a `swish` prompt, a kernel with
no soak in it, a soak with no reboot loop. It also closes the hazard the ruling is actually about
without relying on the byte's value, because a kernel that has printed that banner has been running
for the length of a boot tour and U-Boot's countdown is long over. The price is one sentence and it
is recorded in three places: a session attaching to a board already mid-draw waits out that draw,
up to two minutes, before a banner arrives.

**A `--stop` is also the first thing in this tree that can verify the soak escape end to end.**
`kernel/src/soak.rs`'s own `BUGS` says the escape is a poll of one bit and that *"nothing in this
kernel can prove otherwise, because a UART cannot receive a byte it sends"*, and names a procedure
as the substitute: press a key on the first boot and confirm `DISARMED` before walking away. A host
holding the far end is not under that limit. `--stop` sends a byte it did not print and waits
fifteen seconds (three of the kernel's five-second polls) for the board's own `DISARMED` line, so
that procedure becomes a command with an exit status, which is rung four moving up to rung two.
notes/soak.md's step 4 says so where a reader meets it.

**No exit status was added.** Confirmed is `0`; sent-and-unacknowledged and never-sent are both `3`,
the time running out with the requested thing unreached, which is what `3` already meant. A failure
the board announced (`1`) and a hang (`2`) still win, because those are facts about the board where
an unconfirmed stop is a fact about this tool.

**What could not be tested, stated plainly: no byte has reached a board.** This lane had no board
attached to it and neither radon nor xenon was reachable, so nothing here was run on hardware and
nothing in this block should be read as saying otherwise. What is tested is the decision and the
bytes: eleven host tests in `stop.rs` drive the state machine against transcripts with a `Vec<u8>`
standing in for the port, so a test asserts the exact bytes a board would have received, and two
more in `watch.rs` run the same thing through the real loop. What that leaves unproven is that a
write to the descriptor reaches the UART and that the kernel's poll finds it. **The first real
`--stop` is the experiment and its `DISARMED` line is the result**; it is the same shape as the gap
milestone 249 already carries, one milestone on.

**Two things this lane chose that calef may want to overrule**, both cheap to change and both
recorded where a reader meets them rather than only here:

- **The byte is a carriage return.** The kernel takes any byte, so this is a choice inside a fixed
  contract rather than a wire value two programs agree on. Carriage return because the kernel's own
  instruction is *"press any key"* and Enter is what a person presses when told that, so the tool
  runs the same experiment the documented manual procedure runs. `NUL` was the other candidate and
  the refusal is recorded beside the constant.
- **`--for` defaults to 150 seconds per requested draw plus two minutes** when a stop mode is given
  and `--for` is not, derived from `kernel/src/soak.rs`'s two-minute draw plus a boot. It is a
  default and not an agreement (getting it wrong costs a re-run, never a wrong answer), which is why
  the number is duplicated rather than hoisted into a shared crate.

**One piece of identified work with no home yet, recorded in `stop.rs`'s `BUGS` where the next
reader meets it**: the soak's console markers are string literals in three places (the kernel, the
recogniser, and now this module), agreeing by a reader having checked rather than by the compiler.
`crates/boot_ladder` exists for exactly that and holds the boot tour's markers as shared constants;
`START_MARKER`, `REBOOT_MARKER` and the arming and disarming lines were never hoisted into it. That
is a change to the kernel and is outside parts 1 and 4.

**Parts 1 and 4 are close enough to collide**, and that is an argument for the cluster rather than
against it: part 1 wants to send one keystroke and part 4 asks whether sending anything is allowed.
Answering 4 first makes 1 either trivial or refused, and nothing in four separate proposals said so.
**It was answered first, on 2026-09-19, and it made 1 trivial**, which is the cluster earning its
keep in the one way a cluster can.

**The title changed with this ruling, and the file was renamed with it.** It read *"reads one board
of three and cannot speak to any of them"* when minted on 2026-09-18, and the first half of that was
already false: xenon had been captured the day before. Only the generated index cited the path, so
the rename cost nothing, and a block whose own H1 states a falsehood is the thing this tree objects
to hardest.

**Two of the four parts had decayed by the time they were read**, the same as milestone 323's two.
Part 4's reason for a possible "no" was spent by milestone 218 on 2026-09-16, and part 3's premise
was falsified by xenon's own capture on 2026-09-17, both after filing and both before promotion on
2026-09-18. See 323's *What the two decisions cost*, which argues that promotion is where a stale
premise is cheapest to catch; four instances across two clusters is the evidence for it.

## Follow-on

- **Outstanding.** Part 2, the job-mix sweep recogniser. Untouched by this lane and unaffected by
  it: `crates/board_console/src/progress.rs` still has no marker for a sweep, so a finished one and
  a wedged one still share an exit status. Checked 2026-09-19 by reading `observe`, which matches
  the boot ladder, the soak's start and beat lines, and nothing of `script/job-mix`'s.
- **Outstanding.** Part 3, radon's firmware prologue declared rather than hard-coded. Untouched, and
  the split the profile needs is still exactly where this block says it is: `Stage`'s lower half is
  the VisionFive 2's chain and its upper half is the kernel's, reachable on all three architectures
  since milestone 268. This lane added nothing to either half, because a writing mode is a mode
  rather than a stage. Checked 2026-09-19.
- **Recorded.** *The soak's console markers are string literals agreeing by a reader having checked,
  in three places now rather than two*, in the `BUGS` of `crates/board_console/src/stop.rs`.
  `crates/boot_ladder` is where the boot tour's markers were hoisted for exactly this reason, and
  the soak's never were. It is a kernel change and outside parts 1 and 4.
- **Recorded.** *No byte of the writing mode has reached a board*, in the `BUGS` of
  `crates/board_console/src/stop.rs` and of notes/board-console.md, and in *What landed* above. The
  first bench run closes it, and its `DISARMED` line is the result rather than a green check.
- **Recorded.** *A `--stop` against a board already mid-draw waits that draw out*, in the same two
  `BUGS` sections. It is the price of the gate that makes every other state safe, and it is a
  property of the kernel's announcement being once per boot rather than something this tool can fix.
- **Refused.** A general typing mode, or `--send <text>`. It is the thing the ruling declined and
  the argument is an incident rather than a preference: an open keyboard beside U-Boot's autoboot
  countdown cost milestone 249's lane a power cycle. A second named command with its own purpose is
  the shape to add when one is wanted; a keyboard is not.
- **Refused.** Hoisting `--stop`'s confirmation window and the draw length into a crate shared with
  the kernel. They are a timeout and a default, not an agreement: getting either wrong costs a
  re-run with `--for`, never a wrong answer, and §46's line is that a shared crate is for what two
  binaries must agree on. The markers above are the thing that does qualify.

## Index row

`script/board-console` watches a board's serial port and could not type at it, hard-codes radon's
firmware chain, and cannot tell a finished job-mix sweep from a wedged one. Four lanes filed those
separately between 2026-09-03 and 2026-09-04; read together they say the bench console is a reader.
That is load-bearing rather than cosmetic, because the remaining fatal-risk experiments (milestones
16, 168, 201) are gated on a person sitting at a board rather than on hardware existing, and a
console that can drive one is what turns those from a bench session into a job. calef decided both
gated parts on 2026-09-19: it writes, under a named mode whose bytes are logged, and the three boards
share one tool whose board profile is the firmware prologue alone, with argon deferred behind
milestone 127 rather than guessed from vendor documentation. **Parts 1 and 4 landed the same day**:
`--stop` and `--stop-after <n>` end milestone 249's rebooting soak from a script, the byte goes out
only on the board's own arming announcement and never before it, and every byte sent is printed into
the log in hex. No `--stop` has yet run against a board. Parts 2 and 3 are untouched.
