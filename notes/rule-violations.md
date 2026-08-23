# The violation ledger: a rule that gets broken three times moves up the ladder

*The name `rule-violations.md`, the ledger's column shape, and `script/rule-violations`'s name are
all **provisional**. A lane ships a provisional name and says so; naming is calef's (AGENTS.md).*

Milestone 118 found that `AGENTS.md` warns against two things by name, each exactly once: `pkill`-ing
another lane's QEMU, and `git reset --hard` to "take a measurement". Both had already been violated
by the day the warning was measured: one lane killed another lane's emulator mid-test, and four
agents clobbered work with `reset --hard`, `checkout` or `stash` in a single day. **The rule was
present and was skipped.**
Milestone 118's own roadmap block names the fix for exactly this shape, echoing the ladder
`AGENTS.md`'s "Nobody remembers, so build the mechanism that does not need them to" section already
lays out for everything else: *"A rule that is violated repeatedly is not stated too quietly. It is
on the wrong rung."* Prose is rung four of that ladder. This ledger is what turns "violated
repeatedly" from an impression into a number, so the promotion decision has evidence instead of a
feeling behind it.

## What this is not

**It is not an enforcement mechanism**, and milestone 118's own honest-limit paragraph says so:
*"The ledger is the counterweight and it is weaker than a gate, because it depends on lanes
continuing to report their own mistakes honestly, which is a culture rather than a mechanism."*
Nothing here can see a violation happen. It can only total what somebody wrote down, the same way
`script/audits` cannot run an audit and can only say one is due. **Red means a rule needs a
decision, never that this script made one.**

## The convention

A row per incident (or per a dated batch of them, when the source that reported them did not
distinguish individual incidents), in the table below:

| date | rule | instances | status | source |
|---|---|---|---|---|
| 2026-08-04 | squash against `origin/main` instead of the recorded base SHA | 1 | open | AGENTS.md, "Commits" |
| 2026-08-04 | a destructive git operation (`reset --hard`, `checkout`, or `stash`) used to discard changes or "take a measurement" without committing or stashing first | 4 | open | design/roadmap/118-constitution-budget.md, "What it costs, measured 2026-08-05" |
| 2026-08-04 | `pkill` a QEMU process by name/pattern instead of walking the process tree from the harness that owns it | 1 | open | design/roadmap/118-constitution-budget.md, "What it costs, measured 2026-08-05" |

Columns:

- **date.** When the incident (or the batch) happened, not when the row was added.
- **rule.** The rule's own words or a close paraphrase, so two rows about the same rule can be
  told apart mechanically (see BUGS: today that means *identical text*, not judgement).
- **instances.** How many times it happened. Usually 1; a source that reports an aggregate
  ("three agents... in one day") without naming each one gets a single row with that count, which is
  honest about what is known rather than inventing distinct incidents to hit a row-per-count shape.
- **status.** `open` (nothing has changed about the rule since), `escalated` (a higher-rung
  mechanism now exists, and the row stays as the record of why), or `resolved` (the rule itself is
  gone, e.g. deleted as unenforceable). Only `open` rows count toward a strike.
- **source.** Where the incident is on the record. A lane report or a pull request is fine; today's
  three rows are backfilled from `AGENTS.md`'s own prose and from milestone 118's roadmap block
  (design/roadmap/118-constitution-budget.md), because those are what recorded and measured the
  incidents, not because a lane report is a worse source than a constitution.

**Three strikes on an `open` rule, and it must move up the ladder or be deleted as unenforceable.**
`AGENTS.md`'s own ladder (Nobody remembers...) names four rungs; a rule stuck at rung four (prose)
after three strikes has demonstrated that prose is not the rung it needs. Moving it is an edit to
`AGENTS.md`, which is calef's or the integrator's, never a developer lane's: this ledger's job ends
at naming which rule has crossed the line, not at deciding what replaces it.

## What it found on its first run, 2026-08-22

Backfilled from the incidents `AGENTS.md` and milestone 118's own roadmap block record, listed
above. **One rule is already past the threshold**: *"a destructive git operation... without
committing or stashing first"* carries four open strikes from a single day (2026-08-04), one more
than the "three strikes" the convention describes, and exactly the kind of decision this ledger
exists to surface rather than make. `AGENTS.md` already contains a lot of prose about this (the
"move fast on what can be undone" section, the squash-against-base-SHA scar, the worktree-pruning
warnings) but no mechanism: nothing stops the command, only a git alias or wrapper requiring
confirmation, a pre-command hook, or similar could. **That is a real open decision and this lane is
not deciding it**: see milestone 118's status for the finding, recorded rather than resolved.

The other two rules (`squash against origin/main`, `pkill` a QEMU process by pattern) sit at one
strike each and are informative rather than urgent.

## EXAMPLES

Add a row when a lane's report honestly says it violated a documented rule, or when a maintainer
finds one in a merged pull request's history:

```markdown
| 2026-08-22 | never `pkill` another lane's QEMU | 1 | open | PR #309's report |
```

Check the ledger, against the three rows this note carries today:

```
$ script/rule-violations
rule violations: 3 rows, 6 open instances across 3 rules
   4 strikes  (at threshold)  [open     ] a destructive git operation (`reset --hard`, `checkout`, or `stash`) used to discard changes or "take a measurement" without committing or stashing first
   1 strike                   [open     ] squash against `origin/main` instead of the recorded base SHA
   1 strike                   [open     ] `pkill` a QEMU process by name/pattern instead of walking the process tree from the harness that owns it
```

```
$ script/rule-violations --check
rule-violations: a destructive git operation (`reset --hard`, `checkout`, or `stash`) used to
discard changes or "take a measurement" without committing or stashing first has 4 open strikes
(2026-08-04, 2026-08-04, 2026-08-04, 2026-08-04; see notes/rule-violations.md), which is the
promotion threshold (3).
  Move it up the ladder (a mechanism, in AGENTS.md's ladder sense) or mark its row `resolved` as
  unenforceable in notes/rule-violations.md; either is calef's or the integrator's call, never a
  lane's.
$ echo $?
1
```

## BUGS

- **Not wired into `script/lint` or CI.** `--check` correctly fails today, because the git-clobber
  rule is already at four strikes; wiring that failure into the mandatory gate suite would block
  every lane's pull request over a decision that belongs to calef or the integrator, which is exactly
  the failure mode DECISIONS §61 warns about for an ordinary lint ("adding a lint is a commitment to
  fix every existing violation first"). Whether and when to wire this in is itself an open decision,
  named here rather than made.

- **"The rule" is matched by exact text, not by meaning.** Two rows describing the same rule in
  different words are counted as two different rules. A human curating the table has to normalize
  wording by hand; nothing catches a near-duplicate.

- **Depends on honest self-reporting**, restated from the top of this note because it is the whole
  limitation: a rule violated and never mentioned in any report leaves no row and no strike. The
  ledger's number is a floor on how often a rule was actually broken, never a ceiling.

- **A `resolved` or `escalated` row's instances still print in the report, but do not count toward a
  strike.** That is deliberate (a resolved rule should not keep failing `--check`), but it means the
  human-readable total ("6 open instances") is not the same as "6 instances ever recorded" the moment
  the first row is marked otherwise, and nothing calls that out beyond this paragraph.
