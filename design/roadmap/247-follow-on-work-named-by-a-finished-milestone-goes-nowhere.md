# 247. Follow-on work named by a finished milestone goes nowhere, and this is the third time

**Status: IN-PROGRESS** on `milestone/247-buried-followon`. Minted 2026-09-03 by calef, after milestone 244 named work that would have
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

## What was built, 2026-09-03

**The mechanism.** A `## Follow-on` section on every BUILT or REMOVED block, gated by
`script/roadmap --check` and therefore by `script/lint`. Seven dispositions, each resolving to
something a script can check: `None.`, `Milestone N.` (the block must exist), `Done.` (what carried
it), `Recorded.` (any path it cites must exist), `Refused.` (a reason), `Decision.` (a file under
`design/decisions/`), `Unclaimed.` (what the work is). The vocabulary is tabulated in
design/roadmap/README.md and argued in notes/follow-on-work.md.

It hangs on the **status** rather than on a marker in prose, which is what keeps it off the rung
this block ruled out. A block turning BUILT is the moment the burial happens and it is a state a
script can see; nothing here greps prose for intent, so `git grep -w TODO`'s 82% false-positive rate
does not apply. Third instance of a shape the tree already has, after the TODO gate and the
citations gate.

**The proof, which is the half this block said the sweep alone could not give.** Four burials were
staged and each went red: a finished block with the section removed, a bullet naming a milestone number
nothing has, beside a `Recorded.` pointing at a file that does not exist and a bare `Refused. Later.`, a block
parked on the (then existing) exemption list after its `Built` date, and a block left on that list
after it grew a section. The gate also found rot nobody staged: three roadmap blocks citing paths
that renames had moved out from under them.

**The sweep.** All 139 finished blocks read in full, in twelve parallel lanes, about 159,000 words.
605 dispositions: 291 `Recorded.`, which is the `BUGS` convention working as designed, and 136
`Milestone N.`, usually the successor the block had already cited by number.

**And 42 pieces of work that were named and never taken, across 32 finished milestones**, which is the number this block existed to
produce and which nothing in the tree could see before. Five are recommended for minting in
notes/follow-on-work.md, on a stated bar: a claim this project makes rests on it, or a record is now
known to be wrong. The rest stay `Unclaimed.` and queryable through `script/roadmap --unclaimed`.

**Two dispositions were added by the sweep rather than designed before it**, and that is the
finding worth keeping. `Unclaimed.` and `Done.` were each asked for by several lanes independently,
on the same afternoon, having hit the same wall: work a block named honestly that nobody took, and
work a block named that two ordinary commits then finished. Neither fits the four words the gate
shipped with. The lanes that would not write a comfortable lie **left those items out**, which is
this milestone's own failure mode arriving through this milestone's own mechanism.

## Follow-on

- **Unclaimed.** The 42 items the sweep found are listed nowhere but in the blocks that name them,
  which is deliberate (they are queryable through `script/roadmap --unclaimed`) and means nothing
  ages them. An entry written today and one that has sat for a year look identical, and nothing
  escalates either. Whether the backlog wants a staleness signal is open.
- **Recorded.** In `notes/follow-on-work.md`: `Unclaimed.` and `Recorded.` are separated by
  judgment rather than by anything checkable, so the backlog is under-counted rather than
  over-counted. `Recorded.` is the more comfortable word and the same text can defensibly take
  either.
- **Recorded.** In `notes/follow-on-work.md`: the gate fires once, when a block finishes. Follow-on
  work identified after that lands in a block nothing re-checks, because the section already exists
  and already passes.
- **Decision.** The seven disposition words are a lane's and calef names things. He minted `REMOVED`
  in the status vocabulary himself, so this one is the same shape one level down; the ratification
  ask is `design/decisions/140-follow-on-disposition-vocabulary.md`, which also points at the
  sweep's five proposed milestones.
