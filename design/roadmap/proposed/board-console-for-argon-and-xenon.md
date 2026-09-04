# The bench console serves one board out of three

**Status: PROPOSED 2026-09-03.** Written by the milestone 247 sweep, from milestone 216's block.

**Gate: DECISION.** The behaviour is the same on all three boards; what differs is the banners and
the boot sequence. Whether argon (aarch64) and xenon (x86_64) get this console through a board
profile or through a tool each is calef's call, and it decides what a lane would build. The building
after that is small.

**In brief.** `script/board-console` was built for radon, the VisionFive 2. It opens the port, tees
every byte to a log that is never optional, recognises the runbook's boot sequence, and returns a
different exit status for each way a session can end. Argon and xenon have no console tool at all.
Extending it means either a board profile carrying the banners and the expected sequence per board,
or a separate tool per board that shares the reading half.

## Why this matters

The bench workflow milestone 216 built exists for one board out of three, and the two without it are
the two on architectures where parity is a gate. Every hardware milestone on argon or xenon still
waits on a person watching a screen, which is the exact condition 216 was written to end. The cost
is not theoretical: milestone 195's `BUGS` says none of the UEFI path is proved on a Dell, and
milestone 215 lists three things only xenon can confirm. Each of those is a bench session with no
tool and no log.

The naming half is what makes it a decision rather than a lane's judgment. `script/board-console`,
`cargo xtask board-console` and `crates/board_console` all shipped provisional, and a per-board tool
would mint two or three more names in a family that is not settled yet. A profile mints none.

## The two options, and what each costs

A **board profile** keeps one tool and one name, and the per-board knowledge becomes data: the
port, the speed, the banners, the ordered stages a session can reach. The risk is that the three
boot sequences differ more than a table can express, at which point the profile grows conditionals
and becomes three tools wearing one name.

A **tool per board** admits the differences up front and keeps each one readable. It costs three
names in a provisional family, three places for the reading half to drift apart unless it is a
shared crate, and three things a newcomer has to know exist.

Reading argon's and xenon's actual boot output beside radon's is what would decide this, and it is
a lane's work rather than an argument.

## Where it came from

Milestone 216 (nothing in this tree can read a board) named it: *"Whether argon (aarch64) and xenon
(x86_64) get this console with a board profile or a tool each. Same behaviour, different banners and
a different boot sequence, and the choice is calef's. Until it is made the other two boards have no
console tool at all, so the bench workflow this milestone built exists for one board out of three."*
