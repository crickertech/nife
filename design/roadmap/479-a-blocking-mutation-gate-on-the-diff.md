# 479. A blocking `--in-diff` mutation gate

**Status: REFUSED.** Refused by milestone 438 (design/roadmap/438-a-mutation-gate-on-the-diff.md),
and recorded there on 2026-09-19. Backfilled here on 2026-09-20 by
milestone 448 (design/roadmap/448-a-refusal-gets-a-number.md), which gave a refusal that names work a number, a
status and a condition that would change it. *(Number provisional until the merge queue lands it.)*

**The date is when the refusal was written down, not necessarily when it was made.** Most of this
tree's `- **Refused.**` bullets were written in one sweep on 2026-09-03, so the decision is usually
older than the bullet and its reasoning sits in the block's own prose above it.

## The refusal, in its own words

From '438. Would a diff-scoped mutation check have caught the 55, and what would it cost', under `##
Follow-on`:

> A blocking `--in-diff` gate, per this block's recommendation and the ruling that ends it at
> measurement 1. Nothing is switched on and no workflow, `script/ci-build` row or ruleset entry
> was touched.
>
> -- design/roadmap/438-a-mutation-gate-on-the-diff.md

## Why it is here rather than only there

Cost was not the objection: four seconds on a documentation change is cheaper than several checks
already required. The objection is evidence. The case the gate was built on is not a case it would
have caught, and the experiment that would settle it runs against pull requests this clone does not
contain. The block's own rule ends the work at measurement 1, and nothing was switched on: no
workflow, no `script/ci-build` row, no ruleset entry.

## Revisit

- **Condition.** A second measurement, on the experiment measurement 1 could not run. The block is
  explicit that the mechanism is not refuted and the story is, so what would change this is evidence
  about pull requests as they land rather than about the diff in hand.

## Index row

A gate built, measured and then deliberately not switched on, with the ruling recorded at the
measurement that ended it. The condition is a further measurement rather than a decision, and the
block already says which experiment it would be.
