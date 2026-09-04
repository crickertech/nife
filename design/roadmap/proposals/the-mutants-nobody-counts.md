# An unviable mutant is a hole in the measurement that reads as a pass

**Status: PROPOSED 2026-09-03.** Written by the milestone 246 lane (measured boot's refusal path is
tested by nothing, and one mutant turns it off), which hit it by accident.

**Gate: NONE.** `cargo mutants --list` already reports what this needs, and no run is required to
get the count.

**What the work is.** `cargo mutants` scores a mutant it cannot build as **unviable**, and nothing in
`notes/mutation-testing.md`'s ledger accounts for that category. The ledger has missed, equivalent and
hang. So a published rate cannot distinguish *"no test could kill this"* from *"the tool could not
build it"*, and only the first is a fact about the code under test.

Milestone 246 found the sharp end of it: the tool's only operator on a function returning a struct is
to replace the body with `Default::default()`. `measured_boot::verdict` returned a `Verdict` with no
`Default`, so the mutant did not compile, and the tool reported **nothing at all** about the one
decision in that crate saying whether unmeasured code may run. Deriving `Default` turned it from *1
unviable, 0 tested* into **1 caught**. `measured_boot` alone had nine unviable mutants.

**Why it is worse than a missed mutant.** A missed mutant is visible: it appears in the report, it
lowers the score, somebody argues about it. An unviable one leaves no trace in the number, so a
crate's score is silently computed over a smaller function than the reader believes.

**The cheap version is most of it:** `cargo mutants --list` per crate counting unviable (one command,
no run), then reading them to see whether a small honest change to the code makes the mutant both
buildable and meaningful, then putting the count in the ledger.

**The hazard to name in whatever does this:** making a mutant viable can be gaming. Deriving `Default`
where the default is meaningless adds a mutant any test kills and proves nothing. The test is whether
the default is a value the code could plausibly be wrong with; for `Verdict` it is both the fail-safe
value and exactly the dangerous wrong answer, an absence where there was a refusal.

**Superseded in part:** milestone 250 was minted from this on the day it was written, before the
`proposals/` directory existed. This file is the record of where it came from.
