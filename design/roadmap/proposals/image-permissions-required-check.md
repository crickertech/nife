# `script/image-permissions` reports and does not gate, because it is not in the ruleset

**Status: PROPOSED 2026-09-03.** Written by the milestone 247 sweep, from milestone 208's block.

**Gate: DECISION.** It is one checkbox in the repository's merge queue ruleset, and only calef can
flip it: it is a GitHub setting rather than a file in this tree, so no lane and no pull request can
carry the change. There is nothing to build and nothing to review; the whole item is an ask.

**In brief.** Milestone 208 built `script/image-permissions`, which refuses a kernel image carrying
a writable-and-executable `PT_LOAD`. It runs in CI and its result is visible. It is not on the list
of checks the merge queue requires, so a red run merges anyway.

## Why this matters

A check that reports and does not gate is the shape this project keeps writing down as a failure.
It sits at rung two of AGENTS.md's ladder only if it fails loudly enough to stop something; as
configured it is rung four, a note in a run log that somebody has to read. The defect it exists to
catch is exactly the kind nobody reads a log for: `kernel/link-x86_64.ld` shipped a single `RWX`
segment for however long it did, and the tree found out because a lane deleting a duplicate ELF
parser tripped over `Error::WritableAndExecutable`. That is the same discovery path the check was
built to replace.

There is a second cost, which is what the green check now means. A reader who sees the check listed
reasonably concludes the W^X property is enforced on the image. Until the box is ticked, it is
enforced on the images whose authors happened to look.

## What the ask is, exactly

Add `image-permissions` to the required checks in the merge queue ruleset for this repository, in
the same list the rest of the gates are in. If the answer is no, the honest follow-up is to say so
in `design/roadmap/208-boot-section-wx.md` and stop describing it as a gate.

## Where it came from

Milestone 208 (the x86_64 kernel image ships an RWX segment) named it on the way out: *"Make
`script/image-permissions` a required check in the merge queue's ruleset. It is one checkbox and it
is calef's, because it is a repository setting rather than a file in this tree. Until it is flipped,
a red run is visible and merges anyway, so the gate reports rather than gates."*
