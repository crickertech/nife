# Count how many of the tree's programs can fault under a shell

**Status: PROPOSED 2026-09-03.** Written by the milestone 247 sweep, from milestone 235's block.

**Gate: NONE.** Nothing is owed and nothing is missing. It is a survey of code already in the tree,
and a lane could start it today.

**In brief.** Milestone 235 fixed a shell that hangs forever when a spawned command traps. Nobody
knows how many programs could trigger it. The tree ships **68 programs**, and every one that can
fault reached that path. The work is to go through them and produce a number: which programs have a
reachable fault (an unwrapped index, a `panic!` on bad input, a syscall that returns an error the
program does not handle), and which cannot fault by construction.

## Why this matters

Right now the value of milestone 235's fix is unmeasured. It is either a repair to a defect two
programs could hit or a repair to one that forty could, and those are different claims about how bad
the prompt's behaviour was. The block that fixed it cannot say which, and neither can anybody else.

The survey also produces something the fix did not: a list. A program that can fault under a shell
is a program whose failure mode a user will eventually meet, and knowing which ones those are is the
input to deciding whether any of them should not be able to fault at all.

The honest caveat is that this is a measurement rather than a repair. It changes no behaviour, and
its whole output is a number and a table in a note. That is a real cost to weigh against promoting
it, and it is why the bar is a number a claim rests on rather than a bug a user hits.

## Where it came from

Milestone 235's `## Follow-on`: *"Count which of the tree's 68 programs can fault under a shell, so
the exposure of this defect class is a number rather than a guess. Every program that can fault hit
this path and nobody has looked, so there is no way to say whether the fix mattered to two programs
or to forty."*
