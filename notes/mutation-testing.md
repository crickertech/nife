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
- **`script/mutation --report`'s `(baseline missed)` column is not "last week", it is
  `.cargo/mutants-baseline.txt`, one fixed run from 2026-08-03**, recorded 2026-09-23 by milestone
  512 (the census blamed one pull request for 55 survivors it did not write); the trap is also in
  `script/mutation`'s own comments, beside the column. Read as a recent delta, it reads six weeks
  of growth as however many days happen to sit between the reader and the last thing that touched
  the crate. That is exactly what happened once: `design/fatal-risks.md`'s risk 3 blamed milestone
  319 (the crate that parses firmware)'s pull request for `machine_discovery` going from 22
  survivors to 77 because the crate had just been proved and the census ran two days later.
  Replaying `cargo mutants --in-diff` against that pull request found 4 survivors; the crate
  already carried 73 on the commit before it merged. The column cannot distinguish "22, six weeks
  ago" from "22, two days ago", and nothing else in the tree recorded the crate's history until
  milestone 518 (a census that cannot be attributed)'s per-crate census existed to compare against
  instead.
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

## Appendices

| appendix | what it holds |
|---|---|
| [baseline-2026-08-03](mutation-testing/baseline-2026-08-03.md) | The 2026-08-03 baseline, the calibration, and the recurring patterns |
| [baseline-survivors-abi-to-ipc](mutation-testing/baseline-survivors-abi-to-ipc.md) | The baseline survivors, crate by crate: `abi` to `ipc` |
| [baseline-survivors-grant-plan-to-swish](mutation-testing/baseline-survivors-grant-plan-to-swish.md) | The baseline survivors, crate by crate: `grant_plan` to `swish` |
| [baseline-ledger](mutation-testing/baseline-ledger.md) | The baseline ledger: every survivor's disposition |
| [the-first-weekly-censuses](mutation-testing/the-first-weekly-censuses.md) | The first weekly runs: the 2026-09-03 sample and the 2026-09-14 census |
| [measured-boot](mutation-testing/measured-boot.md) | `measured_boot`: five survivors proved equivalent |
| [uefi-loader](mutation-testing/uefi-loader.md) | `uefi_loader`: a score measured over a file nothing compiled |
| [a-memory-bound-per-mutant](mutation-testing/a-memory-bound-per-mutant.md) | A memory bound per mutant |
| [documentation](mutation-testing/documentation.md) | `documentation`: a crate scored with a third of its tests compiled away |
| [regressions-capability-to-dtb](mutation-testing/regressions-capability-to-dtb.md) | The 2026-09-14 regressions: `capability` to `dtb` |
| [regressions-clock-protocol-swish-filesystem-protocol](mutation-testing/regressions-clock-protocol-swish-filesystem-protocol.md) | The 2026-09-14 regressions: `clock_protocol`, `swish`, `filesystem_protocol` |
| [machine-discovery](mutation-testing/machine-discovery.md) | `machine_discovery`: 77 survivors |
| [new-crate-backlog](mutation-testing/new-crate-backlog.md) | The new-crate backlog: a loop that waits |
| [job-mix-and-jh7110-entropy](mutation-testing/job-mix-and-jh7110-entropy.md) | `job_mix` and `jh7110_entropy` |
| [board-console](mutation-testing/board-console.md) | `board_console` |
| [video-terminal](mutation-testing/video-terminal.md) | `video_terminal` |
| [clock-and-reset-nvme-screen-console](mutation-testing/clock-and-reset-nvme-screen-console.md) | `jh7110_clock_and_reset`, `non_volatile_memory_express`, `screen_console` |
