# `NOT-STARTED` is the third status that can be false, and milestone 75 is the proof

**Status: PROPOSED 2026-09-03.** Written by the milestone 252 sweep, from milestone 134's and
milestone 16's blocks, which both trip over the same stale block.

**Gate: NONE.** Fixing milestone 75's status is a paragraph and a word, and the design question
behind it is answerable by reading `script/roadmap` and the four uncovered statuses.

**In brief.** Milestone 247 gated `BUILT` and `REMOVED`; milestone 252 gated `PARTIAL`. Three
statuses are left uncovered (`NOT-STARTED`, `IN-PROGRESS`, `OPTIONAL`, `RECORDED`), and at least one
of them rots the same way. **`design/roadmap/75-cycle-counter-authority.md` reads `NOT-STARTED` with
`Gate: DECISION` for a decision calef made on 2026-09-02 (§139) and two milestones built the same
week** (229 on 2026-09-02, 237 on 2026-09-03). Three separate blocks cite 75 as a live blocker, and
one of them, milestone 74, carries `MILESTONE 75` in its own gate line, so the staleness propagates
into the readiness report a maintainer uses to brief lanes.

## What is worth deciding

- **Fix 75 first**, which is a status word and a paragraph, and re-read the gate lines that name it.
  That is worth doing whether or not anything below happens.
- **Then the question this proposal exists for**: whether a `NOT-STARTED` block can be checked at
  all. A `PARTIAL` block enumerates work, so the gate can demand the enumeration. A `NOT-STARTED`
  block asserts one thing, that nothing has been done, and the tree cannot see that.
- **The one mechanical handle that does exist**: a `DECISION` gate on a block whose decision file is
  `DECIDED`, and a `MILESTONE N` gate where N is BUILT. `script/roadmap` already refuses the second
  case for a gate line; it does not connect a `DECISION` gate to any file, because a gate line does
  not name one. Making it name one would be a real check and a real cost.
- `IN-PROGRESS` is already the one status with a mechanical check (its branch must not have merged),
  minted for the same reason on 2026-08-17 after all six were found false, and it is evidence that
  this family of defect is worth gating where it can be.

## BUGS

- **The gate for `NOT-STARTED` may not exist.** If the honest answer is that only a person can tell,
  this proposal ends with 75 fixed and a sentence in `design/roadmap/README.md` saying so, and that
  is a legitimate outcome rather than a failure. Milestone 252's own block reserves the same right.
- **`OPTIONAL` and `RECORDED` are deliberately off the work list**, so a stale one costs less than a
  stale `NOT-STARTED`, which is offered to lanes as ready work.
