# The first weekly runs: the 2026-09-03 sample and the 2026-09-14 census

An appendix of [notes/mutation-testing.md](../mutation-testing.md), moved there verbatim on 2026-09-24.

## 2026-09-03: the weekly report had never run, and the first quarter to finish reads 74.4%

The baseline above is from 2026-08-03. **No weekly run has ever refreshed it**, and until milestone
238 nobody knew that: the `mutation testing` workflow failed all four of its scheduled runs and a
scheduled workflow's red is an entry in the Actions tab with no badge. Milestone 232's audit found
it; the diagnosis and repairs are milestone 238's, and the workflow file carries them in full.

**Two of the causes change what this note claims.**

`--shard k/n` is zero-indexed, and the matrix ran `[1, 2, 3, 4]`. So `--shard 4/4` was an argument
error every week, and **`--shard 0/4` never ran at all**. With the old `slice` sharding that quarter
was an alphabetical block: `dtb`, `calendar`, `compositor`, `cred`, `elf`, `capability` and their
neighbours. Nothing was lost from the baseline, which was a full local run, but four weeks of
weekly reports would have been silently missing a nameable quarter had any of them succeeded.

The other cause is **a single mutant allocating without bound**, and the diagnosis took one wrong
turn worth recording. A resource sampler at 60 seconds showed 10 to 15 GB of memory free shortly
before each kill, which reads as runner eviction and was written up as such. At 10 seconds the same
failure is 1.4 GB to 15.8 GB in twenty seconds, then the shutdown signal. **The per-mutant timeout,
auto-derived at 28 to 51 seconds here, cannot catch an allocation that finishes the machine in
twenty.** A sampler whose interval is longer than the event reports innocence.

So this is not the `-j 2` bound below, and lowering it would not help: one runaway takes 14 GB by
itself. The workflow now shards eight ways with `--sharding round-robin`, which is damage control
(a lost shard costs an eighth of every crate rather than all of a few) and not a repair. The repair
is a bound on what one mutant may allocate, and it is scoped as its own work in milestone 238's
block.

### The numbers, and what each one is

Two runs on 2026-09-03, and the second is the one to read.

| | caught | missed | timeout | unviable | viable | killed |
|---|---|---|---|---|---|---|
| whole corpus, 2026-08-03 (baseline) | 4,654 | 391 | 96 | 410 | 5,141 | **92.4%** |
| shard 3 of 4, `slice`, 2026-09-03 | 1,710 | 598 | 27 | 127 | 2,335 | 74.4% |
| **shard 0 of 8, `round-robin`, 2026-09-03** | **910** | **187** | **32** | **97** | **1,129** | **83.4%** |

**The round-robin row is the comparable one, and that is the whole reason the sharding changed.**
It is a uniform one-eighth sample of every mutant in the tree, so it covers **all 60 crates** and
its rate estimates the corpus rate. The `slice` row is an alphabetical block, `nvme` through
`work_steal_slot` (the crate is `non_volatile_memory_express` since 2026-09-18, and the old
spelling is kept here because it is what the shard boundary was computed from; the block would
start elsewhere in the alphabet today), thirteen of whose twenty-one crates did not exist at
baseline; it is a rate for
those crates and not for the tree, and it is kept here only because it was the first shard this
workflow ever completed.

So the honest reading: **the tree's mutation score has fallen from 92.4% to roughly 83.4%** in a
month. It is a sample, not a census, and a second shard would move it; it is not a five-point
question of sampling noise either.

**Corrected 2026-09-03 (milestone 244), and the direction is up.** Removing `system_initializer`,
which no host test could ever have reached, takes 25 uncatchable missed mutants out of the
round-robin sample and 191 out of the `slice` one. The rates are the same runs over a denominator
that no longer counts mutants nothing could kill:

| | caught | missed | timeout | unviable | viable | killed |
|---|---|---|---|---|---|---|
| shard 3 of 4, `slice`, corrected | 1,710 | 407 | 27 | 127 | 2,144 | 81.0% |
| **shard 0 of 8, `round-robin`, corrected** | **910** | **162** | **32** | **97** | **1,104** | **85.3%** |

So the drop is from 92.4% to roughly **85.3%**, not 83.4%. Still a real fall, still a sample, and
still not sampling noise. What changed is that one of the three crates blamed for it was not a crate
with an untested surface; it was a bookkeeping gap, and the two that remain (`uefi_loader` at 15%,
`manual` at 52%) are the real ones.

**Where the drop is.** The crates that existed at baseline are broadly stable or better: `gpt` 55/1,
`elf` 12/0, `calendar` 46/0, `glob` 14/0, `cred` 14/0, `dtb` 43/3, `filesystem_protocol` 65/8,
`grant_plan` 67/2. Three crates carry nearly all of the loss, and all three are new since the
baseline:

- **`system_initializer`: 0 caught, 25 missed in the sample** (0 of 191 in the `slice` run, which
  saw all of them). Every mutant survives. Nothing in the host suite would notice any of its
  functions returning the wrong thing.

  **Retracted on 2026-09-03 by milestone 244, which is what this bullet asked for.** That crate was
  never in `.cargo/mutants.toml`, though the other three in its position are and
  that file's own head comment says its list mirrors `script/coverage`'s and asks the next person to
  keep the two in step. It reaches `user_mode_runtime`, so the host suite
  cannot compile a line of it, and both numbers above are a crate scored against a suite that could
  not have killed anything. It is excluded now, and `script/lint`'s bare-metal gate derives the four
  places that have to agree rather than asking anyone to keep them in step. See the corrected rates
  below.
- **`uefi_loader`: 3 caught, 17 missed.** 15%.
- **`manual`: 56 caught, 60 missed.** 52%, and the crate is the documentation renderer.

That is the finding, and this milestone deliberately does not act on it. `system_initializer` at
zero is a milestone of its own, not a line in a workflow repair.

**It became milestone 244 and the answer was not the expected one.** The lane measured the crate's
196 mutants by function before moving anything: 33 sit in pure logic and 157 in the syscall sequence,
about sixty lines of a 2,632-line file, so nothing was lifted and the block records why. The half of
it that did change the tree is the exclusion above and the gate behind it. See
`design/roadmap/244-the-largest-crate-in-the-tree-is-proved-by-nothing.md`, whose most reusable line
is the method rather than the verdict: **`cargo mutants --list -p <crate>` attributes every mutant to
its enclosing function, so "where are this crate's mutants" is one command, and it is worth asking
before any lane that proposes to restructure code for testability.**

**What this does not say.** It does not re-read `design/fatal-risks.md`'s third risk, which is
calef's. It is a sample rather than a census: seven of eight shards were killed by the memory
failure above, so 8,700 of the 9,857 mutants are still unrun since 2026-08-03. What it removes is
the reason the stale number was acceptable, which was the clause saying a refresh arrives on its
own. A refresh has now arrived, once, and it is lower.

## 2026-09-14: the first census since the baseline, and the fall was an artifact

**The workflow succeeded, all eight shards, for the first time since it was written.** Run
[34833498873](https://github.com/crickertech/nife/actions/runs/34833498873), scheduled, 54 minutes,
**10,012 mutants over 64 crates: 8,303 caught, 771 missed, 203 timed out, 735 unviable**, which is
**91.7% of viable mutants killed**. Milestone 277's memory bound is what made it finish; the runaway
that had taken seven of eight shards every week did not take one.

This is the re-run `design/fatal-risks.md`'s third risk has been waiting for since 2026-08-03, and
it is a census rather than a sample.

### The like-for-like number is up, not down

The section above read the tree's score as having fallen from 92.4% to roughly 85.3%. **On the
evidence of a full run that is wrong, and the correction is worth stating before the numbers**: the
38 crates that existed at baseline scored **93.6%**, against 92.4% for the same 38 crates a month
earlier.

| | crates | caught | missed | timeout | viable | killed |
|---|---|---|---|---|---|---|
| whole corpus, 2026-08-03 (baseline) | 38 | 4,654 | 391 | 96 | 5,141 | **92.4%** |
| the same 38 crates, 2026-09-14 | 38 | 6,008 | 417 | 127 | 6,552 | **93.6%** |
| whole corpus, 2026-09-14 | 64 | 8,303 | 771 | 203 | 9,277 | **91.7%** |

**So there was no fall.** The 85.3% reading was a one-eighth sample taken while two crates were
being scored against test suites that could not run: `uefi_loader`'s `[[bin]]` half, whose 154
mutants came back missed in "0s build + 0s test" because nothing rebuilt, and `documentation`
(then `manual`), measured with a third of its suite compiled away behind a default-off Cargo
feature. Milestone 280 fixed both, and both are now unrecognisable: **`uefi_loader` scores 100% and
`documentation` 95.4%**, the two crates that had been blamed for the drop.

**The gap between 93.6% and 91.7% is the 26 crates that did not exist at baseline**, and that is the
real finding rather than a disappointment. New crates arrive less well tested than old ones, which
is what a month of lanes should be expected to produce and is a worklist rather than a verdict.

### Two things a census shows that a sample cannot

**Three of the baseline's five perfect crates lost their perfect score**, which no sampled run would
have surfaced as a regression because a sample cannot distinguish an absent mutant from a killed
one. These are the survivors worth triaging first, because each is a property that used to hold:

| crate | baseline | 2026-09-14 | missed |
|---|---|---|---|
| `memory_regions` | 100.0% | 88.9% | 0 -> 8 |
| `capability` | 97.4% | 88.2% | 1 -> 8 |
| `clock_protocol` | 96.8% | 91.0% | 2 -> 6 |
| `elf` | 100.0% | 94.2% | 0 -> 6 |
| `swish` | 94.6% | 89.4% | 3 -> 20 |
| `dtb` | 99.7% | 96.6% | 1 -> 14 |
| `filesystem_protocol` | 93.1% | 90.1% | 37 -> 59 |

`nifefs`, `dma_validator` and `bitmap_font` held at 100%. Against those regressions, the largest
gains are real too and mostly where a triage pass was spent: `intrusive_fifo` 57.1% to 100%,
`line_editor` 80.1% to 98.5%, `pci` 77.3% to 90.7%, `ipc` 82.8% to 96.6%, `glob` 89.2% to 100%,
`user_mode_heap` 89.3% to 100%.

**And the eight worst crates in the tree are all new**, none of them in the baseline:

| crate | killed | missed of viable |
|---|---|---|
| `work_steal_slot` | 54.2% | 11 of 24 |
| `memory_corruption_canary_gate` | 66.7% | 6 of 18 |
| `soak_page` | 68.2% | 7 of 22 |
| `timetable` | 73.6% | 48 of 182 |
| `jh7110_entropy` | 76.8% | 19 of 82 |
| `mdns_proto` | 77.3% | 82 of 362 |
| `job_mix` | 77.8% | 6 of 27 |
| `schedule_store` | 78.4% | 8 of 37 |

`timetable` is the one to read first, and not because of its rate. It is the crate holding
`next_after`, the property `design/fatal-risks.md`'s risk 2 names as its strongest counterfactual
(the milestone 6 timer drift, proved in this tree over code the timer does not call). 48 survivors
in a crate carrying a proof is the shape that file's risk 2 is about.

### Two caveats on the comparison, neither of which moves the reading

**The run predates milestone 265's rename by seven hours** (the sweep started 10:30 UTC on
2026-09-14; #860 merged at 17:12), so its artifacts spell the protocol crates `*_proto` and the
baseline file spells them `*_protocol`. Every table above maps the nine affected names; nothing else
about them differs. The next run's artifacts will match the baseline's spelling without help.

**The baseline's `caught` was derived, and this run's is reported.** The head comment on
`.cargo/mutants-baseline.txt` records that the 2026-08-03 run was resumed twice, so its caught
column is total-minus-the-rest and three mutants could not be attributed at all. This run was a
single clean pass per shard. The direction of that difference favours the baseline being slightly
generous, which makes the +1.2 like-for-like gain a floor rather than a ceiling.

**The baseline file is deliberately not updated here.** Replacing it would destroy the comparison
the next run wants, and choosing when a new census becomes the baseline is not a records edit.

### What this does not say

It does not re-read `design/fatal-risks.md`'s third risk, which is calef's and which
`design/roadmap/proposals/fatal-risk-3-against-the-new-number.md` has been holding since
2026-09-03. What it does is give that proposal the number it was written to be read against, and
the number is not the one anyone expected.

It also says nothing about the kernel or the arch trees, which are excluded by construction and are
where risks 5 and 9 live. **Mutation testing measures the test suite, not the code**, and one green
census does not make a habit: this is the first of the weekly runs to finish, and the cadence is
worth a second data point before anyone quotes a trend.
