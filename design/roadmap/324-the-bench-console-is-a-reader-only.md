# 324. The bench console reads one board of three and cannot speak to any of them

**Status: NOT-STARTED.** Minted 2026-09-18 by calef, promoting a cluster rather than its members:
four proposals from three lanes, all about the same tool. *(Number provisional until the merge queue
lands it.)*

**Gate: DECISION.** Parts 3 and 4 only; **parts 1 and 2 need nobody.** What is calef's is whether
argon and xenon get this console through a board profile or a tool each, which decides what a lane
builds, and whether the tool should ever write to a serial port at all. `NONE` stands alone and would
claim nothing is owed, which is false here.

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
   port. `design/roadmap/proposals/board-console-cannot-speak-to-the-board.md`
2. **It cannot tell a finished job-mix sweep from a wedged one.** `crates/board_console` recognises
   the boot sequence and, since milestone 219, a soak's stages and heartbeat; the job-mix sweep has
   no recogniser, so its exit status cannot distinguish the two.
   `design/roadmap/proposals/a-board-console-recogniser-for-the-job-mix-sweep.md`
3. **It serves radon only.** The behaviour is the same on all three boards; the banners and boot
   sequences differ. Board profile or a tool each is calef's call.
   `design/roadmap/proposals/board-console-for-argon-and-xenon.md`
4. **Whether it should type at a board at all**, and if so whether that is this tool with an explicit
   mode or a second one. Milestone 218 may remove the need by fixing autoboot, so the answer may be
   no. `design/roadmap/proposals/board-console-writes.md`

**Parts 1 and 4 are close enough to collide**, and that is an argument for the cluster rather than
against it: part 1 wants to send one keystroke and part 4 asks whether sending anything is allowed.
Answering 4 first makes 1 either trivial or refused, and nothing in four separate proposals said so.

## Index row

`script/board-console` watches radon's serial port and cannot type at it, cannot serve argon or
xenon, and cannot tell a finished job-mix sweep from a wedged one. Four lanes filed those separately
between 2026-09-03 and 2026-09-04; read together they say the bench console is a reader for one board
out of three. That is load-bearing rather than cosmetic, because the remaining fatal-risk experiments
(milestones 16, 168, 201) are gated on a person sitting at a board rather than on hardware existing,
and a console that can drive one is what turns those from a bench session into a job.
