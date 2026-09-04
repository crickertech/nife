# 248. A placement can only be drawn, never constructed, so the strongest finding on this machine rests on two boots

**Status: NOT-STARTED.** Minted 2026-09-03 by calef, from milestone 240's (the soak reports what
happened and not where, so an eightfold difference cannot be explained) handoff and the two censused
boots that followed it. *(Number provisional until the merge queue lands it.)*

**Gate: NONE.** Milestone 240 is merged and already knows where every worker is.

**In brief.** On 2026-09-03 the soak on radon produced a **fifteenfold** throughput spread from one
card and one build, and milestone 240's census explained it. The explanation makes a count the
predictor:

| | IPC groups sharing a core with a grinder | rate |
|---|---|---|
| the fast boot | 1 of 4 | **342,447/s** |
| the slow boot | 4 of 4 | **23,254/s** |

A grinder is pure compute and never yields, so a group co-located with one waits for its timeslice on
every exchange. The boot lottery decides how many of the four groups draw one.

**The evidence is stronger than two points usually are and it is still two points.** The slow
arrangement was *predicted in writing before it was seen*, in notes/soak.md, along with the
observation that would have killed the reading (grinders found piled on one core at 23,000/s). It was
not killed. But a confirmed prediction over one fast boot and one slow boot is not a controlled
result, and **nothing in this tree can construct an arrangement** to make it one: placement is drawn
at spawn and nothing rebalances.

## What this is not, and the distinction is the whole design

**This is not thread affinity, and conflating the two would buy a syscall this project has not
decided it wants.** A syscall surface is one of the expensive, irreversible categories (DECISIONS
§10, §16, and AGENTS.md's *move fast on what can be undone*): every future program is written against
it, and a placement primitive is exactly the kind of thing that looks obvious in the moment and
constrains the scheduler forever.

What this milestone needs is a **test lever in the soak build**, behind the same
`#[cfg(feature = "soak")]` that milestone 240's census already sits behind, costing the shipped
kernel nothing. The soak already knows it is building four groups of a responder, three callers and a
grinder; letting it say *where* is a change to a workload, not to an interface.

**Whether the real thing is worth its surface is precisely the question this milestone exists to help
answer**, and answering it before running the experiment would be deciding the expensive thing on the
strength of two boots. That is the wrong order.

## The experiment this buys

One soak run per arrangement, with the count above as the independent variable:

- 0 of 4 groups sharing with a grinder (all four grinders on one core)
- 1, 2, 3 of 4
- 4 of 4, which is the slow boot

If rate falls monotonically with the count, the mechanism is confirmed under control rather than
inferred from a lottery. If it does not, something else is going on and the two-boot reading was
luck wearing a mechanism's clothes.

**Each run can be short.** The rate is steady within a beat or two and held for three hours in the
2026-09-03 run, so this is minutes per arrangement rather than an evening each.

## The proof that this milestone worked

**A table of rate against the co-location count, every row from a constructed arrangement rather than
a drawn one**, in notes/soak.md beside the two boots that motivated it.

Not a mechanism for setting placement with no measurement made through it, which would be the lever
mistaken for the finding.

## BUGS

- **It measures this workload, not the scheduler.** Four groups and four grinders is a synthetic
  shape chosen for milestone 219; a real system's mix is not that, and the fifteenfold figure should
  not be carried over to one that is not.
- **A constructed placement is not a placement the system would choose**, so this answers "what does
  co-location cost" and never "how often does it happen." The lottery's distribution is still only
  knowable by sampling boots.
- **Nothing here fixes the problem it measures.** If co-location costs an order of magnitude, the
  remedy is scheduling policy the kernel does not have, and this block deliberately does not propose
  one: the tree has no notion that two threads communicate, that one never yields, or that one is
  latency-sensitive, and choosing among those is a design fork rather than a follow-on.
- **The lever must not leak into the shipped kernel.** Milestone 240 kept its census behind the soak
  feature and `script/fastpath-footprint` read the same 6,687 bytes over eight symbols afterwards;
  the same evidence is owed here.
