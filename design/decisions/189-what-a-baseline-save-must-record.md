# 189. Must an icount baseline save record why it moved, and does a second fixed anchor earn its cost?

**Status: PROPOSED.** Raised 2026-09-19 by milestone 435's slice-c lane, which found milestone 415's
`DECISION` gate naming no section and its gate prose gone stale. The audit behind it is the
baseline-audit lane's, 2026-09-15, which calef asked for after PR #886 found a regression that had
hidden under the 10% threshold. *(Section number provisional until the merge queue lands it.)*

## What is being decided

Milestone 415 named four items. **One has landed and one is refused, so this section is about the
two that are left:**

- **Item 2.** Require `cargo xtask bench --save` to carry a reason and write it into the baseline
  file next to the rows that moved, refusing to write without one.
- **Item 3.** Keep a second per-architecture file holding a fixed historical anchor, and fail when
  today's number drifts more than some bound from **it**, in addition to the last-floor check.

## Is the premise true

Checked 2026-09-19 in this worktree, item by item, because milestone 415's gate paragraph no longer
describes what is open.

- **Item 1 landed.** `script/ci-build:122` reads
  `bench|ci|script/bench --check && script/bench --riscv --check && script/bench --x86 --check`, so
  the x86_64 leg is gated. That was commit `ba99c83` on 2026-09-15, in its own commit as the section
  asked. **The gate's clause "the first item is a one-line CI change that is owed already and needs
  nobody's permission" is therefore stale**, and is corrected in milestone 415's block by this
  lane.
- **Item 2 is not built.** `xtask/src/main.rs:7460` parses `--save` as a bare flag beside `--check`
  and `--release`, takes no reason, and `bench/baseline-x86_64.txt`'s header says only that
  *"updating this file is a statement that a performance change is intended and understood; do it in
  the commit that causes the change."* So the only record of why a number moved is still the commit
  message.
- **Item 3 is untouched.** `bench/` holds one baseline file per architecture (`baseline-aarch64.txt`,
  `baseline-riscv64.txt`, `baseline-x86_64.txt`) and no anchor file.

## The structural hole, measured

`--check` fails at more than 10% drift **against the last saved baseline**, and `--save` rewrites
that baseline, so N successive sub-threshold steps accumulate and the gate never fires. The audit in
`notes/benchmarks.md` walked every `--save` event in the history of all three files, from git alone
with no emulation:

| | cumulative drift | largest single step | times the gate fired |
|---|---:|---:|---:|
| riscv64 `ctx_switch` | **+10.78%** | +6.14% | 0 |
| aarch64 `yield_switch` | +9.16% | +6.49% | 0 |
| riscv64 `ipc_rtt` | +9.25% | +4.34% | 0 |
| aarch64 `ipc_rtt` | +8.73% | +4.95% | 0 |
| x86_64 `yield_switch` | +9.94% | +9.94% | 0 |

`coremark`, pure compute with no context switches, is flat to four decimal places across every save
on all three architectures, which is what says the rest is the kernel's switch and IPC paths rather
than measurement noise.

**Most of that accumulation is honest**, disclosed in the commit that caused it. The problem is that
nothing in the tree can tell the honest part from the rest, **and two saves on 2026-09-15 prove
it**: `44890a8a` re-saved the x86_64 baseline attributing +5 to +8% to the toolchain, milestone 300
then measured that toolchain term across exactly those two nightlies at **~0**, and PR #886 found
the real cause and recovered ~5.9% by deleting a const-`false` element still threaded through the
shared context-switch tuple. `85edb1ed` did the same on the other two architectures the same day.

This is the move milestone 237 exists to refuse, one instrument over: re-saving because the number
moved, which launders growth into the new normal and retires the gate that was supposed to catch it.

## What this tree already does in the analogous case

**Rung three: the record goes beside the thing a reader meets.** That is milestone 115's shape and
§75's, and item 2 is exactly it. Today the attribution is rung four, a commit message read once by
one person on the day it is written; both 2026-09-15 mis-classifications sat in commit messages that
nobody re-read until a lane went looking six weeks later.

**A gate that fires on correct work gets dropped.** §61 adopts a lint on evidence from this tree
rather than on its description, and milestone 78 dropped checks on the same ground. That is item 3's
real cost, and it is not the code: **the anchor goes stale for correct reasons.** Milestone 139 is a
decided feature with a real cost at the switch, and `spawn_el0` legitimately fell **32.7%** on both
ISAs when `b918337b` bounded a walk by occupancy. An anchor with no ledger of intended deltas fires
on both.

**And the threshold itself is already decided.** Milestone 25 demoted `--check` from a 2% gate to
the coarse 10% tripwire deliberately, because adding unrelated live code moves untouched benchmarks
several percent non-uniformly through whole-crate inlining decisions; the audit re-confirms it, since
`9890eb02` moved every kernel-side IPC row by 4 to 8.5% by adding one benchmark. **Item 4,
tightening the threshold, stays refused**, and it would not have caught either 2026-09-15 step, both
of which were real costs honestly measured and wrongly explained.

## The question this section actually puts to calef

**Item 2 may not need a ruling at all, and that should be settled rather than assumed.** Milestone
415's own reversibility paragraph says items 1 and 2 are undoable in an afternoon and nobody outside
this tree has acted on them, which by AGENTS.md's test makes item 2 a decision for whoever is
holding the problem. The counter-argument, and the one the gate rests on, is that **it changes what
a person has to type on every bench evening on every board**, which is calef's own workflow rather
than a lane's.

So: **may a lane build item 2 without further ruling?** If yes, milestone 415's gate becomes
`NONE` for that item and item 3 is what remains open. The lane that wrote this section did not
correct the token on its own reading, because getting that wrong puts a block on
`script/roadmap --ready` where a lane stalls on the fork, which is the worse of the two failures.

## Recommendation

**Do item 2, and hold item 3 until item 2 has been in the tree long enough to say whether the
attributions it collects are good enough to gate on.** Item 2 is most of the ledger item 3 would
otherwise have to invent, which is why it comes first rather than because it is smaller.

**The honest limit of item 2, stated so it is not oversold**: it cannot make an attribution true.
`44890a8a` would still have written its false toolchain reason. What changes is that the false
reason is then in the file the next reader opens, where the audit above would have found it in a
grep instead of a bisect. That is rung three rather than rung one, and this section does not claim
otherwise.

## How reversible, and who has acted on it

**Item 2 is an afternoon and nobody outside this tree has acted on it.** Item 3 writes a second
committed floor that every future measurement is compared against, which is a fact other work starts
depending on, so it is the one worth deliberating.

## What is blocked until this is answered

**Milestone 415's remaining two items.** Nothing else: item 1 is gated and the floor it checks
against is the recovered one, because PR #886 had already recovered the ~5.9% x86_64 regression and
re-saved that baseline before `ba99c83` landed.
