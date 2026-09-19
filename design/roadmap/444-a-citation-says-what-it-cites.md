# 444. A citation says what it cites, so a number stops being the identity

**Status: NOT-STARTED.** Minted 2026-09-19 by the maintainer, on calef's ruling the same day:
*"add the gloss ratchet and launch a lane to address the 17% that fail today."* *(Number
provisional until the merge queue lands it.)*

**Gate: NONE.** The ruling is made; what is left is a check, a measurement and a backfill.

## What calef ruled, and one correction to the framing

**Ruled:** a `script/lint` ratchet may require a gloss on a `milestone N` or `§N` citation that
appears on a line a commit **adds**. Existing citations are not touched by the ratchet itself.

**The correction, because the number in the ask means something else.** `script/citations` resolves
560 **glossed** citations: 465 by the target's own title, 29 by quotation, 24 by path, 12 as
cross-references, 29 by date, 1 exempt. The 95 that do not resolve by title are not failures; a path
citation and a dated one are honest and checkable. **The real backlog is citations with no gloss at
all**, which the checker cannot verify in either direction, and the population is much larger than
560: `git grep` counts **3,697 `milestone N`** and **1,953 `§N`** occurrences in `*.rs` alone.
Measure that properly before editing anything.

## Why it matters, and it is the split rather than tidiness

DECISIONS §151 rules that the repository becomes several. A number is one global namespace and stops
meaning anything outside its repository; a glossed citation resolves by searching for the title,
wherever the block ends up. The roadmap-after-the-split proposal measured that the tree is already
**83% split-proof by habit** (465 of 560) and that nobody decided to make it so. A gloss is
therefore the cheap half of an expensive decision that is still open: what a citation's identity
becomes after the split.

## What to build

1. **The ratchet**, in `script/lint`: a `milestone N` or `§N` on a line the commit adds carries a
   gloss, and the gloss is grounded the way `script/citations` already grounds one. It reads the
   diff, not the tree, so it never fires on code a lane did not write. Say what it cannot catch
   (a citation added by a merge commit, a gloss that is grounded but wrong in context).
2. **The measurement**, before any backfill: how many citations carry no gloss, grouped by where
   they live (code comments, notes, roadmap blocks, decisions) and by whether the cited thing still
   exists. That table is the milestone's real deliverable, because it prices the backfill for the
   first time.
3. **A backfill where it pays**, chosen by that table rather than by counting. A code comment citing
   a milestone the reader must follow is worth glossing; a citation inside a dated account, a
   quotation, or a block's own history is not, and `design/naming.md`'s rule that a dated record
   keeps its words applies here too. **Do not rewrite 3,697 sites**; say what you did, what you left,
   and why, and leave the rest as a recorded, sized piece of work.

## BUGS

- **A grounded gloss can still be the wrong citation.** The check proves the words match the target,
  not that the target is the right one to cite; only a reader catches that.
- **The ratchet bites the honest case too**: a lane adding a line to a dated account has to gloss a
  citation that the surrounding passage already explains. Whether that is an exemption or an
  accepted cost is the first thing to answer if it turns out to fire often.

## Index row

calef ruled on 2026-09-19 that a citation added by a commit must say what it cites, after the
roadmap-after-the-split proposal measured that 465 of 560 glossed citations already resolve by
title, and that 3,697 `milestone N` and 1,953 `§N` sites in Rust alone have never been counted for
whether they carry a gloss at all. A number stops being an identity when the repository becomes
several (§151); a gloss survives the move. The ratchet is the cheap half of that decision and does
not prejudge it.
