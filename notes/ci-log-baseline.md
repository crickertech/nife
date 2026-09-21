# The CI log baseline: per-check attribution mined before the logs expire

*Names: `notes/ci-log-baseline.md`, `script/ci-log-baseline` and the CSV below are **provisional**.
Naming is calef's (AGENTS.md); a lane ships a provisional name and says so.*

This tree cannot currently answer "has this gate ever fired in CI?" GitHub Actions keeps a failed
run's metadata forever, but a job conclusion alone says only "the job failed", not which of its
sub-checks did. The one place that answer lives is the job's own log, and GitHub deletes those on a
retention window. `design/roadmap/proposals/no-gate-records-when-it-fires.md` (a research lane's
proposal, calef's to rule on, unedited by this record) mined that answer for the `clippy` job on
2026-09-21 and found the window closing around 2026-10-21. This page and
[`notes/project-metrics/ci-log-baseline.csv`](project-metrics/ci-log-baseline.csv) absorb that
mining into a committed, re-derivable record, and extend it to every other CI job whose log names a
sub-check the same way, before more of it is gone.

**This is not the gate-firing ledger** proposed alongside it. That ledger (still calef's to rule on)
would capture every future, local firing, the ones that never reach Actions because a lane ran
`script/lint`, saw red, fixed it, and pushed green. This page captures only the past, and only the
slice of the past that escaped local gating and reached GitHub. It is not a ranking of which gates
are worth keeping; see the caveat below.

## The record, and its provenance

[`notes/project-metrics/ci-log-baseline.csv`](project-metrics/ci-log-baseline.csv), one row per
failed CI job GitHub still names, for the five job types this script knows wrap more than one
distinguishable check: `run_id,job_id,date,workflow,job_name,attributed_check,branch`.

`script/ci-log-baseline --update` writes it and `--report` reads it back; its own header carries the
usage, the `EXAMPLES` and its `BUGS`.

**Reused rather than refetched, where it was safe to.** A prior research lane (the gate-audit
proposal above) had already fetched all 309 `clippy` job logs and all 99 `bench (icount regression
tripwire)` job logs while writing its proposal, and left the mined output in its session
scratchpad rather than the repo, per its own report. This lane found it there, and re-verified it
rather than adopting it on trust: **five `clippy` rows spread across the full date range (the
oldest failure, one from 2026-08-17, one from 2026-08-25, one from 2026-09-16, and the newest
distinct-check firing) were re-fetched live and matched the cached marker exactly**, and two `bench`
rows (one `TRIPWIRE`, one generic failure) were re-fetched and matched too, down to the exact
benchmark name and the exact `##[error]` line. Everything else in this record (`supply chain`, `cpu
matrix`, `verify (Kani proofs)`, and the run-to-branch mapping) was fetched fresh by this lane, not
reused.

**Which job types, and why only these five.** `script/lint` runs 47 named checks inside one `clippy`
job and exits on the first failure, so the log's last `==>` marker names the check that was running.
Three more job types share that convention closely enough to use the same technique:

- **`supply chain (advisories, licences, vendored integrity)`** runs `script/supply-chain`: one
  `cargo-deny` pass per manifest (six of them), then `script/vendor-verify`'s own per-package pass,
  each announced with its own `==>` line, and `set -e` stops the script at the first one that fails.
  The last marker names it exactly: 12 of 14 failures are the main workspace's own manifest, one is
  `script/vendor-verify` failing on the `redoxfs` pin specifically, and one has no marker at all
  because `taiki-e/install-action` failed to download its own tool before `script/supply-chain` ever
  ran (recorded as unattributable, not silently dropped).
- **`verify (Kani proofs)`** runs `script/verify`'s per-crate Kani loop the same way: one `==> kani:
  <crate> (<what it proves>)` line per harness, `set -e`, last marker names the harness. Only five
  of these have ever failed.
- **`cpu matrix (riscv64 across QEMU CPU models)`** runs `script/cpu-matrix`, and its last `==>`
  marker is **not** useful on its own: the script prints `==> matrix` as a section header after
  every model has run, whether any failed or not, so that marker is identical on all 213 failures
  and would attribute nothing. What does distinguish them is the line the script prints to stderr,
  `cpu-matrix: failed on: <models>`, or, for the rarer case where the probe that confirms `-cpu` is
  actually narrowing the machine fails before the model loop starts, the `==> preflight: ...` marker
  that precedes it (nothing follows a preflight exit, so the ordinary "last marker" rule works there).
  209 of 213 name one or more specific CPU models (`rva22s64` 157 times, `sifive-u54` 155,
  `thead-c906` 154, `rva23s64` 150, `rv64` 148; these overlap, since a single failed job can name
  several models), zero are preflight failures, and four failed during QEMU's own build/install step,
  before `script/cpu-matrix` printed anything of its own.
- **`bench (icount regression tripwire)`** runs `script/icount` via `xtask bench`, which has no
  `==>` convention at all. It prints `bench: CHECK FAIL <name>: <cur> vs baseline <base> (...)` per
  regressed benchmark when the tripwire itself fires, which this record greps for directly, the
  technique the gate-audit proposal already used by hand for this one job type. 43 of 99 carry that
  line (`spawn_el` 21, `map_new` 16, `yield_switch` and `ctx_switch` 4 each, plus a handful of other
  benchmarks once or twice each); the other 56 are recorded as `unattributable: not the tripwire`,
  because their log carries nothing more specific than GitHub's own generic
  `##[error]Process completed with exit code 1`-style line, which this record captures too (truncated)
  rather than reducing to a bare "unattributable".

**Every other failed job type is a single check, and GitHub's job name already is the finest
attribution available.** Examined and ruled out for that reason: `build + test (host + QEMU)`
(327 failures), `coverage (host crates)` (104), `rustfmt` (55), `fastpath footprint (the IPC path
must stay L1i-sized)` (33), `stack frames (no frame over a third of a thread stack)` (14),
`undefined-behavior check (host crates, sampled paths)` (5), `build against the latest nightly`
(19), `architect hold (needs-architect label)` (67, and this one is a status check reflecting a
label rather than a script running a test, so "which check fired" does not apply to it at all),
`mutants (shard N/M)` (varying shard counts across two mutation-testing eras), `re-falsify the
harnesses this change can reach` (4), `report against the baseline` (3), `propose a pin bump` (3),
`is an audit due` (5), and CodeQL's `Analyze (javascript-typescript)` (2). One of these,
`coverage (host crates)`, DOES carry finer detail in its log: `script/coverage`'s floor check names
the offending file on a `coverage: <file>: N% ... under the floor` line, but that line carries no
`==>` and the marker that follows it, `coverage floor FAILED`, is identical on all 104 failures.
Capturing the file-level detail would need a different extraction technique than the one this
record uses everywhere else, and is left as a `BUGS` entry rather than folded in here.

## The honest accounting, 2026-09-21

| | |
|---|---:|
| Failed CI runs on record (metadata; never expires) | 909 |
| Failed CI jobs on record, all types (metadata; never expires) | ~1,310 |
| Failed jobs among the five attributable job types | 640 |
| ...attributed to a specific sub-check | 583 |
| ...recorded as unattributable, with a reason | 57 |
| ...of which "log expired" | 0 |
| ...of which "log unavailable" for some other reason | 1 (a tool-install failure before the job's own script ran) |
| ...of which "ran, but not the check this job type is watched for" | 56 (bench, non-tripwire failures) |
| Oldest date reached | 2026-07-23 (this repository's very first CI run; there is nothing older to reach) |
| Newest date reached | 2026-09-21 |
| Roughly how many GitHub API calls this lane made | ~260: 1 total-run-count probe, ~10 paginated pages to re-list all 909 failed runs with their branch, ~10 probe fetches to check log survival across the date range by hand, 232 fresh log fetches (14 supply chain + 213 cpu matrix + 5 verify), 7 re-verification fetches (5 clippy, 2 bench), plus the calls the discovery process above made while locating and reading the reused cache. The 309 clippy and 99 bench log fetches were **not** repeated; they were reused from a prior lane's cache and spot-checked live instead. |

**Zero logs had expired at capture time, which is not what this lane was told to expect.** The brief
that opened this work stated the oldest failed run's log already read as expired. Checked directly
against the API: job 89101327085 (the run at 2026-07-23T02:13:08Z, the very first CI run this
repository has ever recorded, confirmed by paging to the end of the unfiltered run list) still
returns its full log, as does every job sampled from 2026-07-24 through 2026-08-01 and the specific
`clippy` job the earlier proposal named as its oldest. This agrees with, rather than contradicts, the
gate-audit proposal's own measurement made the same day: retention has not started deleting logs and
is not expected to before roughly 2026-10-21. **Corrected here rather than silently worked around**,
per this tree's own convention: the premise that urgency had already turned into loss was wrong: there
is still a window, not a diminishing one already mid-collapse. The urgency is real (the window does
close, and every day between now and 2026-10-21 is a day closer), just not as far along as stated.

## What this preserves from the gate-audit proposal, and what this record's own data says about it

The proposal's clippy table is reproduced almost exactly by this record's own 309 `clippy` rows,
re-derived independently by grouping this CSV's own `attributed_check` column rather than copied
from the proposal's prose: 78 roadmap status, 50 markdown links and the notes index, 37 naming
conventions, 27 em-dashes, 27 decisions, 21 the host+xtask clippy pass, 18 counted claims, 14
citations, 8 new citations say what they cite, 4 spelling, 2 each for TODO-cites-a-milestone/script
docs/every fence names its counterpart, 1 each for rustdoc/the separate-workspace clippy pass/the
riscv64 shell-feature pass. That means the proposal's derived finding, **that 32 of `script/lint`'s
47 named checks have never failed a CI job**, including several with the best-documented real defect
catches in the tree (`conflict markers`, `sh -n`, `unsafe fn contracts`), is preserved and
cross-checked here rather than merely repeated.

**One number is corrected rather than repeated.** The proposal states 13 firings for "the aarch64
clippy pass, under its three successive names". Grouping this record's own three aarch64 marker
texts gives 16: `kernel + user + user_rt (aarch64)` 10, `kernel + user + user_mode_runtime
(aarch64)` 3, and the oldest form, from before the runtime crate existed as its own name,
`kernel + user (aarch64)` 3. 10 + 3 + 3 = 16, not 13, and the arithmetic on the row above it agrees:
this record's 19 distinct markers sum to exactly 309 only under 16, not 13. Recorded here rather
than silently fixed in place, per this tree's own convention of correcting the record on purpose
when the machine and the document disagree.

**Restated in this record's own voice, because it is worth restating rather than only citing: this
is not a ranking of which gates are worth keeping.** A check that fires locally, gets fixed, and is
pushed green never reaches GitHub Actions at all, so this record and its CSV systematically
undercount exactly the gates that work earliest and cheapest, which is a different thing from the
gates that matter least. A count of zero in this record means "never failed a CI job", never "never
caught anything."

## BUGS

- **This record cannot tell a retry from a new event.** A CPU model or a benchmark that fails on
  three consecutive re-runs of the same push produces three rows here, indistinguishable from three
  separate regressions on three separate days. The gate-audit proposal noted the same limit for
  `bench`'s clustering (13 failures on one day, 10 the next, read as "one condition re-failing"
  rather than ten events); it applies to every job type in this record.
- **`coverage (host crates)`'s per-file detail is not captured**, see above; a future extension would
  need to grep the `coverage: <file>:` lines specifically rather than the `==>` convention this record
  otherwise relies on throughout.
- **The four `cpu matrix` failures attributed to a QEMU build/install step are a real category this
  record does not further distinguish**: a failure there means the runner's QEMU cache was cold or
  stale, not that any CPU model actually ran and failed, and `script/ci-log-baseline` records the
  install-log marker verbatim rather than inventing a category for it.
- **`architect hold` and `is an audit due` are gates over process state, not over code**, and were
  ruled out of this record for that reason; a "which check fired" question does not apply to a label
  or a cadence date the way it does to a script with named sub-checks.
