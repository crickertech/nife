# The branch-prefix taxonomy is a gate enforcing a convention with one consumer

**Status: PROPOSED 2026-09-03.** Written by the milestone 247 sweep, from milestone 130's block.

**Gate: DECISION.** `design/decisions/77-branch-prefixes.md` answers which prefixes belong on the
list and assumes the list stays. Retiring it is calef's call, and a lane deleting a ratified
convention on its own initiative is exactly the move this tree does not make.

**In brief.** `script/lint` checks a branch name against a taxonomy of prefixes. Exactly one of
them, `milestone/N-`, is read by anything: §90's roadmap-block check parses the number out of it. A
grep found no other consumer. So the question is whether to retire the taxonomy down to that one
prefix and let every other branch be named freely, or to keep the list as a convention people are
expected to follow with a gate as its only enforcement.

## Why this matters

A gate that enforces a convention nothing consumes teaches contributors that the gates are
arbitrary, and this tree already has a scar from that exact check: AGENTS.md cites the
branch-prefix check as its worked example of a rung-two mechanism being *wrong about the tree*,
because it rejected the repository's second-commonest prefix, and it failed every group build in
the merge queue's first day by rejecting GitHub's own synthetic branch names. A check with one
consumer and a history of false rejections is a poor trade.

The argument the other way is real and is why this is a decision rather than a cleanup. A shared
vocabulary for branch names is worth something to a reader scanning `git ls-remote --heads`, which
AGENTS.md makes a required step before briefing a lane, and it is the only ledger one session has
of another session's work. Retiring the list does not delete that value; it removes the enforcement
and leaves the convention as prose, which this tree's own ladder says is rung four.

Either answer is fine and cheap. What is not fine is the current state, where the list is enforced,
partly unread, and documented by a decision file that assumes a premise nobody has rechecked.

## Where it came from

Milestone 130's Follow-on: *"Decide whether to retire the branch-prefix taxonomy down to
`milestone/N-`, the one prefix §90's roadmap-block check actually reads. A grep found nothing else
consumes it, so the rest is a gate enforcing a convention with no consumer.
`design/decisions/77-branch-prefixes.md` answers which prefixes belong on the list and assumes it
stays, so retiring it is calef's call."*

## What answering it needs

Not a lane. The lookups are done: one grep established the single consumer, and the decision file
already exists to be amended. What is owed is a `**Status: PROPOSED.**` entry under
`design/decisions/` framing the two options with the false-rejection history attached, which is a
short piece of writing rather than an investigation.
