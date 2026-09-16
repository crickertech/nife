# 300. Decompose the icount baseline drift, and re-baseline only what is proven

**Status: BUILT.** 2026-09-15. *(Number provisional until the merge queue lands it.)* Re-saving a
baseline commits a new performance floor, which is calef's call like any baseline save; this
milestone was briefed with that latitude and executes Decision 1 of the finding below. Decision 2
stays open for calef as the follow-on.

This milestone promotes and carries the proposal *"the icount baselines predate the pinned nightly"*
(surfaced by milestone 299's lane, PR #883). The finding, verbatim, then what the decomposition
found, which corrected the finding's own hypothesis.

## The finding, as surfaced

`bench/baseline-{aarch64,riscv64,x86_64}.txt` are the icount regression tripwire's committed floors.
The aarch64 and riscv64 baselines were last saved 2026-08-27 (`a79fdb95`). Over the following ~19
nightly bumps and ~150 commits, all individually under the 10% tripwire, `main` drifted to ~+6.5%
over those two baselines on the switch-heavy benchmarks. The tripwire fires at +10%, so most of the
headroom was spent, and the next unrelated change adding a few percent would trip the gate for a
reason that is not its own. The proposal read this as the pinned nightly's codegen (PR #880 bumped
to `nightly-2026-09-15` changing only `rust-toolchain.toml`, without re-saving the baselines a new
nightly's codegen was assumed to invalidate) and asked for two things:

1. Re-measure and re-save the baselines on the new nightly (the decision, a new floor committed).
2. Make a toolchain bump re-baseline in the same PR, or fail loudly, so this stops being found by
   accident.

## What the decomposition found

The premise in (1) that the drift was the nightly's codegen turned out to be **false**, and proving
it was the point. Holding QEMU at 11.1.1 and measuring a 2x2x2 grid of {baseline code, HEAD code} x
{nightly-08-27, nightly-09-15}, plus the QEMU term (the dev Mac moved 11.0.2 -> 11.1.1 on 2026-08-28,
after the baseline was saved), splits the drift into three:

- **QEMU (11.0.2 -> 11.1.1): ~0.** The emulator upgrade does not move icount. Measured, not assumed.
- **Toolchain (nightly-08-27 -> nightly-09-15): ~0.** The two endpoint nightlies emit byte-identical
  instruction counts on the same code. The nightly did nothing across this window.
- **Code (a79fdb95 -> HEAD): the entire move,** and the same move on both ISAs (near-identical
  percentages, which codegen noise would not produce).

`git bisect` pinned the code component to **one commit**: `57399c34` (2026-09-02), *"sched: write the
cycle-counter grant at the context switch"*, **milestone 139 / DECISIONS 139 option 4**. It adds a
per-context-switch grant write: `yield_switch` and `ctx_switch` move by the identical +43.3
ticks/switch, and every IPC benchmark moves in proportion to its switch count, while pure compute
(`coremark`) and the no-switch map primitives do not move. Milestone 299's `44890a8a` later trimmed
the aarch64/riscv peak to the net ~+35.7 ticks/switch at HEAD.

139 is a decided feature and its own text places the cost *"at the context switch"*, so this is
intended work, not an unexplained regression. The baselines are re-saved on `nightly-2026-09-15`
with the drift attributed to 139. The full per-benchmark grid, the bisect, and the estimate-vs-actual
flag below live in [notes/benchmarks.md](../../notes/benchmarks.md) under the 2026-09-15 heading.

**x86_64 was already current** (milestone 299 re-saved it 2026-09-15); `bench --x86 --check` returns
byte-identical numbers here, so it was sanity-checked, not re-measured.

## What was built

- `bench/baseline-aarch64.txt` and `bench/baseline-riscv64.txt` re-saved on `nightly-2026-09-15`,
  QEMU 11.1.1, so the committed floor tracks the current toolchain and the ~+6.5% headroom is
  restored. `bench --check` passes on all three architectures against the re-saved floors.
- The per-benchmark decomposition (baseline / QEMU / code / toolchain components) recorded in
  `notes/benchmarks.md`, with the milestone-139 attribution and the bisect that proved it.
- The `rust-toolchain.toml` pin is unchanged at `nightly-2026-09-15` (the old-nightly pin used for
  measurement was never committed).

## Follow-on

- **Proposed.** Decision 2 of the finding, calef's and deferred: make a toolchain bump re-baseline
  or fail loudly. `script/toolchain-bump` raises the pinned nightly without touching the baselines,
  and this milestone proves the drift they leave is real (even where, as here, the culprit turned out
  to be code rather than the nightly). Written up as
  `design/roadmap/proposals/a-toolchain-bump-that-leaves-the-baselines-stale.md`, since it is a
  workflow change calef owns and this milestone deliberately did not touch it.
- **Recorded.** DECISIONS 139 estimated its switch cost as the compare `switch_user_root` already
  pays (~2-3 ticks when nothing is granted) and delivered ~+35.7 ticks per context switch, an order
  of magnitude over; the likely cause is a non-inlined arch function on the hot path. Not a
  correctness bug and legitimately decided, so it is in the re-saved baseline rather than held out,
  but whether the grant should inline to the promised compare wants a look. The measurement and the
  arithmetic are beside the feature in `notes/benchmarks.md` (the 2026-09-15 section).

## BUGS

- **A re-baseline commits a floor measured on one machine, one nightly, one QEMU.** These numbers are
  TCG icount on the dev Mac. A different runner moves them again, which is the whole reason the
  follow-on argues a bump should re-baseline mechanically rather than have a human eyeball a percent.
- **The QEMU term was measured out, not eliminated.** Only 11.1.1 is installable locally, so the
  11.0.2 baseline could not be re-run on its original emulator; the QEMU component was isolated by
  holding 11.1.1 across the code and nightly axes and reading the same-code/same-nightly pair (B - A),
  which came back ~0. If a future QEMU does move icount, this method still separates it.

## Index row

**Built:** 2026-09-15

Decomposed the ~+6.5% icount drift over the 2026-08-27 baselines into QEMU (~0), toolchain (~0), and
code components, refuting the proposal's nightly-codegen hypothesis: the whole move is one decided
feature, milestone 139's cycle-counter grant at the context switch (+35.7 ticks/switch, bisected to
`57399c34`). Re-saved the aarch64 and riscv64 baselines on `nightly-2026-09-15` with the cost
attributed; x86_64 was already current. Decision 2 (a toolchain bump must re-baseline or fail loudly)
left open as the follow-on.
