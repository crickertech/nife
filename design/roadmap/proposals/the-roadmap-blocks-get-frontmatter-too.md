# The roadmap blocks get frontmatter too, once the decisions pilot has held

**Status: PROPOSED 2026-09-23.** Raised by the lane of milestone 582 (a decision's status becomes a
field, and the index becomes generated), which migrated `design/decisions/` and was briefed to
propose this rather than do it.

**Gate: MILESTONE 582.** Not because the code depends on it, but because 582 is the pilot and this
is ten times the blast radius. If the schema turns out to be wrong in a way 211 files reveal, it is
cheaper to learn that there than in 578.

## What it is

`design/roadmap/` carries the same defect `design/decisions/` carried until 582. A block's status
is `**Status: BUILT.**` in prose, and `script/roadmap` recovers it, its Built date, its gate, its
branch and its promoted-from slug with a stack of regexes over paragraphs. The index is generated
already, which is the one thing this directory got right first.

The migration would be the same four steps 582 took: a schema, a mechanical rewrite of every block,
a script that reads fields instead of sentences, and a rule that the prose may not restate a field.

## Why it is not simply 582 again

Three things make it bigger than a bigger version.

The corpus is 578 files against 211, and every one of them is cited by number from somewhere.

The status vocabulary is eight tokens, not four: `BUILT`, `REMOVED`, `SUPERSEDED`, `PARTIAL`,
`IN-PROGRESS`, `NOT-STARTED`, `OPTIONAL` and `REFUSED`. Several carry required companions that
would become fields of their own. `BUILT` owes a date, `IN-PROGRESS` owes a branch name, `REFUSED`
owes a `## Revisit` section, and the gate line owes a vocabulary of four tokens plus a milestone
number.

`script/roadmap` reads more out of prose than `script/decisions` ever did, including the
promoted-from slug and the set of milestones a refusal names. Each of those is a candidate field,
and deciding which become keys is a schema question for calef rather than a mechanical rewrite.

## What it would cost, and what it buys

One lane, on the evidence of 582, where the mechanical rewrite was the cheap part and the
provenance archaeology was the expensive one. The roadmap is younger and better dated than the
decisions corpus, so the equivalent of 582's 152 missing raise dates may be much smaller. Nobody
has counted, and counting is the first thing this lane should do, before proposing a schema.

What it buys is the same two things: a status that no regex has to guess at, and a set of fields a
script can walk without parsing paragraphs, which is what §207 (the roadmap is a graph, and the
block says so in fields a script can walk) already asked for in a different shape.

## What is not proposed

No number. The integrator mints one at promotion, and 582's lane was told not to reach for it.
