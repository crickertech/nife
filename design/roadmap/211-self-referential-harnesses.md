# 211. A harness that states its property through the function under test cannot see that function break

**Status: NOT-STARTED.** Minted 2026-08-31 after the pattern was found twice in two days, by two
different lanes, in two different crates. *(Number provisional until the merge queue lands it.)*

**Gate: NONE.** The harnesses and §134s falsification machinery both exist; what is missing is
somebody asking the question of all of them.

**In brief.** Milestone 194's lane found it in `capability`, and `notes/falsification.md` states the
generalisation rather than the instance:

> `derive_never_widens_rights` states its property *through the predicate it is testing*. It asserts
> `derived.rights.is_subset_of(src_rights)` while `derive` guards on
> `rights.is_subset_of(src.rights)`, so a consistently wrong `is_subset_of` satisfies both sides and
> the harness cannot see the break.
>
> -- notes/falsification.md

Milestone 202's lane then found the same shape in `component_plan`'s proof, independently.

**Two instances is a class.** This milestone is the sweep that asks how many more there are.

## Why it is worth a sweep rather than two fixes

The defect is invisible to everything this project has. The harness passes. `kani::cover!` passes,
because the input set is not empty. Mutation testing does not reach `cfg(kani)` code at all. And
until DECISIONS §134 (a harness carries a machine-replayable falsification record) landed, nothing
even asked the question: **it was found only because somebody broke the code and watched what stayed
green.**

It is also the most dangerous shape a proof can have, because it is *load-bearing in one direction*.
`derive_never_widens_rights` genuinely catches the guard being dropped, which is what its recorded
falsification does. It is blind only to the comparison the guard calls. So it is neither a good
harness nor a useless one, and a reader seeing it green learns something true and something false at
once.

## The sweep

**For every harness, ask: does the assertion reach the same function the code under test uses to
decide?** Where it does, the harness proves the two agree, not that either is right.

Mechanical narrowing is possible and should be tried before reading 145 harnesses by hand: a harness
whose assertion calls a function that also appears in the guarded path is the candidate set, and it
is smaller than the whole. **The judgement is not mechanical**, though: agreeing with itself is
sometimes the property you want, and a sweep that flags every such harness as defective would be
wrong.

**Each finding is either a rewritten harness or a recorded reason it is fine.** A rewritten one
states the property independently, in terms the implementation does not get to choose, the way
milestone 193's kernel harnesses state run arithmetic in `u128` so the harness cannot repeat the
implementation back at itself.

## BUGS

- **The rewrite is harder than the finding**, and this block does not pretend otherwise. Stating a
  property without using the vocabulary the code uses is exactly where specification work is
  expensive, and for some properties there may be no independent statement worth having.
- **It cannot be a gate.** No check can tell "asserts through the function under test" from
  "legitimately asserts agreement", so this is a sweep with a worklist, not a lint.
- **The sweep's own output can rot.** A harness found fine today can become self-referential when the
  code it covers is refactored, and nothing will say so. §134's falsification records are the closest
  mechanism, and they only catch it if somebody re-falsifies after the refactor.
