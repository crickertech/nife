# Sub-tripwire drift accumulates across baseline saves, and one architecture has no gate at all

**Status: PROPOSED 2026-09-15.** Written by the baseline-audit lane, which calef asked for after
PR #886 found a regression that had hidden under the 10% threshold.

**Gate: DECISION.** Two of the four items change how `cargo xtask bench --save` behaves and what a
save is obliged to record, which is a workflow calef owns; the first item is a one-line CI change
that is owed already and needs nobody's permission.

## In brief

`cargo xtask bench --check` fails at >10% drift **against the last saved baseline**, and `--save`
rewrites that baseline. So N successive sub-threshold steps accumulate and the gate never fires.
The audit in [notes/benchmarks.md](../../../notes/benchmarks.md) walked every `--save` event in the
history of all three baseline files, from git alone with no emulation, and the accumulation is real
and measurable:

| | cumulative drift | largest single step | times the gate fired |
|---|---:|---:|---:|
| riscv64 `ctx_switch` | **+10.78%** | +6.14% | 0 |
| aarch64 `yield_switch` | +9.16% | +6.49% | 0 |
| riscv64 `ipc_rtt` | +9.25% | +4.34% | 0 |
| aarch64 `ipc_rtt` | +8.73% | +4.95% | 0 |
| x86_64 `yield_switch` | +9.94% | +9.94% | 0 |

riscv64's `ctx_switch` is past the threshold the gate enforces, reached in steps none of which came
close to it. `coremark`, pure compute with no context switches, is flat to four decimal places
across every save on all three architectures, which is what says the rest is the kernel's switch and
IPC paths rather than measurement noise.

**Most of that accumulation is honest**, disclosed in the commit that caused it, and a good part of
it is whole-crate codegen churn the instrument cannot separate from real cost. The problem is not
that the drift exists. It is that nothing in the tree can tell the honest part from the rest, and
two saves on 2026-09-15 prove it.

## The live instance, which is not hypothetical

`44890a8a` re-saved the x86_64 baseline and attributed its +5 to +8% to the toolchain:
*"It also folds in the same nightly-2026-09-15 drift the other two carry."* Milestone 300 (decompose
the icount baseline drift) then measured that toolchain term across exactly those two nightlies and
found it **~0**, byte-identical counts on the same code. PR #886 names the real cause, a const-`false`
element still threaded through the shared context-switch tuple that the debug build does not fold,
and recovers ~5.9% on x86_64 by deleting it.

So the x86_64 floor carries a removable regression, blessed into the baseline on a stated cause that
measures zero, and `--check` passed the whole time because 5.9% is under 10%. `85edb1ed` did the same
thing on aarch64 and riscv64 the same day, classifying the +6.5% as intended feature cost where #886
shows 91 to 93% of it is removable.

This is the move milestone 237 (the cycle-counter grant costs 136 bytes of IPC fastpath) exists to
refuse, one instrument over: re-saving because the number moved, which launders growth into the new
normal and retires the gate that was supposed to catch it.

## What to do, cheapest first

### 1. Pull the x86_64 leg into CI. One line, owed already, needs no decision.

`script/ci-build`'s bench entry is `script/bench --check && script/bench --riscv --check`. The third
tripwire is built, its baseline is committed, and **nothing pulls it**; `ci.yml` already carries a
`BUGS` note saying so. The audit shows what that costs: x86_64 has had exactly two saves ever, and
the window between them is **1,526 commits**. On the two gated architectures a gross regression
eventually forces a save, so the record has granularity. On the third nothing forces one, which is
why its single step arrived at +9.94% wearing a false attribution.

Cost: one line. It may fail the first time it runs, which is why `ci.yml`'s note says it wants its
own commit rather than riding along with something else.

### 2. Make a save record its own attribution, beside the number. Recommended.

Require `--save` to carry a reason and write it into the file next to the rows that moved, refusing
to write without one. Today the only record of why a number moved is the commit message, which is
rung four of AGENTS.md's ladder: read once, by one person, on the day it is written. Both 2026-09-15
mis-classifications are in commit messages that nobody re-read until a lane went looking six weeks
later.

Writing the attribution into the baseline file moves it to rung three, beside the thing a reader
meets, which is milestone 115's shape. It also makes the wrong state visible rather than
unrepresentable, and that is the honest limit: a lane can still write a false reason, as
`44890a8a` did. What changes is that the false reason is then in the file the next reader opens,
where the audit above would have found it in a grep instead of a bisect.

Cost: roughly the `--save` writer plus a flag, and the rows it already knows have changed. Small.

### 3. A cumulative check against a fixed historical anchor. Real, and more expensive than it looks.

Keep a second per-architecture file holding a historical anchor and fail when today's number drifts
more than some bound from **it**, in addition to the last-floor check. This is the mechanism that
directly answers the structural hole, and the audit is the evidence that it would fire.

The cost is not the code, which is a second file and a second comparison. It is that **the anchor
goes stale for correct reasons.** Milestone 139 (drive the unsafe count down) is a decided feature
with a real cost at the switch. `spawn_el0` legitimately fell **32.7%** on both ISAs when
`b918337b` bounded a walk by occupancy. An anchor with no ledger of intended deltas would fire on
both, and a gate that fires on correct work is the shape §61 and milestone 78 already dropped
checks for. So this option is really "an anchor plus a per-benchmark ledger of what was intended",
and the ledger is the expensive half. Item 2 is most of that ledger, which is why it comes first.

### 4. Tighten the threshold. Refused, and the tree already knows why.

Milestone 25 (cross-OS performance comparison) demoted `--check` from a 2% gate to the coarse 10%
tripwire deliberately, because adding unrelated live code moves untouched benchmarks several percent
non-uniformly through whole-crate inlining decisions. The audit re-confirms it: `9890eb02` moved
every kernel-side IPC row by 4 to 8.5% by adding one benchmark. Tightening the threshold rebuilds
the false-positive problem that demotion was written to escape, and it would not have caught either
2026-09-15 step, both of which were real costs honestly measured and wrongly explained.

**A per-benchmark drift budget** consumed across saves is item 3 with extra bookkeeping: it needs the
same anchor and the same intended-delta ledger, and adds a consumption rule on top. Not recommended
separately.

## Recommendation

Do **1** now, as its own commit. Do **2** next, because it is small and because it is the ledger that
item 3 would otherwise have to invent. Hold **3** until 2 has been in the tree long enough to say
whether the attributions it collects are good enough to gate on. Do not do **4**.

The reversibility test says the same thing: 1 and 2 are undoable in an afternoon and nobody outside
this tree has acted on them, while 3 writes a second committed floor that every future measurement is
compared against, which is a fact that other work starts depending on.

## How this bears on the open toolchain-bump decision

This overlaps calef's still-open decision 2 from PR #883, written up as
`icount-baselines-drift-after-a-toolchain-bump.md`: should a toolchain bump re-baseline in the same
pull request, or fail loudly? This proposal does not decide it, but the audit supplies a fact that
was not available when it was written.

That proposal's premise is that a new nightly's codegen invalidates the baselines. Milestone 300
measured it: across `nightly-2026-08-27` and `nightly-2026-09-15` the toolchain term is **~0**, and
the QEMU upgrade term is also ~0. On that evidence, **a bump-triggered automatic re-baseline would
have written a new floor for a cause that measured zero, and in doing so would have absorbed
milestone 139's regression under a toolchain label**, which is exactly what `44890a8a` did by hand.
That is an argument for the "fail loudly" half of the proposal's own option and against the
"re-baseline in the same pull request" half, and it is a measurement rather than a preference. Still
calef's call.

## BUGS

- **The cumulative numbers are anchored, and the anchor is a judgment.** aarch64 is anchored at
  `74431429` (2026-07-30) rather than at its first save, because `60e75545` pinned the bench to one
  hart after finding the `-smp 4` counter was fiction, which re-means every earlier number. A
  different anchor gives different cumulative figures. The per-save tables are anchor-free and are
  the checkable record.
- **This audit ran no benchmarks.** Every figure is arithmetic over committed text, which is what
  made it cheap and is also its limit: it says what each save *recorded*, not what the tree measured
  between saves. A regression that appeared and was fixed inside one window is invisible here.
- **Item 2 cannot make an attribution true.** It moves a claim from a commit message to the file, so
  a reader meets it. `44890a8a` would still have written its false toolchain attribution; it would
  just have been findable.
