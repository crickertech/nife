# Proof harnesses, Rust, BUGS sections and coverage

*An appendix to [the register of measures](../register-of-measures.md). The name is provisional.*

## Kani proof harnesses, and what can falsify them

The harness count is the whole series; the split into "falsification on record" and "unfalsified"
has exactly one point, because the record itself is four days old. Milestone 194 (build §134: the
falsification record, its lint, and the sweep that replays it) built it, and DECISIONS §134 (a
harness carries a machine-replayable falsification record, or it is not evidence) is the reason.

**36 of 146.** This is fatal risk 2's number, and the risk is that the proofs prove trivia. A proof
nobody can turn red is one level of indirection away from evidence, which is the same shape as a
roadmap status that was wrong in both records. A harness with no record at all is counted as
unfalsified, because that is what it is.

The harness count itself is taken after blanking comments, which matters more than it sounds: a raw
grep for `#[kani::proof]` finds 151, because `kernel/src/syscall.rs` explains in prose what one is
and `vendor/` and a lint shim carry four more.

## Rust in the tree

Every tracked `.rs` file outside `vendor/`, with `kernel/src` split from the rest. The split is the
point rather than a courtesy: the kernel is commented far more heavily than production code would be,
deliberately, so a single tree-wide ratio hides the thing that makes the number interesting.

A line counts as code if anything survives blanking its comments and string literals, and as a
comment otherwise. A line with code and a trailing comment is code.

**Two things worth reading.**

**The kernel's comment share is not flat, it is climbing.** `kernel/src` was 39.3% comment lines at
2026W31 and is 45.3% at 2026W36, rising every week in between. `AGENTS.md` says 40%, which was
accurate when it was written and is now three weeks stale. The rest of the tree is doing the same
thing more slowly, 31.5% to 41.4%. Whether that is the kernel getting better documented or the
comment-to-code ratio drifting past what a reader wants is not a question this instrument can
answer; it can only say the number moved. Flagged rather than corrected in `AGENTS.md`, because that
is calef's file.

**Total Rust fell for the first time between 2026W35 and 2026W36**, from 192.0k lines to 189.0k,
while the week was still adding code. That is milestone 54's deletion, which is also the `REMOVED`
bar in the Milestones by status chart above. A line count that only ever rises is measuring typing;
one that falls when code is deleted is measuring the tree.

## BUGS sections

**Rising is good here, and it is worth saying plainly because the word says the opposite.** A `BUGS`
section is the FreeBSD convention this tree copies hardest: an honest limitation written next to the
feature it limits, in the manual, rather than hidden in a tracker. `AGENTS.md` calls them "not
modesty, they are the mechanism", and the reason is a newcomer's: someone who hits a limitation the
docs named will trust the docs, and someone who hits one the docs hid will not trust anything again.

Zero in the first two weeks, 359 at 2026W36 (238 markdown headings and 121 in Rust doc comments).
A falling line here would be the alarming one.

## Coverage

**Every week but the first is measured, and each was measured by its own tree.** This section
used to say coverage could not be recovered from history. That was a rule about `script/metrics`,
which may not build or check anything out, mistaken for a fact about the measurement. On 2026-09-19
a lane checked out each week's representative commit in a throwaway worktree, ran **that commit's**
`script/coverage` on **that commit's** pinned nightly, and handed the lcov to
`script/metrics --coverage-for <WEEK> --coverage-from <lcov>`. That is the same instrument the
weekly workflow runs, so the backfilled cells mean what the live ones mean.

**This is the one series on the page that is not a restatement.** Every other column applies
today's definitions to an old tree. Coverage applies each week's own: its own crate exclusions and
its own feature flags, so the scope moves from week to week exactly as it moved at the time.

| week | commit | toolchain | lines hit / found | per cent |
|---|---|---|---|---|
| 2026W29 | `a80e5182d54c` | none | | **empty** |
| 2026W30 | `aef018cdaa3a` | nightly-2026-07-26, **reconstructed** | 1954 / 2114 | 92.4 |
| 2026W31 | `190268d086f0` | nightly-2026-08-02 | 11019 / 12106 | 91.0 |
| 2026W32 | `f6fd097488b1` | nightly-2026-08-04 | 15712 / 16746 | 93.8 |
| 2026W33 | `60698aa1a594` | nightly-2026-08-16 | 20573 / 21976 | 93.6 |
| 2026W34 | `132f6ad08ade` | nightly-2026-08-23 | 25084 / 26624 | 94.2 |
| 2026W35 | `685900ec6bf5` | nightly-2026-08-30 | 27330 / 28959 | 94.4 |
| 2026W36 | `d0b254c5a5e6` | nightly-2026-09-06 | 26066 / 27604 | 94.4 |
| 2026W37 | `5cd67cd3f193` | nightly-2026-09-13 | 26472 / 27952 | 94.7 |
| 2026W38 | `5d9d5e4c1f6a` | nightly-2026-09-17 | 26063 / 27525 | 94.7 |
| 2026W39 | `3dd86628a` | nightly-2026-09-23 | 32365 / 33994 | 95.2 |

Four things a reader should hold against those numbers:

- **2026W39 was measured by CI, not on the dev Mac, and at a different commit from the one in its
  row.** The dev machine was under four lanes' builds that evening, and this page's own guidance is
  that a heavy job taken under contention is worth less than an honest gap. The lcov is the one
  `ci.yml`'s coverage job uploaded for `3dd86628a` (run 35917383053, green), which is the same
  instrument on the same week; the row's other columns are read from `35390e595f89`, later in that
  week. That is the "nothing ties an lcov file to a commit" caveat above, firing on purpose rather
  than by accident, and it is the same size of gap this page already accepts between a live cell and
  a backfilled one.

- **2026W29 is empty because no instrument existed.** `script/coverage` arrived on 2026-07-22.
  Running a later script on that tree would measure something the week never measured, which is a
  restatement of a different kind from the rest of this page, and the bar is left missing rather
  than invented.
- **2026W30's toolchain is a reconstruction.** Its `rust-toolchain.toml` said `nightly`, unpinned,
  so the compiler that week actually used is not recorded anywhere. The backfill used the nightly
  dated on the commit's UTC day, which is what the floating channel resolved to when it was made.
  Every later week is pinned and was measured on exactly its pin.
- **2026W36 was the control, and it reproduced.** Its 94.4 was measured by the weekly workflow on
  2026-09-07 on Linux (cargo-llvm-cov 0.9.0, 26029 of 27571 lines); the backfill on the dev Mac
  (cargo-llvm-cov 0.8.7) got 26066 of 27604. Both round to 94.4. The 33-line gap is the platform
  and tool version, and it is the size of error to expect between a live cell and a backfilled one.
  The tool is today's `cargo-llvm-cov` for every backfilled week, not the version each week had.

2026W38's earlier 94.6 was taken at that week's first Monday commit; it was re-measured at the
row's current commit, so its cell and the rest of its row now describe the same tree. 2026W31's dip
is a change of scope rather than of testing: the measured set went from 15 files to 49 that week,
which is also the week rule 7 turned `#[path]` modules into crates.

The floor `script/coverage` gates on is per file, not this aggregate; the dashed line is that floor
drawn for scale, and the panel below plots the number it is actually drawn against.

## The lowest-covered file

**The chart above is the trend; this one is the thing that can fail a build.** `script/coverage`
gates at 80% **per file**, never on the aggregate, and the two can move in opposite directions
without contradicting each other: the workspace can sit at 94.7% while one file slides from 85% to
81% and the aggregate does not visibly twitch, because that file is a few hundred lines out of
twenty-eight thousand. Reading the aggregate as "the coverage the floor is set against" is the
natural reading and it is wrong; this panel exists so the page stops inviting it. The argument is
`script/coverage`'s own, applied to the dashboard: a per-total number hides a hole, because a big
well-tested crate subsidizes an untested one.

**The series starts at 2026W39, and a short line here is a missing record rather than a new
problem.** A per-file minimum needs that week's lcov, and only the aggregate was ever kept from
each run, so there is nothing to recompute the earlier weeks from. Re-measuring them the way
coverage itself was backfilled would not help either: the exemption list in `script/coverage` has
changed several times (build scripts in 2026-08-30, the host half of `stick_maker` in 2026-09-19),
so a minimum taken today over an old lcov would be a minimum over a population that week's gate did
not have. Empty weeks are marked on the chart rather than drawn as zero, the same treatment
`unsafe_trust_*` gets for the ten weeks before its census existed.

**Where the number comes from, and why not from the lcov.** `script/coverage` writes
`target/llvm-cov/floor.txt` beside its lcov, in the same awk pass that applies the floor, and
`script/metrics --coverage-min-from <floor.txt>` reads it. Nothing re-parses the lcov to find a
minimum, and that is deliberate rather than tidy: the lowest file in an unfiltered lcov is one of
the files that cannot execute a line on the host (`virtio`, the protocol wrappers, a `build.rs`),
so an independently derived minimum would be a number no build can ever fail on. The exemptions
that decide the population live in `script/coverage` and nowhere else, so the minimum has to be
taken there too.

`floor.txt` carries more than the one plotted cell: which file is lowest, how many files are
gated, and how many a floor of 85 or of 90 would newly fail. `script/coverage` prints the same
three lines at the end of every run. That last pair is the number a proposal to raise the floor
needs, and it is reported rather than left to be re-derived from the HTML report.

**What the distribution actually says, measured 2026-09-23.** Over the 116 files the floor acts on
at `3dd86628a`: none under 80%, **one** in 80-84.9%, **one** in 85-89.9%, 16 in 90-94.9%, 66 in
95-99.9%, and 32 at 100%. The minimum is **84.0%**, `crates/machine_discovery/src/interrupt_id.rs`
(21 of 25 lines), and the next one up is
`crates/globally_unique_identifier_partition_table/src/lib.rs` at 89.7%. So **a floor of 85 newly
fails one file, and a floor of 90 newly fails two.** Raising the floor is not a backlog here; it is
two files. That is the number the question needs, and it is calef's to act on. `script/coverage`
prints it on every run, so it never has to be re-derived.

**The count at 90 is platform-sensitive, and the platform that gates is CI.** The same run on the
dev Mac later that evening (cargo-llvm-cov 0.8.7, nightly-2026-09-23) agrees exactly on the minimum,
84.0% and the same file, and on the aggregate to two decimals, 95.21 against CI's 95.2. It disagrees
on one file: `crates/globally_unique_identifier_partition_table/src/lib.rs` is **315 of 351 lines
(89.7%) on CI and 318 of 351 (90.6%) here**. Three lines, straddling 90, which is enough to move the
85-89.9% band from one file to none and the answer at a floor of 90 from two files to one. So a
floor of 90 would fail a build on CI while passing for the person asked to fix it, which is the
worse of the two directions. This is the same class of platform gap the 2026W36 control already
prices at 33 lines; it only becomes visible here because a single file happens to sit on the
boundary. A floor of **85** has no such ambiguity on either platform.

**A higher floor is not automatically a better one, and this panel must not be read as a target.**
`script/coverage` says it plainly: the floor is a floor, a file at 81% is not "done", and 100% is
not the goal because tests have to prove something. `AGENTS.md` is blunter still: do not add filler
tests. A floor raised above what the tree has earned pushes a lane toward writing whatever reaches
the number, and this tree has already caught three variants of a test that passes while proving
nothing. The number to watch on this chart is the direction, and a drop is worth more attention
than the level.

**BUGS.** The series cannot be backfilled, for the reason above. Nothing ties `floor.txt` to a
commit any more than it ties the lcov to one, so `--coverage-min-from` believes the caller about
which week it measured. The cell is carried across a `--backfill` like the aggregate, so after a
row is repointed at a later commit of the same week the minimum describes the earlier one until
someone re-measures. And the minimum is one file: two files at 81% and one at 81% draw the same
bar, which is what `floor.txt`'s band counts are for and the chart is not.
