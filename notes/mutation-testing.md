# Mutation testing: the baseline and the triage rule

Milestone 85. `script/mutation` runs [cargo-mutants](https://mutants.rs/) over the host crates:
rewrite one function at a time (replace a return value, delete a match arm, flip an operator),
rerun the mutated package's tests, and record whether anything noticed. Coverage answers "did this
line run under a test"; a mutation run answers "would any test notice if this line were wrong",
which is the property a test suite exists for.

The tool is pinned in `.cargo-mutants-version` (the `.cargo-deny-version` discipline, and with an
extra tooth here: cargo-mutants changes which mutants it *generates* between versions, so an
unpinned tool moves the weekly numbers with nothing in the tree having changed). Exclusions live in
`.cargo/mutants.toml`, each with its reason; config, not a code dependency, per DECISIONS §46. The
weekly `mutation testing` workflow reruns the same command eight-way sharded (four until milestone
238, and round-robin rather than alphabetical since) and publishes the per-crate table against
`.cargo/mutants-baseline.txt`. A report, not a gate, until the weekly numbers prove stable enough
that a new survivor deserves to fail something.

**The current census is the run of 2026-09-14**, the first the workflow ever completed: 10,012
mutants over 64 crates, 91.7% of viable mutants killed, and 93.6% over the 38 crates the baseline
below covers. The baseline section is kept as the 2026-08-03 measurement it is, because it is what
the next run diffs against; read it as a fixed point rather than as the tree's score today.

## The triage rule

Every survivor becomes exactly one of three things, and nothing stays untriaged:

1. **A test worth writing.** The mutant found a property no test asserts; assert it. This is the
   product working as intended.
2. **A recorded exclusion.** The function cannot be meaningfully tested on the host, or the mutant
   is semantically equivalent to the original. It goes in `.cargo/mutants.toml` with the reason
   next to it; an exclusion without a reason is a hole, not a decision.
3. **An honest deferral, recorded here.** A real gap that a test could close but whose test is not
   worth its cost yet. Named in this note's table so the weekly report's number has a ledger behind
   it, and nothing is silently accepted.

## The baseline

The machine-readable copy (what the weekly report diffs against) is `.cargo/mutants-baseline.txt`,
written by `script/mutation --save-baseline`; the table below is the same numbers with the story
attached. A **timeout** here is almost always a detected hang, not an undetected bug: the classic
case is `+=` to `-=` on a walker's cursor, which loops forever and trips cargo-mutants'
auto-timeout. It is still listed per crate because a timeout that is *not* a hang would be
invisible inside a merged "caught" number.

The table is the run of 2026-08-03: **5,551 mutants over 38 host crates, 4,654 caught, 391
missed, 96 timed out, 410 unviable**, which is **92.4% of the viable mutants killed**. It cost about
five and a half hours at `-j 2` on an 8-core, 16 GiB machine.

Crate names below are the ones the run measured. Where one has since changed, the row keeps the
measured spelling so the number stays traceable to the command that produced it: `asid` is
`address_space_identifier` since 2026-09-18 (DECISIONS §154).

**Read the `missed` column as a worklist, not as a score, and read it as of that run.** Every one of
those 391 has since been triaged, and the commits on this branch close about two hundred of them,
so a rerun today would report a smaller number. The table is not restated here from a rerun because
the honest way to re-measure is the weekly workflow, on a machine that is not also running the
triage. What each survivor turned into is the section below, crate by crate; the counts there and
the counts here are measurements of two different moments, and both are labelled.

```
crate           caught  missed  timeout  unviable   total   killed%
abi                 27       1        0         1      29    96.4
asid                21       2        0         2      25    91.3
bitmap_font             11       0        0         2      13   100.0
block_roster        53       1        0         4      58    98.1
c_seam              21       1        0         3      25    95.5
calendar           369       7        3        16     395    98.2
capability          37       1        0        16      54    97.4
clock_protocol         54       2        7         3      66    96.8
compositor         209      14        0        10     233    93.7
coremark           106       2        1         0     109    98.2
cred               127       3        1        16     147    97.7
cred_proto          72       2        0        13      87    97.3
nifefs          107       0        0         8     115   100.0
dma_validator       79       0        0         6      85   100.0
dtb                294       1       20        10     325    99.7
elf                 98       0        0         4     102   100.0
entropy_protocol       21       2        0         0      23    91.3
frames              93       3        1        12     109    96.9
fs_proto           489      37        9        31     566    93.1
graphics_protocol          120       6        0         6     132    95.2
glob               110      14        6         5     135    89.2
gpt                423       5        1        36     465    98.8
grant_plan         367      26       20        76     489    93.7
intrusive            4       3        0         4      11    57.1
ipc                 24       5        0         4      33    82.8
isa                147      22        3        40     212    87.2
line_editor        183      46        2         6     237    80.1
measured_boot      118       9        4         6     137    93.1
network_time_protocol          156      16        0        14     186    90.7
paging             256      39        0        17     312    86.8
pci                 88      27        4         3     122    77.3
regions             11       0        0         1      12   100.0
sink_proto          43       2        0         2      47    95.6
slots               26       5        0        13      44    83.9
socket_protocol        15       2        0         0      17    88.2
swish               44       3        9         2      58    94.6
user_mode_heap           20       3        5         7      35    89.3
video_terminal     211      79        0        11     301    72.8
TOTAL             4654     391       96       410    5551    92.4
```

**`killed%` counts a timeout as a kill**, because every one of the 96 was checked by hand and every
one is a detected hang (below). It is over viable mutants, so `unviable` is excluded from the
denominator: an unviable mutant does not compile, which says nothing about the tests.

**Five crates scored 100%**: `nifefs`, `dma_validator`, `elf`, `memory_regions` and `bitmap_font`. The first
two are the trust-boundary parsers, and they got there the hard way, by having every one of their
24 first-pass survivors turn out to be a real gap that a test then closed. That is the number to
compare a crate against next week.

**Four numbers deserve their asterisk before anyone reads them as quality.** `video_terminal` at
72.8% and `pci` at 77.3% were simply untriaged when the run happened; both are now done and the
survivors were overwhelmingly real. `intrusive_fifo` at 57.1% is 4 caught out of 7 viable, which is a
crate small enough that one mutant moves the percentage nine points. And `line_editor` at 80.1% is
what a terminal looks like when its tests assert what the screen shows and never where the cursor
is.

**How the numbers were derived, because the method matters if you reproduce them.** The run was
resumed twice (once after an OOM at `-j 4`, once to finish the tail), so its `caught.txt` holds only
what the final pass caught and the earlier passes' kills are in `previously_caught.txt`, which also
carries their unviable mutants. Reading `caught` off that file would undercount. The table instead
takes the total per crate from `cargo mutants --list` on the merged tree, subtracts missed, timeout
and the union of every pass's unviable, and calls the rest caught. Three of 5,551 listed mutants
could not be matched to any pass's outcome (two in `glob`, one in `network_time_protocol`, both crates whose
line numbers moved when tests landed) and are counted as caught, which is the only place this table
guesses. `script/mutation --save-baseline` writes the machine-readable copy the weekly job diffs
against.

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

## Calibration: the exhaustive crates

The roadmap block predicted `network_time_protocol` and `gpt` would score near-perfectly as a check on the
tool. The honest verdict: **they scored near-perfectly exactly where their exhaustive method
reaches, and the tool's value was showing precisely where that is.**

- **network_time_protocol**: 155 of 186 mutants caught on first contact (83%, or 90% of the viable ones), and
  every conversion the sweeps quantify over was mutation-proof. The 17 survivors sat in what the
  sweeps never read: a range constant tested from only one side, accessors no test called, a
  fixture whose tiny root delay could not tell `/ 2` from `% 2`, and stratum 15, which no
  rejection test ever probed (only 16). Four tests closed them; the rest are equivalents, one of
  them proven equivalent *by the sweep itself* (the 1e9-value round-trip passes under the mutant,
  because the backward conversion's rounding absorbs the forward truncation).
- **gpt**: the same shape with a sharper lesson. The single-byte-corruption sweep proves
  *rejection*, not *rejection for the right reason*: every corrupt header dies at the CRC before
  the layout checks run, so every layout comparison could rot invisibly, and one of them (`>` at
  the backup-array boundary) was hiding a real wrong-accept bug the mutant fixed.

So the calibration check passed in the way that matters: the tool agreed with the exhaustive
suites about everything they actually assert, and disagreed only where they assert less than they
appear to. That second half is the finding.

## Survivors and where each one went

Grouped by crate; every survivor from the baseline run is accounted for in one of the three
buckets. "Killed by" names the test written for it.

### Patterns that recur, named once

- **Bit-constant survivors** (`1 << n` becoming `1 >> n`): every crate that defines a wire format
  as shifted constants had them, and the fix is one test pinning the exact values. `1 << 0`
  mutated to `1 >> 0` is the degenerate case: both are 1, so that one mutant is *equivalent* and
  is recorded as such wherever it appears (abi 238, c_seam 145, capability 63 had the killable
  siblings).
- **Boundary survivors** (`>` becoming `>=` at a length or range check): the tests exercised a
  short value and an over-long value but never the exact limit, so the limit itself was never
  proven legal. The fix is a test at the boundary, and in one case (gpt, below) the mutant was
  not a missing test but a missing `=`.
- **debug_assert shielding**: a belt-and-braces release-mode guard sitting *behind* a
  `debug_assert!` of the same condition is unobservable under `cargo test`, which runs with debug
  assertions on: every input that would reach the guard's differing behaviour panics first, in
  both the original and the mutant. Those mutants are recorded equivalent-under-harness, not
  excluded in config, so they stay visible if the assertions ever move.
- **Single-threaded blindness**: mutants in seqlock/atomic orderings (clock_protocol's `publish`)
  change nothing a single-threaded test can observe. Concurrency claims are argued in the code's
  comments and, where they are pure, proved; a unit test cannot carry them.

### gpt: one real bug, fourteen missing tests

The `>` to `>=` survivor at `Gpt::parse`'s backup boundary was a genuine wrong-accept:
`block_count - backup_reserved` is the backup array's first block (it is exactly what
`backup_entry_lba` computes), and equality put one usable block inside the backup entry array,
where a partition would overwrite it. **The mutant was the fix**; parse now refuses equality, and
the boundary has a test on both sides. No test could see the difference before because
`real_disks.rs` corrupts one byte at a time, so every corrupt header dies at the CRC before the
layout checks run, and every table `create` makes is tight on both boundaries. The other fourteen
gpt survivors were killed by forging CRC-valid headers with one layout lie each (`table.rs`).

This is also the honest asterisk on the exhaustive-suite calibration: an exhaustive sweep proves
*rejection*, not *rejection for the right reason*. A mutant that weakens check B survives any
input that also trips check A.

### The hand-triaged crates

- **abi** (5): rights bits and the fault slot, killed by `rights_are_distinct_single_bits` and
  `the_fault_slot_is_inside_the_capability_table`; `1 << 0` equivalent as above.
- **asid** (2): both in `free`'s release-mode range guard, equivalent-under-harness
  (debug_assert shielding, above).
- **block_roster** (3): the header-only page, killed by
  `a_header_only_page_is_an_empty_roster_not_a_short_one`; `capacity_of`'s `<` to `<=` is
  equivalent (`len == HEADER_BYTES` yields capacity 0 down both arms).
- **c_seam** (5): verdict bits, killed by `the_verdict_bits_are_distinct_single_bits`; the
  `1 << 0` sibling equivalent.
- **calendar** (18): two real parser edges (a fraction scan that could read one past the end;
  an offset-colon check whose index could rot to `i - 3`, which lands on the seconds colon in
  every well-formed input, so `+05300` parsed), killed by `parser_edges_the_mutation_run_found`;
  the absolute weekday, unix zero without `-0`, the three `Formatted` impls, and one message per
  refusal, each with its own test. The audit added two more. `from_hm`'s `hours < 0` is **not**
  masked by the `!= 0` clauses, only its `<=` siblings are: rotted to `hours == 0` it refuses
  `from_hm(-5, -30)`, which is how -05:30 is written, and accepts `from_hm(-5, 30)` as -07:30. Every
  test used positive hours, which is one row of the guard's truth table. And the six-byte guard
  ahead of the offset is a length check, not a trailing-bytes check: rotted to `<` it refuses the
  same inputs and renames one refusal, so `+05:60x` is now pinned as `BadOffset`. Equivalent:
  `from_hm`'s two `<` to `<=` mutants (masked by the `!= 0` clauses beside them), `Writer::byte`'s capacity guard (`FMT_CAP` is two bytes over the
  longest output, so the boundary is unreachable), `Writer::offset`'s sign at zero (both format
  paths branch to `Z`/`UTC` before a zero offset can reach it), and the redundant `+ with -` at
  the offset-length guard (the `number`/`expect` helpers bounds-check behind it, so every path
  still errors identically).
- **capability** (11): rights bits, `from_bits` masking (an OR there turns undefined bits into
  defined rights), idempotent union, and `insert_at` landing in the named slot; killed by
  `rights_bits_are_the_wire_format` and `insert_at_fills_exactly_the_named_slot`.
- **clock_protocol** (10): the request wire format and the sanity window's seconds-times-a-billion
  arithmetic, killed by `the_request_word_is_the_wire_format_it_claims` and
  `the_sanity_window_is_where_it_says`. Equivalent: the CAS's `s + 1` (single-threaded blindness,
  above; nothing observes the odd window, and the sequence still advances by two) and `decide`'s `>`
  at equal timestamps (a zero step is accepted down both branches). The other seven seqlock mutants
  are **not** equivalent and are not survivors: flipping either spin guard or the reread check makes
  a single-threaded `read` or `publish` spin forever, so all seven are timeouts.
- **cred** (21): the longest legal identity and secret were never exercised end to end, the
  memory ceiling could become either a divide or an add, and only the divide was caught, because
  the test that was supposed to hold it named the ceiling as `Cost::MAX_M_KIB` on both sides of its
  own assertion: `1024 / 1024` is 1, which falls below `MIN_M_COST` and fails, but `1024 + 1024` is
  2048, which is a legal cost, so every symbolic check passed while every real policy was refused.
  The ceiling is now pinned as `1_048_576`, hand-computed, which is the nifefs lesson applied to
  a constant instead of an image. The redacting `Debug` could also be
  replaced with one that prints nothing; killed by `the_longest_identity_and_secret_are_legal`,
  `the_cost_is_what_it_says_up_to_the_real_ceiling`, `a_store_is_empty_until_it_is_not`, and
  `debug_prints_the_redaction_and_nothing_secret`. Equivalent: `MAX_P_COST`'s exact value (any
  `p` large enough to notice it already fails the `m_kib < p * 8` check first).

- **cred_proto** (6 after the `proofs::` exclusion): the request word pinned as one exact number
  with opcode `SEAL` (every prior test used opcode 1, so an `op` returning the constant 1
  passed), the smallest page the layout fits accepted at both ends, and `wipe`'s bound asserted
  from both sides. Equivalent: the two `|` to `^` mutants in `req` (the three fields are masked
  into disjoint bit ranges, and `x | y == x ^ y` whenever `x & y == 0`).
- **coremark** (2): both equivalent by arithmetic. The list tie-break's `>` to `>=` shifts an
  equal u16 past an equal u16, which is bytewise identity, so the published-CRC pin cannot see
  it; the fsm counter tops out at 256, so bits 16 and up are zero down both shift directions.

- **compositor** (66, the largest cluster): the pattern generators mixed bits no test compared
  against a known answer, so every `&` could become `|` and every shift could reverse. Killed by
  five tests: two hand-computed pixels per generator at coordinates whose bit patterns
  distinguish the operators, the surface checksum pinned to an independently computed FNV-1a
  value plus a read count, the window digest cross-checked row-major, stride and
  `MAX_SURFACE_BYTES` as exact numbers, and a zero-width rect asserted empty. Equivalent (14, audited one by one):
  min/max selections at equal operands, `|` vs `^` over disjoint masked bit fields, `intersect`'s
  early return (the arithmetic path returns EMPTY anyway), and a max-accumulate's `>` at equality.

- **nifefs** (12) and **dma_validator** (12): all 24 were real gaps, none equivalent, which
  fits both crates' role as trust-boundary parsers. Two recurring causes: layout constants with
  no independent pin (every test compared an image against the constant it was built from, so
  both sides moved together; the documented values are now hand-computed in the tests), and
  boundaries never hit exactly (a file ending exactly at the image end now round-trips; one byte
  under is truncated). dma_validator's ring tests had all used batch indices where `slot * 2`,
  `slot + 2`, `idx % 8` and `idx / 8` coincide; a batch starting at index 11 separates all four,
  with poisoned slots to catch a walk landing anywhere but the declared next. Six of the subtlest
  kills were verified by applying the mutation by hand and watching the test fail.

- **frames** (5): three real (a stuck `Some(true)` from `is_used`, `index_of` refusing the base
  frame, a zero-size `mark_used` rounding up to one frame), killed by
  `the_base_frame_and_the_empty_range_are_exact`. Equivalent: the alloc hint (an optimization; a
  scan starting on the just-used frame finds the same next free one) and `alloc_contiguous`'s
  early return (no run of zero or of more than `total` ever matches, so the scan reproduces it).

- **elf** (10): all real, none equivalent. Every fixture set PF_R, so `is_readable`'s mask could
  become any operator and the validator's execute-only branch was dead in the suite; an
  execute-only segment now asserts both sides, the header-table bounds get their exact edges, and
  `u16le` is pinned on bytes whose halves differ (every field in the old fixtures had a zero high
  byte, so reading the wrong neighbour byte read the same).
- **entropy_protocol** (3): `op` pinned with a non-GET opcode (`GET` is 1, so a body replaced by the
  constant 1 passed every round trip). Equivalent: `|` vs `^` over disjoint masked operands, and
  `want`'s `>` at `n == MAX_BYTES`, where both branches return the same 8.

- **graphics_protocol** (22): the test pattern's channel math had no pinned pixel, so a wrong buffer
  could only be wrong the same way on both sides; five hand-computed pixels, a one-bit-change
  digest test (an FNV whose xor became or collides exactly where it matters), and the errno's
  minus sign. Equivalent (7): OR-vs-XOR in `req` and the `rect` packing, where every field is
  masked into disjoint bits.
- **glob** (21): the first pass wrote both of its new tests inside `#[cfg(kani)] mod verification`
  instead of `mod tests`, so `cargo test` never compiled them and they killed nothing. The
  paragraph that claimed fourteen was describing tests that had never run, and `.cargo/mutants.toml`
  already records why: `mod verification` is invisible to `cargo test`, which is the reason mutants
  inside it are excluded. Both tests are good and both kill once moved, and moving them takes
  eleven of the twenty-one. **`script/lint` now fails on a `#[test]` inside a `#[cfg(kani)]`
  module**, because there are 22 such modules in the tree and this failure mode reports a coverage
  it does not have. The step count is the DoS-bound contract (Kani proves only that it stays under
  `cost_bound`, which a counter stuck at zero also satisfies), so each class feature now costs
  exactly its own scan; and `[A-\]]` matches at its endpoint, resumes after the escape, and a class
  ending in a bare escape is unterminated rather than a read past the end. The remaining three
  needed arithmetic nobody would guess at. `scanned += after_hi - after_lo` charges exactly the
  bytes a range skips, so a resume index landing *inside* the range re-scans one at a time what it
  should have stepped over and the ledger balances; no step assertion can see it. Membership can:
  resuming at the `-` makes `[a-c]` match a hyphen, resuming at the high end makes `[z-a]` match
  `a`, and a wider class is a larger grant. The other coincidence is `2 + 2 == 2 * 2`, because at
  the head of a one-member class `scanned` is 2 and the tail is 2 bytes, so the addition and the
  multiplication agree and so do `4 - 2` and `4 / 2`; one member in front (`[xa-c]`) separates all
  four. The 7 timeouts are all detected hangs, every one a cursor that stops advancing. No
  equivalent mutants and nothing deferred: 21 of 21.

- **dtb** (52, second largest): one recurring cause. Every test tree declared the fixtures' 2/2
  cell layout, so the `#address-cells` match arms could be deleted, inheritance could stop at the
  default, and a `reg` could decode with its own node's widths instead of its parent's, all
  invisibly. Closed in two passes, nineteen `hostile.rs` tests in all: the first eleven covered
  the 1-cell layout, inheritance, parent-vs-own decode, the compatible walker's root guard, prop
  lookup, a reservation block with entries (including one at address zero, which an `&&` rotted
  to `||` reads as the terminator), initrd's widths and exclusive end, and the header comparisons
  met exactly. **A second pass audited the first against the survivor list and found thirteen not
  actually killed**, most of them the `reserved_memory_regions` cluster nothing had ever called
  off the fixtures; eight more tests closed those, three verified by applying the mutation and
  watching the one test fail. The audit is the honest caveat on hand-triage: reasoning about
  which test kills which mutant is fallible, and the weekly rerun is the check on it. The 20
  timeouts are detected hangs, each confirmed by applying the mutation and watching the suite fail
  to finish. Fourteen are a cursor `+=` becoming `-=`, where the walker bounces between two offsets
  that both read as an empty-named `FDT_BEGIN_NODE`. The other six are a plain `+` becoming `-` in
  `let value_at = at + 8`, a different mechanism worth naming: `at` lands back on the same
  `FDT_PROP` token and the walker re-reads one property forever. Equivalent: `cells()`'s `|` vs `^`
  (ORs into freshly shifted zeros), confirmed by reading the masks and by running it.
- **fs_proto** (72 after the `proofs::` exclusion; the largest crate): two findings. Most
  survivors were equivalent, not gaps, and a later pass audited that claim **site by site rather
  than in aggregate**, because "the masks are disjoint" is a sentence about 26 different lines.
  Every request word, spec word and rights bundle ORs fields the code masks into disjoint bit ranges
  (`fs::req` splits 63:56, 55:40 and 39:0; `grant::spec` puts the length under `0xff` and the rights
  above bit 8; `xattr::spec` and `xattr::reply` split at bit 32; the six `dir` rights and the eight
  `attrs` claims are distinct single bits), so `|` and `^` agree on every reachable input. All 26
  were checked against the constants and 14 hand-mutated, one per distinct expression. Two more are
  the `verb::TABLE` order check inside a `const _: ()` block, where both mutants only make the loop
  vacuous and a `const` item has no runtime observable, so no test can reach them; the property is
  pinned at runtime by `every_verb_has_a_row_that_says_what_its_words_mean`. **The audit found one
  claim that was wrong.** `xattr::store::write_record`'s guard is not a mask, and with its second
  `||` turned into `&&` the value limit stops being enforced, because the name clause beside it is
  dead in both callers. That limit is load-bearing: `set` and `remove` re-emit records whose lengths
  come off the blob rather than from a bounds-checked caller, and a value length is a `u16` reaching
  65535 against a `MAX_VALUE` of 3072, killed by
  `a_record_wider_than_the_contract_is_refused_when_a_blob_is_rewritten`. The real gaps were the rights bundles as numbers
  (an `&` in `REMOVE_TREE` collapsed it to zero), one sentence per explained errno, the verb
  predicates as an exact partition, both dirent length limits, `pack_name` handed a
  seventeen-byte name (the mutated bound indexes past the packed word and panics), the nameset
  cursor byte for byte, the attribute record's exact bytes at an exactly sized buffer, and every
  witness verdict module pinned to distinct single bits, because both ends of a QEMU test build
  their words from the same constants and a shifted-to-zero claim silently stops being checked.

- **gpt, second pass** (18 more): the interrupted run's resume reached the modules the first pass
  never got to (entry, guid, header, span, the MBR validator), and the survivors were the same
  two shapes. Boundaries never met exactly: a header the size of its block, a partition on the
  first usable block, an exact-fit name buffer, entry sizes other than 128 (every fixture uses
  128, so the size guard had never been *judged*). And values every fixture shared: the
  protective record always in MBR slot 0, where `i * 16` is immune to arithmetic mutation, and
  every partition name ASCII, whose UTF-16 high byte is zero, which hid an off-by-one in name
  decode. One test each; `check_protective_mbr` also needed one bad block through the wrapper,
  since every existing call handed it a good one. Equivalent: the hex-nibble `|` vs `^` (disjoint
  bits), the span guard on a 64-bit host (it stops being equivalent the day anything builds for a
  32-bit target, which is exactly when you want the signal), `1 << 0` under shift direction, and `<`
  vs `<=` at the backup-reserve underflow guard (a disk that small dies on another arm of the same
  refusal). The audit found one more real gap of the first shape: `check_partitions`'
  `last_lba < first_lba` rotted to `<=` refuses a partition of exactly one block, which is legal
  because `last_lba` is inclusive, and every partition in both suites spans thousands of blocks, so
  the check had only ever been met from far away.

- **intrusive** (3): the run queue's `len` was asserted after every push and never after a pop
  (`-=` could become `+=`), and `is_empty` was only met empty; both now asserted mid-drain in
  `fifo_order`.
- **ipc** (5): the manual `PartialEq` impls were only ever compared equal, so a body stuck at
  `true` passed; distinct variants now must disagree and the hang-dump `Debug` strings are
  pinned. Equivalent: `one_queue_invariant` replaced with `true`, because the API maintains the
  invariant, so no reachable endpoint state returns false; the checker exists for kernel-side
  debug assertions whose states are built by kernel code this crate cannot construct.

- **grant_plan** (26): 25 real. The tokenizer and parser were only ever fed lines whose cursor
  arithmetic coincided at small indices (`i + 1` and `i * 1` agree at 1), so buffer ceilings, a
  trailing `--mem`, and the flag-cluster bound could all rot into indexing past the line; the
  recursion bit was only tested on `rm`, whose `r` is bit 0, where shift direction cannot matter;
  `prog_id`'s round trip used id 1, the constant the mutant substitutes, and `from_id`'s arm list
  had drifted by one program. Refusal sentences and Debug renderings pinned exactly. Equivalent:
  `RECURSIVE`'s `1 << 0`. Deferred finding, integrator's call: `NameSet`'s Debug renders numeric
  byte lists, not the legible names its own header promises; pinned as-is because the fix changes
  rendered output.

- **isa** (22): 12 real, and the shape is the wrong-accept this crate exists to prevent. Both
  riscv64 fixtures declared their narrowest hart first, so the widest-wins fold never replaced an
  answer it already held: rv32 after rv64 could keep rv64 (and boot on the rv32 machine), and a
  base-less extensions node could clobber the base under one flipped `&&`. A wide-first fixture
  exercises every replacement arm; `Missing::any` gets one single-missing case per clause;
  ASIDBits `0b0000` must decode to the 8 bits `crates/address_space_identifier` assumes. Equivalent (10): idempotent
  assignments at `<=` boundaries, disjoint-operand `|` vs `^`, `1 << 0`, and six compile-time
  duplicate-check loops whose guards are vacuous on a table that already passes, a pattern the
  weekly report will keep resurfacing (noted here so nobody re-triages it).

- **measured_boot** (9): one real (uppercase hex was dead in the suite; every test round-tripped
  through our own lowercase `to_hex`). The other eight are textbook equivalent mutants worth
  naming because they will resurface weekly: SHA-256's `ch` has operands masked disjoint by `e`
  and `!e` (`|` equals `^`), `maj` computed with OR is the same majority function as with XOR
  (they agree wherever at least two inputs agree), `update`'s boundary take is the same number
  down both branches at `len == want`, and the hex nibble combine ORs into cleared low bits.

- **paging** (39): **seventeen real, and not one of them a bug**, which is the result to want from
  page-table math. Eleven are accessors and predicates nothing inside the crate reads back:
  `Flags::bits`, `is_user_page_va` (whose `&&` to `||` mutant shorts the user-VA gate into an
  or-gate that admits an aligned *kernel* address to a user `MAP`), `PageFormat::half_base`, and
  `Mapper::root`, whose value is what the kernel writes into TTBR0/satp, so a constant there
  installs the wrong table in silicon while every walk test still passes. Four more are descriptor
  bits the portable `Flags` do not carry, so the encode/decode round-trips are structurally blind
  to them: aarch64's `TABLE_OR_PAGE` (bit 1, which `is_present` never reads, so a descriptor that
  loses it walks perfectly in software and is a translation fault at L3 and a *block* descriptor
  above it), aarch64's inner-shareable `SH` on normal memory (the `delete !` mutant hands it to
  device memory instead, leaving normal pages with a core-private view another core's write never
  invalidates), and Sv39's `D`. The last two are in the DMA domain, where every existing test
  granted exactly one page: the region shape that cannot tell a page count from the constant `1`,
  nor `grant_page`'s whole-page test from its opposite, both killed by a grant of two pages and a
  half. Equivalent: twenty-two, in two families. `1 << 0` to `1 >> 0` (`CAP_WRITE`, aarch64
  `VALID`, Sv39 `V`) and `0b00 << 6` (`AP_RW_EL1`), the degenerate shifts named above; and every
  `|` to `^` in the `Flags` constructors and in `table_entry`/`leaf_entry` on both formats, because
  the six `CAP_*` are distinct single bits and a descriptor's address mask (aarch64 bits 47:12,
  Sv39's PPN at 53:10) shares no bit with its attributes or with valid/type at bits 1:0.
- **pci** (31: 27 missed, 4 timeouts): nineteen real, and they share one cause, which is that the
  fake config space held exactly one device. Enumeration's multifunction handling was never
  exercised (nine mutants on the single line that reads the header type), the per-function "nobody
  home" check was never reached because no device had a function 1, and the fixture's 64-bit BAR
  was both unassigned and in the last slot pair, so a base that dropped its high half and a walk
  that ran one slot past BAR5 both read as correct. Killed by
  `enumeration_follows_the_multifunction_bit_and_skips_what_does_not_answer`,
  `an_empty_slot_costs_one_config_read` (the vendor-id guard changes no output at all, only read
  count, so a count is its only witness: 45 reads on a 32-slot bus against 306 without it),
  `a_64_bit_bar_in_the_first_slot_keeps_its_high_half`,
  `the_command_bits_are_the_specified_positions`, and
  `a_function_without_a_capability_list_is_not_walked`. Equivalent, seven, all one argument: the
  `|` to `^` flips in `ecam_offset`, `requester_id`, and both halves of `read_bars`'s 64-bit
  assembly are ORs of **disjoint** bit ranges (`bus:8 | dev:5 | fn:3` packs without overlap for
  every BDF a caller can produce, which is what the Kani proof assumes), and `size`'s
  `mask | 0xffff_ffff_0000_0000` is the same case against a constant. All four timeouts are
  genuine detected hangs on the BAR cursor, and a fifth mutant joined them: `i += 2` to `i *= 2`
  differs from the original only at slot 0, so the new first-slot fixture turned a silent survivor
  into a hang.
- **line_editor** (48): 45 real, and they cluster by what the terminal model hid. `Screen` asserts
  what the user sees, which is the right contract, but a test that checks only the finished line
  never proves *where the cursor is*, so the whole movement layer was mutable at will: ^E and ^F
  could be deleted, `right` could be emptied to `()` or have its `<` flipped four ways, `left`'s
  `> 0` could become `>= 0`. `control_key_movement` and `right_arrow_moves_and_stops_at_end` type a
  character after every move and close all nine. The history ring's index arithmetic
  (`(hist_next + HIST - k) % HIST`, eight mutants across `repeats_newest` and `hist_next_entry`)
  survived because no test crossed the wrap point, where a `+` for a `-` and a `%` for a `/` still
  land on a plausible entry; `wrapped_ring_walks_correctly_both_ways` walks ten commands through a
  ring of eight in both directions, `duplicate_entry_is_stored_once` pins the dedup by bell count
  (the screen cannot tell one stored copy from two, which is why `repeats_newest -> false` survived
  everything else), and `empty_lines_stay_out_of_history` covers the `len > 0` half of the same
  guard. The repaint arithmetic (`len - cur` in `start_line`, in ^L, in `yank`, in the stash
  restore) needed a cursor left off the end and *then* a keystroke, since the echo alone redraws
  the same glyphs either way. The rest are one test each: the CSI `;` arm and its `n += 1` (one of
  them a `-=` that underflows to a panic), ^W's two scan loops (whose `>` to `>=` reads `buf[a - 1]`
  off the front), the three-way split at `kill_len > 0`, and `csi_move`'s digit loop, which nothing
  had ever driven past nine columns. Equivalent (3): `req`'s `|` to `^` (opcode in bits 63:56,
  length in 31:0, disjoint), `FLAG_EOF`'s `1 << 0`, and `csi_move`'s `1` arm, which only elides a
  count: delete it and `n == 1` emits `CSI 1 D`, which ECMA-48 defines as the same motion as
  `CSI D`. That last is byte economy on a serial line, not behaviour, and the check that it is not
  an excuse is its sibling one line up: deleting the `0` arm makes a zero-column move travel a
  column, and `backspace_erases_on_screen` kills it.
- **video_terminal** (79, the largest single block in the run): **64 real, 15 equivalent, none
  deferred**, and the shape splits three ways. The geometry and colour accessors were never called
  with a *number*, only compared against another value computed the same way, so `cols()`,
  `width()`, `height()`, `colours()` and `to_pixels` could return a constant or swap an operator
  unnoticed. A 6 by 3 grid separates `6*8` from `6+8` from `6/8`; three pixels of a glyph at cell
  (1,1) separate a divide from a remainder, which agree at (0,0), where every existing pixel
  assertion sat; and a union with a rect *inside* the first is the only shape that reads the first
  operand's far edges. The parser's less-travelled arms were individually deletable because nothing
  fed them: a tab, a bare control code, a string terminated by `ESC \`, `CSI 1J`, SGR 27, 39, 49
  and 90-97. The switches that turn something *off* are the ones worth naming, because a terminal
  that only ever sets attributes passes every test while leaving a line reversed forever. Third,
  clamps and damage were asserted only where they did not bind: `\x1b[99B` and `\x1b[99;99H` now
  pin the clamps, since one too generous parks the cursor off the grid where every later write is
  silently dropped, and a bare `LF` on the bottom row with the cursor hidden is what makes
  `damage_all` non-deletable. Equivalent (15): disjoint-mask `|` versus `^` at three sites,
  `DEFAULT_BG << 4` on a zero operand, the four min/max selections in `union` at equality, SGR
  40-47's `p - 40` versus `p + 40` (40 is a multiple of 8 and the background is masked to three
  bits, so they agree for all eight legal parameters), `erase_display`'s `to > from` (true for
  every mode), two match arms that fall through to a byte-identical body, and three range guards no
  caller can reach because `col < cols` and `row < rows` are invariants of every write. Those last
  three are the ones most worth leaving visible: a new caller that broke either invariant turns all
  three back into real bugs. **The correction worth recording**: `csi`'s `>` to `==` at the
  parameter limit was argued equivalent by hand and is not. `== MAX_PARAMS` fires on the fourth
  separator, so it drops a legal `CSI 1;2;3;4 m` entirely, and only running the mutation caught it.
- **slots** (5): four real, all in the half of the API the Kani harnesses never touch. `get_mut`
  could return `None` for a live name with nothing on the host noticing, while the kernel
  `unwrap()`s it on the switch path, so the mutant is a kernel panic dressed as a lookup miss;
  `is_empty` could be stuck at `true`, stuck at `false`, or inverted, because nothing in the tree
  calls it at all, which is exactly the accessor that rots unobserved. Killed by
  `get_mut_reaches_the_live_entry_and_writes_through_it` and
  `a_table_is_empty_only_while_it_holds_nothing`. Equivalent: `name`'s `|` to `^`, where the
  generation is shifted into bits 63:32 and the slot is `< N <= u32::MAX` by the const assert in
  `new`, so the two operands never share a bit.
- **socket_protocol** (2): one real. `DATA_MAX` is `4096 - OFF_PAYLOAD` and the test only asked
  whether a full payload *fits* the frame, which `4096 / OFF_PAYLOAD` also does, so the constant
  could shrink by 3576 bytes unnoticed; it is now pinned as the whole page after the header, which
  refuses both a payload that overruns the grant and one that leaves granted bytes unreachable.
  Equivalent: `req`'s `|` to `^`, where the opcode is a byte and the socket id is shifted past it,
  and the crate's own `every_opcode_fits_in_its_byte` is what keeps the two ranges disjoint.
- **sink_proto** (2): both equivalent, no gaps. `req` ORs an opcode shifted to bits 63:56 into a
  length masked to bits 31:0. And `pack`'s `bytes.len() < INLINE_MAX` to `<=` is the boundary where
  both arms return the same number: at exactly sixteen bytes `bytes.len()` *is* `INLINE_MAX`, so no
  slice length distinguishes them.
- **user_mode_heap** (6): three real, all in the split arithmetic, and the reason nothing saw them is
  that a block's *size* is never readable. `free_bytes` is an independent counter a wrong split does
  not touch, and every test asked only for the block **count**, which the coalescing invariant
  expects to be 1. So `block_count` could return the constant 1 and pass everything; `alloc`'s
  `tail > 0` could become `>=`, writing a zero-length free node one node past the donation; and
  `bsize - front` could become `bsize + front`, inflating the remainder by twice the front padding
  and leaving the heap willing to hand out memory nobody gave it. Killed by
  `a_block_that_fits_exactly_leaves_nothing_behind` and
  `a_split_never_invents_bytes_nobody_donated` (the first donates its arena away from the process
  heap on purpose, so a mutant writing past the region lands in slack rather than in the test
  runner's own allocations). The 3 timeouts are detected hangs of the same shape: a wrong `front`
  makes `insert_free` write a node whose `next` points at itself. Two of the three fail an assertion
  before anything hangs and are recorded as timeouts only because the thrashing test spins in the
  same binary, which is the honest caveat on reading a timeout as "not caught".
- **swish** (2 real, 1 equivalent, 9 timeouts): the shell's own sentences, where a regression is
  user-visible and nothing else looks. `echo`'s `i > space` could become `>=`, emitting a
  zero-length whitespace run; in the program `out` is the terminal endpoint, so an empty write is a
  round trip carrying no bytes. `write_pwd` could be replaced with nothing at all, because no test
  called it, so a `pwd` that printed an empty line would have shipped. Equivalent: deleting
  `write_refusal`'s bare `Refusal::NoSuchProgram` arm, because the guarded arm above takes every
  non-empty program name and the `_` arm it falls into prints a prefix only when `Prog::from_name`
  resolves, which an empty name never does. The 9 timeouts are all detected hangs in `echo`'s
  two-cursor scan.

## The ledger: every survivor's disposition

The triage rule says nothing stays untriaged, so this is the accounting that backs the claim. Rows
are the run's survivors (missed plus timeout); "killed" means a test was written and **verified by
applying the mutation and watching that named test fail**, "equivalent" means the mutant provably
cannot differ from the original, and "hang" means a timeout that was confirmed to be an infinite
loop rather than an undetected bug.

| crate | survivors | killed | equivalent | hang | deferred |
|---|---|---|---|---|---|
| video_terminal | 79 | 64 | 15 | 0 | 0 |
| line_editor | 48 | 45 | 3 | 0 | 0 |
| fs_proto | 46 | 1 | 36 | 9 | 0 |
| grant_plan | 46 | 25 | 1 | 20 | 0 |
| paging | 39 | 17 | 22 | 0 | 0 |
| pci | 31 | 20 | 7 | 4 | 0 |
| isa | 25 | 12 | 10 | 3 | 0 |
| glob | 21 | 14 | 0 | 7 | 0 |
| dtb | 21 | 0 | 1 | 20 | 0 |
| network_time_protocol | 16 | 4 | 12 | 0 | 0 |
| compositor | 14 | 0 | 14 | 0 | 0 |
| measured_boot | 13 | 1 | 8 | 4 | 0 |
| swish | 12 | 2 | 1 | 9 | 0 |
| calendar | 10 | 2 | 5 | 3 | 0 |
| clock_protocol | 9 | 0 | 2 | 7 | 0 |
| user_mode_heap | 8 | 3 | 0 | 5 | 0 |
| gpt | 6 | 1 | 4 | 1 | 0 |
| graphics_protocol | 6 | 0 | 6 | 0 | 0 |
| slots | 5 | 4 | 1 | 0 | 0 |
| ipc | 5 | 4 | 1 | 0 | 0 |
| frames | 4 | 1 | 2 | 1 | 0 |
| cred | 4 | 1 | 2 | 1 | 0 |
| intrusive | 3 | 3 | 0 | 0 | 0 |
| coremark | 3 | 0 | 2 | 1 | 0 |
| the 2-survivor crates | 12 | 1 | 11 | 0 | 0 |
| the 1-survivor crates | 4 | 0 | 4 | 0 | 0 |
| **total** | **487** | **225** | **170** | **95** | **0** |

The 2-survivor crates are `asid`, `credential_protocol`, `entropy_protocol`, `byte_sink_protocol`, `socket_protocol` and
`generational_table`' siblings; the 1-survivor crates are `abi`, `block_roster`, `c_seam` and `capability`. Their
survivors are the recurring patterns named at the top of this section, one or two each.

**Nothing is deferred, and that is a claim worth being suspicious of**, so here is what it rests on.
Every "equivalent" in the table was argued from the code, and in the crates a later pass audited
(compositor, frames, calendar, cred, clock_protocol, gpt, fs_proto, dtb, glob) every one was also
**re-run under its mutation**. That audit changed six verdicts: five mutants called equivalent were
real gaps (`frames::index_of`'s upper bound, `calendar::from_hm`'s sign guard and its offset-length
check, `cred`'s memory ceiling, `gpt::check_partitions`' one-block partition), and glob's entire
first pass turned out to have written its tests where `cargo test` could not see them. **A verdict
reached by reading is wrong about ten percent of the time; a verdict reached by running is not.**
The crates that were not re-audited (`grant_plan`, `machine_discovery`, `measured_boot`, `network_time_protocol`, `ipc`,
`intrusive_fifo`) had their kills verified the same way when they were written, but their *equivalence*
claims rest on argument alone, and the weekly run is what will check them.

**The alarming survivors, named.** A survivor in a security boundary is worth more attention than
fifty in a display crate, so: `capability`, `memory_regions`, `dma_validator`, `nifefs` and `elf` have
**zero real survivors** between them, and the three trust-boundary parsers score 100%. The one
security-relevant survivor the run found anywhere was `filesystem_protocol::xattr::store::write_record`, whose
value limit stopped being enforced under a single `||` to `&&`, on a path that re-emits records
whose lengths come off the blob rather than from a bounds-checked caller. It is closed. The
next-most-serious were `paging`'s user-VA gate and `Mapper::root` (a constant there installs the
wrong table in silicon), `machine_discovery`'s widest-wins fold (an rv32 hart booting an rv64 answer), and
`generational_table::get_mut` (a `None` the kernel `unwrap()`s on the switch path). All closed.


## 2026-09-03: `measured_boot` re-run, and its five survivors proved equivalent rather than argued

Milestone 246 moved measured boot's load-or-refuse decision into this crate, so the crate was re-run
whole: **151 mutants, 133 caught, 5 missed, 4 timeouts, 9 unviable.**

**The new function is one mutant and it is caught.** `verdict` is the decision that says whether
unmeasured code may run, and `cargo mutants`' only operator on a function returning a struct is to
replace the body with `Default::default()`. Without a `Default` impl that does not compile, so the
first run scored it **unviable** and the tool said nothing at all about the line. `Verdict` now
derives `Default`, which is both the fail-safe value and exactly the dangerous wrong answer here (an
absence where there was a refusal); `cargo mutants --in-diff` over that lane's diff went from
*1 unviable, 0 tested* to *1 caught*. **A crate can score 100% on a function the tool never
mutated**, which is the general lesson: an unviable mutant is a hole in the measurement, not a pass.

**The five missed are provably equivalent**, which upgrades this crate's row in the ledger above from
"argued" to proved. The section on scope says an equivalence verdict reached by reading is wrong
about ten percent of the time; these five are algebraic identities rather than readings:

- `Sha256::compress`, `ch = (e & f) ^ ((!e) & g)`, `^` to `|`. The two operands are disjoint by
  construction (`e` and `!e` cannot both be set in a bit position), and `^` and `|` agree wherever
  the operands never both hold.
- `Sha256::compress`, `maj = (a & b) ^ (a & c) ^ (b & c)`, `^` to `|`, twice. Count the set inputs in
  one bit position: at 0 or 1 every pair-AND is 0, at 2 exactly one is 1, at 3 all three are 1. XOR
  and OR agree on all four counts (0, 0, 1, 1), which is why both spellings of `maj` compute the
  bit-majority.
- `parse_hex`, `(hi << 4) | lo`, `|` to `^`. `lo` is a nibble and `hi << 4` has its low four bits
  clear, so the operands are disjoint and the two operators agree.
- `Sha256::update`, `bytes.len() < want` to `<=`. In the equal case both arms yield `want`.

**No test can kill any of the five**, so nothing here is deferred and nothing is a gap. The four
timeouts are `update`'s loop-control mutants, which hang rather than lie, and are the same family
this note records elsewhere.


## Scope and honest caveats

- **Scope is the main workspace's host crates.** The exclusions (and their reasons) are in
  `.cargo/mutants.toml`: the bare-metal crates cannot compile for the host, `supervision_protocol`,
  `swap_protocol` and `virtio` compile but cannot execute a line without a kernel underneath, and
  `xtask` is the build system, whose tests are the gates it runs.
- **`redoxfs_server` and `tools/redoxfs_host` are not mutated.** Each is its own workspace (kept out of
  ours so upstream RedoxFS never meets our clippy/fmt gates), and cargo-mutants works one workspace
  at a time. `redoxfs_server`'s pure logic is small and host-tested, but a run there mutates against a
  suite whose heavy half lives under QEMU (`script/test`'s redoxfs leg), so its score would
  overstate the gap. Deferred, on the record, not forgotten.
- **A survivor count is not a quality score across crates.** Crates differ in how much of their
  surface is host-assertable; compare a crate to its own last week, not to its neighbours.
- **Timeouts are auto-derived** by cargo-mutants from each package's baseline build and test time,
  so a mutant that makes a loop spin forever is recorded as `timeout`, not hung. The baseline's
  timeouts were checked and are detected hangs (cursor arithmetic in walkers), which is the tests
  noticing, not missing; a timeout on a mutant that could NOT hang would be triaged as a survivor.
- **A mutant that hangs is not a mutant that survived, and this instrument cannot say so.** The
  point above is the reading; this is the limitation underneath it. cargo-mutants 27.1.0's complete
  set of limits is the clock (`--timeout`, `--build-timeout`, their two multipliers and
  `--minimum-test-timeout`, checked by milestone 277 (bound what one mutant may allocate) rather
  than assumed), so a suite that deadlocked and a suite that was merely slow produce the same `TIMEOUT` row, and
  `script/mutation --report` lists both under "the survivors themselves". Nine survivors across
  milestone 326 (nobody has been assigned to turn a mutation score upward)'s two lanes were
  non-terminating rather than wrong, and each had to be argued in prose, in this file, one at a
  time. **What would close it** is a rule in `--report` comparing a
  timeout against the package's own baseline test time: a mutant that exceeds it by orders of
  magnitude is a deadlock, one that exceeds it by a factor of two is a slow test. Until then, read
  every `timeout` row as unclassified rather than as a survivor, and expect the triage to say which
  it was.
- **A deadlock in one test hides an assertion failure in another**, which is what makes the point
  above cost something rather than merely being imprecise. The classification is per *run*: if any
  test in the binary hangs, the mutant is a `TIMEOUT` however loudly the others failed. Milestone
  326 met this twice, in `memory_corruption_canary_gate` (where deleting `ArmGuard`'s `Drop` fails a
  named assertion and hangs a sibling test) and in `jh7110_entropy` (where `Pool`'s own **doctest**
  calls `take` directly, so no change to the test module can move the classification; confirmed by
  hand-applying the mutant and watching `cargo test --doc` sit at "has been running for over 60
  seconds"). The lesson for a triage: bounding *one* blocking call proves the property but does not
  move the number, and bounding *all* of them is only possible where no doctest blocks.

## 2026-09-04: `uefi_loader`'s 15% was measuring a file nothing compiles

The section above names `uefi_loader` as one of the two crates that carry the tree's fall from 92.4%,
at **3 caught, 17 missed** in the round-robin sample. That crate is the code that boots xenon, on a
machine where a fault has no console and no debugger, so it went first.

**The number was arithmetic, not a finding, and it is the `system_initializer` result again in
different clothes.** Milestone 244 found the largest crate in the tree scored 0 of 191 because the
host suite could not compile a line of it. This is the same failure one level down, at a **target**
rather than a crate: `uefi_loader`'s `[[bin]]` carries `required-features = ["uefi"]`, so
`cargo build --workspace` and `cargo test` never put `src/main.rs` in the build graph at all.
cargo-mutants does not read the build graph. It edits the file, nothing rebuilds, the tests pass, and
every mutant is recorded MISSED **in "0s build + 0s test"**, which is the tell and is printed in the
output nobody was reading.

The whole-crate run on 2026-09-04, before any change:

| | mutants | caught | missed | unviable | killed |
|---|---|---|---|---|---|
| `uefi_loader`, whole crate | 189 | 32 | 156 | 1 | **17.0%** |
| of it, `src/main.rs` (never compiled on the host) | 154 | 0 | 154 | 0 | 0% |
| of it, `src/handoff.rs` + `src/image.rs` (the pure half) | 35 | 32 | 2 | 1 | **94.1%** |

**So the half the design lifted out in order to be host-testable was at 94% the whole time**, and the
crate-level 15% was 154 mutants in a file the tool was alone in reading. That is worth stating
plainly, because the published rate had `uefi_loader` standing for "a subsystem nobody tests" when
what it actually stood for was a measurement bug.

**The residue is real and is not fixed by excluding it.** `src/main.rs` is 790 lines that call
firmware and leave long mode, and the only thing that proves it is `cargo xtask uefi-boot` under
OVMF, on `script/test`'s own leg. `load`, `say_conflict` and `find_screen` carry logic (66, 28 and 9
mutants) that a host test could reach if it were lifted the way `handoff` and `image` were; whether
that is worth doing is
`design/roadmap/381-the-uefi-loaders-firmware-half-is-proved-by-one-boot.md`.
Excluding the file makes the number honest, not the file proved.

### The two real survivors, and the hole beside them

- **`physical_span`'s `first & !(page_size - 1)`, twice** (`- with +`, `- with /`). Every test here
  started its lowest segment on a page boundary, where that mask is the identity and both mutants
  clear bits the inputs never had set. A segment starting mid-page separates them, and the property
  is load-bearing rather than arithmetic: the firmware is asked for this range with one
  `AllocatePages(AllocateAddress)`, which takes a page number, so a span beginning above the
  segment's first byte asks for memory that starts after the bytes about to be written into it.
  Killed by `a_segment_starting_mid_page_pulls_the_span_down_to_its_page`, verified by applying both
  mutations and watching that named test fail.
- **`parse`'s one mutant is unviable, and that hid the fact that nothing called `parse`.** See the
  next section.

**After: 35 mutants, 34 caught, 0 missed, 1 unviable. 100% of viable.** No equivalents claimed and
nothing deferred, which is a small enough crate that the claim is cheap rather than impressive.

**The gate, because none of the above should need remembering.** `script/lint` now derives from
`cargo metadata` that every target with `required-features` is excluded in `.cargo/mutants.toml`,
which is the milestone 244 gate's shape applied to the question its dependency-graph derivation
cannot see. Exactly one target matches today; the gate exists for the second one.

### The unviable mutant, and the case for leaving it unviable

`uefi_loader::image::parse`'s only mutant is `Ok(Default::default())`, and `elf::Elf` has no
`Default`, so cargo-mutants has said **nothing at all** about that function since it was written.
This is milestone 250's shape exactly, and it came with the classic symptom: **nothing in the tree
called `parse` either**, so there was no test for the tool to have failed.

Milestone 246's repair was to derive `Default` on `Verdict` and turn the hole into a kill. **That is
the wrong move here, and the reason is worth recording rather than the verdict.** `Verdict` is a data
struct whose default is simultaneously the fail-safe value and the dangerous wrong answer. `Elf` is a
**validated token**: its own doc says *"an `Ok(Elf)` has nothing left to check; every later accessor
and `segments` iteration step trusts this pass completely."* A `Default` impl makes an unvalidated
`Elf` constructible by every consumer in the tree, which is rung one of AGENTS.md's ladder run
backwards, to buy one mutant on a two-line `map_err` wrapper in one of them. 250's own BUGS section
asks whether a default is a value the code could plausibly be wrong with; here it is, and the
objection is the cost rather than the meaning.

**What was done instead is what the mutant would have asked for.** Two tests now call `parse`, and
`an_accepted_image_arrives_with_its_segments_and_its_entry` asserts precisely what an
`Ok(Default::default())` would violate: the accepted image comes back **carrying its segments and its
entry**, so a wrapper that returned an empty `Elf` fails on the host rather than booting a machine
into a kernel with no segments. The mutant stays unviable and is now a recorded hole with a test
standing where it would have stood, which is the honest disposition rather than a repair.

## 2026-09-12: bounding what one mutant may allocate

Milestone 277. Every scheduled run of the `mutation testing` workflow had died, and after milestone
238 fixed the off-by-one in the shard indices exactly one cause was left: **one mutant went 1.4 GB to
15.8 GB in twenty seconds and took the runner agent down with the machine.** The workflow's own BUGS
header has the ten-second samples that caught it, and the reason a sixty-second sampler had reported
innocence an hour earlier.

**A clock cannot catch an allocation, and a clock was the only bound there was.** cargo-mutants
27.1.0 offers `--timeout`, `--build-timeout`, a multiplier for each, and `--minimum-test-timeout`.
That is the complete list, checked against `cargo mutants --help` rather than assumed, because the
cheapest possible outcome here was that the tool already had the feature and the milestone was moot.
It does not. On this tree the auto timeout derives to 28-51 seconds, and 16 GB is gone well inside
that, so the bound that existed could never have fired.

### The mechanism: a cargo runner

cargo runs a test binary through `target.<triple>.runner` when one is set. That is the only point in
the pipeline that sees **exactly one test binary** and neither rustc, nor cargo, nor the other `-j 2`
job's binary, so it is the one place a genuinely per-mutant bound can live.
`scripts/memory-bounded-runner.sh` (**name provisional**; names are calef's) sets `RLIMIT_AS` with
`ulimit -v` and execs the binary. `script/mutation` points `CARGO_TARGET_<HOST>_RUNNER` at it for the
length of a mutation run and nothing else in the tree does, so an ordinary `cargo test` is untouched.

Putting it in `.cargo/config.toml` instead was refused for that last reason: that file would bound
every host `cargo test` in the tree, and a memory ceiling is a property of a mutation run rather than
of the tree.

### The number, from both directions

**4 GiB, and it is measured twice rather than picked.** Reading each child's own `VmPeak` across all
**143** host test binaries (`cargo test --workspace` minus the bare-metal crates, at
`--test-threads=4`):

| | peak address space |
|---|---|
| largest (`board_console`) | 1,028 MiB |
| next (`graphics_protocol`) | 481 MiB |
| mean across 143 binaries | 169 MiB |

So 4 GiB is **4.0x the largest honest test binary in the tree**. The other half of the choice is the
machine rather than the tree: `-j 2` means two test binaries can be resident at once, so a runaway in
each costs twice the ceiling, and 8 GiB is survivable on the 16 GiB boxes this runs on where twice a
larger ceiling would not be. Four is the largest value that keeps both true.

Corroborated by lowering it until it bites, which is the half that proves the bound is enforced and
not decorative. The honest suite is unaffected at 4 GiB and still unaffected at 1 GiB; at 256 MiB
`board_console` fails, exactly where its measured peak says it should.

### That it fires, demonstrated

A bound nobody has watched kill something is decoration (DECISIONS §134 is the same argument about
proofs). A synthetic crate whose `i += 1` cargo-mutants rewrites to `i *= 1` leaves the loop counter
at zero forever and pushes without limit, which is the exact shape the workflow measured. Run at four
ceilings, it dies at **whatever ceiling it is given** and nowhere else:

| ceiling | died after | allocation that failed |
|---|---|---|
| 512 MiB | 0.4 s | 536,870,912 bytes |
| 1 GiB | 1.7 s | 1,073,741,824 bytes |
| 2 GiB | 3.9 s | 2,147,483,648 bytes |
| 4 GiB | 8.6 s | 4,294,967,296 bytes |

Being bounded only by the ceiling is what "unbounded" means, and it is why no clock was ever going to
help. Under `cargo mutants` with the bound in place, that crate's full set of 8 mutants came back **8
caught** with the runaway among them and the run exiting 0.

**The accounting is the point, not the kill.** `script/mutation` treats 0 as all-caught, 2 as
survivors and 3 as timeouts, and **everything else as a broken run that exits fatal**, so a bound that
killed a mutant but turned the run red would be the same outcome with a new cause. It does not: at
8.6 s the allocation failure lands well inside the 20 s minimum auto timeout, Rust's allocation error
handler aborts the process, cargo sees a failed test, and cargo-mutants records an ordinary **caught**
mutant. The sweep continues.

The failure path is covered too. `MUTATION_MEMORY_LIMIT_KB=1024 script/mutation -p bitmap_font` fails
the unmutated baseline and exits **4**, unchanged, with a new line naming the ceiling as the suspect,
because an honest test binary that cannot fit under the ceiling fails in exactly the shape a broken
baseline does and the next reader should not have to find the runner on their own.

### What is not bounded, and where this is not enforced

- **The build is not bounded.** This wraps the test binary, so a mutant whose damage lands in rustc is
  still unbounded. None has been observed (the measured runaway is test-side), and a linker
  legitimately wants a great deal of address space, so it would need a different number.
- **It is off by default on macOS**, and that is a gap rather than a verdict. XNU **does** enforce
  `RLIMIT_AS`, contrary to the folklore and contrary to what this lane first wrote down before
  reading the source: `bsd/kern/kern_resource.c` hands the limit to `vm_map_set_size_limit()`,
  `vm_map_enter()` fails with `KERN_NO_SPACE` once `map->size` passes it, and
  `vm_map_inherit_limits()` carries it across exec, all of which has been true since at least
  xnu-8792 (macOS 13). What is **not** known is what number is safe there, because this lane had only
  Linux to measure on and macOS reserves address space far more freely than glibc does. Defaulting it
  on with an unmeasured number would fail every Mac sweep at its baseline; defaulting it off and
  saying so leaves the dev Mac exactly as exposed as it was before. Turning it on is one environment
  variable, and whoever measures it first should write the number into the runner's header.
- **The ceiling is address space, not resident memory**, so it over-counts reservations nothing
  touches: `RUST_MIN_STACK` is 16 MiB here and `--test-threads=4` makes that 64 MiB of untouched stack
  before a byte of heap. That over-count is priced into the 4x headroom rather than argued away.
- **The weekly workflow has still never succeeded.** This was demonstrated against a synthetic
  runaway and measured against the honest suite, both on Linux. The first green scheduled run is the
  evidence that matters and it has not happened yet.

## 2026-09-13: `documentation`'s 52% was a crate scored with a third of its tests compiled away

Milestone 280. The section above named `manual` (ratified `documentation` on 2026-09-13) as the
second of the two crates carrying the tree's fall from 92.4%, at **56 caught, 60 missed** in the
round-robin sample. `uefi_loader` went first and turned out to be arithmetic; this one was expected
to be the real gap, because it is ordinary host-testable `no_std` Rust with no allocator on the read
path.

**It was both, and the measurement bug was the larger half.** This is the `system_initializer`
result (milestone 244) and the `uefi_loader` result (2026-09-04) a **third** time, one level further
in again: not a crate the host suite cannot compile, and not a target the build graph never selects,
but a **Cargo feature** that a single-package test run does not turn on.

`cargo-mutants` tests one package at a time. `--test-workspace` is off by default and turning it on
would run the whole workspace suite once per mutant, so the command behind every number this sweep
has ever published for this crate is `cargo test -p documentation` with that crate's **default**
features. `builder` is not one: it gates the index **writer**, which allocates, and it is off by
default so the guest program links no allocator. Ninety-two of the crate's 1,043 mutants live behind
it, and nothing could ever have killed one.

**The writer was not the cost, though, and that is what nobody had noticed.** `builder` also gates
**six of the crate's twelve index tests**, because a test that reads an index needs `build` to make
one to read. So the run compiled away half the suite and then scored the *reader* against what was
left: `lookup`, `postings`, `page_record`, `score`, `Ranked::offer`, `search` and `location` had no
test at all in the configuration being measured.

### The numbers, whole-crate, same machine, same pinned tool

| | mutants | caught | missed | timeout | unviable | killed |
|---|---|---|---|---|---|---|
| as the sweep measures it, before | 1,043 | 499 | 455 | 48 | 41 | **54.6%** |
| with `builder` compiled, before any new test | 1,043 | 733 | 211 | 58 | 41 | **78.9%** |
| **after milestone 280** | 1,056 | 908 | 47 | 60 | 41 | **95.4%** |

"Killed" is caught plus timeout over viable, the same arithmetic as the baseline table above. The
published 52% was a one-eighth sample; the whole-crate figure in that same configuration is 54.6%,
so the sample was honest about a crate being measured wrongly.

### The fix, and why it is not `--all-features`

**A dev-dependency on itself.** Cargo allows a package to depend on itself for tests only, and
resolver 2 unifies the feature across the test build, so `cargo test -p documentation` now compiles
the builder and runs all twelve index tests while `cargo build -p documentation` still does not.
Nothing a consumer sees changes, because a dev dependency is not transitive: `user` and `swish` link
no allocator and did not have to say so.

**`--all-features` was measured rather than assumed, and it fails.**
`cargo check -p uefi_loader --all-features` panics in that crate's build script without
`NIFE_UEFI_KERNEL`, by design, because the UEFI application embeds the kernel it boots. Three
packages in this workspace declare a feature (`documentation`, `kernel`, `uefi_loader`) and they want
three different answers, so one global switch is wrong for at least two of them.

**`script/lint` gained the third gate in this family**, beside the two the earlier two findings left.
It derives from `cargo metadata` that every feature of every mutated crate is enabled when that
crate's own tests run, and takes `required-features` as the one honest exemption: a feature whose
only job is to gate targets already excluded (`uefi_loader`'s `uefi`) has no library code to compile.
Verified in both directions, by removing the dev-dependency and watching it name the crate and the
feature.

**`script/coverage` hit exactly this failure on this crate two weeks earlier and fixed only itself**,
which is the reason a gate was worth the twenty lines. On 2026-08-30 (PR #590, milestone 87) the
coverage floor reported 33.3% on an index reader that is round-tripped against its own writer,
because `builder` was off. It grew `--features documentation/builder` and a comment saying that a
feature gating something un-buildable *"is a trap for every workspace-wide sweep, not just this
one"*. The mutation config, whose own head comment asks the next person to keep the two lists in
step, was not touched, and fourteen days later published 52% for the same crate for the same reason.
That is rung four doing what rung four does.

### Two features the sweep found that no test could have

Both are the same shape and it is worth naming, because a mutation run is the only instrument in the
tree that sees it: **a value computed correctly and read by nobody.** There is no output for a test
to assert, so the code cannot fail; the tell is a *cluster* of survivors inside one function,
including the mutant that replaces the whole function with `()`.

- **Table column alignment.** `read_align` parsed `:---`, `---:` and `:---:` off the delimiter row
  into a per-column array from the first day, and `flush_table` padded every cell on the right
  whatever that array held, so all three rendered identically to `---`. Sixteen survivors. It is
  honoured now, and resets with `delimited`, because both are properties of one delimiter row.
- **Tab indentation.** `indent_of` returns a byte offset and a column count, counting a tab as four
  columns; every caller took the offset and dropped the count, including the one that uses it as a
  margin. Invisible on this corpus, which is why only a sweep could have found it: every
  tab-indented line in this repository's markdown is inside a fence, where that code does not run.

### The ledger for this crate

Forty-nine tests were added across four passes (the crate had 32 and has 81), each written against a
named survivor rather than against a feature list. The dispositions of what the final run still reports, using this note's own
vocabulary:

| | count | what they are |
|---|---|---|
| killed | 164 | the fall in missed mutants from 211, each kill verified by the sweep itself |
| equivalent | 17 | proved unable to differ; the groups are below |
| hang | 60 | every timeout is a loop counter or a loop bound, listed below |
| deferred | 30 | real gaps whose test needs a fixture sitting on an exact byte boundary |

**The equivalence groups, because a verdict reached by reading is wrong about ten percent of the
time and these are the ones to re-check first.**

- **`lookup`'s `here` (3).** `(h.terms - lo * per).min(per)` is the count of records to search on the
  final page. Every mutant of it yields a value **at least** the true one, and `.min(per)` caps them
  all at the 128 records the page buffer holds, so none can under-read. The records past the real
  end are the builder's zero padding, where `cmp` reads a length byte of zero and compares an empty
  slice against a non-empty key, which is `Less` and never `Equal`. Verified by applying one of
  them: Rust's left-associativity turns `terms - lo * per` into `(terms - lo) + per`, not the
  underflow the mutant looks like.
- **The output cursor where nothing reads it (5).** `Renderer::col` has two consumers: a wrap
  decision, and `close_line`'s "is a line open" test. Inside `rule` and `flush_table`'s row loops
  the next thing to touch it is `close_line`, and every row ends with `col += width` for a width of
  at least one, so a wrong value cannot reach zero and cannot reach a wrap. The mutants
  that **can** are killed: a blank line inside a fence (the one code line of zero visible width), an
  image followed by text that wraps, and a quoted paragraph at two levels.
- **Absorbed by the code after them (8).** `line_done`'s explicit space-skip after a block quote
  marker is redundant with the `indent_of` call on the next line, twice; `heading_at` guarantees a
  space at the index `heading` slices from, and `inline` skips leading spaces, so an off-by-one
  there emits the same words; `read_align` starting at the `|` instead of past it produces an empty
  first spec, which it already skips; `flush_table`'s `vis > width` assigns the same value under
  `>=`; the column-selection `c < cols[r]` under `<=` reads a `(0, 0)` cell that is the same as its
  own else branch; `search`'s `done < count` under `<=` runs one more iteration that asks for zero
  postings and breaks; and `finish`'s `used > 0` under `>=` ends the document by classifying a line
  of no bytes, which emits nothing.
- **`title_of`'s line walk (1).** `while start < text.len()` under `<=` runs one extra iteration on
  an empty slice, which is never a heading.

**The deferrals, and what each one's test would cost.** Thirty, and they are gaps rather than
equivalents, recorded as gaps:

- **The inline scanner's `i + 1` forms at the exact last byte of a line (19).** Every one is a
  lookahead guarded by `i + 1 < end`, and the mutants either drop the guard or read one byte past
  the classified span. A test needs a line whose final byte is the opening half of a two-byte
  marker: `~`, `!` or `*` as the last character, with the construct it would open truncated by the
  line ending. Fifteen of the nineteen also need the depth counter at its bound at the same time.
- **The table arenas at their exact fill points (5).** `take_table_row`'s row buffer flushes when
  the cell text would pass 8,192 bytes, and the mutants move that threshold by one or change which
  sum is compared. A test has to construct a table whose cumulative cell text lands on 8,192
  exactly, which pins an implementation constant rather than a property, and `read_align`'s cell
  walk wants the same.
- **The output cursor where it feeds a wrap (4).** `unit`, `unit_bytes`, `unit_wrapped` and
  `before_unit`, the four places `col` is read again before `line_start` resets it. The three passes
  above killed the ones that reach zero or move a visible wrap; what is left needs the cursor wrong
  by an amount that lands on a wrap boundary.
- **`past_quote`'s loop bound (2).** Its three length tests are separated by the blank-quoted-line
  test above; the two on the loop condition itself want a fence opened at a depth greater than the
  markers any line inside it carries, at the exact byte where the line ends.

A mutation of any of these changes behaviour only for an input this repository's markdown does not
contain. That is why the deferral is honest rather than tidy, and it is also the limit of the
renderer's own argument: it was written instead of taking `pulldown-cmark` on the grounds that the
input set *is* this repository, so a gap outside that set is exactly the cost that bargain has.

**The timeouts are hangs, and the evidence is that every one of them is loop control.** Fifty-seven
of the 60 are `+=` or `-=` on a loop counter or the bound of a `while`, spread over `inline` (12),
`unit_wrapped` (6), `lookup` (5), `line_done` (5), `closer` (4), `read_align` (3), `pad` (3), and
eleven others. The remaining three replace a whole function whose return value is the loop's step:
`char_len -> 0` and `closer -> Some(0)`/`Some(1)`, each of which leaves the caller advancing by
nothing. A mutant that makes a loop stop advancing hangs rather than lying, which is
the tests noticing; the note's scope section is explicit that a timeout on a mutant that could not
hang would be triaged as a survivor instead, and none of these is that.


## 2026-09-19: milestone 326, triaging the census's regressions

The census of 2026-09-14 produced 771 survivors and nobody had looked at one. `design/fatal-risks.md`'s
risk 3 is AMBER for exactly that reason, and its stated condition for going back to green is that
milestone 326's first two parts carry no untriaged survivor. This section is that accounting, crate
by crate, in the block's own order.

Each crate below was re-derived with `script/mutation -p <crate>` rather than read out of the census
artifacts, per that block's BUGS section: the identities are not in the tree, only the counts.
**Every kill here was verified by re-running the sweep**, which is the discipline the ledger above
argues for ("a verdict reached by reading is wrong about ten percent of the time; a verdict reached
by running is not"). So is every equivalence claim: the three mutants called equivalent below are
the three the second run still reports.

### `capability`: 8 survivors, 5 killed, 3 equivalent

**Before: 60 caught, 8 missed, 16 unviable (88.2% of viable). After: 65 caught, 3 missed (95.6%).**
The baseline's single survivor in this crate was the `1 << 0` degenerate case, which is still here
and still equivalent; everything else arrived with milestone 126's `SURVEY` predicate and milestone
231's high-water mark.

**`survey_includes` had no `cargo test` caller at all**, and this is the finding worth reading twice.
Replacing the whole function with a constant `true` survived, and so did a constant `false`. The
predicate decides which threads a supervision rendezvous may *see* (milestone 126), and calef's
ruling of 2026-08-17 that a domain names its members and never acts on them is the reason it is a
separate right rather than a corner of `READ`. It is proved for every input by two Kani harnesses,
which is why this is a gap in the suite rather than in the code, but `script/verify` is a different
gate on a different cadence: a build answering "everyone is in your domain" would have reached a
reviewer with `script/test` green. Closed by
`a_survey_shows_this_rendezvous_children_and_nobody_else`, four cases against the four the reap gate
already had.

**Three mutants in the packed high-water word survived a floor.**
`the_global_high_water_mark_is_never_below_a_table_that_reached_it` asserts only `peak >= cs.peak()`,
deliberately, because `PEAK` is one static shared by every `CapabilityTable` in the binary. A floor
of five does not notice `>>` becoming `<<` in the peak half, or `&` becoming `|` or `^` in the
ceiling half, because every one of those produces a number that is still larger than five. Closed by
`the_high_water_word_reports_the_occupancy_and_the_capacity_that_set_it`, which buys exactness back
from a shared static by setting a record no other test in the module can reach: `fetch_max` only
raises, so 40 of 64 is the standing record whatever order the harness runs things in.

**The three equivalents, argued from the code and confirmed by the second run.**

- **`Rights::READ`'s `1 << 0` under `1 >> 0` (1).** Both are 1. This is the degenerate case the
  patterns section above already names, recorded rather than excluded so it stays visible if the
  constant ever moves off zero.
- **`grew`'s `self.used > self.peak` under `>=` (1).** The extra branch fires only when `used`
  already equals `peak`, where the assignment `self.peak = self.used` is a no-op and the
  `note_peak(self.peak, N)` beside it re-offers a packed word this table has already offered:
  `peak` is only ever assigned on that line, and `note_peak` is called in the same breath every
  time it is, so any state with `used == peak` has already published that pair. `fetch_max` is
  idempotent, so the mutant differs by one relaxed atomic operation and nothing observable.
- **`note_peak`'s `((peak as u32) << 16) | ceiling` under `^` (1).** The two operands have no bit
  in common: the shift clears the low sixteen bits, and the line above clamps `ceiling` with
  `min(ceiling, u16::MAX as usize)` so it cannot reach the high ones. On disjoint bits `|` and `^`
  are the same function. The clamp is load-bearing for this claim, and it is itself a caught mutant.

### `memory_regions`: 8 survivors, 6 killed, 2 excluded

**Before: 64 caught, 8 missed, 2 unviable (88.9% of viable). After: 70 caught, 0 missed (100.0%).**
Back to the perfect score the baseline recorded, and the two exclusions are a category rather than
this crate's business.

**`has_children` had four survivors, one of them the whole function replaced by `false`.** The
reason is worth keeping because it is a shape rather than an oversight: `claim_for_destroy` reads
the `children` count through `destroy_outcome` and not through this accessor, so
`a_parent_refuses_until_its_last_child_returns` proves the *refusal* without ever calling the
predicate the kernel asks first. The one test that did call it
(`a_dead_name_is_inert_everywhere`) called it on a dead name, where `get` returns `None` and the
closure inside is never evaluated, which is why `children > 0` under `==`, under `<` and under `>=`
all survived beside the constant. Closed by
`has_children_answers_for_the_living_the_childless_and_the_dead`, which asks it of a fresh root, a
root with a child, a leaf, and a dead name.

**`retype_object_page`'s arithmetic had two**, `base_page + watermark` under `-` and `watermark += 1`
under `*=`. Both are also in `retype_page`, where both are caught, and the split is the whole
explanation: every existing test calls the object retype exactly once, at watermark zero, where the
two arithmetics agree and a watermark that never advances is invisible. A page handed out twice is
two kernel objects on one frame. Closed by
`the_object_retype_walks_the_region_one_page_at_a_time`, which takes two pages from a non-zero base
and then exhausts the region through the other entry point to show the budget is shared.

**The two remaining are the loom model, and they are excluded as a class.** `Reached::mark` and
`Reached::assert` are the non-vacuity flags described in this file's own verification section, and
they sit in `mod interleavings`, which is `#[cfg(all(test, loom))]`: `cargo test` never compiles it,
so a mutant there always survives. That is exactly what `verification::` and `proofs::` are already
excluded for, and the entry added to `.cargo/mutants.toml` says so. It covers all five loom models
in the tree (`clock_protocol`, `memory_corruption_canary_gate`, `memory_regions`,
`thread_wake_handshake`, `work_steal_slot`), every one behind the same `cfg`. Their checker is
`script/interleaving-check`; excluding them here does not make them proved.

### `elf`: 6 survivors, 6 killed

**Before: 98 caught, 6 missed, 21 unviable (94.2% of viable). After: 104 caught, 0 missed (100.0%).**
Back to the baseline's perfect score, and all six sat in the machine-check machinery milestone 288
added on 2026-09-14, which is why the fall is recent.

**All six are the same mistake in two functions: a test that proves a refusal rather than the thing
being refused.** `a_binary_for_the_other_supported_machine_is_refused` iterates `FOREIGN_MACHINES`
and expects each entry turned away, which is exactly the shape milestone 288 wrote it into, and it
is satisfied by *any* wrong number. So replacing the whole derivation with `[0; _]` survived, and so
did `[1; _]`: an array naming no architecture at all would have gone on passing a symmetry test
while proving nothing about symmetry. That is the gpt lesson in this file's calibration section
(rejection is not rejection for the right reason) arriving in a crate nobody expected it in.
Closed by `foreign_machines_is_every_known_machine_but_this_builds`, which asserts the membership
rather than the consequence: every known machine is in exactly one of native and foreign.

**`machine_no_nife_build_accepts` had the mirror pair, plus its loop bound.** The function hands
back the machine number it was given, having proved no nife build accepts it, and it exists because
a test naming `EM_X86_64` as foreign stopped being true the day `x86_64` became a target
(milestone 161). Returning a constant instead survived for the same reason as above. Its loop bound
`i < KNOWN_MACHINES.len()` survived under `==` and under `>`, both of which never enter the loop and
so never check anything, and no test could see it because **no test ever gave the guard a machine it
should reject**: a guard is only tested by tripping it. Closed by
`a_machine_no_build_accepts_is_handed_back_unchanged` for the identity and
`a_machine_this_tree_runs_on_is_refused_by_the_guard`, a `should_panic` that trips it with
`NATIVE_MACHINE`. The runtime panic is the same assertion a `const` context turns into a build
error, which is what the guard is for.

### `timetable`: 48 survivors, 12 killed, 36 excluded, and `next_after` was not among them

**Before: 134 caught, 48 missed, 12 unviable (73.6% of viable). After: 146 caught, 0 missed
(100.0%).**

**The headline first, because the block asked for it.** `design/fatal-risks.md`'s risk 2 names
`next_after` as its strongest counterfactual (the milestone 6 timer drift, proved over code the
timer does not call), and milestone 326 put this crate second on the list for that reason rather
than for its rate. **No survivor touched `next_after`, or the phase arithmetic, or the firing
decision.** Every mutant in that function was caught before this lane touched anything, by
`next_after_is_strictly_in_the_future_and_keeps_its_phase` and the doctest beside it. That is
evidence *against* risk 2 in the one place the roadmap thought it most likely, and it is worth
saying as plainly as a finding would have been.

**Thirty-six of the 48 were the crate's own Kani harnesses, and that is a defect in the
instrument.** `.cargo/mutants.toml` excludes `verification::` and `proofs::` as module paths,
because a `#[cfg(kani)]` harness is never compiled by `cargo test` and so always "survives".
`timetable` puts its harnesses in `src/proofs.rs` rather than in an inline `mod proofs { .. }`, and
**cargo-mutants names a mutant by the function's path within the file it parsed**: the inline form
yields `proofs::a_fire_is_strictly_in_the_future`, the file-per-module form yields
`a_fire_is_strictly_in_the_future` with no prefix, and the regex misses it. So this crate's 73.6%
was a score with its own proof file counted against it, in the same family as `system_initializer`
(milestone 244) and `uefi_loader`'s `[[bin]]` half (milestone 280): arithmetic rather than a
finding. Closed with `**/src/proofs.rs` and `**/src/verification.rs` in `exclude_globs`, a glob
rather than a regex because the file layout is the thing that differs. `timetable` is the only
crate in the tree with that layout today; the sibling glob is there so the convention cannot arrive
unexcluded.

**The twelve real survivors, all in the parser, the admission check and the plan writer.**

- **`Error::line` replaced by a constant `1` (1).** `each_error_points_at_the_line_that_is_wrong`
  compares whole `Error` values, so the line is checked and the accessor never called. It is what
  sends a person to the fault. Closed by `error_line_reads_the_number_each_variant_carries`.
- **The plan writer's millisecond branch (3), never rendered by any test (1).** Every plan in the
  suite used seconds or minutes, so `nanos / (NANOS_PER_SEC / 1000)` could become `%`, and either
  `/` could become `*`, with nothing to see it.
- **The plan writer's length arithmetic (3).** `write_schedule` fills a 32-byte space-filled buffer
  and emits `buf[..n.max(SCHEDULE_COLUMN)]`, so for any schedule inside twelve columns the returned
  length does not reach the output at all: `6 + write_interval(..)` under `-`, and
  `n += unit.len()` under `-=` and under `*=`, were all invisible against `30s` and `1m`. A
  seven-digit interval pushes past the column, where the first underflows, the second truncates the
  text and the third pads it.
- **`e.mem_pages > 0` under `>=` and `e.arg != 0` under `==` (2).** Both were only ever exercised
  true, so nothing proved the lines are *omitted* for an entry that asks for neither. All eight of
  the above are closed by `the_plan_prints_milliseconds_long_intervals_and_the_absent_grants`.
- **`admit`'s `Holdings { dir: held.dir, .. }` with the field deleted (1)**, falling back to the
  default `false`. A scheduler that holds a directory would have reported every `rm` line as
  unbackable. Closed by `a_designation_is_backed_by_the_directory_the_scheduler_holds`.
- **Both `!` in `unbacked`'s `!held.dir` tests (2), and these are the ones with a finding under
  them.** They survived because **`admit` cannot reach either branch**: it hands
  `grant_plan::plan` the same `dir` bit `unbacked` then re-tests, so a plan needing a directory the
  scheduler lacks is refused during planning and never arrives at the check that would name it
  unbacked. `Unbacked::File` has a second reason, which is that no shipped program declares a
  `FileSpec::Required`, so `Endowment::file` is `None` for every plan this crate can build. The
  mutants are killed by calling `unbacked` directly, and the reachability is recorded in a `BUGS`
  section on `Unbacked` itself, where a reader meets the variants. **Whether `admit` should stop
  pre-consuming the holding is a behaviour change rather than a test**, so it is recorded and not
  made.

### `dtb`: 14 survivors, 13 killed, 1 equivalent; the 29 timeouts are hangs

**Before: 368 caught, 14 missed, 29 timeouts, 19 unviable (96.6% of viable). After: 381 caught, 1
missed (99.7%)**, which is the baseline's own number.

**Thirteen of the fourteen are one omission repeated across four walkers.** `Dtb` has five walks
over the structure block that each keep their own 16-entry per-depth array, and
`a_declaration_at_the_stacks_edge_is_ignored_not_indexed` (milestone 42's fuzzing leg) tests exactly
one of them, `node_reg`'s. `node_prop_compatible`, `node_prop_inherited`, `phandle_prop` and
`node_prop` carry the same `depth < MAX_DEPTH` guard and had `<=` alive in every one: at depth
exactly 16 that is an out-of-bounds index in a parser the kernel runs on firmware bytes, before
there is any way to report a failure. Three of them also had the guard alive under `==` and `>`,
which skip the per-node reset for every depth a real tree reaches, so a sibling answers with the
property of the node that closed before it. Closed by three tests in `tests/hostile.rs`:
`every_walkers_stack_edge_is_ignored_rather_than_indexed`,
`a_sibling_does_not_answer_with_its_predecessors_property`, and
`an_inherited_property_comes_from_the_named_node_not_the_first_one`, which also covers
`node_prop_inherited`'s target guard under `||`: `(A && B) || target_at.is_none()` selects the
**root**, whose slot then answers for a node the tree does not contain.

**And the sweep audited the tests, which is worth recording as a method note rather than as an
anecdote.** The first version of two of those tests passed and killed nothing, because
`node_prop_compatible` takes `(compat, name)` and they were written `(name, compat)`: a vacuous
assertion that reads correctly. The re-run is what said so, and it said so in the only way that is
not an argument. This is the ledger's own rule ("a verdict reached by reading is wrong about ten
percent of the time; a verdict reached by running is not") applied to the *kill* rather than to the
equivalence.

**The one equivalent.** `cells`'s `value = (value << 32) | be32(..)` under `^`. The shift clears the
low thirty-two bits and a `be32` cannot reach the high ones, so the two operands are disjoint and
`|` and `^` are the same function on them; the identical argument as `capability::note_peak` above.

**The 29 timeouts are hangs, and they are the baseline's `dtb` row grown with the walkers.** All 29
are `at += 4` under `-=`, `at += align4(..)` under `-=`, or `at = value_at + align4(len)` under `-`,
spread over the nine walks: every one is the structure-block cursor, and a cursor that stops
advancing re-reads the same token forever. That is the tests noticing rather than missing, which is
this file's standing reading of a timeout whose mutant could hang.

### `clock_protocol`: 6 survivors, 3 were the loom model, 3 are equivalent and none is a gap

**Before: 54 caught, 6 missed, 7 timeouts, 5 unviable (91.0% of viable). After: 54 caught, 3 missed
(95.3%).** **Nothing was killed here and no test was written, which is the correct outcome rather
than a shortfall**: three survivors left with the `interleavings::` exclusion above, and the other
three cannot differ from the original under `cargo test`. The rate moved because the measurement
stopped counting a file the suite never compiles, not because the suite got better.

- **`spin_hint` replaced by `()`.** Under `not(loom)` it is `core::hint::spin_loop()`, which is
  documented as a hint with no effect on program semantics, so removing it cannot change any
  observable behaviour of a host test. Under `--cfg loom` it is `yield_now()` and it is
  load-bearing, for liveness rather than correctness: loom's scheduler is cooperative and a spin
  that never yields starves the writer it waits on. That is `script/interleaving-check`'s property
  and not this suite's.
- **`publish`'s `compare_exchange_weak(s, s + 1, ..)` under `*`.** `s * 1` is `s`, so the claim
  leaves the sequence **even**: the odd marker that tells a reader a write is in flight is never
  set. Single-threaded that is invisible, because the publish completes before any read begins and
  the final sequence is `claimed + 2` either way; this is the "single-threaded blindness" pattern
  named at the top of this file, and it is recorded **equivalent-under-harness** rather than
  excluded, so it stays visible.
  **It is killed by the gate that owns it, and that was measured rather than assumed**: with the
  mutation applied, `script/interleaving-check -p clock_protocol` fails **four** of its harnesses
  (`a_reader_never_sees_half_a_publish`, `a_racing_reader_sees_an_unrecognised_page_or_a_whole_one`,
  `the_generation_a_reader_sees_matches_the_pair_it_read`,
  `two_writers_serialise_rather_than_corrupt_the_page`). A host test cannot carry a concurrency
  claim; the loom model can, and here it does.
- **`policy::decide`'s `proposed_nanos > current_nanos` under `>=`.** At equality the two branches
  compute the same thing: the `if` arm tests `0 > MAX_STEP_FORWARD_NANOS` and the `else` arm tests
  `0 > MAX_STEP_BACKWARD_NANOS`, both constants are non-zero, and both fall through to
  `status::ACCEPTED`. Equivalent for every input, not merely untested.

**The 7 timeouts are the seqlock's retry loops**, the same `dtb` cursor family one level up: `s & 1
!= 0` under `==`, `&` under `|` and `^`, and the reader's sequence comparison, each of which turns
a bounded retry into one that never exits. The baseline's `clock_protocol` row recorded 7 hangs of 9
survivors for the same reason.

### `swish`: 20 survivors, 16 killed, 4 equivalent

**Before: 157 caught, 20 missed, 11 timeouts, 12 unviable (89.4% of viable). After: 173 caught, 4
missed (97.9%).** Every survivor was in the shell's printers or its two smallest accessors, which
is what a crate that is mostly rendering should be expected to produce.

**One of them is a test that could not fail, and it is the one to read.** `write_found` right-aligns
a match count and starts every title in the same column, and
`a_search_answer_names_pages_a_reader_can_type` asserted
`cols == [APROPOS_TITLE, APROPOS_TITLE]`: the rendering compared against the constant it is derived
from, which is an identity. Six mutants lived in that one line, one for every `+` in
`const APROPOS_TITLE: usize = 2 + APROPOS_COUNT + 2 + 28 + 2`, because changing the definition moved
both sides of the assertion together. The fix is the literal 38, which is what a reader at eighty
columns actually gets. A seventh survivor was next to it: the digit-counting loop's `count / 10`
under `%`, invisible because every fixture used a two-digit count, so a new test uses a three-digit
one and pins the column under the widest row.

**The rest are the ordinary shape.** `write_batch` had no caller in the crate's own tests, so its
whole body could be `()`: a sweep would have shown a person no set while handing each batch a real
one, which is exactly the property the batching lane exists to make visible. `Status::from_code` had
no caller either, so it could be `Default::default()` with two arms deleted; it crosses an atomic
cell as a `u64`, so the round trip is the contract. `write_apropos`'s `offered() > results().len()`
under `>=` printed the truncation tail when nothing was truncated, which no test forbade.
`Sequence::is_plain` could be a constant `true`, and that one is worth naming: it is the
pre-connector shape test, so a `true` there routes `date && wc` down the single-command path and
drops everything after the first connector. Every fixture asked it only of a one-segment line.

**And one kill is honest about being a side effect.** `write_duration`'s
`(nanos % SEC) / MILLI` under `+` is killed by a new assertion that the printer is total on every
`u64`, which it earns on its own merits (the shell hands it `end - start` off a counter it does not
own, and a clock that went backwards produces a number nobody chose). It kills the mutant by
overflow rather than by disagreement, and the two mutants in the smaller units, where the addition
cannot overflow, are equivalent instead.

**The four equivalents.**

- **`write_duration`'s `(nanos % MILLI) / MICRO` and `nanos % MICRO` under `+` (2).** The fraction
  is printed digit by digit as `t / 100 % 10`, `t / 10 % 10`, `t % 10`, which is `t mod 1000`. Since
  `MILLI` and `MICRO` are exact multiples of the unit below, adding one adds exactly 1000 to the
  quotient, and `(t + 1000) mod 1000 == t mod 1000`. The mutants print the same three digits for
  every input that does not overflow, and in these two branches the input cannot.
- **`write_refusal`'s `Refusal::NoSuchProgram => {}` arm deleted (1).** That arm is reached only
  when the guard above it fails, which is exactly when `spec.prog` is empty. Deleting it sends that
  case to `_`, which prints a prefix only `if let Some(p) = Prog::from_name(spec.prog)`, and
  `Prog::from_name(b"")` falls to its `_ => None`. Same output, by two routes.
- **`Sequence::is_empty` under a constant `false` (1).** The function is documented "never true" and
  the invariant holds: `split` always produces at least one segment, so `self.n == 0` is `false` for
  every value this type can take. `-> true` and `==` under `!=` are killed by the assertions added
  beside `is_plain`; this third one cannot be, because it is what the function already does.

**The 11 timeouts are hangs**, the same loop-control family as `dtb` and `clock_protocol`: `echo`'s
word cursor under `-=` and `*=`, `write_found`'s digit loop under `>=`, `pad`'s `left -= take` under
`/=`, and `split`'s segment cursor under `*=`. Each stops the loop advancing.

### `filesystem_protocol`: 59 survivors, 19 killed, 38 equivalent, 2 recorded gaps

**Before: 529 caught, 59 missed, 10 timeouts, 47 unviable (90.1% of viable). After: 548 caught, 40
missed (93.3%).** This is the crate the baseline already found dominated by equivalents (36 of 46
under its old name `fs_proto`), and the census's 59 have the same shape: a contract crate is mostly
constants and packing, and most of the mutants a tool can make there cannot change a value.

**The nineteen real gaps, and four of them are witness bits that report nothing.**

- **`fixture::twodir` had no distinctness test at all (5).** Five of its six bits could each become
  zero, and two of them are the structural finding `notes/dir-capability.md` records (the endpoint
  is the boundary, witnessed with two live caretakers). A witness bit of zero is a probe that
  reports nothing while its boot passes. Closed by `the_two_grant_bits_are_distinct`, in the shape
  the escape, directory and navigation fixtures already use.
- **`fixture::navscape`'s list had gone stale again (3).** `BIND_REACHED_TARGET`,
  `BIND_ASCEND_REACHES_REAL_PARENT` and `BIND_STOPS_AT_TRUE_ROOT` (2026-08-30) were never added to
  `the_navigation_bits_are_distinct`, whose body already carries a comment about the *previous* six
  that were added late. That is rung four of AGENTS.md's ladder failing the way rung four does, and
  this time the mutation run is what noticed rather than a reader. The three are added; the
  mechanism that would stop it recurring is named in milestone 326's handoff rather than built here.
- **`fixture::throughput::name` had no caller (9).** The whole function as `None`, as `Some("")` and
  as `Some("xyzzy")`, plus each of its six arms deleted. The bench boot prints these names beside
  the numbers, so two phases sharing one is two rows a reader cannot tell apart. Closed by
  `every_throughput_phase_tag_has_its_own_name`, which also pins the count against `PHASES`.
- **`statfs::free_bytes` replaced by `0` (1).** Its only assertion was
  `free_bytes(4096, 0) == 0`, which the mutant satisfies. A volume with room would have reported
  itself full to everything that asks before it writes.
- **`statfs::encode`'s `out.len() < LEN` under `<=` (1).** The tests covered `LEN - 1` and 64 and
  never `LEN` itself, which is the one size a caller sizing its page from the constant would hand
  it.

**The thirty-eight equivalents, in four groups, so a later reader can re-check them by group rather
than one at a time.**

- **Packed wire words whose fields are bit-disjoint (8).** `blk::req`, `fs::req` (two), 
  `fs::rename_dst`, `grant::spec`, `nameset::encode`, `xattr::spec` and `xattr::reply` all build one
  word as `(field << shift) | (other & mask)`. Each shift clears exactly the bits the mask keeps, so
  `|` and `^` are the same function on those operands. The disjointness is not assumed: it is what
  `a_handle_never_collides_with_the_opcode_or_length`, `a_granted_name_survives_the_two_argument_words`
  and `a_rename_carries_two_directories_and_two_lengths_without_them_bleeding` already prove, and
  `nameset`'s type bit is `1 << 7` against a length the encoder refuses above `grant::MAX_NAME`.
- **Unions of distinct single-bit rights (14).** `dir::ALL`, `dir::REMOVE_TREE`, `Verb::mutates`'s
  mutating mask, and four `needs_any`/`needs_all` rows in `verb::TABLE`. Every operand is a distinct
  `1 << n`, so `^` is `|`; and that the bits are distinct is itself pinned, by
  `undefined_rights_bits_cannot_be_smuggled_into_a_root` and by every non-degenerate `1 << n` in
  `dir` being a caught mutant.
- **The attrs witness union (7).** `fixture::attrs::EXPECTED` is the eight distinct bits of its own
  module ored together, and `the_attribute_bits_are_distinct` is what makes the disjointness a fact.
- **`1 << 0` under `>>` (9).** `dir::ENUMERATE`, `dirent::IS_DIR`, `grant::READ`, and the first bit
  of six fixture witness sets. Both sides are 1. The degenerate case this file's patterns section
  already names, recorded rather than excluded so it stays visible if a constant ever moves off zero.

**And two recorded gaps, which are the same object as the Kani harnesses one paragraph down.**
`verb`'s second `const _: () = { .. }` walks `TABLE` asserting each row sits at its own opcode, and
its loop bound `i < TABLE.len()` survives under `==` and under `>`: both make the loop body never
run, so the block compiles and checks nothing. **No `cargo test` can see this**, because the checker
is `rustc` and the evidence of success is that the build happened. The comment above it already says
a runtime test cannot do this job (the three caretakers are `no_std` binaries). It is recorded here
rather than excluded because the exclusion would have to be by line and would hide anything else
that lands there.

**The 10 timeouts are hangs**, all of them iterator cursors: four `Iterator::next` implementations
replaced by `Some(Default::default())`, which never consumes its buffer, and six `at += ..` under
`*=` in the record walkers and the name packers. A walk that stops advancing loops forever, which is
the tests noticing.

### `machine_discovery`: 77 survivors, 58 killed, 11 equivalent, 8 recorded gaps

**Before: 536 caught, 77 missed, 9 timeouts, 71 unviable (86.2% of viable). After: 594 caught, 19
missed (95.5%).** The before column reproduces the 2026-09-19 census row for row, which is worth
saying because this crate is the one that census blamed for the corpus falling. Every kill below was
verified by re-running the sweep; every equivalence claim is a mutant the final run still reports.

**One sentence explains all 77, and it is not a truncation bug**: every fixture in this crate is a
table or a device tree that is **well formed**, so a guard was only ever asked about inputs with
room to spare on both sides of it. The crate has an `AcpiError::Truncated` variant, prose about
tables "shorter than it is", and a `MAX_CPU_NODES` the record can overflow, and nothing ever handed
a parser an input at any of those edges.

#### The one-property hypothesis, measured and refused

This crate was taken on the reading that **43 of the 77 were one defect wearing 43 hats**, the
short-buffer guards at the top of every parser, and that a single prefix property would kill most of
them and take the crate to roughly 96%. That was worth testing rather than assuming, so it was
tested on its own: the prefix property was written first, alone, and the sweep re-run before
anything else was touched.

**It killed 12, and took the crate from 86.2% to 88.1%.** Re-derived, 41 of the 77 *are* bounds,
length or offset shapes, so the reading of the survivor list was close. But only 12 of those are
guards a prefix of a valid input can reach, and the other 29 want inputs a prefix cannot produce:

- **The entry-walk guards want a table that is well formed and ends exactly at the buffer**, not a
  short one. `at + 2 > body.len()` under `>=` is invisible unless an entry fills the body to its
  last byte, and `len >= 8` under `true` needs an entry of a *decoded type* that is too short for
  that type's own fields, inside a list that is otherwise fine.
- **`McfgEntry::size`, `mcfg_entry` and `root_entry` take an index**, so no length exists to
  truncate.
- **The `cpu_list` and `plic` guards want a malformed device tree**, which is a fixture rather than
  a slice.

**And the shape the property is written in decides half the result, which is the part worth
carrying to the next crate.** "Feed every prefix and assert it returns an error" kills `<` under
`==` and under `>`, because those read past the end and panic. **It does not kill `<` under `<=`**:
at every length such a loop feeds, the original refuses too. The mutant that survives a refusal test
is the one that refuses a structure which is *exactly long enough*, and the only input that
distinguishes it is the boundary length. So the property has to be two-sided, every short prefix
refused **and** the shortest sufficient one accepted; six of the twelve died only to the second
half. `parse_sdt_header` needed an empty-bodied table for the same reason, so its own `length` field
is exactly `SDT_HEADER_LEN`.

#### The MADT and DMAR entry walks (16)

These two iterators are the same shape twice: `[type, length, ..]` entries, self-describing, walked
until the body runs out. Both are read from firmware bytes on the x86 boot path and nothing else
checks the lengths they trust.

`at + 2 > body.len()` was alive under `>=` and `len < 2` under `<=`, both killed by a body of
exactly `MADT_FIXED_LEN + 2` holding the shortest entry the format allows. The DMAR's pair needed
**two** four-byte structures rather than one, which is what tells a cursor that advanced wrongly
(`at += len` under `*=`) from a walk that stopped early (`at + len > body.len()` under `<`): with a
single entry both mutants and the original all end the walk in the same place.

**The four `len >= N` match guards were alive under `true`**, which decodes a processor entry that
has no flags word and reads the bytes after it. Closed by
`an_entry_too_short_for_its_own_kind_is_not_decoded`, four bytes of each decoded type, each reported
by its type code instead. Type 5 was alive under `false` and under `<` as well, because
**`LocalApicAddressOverride` had no test at all**: a machine whose local APIC sits above 4 GiB says
so in that entry and nowhere else, since the fixed part's field is 32 bits, so a decoder reporting
it as an unrecognised type would use the low address and touch memory that is not the APIC.

#### The MCFG (5)

`McfgEntry::size` was alive under `<=` and `==` on its ends-before-it-begins guard, because no test
ever asked about a window of **one** bus, where equal bus numbers are a mebibyte rather than
nothing; and alive under `+` for `-`, because the only window tested starts at bus 0, where a sum
and a difference agree.

Two offset mutants sat beside them (`segment: u16(body, at + 8)` under `-`, and `u16`'s own
`bytes[at + 1]` under `-`) and they are the more interesting pair: **the single window this crate
tested is zero in every field a wrong offset would reach**, so the segment could be read out of the
base's low half and still answer 0. Closed by a second window with distinct values in every field.

#### The device trees (12)

Four shapes no dump can produce, so three fixtures are new (`many-harts`, `plic-shapes`,
`partial-psci`) and `interrupt-shapes` grew a node. Names provisional.

- **`#address-cells` two bytes wide, and a `reg` short of its declared width at both widths.** The
  CPU bindings require the property, so two bytes of it is a malformed tree and there is no correct
  reading; what there is, is a decoder that must not read the two bytes after it. All three guards
  were alive under `true`.
- **Eighteen cores.** `CpuList::truncated` was alive as a constant `false` and under `<` for `>`,
  and the reason is that **no fixture in the tree overflows sixteen slots**, so a predicate no
  caller could rely on was passing a test that asserted it on a seven-core machine.
- **A `/psci` node stating only `method`.** `method.is_none() && cpu_on.is_none() &&
  compatible.is_none()` was alive under `||` at both positions, and `can_start_a_core` as a constant
  `true` and under `||`. All four are the gap between "no PSCI node" (which `no-psci.dtb` covers)
  and "a complete one" (every other fixture): a machine whose firmware published a conduit and no
  function id is one where starting a core is impossible for a reason a boot line can name, and a
  decoder demanding all three properties would report it as having no PSCI at all.
- **The context map's two walks falling out of step.** `plic-shapes` carries three: the PLIC named
  like its harts' own controllers **and declared before `/cpus`** (both JH7110 fixtures declare
  theirs after, so a walk that failed to filter it by `compatible` still got the right answer there);
  a hart controller with no `phandle`, which still consumes its hart's slot; and a hart whose id is
  `MAX_CONTEXT_HARTS` exactly, which the tree names an S context for and the record has no slot for.
  The last one is a write past a sixteen-element array under `<=`.
- **PPI 16.** `interrupt-shapes` had `badppi@` at 99, which any reading of the bound turns away. 16
  is the first number that is not a PPI, and folded into the bank it becomes INTID 32, which is SPI
  0: a line a different device owns.

#### The rest (10)

`Conduit::name`, `EnableMethod::name` and `MemoryKind::name` were each alive under `""` and
`"xyzzy"`: three enums whose only consumer is the boot print, and nothing asserted a word. A
revision-2 RSDP with a **null XSDT**, which is the only input where the second half of
`root_table`'s condition is asked, and where following the zero sends the kernel to physical address
0. `parse_hex` given `0x` with no digits, and `parse_decimal` given the character after `'9'` read
as a tenth digit. And blue, which is the only one of the three colours that travels **up** the word,
so red alone could not tell a sixteen-bit shift from either direction.

#### The eleven equivalents, by group

- **`1 << 0` under `>>` (2).** `MADT_PCAT_COMPAT` and `SBI_TIME`. Both sides are 1; the degenerate
  case this file's patterns section already names, recorded rather than excluded so it stays visible
  if either constant moves off zero.
- **Disjoint operands under `^` for `|` (5).** `PixelOrder::store`'s two (bits 8..16, 16..24 and
  0..8, built from masks that cannot overlap), `parse_hex`'s `(value << 4) | digit` where the shift
  clears exactly the four bits a hex digit occupies, `eid`'s `(acc << 8) | b[i]` for the same reason
  a byte cannot reach above bit 7, and `pmu_event`'s `(type << 16) | code`. The last is the weakest
  of the five and worth naming as such: the disjointness there is **documented rather than
  enforced**, `event_idx[19:16] = type` and `[15:0] = code` with no mask in the code, so it holds for
  every input the encoding admits and not for every `usize`. What makes it checkable is that the
  existing `pmu_event(1, 0xffff)` assertion already exercises the widest code the field has room for.
- **An assignment that is a no-op at equality (2).** `Isa::from_device_tree` narrows twice, `base <
  out.base` and `t < out.mmu`, each guarding `out.base = base` and `out.mmu = t`. Under `<=` the
  extra branch fires only where the two compare equal, and the assignment then writes the value that
  is already there. **What makes that "the same value" rather than "an equal one" is that neither
  ordering is coarser than its equality**: `MmuType` derives `Ord` over its variants, and `Base`'s
  hand-written `cmp` maps its four variants to four distinct widths (0, 32, 64, 128), so `Equal`
  means the same variant in both. An ordering that put two variants at one rank would break this
  argument rather than the code.
- **A bound the other ceiling makes unreachable (1).** `plic`'s `n < MAX_CONTEXT_HARTS` under `<=`.
  The conjunct after it is `cpus.cpus().get(n)`, and that slice is at most `MAX_CPU_NODES` long;
  both constants are sixteen, so `get(16)` is `None` for every tree and the mutant's extra iteration
  never reaches the array it would index. **This one is worth re-checking if either constant ever
  moves**, because the equivalence is a relationship between two numbers rather than a property of
  the line.
- **A guard that is vacuous on the host (1).** `Framebuffer::span`'s `bytes <= usize::MAX as u64`
  under `true`. On a 64-bit host `usize::MAX as u64` is `u64::MAX`, so the guard is already always
  true and no host test can distinguish them. It is not dead code: it is what makes the span honest
  on a 32-bit target, which this crate is built for.

#### The eight recorded gaps, which are two instrument holes rather than two omissions

**Six are compile-time `const _: () = { .. }` blocks**, `IMPLEMENTERS`' duplicate-code check in
`aarch64.rs` and `IMPLEMENTATIONS`' sequential-id check in `riscv64.rs`. Their loop bounds survive
under `==`, `>` and `<=`, each of which makes the loop body never run, so the block compiles and
checks nothing. **No `cargo test` can see this, because the checker is `rustc`** and the evidence of
success is that the build happened. Exactly the same object as `filesystem_protocol`'s `verb::TABLE`
walk recorded above, and recorded here for that entry's reason: an exclusion would have to be by
line and would hide anything else that lands there.

**Two are a `const` inside a `#[cfg(kani)] mod verification` block**, `x86_64.rs`'s
`const N: usize = V1_LEN + 1;` under `-` and `*`. `cargo test` never compiles that module, so a
mutant there always survives, and `.cargo/mutants.toml`'s `verification::` regex cannot reach it:
cargo-mutants names a mutant by its path within the parsed file, and **a `const` gets no module path
in its name at all**, so there is nothing for a module-path regex to match. This is the third member
of a family this file already carries twice, after `timetable`'s `proofs.rs` (a module that is its
own file) and the two globs added beside it. The concurrent census-delta lane of the same day measured
the tree-wide extent (three mutants, of which these are two) and filed a proposal for the general
fix, whose number is minted at merge; no test is owed here, because it is code no host build
compiles.

**The 9 timeouts are hangs and are unchanged by any of this.** Four are `+=` under `*=` on a
walker's cursor (`Isa::implementer_name`, `Sbi::impl_name`, `lookup`), and five are the MADT and
DMAR entry walks' `len < 2` and `len < 4` under `==` and `>` and their `||` under `&&`: a length of
zero that the walk accepts never advances. That is the tests noticing rather than missing, which is
this file's standing reading of a timeout whose mutant can hang.

## 2026-09-20: milestone 326 part 3, the new-crate backlog

Part 3 is the rest of the 1.9-point gap between the like-for-like 93.6% and the corpus 91.7%: the
26 crates that did not exist at the August baseline, of which the seven worst in the tree are all
members. The block says to take it as a worklist and not to try to close it, so this section
accounts for those seven and stops there.

Same discipline as the 2026-09-19 section above, and it is the discipline rather than the number
that makes the section worth reading: every crate was re-derived with `script/mutation -p <crate>`
on this lane's own worktree rather than read out of the census, **every kill was verified by
re-running the sweep and watching the mutant die**, and every equivalence claim below is a mutant
the second run still reports.

| crate | before | after | killed | equivalent | excluded | recorded gap |
|---|---|---|---|---|---|---|
| `work_steal_slot` | 0 | 0 | 0 | 0 | 0 | 0 |
| `memory_corruption_canary_gate` | 8 | 2 | 6 | 2 | 1 | 0 |
| `soak_page` | 7 | 0 | 7 | 0 | 0 | 0 |
| `jh7110_entropy` | 23 | 10 | 13 | 6 | 0 | 4 |
| `multicast_dns_protocol` | n/a | n/a | n/a | n/a | n/a | n/a |
| `job_mix` | 10 | 4 | 6 | 3 | 0 | 1 |
| `schedule_store` | 8 | 0 | 8 | 0 | 0 | 0 |
| **total** | **56** | **16** | **40** | **11** | **1** | **5** |

`before` and `after` count **missed plus timeouts**, which is what `script/mutation --report` lists
as "the survivors themselves". The one exclusion is a category rather than a crate's business and is
counted where it was found. Four of the six `jh7110_entropy` equivalents are one line.

**Two of the census's seven numbers were artifacts of the instrument, and the block predicted one of
them.** `work_steal_slot` read 54.2% because its loom model was counted; the `interleavings::` entry
part 1 added removes it, and the crate now reports **14 mutants, 13 caught, 1 unviable, no
survivors**, with nothing owed. `multicast_dns_protocol` (the census's `mdns_proto`, renamed by
milestone 265 (`_proto` is a truncation)) read 77.3% with 82 survivors and **is not in this tree**:
milestone 298 (retire the multicast DNS responder and its two crates) retired both of them on
2026-09-15, six weeks after the baseline and a day
after the census. Its row closes by deletion. `memory_corruption_canary_gate`, the other crate the
block flagged as suspect, was genuinely 50.0% once the loom mutants were out, which is worse than
the 66.7% it was flagged at rather than better.

### The pattern the whole part turned out to be about: a loop that waits

**Eight of the fifty-six survivors were not wrong answers, they were deadlocks**, and they are the
reason this section is ordered the way it is rather than worst-first. A spin loop is broken by
making it never accept, and a function that gathers is broken by making it never advance; neither
produces a wrong return value, because neither returns. The suite's answer to that is to hang, and
`cargo mutants` can only report a suite that did not finish as a timeout, which it cannot
distinguish from a slow one.

This file's standing reading is that such a timeout is the tests noticing rather than missing, and
that stays true. What part 3 adds is that **noticing by hanging is worth converting into noticing by
failing, where the crate lets you**, because the two are very different for the human who runs
`cargo test` and gets no output at all.

- **`memory_corruption_canary_gate`: converted, all four.** Every host test body now runs on a
  worker with a five-second deadline, so `arm` and `disarm` failing to accept is a stated assertion
  (`a spin loop never made progress, so the gate deadlocked`) instead of a hung binary. A panic
  inside the body drops the sender, so a real assertion failure and a hang are told apart rather
  than both reading as a hang. Liveness is a property this crate owes the kernel more than the host:
  `arm` spins on a core its owner cannot be preempted from.
- **`job_mix`: removed, by deleting the loop.** Its one timeout was `jobs_of_kind`'s hand-rolled
  index under `*=`, which never advances. A `for &k in &MIX` has no increment to lose, so the mutant
  does not exist rather than being caught: rung one of AGENTS.md's ladder where the old code sat at
  rung zero.
- **`jh7110_entropy`: not converted, and the reason is measured.** `Pool::take`'s four are the same
  shape, and `take_returns_rather_than_spinning` now states the property on a deadline. The
  classification does not move, because **`Pool`'s own doctest calls `take` directly**, `cargo test`
  runs doctests, and there is nowhere in a doctest to put a deadline. Confirmed by hand-applying the
  `self.cursor != self.filled` mutant and running `cargo test -p jh7110_entropy --doc`, which sat at
  `has been running for over 60 seconds`. Recorded as four gaps rather than argued away; what would
  close them is a way to tell a deadlock-timeout from a slow-test one, which cargo-mutants 27.1.0
  does not offer (its whole set of limits is the clock, per milestone 277's own check).

### The exclusion, which is measured and not assumed

`.cargo/mutants.toml` grows a third `exclude_re` entry, `tests::`, and it belongs with the two part 1
added rather than beside them by coincidence.

cargo-mutants already declines to mutate anything under a plain `#[cfg(test)]`. It does not
recognise the compound form, and the five loom crates have to write `#[cfg(all(test, not(loom)))]`
because the loom model is the other half of the same `test` cfg. So a helper function in one of
those modules is mutated where the identical helper in an ordinary crate is not, which is how
`memory_corruption_canary_gate`'s new deadline helper came back MISSED the moment it was written.

**Measured, in the form the seven-questions rule asks for**: `cargo mutants -p calendar --list`
returns 395 mutants and none of them is `tests::*`, although `calendar`'s test module has a
`fields` helper of exactly the shape that was mutated here.

A mutant of a test helper is never a defect in the shipped system, which makes this a **stronger**
exclusion than the two above it rather than a weaker one: `verification::` and `interleavings::`
at least name real code with a checker of its own, and this names code that is compiled into
nothing.

### `work_steal_slot`: no survivors, and the census's 54.2% was the loom model

**Before and after: 14 mutants, 13 caught, 1 unviable.** Nothing owed and nothing done. The block
listed this crate first and called its number suspect; the number was the `mod interleavings` block,
which part 1's exclusion removes, and what is left is a crate whose tests kill everything.

### `memory_corruption_canary_gate`: 8 survivors, 6 killed, 2 equivalent

**Before: 8 caught, 4 missed, 4 timeouts, 2 unviable (50.0% of viable). After: 14 caught, 2 missed,
2 unviable (87.5%).** The census read 66.7%; the difference is the loom mutants leaving the count.

**The four timeouts are the deadline conversion above.** They are `arm`'s two acceptance tests
(`seen == DISARMED` under `!=`, and the `||` under `&&`), `disarm`'s `seen == ARMED` under `!=`, and
deleting `ArmGuard`'s `Drop`, which strands the gate in `ARMING` so the next `disarm` never returns.

**The two real misses were both the re-arm transition, and nothing in the suite armed an already
armed gate.** Every other path reaches `arm` from `DISARMED`, and `DISARMED` is the one state where
an `arm` that returns its guard *without* winning the compare-exchange is invisible: the gate is not
`ARMED` either way, so `armed_hint` and `try_check` answer the same. From `ARMED` the same mistake
leaves the old plan readable and admits a check pass **while the plan is being rewritten**, which is
the torn plan the module documentation is about and the bug this crate exists to have fixed. Closed
by `rearming_an_armed_gate_takes_it_out_of_armed`, which kills `&&`-to-`||` outright and takes
`seen == ARMED` under `!=` into the deadline.

**The two equivalents are `pause`, and they are two functions rather than one.**

- **`#[cfg(not(loom))] fn pause` under `()` (1).** The body is `core::hint::spin_loop()`, a
  scheduling hint with no semantic effect: the loop still spins, and the protocol cannot observe
  whether the hint was issued. Equivalent by construction, which is exactly why the same function
  can be written twice under two cfgs.
- **`#[cfg(loom)] fn pause` under `()` (1).** The loom twin is `loom::thread::yield_now()`, and
  removing it is **not** equivalent under loom, where the yield is what tells the model checker the
  loop is waiting. It is unreachable rather than equivalent: `cargo test` never compiles it. The
  same object as the `interleavings::` module one level down, and it cannot be excluded the same
  way, because a free function's mutant name carries no module path and the two `pause` mutants are
  therefore indistinguishable by regex. `script/interleaving-check` is its checker.

### `soak_page`: 7 survivors, 7 killed, none left

**Before: 15 caught, 7 missed (68.2%). After: 22 caught, 0 missed (100.0%).** The whole crate is
four `const fn`s and a transform, so this one is short and the finding is sharp.

**Five of the seven were offsets that give two workers the same eight bytes**: `rounds` collapsed to
a constant, striding by one instead of eight, dividing instead of multiplying, and `mismatches` and
`wakes` counting backwards through their arrays. The two existing tests check that each offset fits
in the page and that the **ends** of each array clear the **start** of the next, and every one of
those five mutants passes both. The page's entire lock-free argument is that each `u64` has exactly
one writer; a collision is not a slow workload, it is a silent one, and the module documentation
says so two paragraphs before the functions that were wrong.

Closed by `every_slot_is_its_own_aligned_word`, which asserts distinctness and 8-alignment across
all 192 slots. **Distinctness and not values**, deliberately: the kernel and the workload both reach
the page through these functions, so any injective, aligned, in-page assignment is a correct one,
and pinning the arithmetic would test the code against itself.

**The seventh was `answer`'s `^` becoming `|`**, and it is the more interesting one. The three
existing assertions (neighbouring sequence numbers differ, no answer is zero, no answer echoes its
input) all pass under `|`, because a mask still varies and still never produces zero. What it stops
being is a **bijection**, and injectivity is the whole reason the transform exists: "a reply
carrying *some* value is not mistaken for a reply carrying the *right* value". Closed by
`the_answers_share_no_bit_in_common`, which intersects sixty-four answers: a mask leaves its own
bits standing in every output, a xor clears a bit as often as it sets one. A total tell rather than
a sampled one, and cheaper than hunting a collision.

### `schedule_store`: 8 survivors, 8 killed, none left

**Before: 29 caught, 8 missed, 5 unviable (78.4%). After: 37 caught, 0 missed (100.0%).**

**Six were a `>` that could become `>=` with nothing noticing, and they are one blind spot rather
than six.** Every refusal test in this crate hands the code a value **one past** the limit: a name
of `MAX_IDENTITY_LEN + 1` bytes, `MAX_IDENTITIES + 1` identities, a three-byte buffer for a
five-byte name. A value one past the limit is refused either way. Nothing anywhere asked what
happens **at** the limit, so a store that quietly lost the 64th byte of a name, the eighth identity,
or the last byte a buffer had room for would have passed the whole suite. Closed by
`the_last_thing_that_fits_still_fits`, through both the parse and the render half of each bound.

**The buffer case is the one that is not merely a lost feature.** The bound is
`n + name.len() + 1 > buf.len()`, where the `+ 1` is the newline that has not been written yet, and
two mutants (`-` and `*` in place of that `+`) drop the term. The check then passes on a buffer with
room for the name but not its terminator, and the next line indexes one past the end of a caller's
buffer. In a `no_std` crate the kernel links, that is a panic rather than a wrong answer.
`the_buffer_bound_counts_the_newline_it_has_not_written_yet` holds both sides: a buffer of exactly
the document's length renders, one byte less is refused.

**The other two were `Error::line`, which no test called at all.** The refusal tests compare whole
variants (`Err(Error::NameTooLong(1))`), so the accessor a caller actually reads the line through
was never exercised, and a `line` returning a constant was invisible. A configuration error pointing
at the wrong line is precisely what this crate's 1-based convention exists to prevent, and it
inherits that convention from `timetable::Error` with the reason attached ("a configuration error
nobody can find in the file is one nobody will fix"). `each_error_carries_the_line_it_is_about`
reads it on line 3 and line 9, neither of which is a constant a mutant would reach for.

### `job_mix`: 10 survivors, 6 killed, 3 equivalent, 1 recorded gap

**Before: 41 caught, 9 missed, 1 timeout, 3 unviable (80.4% of viable). After: 38 caught, 4 missed,
7 unviable (90.5%).** The caught count falls because four of the survivors became **unviable**, and
that is the result rather than an accounting artifact: they no longer compile.

**Five of the ten sat in a gap a compile-time assertion had left open**, which is the finding here.
`TASK_BUDGET_PAGES` is `1 + 8 + max(MAP_REGION_PAGES, CHILD_PAGES)`, and the file already carried
`const _: () = assert!(TASK_BUDGET_PAGES > MAP_REGION_PAGES && TASK_BUDGET_PAGES > CHILD_PAGES);`
with a comment citing AGENTS.md's ladder. The assertion is true of 19, of 24 and of 144, so every
one of the five mutants compiled and passed: `1 * 8` for `1 + 8`, `9 * 16` for `9 + 16`, and three
ways of picking the **smaller** region, which leaves a task running `MAP` nine pages short of what
the paragraph above the constant promises it.

**The rung was right and the assertion was too weak**, which is worth separating because the
temptation is to read this as an argument for a test. It is not: the two bounds the sizing argument
actually makes are still relations between constants in one file. They are now written as such, at
least `1 + 8 + MAP_REGION_PAGES`, at least `1 + 8 + CHILD_PAGES`, and strictly less than
`1 + 8 + MAP_REGION_PAGES + CHILD_PAGES`, the last because a task runs one job at a time and gives
its region back. Four mutants stopped compiling.

**Milestone 250 (an unviable mutant is a hole in the measurement that reads as a pass) applies
here and is named rather than dodged**: so moving four mutants from MISSED to unviable improves the rate partly by
shrinking the denominator. What makes it the right move here is that the thing which now refuses
them is the compiler, which is a stronger checker than a test and runs on every build.

**`order` keeps three survivors and gains one test**, and the test is about the draw rather than the
output. The function takes the LCG's **high** bits because the low ones cycle short; taking the low
ones makes every draw a multiple of 2^33, which is zero modulo every power-of-two swap index, so the
last position receives the mix's first job on every seed. Nothing else in the suite noticed:
`an_order_is_a_permutation_of_the_mix` still holds, and
`distinct_seeds_do_not_all_walk_the_mix_in_lockstep` still counts 90-odd differing orders.
`no_position_in_the_order_is_pinned_to_one_job` sweeps 512 seeds and asks that every job kind reach
every position.

**The three that remain.**

- **`while i > 1` under `>= 1` (equivalent).** The extra pass has `i` at 0, so `j` is
  `x % 1`, which is 0, and the swap is `out.swap(0, 0)`. One more LCG step on a value nothing reads
  afterwards, and a self-swap.
- **`MAP_REGION_PAGES > CHILD_PAGES` under `>=` (equivalent).** `if a > b { a } else { b }` and
  `if a >= b { a } else { b }` differ only where `a == b`, and there both branches yield the same
  value. Equivalent for every pair of operands, not merely for 16 and 10.
- **`% (i + 1)` under `% i` (equivalent in contract, and this one is an argument rather than a
  proof).** The mutant is Sattolo's algorithm: it still produces a permutation, still
  deterministically from the seed, and still puts no two tasks in lockstep, which is the whole of
  what `order` is documented to promise. What it loses is uniformity over all `16!` permutations,
  drawing from the `15!` cyclic ones instead, and no task in this benchmark can tell. **Re-check it
  if `order` is ever asked for a uniform shuffle**, at which point it is a defect and not an
  equivalence.

**The recorded gap is a finding about the code, not about the tests**, and it is in a `BUGS` section
on `order` where a reader meets the function. `seed | 1` exists to keep the LCG off its degenerate
state, and it does that by discarding bit zero, so **`order(2k)` and `order(2k + 1)` are the same
permutation for every `k`**. Measured, by asserting it for eight pairs. Nothing in the tree hits it,
because `kernel/src/job_mix.rs` hands every task "a distinct, odd, well-spread seed" and an odd seed
passes through `| 1` unchanged; a caller seeding tasks by consecutive index would put every adjacent
pair in lockstep, which is the one property this function exists to prevent. It is recorded rather
than fixed because changing the arithmetic changes every permutation this function has ever
produced, and a published benchmark number is a fact that has left the machine.

### `jh7110_entropy`: 23 survivors, 13 killed, 6 equivalent, 4 recorded gaps

**Before: 59 caught, 19 missed, 4 timeouts, 3 unviable (72.0% of viable). After: 72 caught, 6
missed, 4 timeouts (87.8%).**

**Fifteen of the nineteen were a `<<` becoming a `>>` in a register bit constant**, which takes a
named bit to **zero**. At that point `stat & STAT_SEEDED` is false for every status word the device
will ever present, and every existing test still passes, because the tests sample `interpret`'s
verdicts and never name the bit they are exercising. Thirteen died to one new test.

The test is a transcription check and says so in its own doc comment. These constants came off the
TRM's register tables by hand, one line at a time, so the failure to guard against is not a clever
one: it is a bit copied to the wrong line, which reads as a device permanently in mission mode, or
never seeded, or reporting a lockup the silicon never raised. **The distinctness half is the part a
reader should weigh more**, because two constants twenty lines apart, both `1 << 3`, look correct
individually and there is no other check in the tree that would catch it.

**The six that remain are all one arithmetic fact each, and all six are equivalent.**

- **`IE_RAND_RDY_EN` and `ISTAT_RAND_RDY`, both `1 << 0`, under `>>` (2).** `1 >> 0` is `1`. The
  degenerate shift this file's patterns section already names, in the one crate where two constants
  happen to sit at bit zero.
- **`ISTAT_ALL`'s four `|` under `^` (4).** The five bits it unions are bits 0 through 4 of `ISTAT`
  and no two are the same bit, so on disjoint operands `|` and `^` are the same function. The
  argument is the same one `capability::note_peak` carries above, and because an argument a reader
  has to take on trust is worse than one they can run,
  `the_clear_mask_is_every_named_istat_bit_and_nothing_else` now asserts the disjointness the claim
  rests on: five bits set, each named constant present.

**The four timeouts are `Pool::take`'s gather loop and stay recorded gaps**, for the doctest reason
given at the top of this section. They are a `refill` that reports success without filling, `got < n`
widened to `got <= n`, the spent-buffer test inverted, and `got += run` turned into a multiply; each
leaves a pass of the loop that makes no progress, and none of them returns at all.
`take_returns_rather_than_spinning` states the property on a five-second deadline, which is worth
having for the human who runs `cargo test`, and does not move the classification.

## Pending: one survivor nobody has triaged, found by a lane that could not file it

**`compositor`: `replace * with + in Rect::area`.** Milestone 517 (what fraction of survivor growth
arrives on lines a pull request touched) ran the mutation over the tree as it stood on 2026-08-03 and
found this mutant in that run's `caught.txt` and in the 2026-09-19 census's survivors. **It is the
single genuine decay on a line nobody edited** among 142 candidates: the other 141 were already
survivors in August.

It is recorded here rather than fixed because the lane that found it held neither this file nor
`.cargo/mutants.toml` at the time, both of which were owned by milestone 326's triage lanes. **What
would close it**: a test that distinguishes `w * h` from `w + h`, which needs a rectangle whose
width and height are neither equal nor {0, 2}, since `2 * 2 == 2 + 2` and `0 * n == 0 + n` only when
`n` is 0. Most fixture rectangles in that crate are squares, which is the likely reason it was never
caught.

## 2026-09-20: `board_console`, milestone 326 (turn a mutation score upward) part 3

`board_console` is one of the new-crate backlog's worst rates, at **238 viable, 43 survivors, 79.4%
caught** in the 2026-09-19 census. Untaken until now. Same discipline as the sections above: every
number below is re-derived with `script/mutation -p board_console` on this lane's own worktree
rather than trusted from the census, every kill was verified by re-running the sweep and watching
the mutant die, and both equivalence claims are mutants the second run still reports.

**Before: 242 caught, 43 missed, 6 timeouts, 31 unviable (83.2% of viable, 85.2% counting a timeout
as a kill). After: 283 caught, 4 missed, 6 timeouts, 31 unviable (96.6%, 98.6% counting timeouts).**
The census's own count differs from this run's (238 viable against this run's 291) for the reason
its own `BUGS` section gives: the tool version and the tree both move between a census and a lane
taking it. This run's `before` column is what this branch measured before anything changed, which is
the honest comparison point for the `after` beside it.

| module | survivors | killed by a test | equivalent | recorded gap |
|---|---|---|---|---|
| `board.rs` | 3 | 3 | 0 | 0 |
| `lottery.rs` | 8 | 8 | 0 | 0 |
| `port.rs` | 12 | 9 | 0 | 3 |
| `progress.rs` | 9 | 8 | 1 | 0 |
| `screen.rs` | 2 | 2 | 0 | 0 |
| `stop.rs` | 2 | 2 | 0 | 0 |
| `watch.rs` | 7 | 7 | 0 | 0 |
| **total** | **43** | **39** | **1** | **3** |

The 6 timeouts (all `screen::parse_pixmap`'s header-token loop, `+=` becoming `*=` or `-=`, or a
comparison flipped so the whitespace-and-comment skip never reaches a byte that ends it) are not in
the 43: this file's standing reading applies unchanged, they are the tests noticing by hanging
rather than by failing, and `cargo-mutants`' own 20-second per-mutant bound is the deadline. Nothing
here converts them, because nothing in this crate's suite calls the parser through a doctest or any
other path a deadline-wrapped test could not also reach; nothing here needed the conversion either.

**`board.rs`, 3 killed.** `Sign::seen_in`'s `None if complete => rest` arm (a complete line with no
trailing whitespace after a `WordAfter` prefix, e.g. `U-Boot 2021.10` with nothing following it) had
no test with a *complete* line short enough to hit it; the existing test only fed that shape as a
partial line. `Rung`'s `PartialEq` was only ever exercised through `<`, which goes through
`partial_cmp`/`Ord` rather than `eq`, so `assert_ne!` on two different depths was the one assertion
missing. `Profile::keys` was only ever checked for *emptiness* (`XENON.keys().count() == 0`), which
an always-empty iterator also satisfies; checking `RADON.keys()`'s actual contents closes it.

**`lottery.rs`, 8 killed.** Two were `parse_core_line` itself, never called directly by any existing
test (only through `tally`, whose fixtures happen not to probe the exact byte offset or a malformed
token). The `+`-to-`-` mutant shifts the slice nine bytes early; every fixture in the suite has
enough real content shifted into view that the guard filters it back out, so the miss needed a
token planted at exactly the wrong distance to catch (`core=0 G9 threads=1 R2`, engineered so the
shifted read lands inside `G9`). The `||`-to-`&&` mutant makes the "skip a malformed token" guard
unsatisfiable (an empty remainder is vacuously all-digits, so "not all digits AND empty" can never
hold), so a token like `Grinder` starts reading as a grinder; the test that catches it is the token
`||` was guarding against. The other six are all `Series::report`, none of them exercised because
every existing test asserts specific substrings are present and never that an *absent* one stays
absent: the "never reached the workload" line printed when `attempts == draws.len()` (`>` to `>=`),
the distribution header printed only when there is nothing to show it for (`!judged.is_empty()`
deleted), a bucket's `==` filter flipped to `!=` (invisible in the old test because the wrong bucket
produced the *same* substring the right one would have, just in a different row), a single rate
rendered as a degenerate range (`lo == hi` guard disabled), and the exclusion footer printed when
nothing was excluded (`<` to `<=`). Closed with `Series`/`Draw` built directly rather than through
`tally`, since both are `pub` structs with `pub` fields kept exactly for this: pinning the report to
values chosen for the assertion rather than to whatever a fixture happens to contain.

**`port.rs`, 9 killed, 3 recorded gaps.** `candidates()` read `/dev` directly, so nothing about its
result could be pinned down by a host test without depending on what happens to be plugged into the
machine running it; per this crate's own `BUGS`, that is "this lane's own Mac has the CH343 attached
... CI has nothing". Following the same move `choose`/`pick` already made, the walk is now `scan(dir:
&Path)`, with `candidates()` as a one-line `scan(Path::new("/dev"))`, and `scan` is tested against a
temporary directory built for the purpose (a real prefix, a look-alike with the wrong prefix, and a
sort check) rather than against `/dev`. The `stty()` boundary itself (nine mutants collapsed to one
line: this module's own header calls it "the whole IO residue... on purpose") had never been called
by a test directly; every existing test reaches it only through `open`/`configure` against a regular
file, whose complaint text the tests check only for *presence*, which every mutant's `Some(...)`
also produces. A direct call with a flag no `stty` accepts is a real, portable failure with no board
required, and checking the exit status is neither the empty string nor the mutants' `"xyzzy"`
placeholder kills all nine at once, none of them by asserting exact OS-specific wording. **Two of the
three port.rs mutants left are `candidates`'s trivial delegation**, and are recorded gaps rather than
closed: `scan`'s tests prove the logic, but `candidates() = scan(Path::new("/dev"))` itself still
depends on what is plugged into the machine running `cargo test` to be observably non-trivial, the
same limitation the module's own `BUGS` already names for `configure`/`confirm_speed`. **The third is
`confirm_speed`'s whole body replaced with `None`.** Every path this test can build (a regular file,
a missing path, invalid UTF-8) makes the underlying `stty -f <path>` fail (`ok=false`, checked
directly against `/dev/null`: `stty: /dev/null isn't a terminal`), which already returns `None` by
the function's own `if !ok { return None; }`, so the mutant is indistinguishable from correct code on
every input a host test can construct. Reaching the `Some(...)` arm needs a file `stty` treats as a
real tty, which is exactly the "No test opens a real serial device" limitation this crate's `BUGS`
already carries; closing it needs a pseudo-terminal pair (`posix_openpt`), which is a dependency
decision under DECISIONS §46 (thin primitives or whole subsystems) that a mutation-triage lane has no
authority to take.

**`progress.rs`, 8 killed, 1 equivalent.** `machine_line`'s capture guard was `&&`-to-`||`, the same
"kept from first arrival" property `banner_line` already has a test for, `machine_line` did not.
`Stage::label` and its `Display` impl (a whole-body replacement to `""` or a no-op write) had never
been asserted on directly; every report reads them by composing a string around them, which does not
pin the label's own text down. `Failure::describe`'s one match guard (`reason.is_empty()`, forced to
`true` or to `false`) had a test for each *shape* of `Failure::FirmwareRefused` but neither called
`.describe()`. `LineFeeder::tail`'s whole body was never read back directly, only through the
`Feeding::tail` it also produces. **The one equivalent is `BootProgress::reach`'s `>` becoming
`>=`.** The extra branch fires only when `stage == self.reached`, which for the one variant carrying
data (`Stage::Firmware(&'static Rung)`) can only hold when the two are the *same* rung: within one
session every `Stage::Firmware` comes from the one profile a `BootProgress` was built with, and
`every_profiles_depths_count_from_one_without_gaps` (in `board`, and depth is `Rung`'s whole identity)
is what makes a depth point at exactly one rung in that profile. Reassigning `self.reached` to a
value observably identical to what it already held is not observable. Recorded at the function
itself rather than only here, per this file's own convention.

**`screen.rs`, 2 killed.** `ReadError`'s `Display` (a no-op write, same shape as `Stage`'s above) had
only ever been checked by `PartialEq` on the *value*, never by reading what a person at a bench
would see. The header's three-way `max != 255 || width == 0 || height == 0` had one existing test
for the `max` term alone (`b"P6\n2 2\n254\n"`, both dimensions valid); an `||` flipped to `&&`
anywhere in the chain survives that test because the single true term already short-circuits the
*correct* code. Closed with `width == 0` and `height == 0` each alone, the other dimension valid,
which a flipped `&&` fails on regardless of which of the two `||`s it replaced.

**`stop.rs`, 2 killed.** `Report::describe`'s whole body (`String::new()` or `"xyzzy".into()`) was
never called by any test; every existing one checks `Escape::report()`'s *value*, and `describe()`'s
wording is only ever read by `Session::summary`, which composes rather than asserts on it. One test
covering all five variants, checking a distinguishing substring each.

**`watch.rs`, 7 killed.** `Session::summary`'s whole body, the same shape as `Report::describe` one
level down and never called directly for the same reason; `Session`'s fields are all `pub`, so it is
built directly rather than watched through a real session. `bytes += chunk.len()` becoming `*=` was
invisible because the one existing assertion on `.bytes` checks the *zero* case, where `0 * n` and
`0 + n` agree for `n = 0`; the fix asserts `session.bytes == sink.len()` on a real transcript, where
every byte read is also a byte written and the two counts have to agree by construction. The tail
flush's `!feeder.tail().is_empty()` deleted (so the flush fires only on an *empty* tail, a no-op) had
never been exercised: nothing in the suite fed a source that ends mid-line, which this module's own
header names as the reason the flush exists at all (`[PANIC] ...` with no trailing newline, followed
by a halted machine). `reader`'s `Err(e) if e.kind() == io::ErrorKind::Interrupted` (all three of the
guard forced `true`, forced `false`, and its `==` flipped to `!=`) had never been reached by any
test, because nothing in the suite hands the reader thread a real `io::Error`. Closed by a source
that returns `Interrupted` once and a genuine error after it, checking that the interrupted read was
retried (no `Failed` sent, the loop kept going) and the real one was not (propagated as the `Err`
`watch` returns).

**No defect in shipped behaviour was found.** Every survivor here was a missing test, or one of the
two documented, measured equivalences above.

## 2026-09-20: `video_terminal`, milestone 326 (turn a mutation score upward) part 3

`video_terminal` was the largest untriaged survivor set in the tree, at **377 viable mutants, 79
survivors, 79.0% caught** in the 2026-09-19 census. Same discipline as the sections above: every
number below is re-derived with `script/mutation -p video_terminal` on this lane's own worktree
rather than trusted from the census, every kill was verified by re-running the sweep and watching
the mutant die, and every equivalence claim is a mutant the second run still reports.

**Before: 298 caught, 79 missed, 0 timeouts, 15 unviable (79.0% of viable). After: 361 caught, 16
missed, 0 timeouts, 15 unviable (95.8%).** This run's `before` column matches the census row for
row exactly, which this crate's own scope explains: it has no Kani harnesses and no loom model (no
`proofs::`, `verification::`, or `interleavings::` module anywhere in it), so it carries none of the
"never the crate's code" artifact the earlier sections warn about, and `cargo mutants -p
video_terminal --list` confirms the mutant set is genuinely `lib.rs`, `keymap.rs`, and `script.rs`.

| file | survivors | killed by a test | equivalent | recorded gap |
|---|---|---|---|---|
| `lib.rs` | 79 | 63 | 16 | 0 |
| `keymap.rs` | 0 | 0 | 0 | 0 |
| `script.rs` | 0 | 0 | 0 | 0 |
| **total** | **79** | **63** | **16** | **0** |

`keymap.rs` and `script.rs` carried no survivors at all; every mutant in either file was already
caught before this lane touched anything. All 79 survivors are in `lib.rs`, and no exclusion and no
recorded gap was needed: every one closed as a killed test or a demonstrated equivalence.

**Five tests were added, each closing coverage this crate never had before it, for 63 survivors.**

**Multi-byte UTF-8 decoding had never been tested at all (18 survivors, 17 closed here; the
eighteenth is equivalent, below).** Milestone 142 (a text display good enough that people use it
instead of a GUI) increment 2 added it, and every mutant in
`Vt::ground`'s continuation-byte branch, its shift-and-accumulate arithmetic, and all three
lead-byte masks (two-, three-, and four-byte sequences) survived because nothing in the suite had
ever fed a non-ASCII byte to `feed`. `utf8_decodes_every_sequence_length_and_recovers_from_a_bad_
one` feeds one character of each length (`é`, `日`, `🎉`), a truncated sequence (a lead byte
followed by a non-continuation byte), and a lone continuation byte that can never start one,
checking the exact `char` landed rather than just that something non-blank did. On inspection the
decoder itself is correct RFC 3629 (the lead-byte ranges, the overlong-form exclusion, the
replacement-character fallback all match); this was a coverage gap, not a defect.

**Scrollback (milestone 142 increment 2, a text display good enough that people use it instead of a
GUI) had no test at all: not `scroll_up`, not `scroll_down`, not `view_offset`, not
`scrollback_len`, and nothing scrolled far enough to read history back (42 survivors across
[`SCROLLBACK_CELLS`] itself, `Vt::cell`'s history branch, `scrollback_cell`,
`push_scrollback_row`, the two getters, and both scroll methods; 41
closed here, and the forty-second, `push_scrollback_row`'s source index, is equivalent, below).**
`scrollback_survives_the_rings_wrap_and_reads_back_in_order` feeds 350 lines through a two-row grid,
349 scrolls, comfortably past [`SCROLLBACK_ROWS`]'s 300-row cap: the ring wraps and its oldest 49
pushes are overwritten, which is the shape needed to prove the ring's modular arithmetic rather than
only its first lap. Six checkpoints (`view_offset` 1, 2, 50, 150, 299, 300) each check **both**
display rows: row 0 alone cannot distinguish `Vt::cell`'s `age = view_offset - 1 - row` from a
mutant's `+ row`, because at `row == 0` the two terms are the same value; row 1 is what makes the
sign matter, and at `view_offset == 1` row 1 comes from the *live* half of `Vt::cell` instead
(`live_row = row - view_offset`), which is what closes that arithmetic's own mutant.
[`SCROLLBACK_CELLS`] closes as a side effect of the same test: that constant sizes the `scrollback`
array, and a mutant computing it as `MAX_COLS + SCROLLBACK_ROWS` (432 cells) rather than `MAX_COLS *
SCROLLBACK_ROWS` (39,600) panics on an out-of-bounds index partway through the very first feed loop,
well before any assertion runs; a panicking test is a caught mutant, not a special case.

**`reset_to` had never been called (1 survivor, the whole function body).** Every other test builds
a `Vt` at its final size with `Vt::new` or never resizes, so the function that exists specifically
for a **runtime** geometry (`components/src/display_terminal.rs`'s own bring-up, per its own doc
comment) was completely unexercised. `reset_to_clears_everything_and_retargets_the_geometry` feeds
content that scrolls something into history, retargets to a different size, and checks the geometry,
the cursor, every cell, the damage rectangle, and that the old grid's scrollback did not survive the
retarget (`sb_len` clearing is `reset_to`'s own responsibility, separate from `scroll_down`'s).

**The size clamp had never been tested at its own boundary (2 of 4 survivors; the other 2 are
equivalent, below).** `clamp_cols`/`clamp_rows` had no test at `cols == MAX_COLS` or `cols ==
MAX_COLS + 1`, so a mutant relaxing `cols > MAX_COLS` to `cols == MAX_COLS` (which clamps only the
exact boundary value and lets anything past it through unclamped) survived.
`geometry_clamps_exactly_at_the_boundary_not_one_off_it` checks the boundary itself, one past it,
and zero, in both dimensions.

**`put` and `damage_cell`'s defensive guard had never been exercised from either side (2
survivors).** `if col >= self.cols || row >= self.rows` relaxed to `&&` only refuses a coordinate
where **both** axes are out of range; no caller in this crate ever calls either method with a
partly-out-of-range coordinate (`print` always writes at the cursor, which is always in bounds;
`erase_line` bounds its column with `.min(self.cols)` before calling `put`), so the guard was dead
as far as any public-API test could tell. `put_and_damage_cell_reject_a_partly_out_of_range_
coordinate` calls both private methods directly (the same `use super::*` access every test in this
module already has) with one axis in range and the other not, on both axes, and checks the private
`cells` field and `damage()` are both untouched.

**Sixteen are equivalent, and every one is argued from the code rather than asserted.**

- **`Attr::DEFAULT`, 2 mutants (`DEFAULT_FG | (DEFAULT_BG << 4)`).** `DEFAULT_BG` is `0`
  (`pub const DEFAULT_BG: u8 = 0;`). Shifting `0` left or right by any amount is `0` either way, and
  OR-ing or XOR-ing `7` with `0` is `7` either way, so both the `|`-to-`^` and `<<`-to-`>>` mutants
  compute the same constant no test could ever separate them on.
- **`CellRect::union`'s four min/max selections, 4 mutants**
  (`if self.col < o.col { self.col } else { o.col }` and its three siblings for `row`, `right`,
  `bottom`). A selector of this shape (`if a < b { a } else { b }`, or the `>` form for a max) gives
  the *same value* on both branches whenever `a == b`, because at that point `a` and `b` are the
  same number; `<` vs `<=` (or `>` vs `>=`) only disagree about which branch fires at that one point,
  and the branch that fires no longer matters once the two branches would return the same thing. So
  no input, not just no input a test happened to try, can distinguish the strict comparison from the
  non-strict one here.
- **The `>=` half of the size clamp, 2 mutants** (`cols > MAX_COLS as u32` in `clamp_cols`, and the
  same shape in `clamp_rows`). The same selector argument as `union`, applied to `if cols > MAX_COLS
  { MAX_COLS } else { cols }`: at `cols == MAX_COLS`, the "else" branch returns `cols`, which **is**
  `MAX_COLS`, so both branches already agree at the one point `>` and `>=` disagree about which of
  them to take. (This is the other half of the clamp's 4 survivors; the `==` half above is a real
  gap, not this one, because `==` only clamps the single boundary value and lets everything past it
  through, which is a different, genuine behaviour change.)
- **`Vt::pixel`'s `col < self.cols` in the cursor-overlay check, 1 mutant.** This term is reached
  only after `col == self.col` already matched in the same `&&` chain, and `self.col` is never `>=
  self.cols` (every cursor-moving path clamps with `.min(self.cols - 1)`/`.min(self.rows - 1)`, and
  `print`'s deferred wrap never advances `col` past `cols - 1` either; see [`Vt::erase_display`]'s
  own doc comment for the fuller statement of this invariant). So whenever `col == self.col` holds,
  `col < self.cols` already holds too, and relaxing it to `<=` cannot change which branch is taken.
- **`Vt::ground`'s continuation-byte accumulator, 1 of the 18 UTF-8-shaped mutants**
  (`(self.utf8_code << 6) | (b & 0x3f) as u32`). `self.utf8_code << 6` always has its low six bits
  zero (a left shift by six fills them with zero), and `b & 0x3f` is exactly six bits, so the two
  operands never share a set bit. OR and XOR agree whenever operands share no set bits, on every
  input, not because of anything a caller does.
- **`Vt::push_scrollback_row`'s source index, 1 mutant** (`row * cols` read as `row / cols`). This
  is a private method with exactly one call site (`Vt::line_feed`), which always passes the literal
  `0`: the row about to scroll off the top is always the grid's first row, because scrolling is what
  makes room at the *bottom*. `0 * cols` and `0 / cols` are both `0`, on the only input this function
  is ever given.
- **`Vt::csi`'s private-use/intermediate-byte arm, 1 mutant** (deleting `0x20..=0x2f | 0x3c..=0x3f
  => self.ignore = true`). Those two byte ranges are not matched by any other arm in the same
  `match` (`b'0'..=b'9'`, `b';'`, `0x40..=0x7e`), so deleting the arm routes them to the catch-all
  `_ => { self.ignore = true; }`, which does the identical thing. The two arms stay written
  separately because a reader needs to see the two families of swallowed byte (a sequence this
  engine chose not to implement, versus a stray control code) named apart, even though the code that
  runs is the same either way.
- **`Vt::erase_display`'s damage guard, 1 mutant** (`if to > from`). `to > from` holds on every
  reachable input: mode 0's `end - here = (rows - row) * cols - col >= cols - col >= 1` (since `row
  < rows` and `col < cols`, both invariants this type maintains everywhere); mode 1's `here + 1 >=
  1`; the default's `end = rows * cols >= 1` (since `cols >= 1` and `rows >= 1`, `Vt::clamp_cols`/
  `Vt::clamp_rows`'s own floor). So `to == from` is unreachable, and relaxing `>` to `>=` cannot
  change which branch fires.
- **`Vt::sgr`'s bit-independent recolouring, 3 mutants.** `(p as u8 - 30) | bright` (the 16-colour
  foreground SGR codes, 30–37) and `(p as u8 - 90) | 8` (the bright foreground codes, 90–97): both
  left operands occupy bits 0–2 only (`p - 30` and `p - 90` each range `0..=7`), `bright` and the
  literal `8` occupy bit 3 only, so OR and XOR agree on every input, the same shape as the UTF-8
  accumulator above. `p as u8 - 40` (the background codes, 40–47) read as `p as u8 + 40`: `Attr::new`
  masks its `bg` parameter with `& 0x07` before storing it, and `(p - 40)` and `(p + 40)` differ by
  exactly `80`, which is a multiple of `8`, so the two are congruent mod 8 and the masked result is
  identical for every `p` in `40..=47`.

**No defect in shipped behaviour was found.** Every survivor here was a missing test or one of the
sixteen equivalences above, each demonstrated rather than asserted, per this crate's own two
non-selector cases (`erase_display`, `pixel`) resting on an invariant named and cited at the
function itself rather than only here, matching this file's own convention.
