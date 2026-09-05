# 259. Sweep `notes/` for claims that stopped being true, because the gate cannot see them

**Status: NOT-STARTED.** Minted 2026-09-05 by calef, as the other half of
`design/roadmap/proposals/a-note-that-cites-a-milestone-that-moved.md`, which catches the notes that
cite a milestone and says plainly that the notes which cite nothing are the worse half.
*(Number provisional until the merge queue lands it.)*

**Gate: NONE.** Reading the tree needs nobody's permission and no hardware.

## Why a sweep rather than a check

**A page that says "there is no networking" cites nothing, because there is nothing to cite.** That
is the shape of the worst instance found so far, and it is inherent: a negative claim has no anchor,
so no citation gate can hang off it. Detecting negative claims in prose is the `git grep -w TODO`
experiment again, which AGENTS.md prices at an **82% false-positive rate**.

**But a signal useless as a gate is fine as a worklist**, and that is the whole reason this is a
milestone and not a lint. 82% false positives fails rung two and is perfectly serviceable for
ranking 175 files a person is going to read. The heuristic the check must refuse is the heuristic
this sweep should use.

**The precedent is milestone 247** (follow-on work named by a finished milestone goes nowhere),
which swept 139 finished blocks for buried work and produced the `## Follow-on` convention. This is
that exercise aimed at `notes/`.

## What went wrong, so the sweep knows what it is looking for

`notes/why-not-general-purpose.md`, rewritten 2026-09-05, is the worked example and every property
of it is worth carrying:

- **Five of six gap rows were false.** It claimed no networking, no writable filesystem, no display
  and single-core, and that *"you cannot drop in existing software; every program is hand-written
  against our ABI"*. Milestone 121 ran unmodified `ripgrep` with zero patches on 2026-08-31.
- **It cited none of the milestones that falsified it.** Milestone 7 and §4, §5, §10, §11; not 30,
  33, 57 or 121. So the citation check would not have caught it.
- **It was built on the retired framing.** It grounded itself by quoting the project's goal as
  *"understanding how operating systems work, not running applications"*, which AGENTS.md names
  specifically and says to treat as stale wherever found.
- **It asked to be updated and was not.** Its closing line said to add to it *"when a subsystem that
  would move the needle (a writable FS, a POSIX shim, a net stack) actually gets built."* Two of the
  three were built.

## The distinction that scopes this, and it is not age

**Measured 2026-09-05: 175 notes, 62,993 lines. Median 12 days since last touch, 44 untouched for
21 days or more, 26 for 30 or more.** An age-ranked sweep would start with `notes/mmu.md`,
`registers.md`, `uart.md`, `page-tables.md` and `higher-half.md`, all 54 to 55 days old.

**Every one of those is fine, and reading them first would waste the sweep.** They explain how an
MMU works, what a system register is, how a page table is walked. That machinery does not change
when a milestone lands.

**The split is what a note makes claims about, not when it was written:**

- **Concept notes** explain hardware, an algorithm, or a specification. They go stale only when the
  world does, which is rarely and visibly.
- **State notes** describe what nife currently is, has, lacks, or measures. **They go stale by
  construction**, because the system changes daily, and they are where every instance found so far
  has lived.

A note can be both, which is the interesting case: a concept note with a paragraph saying "we do not
do this yet" carries a state claim inside an otherwise durable page, and that sentence is exactly
what nobody rereads.

## How to run it

**A classification pass first, then deep reads.** Read every note's title and opening paragraphs,
which is cheap over 175 files, and sort them into concept, state, and both. Then read the state ones
against the tree. Do not read 63,000 lines in order.

**Rank the deep reads by how load-bearing the claims are**, not by age. A note a newcomer meets
early, or one that tells a reader what the system cannot do, outranks a note about a subsystem's
internals, because AGENTS.md's third principle is that a stranger must reach a correct mental model
without asking anyone.

**Check claims against the tree, not against memory.** Every stale claim found so far was found by
somebody looking at the code or the roadmap, and this session put three wrong citations into one
decision file by trusting recall.

**Fix in place, and keep what was wrong visible where it is instructive.** The
`why-not-general-purpose.md` rewrite kept the old claims as a "Then" column, because how fast they
went stale is itself worth seeing. That is a judgment per note rather than a rule.

## The proof that this milestone worked

**A written count**: notes read, notes found stale, claims corrected, and the ones deliberately left
alone with the reason. A sweep that reports only what it fixed hides its own coverage, which is the
number a future reader needs to know whether to run it again.

**And at least one recurring shape named.** 247's value was not the 139 blocks it read, it was the
`## Follow-on` convention that came out of them. If this sweep finds that stale claims cluster in one
kind of note or one kind of sentence, that finding is worth more than the corrections.

## BUGS

- **A sweep is a snapshot and this one will go stale too**, on a tree where 68 milestone rows flipped
  to `BUILT` in fourteen days. The proposal it comes from is the recurring half; this is the
  one-time half, and neither substitutes for the other.
- **`notes/` is not the whole surface.** Milestone 66's stale gap table and milestone 99's compressor
  claim are both in `design/roadmap/`, and 99's is still there. This milestone is scoped to `notes/`
  because that is where the newcomer-facing prose lives, and the roadmap has the same disease.
- **"Load-bearing" is a judgment and this block does not define it.** That is deliberate, since a
  definition tight enough to check would be tight enough to be wrong, but it means two lanes would
  rank differently.
- **The 82% figure is borrowed.** It was measured for `git grep -w TODO` against a different
  question, and nobody has measured the false-positive rate of a negative-claim grep over `notes/`.
