# 238. Two scheduled checks have never once succeeded, and one of them is a fatal risk's refresh

**Status: BUILT 2026-09-03.** Minted the same day by calef, from milestone 232's (audit every check
against two questions: does anything run it, and does it block) findings, on the argument that
fixing the mechanism beats rewording the risk it holds up. *(Number provisional until the merge
queue lands it.)*

It was minted with no gate and needed none: both symptoms were already recorded and both repairs
live in this tree.

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

## What was found and built, 2026-09-03

**The shard-4 question had a boring answer and an interesting corollary.** `cargo mutants --shard
k/n` requires `k < n`, so `--shard 4/4` was rejected as an invalid argument in zero seconds on every
run: not a memory kill, not an eviction, an off-by-one in the matrix. The corollary is the part
worth keeping: shards are zero-indexed, so **`--shard 0/4` was never run**, and with the default
`slice` sharding that quarter was an alphabetical block (`dtb`, `calendar`, `compositor`, `cred`,
`elf`, `capability`). `script/mutation` had reported the failure correctly and loudly the whole
time, to a scheduled job nothing reads.

**The other deaths are runner eviction, and that is measured rather than assumed.** A resource
sampler was added to the job, because a `shutdown signal` kills the runner agent and every
`if: always()` step with it, so no autopsy can run afterwards. Shard 0 died 57 seconds after a
sample showing **14.7 GB of memory available of 16 and 107 GB of disk**; shard 2 at 12.3 GB, shard 1
at 10.7 GB. Across five runs, 13 of 20 shard jobs went this way, from 3 to 49 minutes in, while one
shard completed 2,462 mutants in 17m34s on the same machine class the same morning. So `-j 2` is not
the constraint in CI and lowering it would buy nothing. Nothing in this repository can stop the
eviction; the workflow now shards **eight ways with `--sharding round-robin`**, which makes each job
a smaller bet and, more usefully, makes a lost shard cost an eighth of every crate instead of all of
a few. **Nothing is measured less**: the same 9,857 mutants are attempted.

**A number came back, and it is a partial one that points down.** The one shard that finished killed
**74.4%** of its viable mutants against the whole-corpus baseline of 92.4% from 2026-08-03. Those
are not comparable and notes/mutation-testing.md says so at length: an alphabetical slice is not a
random sample, and thirteen of its twenty-one crates did not exist at baseline. The crates that did
are roughly stable. The drop is in crates that arrived with no host tests, and one is stark:
**`system_initializer` is 0 caught, 191 missed.**

**The Miri fix was not one flag.** `-Zmiri-env-forward=CARGO_MANIFEST_DIR` works and lands on a
second wall, `read_dir` against Miri's isolation; behind that sat five `board_console` tests doing
host I/O, invisible until the first two were cleared because `cargo miri test` stops at the first
failure. Clearing all of it needs `-Zmiri-disable-isolation` for the whole workspace run, and the
corpus test costs 0.74 seconds natively against **more than 12 minutes under Miri, unfinished when
killed**. Both crates have no dependencies and no `unsafe`, so there is nothing there for Miri to
judge. The tests gate themselves out under `cfg(miri)`, which is the convention `gpt`, `glob`,
`calendar`, `cred` and `ntp_proto` already use, with the reasoning at each test.

**And a fourth defect, found only because the third was fixed.** The `report against the baseline`
job failed even when a shard survived: `download-artifact` gives each artifact its own subdirectory
when it fetches several and unpacks a *lone* artifact flat, so `shards/mutants-shard-*` matched
nothing exactly in the case the job's `if: always()` exists to serve. It cost the one real report
this milestone's first run produced.

**Something to notice next time: `script/cadence-check`.** It reports any scheduled workflow whose
last successful scheduled run is more than 15 days old, and a workflow too young to have fired is
excused by its own creation date. It is **deliberately not a scheduled workflow**, which is the
whole design and the reason milestone 232 declined to build it: a cron that watches crons dies the
way its subjects die. Delivery is `scripts/trunk-health.sh`, which already runs under `launchd` and
could not have caught this on its own, structurally: its trunk check filters runs to `main`'s
current tip, and a weekly run matches that tip only until the next merge. It is not in `script/lint`
or `script/gates`, for `script/audits`' recorded reason.

## BUGS, after the work

- **Fatal risk 3 was not re-read, deliberately.** That is calef's, and the block above says so. What
  changed is that the clause making a stale number acceptable, "the weekly workflow already
  publishes the report", is now true rather than false. The number it will publish is not yet a
  whole-corpus number, and 74.4% on one alphabetical quarter is not a verdict.
- **Eviction is diagnosed, not solved.** Eight round-robin shards shorten the exposure per job and
  make a partial report representative; they do not stop GitHub reclaiming a runner. If that proves
  insufficient the next option is a self-hosted runner, which is a machine decision rather than a
  workflow edit.
- **The cadence check inherits patagonia's gap.** `scripts/trunk-health.sh` runs under `launchd` on
  one Mac, so a machine asleep is a watcher not watching. That cost is already recorded and accepted
  in AGENTS.md; this does not change it.
- **`audit cadence` is still red and this does not touch it.** It is red because two audits are
  genuinely due, which is the signal working. `script/cadence-check` will keep naming it until
  somebody runs them, and there is no way to tell a correct red from a broken one from outside.
- **Three quarters of the mutant corpus is still unmeasured since 2026-08-03.** A full refresh needs
  a run where enough shards survive, and no such run has happened yet.
