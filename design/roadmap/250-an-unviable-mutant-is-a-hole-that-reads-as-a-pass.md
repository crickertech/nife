# 250. An unviable mutant is a hole in the measurement that reads as a pass

**Status: NOT-STARTED.** Minted 2026-09-03 by calef, from milestone 246's (measured boot's refusal
path is tested by nothing, and one mutant turns it off) handoff. *(Number provisional until the merge
queue lands it.)*

**Gate: NONE.** `cargo mutants --list` already reports what this needs.

**In brief.** Milestone 246 moved measured boot's refusal decision into `measured_boot::verdict` and
tested it. Then it found something nobody was looking for: **`cargo mutants` had never mutated that
function at all.**

The tool's only operator on a function returning a struct is to replace the body with
`Default::default()`. `Verdict` had no `Default`, so the mutant did not compile and was scored
**unviable** rather than missed. The crate could have reported a perfect score while the tool said
**nothing whatsoever** about the one decision in it that says whether unmeasured code may run.
Deriving `Default` turned it from *1 unviable, 0 tested* into **1 caught**.

**That is the shape worth sweeping for**, and it is worse than a missed mutant. A missed mutant is
visible: it appears in the report, it lowers the score, somebody argues about it. An unviable one
leaves no trace in the number at all, so a crate's score is silently computed over a smaller function
than the reader believes.

## Why the ledger does not currently catch it

notes/mutation-testing.md accounts for **missed**, **equivalent** and **hang**, and does not account
for unviable at all. Nothing in any published rate distinguishes *"no test could kill this"* from
*"the tool could not build it"*, and the second is not a fact about the code under test.

`measured_boot` alone had **nine** unviable mutants, and one of them was hiding the crate's only
security decision.

## What to do, and the cheap version is most of it

1. **`cargo mutants --list` per crate, counting unviable**, which is one command and needs no run.
2. **Read them.** The question for each is whether a small, honest change to the code under test makes
   the mutant both buildable and meaningful. `Verdict` deriving `Default` is the worked example, and it
   is a good one because the default is simultaneously the fail-safe value and exactly the dangerous
   wrong answer: an absence where there was a refusal.
3. **Put the count in the ledger**, so a published rate carries how much of the corpus the tool could
   not reach.

## The proof that this milestone worked

**An unviable count per crate in notes/mutation-testing.md**, and at least the highest-stakes ones
read and dispositioned: made viable, or recorded with the reason they cannot be.

Not a lower unviable count on its own, which could be bought by deleting code the tool struggles with.

## BUGS

- **Making a mutant viable can be gaming.** Deriving `Default` on a type where the default is
  meaningless would add a mutant that any test kills, raising the score and proving nothing. The test
  is whether the default is a value the code could plausibly be wrong with, which is a judgement
  rather than a rule, and each one should say why it is not gaming.
- **This is a property of `cargo mutants`**, whose pinned version is in `.cargo-mutants-version`. A
  tool upgrade can change which mutants are viable, and the count is therefore versioned rather than
  absolute.
- **It says nothing about functions the tool generates no mutants for at all**, which is a different
  and larger hole: milestone 246's block already records that `system_initializer::measured` gets no
  mutants and that "no mutants generated" is a property of the tool rather than a proof.
