# 324. The bench console cannot speak to any of the three boards

**Status: NOT-STARTED.** Minted 2026-09-18 by calef, promoting a cluster rather than its members:
four proposals from three lanes, all about the same tool. *(Number provisional until the merge queue
lands it.)*

**Gate: NONE.** It was `DECISION` on parts 3 and 4 when this block was minted; calef answered both on
2026-09-19, in the order this block asked for. A lane can take every part today.

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

1. **It cannot stop a reboot loop.** Milestone 249's self-rebooting soak is stopped by pressing a
   key, so nine draws is a whole evening at the board. A mode on a script that already holds the
   port. Found by milestone 249's lane, which built the escape this would automate. **Part 4's
   answer makes this the smallest part rather than the blocked one**: `--stop` sends the byte and
   logs that it sent it, and `--stop-after <n>` ends a series with exactly the sample it was asked
   for, which is 249's own wording for what a writing mode buys.
2. **It cannot tell a finished job-mix sweep from a wedged one.** `crates/board_console` recognises
   the boot sequence and, since milestone 219, a soak's stages and heartbeat; the job-mix sweep has
   no recogniser, so its exit status cannot distinguish the two. Found by milestone 168's lane.
3. **It serves two boards of three, not one, and the third is not ready to be served.** The part was
   filed on 2026-09-03 saying *"it serves radon only"*, and that was true then. On **2026-09-17** it
   captured xenon's boot over `/dev/cu.usbserial-A28FR8LZ`; the capture is in the tree at
   `bench/xenon-2026-09-17/first-light-095500.log` and a `--replay` of it on 2026-09-19 reports the
   banner, the machine line, the five-of-five self-test and the measured-boot refusal with the right
   diagnosis. Nothing was added for xenon, because xenon boots through PVH straight into our banner
   and the portable half of `Stage` is its whole boot. So what is left of this part is radon's
   firmware prologue being hard-coded rather than declared, which is refactoring against a working
   second case rather than a port. Found by milestone 247's sweep, from milestone 216's block.
4. **It cannot type at a board, and it should be able to.** Decided above: a named mode, its bytes
   logged, not a keyboard. What a lane builds is `--stop`, `--stop-after <n>`, and the rewritten
   invariant in `script/board-console`'s header. `notes/board-console.md` already carries the ruling
   beside each limitation it supersedes, and says plainly that until this lane lands the tool still
   behaves as the note describes. Found by milestone 247's sweep, from milestone 216's block.

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

## Index row

`script/board-console` watches a board's serial port and cannot type at it, hard-codes radon's
firmware chain, and cannot tell a finished job-mix sweep from a wedged one. Four lanes filed those
separately between 2026-09-03 and 2026-09-04; read together they say the bench console is a reader.
That is load-bearing rather than cosmetic, because the remaining fatal-risk experiments (milestones
16, 168, 201) are gated on a person sitting at a board rather than on hardware existing, and a
console that can drive one is what turns those from a bench session into a job. calef decided both
gated parts on 2026-09-19: it writes, under a named mode whose bytes are logged, and the three boards
share one tool whose board profile is the firmware prologue alone, with argon deferred behind
milestone 127 rather than guessed from vendor documentation.
