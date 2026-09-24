# Proof harnesses, Rust, BUGS sections and coverage

*An appendix to [the register of measures](../register-of-measures.md). The name is provisional.*

## Kani proof harnesses, and what can falsify them

The harness count is the whole series. The split into "falsification on record" and "unfalsified"
has exactly one point, because the record itself is four days old. Milestone 194 (build §134: the
falsification record, its lint, and the sweep that replays it) built it. DECISIONS §134 (a harness
carries a machine-replayable falsification record, or it is not evidence) is the reason.

36 of 146 harnesses have a falsification on record. This is fatal risk 2's number, and the risk is
that the proofs prove trivia. A proof nobody can turn red is one level of indirection away from
evidence. That is the same shape as a roadmap status that was wrong in both records. A harness with
no record at all counts as unfalsified.

The harness count is taken after blanking comments. A raw grep for `#[kani::proof]` finds 151:
`kernel/src/syscall.rs` explains in prose what one is, and `vendor/` and a lint shim carry four
more.

## Rust in the tree

The series counts every tracked `.rs` file outside `vendor/`, with `kernel/src` split from the
rest. The kernel is commented far more heavily than production code would be, deliberately. A
single tree-wide ratio would hide that.

A line counts as code if anything survives blanking its comments and string literals, and as a
comment otherwise. A line with code and a trailing comment is code.

### The kernel's comment share is climbing

`kernel/src` was 39.3% comment lines at 2026W31 and is 45.3% at 2026W36, rising every week in
between. `AGENTS.md` says 40%, which was accurate when written and is now three weeks stale. The
rest of the tree is moving the same way more slowly, from 31.5% to 41.4%. This instrument cannot say
whether the kernel is better documented or its comment ratio is drifting past what a reader wants.
It can only say the number moved. The figure in `AGENTS.md` is flagged rather than corrected,
because that is calef's file.

### Total Rust fell for the first time

Between 2026W35 and 2026W36 total Rust fell from 192.0k lines to 189.0k, while the week was still
adding code. That is the deletion of milestone 54 (a network file service a Mac can actually mount).
It is also the `REMOVED` bar in the Milestones by status chart in `notes/project-metrics.md`. A line
count that only ever rises is measuring typing; one that falls when code is deleted is measuring the
tree.

## BUGS sections

Rising is good here, although the word suggests the opposite. A `BUGS` section is the FreeBSD
convention this tree copies hardest: an honest limitation written next to the feature it limits, in
the manual, rather than hidden in a tracker. `AGENTS.md` calls them "not modesty, they are the
mechanism". The reason is a newcomer's. Someone who hits a limitation the docs named will trust the
docs; someone who hits one the docs hid will not trust anything again.

The count was zero in the first two weeks and 359 at 2026W36 (238 markdown headings and 121 in Rust
doc comments). A falling line here would be the alarming one.

## Coverage

Every week but the first is measured, and each was measured by its own tree. This section used to
say coverage could not be recovered from history. That was a rule about `script/metrics`, which may
not build or check anything out, mistaken for a fact about the measurement.

On 2026-09-19 a lane checked out each week's representative commit in a throwaway worktree. It ran
that commit's `script/coverage` on that commit's pinned nightly. It handed the lcov to
`script/metrics --coverage-for <WEEK> --coverage-from <lcov>`. That is the same instrument the
weekly workflow runs, so the backfilled cells mean what the live ones mean.

Coverage is the one series in the deck that is not a restatement. Every other column applies
today's definitions to an old tree. Coverage applies each week's own crate exclusions and feature
flags, so the scope moves from week to week exactly as it moved at the time.

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

Hold these caveats against the numbers:

- 2026W39 was measured by CI, not on the dev Mac, and at a different commit from the one in its
  row. The dev machine was under four lanes' builds that evening. The register's own guidance is
  that a heavy job taken under contention is worth less than an honest gap. The lcov is the one
  `ci.yml`'s coverage job uploaded for `3dd86628a` (run 35917383053, green): the same instrument on
  the same week. The row's other columns are read from `35390e595f89`, later in that week. This is
  the "nothing ties an lcov file to a commit" caveat, recorded in the BUGS header of
  `script/metrics`, firing on purpose. The gap is the same size the 2026W36 control prices between a
  live cell and a backfilled one.
- 2026W29 is empty because no instrument existed. `script/coverage` arrived on 2026-07-22. Running
  a later script on that tree would measure something the week never measured. That would be a
  different kind of restatement from the rest of the deck, so the bar is left missing rather than
  invented.
- 2026W30's toolchain is a reconstruction. Its `rust-toolchain.toml` said `nightly`, unpinned, so
  the compiler that week actually used is not recorded anywhere. The backfill used the nightly
  dated on the commit's UTC day, which is what the floating channel resolved to when it was made.
  Every later week is pinned and was measured on exactly its pin.
- 2026W36 was the control, and it reproduced. The weekly workflow measured 94.4 on 2026-09-07 on
  Linux (cargo-llvm-cov 0.9.0, 26029 of 27571 lines). The backfill on the dev Mac (cargo-llvm-cov
  0.8.7) got 26066 of 27604. Both round to 94.4. The 33-line gap is platform and tool version, and
  it is the size of error to expect between a live cell and a backfilled one. Every backfilled week
  used today's `cargo-llvm-cov`, not the version each week had.

2026W38's earlier 94.6 was taken at that week's first Monday commit. It was re-measured at the
row's current commit, so its cell and the rest of its row now describe the same tree. 2026W31's dip
is a change of scope rather than of testing. The measured set went from 15 files to 49 that week,
which is also the week rule 7 turned `#[path]` modules into crates.

`script/coverage` gates per file, not on this aggregate. The dashed line on the coverage chart is
that floor drawn for scale. The lowest-covered file panel plots the number the floor is actually
drawn against.

## The lowest-covered file

The coverage chart is the trend; this panel is the thing that can fail a build. `script/coverage`
gates at 80% per file, never on the aggregate. The two can move in opposite directions without
contradicting each other. The workspace can sit at 94.7% while one file slides from 85% to 81%, and
the aggregate does not visibly twitch. That file is a few hundred lines out of twenty-eight thousand.
Reading the aggregate as "the coverage the floor is set against" is natural and wrong, and this
panel exists so the deck stops inviting it. The argument is `script/coverage`'s own: a per-total
number hides a hole, because a big well-tested crate subsidizes an untested one.

### The series starts at 2026W39

A short line here is a missing record rather than a new problem. A per-file minimum needs that
week's lcov, and only the aggregate was ever kept from each run. There is nothing to recompute the
earlier weeks from.

Re-measuring them the way coverage was backfilled would not help either. The exemption list in
`script/coverage` has changed several times: build scripts in 2026-08-30, the host half of
`stick_maker` in 2026-09-19. A minimum taken today over an old lcov would cover a population that
week's gate did not have. Empty weeks are marked on the chart rather than drawn as zero. The
`unsafe_trust_*` series gets the same treatment for the ten weeks before its census existed.

### Where the number comes from

`script/coverage` writes `target/llvm-cov/floor.txt` beside its lcov, in the same awk pass that
applies the floor. `script/metrics --coverage-min-from <floor.txt>` reads it. Nothing re-parses the
lcov to find a minimum, deliberately. The lowest file in an unfiltered lcov is one that cannot
execute a line on the host (`virtio`, the protocol wrappers, a `build.rs`). An independently derived
minimum would be a number no build can ever fail on. The exemptions that decide the population live
in `script/coverage` and nowhere else, so the minimum is taken there too.

`floor.txt` carries more than the plotted cell. It records which file is lowest, how many files are
gated, and how many a floor of 85 or of 90 would newly fail. `script/coverage` prints the same three
lines at the end of every run. That last pair is the number a proposal to raise the floor needs, and
it never has to be re-derived from the HTML report.

### The distribution, measured 2026-09-23

Over the 116 files the floor acts on at `3dd86628a`:

- none under 80%
- one in 80-84.9%
- one in 85-89.9%
- 16 in 90-94.9%
- 66 in 95-99.9%
- 32 at 100%

The minimum is 84.0%, `crates/machine_discovery/src/interrupt_id.rs` (21 of 25 lines). The next one
up is `crates/globally_unique_identifier_partition_table/src/lib.rs` at 89.7%. So a floor of 85
newly fails one file, and a floor of 90 newly fails two. Raising the floor is not a backlog; it is
two files. Acting on that is calef's call.

### The count at 90 is platform-sensitive

The platform that gates is CI. The same run on the dev Mac later that evening (cargo-llvm-cov 0.8.7,
nightly-2026-09-23) agrees exactly on the minimum: 84.0%, the same file. It agrees on the aggregate
to two decimals, 95.21 against CI's 95.2.

It disagrees on one file. `crates/globally_unique_identifier_partition_table/src/lib.rs` is 315 of
351 lines (89.7%) on CI and 318 of 351 (90.6%) on the Mac. Three lines straddle 90. That moves the
85-89.9% band from one file to none, and the answer at a floor of 90 from two files to one. A floor
of 90 would fail a build on CI while passing for the person asked to fix it, the worse of the two
directions. The 2026W36 control prices this class of platform gap at 33 lines. It shows here only
because a single file sits on the boundary. A floor of 85 has no such ambiguity on either platform.

### A higher floor is not automatically better

This panel must not be read as a target. `script/coverage` says the floor is a floor: a file at 81%
is not "done", and 100% is not the goal because tests have to prove something. `AGENTS.md` says not
to add filler tests. A floor raised above what the tree has earned pushes a lane toward writing
whatever reaches the number. This tree has already caught three variants of a test that passes while
proving nothing. Watch the direction on this chart; a drop deserves more attention than the level.

### BUGS

- The series cannot be backfilled, for the reason under "The series starts at 2026W39".
- Nothing ties `floor.txt` to a commit, any more than it ties the lcov to one. `--coverage-min-from`
  believes the caller about which week it measured.
- The cell is carried across a `--backfill` like the aggregate. After a row is repointed at a later
  commit of the same week, the minimum describes the earlier commit until someone re-measures.
- The minimum is one file. Two files at 81% and one at 81% draw the same bar. `floor.txt`'s band
  counts show the difference; the chart does not.
