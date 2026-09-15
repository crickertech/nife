# The icount baselines predate the pinned nightly, so the tripwire's headroom is eroding unseen

**Status: PROPOSED 2026-09-15.** Surfaced by milestone 299's lane while fixing an icount regression:
the leak it fixed was real, but investigating it showed `main` itself already sits over the committed
baselines, from a cause unrelated to 299.

**Gate: DECISION.** Re-saving a benchmark baseline commits a new performance floor the tripwire
measures against, which is a fact the tree trusts; it is calef's, the same as any baseline save.

## The finding, verified

`bench/baseline-{aarch64,riscv64,x86_64}.txt` are the icount regression tripwire's committed floors.
PR #880 (2026-09-15) bumped the pinned toolchain to `nightly-2026-09-15` and changed **only**
`rust-toolchain.toml` (verified: one file, one line). A new nightly emits different instruction
sequences, so the deterministic icount numbers moved, and the baselines were **not** re-saved.

Measured 2026-09-15 by 299's lane against `origin/main`:

- **aarch64** and **riscv64** `yield_switch`/`ctx_switch` sit **~+6.5%** over their committed
  baselines, purely from the nightly (299's code adds nothing to those paths, confirmed byte-identical).
- **x86_64** sits **+5-8%** over its baseline on paths the port capability never touches
  (`ipc_rtt`, `relay_rtt`, `spawn_reap`), while `coremark` (pure compute) is flat.

The tripwire fires at +10%. So every architecture is now spending most of its headroom on toolchain
drift, and the next unrelated change that adds a few percent will trip the gate **for a reason that
is not its own** — which is the failure this proposal exists to prevent, not a hypothetical.

## What to do

1. **Re-measure the whole tree on `nightly-2026-09-15` and re-save all three baselines** in one pass,
   so the tripwire's headroom measures real regressions again rather than the toolchain. This is the
   decision (a new floor is committed).
2. **Make a toolchain bump re-baseline, or refuse to be silent about it.** `toolchain-bump.yml`
   changed the nightly without touching the baselines the change invalidates. The recurring fix is
   that a bump either re-saves the baselines in the same PR (so the floor tracks the toolchain) or
   fails a check that says "baselines are stale for this nightly," rather than leaving the erosion to
   be discovered by a lane chasing an unrelated regression months later.

## BUGS

- **A blanket re-save hides a real regression that happened to land in the same window.** The honest
  re-baseline reads the diff per metric: a uniform ~+6.5% across unrelated benchmarks is the
  toolchain; a single benchmark far above that is a regression to keep, not to bless. 299's lane
  already used exactly this reasoning to separate its own x86 cost from the drift.
- **This is measured on one machine, one nightly.** The numbers above are TCG icount on the dev
  Mac's CI-arm runners; a different runner or nightly moves them again, which is the whole reason a
  bump should re-baseline rather than a human eyeball a percentage.
