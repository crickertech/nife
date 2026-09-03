# 236. Three derivations are copied between scripts, and nothing notices when they drift

**Status: NOT-STARTED.** Minted 2026-09-02 by calef, from milestone 234's (the project's own numbers,
one row per ISO week) lane, which had to copy them to do its job. *(Number provisional until the
merge queue lands it.)*

**Gate: NONE.** Both copies are in this tree and the cheap answer needs nothing that does not exist.

**In brief.** Three derivations now exist twice:

- the **`unsafe` census**, in `script/lint` and again in `script/metrics`;
- the **code and comment line split**, likewise;
- the **harness count**, in `script/falsifications` and again in `script/metrics`.

They are copied rather than imported because each lives inside a `#!/bin/sh` script's inline python
heredoc, where nothing can import anything.

**So if a gate changes its definition, the dashboard keeps the old one silently.** The numbers on
`notes/project-metrics.md` would drift away from the numbers the gates enforce, both would look
authoritative, and no check would fire. That is rung four of AGENTS.md's ladder holding together the
page whose entire purpose is measurement.

## Why it is not simply "make it a crate"

The obvious fix is a host crate both callers depend on, and it may well be right, but two things make
it a decision rather than a refactor:

- **These are shell entry points on purpose.** `script/` is the "Scripts to Rule Them All" front door
  and its members are `#!/bin/sh` by convention; turning three of them into consumers of a Rust crate
  changes what a `script/` command is. AGENTS.md's naming table already treats `script/` as its own
  domain.
- **A derivation used by a gate has different requirements from one used by a report.** The gate must
  be fast enough to run on every push and refuse ambiguity; the report must run over eight historical
  checkouts and tolerate a tree that no longer parses. Those may not want the same code, and saying
  so is part of the work.

## What it needs

**One definition per question, or a check that fires when two disagree.** The second is weaker and
much cheaper, and this block does not choose: a test that runs both derivations on the current tree
and fails on a mismatch would catch every drift that matters without moving any code, at the cost of
keeping two implementations honest rather than removing one.

## BUGS

- **Nobody has seen this drift yet.** It is a hazard rather than a defect, which is the argument for
  the cheap answer and against a large refactor.
- **The count may be higher than three.** Those are the three milestone 234's lane needed; nothing has
  swept `script/` for others, and the same shape may exist between gates that never talk to a report.
- **`script/metrics` deliberately tolerates old trees** that the gates would reject, so a shared
  implementation would have to grow that tolerance and might then be looser than a gate should be.
