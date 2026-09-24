# What the provenance record cannot see

*An appendix to [`design/naming.md`](../naming.md), which is the rule. This file holds the known limits of `Name:` blocks and `script/names`, including the kinds of name the worklist does not cover and why `scripts/` helpers are left out. It exists to verify or challenge the main page, and a reader who only needs to name, ratify or rename something should not have to open it. The directory `design/naming/` and this file's stem are provisional names, minted 2026-09-24 by the lane that split the file; naming is calef's.*

## BUGS

- **It checks that a name carries a reason, never that the reason is still true.** A block whose
  argument was overtaken looks exactly like one whose argument holds. That is the same limit
  `script/decisions --check` records for `§N` citations, and it is not closeable by a script,
  because a reason is prose and prose is checked by reading. Milestone 97 is the neighbouring case.
- **It cannot tell an honest `unrecorded` from a lazy one.** The 44 above were each researched
  against the git history, and nothing stops the forty-fifth from being a shrug. The only defence is
  that an `unrecorded` entry cites the commit that introduced the name, so the next reader starts
  where this one stopped rather than from nothing.
- **`recorded` is the state easiest to claim and hardest to check**, which is the price of splitting
  it out. The citation is checked for being present and never followed, so `recorded (milestone 46)`
  on a name milestone 46 never mentions passes the gate exactly as well as a true one. Read the
  citation before trusting the state, and note that a wrong one costs less than the alternatives: it
  demotes the entry from "research owed" to "signature owed" in a worklist, rather than putting a
  false claim in the tree.
- **Seven of the ten `recorded` lean on a rule that was derived from the names it now explains.**
  `*_proto` won in milestone 46 partly *because* "it is what the actual crates already were", so
  saying `fs_proto` is recorded by that rule is not fully independent of `fs_proto`. It is not
  circular either, since the decision adjudicated four live spellings and `script/lint` has enforced
  the winner since, but a reader deciding how much weight the state carries should know it leans on
  one decision and that the decision partly ratified the status quo.
- **The tree has more kinds of name than the table covers, and this bullet is the one place that
  says which.** Everything else that states the worklist's scope (`script/names`' own comment,
  `scripts/name_provenance.py`, `scripts/roadmap_proposals.py`) cites this bullet rather than
  restating it, on purpose: the last two copies of this claim went stale for a month after the
  coverage grew and nothing compared them against the tool. Crates, programs, `script/`
  entry points and Cargo packages carry blocks. Directories, types and `scripts/` helpers
  do not, and at least one ratified name had no home as a result:
  **`design/audit-reports/`** (calef, 2026-08-04), where `audit-trail` was refused because
  `design/decisions/35-scanner-findings.md` already uses that phrase for a chronological record of
  dismissals and it is also what an operating system means by it (`auditd`), and bare `audits` was
  passed over because every file in the directory is a report. Recorded here rather than stretched
  into a schema that does not fit it. **Closed for directories on 2026-08-16 by §75**: a directory
  under `design/` or `notes/` now carries its provenance in its own `README.md`, applied that day
  to `design/decisions/`, `design/roadmap/` and `notes/`, with `design/audit-reports/`'s line owed
  by the milestone-92 commit that creates it. **Closed for Cargo packages on 2026-08-18**, when calef found
  `script/names std_exerciser` answering "neither a name in the tree nor a recorded refusal" and the
  `package` kind was added: `kernel`, `xtask`, `redoxfs_server` and `tools/redoxfs_host` now carry
  blocks in their manifests, and milestone 276's weekly series shows the hole closing in 2026W34.
  Types are still uncovered. **`scripts/` is uncovered on purpose**, priced and refused on
  2026-09-20 by milestone 446 (the naming worklist says what it covers, and stops saying what it
  used to); its own bullet is below, because it is a decision rather than a gap.

  **That blind spot has a live casualty, found while triaging.** `disk_partitioner`'s introducing
  commit (2026-08-03) named two provisional things: itself, and `fs_maker`. The first is on a
  covered surface, so it is in the worklist with the word "provisional" quoted in its block. The
  second is at `redoxfs_server/src/bin/mkfs.rs`, where nothing looks, so it was resolved to `mkfs` by
  whoever was mid-task and no record anywhere says a decision was owed. That is the exact failure
  this milestone exists to prevent, still happening one directory over.
- **`scripts/` helpers are deliberately outside the worklist, and the numbers are why.** There are
  17 files in `scripts/`, of which **9 already carry a `Name:` paragraph** that nobody asked them
  for, written by the lane that added the file. So the question is not whether a helper may argue
  its own name (it may, and more than half do) but whether the worklist should **enumerate** them,
  and enumerating costs **about 15 rows on a worklist that is 76 deep today**, a fifth again of the
  only queue in this tree whose sole consumer is calef's attention. Three things decided it against.
  **The worklist is ordered by exposure**, and its own header says so: a program is typed at the
  prompt, a crate is what a newcomer greps, a `script/` entry point is typed by whoever works on the
  tree. The **Scripts** section above defines `scripts/` as the drawer that is *not* typed by
  people, so enumerating it would add a tier below the bottom tier of a list whose whole ordering is
  exposure. **Nothing is being lost today**: not one of those 9 paragraphs records a refusal, so
  milestone 115's "the refusals are the valuable half" claim gives up nothing by leaving them out,
  and that is the number to re-measure if this is ever revisited. And **the 8 without a paragraph
  are the machine-invoked ones** (`qemu-runner-*.sh`, `qemu-bounded.sh`, `memory-bounded-runner.sh`,
  `build-ripgrep.sh`, `rust_source.py`), so a gate demanding blocks would mostly manufacture
  rulings on names nobody types. **If it is ever built, order it last**, after `script/`, for the
  same exposure reason. What the refusal did buy: `script/names <helper>` no longer answers
  "neither a name in the tree nor a recorded refusal" about a file that argues its name at length,
  and instead says the name is out of scope and points at the file.
- **A type's name is a naming decision the mechanism does not see.** `BootEndowment` was ratified on
  2026-08-04 (replacing `Grants`) and is mentioned inside `system_initializer`'s block only because
  its crate happens to export it. `supervision_protocol::Endow` is an open naming question (§69) and
  appears nowhere in this record.
- **The `Name:` marker is a string in a comment**, so a header that never had one is caught by the
  gate while a header that loses one to an edit is caught only if the edit removes the whole line.
- **A correctly formatted `ratified` is never checked against calef, and on 2026-08-14 two lanes
  contradicted each other about the same name on the same day.** `crates/cpu_set` was introduced
  twice in one stack of pull requests. #176 carried `Name: ratified (calef, 2026-08-14, the same day
  the first-silicon online-set sweep introduced it)`. #178, one branch later in the same stack,
  carried `Name: unrecorded, provisional (introduced 2026-08-14 by the first-silicon online-set
  sweep; calef has not seen it)`. The second is the true one, and the two sat in the queue together
  asserting opposite things about whether a ruling had happened. (Both headers said `Chris` when
  they were written; they are quoted here in the referent this tree adopted on 2026-08-15, which is
  also how the surviving one now reads in `crates/cpu_set`.)

  **The gate caught the false one, and caught it for the wrong reason.** `script/names --check`
  rejected #176 because the date sits inside the parenthetical where `ratified (\d{4}-\d{2}-\d{2})`
  cannot reach it. Written as `Name: ratified 2026-08-14 (calef, …)`, the identical false claim would
  have passed every check in CI and landed on `main`. What stopped it was punctuation.

  **This follows from the design and is not a defect in it.** The gate checks that a block names one
  of the three states and never that the state is `ratified`, because a gate keyed on ratification
  holds every unrelated merge behind a queue only one person can drain, which is the wall milestone
  115 was written not to build. The cost of that choice is what this entry records: **claiming
  calef's ruling is as cheap as claiming anything else, and nothing downstream disagrees.** The
  `recorded` entry above says the citation is never followed; this is the same hole one state up,
  where the claim is not a citation anybody could follow but an assertion about a person.

  A lane that does not know this will write `ratified` meaning "this name seems settled". Write
  `unrecorded, provisional` instead and say so in the report. It costs nothing, `script/names
  --unratified` is a worklist rather than a wall, and an unratified name has never failed a build.

  **What would help without building the wall**, if this recurs: surface newly added `ratified`
  blocks in a diff for the integrator rather than blocking on them. `ratified` is the one state a
  lane structurally cannot be entitled to assert, so a new one appearing in a pull request is worth a
  human glance even though it must not be worth a red check. That is rung two of AGENTS.md's ladder
  applied to the one state that currently sits on rung zero. Not built, and not obviously worth
  building for a hazard observed once.
