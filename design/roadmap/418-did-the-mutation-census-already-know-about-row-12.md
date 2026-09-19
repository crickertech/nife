# 418. Did the mutation census already know about row 12, and if so who read it

**Status: PROPOSED 2026-09-16.** Found by milestone 307, which broke
`paging::x86_64::no_vtd_entry_ever_sets_a_reserved_bit` and found it could not fail.

**Gate: NONE.** This is a measurement a lane can take with tools that exist, and it wants a lane
rather than an hour because the answer is not guessable from the tree.

## In brief

Milestone 307 found that row 12's proof stated all three of its assertions, and its `kani::assume`,
through `VTD_ADDR_MASK`, so widening that constant left the harness **SUCCESSFUL, 0 of 45 failed**
while every VT-d leaf carried a hardware-reserved bit.

**A mutation of `VTD_ADDR_MASK` is precisely what `script/mutation` exists to try**, and
`crates/paging` is a host crate already inside risk 3's census. So there are exactly two possibilities
and they call for different work:

- **The census already surfaced this as a survivor and nobody acted on it.** Then the finding is not
  about the harness at all; it is that a report nothing forces anyone to read is rung four, which is
  the failure `AGENTS.md` names more often than any other, and the fix is about the census's output
  rather than about `paging`.
- **The census did not surface it.** Then there is a gap in what the census mutates or in how it
  judges a survivor, in the crate holding three of the table's 26 claims, and that gap is worth its
  own work.

Either answer is worth more than the assumption, and the assumption a reader would make from
`design/fatal-risks.md` today is the second.

## What it would take

Run `script/mutation` against `crates/paging`, `crates/dma_validator`, `crates/component_plan` and
`crates/capability`, which between them carry rows 1 to 18, and compare its survivor list against the
per-row verdicts milestone 307 recorded in `notes/confinement-claims.md`. The two instruments measure
different things and the comparison is the deliverable: a census asks whether a change to the code is
caught by *something*, and 307 asked whether a named assertion can catch *anything*. Where they
disagree, the disagreement is the finding.

`AGENTS.md`'s memory ceiling applies: a mutation sweep is never run beside lanes.

## What is blocked until it is answered

Nothing is blocked. What is at stake is whether `design/fatal-risks.md`'s risk 3 and risk 7 are
measuring overlapping ground or adjacent ground, which both entries currently assume without either
having checked.
