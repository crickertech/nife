# 238. Two scheduled checks have never once succeeded, and one of them is a fatal risk's refresh

**Status: NOT-STARTED.** Minted 2026-09-03 by calef, from milestone 232's (audit every check against
two questions: does anything run it, and does it block) findings, on the argument that fixing the
mechanism beats rewording the risk it holds up. *(Number provisional until the merge queue lands
it.)*

**Gate: NONE.** Both symptoms are recorded and both repairs are in this tree.

**In brief.** Milestone 232's audit found two scheduled workflows that have never produced a result,
and their consequences are not the same size.

## The mutation workflow, which is holding up a fatal risk

**Four scheduled runs, four failures, zero reports**: 2026-08-10, -17, -24 and -31, verified against
the API rather than inferred.

`design/fatal-risks.md` risk 3 (the tests do not test anything, and the quality is illusory) reads
**MEASURED, green**, on 92.4% of viable mutants killed, and closes:

> **The remaining experiment is cheap:** re-run it and compare against `.cargo/mutants-baseline.txt`.
> No new milestone; milestone 85 already owns it and the weekly workflow already publishes the
> report.

**That last clause is false and has been since the workflow was written.** It is also what makes a
stale number acceptable, because it says a refresh is arriving on its own. The 92.4% is from
2026-08-03 and 2,529 commits have landed since.

**The symptom is described and deliberately not diagnosed** by the lane that found it, because the
step logs have aged out of `gh`: on the most recent run all four shards died together about fourteen
minutes in with `The runner has received a shutdown signal`, which is the shape of a run being
reclaimed rather than a defect in a shard. On the three before it, **shard 4 died in about twenty
seconds every time** while its siblings ran for half an hour. Whether that is a memory kill, an
eviction, or something in `script/mutation --shard 4/4` is the first thing to find out, and the
workflow's own comment already says so.

## The Miri check, which has been red for three weeks on nothing

`crates/manual/tests/render.rs:279` reads `CARGO_MANIFEST_DIR` at run time and **Miri does not
forward the environment**, so the test fails there and passes under `cargo test`. **It is not
undefined behaviour and never was.** The fix is one flag. Milestone 232's lane did not apply it
because an audit repairs nothing, which was right.

## Why they belong together

Both are the same defect one level up from the code: **a check that runs on a schedule, fails, and
tells nobody.** Nothing in `script/gates` or CI turns red when a scheduled workflow has been failing
for a month, so the only thing standing between a dead cadence and a stale claim is somebody
happening to look. Milestone 232 was that somebody, once.

## What it needs

- **The mutation workflow producing a report**, and the shard-4 question answered rather than worked
  around. If the answer is that the runner cannot hold the job, say so and price the alternatives
  (more shards, a longer schedule, a smaller corpus) rather than quietly reducing what is measured.
- **The Miri run green**, or an honest statement of what it cannot cover.
- **Something that notices next time.** This block does not say what: a check that reads the API for
  scheduled-workflow health is one shape, and it is also the shape that goes stale itself. Whoever
  takes this should decide with the same suspicion milestone 232 applied to everything else.

## BUGS

- **A green mutation run is not the same as a good one.** The number it produces has to be compared
  against `.cargo/mutants-baseline.txt`, and a run that succeeds while the score falls is a different
  finding that this milestone does not cover.
- **Fixing the cadence does not re-read risk 3.** It removes the reason the verdict is currently
  unsupported; whether the verdict then holds is a question for whoever reads the new number, and it
  is calef's.
- **Neither repair says anything about the other seven scheduled workflows**, which milestone 232
  inventoried but did not run.
