# 262. The naming rule is written twice, so changing it means changing two files and remembering to

**Status: NOT-STARTED.** Minted 2026-09-05 by calef, on finding that a rule change had to land in
`AGENTS.md` and `notes/naming.md` together or the note would have contradicted the constitution the
moment it merged. *(Number provisional until the merge queue lands it.)*

**Gate: NONE.** Moving prose between two files this tree already owns.

## The measurement

`AGENTS.md` is **1,043 lines under a hard ceiling with zero headroom by design** (milestone 118), and
**132 of them, 12%, are the naming section**. `notes/naming.md` is **760 lines** on the same subject.

**Both stated the same rule.** `AGENTS.md` said standard terms *"should not be touched"* with a
list; `notes/naming.md:154` said *"Standard terms are already right and must not be touched"* with
the same list. On 2026-09-05 that rule changed, and it had to be edited in both places in one commit.
Nothing would have caught it if it had not been: no gate compares the two, and the note is the file a
lane reads *second*, so the contradiction would have been discovered by somebody acting on the wrong
one.

**That is this tree's recurring shape**, named by milestone 259's sweep the same day: *a note gets
corrected where its subject lives and not where its framing does*. Here there are two subjects and
one framing, which is the same defect wearing a second hat.

## The split to make, and it is not rule versus detail

**The constitution keeps the tests a lane applies.** Who names things, the `snake_case`/hyphen
domain table, nouns rather than verbs, and now the acronym test. Short, imperative, and read every
session, because `AGENTS.md` is the file an agent has in front of it and
`notes/naming.md` is a file it visits only when something sends it there. **Moving a rule to a note
demotes it to rung four of this file's own ladder**, and naming is the most frequently applied rule
in the project: every lane ships names.

**The note keeps the argument and the history.** Most of those 132 lines are already that rather than
rule: the two-tier convention calef rejected and why, the `wc` example that killed it, the evidence
that a rule was needed at all, the three crates settled the day it was written, the count that was
wrong because a grep matched only single-line includes. Every one of those is worth keeping and none
of it is something a lane applies.

**Expect to reclaim roughly half the section**, which is real budget in a file that has none, and the
number should be reported rather than estimated when the work is done.

## What good looks like

- **One statement per rule, in `AGENTS.md`**, with the note carrying its case.
- **The note says it is the case rather than the rule**, so a reader who finds a fuller treatment
  there does not think they have found a second authority.
- **Nothing is deleted.** Every argument moved, none dropped, because the history is why the rules
  survive contact with someone who disagrees.

## BUGS

- **Nothing will stop it happening again.** No gate compares a rule's statement in `AGENTS.md`
  against `notes/naming.md`, and none plausibly could, since the two are meant to say different
  things about the same subject. This milestone reduces the surface rather than closing it.
- **`AGENTS.md` is calef's file.** A lane may not edit it (milestone 118 records that a lane cannot
  even carry the budget marker there), so the note half is a lane's and the constitution half is
  calef's, and the milestone is not done until both have happened.
- **The naming section is not the only duplicated one.** Nobody has checked whether the merge queue,
  the lane roles or the dependency rule are stated twice the same way; this block is scoped to the
  one that was caught.
- **"Roughly half" is a guess**, from reading the section rather than from counting what is rule and
  what is argument.
