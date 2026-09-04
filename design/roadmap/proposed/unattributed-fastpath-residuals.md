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

## Where it came from

Milestone 237's `## Follow-on`: *"Attribute the riscv64 and x86_64 fastpath residuals, then
re-record those baselines in the commit that does it. riscv64 sits at 5132 against a 5106 baseline
and x86_64 at 6687 against 6639, and neither gap is bisected to a milestone, so re-saving them
today would be the absorb-the-growth move this block exists to refuse. Only aarch64 was
re-recorded here."*
