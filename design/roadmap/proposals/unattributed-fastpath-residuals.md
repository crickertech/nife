# The riscv64 and x86_64 fastpath residuals are unattributed, so their baselines cannot be re-saved

**Status: PROPOSED 2026-09-03.** Written by the milestone 247 sweep, from milestone 237's block.

**Gate: NONE.** The measurement runs on the dev machine under emulation, the tooling
(`script/fastpath-footprint`) exists, and bisecting a size delta needs no hardware and no decision.

**In brief.** `script/fastpath-footprint` compares the IPC fastpath's code size against a stored
baseline per architecture. Two of the three have drifted and nobody knows why: **riscv64 sits at
5132 against a 5106 baseline, and x86_64 at 6687 against 6639**. Milestone 237 attributed the
aarch64 growth to the cycle-counter grant, fixed it, and re-recorded the aarch64 baseline only. The
work is to bisect each of the two remaining gaps to the milestone that caused it, then re-record
those baselines **in the same commit that explains them**.

## Why this matters

Milestone 237 exists to refuse a specific move: re-saving a baseline because the number moved, which
launders growth into the new normal and retires the gate that was supposed to catch it. Re-recording
riscv64 and x86_64 today, with the causes unknown, would be exactly that move. So the two baselines
stay stale, and while they are stale the headroom they report is not the headroom that exists.

That is the same trap 237 was minted to escape, one architecture over. Milestone 237's own block
records that two lanes each measured "within bound" against the same stale aarch64 baseline and
neither re-saved it, and aarch64 headroom fell from 3.9 points to 1.5 with nothing firing. Nothing
prevents the other two ISAs walking the same path, and they have a head start.

It is also an architectural parity question. AGENTS.md rule 5 makes parity a gate: a kernel
capability ships on every supported architecture or a scope note records the gap. A footprint budget
that is current on one ISA and stale on two is that gap, in the instrument rather than in the
feature.

## The numbers have moved since this was written, and aarch64 has rejoined them

**Re-measured 2026-09-04** by the PR #716 lane, on `main` at `44e33268`, built under
`nightly-2026-09-03` (the pin this proposal was written against, so the compiler is held fixed and
the deltas are the tree's own):

| ISA | `ipc_fastpath` | baseline | residual | this proposal said |
|---|---|---|---|---|
| aarch64 | 5888 | 5852 | +0.6% | (re-recorded by 237, no residual) |
| riscv64 | 5122 | 5106 | +0.3% | 5132, +0.5% |
| x86_64 | 6767 | 6639 | +1.9% | 6687, +0.7% |

Two things changed and both sharpen the case rather than weakening it.

**aarch64 is drifting again.** Milestone 237 attributed its growth to the cycle-counter grant, fixed
it, and re-recorded the baseline at 5852. That was eight days ago and it is at 5888. So the fix held
for a week, which is what this proposal predicts: re-recording an instance does nothing to a
mechanism that has nobody remembering it. All **three** ISAs now carry an unattributed
`ipc_fastpath` residual, not two.

**x86_64's residual has nearly tripled**, 0.7% to 1.9%, against a 5% bound. It is the closest of the
three to its ceiling and it is moving the fastest, which makes it the one to bisect first rather
than last. Nothing has fired, and nothing will until it has absorbed the remaining 3.1 points.

**`syscall_entry` is a separate story and it is the healthy half.** All three sat exactly on their
baselines under that build (3304, 1870, 1637). The drift is entirely in the closure half, which is
worth knowing before bisecting: whatever is accumulating is in the IPC and switch closure, not in
the trap entry path.

## Where it came from

Milestone 237's `## Follow-on`: *"Attribute the riscv64 and x86_64 fastpath residuals, then
re-record those baselines in the commit that does it. riscv64 sits at 5132 against a 5106 baseline
and x86_64 at 6687 against 6639, and neither gap is bisected to a milestone, so re-saving them
today would be the absorb-the-growth move this block exists to refuse. Only aarch64 was
re-recorded here."*
