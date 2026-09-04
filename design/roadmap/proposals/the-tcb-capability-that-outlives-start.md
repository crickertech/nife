# Five spawn sites keep a thread capability past `START` and none of them says why

**Status: PROPOSED 2026-09-03.** Found by the `maintainer/research-spawn-retention` lane while
counting what a spawner retains over a child, for the decision that milestone 133 (ending a
permanently blocked thread, and deciding who may) is downstream of.

**Gate: NONE.** Nothing is owed and nothing is blocked. It is a small tidy that becomes a real one
the moment a retention convention is chosen, so it is worth being on a list rather than in a lane
report.

**In brief.** Thirty `START` sites exist in shipping and test code. Twenty-five call `cap_delete` on
the `ThreadControlBlock` capability immediately afterwards. Five do not, all in `user/src/hello.rs`
(lines 468, 500, 523, 671 and 723 at `49db708f`), and none of them records a reason. Either delete
the capability at those sites too, or write one line at each saying why the kernel's own test
program keeps one.

## Why it is worth a line

The capability is inert after `START`: every method on it refuses a thread that is not an `Embryo`,
checked in `kernel/src/sched.rs` rather than in the dispatcher. So nothing is currently wrong, and
that is exactly what makes this the shape AGENTS.md's ladder warns about. The five sites are an
unmarked exception to a convention twenty-five sites follow and nobody wrote down, so a reader
meeting them cannot tell a decision from an oversight, and the next spawn path copies whichever one
it happened to read.

It stops being cosmetic if the retention question is answered in the direction of a lifetime handle.
Under that answer these five sites would silently acquire an authority nobody chose to give them,
which is the one cost that cannot be found by reading the change that introduces it.

## What doing it looks like

The parents at all five sites `exit()` within a few lines, so deleting is free and changes no
behaviour: the slot is reclaimed with the thread's whole table. The alternative, a comment at each
site saying the capability is deliberately abandoned because the holder is about to exit, is equally
acceptable and is the cheaper answer if someone would rather not touch a file the kernel's test
suite reads. What is not acceptable is leaving both readings available.
