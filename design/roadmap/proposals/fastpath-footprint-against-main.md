# Should the fastpath footprint gate compare against `main` instead of a stored baseline

**Status: PROPOSED 2026-09-03.** Written by the milestone 247 sweep, from milestone 237's block.

**Gate: DECISION.** It is calef's call, and it is the shape of the gate rather than a bug in it.
Both options have real costs and the tree has evidence for each. Nothing should be built until the
question is answered, because the two designs share almost no code.

**In brief.** `script/fastpath-footprint` measures the IPC fastpath's code size against a **stored
baseline file**, which somebody has to remember to re-record when growth is understood and accepted.
The alternative is to measure against `main` at the time the pull request runs, so the comparison
has no stored state and cannot go stale. Milestone 237 fixed one instance of the stored baseline
going stale and explicitly did not touch the mechanism. The work is to decide which shape the gate
takes, and then to build it.

## Why this matters

The stored baseline has already failed in the way it fails. Milestone 237 records it: **two lanes
each measured "within bound" against the same stale baseline and neither re-saved it, so aarch64
headroom fell from 3.9 points to 1.5 with nothing firing.** The gate was green throughout. Nothing
in the mechanism can distinguish "this change is small" from "this change is small and the last four
were not."

Leaving it undecided is the worse outcome of the two, because it means the failure recurs and gets
patched per instance. Milestone 237 was that patch. There are two more baselines waiting to do the
same thing.

## The costs on each side, which is the material calef needs

**A stored baseline** is an absolute budget. It answers "how big is the fastpath allowed to be",
which is the question that matters for a claim about this kernel, and it catches slow accumulation
across many small changes because the reference does not move. Its failure mode is the one already
observed: the reference goes stale, growth is absorbed, and re-recording is a manual act nobody
owns.

**Comparing against `main`** is a delta budget. It cannot go stale and needs nobody to remember
anything, which is a full rung up AGENTS.md's ladder. Its failure mode is the inverse and is worse
in the long run: every individual change is within bound and the absolute size grows without limit,
because the reference moves with the tree. It also costs a second build per run, since `main` has to
be built to be measured.

The third shape, which neither the block nor this proposal has priced, is **both**: a delta check
against `main` for the pull request plus an absolute ceiling that only calef raises. That is more
machinery, and whether the extra check earns its keep is part of the same decision.

## Where it came from

Milestone 237's `## Follow-on`: *"Whether the fastpath footprint gate should compare against `main`
rather than a stored baseline file. It is calef's call and has costs on both sides. Two lanes each
measured 'within bound' against the same stale baseline and neither re-saved it, so aarch64 headroom
fell from 3.9 points to 1.5 with nothing firing. This milestone fixed the instance, not the
mechanism."*
