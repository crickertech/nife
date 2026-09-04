# 140. The words a finished milestone may use to say what happened to the work it named

**Status: PROPOSED.** *(Number provisional. Section numbers are the integrator's at merge, per
AGENTS.md, and this one was written by the milestone 247 lane.)*

## What is being decided

Milestone 247 built a gate: every `BUILT` or `REMOVED` roadmap block now carries a `## Follow-on`
section saying what became of the work it named on its way out, and `script/roadmap --check` fails a
block that does not. **The mechanism is not in question here.** What is in question is the seven
words a bullet may open with, because a vocabulary is a thing readers learn and a thing every future
block is written against, and this tree already treats status vocabulary as calef's: he minted
`REMOVED` himself on 2026-08-30 when the six words then available could only lie about milestone 54.

The words, as shipped:

| Word | Means | Gate requires |
|---|---|---|
| `None.` | Nothing was identified | It stands alone |
| `Milestone N.` | It became milestone N | Block N exists, and is not this one |
| `Done.` | Done, and not as a milestone | Prose saying what carried it |
| `Recorded.` | A limitation, and it stays one | Prose; any path it cites must exist |
| `Refused.` | Considered, deliberately not taken | Prose giving the reason |
| `Decision.` | calef's call, written up as one | A file under `design/decisions/` |
| `Unclaimed.` | Named, nobody took it, not refused | Prose saying what the work is |

## What else was considered

**Four words, which is what shipped first thing that morning**: `None.`, `Milestone N.`, `Recorded.`,
`Refused.`. That set is what the roadmap block's own reasoning implies, and it survived contact for
about four hours.

**`Decision.` was added before the sweep**, on the first block read. Milestone 55 names a vocabulary
gap that is explicitly calef's call, and forcing it into `Refused.` would have been a lie in the one
place this gate exists to stop lying. It is not a new tracked form: AGENTS.md already says open
decisions live in a file rather than in a conversation.

**`Unclaimed.` and `Done.` were added by the sweep, not designed.** Twelve lanes read all 139
finished blocks. Three reported, independently and within the same hour, that there is no word for
work a block named honestly that nobody has taken; four reported that there is no word for work a
block named that two ordinary commits then finished. The evidence that this is the right call rather
than a convenience is what those lanes did while the words were missing: **they left the items out.**
That is the exact silence milestone 247 exists to stop, arriving through milestone 247's own
mechanism, and it is recorded in notes/follow-on-work.md.

## The recommendation, and it is a ratification rather than a fork

**Take the seven as they stand**, and the argument is that they were derived from the corpus rather
than guessed at: four were designed, three were forced by 139 blocks of real prose, and the sweep
that forced them is the largest read of this roadmap anyone has done.

**What is genuinely open, and is smaller than the vocabulary:**

1. **The words themselves.** `Unclaimed.` is the one worth arguing about. It is a noun-ish adjective
   where the others are verbs or nouns, and `Open.`, `Orphaned.` and `Unowned.` were all available.
   `Unclaimed.` was chosen because §90 already makes a draft pull request *the claim* on a milestone,
   so "unclaimed" names the absence of a thing this project already has a word for.
2. **Whether `Unclaimed.` should age.** Nothing distinguishes an entry written today from one that
   has sat a year, and nothing escalates either. That is deliberate for now (a backlog, not a queue),
   and it is the first thing that will look wrong in six months.

## What is blocked until this is answered

**Nothing.** The gate is live, all 139 blocks are swept, and the words work. A rename is a mechanical
edit across the sections plus one regex in `script/roadmap`, which is why this is `PROPOSED` rather
than held: the *move fast on what can be undone* tenet puts a vocabulary on the expensive side, but
this one is early enough that nobody outside the tree has learned it yet.

**The cost of saying nothing** is that the words harden by use. Every block written from here is
written against them, and the reversibility above decays with each one.

## The sweep's five proposed milestones, which are a separate ask

Listed in notes/follow-on-work.md with the bar used to pick them out of thirty-six: a claim this
project makes rests on it, or a record in the tree is now known to be wrong. They want numbers minted
at merge, not a decision here.
