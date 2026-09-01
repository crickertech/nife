# 194. Build §134: the falsification record, its lint, and the sweep that replays it

**Status: BUILT 2026-08-31.** Minted 2026-08-30. DECISIONS §134 (a harness carries a
machine-replayable falsification record, or it is not evidence) was decided the same day and had no
tracked work, which is the state AGENTS.md says a finding may never be left in. All five increments
landed: the record and its two-directional lint, `script/falsifications`, the weekly sweep, the
per-pull-request half in `verify.yml`, and the retroactive pass putting all 141 harnesses at
`unfalsified` with six real falsifications on top. *(Number provisional until the merge queue lands
it.)*

**The Inductive Validity Core check came back no**, which this block asked for first: an IVC comes
out of an induction step, and Kani is a bounded model checker that has none. `notes/falsification.md`
records what was looked at. So the milestone did not shrink, and options C and D collapse to C.

**One finding changed §134**: the ratified patch path could not name eighteen of `paging`'s
twenty-six harnesses, and calef amended it to carry the module path the same day.

**In brief.** §134 decided that every Kani harness carries evidence it can fail, as a unified diff a
script can replay, checked weekly and per-pull-request for touched harnesses. This is that work.

## Do this first, because a yes makes most of the rest unnecessary

**Check whether Kani or CBMC can produce Inductive Validity Cores.** §134 names them as the cheaper
family: an IVC is the minimal set of model elements a proof actually needed, which gives coverage
with no re-run at all. Nobody has looked. This is minutes of reading and it can shrink the milestone
to a fraction, so it comes before any code.

## The increments

1. **The record and its lint.** A `Falsification:` block above each harness, beside `Name:` and in
   the shape milestone 115 established, with three states: `replayable <path>`, `attested <date>`, or
   `unfalsified`. The lint checks form and never truth, exactly as `script/names` does, and checks
   both directions of the path: every `replayable` resolves, and every patch under a crate's
   `falsifications/` has a harness.
2. **The reporter**, `script/falsifications`, printing the ratio that is the claim's honest
   denominator. Patches live at `<package>/falsifications/<module.path>.<harness_fn_name>.patch`, amended
   2026-08-31 because the first spelling assumed a harness name is unique in its crate and `paging`'s
   eighteen ISA-parity harnesses share six names.
3. **The sweep.** Apply each recorded diff, run that one harness, require red, revert. Weekly,
   against a baseline, with `script/mutation`'s posture: a report, not a per-commit gate.
4. **The per-pull-request half**, which is the part `script/mutation` has no equivalent for. A lane
   touching a harness or the code it covers re-falsifies that harness only, seconds rather than an
   hour. `script/verify --affected-since` already computes which harnesses a diff can reach.
5. **The retroactive pass.** 145 harnesses land at `unfalsified` and the worklist derives itself. Work
   through it by value, starting where milestone 191 (did the proofs catch the bugs?) found chaff:
   `capability::subset_is_reflexive` proves a tautology, and twelve of the 26 `paging` harnesses
   restate six properties once per ISA.

## BUGS

- **Nothing forces the ratio upward.** Every harness may sit at `unfalsified` while the lint stays green,
  which is the honest cost of making the convention shippable at all.
- **A diff rots against refactors**, which §134 argues is correct and which is still churn.
- **The retroactive pass is judgement, not mechanism.** Falsifying somebody else's harness means
  understanding what it was for, and a wrong falsification is worse than none because it certifies.
