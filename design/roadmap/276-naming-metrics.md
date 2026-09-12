# 276. The dashboard counts milestones and decisions by status, and names not at all

**Status: NOT-STARTED.** Minted 2026-09-11 by calef, from asking whether the metrics dashboard
carried naming data and finding it does not. *(Number provisional until the merge queue lands it;
milestone 275 is in flight ahead of it.)*

**Gate: NONE.** Not a design fork. `script/names` already computes the counts and `script/metrics`
already has the machinery to read a historical tree in bulk.

## The gap, stated plainly

`notes/project-metrics/weekly.csv` carries **34 columns**. Milestones get seven, broken out by their
full status vocabulary (`built`, `partial`, `in_progress`, `removed`, `not_started`, `optional`,
`recorded`). Decisions get six the same way (`decided`, `amended`, `superseded`, `proposed`,
`unrecorded`). **Names get none**, despite having an exactly parallel four-word vocabulary that
`script/names` already computes and reports: `ratified`, `recorded`, `provisional`, `unrecorded`.

Today that reads **101 unratified of 205**: 0 unrecorded, 37 recorded (a ruling only), 64
provisional. Nothing shows whether those numbers are climbing or falling week over week, which is
the one thing the other status columns exist to make visible.

## What to build

1. **Columns mirroring the existing pattern**: `names_ratified`, `names_recorded`,
   `names_provisional`, `names_unrecorded`, `names_total`. Follow `milestones()` and `decisions()` in
   `script/metrics` rather than inventing a third shape.
2. **A chart, if it earns one.** The existing page renders series that move. If this one turns out to
   be a near-flat line plus a slowly-growing provisional count, a table row may serve better than a
   plot; that is the builder's call from the data.
3. **The caveat on the page, not just in this block.** See below. It is the part most likely to be
   dropped as "obvious" and it is the part that keeps the number honest.

## Two things that make this harder than the existing columns

**The data is spread across every named thing, not in one index file.** `milestones()` and
`decisions()` each parse a single README. Naming provenance lives *at the name* (milestone 115's
whole design: the crate header that would carry a refusal is what a proposer actually reads), so the
computation has to read every program, crate, package manifest and `script/` entry point in that
week's tree. `script/metrics` already streams a whole tree's blobs in one `git cat-file --batch`, so
the machinery exists; the cost is the enumeration, not the reading.

**The early weeks will read as zero, and that is an artifact rather than a fact.** The series starts
**2026W29 (2026-07-15)**. Provenance blocks did not exist until **milestone 115, 2026-08-04
(2026W32)**. So three weeks of this series would show no ratified names, not because nothing was
ratified but because the convention that records ratification had not been invented. `script/metrics`'
own header already warns that *"every row is a RESTATEMENT, not a report... regenerated from git
history by applying TODAY's definitions to old commits"*; this series is the sharpest instance of that
hazard in the file, and the page should say so where those weeks are rendered rather than leave a
reader to infer a trend from a convention's birth.

## BUGS

- **A rising provisional count is not necessarily debt, and a dashboard makes it look like one.**
  `AGENTS.md` is explicit that `script/names --unratified` is *"a worklist rather than a wall,
  precisely so that an unratified name never blocks anyone's build."* A provisional name that works
  costs nothing while it sits. The page must say what a rising line does and does not mean, or this
  column manufactures pressure the project deliberately decided not to apply.
- **It measures the record, not the names.** A name can be ratified and bad, or provisional and
  perfect. This counts signatures.
- **`script/metrics`' own name is provisional** (its header, line 45: *"calef has not ratified it"*),
  so this milestone adds a naming metric computed by a tool that would appear in its own unratified
  column.

## Follow-on

- **None.** Pending this milestone's own build.
