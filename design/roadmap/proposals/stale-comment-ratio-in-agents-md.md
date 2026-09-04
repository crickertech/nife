# The comment ratio AGENTS.md quotes is wrong, and it climbs every week

**Status: PROPOSED 2026-09-03.** Written by the milestone 247 sweep, from milestone 234's block.

**Gate: DECISION.** `AGENTS.md` is calef's file and the correction is a one-line edit only he makes.
The measurement is already done and stored, so nothing else is owed: what is missing is the edit and
a decision about whether a hand-copied number belongs in that file at all.

**In brief.** `AGENTS.md` tells a reader that `kernel/src` "measures 40% of them" when discussing
comments in the line count. Milestone 234's weekly series measures that ratio directly: **39.3%
eight weeks ago and 45.3% now**, climbing every week. The figure was true when it was written and is
not true today. The work is to correct it, and to decide whether the sentence should quote a number
at all or point at `notes/project-metrics/weekly.csv`, which cannot go stale.

## Why this matters

This is the project's front-door file. A newcomer reads it before they read anything else, and
AGENTS.md's own third principle is that a stranger must be able to succeed on the tree alone. A
wrong number there is the exact defect that principle names: *"a newcomer who hits a limitation the
docs named will trust the docs. One who hits a limitation the docs hid will not trust anything
again."*

It is also drifting rather than merely stale. Correcting 40% to 45.3% buys a few weeks and then the
same bullet is true again, which is why the second half of this is a question and not just an edit.
A pointer at the series is rung one, since the series recomputes itself; a hand-copied percentage is
rung four and has now demonstrated its failure mode.

## Where it came from

Milestone 234's `## Follow-on`: *"Correct AGENTS.md's 'kernel/src measures 40% of them' comment
ratio. The series puts it at 39.3% eight weeks ago and 45.3% now, climbing every week, so the figure
a reader meets in the project's own front-door file is wrong and drifting further from true.
AGENTS.md is calef's file, so the one-line edit is his to make; this page flags the staleness and
cannot fix it."*
