# The unsafe census, week by week and by trust boundary

*An appendix to [the register of measures](../register-of-measures.md). The name is provisional.*

## unsafe blocks outside `kernel/src/arch/`

Blocks per 10,000 code lines, outside `kernel/src/arch/`, which is `script/lint`'s census and the
one number in this tree that is actually gated on a direction. The dashed line is the ceiling.

227, 243, 138, 121, 111, 93, 77, 77. **The absolute count of unsafe blocks outside `arch/` went from
171 to 704 over the same period while the density fell by two thirds.** Both are true and only the
second one is about soundness: the first is a system being built. That is why the gate holds a
ratio.

The one rise is 2026W29 to 2026W30, and it is the honest shape of an early kernel: the tree was
7,500 lines and one driver moved the number.

### How much this particular series has actually been checked

This is the one place on the page where a number has been checked two different ways, so it is worth
separating what that bought.

**A consistency check, which is what comparing against `notes/unsafe-obligations.md` is.** That note
carries a seven-point table, 227.8 down to 78.8, taken on seven different dates. It is *not* an
independent record: it is the output of this same derivation, written down seven times by lanes.
Reproducing it proves the walk agrees with what was recorded, and nothing more. It did agree. On
2026-08-18 the reconstruction hits **747 blocks over 80,359 lines exactly**, at commit `93607fa4`,
which is both figures to the digit. `unsafe impl Send`/`Sync` and the inside-`arch/` count match the
recorded value at five of the seven dates, and are within three at the other two. The residual is
which commit inside the day the column was taken at, not a difference in definition. The 2026-08-23
column is the worked example: the table prints 777 and this reconstruction prints 799, and the note's
own prose already says the count immediately before that lane's reduction "was 799, not 777", over
85,476 lines, which is this reconstruction to the digit.

**An independent check, which is a different thing.** A second counter was written from scratch for
this: a character scanner that tracks Rust's real lexical states rather than blanking comments and
literals with one regular expression. It differs deliberately in two places the regex is loose:
block comments **nest** in Rust and the regex's do not, and a lifetime `'a` is not the start of a
character literal. Run over every in-scope file at `705e3919`, the two methods agree on **701 blocks outside `arch/`
and 253 inside, with zero files disagreeing** (that commit's own row; the number moved with the tree
afterwards). That is the check worth having, and it says the shortcuts in `script/lint`'s regex do
not bite in this tree.

## The same unsafe blocks, by trust boundary

**The chart above this one mixes two populations that mean opposite things, and this is the same
census with that mixed once and for good.** `unsafe_outside_arch` (824 today) adds every `unsafe`
block that is not in `kernel/src/arch/`, kernel and userspace both, into one number. In a capability
microkernel that is not a detail: an `unsafe` block inside `kernel/src` runs with nothing confining
it, and an `unsafe` block in a userspace program (`components/`, `fixtures/`, or a crate that ships
only into one of them) is confined by the same MMU-plus-capability-table mechanism that confines
every other program. A single density cannot answer "how much of the code that matters for isolation
is unsafe", because it never separates the two populations that question is about. Raised as exactly
this problem by a research lane on 2026-09-20 (`notes/trusted-base.md`, landing alongside this
section, written for the RedLeaf comparison; see `notes/redleaf.md`), which hand-computed a first cut
and flagged in its own `BUGS` that nothing kept it computed. This split is that mechanism.

**Today, 2026-09-20: 694 kernel, 382 userspace, 19 shared, 37 boot chain, 0 unclassified**, which
sums to 1,132, not 1,138 (824 + 314). The 6-block difference is not the split disagreeing with the
old census; it is a separate, smaller correction this pass found while classifying `crates/`: three
crates (`board_console`, `portable_executable`, `stick_maker`) are host tooling that runs on the
developer's Mac and never on nife, exactly like `bench/host/`, `xtask/` and the rest of
`unsafe_census`'s own `HOST_ONLY` exclusion, just not under one of `HOST_ONLY`'s path prefixes; each
says so in its own header. `unsafe_outside_arch`/`unsafe_density` above are left untouched by this
finding (they still mean exactly what they meant, and `script/lint`'s ceiling still gates the same
number it always has); this split simply does not count those three crates' 6 blocks toward either
side, because the question this split answers, kernel privilege or userspace confinement, has no
answer for code that runs on neither. Whether `unsafe_census`'s own `HOST_ONLY` should widen to match
is calef's call, recorded rather than made here.

**What each bucket is**, and the boundary decisions behind it are in `scripts/rust_source.py`'s own
comment on `trust_boundary_census`:

- **kernel** (694 blocks, 48,724 code lines, density **142** per 10,000): `kernel/src/**` (arch and
  not) plus the sixteen `crates/` members reachable, over a real `cargo metadata` dependency edge,
  **only** from the `kernel` package: `paging` and `dma_validator` among them, both lifted out of
  `kernel/src` on purpose so Kani could reach them. Counting only `kernel/src/**`, as the hand
  computation in `notes/trusted-base.md` does, misses these sixteen crates entirely: its 577 is
  *undercounting the trusted base by the 117 blocks those crates carry*, which is worth flagging to
  that note's own lane before it lands, since its `BUGS` section already names this exact risk
  ("a tree can shrink [the kernel line count] by moving code out of `kernel/src` without reducing
  what anyone has to trust") without checking whether it had happened to its own number.
- **userspace** (382 blocks, 31,639 code lines, density **120** per 10,000): `components/`,
  `fixtures/`, the sixteen `crates/` members reachable only from them, and five packages that are
  each their own cargo workspace and never appear in the main one (`redoxfs_server`'s `el0` build,
  `std_exerciser`, `entropy_backend`, `cryptography_exerciser`, `cryptography_provider`); every one
  of them is, by its own header, a program or library that runs on nife at EL0 and never as kernel
  code.
- **shared** (19 blocks, in 5 of 36 crates reachable from both sides): code that genuinely executes
  with kernel privilege in the kernel binary and, separately, under confinement in a userspace
  program. Not a hedge: `environment_protocol`'s `ConfigPage` and `clock_protocol`'s `ClockPage` are
  built by the kernel's `unsafe fn new`/`from_raw_parts` and read back through the identical
  accessor by a userspace `std` program, so the same unsafe source is real in both roles. Folding it
  into either side would overcount one and undercount the other; telling apart, per call site,
  whether a specific block also runs from the kernel's own `#[cfg(test)]` oracles (several of these
  crates' `Cargo.toml` comments say their kernel-side use is exactly that, a test predicting what a
  client sent, and would never ship) is a source-level read this pass did not do and is recorded
  as future work.
- **boot chain** (37 blocks): `uefi_loader` and the one crate only it reaches (`sealed_pair`). It
  runs once, before the kernel starts, with the full privilege of the pre-OS environment, to decide
  which kernel image gets control, and its memory is gone by the time the kernel's isolation
  boundary exists to enforce anything. That is a chain-of-trust question, not a runtime-isolation
  one, so it is reported on its own rather than folded into either of the other two; which claim it
  backs is calef's to decide.

**Which number the ceiling should be held against.** `script/lint`'s `<!--count-at-most:unsafe-
density-outside-arch-->` (`notes/unsafe-obligations.md`) holds the mixed density (currently 88
against 77) and this pass does not change that marker's value; a gate's threshold is calef's. What
this pass can say is what the ceiling would mean under each candidate: **142** if held against the
kernel-only density, which is the number that answers "how much of the code nothing confines is
unsafe"; **120** against the userspace density, the confined population, where a ceiling matters
less because a bug there is a bug in one program, not in the base; or **77**, the status quo, which
answers neither question precisely because it is built from both. Recommendation: the kernel density
is the one worth a ceiling of its own, because it is the population where the mixed number's blind
spot actually lives, but a ceiling set on it starts cold (no history of it moving deliberately) and
that is a decision for calef to make with these numbers in hand, not one this pass makes for him.

**The history, backfilled, and where it is honest about a gap rather than papering over one.**
`script/metrics --backfill` restated the whole series with this split, the same way milestone 448
restated `SUPERSEDED`/`REFUSED` into weeks already written. Of the ten weeks recorded before
`--backfill` ran, **eight (2026W29 through 2026W36, then W37 too) carry a nonzero
`unsafe_trust_unclassified`** (57 up to 188 blocks), because this tree spells its crate names out and
most of them did not always have their current spelling: `crates/ipc`, `crates/dtb`, `crates/asid`
and around forty more were renamed to `inter_process_communication`, `device_tree_blob`,
`address_space_identifier` and so on over the weeks this series covers, and the classification table
in `scripts/rust_source.py` is built from today's names (the same restatement trade
`MILESTONE_STATUSES`/`NAME_STATUSES` already make, stated in that file's own header). **This was not
reconstructed for the same reason a shortcut was refused rather than taken**: `git log
--diff-filter=R --summary` finds candidate renames, but at least one pairing it offers is wrong
(`crates/canary_gate` paired with `crates/work_steal_slot` by a `Cargo.toml`-only content match,
while `canary_gate`'s own `src/lib.rs` correctly pairs with `memory_corruption_canary_gate`), and a
hand-verified alias table for around forty old names, several of them (`mdns_proto`, `smb_proto`,
`ntlm`) naming protocols since removed outright (SMB and Time Machine are out of this project's
customer path entirely, per `AGENTS.md`) with no current bucket to map to at all, is real work this
pass chose not to rush. **From 2026W38 (2026-09-20) the split is exact: `unsafe_trust_unclassified`
reads 0.** A future lane rebuilding that alias table, verified file-by-file rather than trusted from
`--summary` alone, is the way to close the gap; until then, read the early weeks' kernel/userspace
bars as undercounts of both sides by whatever their `unclassified` band carries, exactly the caution
this page already asks for `names_no_block` in the weeks before naming provenance existed.

**That backfill was lost for two days and restored 2026-09-23.** The `unsafe_trust_*` columns
above landed on `main` at 2026-09-20 23:30 (`3764a78d3`) with the ten weeks above already backfilled,
matching this section's own numbers. `weekly.csv` was then the single series file; milestone 581
(one metrics file per measure) has since retired it. The next commit to touch `weekly.csv`
(`6e8934474`, "Rebuild the weekly series after the merge", 2026-09-21) was resolving a conflict
between two branches that had each added columns to the same file, and its message says the plan
was to restore its own branch's cost columns and "let `script/metrics` add the trust-boundary
columns from the merged script." What ran next was `--update`, not `--backfill`: `--update` only
recomputes the current week and any week missing outright, so it added the ten new column headers
to every row but left `unsafe_trust_kernel`, `unsafe_trust_kernel_code_lines`,
`unsafe_trust_kernel_density`, the userspace triple, `unsafe_trust_shared` and
`unsafe_trust_boot_chain` **empty, not zero**, for 2026W29 through 2026W38, which is why
`unsafe-trust.svg` drew one point instead of ten from 2026-09-21 until this was found and
`script/metrics --backfill` was rerun on 2026-09-23. The nine restored values match the
2026-09-20 backfill exactly (the `unclassified` counts for 2026W29 through 2026W37 are the same
57-to-188 figures this section already names, and 2026W38 is still exactly 0), so this is the same
known rename-alias gap reappearing rather than a new one. **The lesson is about the merge, not the
census**: any column added by one branch while another branch is independently adding columns to
the same CSV needs a `--backfill` after the conflict is resolved, because `--update` treats every
already-present row as already correct and will not notice that a resolved merge just introduced
blank cells into it. Checked for the same shape elsewhere on 2026-09-23 and found nowhere else:
every other column that could go this route (`milestones_superseded`, `milestones_refused`, and
the fatal-risk columns, then `fatal_risks_tested` and `fatal_risks_untested`) was in fact backfilled
at the commit that added it,
and the flow columns (`milestones_built_this_week`, `merged_pull_requests`, the four cost columns)
are recomputed for every row on every run regardless of mode, so they cannot hold a stale blank this
way.

## BUGS

- **`patches/std-nife/overlay/` is outside the unsafe census, and it is our code.** Thirty-seven
  `unsafe {}` blocks in the `std` platform layer are counted by nothing here. Two separate reasons,
  and only the first is a decision: a ceiling asserts a direction, and that code implements `std`'s
  internal interfaces, so it cannot be restructured to hold fewer unsafe blocks without diverging
  further from the crate we track. The second is worse and is not a decision at all: **that code is
  compiled into `std` by the farm and never by a clippy configuration here**, so
  `undocumented_unsafe_blocks` and `unsafe_op_in_unsafe_fn` do not reach it either. Fifteen of its
  blocks have no `SAFETY:` comment in the form the lint wants, and nothing has ever said so. That
  is a coverage hole in the lint policy rather than a gap in this register, and it wants a lane.

- **Unsafe density can be diluted by writing more safe code, and nothing stops that.** The
  denominator is non-blank lines after comments and string literals are stripped, so prose cannot
  move it, but a verbose safe refactor can. The counter-argument is that the effect is small at
  80,000 lines and that the alternative, a raw count, was measured and is worse. Watch the printed
  numerator, which `script/lint` prints beside the ratio for exactly this reason.

- **The unsafe derivation is a text scanner, not a parser.** It blanks comments and literals with a
  regex before matching keywords, which is what keeps fourteen `unsafe {}` written inside doc
  examples out of the count. Block comments are matched non-greedily and Rust's nest; the tree has
  no nested ones, and a nested one could only make the count too high, which fails loud. Same caveat
  as `script/lint`'s `# Safety`, dead-code and `#[path]` checks, which are built the same way.

- **Nothing here measures the verification argument, and nothing can.** Unsafe density says how much
  code is outside the compiler's guarantees; it says nothing about whether the invariants written in
  the `SAFETY:` comments are true. §61 already records that a lint checks a comment exists and never
  that it is right. A register of numbers is not a substitute for reading them.
