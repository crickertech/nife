# Give the three instruments nothing runs a caller

**Status: PROPOSED 2026-09-03.** Written by the milestone 247 sweep, from milestone 232's block.

**Gate: DECISION.** Which of the three joins `script/gates`, which joins CI and which gets a cadence
is calef's call, because two of them are expensive enough that "run it in CI" changes what a pull
request costs. The measuring and the wiring are a lane's; the placement is not.

**In brief.** Milestone 232's audit found three instruments in this tree that render a verdict and
that nothing ever calls: `script/interleaving-check` (26 loom harnesses, 12.4 seconds, green),
`script/crate-probes` (about 3 minutes, 43 of 50 passing) and `script/rule-violations --check`. Each
one runs, each one answers a real question, and each one only answers it when a person remembers to
type it. The work is to give each a caller: the pre-push gate, a CI job, or `script/cadence-check`'s
schedule.

## Why this matters

`design/fatal-risks.md`'s first risk stands **GREEN on a hand-run instrument**. That is a claim the
project makes about whether it should continue, resting on somebody having typed a command once.
Nothing re-runs it, nothing notices when it goes red, and the green will keep reading as current for
as long as nobody looks. The other two are the same shape at lower stakes: `crate-probes` sits at 43
of 50 and no one is told when that falls, and `rule-violations --check` enforces rules AGENTS.md
treats as load-bearing.

This is rung two of AGENTS.md's ladder going unclaimed while rung four holds a fatal risk. A gate
that fails loudly is exactly what these three are; they are just not attached to anything.

## What it would take

The cheap half is already priced. `interleaving-check` is 12.4 seconds and belongs in `script/gates`
on cost alone. The other two are the decision: `crate-probes` builds fifty crates and
`repeat-under-load` boots QEMU repeatedly, so milestone 232 deliberately refused to assume CI was
the answer and priced them instead. A cadence entry (the `script/cadence-check` mechanism milestone
238 extended) is the third option and is the one that fits an expensive check whose answer changes
slowly.

## Where it came from

Milestone 232's `## Follow-on` named it: *"Give the three instruments nothing runs a caller ...
Deciding which joins `script/gates`, which joins CI and which gets a cadence is unowned, and fatal
risk 1 stands GREEN on a hand-run instrument meanwhile."* The block's own `BUGS` section carries the
sibling observation that two of the three are expensive, which is why that block priced them rather
than prescribing CI.
