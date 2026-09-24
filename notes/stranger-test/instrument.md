# Stranger test instrument history

*An appendix to [the stranger test](../stranger-test.md), which holds the current state. This page is the record for how the harness, the rubric and the cadence came to be as they are.*

## The harness, built 2026-08-18

`script/stranger-test` runs the protocol above. Before it existed, four runs each rebuilt it by
hand, and **each got the isolation wrong somewhere the others had not**: run 2's stranger was a
subagent and had `AGENTS.md` in context at turn zero, run 3's log files sat in the stranger's own
working directory under names that told it which run it was, and run 4's `pkill` shim was right
because somebody remembered rather than because anything held it. That is rung four of CLAUDE.md's
ladder holding up the milestone that exists to move things off rung four, and the `BUGS` entry
below has said so since run 1.

    script/stranger-test                       run against HEAD
    script/stranger-test --commit origin/main  run against a named commit
    script/stranger-test --prepare-only        build the isolated tree, probe it, and stop
    script/stranger-test --smoke               exercise the whole pipeline; not a measurement

What it now holds, each of which a person previously had to remember: the clone goes **inside** the
stranger's working directory rather than being it; the answer key is this note, its `notes/README.md`
entry and any markdown link to it, removed with the deletion amended into the tip; every artifact
the harness writes goes in a sibling directory with no run number in any path element; `pkill` and
`killall` are shadowed so a stranger following `README.md`'s own quit instruction cannot kill
another lane's emulator; the isolation is **probed** before the run rather than assumed, and the run
stops if the probe does not answer NONE; the stranger is told it is being measured, which is run 4's
finding acted on rather than a softening; and the account-wide `nife-dev` link goes back where it
was found, because a stranger's `script/test` takes it like any other lane and unlike a lane its
tree is disposable.

**The rubric is read out of this file rather than copied into the script**, so the table below stays
the only rubric and an amendment to it is asked at the next debrief without anyone editing the
harness. Only the question column ever leaves this file. The first draft of that extraction matched
every `| M<n> |` row in the note, which swept the *scoring* tables of runs 3 and 4 into the
stranger's own question list; the `--smoke` run caught it, in the reply, and that is the whole
argument for `--smoke` existing.

**What the harness cannot do is in its own `BUGS` section**, where a reader meets the tool rather
than here. Two of those belong in this note as well, because they bound what any future run can
claim: it cannot make the operator un-read `AGENTS.md`, and it cannot stop the tree leaking that a
test exists, which is now a `script/` entry point and a row in `notes/scripts.md`'s table on top of
everything that leaked before. The response to the second is disclosure, not concealment.

### Amendments, 2026-08-18, forced by run 3 and applied before run 4

The rubric section above says a stranger falling down somewhere it does not ask about means the
rubric is what needs amending. Run 3 falsified two rows rather than falling outside them. Run 3's
lane recorded the corrections here and left the table as written; **run 4's lane applied them to the
table itself, on 2026-08-18 and before its run started**, because a rubric that says one thing in
its table and another four paragraphs below is two rubrics, and the next run would grade against
whichever it read. Both rows above now carry the amended wording. What changed and why:

**M8's premise is stale.** It asks for "the three provenance states" and there are **four**: §89
landed `provisional` on 2026-08-16, ten days after the rubric was written, and
notes/adding-a-program.md states three and then corrects itself to four in the same section. The
stranger answered four and said the question was out of date, which is the better answer and would
have scored as "wrong" against the table as written. A rubric that predates a decision grades
against a tree that no longer exists. **The amended row states no count**, because the
count is the part that went stale and a row that counts something will go stale again; it asks what
states exist and points at the gate that enumerates them.

**M1 quotes a phrase the tree does not use.** "Designation is authorization" is object-capability
vocabulary from outside this project; the stranger looked for it, did not find it, and flagged that
it was importing the phrase rather than reading it. What the tree says in its own words is
`swish`'s banner, *"naming a resource in a command IS granting it"*, and `grant_plan`'s
`Refusal::NoSuchProgram`. **A rubric row that quotes a phrase the tree never wrote tests whether the
stranger already knew the field**, which is the opposite of what it is for. **The amended row asks the
question without importing anyone's vocabulary**, so a stranger can answer it from `crates/abi`,
from `notes/capabilities.md`, or from `swish`'s banner (*"naming a resource in a command IS granting
it"*) rather than by recognising a phrase.

**Scoring is per question: answered, partly answered, wrong, or absent.** "Wrong" is worse than
"absent" and is recorded separately, because a misleading document costs more than a silent one.

## The cadence: monthly, decided 2026-08-18

calef, 2026-08-18: **the stranger test runs monthly.** That closes the one thing five runs each
named as the reason milestone 117 could not move. The milestone's own sentence is *fix what the run
finds, then run it again*, and until this decision nothing scheduled the "again": the test ran when
somebody thought of it, which is rung four of CLAUDE.md's ladder holding up the milestone that
exists to move things off rung four. Four runs said so in their handoffs and the fifth said it was
the only remaining decision.

**The date of the last run comes from the `### Run <n>, <date>:` headings below**, which every run
already writes because that is how this note records a run. Nothing is maintained for the tripwire's
benefit, which is the property that stops a schedule and a record from disagreeing: a second copy of
the date, in a script or in a workflow, would be a fact kept in two places, and this tree has
watched that fail often enough to name it. §74 put the audit cadences in the audit index for the
same reason.

### What a run costs, stated where the cadence is decided

**One stranger session plus one lane**: roughly 200k lane tokens on top of the stranger's own, and
about half an hour of wall clock, with the score and the write-up on the end of that.

Run 5's `$10.56` is the number to quote carefully, and it has been quoted without its caveat
already. It is the CLI's `total_cost_usd` for the **stranger process alone**. It excludes the lane
that pre-registered the run, watched it, debriefed it, scored it and wrote it up, which is the
larger half of the cost. A monthly cadence is therefore a claim that a stranger's questions are
worth about that much once a month, which is the claim calef made and which the runs support: five
runs have produced roughly seven findings each, including two defects nobody in the tree could see.

**The value decays with the tree rather than with the clock**, which is the honest argument for a
count-based trigger of the kind §74 chose for audits, and against 30 days being the right unit
forever. It is not built that way here for one reason worth writing down rather than inferring: an
audit's triggers are countable (milestones built, components, ABI constants, packages) and this
one's are not. What ages a stranger run is how much of the *documentation* has moved, and the only
count for that is `script/audits --worklist`, which is a heuristic that never reads a sentence. So
the calendar is doing the whole job here where for audits it is a backstop, and it should be
revisited the first time a run comes back saying nothing had changed.
