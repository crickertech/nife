# Fatal risks, decisions, names, proposals and milestones by status

*An appendix to [the register of measures](../register-of-measures.md). The name is provisional.*

## The nine things that would kill nife

From `design/fatal-risks.md`, by **Experiment status**: the field calef ratified on 2026-09-23, with
three values and no fourth. `RUN` means the experiment has been performed, `NOT-RUN` that it has not
and could be, `CANNOT-RUN` that it cannot be performed at all. `script/fatal-risks` fails on any
other word, which is why this can be charted as an enumeration rather than read out of a sentence.

**It says whether an experiment happened. It never says what it found.** That is the half of the
2026-09-23 proposal calef did not take, and leaving it out is deliberate rather than pending:
`GREEN`, `AMBER`, `MEASURED` and `AUDITED` are in the file, in prose, beside the argument that earns
them, and a colour band on a chart would be a worse version of a paragraph. `CANNOT-RUN` is the one
value that carries a judgement anyway, and it is the file's own: risk 8 cannot be observed until
milestone 198 (a package manager, and the trivial install that makes a second customer possible)
lands, and **a fatal risk that cannot be tested is the most dangerous state a fatal risk can be in**.

**This replaced a tested/untested pair on 2026-09-23, and the pair's refusal of a verdict column was
right when it was written.** It said a colour series would be "a script reading a sentence and
guessing", which it would have been: four statuses, three of them ending in GREEN or AMBER and risk
7's in neither. What changed is not the reading but the thing read. The pair had to go for a simpler
reason as well: all nine entries carried a status line by 2026-09-23, so it sat at nine and zero and
told a reader nothing.

**Weeks before the field existed are read through the words the file used then**, so the early bars
are shorter than nine: a risk with no status line at all counts in none of the three, because an
entry that said nothing said nothing. `MEASURED` and `AUDITED` are read as `RUN` for those weeks,
and `NOT YET`, `UNRUN` and `UNTESTED` as `NOT-RUN`, `NOT-RUN` and `CANNOT-RUN`. None of those five
words may be written today.

## Architecture decisions by status

From `design/decisions/README.md`. The grey band in the first three weeks is the honest bucket: a
decision was a `## N.` heading in one 5,320-line `DECISIONS.md` and **nothing said whether it still
held**. Milestone 114 (split `DECISIONS.md`, and give a decision a status) is where a status exists
at all, so counting those early decisions as `DECIDED` would invent a claim the record never carried.
They are counted, and counted as having no status.

139 decisions in eight weeks, of which 17 are `AMENDED` and 3 `SUPERSEDED` at 2026W36. Twenty
decisions revised or replaced out of 139 is the number that says the vocabulary is doing work: a tree
where nothing was ever amended would mean either that every first answer was right or that nobody
went back.

`PROPOSED` is the queue waiting on calef and it stays small (10, 4, 8, 2, 3). It is a queue depth
rather than a backlog, which is the shape it should have.

## Names by what the tree records about them

From the provenance block in each named thing's own header, which is where milestone 115 (the names that
were ratified, and the ones that were refused) put it: a crate's `src/lib.rs`, a program's, a `script/` entry
point's comment, a Cargo package's manifest. `script/names` derives the same four counts by walking
the working tree; this derives them from git history. They share the parse
(`scripts/name_provenance.py`) and not the file walk, so the two agree by construction rather than by
luck, and at 2026W36 they do: 204 names, 104 `ratified`, 37 `recorded`, 63 `provisional`, 0
`unrecorded`.

The four words are what a block *says*. **`Ratified`** is calef ruling, with a date and what was
refused. **`Recorded`** is the tree arguing the name somewhere and nobody ever putting it to him.
**`Provisional`** is whoever coined it saying out loud that they expect it to change. **`Unrecorded`**
is nothing outside the block saying why the name is what it is.

**The total is every named thing, and the four statuses do not have to add up to it.** The pale band
is the difference: named things carrying no block at all. That is a different claim from
`Unrecorded`, which is a block saying the history is silent, and the two are kept apart for the same
reason the decisions chart above will not read a statusless decision as `DECIDED`.

**The first three weeks are the sharpest restatement artifact on this page, and there the band is the
whole bar.** There were 16 named things at 2026W29 and 119 at 2026W31, not one of them carrying
provenance, because the convention that records it did not exist until 2026-08-04. So read those
bars as "nobody was writing this down yet", and read the first blue bar as a convention arriving
rather than as 72 names being ratified in a week. Backfilling them was considered and refused: a
ratification invented to fill a cell would put a false claim in the one record whose entire job is
saying who claimed what.

**The band that survives into 2026W32 and 2026W33 is not an artifact, and it is the most useful thing
this series found.** It is seven names, and all seven are Cargo packages: `kernel`, `user`, `xtask`,
`redoxfs_server`, `redoxfs_host`, `std_exerciser` and the fuzz package. Milestone 115 covered three
surfaces and a package was not one of them, so for two weeks `script/names std_exerciser` answered
"neither a name in the tree nor a recorded refusal" while looking exactly as authoritative as a true
answer. calef found it on 2026-08-18, the `package` kind closed it, and the band goes to zero in
2026W34. **Nothing told this series about that hole**; it walks four kinds today and finds the fourth
missing from the weeks before it existed. A registry with a hole answering confidently is the failure
`script/names`' own header records, and this is what it looks like from outside.

**2026W36 is one milestone doing one thing.** `Unrecorded` goes from 60 to zero, `Recorded` from 9 to
37 and `Provisional` from 18 to 63. That is milestone 264 (sixty names the history cannot justify,
and the research that would let calef rule on them), which converted every name nobody had written a
reason for into one that says something: the reasoning where the history supplies it, an argued
proposal where it does not. The bar is the same height it would have been without it. What changed is
what the tree can say about the names in it.

### A rising `Provisional` band is not debt

This is the number here most likely to be misread, and the misreading would cost something real, so
it is worth saying flatly.

**Nothing in this tree fails because a name is unratified.** `AGENTS.md`:

> `script/names --unratified` is a worklist rather than a wall precisely so that an unratified name
> never blocks anyone's build.

`script/names --check` gates on a block being *present*, never on it saying `ratified`, and that is
deliberate: a gate that demanded the queue be drained would block every unrelated merge behind a
review nobody can hurry.

**A provisional name is the mechanism working, not the mechanism failing.** It is what `AGENTS.md`
tells a lane to ship when it needs a name and the decision is calef's, and it exists to convert an
expensive decision into a cheap one by refusing to pretend it is settled. A lane that coins a name,
argues it, records what it refused and marks the result provisional has done the thing the convention
asks for. A lane that quietly ships a name without saying it is unsigned has not, and it will not
show up on this chart at all, which is the limit of what a count of signatures can see.

So the green band going up means lanes are naming things and being honest that nobody ruled. It is a
queue depth against one person's attention, and this project's scarcest resource is exactly that
attention, so a growing queue says the tree is growing faster than one reviewer rules on it and says
nothing at all about the names being wrong. **The band worth an alarm is `Unrecorded`**, because that
is a name nobody anywhere argued for, and it is the one this chart has at zero.

## Unnumbered proposals

**74 at 2026W36, and zero in every week before it**, which is the `proposals_unnumbered` column in
the CSV. `design/roadmap/proposals/` was created on 2026-09-04 by milestone 247 (follow-on work named
by a finished milestone goes nowhere, and this is the third time).

**They are drawn on top of the milestones chart, since 2026-09-19**, as the eighth series. This page
used to say there was no chart because there was one bar, which was true at 2026W36 and stopped
being true two weeks later without anybody revisiting it: the column had been collected every week
and drawn nowhere. **The bar totals on that chart now include them**, so 2026W38 reads 431, which is
324 numbered milestones and 107 proposals, and the jump at 2026W36 is the pile appearing when the
directory did rather than a burst of milestones. They sit on top because they are the work that has
not entered the roadmap yet, and because a new slot is appended so that no existing series changes
colour.

**Nothing else on this page could count these, and that is the reason for the column.** The
milestones chart keys on a milestone number, reading the blocks in `design/roadmap/` for a revision
that has no index and the index rows for one that does (a week counted before a stale index was
regenerated undercounts by however many milestones landed in between; it self-corrects on the next
regeneration, and past weeks are read from their own revisions).
A proposal is *defined* by not having one: a lane that finds work it is not doing writes
`design/roadmap/proposals/<slug>.md`, because the thing concurrent lanes collide over is the number
and not the authority, and an integrator assigns the number at promotion. So the pile was invisible
to every column here by construction, not by oversight.

### What a rising line means here, which is not what it means for names

The naming section above says a rising `Provisional` band is not debt. **Do not carry that reading
across.** A provisional name costs nothing while it sits, because nothing is waiting on it. An
identified piece of work that nobody has scheduled is a different object: something in this tree was
found to be wrong or missing, and the finding is parked. That is closer to debt, and it would be
dishonest to file it under the same reassurance.

But the count alone cannot tell you whether the pile is stalling, for two reasons that are worth
stating rather than leaving to a reader's optimism.

**It is a net count, and the flow is gross.** Five proposals have left the directory since it
existed, so 86 have been written and 81 remain. They left in two different ways, which is the more
interesting half: one was promoted to a number the ordinary way (milestone 256 (x86_64 places PCI
BARs in a hardcoded window, and on xenon that window is RAM)), and the others were **done**, by a
lane that picked the file up and fixed the thing, sometimes filing a narrower proposal in its place (`the-tcb-capability-that-outlives-start`
became a fix plus `the-region-half-of-the-retention-declaration`). A flat line on this column would
be consistent with a stalled pile and equally consistent with one draining exactly as fast as it
fills, and nothing here distinguishes them.

**Age is the tell, and age is not in this column.** `script/roadmap`'s own header says so: a gate on
age ("no proposal older than N days") would be routed around by not writing proposals, which is
worse, so what it does instead is print the count and the date of the oldest on every `script/lint`
run. Today the oldest is 2026-09-03 and the directory is a week old, so nothing has had time to go
stale and the count says nothing yet. **The number to watch is not this one going up; it is this one
going up while the oldest date stops moving.** `script/roadmap --proposed` lists them oldest first
and is the view that answers it.

**And a rising line is still better than the alternative it replaced.** The work in this pile used
to live in lane reports, which are read once, by one person, on the day they are written.
`AGENTS.md` records what that cost: milestone 90 (a guard page under the per-CPU secondary stacks)
exists only because calef happened to be at his desk the day a report named it.
Milestone 94 (the untracked-work sweep, and the convention that ends the category) swept the tree
for exactly this category and then left its own inventory in a pull request body for twelve days. 74 visible proposals is a worse
number than 74 scheduled milestones and a far better one than 74 findings nobody can enumerate.

## Milestones by status

From the milestone blocks in `design/roadmap/`, and from `design/roadmap/README.md`'s index table
for the weeks before calef retired it on 2026-09-21. The two zero weeks are a restatement artifact and
they are the sharpest one on this page. There really was a roadmap in 2026W30: `design/roadmap.md`
landed 2026-07-22 with ten rows in it. It had no status column. A milestone's state was prose inside
a cell, phrased a dozen different ways (`Built`, then `Built (frame scope)`, then `Built:` followed
by a paragraph on which half), which is exactly the defect the status vocabulary was later minted to
fix, and it is unparseable now for the same reason it was unreadable then. The first bar this chart
can draw is the week the column exists.

Rows 1 to 11 were backfilled into the roadmap later by milestone 76 (split the roadmap:
`design/roadmap/README.md` as index, one file per milestone). Its index half was retired
2026-09-21 and the one-file-per-milestone half is the roadmap today. Eleven milestones had been built before
the first bar; none of them is in it.

**The interesting line is not `BUILT`, it is `NOT-STARTED`.** Reading the rows as they stand at
2026W36: built milestones went 26, 63, 73, 97, 103, 125 across the six weeks the roadmap has
existed, which is a steady rate. Not-started went 16, 35, 32, 43, 62, 79. **The roadmap is growing
faster than the lanes drain it**, and the gap widened most in the last two weeks. That is what a
project generating its own work looks like, and it is the number to watch if the ranking function
ever needs defending: a backlog that grows faster than it is consumed is only healthy while
something is choosing the order.

`REMOVED` appears for the first time in 2026W36, with two rows. The token itself was minted
2026-08-30, when milestone 54 (a network file service a Mac can actually mount) was deleted and the
six words then available could only lie about it.

**Two statuses were missing from this chart's own vocabulary until 2026-09-20, and one of them had
been missing for five days without anyone noticing.** `script/metrics` keys the count on a fixed
list, and a token it does not hold is counted as nothing rather than as an error, so
`milestones_total` went short by exactly the blocks it could not see. `SUPERSEDED` was minted
2026-09-15 and never added: seven blocks were invisible, and 2026W38's total read 435 where the
tree had 442. Milestone 448 (a refusal gets a number, a status, and a condition that would change
it) added `SUPERSEDED` and `REFUSED` together and restated the history with `--backfill`, so the
correction reaches the weeks already written rather than only the next one. The failure is worth
keeping in view: a chart that undercounts silently looks exactly like a chart that is right.

**`REFUSED` will move every denominator on this page, and it is not work appearing.** Milestone 448
backfilled 42 blocks for refusals that name executable work, taking the roadmap from 444 milestones
to 487. None of them is a backlog item: they are excluded from `script/roadmap --ready`, from the
gate classification and from every count that reads as outstanding, and the ready count was 120
before and 120 after. The numbers that move are the totals a stranger quotes, so they are recorded
here rather than left to be rediscovered as a cliff in a bar chart. The column arrives on this chart
only when the index table is regenerated at merge, because this chart reads index rows and a lane
never edits that table.
