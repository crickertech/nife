# 275. A gate that diffs `design/fatal-risks.md` against the roadmap it cites

**Status: NOT-STARTED.** Minted 2026-09-11 by calef, from a maintainer review of
`design/fatal-risks.md` against the tree the previous day, which found the file's own `BUGS`
warning ("nothing gates this file") firing four separate ways in one pass. *(Number provisional
until the merge queue lands it.)*

**Gate: NONE.** Not a design fork. `script/roadmap` and `script/decisions` already parse the
milestone and decision index into structured data; this reads that output and `design/fatal-risks.md`
and compares them.

## What the 2026-09-10 review found, because this milestone exists to stop finding it by hand

Four instances, one review, no gate involved in catching any of them:

- **Risk 2** cited milestone 191 as `RUN, 2026-08-30. AMBER`, and that was true. Milestone 191's own
  roadmap block still read `NOT-STARTED`, eleven days after the work merged (PR #589) — the
  status-in-two-places defect `§76`'s own sweep found before, recurring in the one place nothing
  compares the two records.
- **Risk 3** said re-running the mutation sweep was cheap because "the weekly workflow already
  publishes the report." The workflow had not succeeded once since 2026-08-03: a runaway mutant
  exhausts the CI runner's memory and kills the shard, inside the timeout meant to catch it. Already
  diagnosed in `mutation.yml`'s own header on 2026-09-03; `design/fatal-risks.md` was not updated for
  eight more days.
- **Risk 6** said "unmeasured, nothing timestamps the step" for hours after the instrument that
  measures it (`design/roadmap/proposals/time-the-hw-entropy-step.md`) merged and gave a real number.
- **Risk 9** cited milestone 164 as blocking (no `fs_server`) ten days after 164 was `BUILT`, and
  cited milestone 177's premise a day after that premise stopped holding (`DECISIONS §149`).

**The pattern across all four:** each is a fact `design/fatal-risks.md` states about a milestone or
decision it names, and every one of the four went stale the moment the thing it named changed status,
because nothing reads the two together. `script/decisions --check` and `script/roadmap --check` both
prove a citation *resolves*; neither asks whether what `fatal-risks.md` says about what it resolved to
is still true.

## What this needs

1. **Extract every milestone and decision citation in `design/fatal-risks.md`**, the same way
   `script/citations` already extracts them tree-wide, but scoped to this one file since its citations
   carry a stated status claim (`RUN`, `MEASURED`, `unmeasured`, `blocking`, and similar) that a bare
   `§N` cross-reference elsewhere in the tree does not.
2. **Compare the claimed status against the roadmap's or decisions' own recorded status.** The
   simplest version: a risk marked with a specific milestone's status word (e.g. "milestone 191...
   AMBER" implying 191 is done) should fail loudly if that milestone's own `Status:` line disagrees
   in a way that matters (`NOT-STARTED` when the risk entry assumes it ran, `SUPERSEDED` when the risk
   entry assumes it stands).
3. **This cannot be fully mechanical**, and the block should say so rather than overpromise: risk 9's
   two stale claims were about a *premise* (does 177 need the graphical stack) that changed for a
   reason no status field encodes. The gate's honest scope is catching the *first* kind of drift
   (a status word disagreeing with the roadmap), not the second (an argument's premise being
   overtaken). Naming the boundary now is cheaper than a gate that claims more than it checks.
4. **Where in `script/lint` or `script/gates` this belongs**, and whether it runs on every push or on
   a cadence: `design/fatal-risks.md` does not change often, so a cheap weekly check (the shape
   `script/cadence-check` already uses for scheduled workflows) may fit better than a per-push cost
   on every unrelated pull request.

## BUGS

- **Nothing here catches an entry that was never wrong syntactically but became wrong in substance**,
  the risk 9 shape above. That gap should stay named rather than quietly scoped away once the
  mechanical half is built.
- **A false negative is cheap here and a false positive is not.** `design/fatal-risks.md` is meant to
  be read and trusted; a gate that cries wolf on a status word that moved for a harmless reason
  (a milestone amended, not overturned) teaches the next person to ignore it, which is worse than the
  drift this milestone exists to catch.

## Follow-on

- **None.** Pending this milestone's own build.
