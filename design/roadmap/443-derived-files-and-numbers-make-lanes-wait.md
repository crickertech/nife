# 443. Lanes wait on each other for three reasons, and none of them is the work

**Status: NOT-STARTED.** Minted 2026-09-19 by the maintainer, on calef's question: *"Is there any
way to decouple all of this so that we don't have to sequence?"* *(Number provisional until the
merge queue lands it.)*

**Gate: NONE.** Every item is mechanical and inside this tree.

## What happened on 2026-09-19, which is the evidence

Eleven pull requests were in flight across two sessions. Four of them (#988, #993, #995, #997) were
blocked at once, and **none of the blocks was about the code in them**:

1. `design/roadmap/README.md` is a **generated index of 441 rows**, committed. Every pull request
   that mints or finishes a milestone rewrites rows in it, so unrelated lanes conflict in one file.
   The stick-program lane and the layout-control lane each reported, independently and without
   being asked, that they deliberately did not regenerate it.
2. **`script/fatal-risks` reads that index** (`script/fatal-risks:409`) rather than the milestone
   blocks. So a stale derived file made a truthful record look wrong: risk 9's citation of milestone
   177 "looked current only while 177's row was not BUILT", which is how a lane put it. This is the
   coupling that actually jammed the four.
3. **Numbers are minted by hand and must be contiguous.** Two sessions collided on `§156`, `§158`
   and milestone 439 in one day, and `script/lint` fails on a gap, so whoever lands second
   renumbers. The tree's own rule (the integrator mints) assumes one integrator, and there were two.

**A fourth cause was the maintainer's, not the tree's**, and is recorded here so the fix is not
mistaken for a whole answer: lanes were cut from other lanes' branches (#987 on #985, #993 on #990,
#997 on #996), which turns one delay into four. The merge queue already lands groups of five, so
lanes cut from `main` would have been independent. That is a rule for whoever briefs, not code.

## What to build

1. **`script/fatal-risks` reads the blocks, not the index.** One file, and it removes the coupling
   that caused the jam. The blocks are per-milestone files, so two lanes touching different
   milestones cannot conflict.
2. **Stop committing the generated index, or regenerate it after merge.** Two shapes, and the
   milestone should choose with reasons: drop `design/roadmap/README.md` from the tree and have
   `script/roadmap` print it on demand (a reader loses a browsable file on GitHub, which is what it
   is for), or keep it and have CI regenerate it on `main` after each merge, so no lane ever edits
   it. Check every consumer first: `script/lint`, `script/roadmap --check`, `script/citations` and
   anything else that reads it, and make each read the blocks instead.
3. **Let numbering have gaps, and assign numbers late.** `script/lint`'s contiguity check is what
   forces a renumber; a gap is not a defect, a duplicate is. Decide whether numbers are assigned at
   merge (a slug-first filename, numbered when it lands) or simply allowed to be sparse, and say why
   in the block. `design/naming.md` and AGENTS.md's "anything global to the tree is assigned by the
   integrator" both describe the current rule and should be corrected together if it changes.

## What this does not do

- It does not make two lanes safe in the same source file. That is milestone 365
  (`xtask/src/main.rs` is 10,700 lines) and the hotspot rule in AGENTS.md.
- It does not remove the merge queue's serialisation, which is not the problem: the queue lands
  groups of five, and today's jam was conflicts and false gate failures, not throughput.

## BUGS

- **The index is also how a reader browses the roadmap on GitHub.** Any fix that deletes it trades a
  contributor-facing artifact for lane independence, and that trade should be made on purpose.
- **A post-merge regeneration commit is a bot writing to `main`**, which this tree has never done.
  It needs an identity (milestone 128) and a rule about what else such a commit may touch.

## Index row

Eleven pull requests in flight, four blocked at once, and none of the blocks about the code in them:
a committed 441-row generated index that every milestone rewrites, a gate reading that derived file
rather than the blocks it is derived from, and hand-minted contiguous numbers that two sessions
collide on. Minted from calef's question of 2026-09-19, after a day in which lanes waited on each
other for reasons that were all bookkeeping.
