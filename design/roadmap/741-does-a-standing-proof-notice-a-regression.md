---
status: PARTIAL
raised: 2026-10-04
milestone_dependencies: 191
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 741. Does a standing proof notice a regression?

Raised 2026-10-04 (UTC) by calef for fatal risk 2 (the proofs prove trivia, and the real bugs live
where Kani cannot reach). Milestone number provisional; the maintainer mints it at merge. *(Title and
slug are drafts.)*

The red half of risk 2's amber is that no standing proof has caught a regression: every catch in
milestone 191 (did the proofs catch the bugs?) happened while a harness was being written. The
defects that escaped most recently were tests and gates that could not fail, so this asks whether
the proofs are in the same class, by measurement rather than by reading.

**The question:** of the code a standing Kani harness covers, which cargo-mutants mutants does a
proof kill, which do only the tests kill, and which does nothing kill? Answered per harness as
killed / total mutants in the functions it reaches.

## Done when

- `helpers/kani_reach.py` (name provisional) plans, proves and reports one package, and
  `.github/workflows/kani-reach.yml` runs it sharded in CI.
- A pilot (nifefs, elf, and the kernel's aarch64 harnesses) is reported before scaling.
- Every crate in `script/verify`'s table is measured, or the measured cost says why not.
- The numbers, per harness, live in `notes/kani-reach-2026-10-04.md`, with one facts-only line in
  risk 2's section of `design/fatal-risks/README.md`.

## Index row

Mutation-tests the proofs themselves: every cargo-mutants mutant of the code a standing Kani
harness reaches is proved, so a harness that kills nothing is named as one that no regression could
turn red. It answers the survivorship half of fatal risk 2's amber with a number per harness.

## Follow-on

- **Outstanding.** The pilot is running in CI on `lane/kani-reach`; checked by `gh run list -w
  kani-reach.yml` on 2026-10-04.
