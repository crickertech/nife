# The icount baselines predate the pinned nightly, so the tripwire's headroom is eroding unseen

**Status: PROPOSED 2026-09-15.** Surfaced by the lane of milestone 299 (the x86 port-range capability) while fixing an icount
regression:
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
is not its own**, which is the failure this proposal exists to prevent rather than a hypothetical.

## What to do

1. **Re-measure the whole tree on `nightly-2026-09-15` and re-save all three baselines** in one pass,
   so the tripwire's headroom measures real regressions again rather than the toolchain. This is the
   decision (a new floor is committed).
2. **Make a toolchain bump re-baseline, or refuse to be silent about it.** `toolchain-bump.yml`
   changed the nightly without touching the baselines the change invalidates. The recurring fix is
   that a bump either re-saves the baselines in the same PR (so the floor tracks the toolchain) or
   fails a check that says "baselines are stale for this nightly," rather than leaving the erosion to
   be discovered by a lane chasing an unrelated regression months later.

## And it has recurred since, which settles the second half of the fix

The proposal was written against `nightly-2026-09-15`. `rust-toolchain.toml` today pins
**`nightly-2026-09-20`**. So the toolchain moved again in the six days this proposal spent on a
branch, and the baselines were **again** not re-saved. That is the recurrence the fix's item 2
predicts, observed rather than argued, and it is why "re-measure once" is the smaller half of the
answer: without a check that a bump re-baselines or says it did not, the erosion resumes the next
time somebody bumps.

## The same cause has now eroded a second gate, measured 2026-09-21

**`script/fastpath-footprint` is over its baselines too**, and it was found independently, by the
lane building a thread's own CPU page. On `main`, with nothing of that lane's in the tree:

| metric | over baseline |
|---|---|
| `ipc_send_recv` | **+1.0%** (6300 against 6236) |
| `ipc_call_reply` | **+1.4%** (8234 against 8122) |
| `syscall_entry` | **+3.9%** (1701 against 1637) |

`syscall_entry` is the one to read. It is the flattest path in the kernel, it has no closure, and a
lane that added a store to the switch path measured it **byte-identical** under its own change. So
the 3.9% is not anyone's feature; it is the same toolchain drift this proposal was written about,
showing up as bytes instead of instructions.

**That matters more than the icount case, because the bound is tighter.** The icount tripwire fires
at +10% and the footprint bound is 5%, so `syscall_entry` has already spent **78% of its headroom**
on the compiler. The next change that costs it 1.1% trips a gate for a reason that is not its own,
which is precisely the failure milestone 415 (sub-tripwire drift accumulates across baseline saves)
records for the bench tripwire, now observed on a third gate.

**So the decision is one decision, not two**, and any re-save should cover `bench/baseline-*.txt`
and `bench/fastpath-*.txt` in the same pass, on the same nightly, with the per-metric read the
`BUGS` section below already requires.

## Why this proposal was hard to find, which is its own finding

It was written on 2026-09-15 and lived on an **unmerged branch** until 2026-09-21. `AGENTS.md` says
an unmerged branch is either abandoned or holding knowledge that is not on `main`, and that the
second case is a bug in where the knowledge lives: *"Nobody reads branches."* In those six days a
second gate eroded and a second lane rediscovered the same cause from scratch. The proposal was
correct the whole time and invisible the whole time.

## BUGS

- **A blanket re-save hides a real regression that happened to land in the same window.** The honest
  re-baseline reads the diff per metric: a uniform ~+6.5% across unrelated benchmarks is the
  toolchain; a single benchmark far above that is a regression to keep, not to bless. 299's lane
  already used exactly this reasoning to separate its own x86 cost from the drift.
- **This is measured on one machine, one nightly.** The numbers above are TCG icount on the dev
  Mac's CI-arm runners; a different runner or nightly moves them again, which is the whole reason a
  bump should re-baseline rather than a human eyeball a percentage.
