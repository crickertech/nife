# 247. Follow-on work named by a finished milestone goes nowhere, and this is the third time

**Status: NOT-STARTED.** Minted 2026-09-03 by calef, after milestone 244 named work that would have
been buried had he not asked for it by name. *(Number provisional until the merge queue lands it.)*

**Gate: NONE.** The rule already exists; what is missing is anything that notices when it is not
followed.

**In brief.** calef, 2026-09-03: *"we keep burying work identified by completed milestones."*

There are **151 blocks marked BUILT**. Each was allowed to finish, and a finished milestone's block
is the last place anyone looks. The work those blocks named on their way out is scattered through
`BUGS` sections, "handoff" paragraphs and lane reports, and **nothing in this tree can tell which of
it became a milestone and which of it evaporated.**

## This is the same failure three times, which is what makes it a milestone and not a chore

AGENTS.md already carries the rule, in as many words:

> Identified work leaves the lane in a tracked form, or the merge waits.

And it already carries the reason the rule does not hold, which is that the rule is **rung four**:

> A lane report is read once, by one person, on the day it is written. A pull request body is read
> while the diff is open and never again. Both feel like records while you are writing them, which is
> what makes this the failure that recurs.

The recurrences are on the record and they are not near-misses:

- **Milestone 90** exists only because calef happened to be at his desk the day a report named it.
- **Milestone 94** swept the tree for exactly this category and then **left its own inventory in a
  pull request body for twelve days**, by which point the item-level list was gone and had to be
  re-derived (notes/untracked-work-sweep.md).
- **Milestone 244**, on the day this was minted, named an unvouched-binary hazard in a `BUGS` section
  and a design fork in a handoff paragraph. Both were surfaced because calef asked *"any work to
  follow up on 244?"* by hand. That question is the mechanism today, and it is rung zero: somebody
  has to remember to notice.

## The two halves, and the second is the one that lasts

**A sweep**, which is the part that is owed now: read the 151 BUILT blocks and their `BUGS`
sections, and for every piece of named follow-on work decide one of three things. It became a
milestone (say which). It is a recorded limitation and stays one (say where). Or it is neither, and
then it gets minted or explicitly refused. **An explicit refusal is a success**; the defect is
silence, not the absence of a milestone.

**And a mechanism**, because a sweep that is not made repeatable is milestone 94 again with a new
number. The shape is not specified here, because choosing it *is* the milestone. What the shape has
to survive is stated instead:

- **It cannot be a lint that greps prose for intentions.** AGENTS.md priced that already: `git grep -w
  TODO` runs an 82% false-positive rate, and a check that cannot tell an observation from an intention
  will be disabled within a week.
- **It must not weaken the `BUGS` convention.** A recorded limitation is a fact a reader meets beside
  the feature, and it is the FreeBSD posture working as designed. The goal is to route intentions
  *into* the tracked forms, never to drive limitations out of `BUGS`.
- **The likely rung is two**, and the tree already has the pattern: a marker with a resolvable
  referent. `script/lint` already fails a `TODO` that does not name a milestone, and the citations
  gate already checks that every glossed citation is grounded in the document it names. A block that
  proposes follow-on work in a form a script can find, resolving to a minted block or to a stated
  refusal, is the same shape a third time.

## The proof that this milestone worked

**Two things, and the sweep alone is not enough.**

1. A written disposition for every piece of follow-on work named by a BUILT block: minted, recorded,
   or refused with a reason.
2. **Something that fails when a new one is buried**, demonstrated by burying one and watching it go
   red. Without this the sweep has a shelf life and milestone 94 already measured it at twelve days.

## BUGS

- **151 blocks is a lot of prose and the sweep will be imperfect.** A sweep that finds most of it and
  ships the mechanism is worth more than an exhaustive one that does not, and a partial sweep should
  say what it did not read rather than implying it read everything.
- **This cannot recover what was never written down.** Work named only in a chat window or a lane
  report that nobody landed is not in the tree to be found, and the count of what was lost is
  unknowable rather than zero.
- **A mechanism that is too strict will be routed around.** If every observation in a `BUGS` section
  has to resolve to a milestone, the honest thing to write becomes expensive and people will write
  less, which costs more than the burial does. Whatever ships has to leave "this is a limitation and
  it stays one" cheap to say.
