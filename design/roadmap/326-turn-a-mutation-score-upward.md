# 326. Nobody has been assigned to turn a mutation score upward

**Status: PARTIAL.** Minted 2026-09-19 by calef, from the gap his own fatal-risk-3 ruling named
the same day. *(Number provisional until the merge queue lands it.)* Parts 1 and 2 are in progress
on `milestone/326-mutation-survivor-triage`; parts 3 and 4 are untouched and deliberately so.

## Progress, 2026-09-19

Measured per crate with `script/mutation -p <crate>`, before and after, on the lane's own worktree.
Every kill was re-run under its mutation; every equivalence claim below is a mutant the second run
still reports. The accounting is in `notes/mutation-testing.md`'s `## 2026-09-19` section, which is
where the reasons live.

| crate | missed before | missed after | killed | equivalent | excluded |
|---|---|---|---|---|---|
| `capability` | 8 | 3 | 5 | 3 | 0 |
| `memory_regions` | 8 | 0 | 6 | 0 | 0 |
| `elf` | 6 | 0 | 6 | 0 | 0 |
| `timetable` | 48 | 0 | 12 | 0 | 0 |
| `dtb` | 14 | 1 | 13 | 1 | 0 |

**Gate: NONE.** Everything this needs exists: `script/mutation -p <crate>` runs one package,
`.cargo/mutants.toml` is where an exclusion goes with its reason, and milestone 85 already set the
rule a survivor is triaged under. No decision is owed and no hardware is involved.

**Why it exists, and the shape is worth naming.** Four milestones touch mutation testing and
**three of them are repairs to the instrument**: 85 built `script/mutation` and triaged the
baseline's 391 survivors, 238 repaired the workflow's shard indices after it had never once
succeeded, 277 bounded the runaway mutant that took seven of eight shards every week, and 280
explained the two crates whose broken suites made a fall look real. Only 85 ever turned a score, once,
in August. So the tree has an excellent instrument, a working cadence, and nobody pointed at the
number it produces.

That is what holds `design/fatal-risks.md`'s risk 3 at **AMBER** rather than green, on calef's
reading of 2026-09-19. Milestone 85's rule is that **every survivor becomes a test, an exclusion with
a reason, or a recorded gap**. It held for the baseline's 391. The census of 2026-09-14 produced
**771**, and none of them has been looked at.

## What the work is, in priority order, and the order is the argument

1. **The seven regressions, because each is a property that used to hold.** This is the only part
   that is not ordinary backlog: a crate that never had coverage is a gap, while a crate that scored
   100% and now does not has lost something it had. A census is the first instrument in this project
   able to tell those apart, because a sample cannot distinguish an absent mutant from a killed one.

   | crate | baseline | 2026-09-14 | missed |
   |---|---|---|---|
   | `memory_regions` | 100.0% | 88.9% | 0 to 8 |
   | `elf` | 100.0% | 94.2% | 0 to 6 |
   | `capability` | 97.4% | 88.2% | 1 to 8 |
   | `dtb` | 99.7% | 96.6% | 1 to 14 |
   | `clock_protocol` | 96.8% | 91.0% | 2 to 6 |
   | `swish` | 94.6% | 89.4% | 3 to 20 |
   | `filesystem_protocol` | 93.1% | 90.1% | 37 to 59 |

   **`capability` is first inside this list**, because it is the crate the capability core's proofs
   are about, and a survivor there is a place the confinement argument could be wrong with every
   test green. `memory_regions` and `elf` are next, as the two that fell from a perfect score.

2. **`timetable`'s 48 survivors, and not because of its rate.** At 73.6% it is not the tree's worst
   crate, but it holds `next_after`, the property `design/fatal-risks.md`'s risk 2 names as its
   strongest counterfactual: the milestone 6 timer drift, proved in this tree over code the timer
   does not call. **48 survivors in a crate carrying a proof is the exact shape risk 2 is about**, and
   it is the one place on this list where a survivor might be evidence for a fatal risk rather than
   a missing test.

3. **The rest of the new-crate backlog, as backlog.** The 1.9-point gap between the like-for-like
   93.6% and the corpus 91.7% is the 26 crates that did not exist at baseline, and the eight worst
   crates in the tree are all of them new (`work_steal_slot` 54.2%, `memory_corruption_canary_gate`
   66.7%, `soak_page` 68.2%, `jh7110_entropy` 76.8%, `mdns_proto` 77.3% with 82 survivors, `job_mix`
   77.8%, `schedule_store` 78.4%). New code arriving less tested than old code is what a month of
   lanes should be expected to produce. This part is a worklist and should be taken as one; a
   milestone that tried to close all of it would be a milestone that never finishes.

4. **Rewrite the baseline once the triage lands.** `.cargo/mutants-baseline.txt` is still the
   2026-08-03 run, which is what every weekly report diffs against, so the tree's own comparison
   point predates the census by six weeks. `script/mutation --save-baseline` is the mechanism.
   **Do this last**, because a baseline written before the survivors are triaged would enshrine the
   regressions as the new normal, which is the one way this milestone could make things worse.

## What "done" means, and it is deliberately not a number

**Not a target score.** A percentage target is the thing milestone 85's rule exists instead of: it
can be met by excluding awkward crates, and the exclusion is invisible in the rate. Done here is
that **parts 1 and 2 have no untriaged survivor left**, each one closed as a test, an exclusion in
`.cargo/mutants.toml` carrying its reason, or a recorded gap in `notes/mutation-testing.md`'s ledger.
Whatever the score does as a result is the outcome, not the goal.

**A survivor proved equivalent is a success, not a dodge**, and milestone 85's ledger already has
that column. What is refused is an exclusion whose reason is that the test would be tedious.

## BUGS

- **A triaged crate can regress the same week.** Nothing in this milestone stops the next lane
  adding an untested surface, and the weekly report is a report rather than a gate (roadmap 85,
  deliberately). This is a sweep, and sweeps need repeating; whether that should become a gate is a
  question this block does not answer and should not.
- **The census is one data point.** The weekly workflow succeeded for the first time ever on
  2026-09-14, so "the cadence is alive" rests on a single success. If the second run disagrees with
  the first, part 1's regression table is the thing to re-derive before acting on it, and
  `script/mutation -p <crate>` can settle any single row without waiting for a sweep.
- **The survivor identities are not in the tree.** `.cargo/mutants-baseline.txt` carries per-crate
  counts and not the mutants themselves, so a lane re-derives a crate's survivors with
  `script/mutation -p <crate>` rather than reading them from the repository. That is cheap per crate
  and is why this block is scoped per crate rather than per survivor.
- **This measures the test suite, not the code**, which is the standing caveat on every number here,
  and it covers **host** crates only. The kernel and the arch trees, where risks 5 and 9 live, are
  outside it entirely.
- **Memory, not time, is what kills this job.** Milestone 277 bounded one mutant's allocation for
  exactly this reason, and AGENTS.md's rule is that a mutation sweep never runs beside lanes. A
  per-crate run is small enough not to be a sweep, and a lane taking part 3 in bulk should read that
  rule first.

## Index row

`design/fatal-risks.md`'s risk 3 is AMBER rather than green because 771 survivors from the
2026-09-14 mutation census have never been looked at, and milestone 85's rule is that every one
becomes a test, an exclusion with a reason, or a recorded gap. Four milestones touch mutation
testing and three are repairs to the instrument; only 85 ever turned a score, in August. The order
here is the argument: seven crates that regressed from a score they used to hold, `capability`
first; then `timetable`'s 48 survivors, because it carries the proof risk 2 calls its strongest
counterfactual; then the new-crate backlog as backlog; then the six-week-old baseline every weekly
report still diffs against.
