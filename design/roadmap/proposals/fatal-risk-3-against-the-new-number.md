# Re-read fatal risk 3 against the number the mutation sweep actually published

**Status: PROPOSED 2026-09-03.** Written by the milestone 247 sweep, from milestone 238's block.

**Gate: DECISION.** `design/fatal-risks.md` holds the nine claims that, if false, mean the project
should stop. A verdict on one of them is calef's and nobody else's. The measurement it needs is
already published, so nothing but the reading is outstanding.

**In brief.** `design/fatal-risks.md`'s third risk (the tests do not test anything, and the quality
is illusory) reads **MEASURED, green**, on 92.4% of viable mutants killed, dated 2026-08-03. The
first mutation report that workflow ever published says **83.4%**, on a uniform one-eighth sample
across all 60 crates. The work is to decide whether the verdict still holds at the new figure, and
to rewrite the entry either way.

## Why this matters

The file is stale in two separate ways and only one of them is the number.

The figure itself is from 2026-08-03 with **2,529 commits landed since**. That would be ordinary
drift. The second problem is the sentence beside it: the entry closes by saying *"the weekly
workflow already publishes the report,"* which tells a reader the refresh is arriving on its own.
Milestone 238 established that the workflow had never once succeeded, so that clause was false from
the day it was written, and it is what made the stale number feel acceptable. A reader trusting the
file today is told a green verdict is current and self-maintaining, and neither half is true.

83.4% is not obviously a failing score, which is the point. This is a judgment about a risk's
verdict, not an arithmetic comparison, and the two numbers are not measured the same way: 92.4% was
one corpus, 83.4% is a one-eighth uniform sample. Whether that is a real fall, a sampling artifact,
or a fall concentrated in a few crates is the reading. Milestone 244 already answered part of it for
`system_initializer`, and two more crates are named and unanswered.

## What is in hand to read it with

- `notes/mutation-testing.md`, which tabulates what the two partial runs did and did not cover.
  Seven eighths of the corpus is still unmeasured since 2026-08-03.
- Milestone 244's measurement of `system_initializer`, the crate carrying most of the fall, which
  closed `RECORDED` because the pure fraction a mutation can reach in it is small.
- `.cargo/mutants-baseline.txt`, which is what a score is supposed to be compared against.
- Two crates still unexplained, `uefi_loader` at 15% and `manual` at 52%, which have their own
  proposal.

The honest option list is three: the verdict holds and the entry gets the new number, the verdict is
downgraded from green, or it becomes provisional until a full sweep runs rather than a one-eighth
sample. The false clause about the workflow refreshing itself has to go in all three cases.

## Where it came from

Milestone 238's `## Follow-on`: *"Re-read fatal risk 3 against the new number, which is calef's
call. `design/fatal-risks.md` still reads MEASURED and green on 92.4% from 2026-08-03, while the
first published mutation report says 83.4% on a uniform one-eighth sample across all 60 crates.
Until somebody decides whether the verdict holds, that file carries a stale figure and tells a
reader the refresh arrives on its own."*
