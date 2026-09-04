# Four crates were lifted so loom could search them, and nothing checks that the callers still call it

**Status: PROPOSED 2026-09-03.** Written by the milestone 247 sweep, from milestone 136's block.

**Gate: NONE.** Milestone 136 built the mechanism for one crate and it works, so the pattern to
copy is in the tree. The block declines to choose between copying it four times and generalising
it, and that choice is a lane's to make and recommend, not calef's: it is reversible, it is
internal to `script/lint`, and nothing outside this repository acts on it.

**In brief.** `steal_request`, `clock_proto`, `wake_handshake` and `canary_gate` were each lifted
out of their callers so loom could search their concurrent transitions. Loom then proves things
about the lifted code. Nothing checks that the callers still route through it, so a caller that
grows a second path around the searched surface takes the proof's name without its coverage.
Milestone 136 closed this for `memory_regions` with a three-piece gate. Either repeat that gate
four more times, or build one mechanism that pins a loom-searched surface together with its
callers.

## Why this matters

A loom search is one of this project's strongest correctness claims and it is the one most easily
hollowed out, because the proof and the thing being claimed are in different files. The proof says
"these transitions are safe". The claim a reader takes away is "this subsystem's concurrency is
searched". The bridge between them is that the subsystem calls the searched code and nothing else,
and today that bridge is an assumption in four places.

It fails silently, which is the property that makes it worth a gate rather than a note. Nothing
goes red. A crate keeps its harnesses, the suite keeps passing, and the searched surface quietly
stops covering the caller. Compare the failure this tree already caught in the same family:
milestone 136's own `BUGS` records that its warrant check is line order inside a function rather
than dataflow, and that a body which called `claim_for_destroy`, discarded the result, and freed
pages some other way would pass. That is a known hole in a gate that exists. Four crates have no
gate at all.

## The fork, and what it costs

**Copy the gate four times.** Cheapest to start, and each copy is checked against the crate it
covers, so a wrong pin fails loudly and locally. The cost is four near-identical blocks in
`script/lint` that drift, and a fifth loom-searched crate later gets a fifth copy or gets
forgotten.

**One mechanism.** A single check that takes a crate name, reads the loom-searched surface, and
pins the callers. More work up front and one more thing to be wrong about the tree, which
AGENTS.md's ladder explicitly prices as rung two's weakness. It pays off the moment a fifth crate
is lifted, and this tree has lifted five so far.

A lane should measure the first before arguing for the second: writing one copy is the honest way
to find out how much of it is actually shared.

## Where it came from

Milestone 136's Follow-on: *"Gate the other four loom-searched crates (`steal_request`,
`clock_proto`, `wake_handshake`, `canary_gate`): each was lifted so loom could search it, and
nothing checks that its callers still call the lifted code. Repeat this block's three-piece gate
four more times, or build one mechanism pinning a loom-searched surface and its callers; the block
declines to pick."*

The same block also records, honestly, that nothing checks a newly pinned item is actually searched
by loom: the failure message asks for it in words, which is rung four and says so. Whichever shape
this takes inherits that hole and should say so where a reader meets it.
