# The security audit that has been due since August

**Status: PROPOSED 2026-09-17.** Raised by milestone 311, which repaired the tripwire and then read
what it said.

**Gate: NONE.** It is a reading of the tree, and its output is a report plus two index rows; nothing
has to build or boot before it can start.

**Promoted:** minted as **milestone 313** and built on 2026-09-17, under the userspace-confinement
lens the maintainer chose. The record is
[design/roadmap/313-confinement-audit.md](../313-confinement-audit.md), and the report it produced is
`design/audit-reports/2026-09-17-userspace-confinement.md`. The status line above keeps its original
date because that is what makes the pile measurable.

**What this proposal got the scale of wrong**, kept rather than edited away. It priced the sweep by
the trigger counts, "112 milestones" and "+45 components", and read those as the population to
review. Milestone 313 found the population was **two objects**, not forty-five, because it read every
capability mint site rather than the component names. The triggers are a good alarm and were never a
scope estimate; this is what taking them for one looks like.

## In brief

`script/audits` on 2026-09-17, with every count taken from the tree rather than remembered:

```
security      last 2026-08-17 (Newly minted authority, read adversarially: the seven ABI co)
  milestones built     76 -> 188  (+112, fires at 15)   <-- FIRED
  components           110 -> 155  (+45, fires at 8)   <-- FIRED
  ABI constants        50 -> 52  (+2, any change fires)   <-- FIRED
  external packages    108 -> 108  (+0, any change fires)
  calendar             31 days since (6 weeks)
  DUE: milestones built +112 (fires at 15); components +45 (fires at 8); ABI constants +2 (fires at 1)
```

**Three of the four count triggers have fired, the milestone one by seven and a half times its
threshold.** §74 chose 15 milestones and 8 components as the interval this project would accept
between adversarial reads. 112 and 45 have landed.

The calendar is the only trigger that has not fired, at 31 days against 42, and that is worth one
sentence because it is the trigger a reader reaches for first: **a mechanism resting on six weeks
alone would be reporting green today**, with 112 milestones of unreviewed change behind it. §74's
"event triggers first, a count second, the calendar a backstop" is doing exactly what it was written
to do, and this is the first occasion on which the ordering has mattered.

## Why it is a proposal and not simply a lane

Two things need calef, and the first is the expensive one.

**The lens is the audit.** `design/audit-reports/README.md`'s own instruction is to pick the lens the
last audit lacked, and it names three candidates not yet taken: supply chain, userspace confinement,
and the syscall surface itself. Choosing among them is a judgment about where this tree is most
likely to be wrong, which is the thing an audit is for and not a thing a lane should decide for
itself on the way past.

**And 112 milestones is too much tree for one lens.** Every prior audit on record took a bounded
scope (the assembly; the shared pages; untrusted counterparty input; the seven constants that landed
overnight). A lane briefed to "audit the last month" would produce breadth without depth, which
`design/audit-reports/README.md`'s first `BUGS` entry already warns about: the mechanism guarantees
that audits happen, not that any audit is good. Whether this is one audit or a short series is part
of the decision.

## What is known about the scope without deciding it

Offered so the decision is cheap rather than researched twice:

- **The two uncountable event triggers are both live questions.** "Has a new component taken device
  or network authority since the last audit?" covers 45 new components in a month. "Has this booted
  on a new machine class?" covers `radon` and `xenon`, both of which have booted real silicon since
  2026-08-17.
- **Two new ABI constants** landed in the window, and "newly minted authority" is the lens the
  2026-08-17 audit took on seven of them, so that lens has a tested procedure and a small scope.
- **External packages did not move at all** (108 to 108), so the supply-chain lens has the least new
  material of the three candidates, which is an argument for taking it later rather than now.
- It sits on `design/fatal-risks.md`'s **risk 7** (the confinement claim is false), which is why the
  cadence exists at all and why this outranks most of what is ready.

## What closes it

A report in `design/audit-reports/`, both index rows (`script/audits --baseline` prints the counts),
every finding dispositioned as fixed, minted or accepted, and any doc the audit finds overclaiming
fixed in the same lane. `script/audits --due` then exits 0 on its own, without anybody editing a
table to make it do so.
