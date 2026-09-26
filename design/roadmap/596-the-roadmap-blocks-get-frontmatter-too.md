# 596. The roadmap blocks get frontmatter too

**Status: IN-PROGRESS.** On `milestone/596-roadmap-frontmatter`, promoted from the proposal
`the-roadmap-blocks-get-frontmatter-too` on calef's ruling of 2026-09-26 (UTC): *"Roadmaps should
shift from bolder sections to front matter. We did a trial and it seems to be working. So let's
roll it out."* The trial is milestone 582 (a decision's status becomes a field, and the index
becomes generated). The number is provisional; 595 was the highest claimed when this lane cut its
branch, and the integrator mints it at merge. This is the last block written in the old form.

**Gate: DECISION.** The schema is an architect's call, because key names are a format several
scripts agree on. Phase 1 below counts what the migration faces and proposes the schema; nothing
migrates until it is answered.

## What the proposal said, kept

`design/roadmap/` carries the defect `design/decisions/` carried until 582. A block's status is a
sentence, and `script/roadmap` recovers the status, the Built date, the gate, the branch and the
promoted-from slug with regexes over paragraphs. The migration is 582's four steps: a schema, a
mechanical rewrite, scripts that read fields, and a rule that prose may not restate a field. The
proposal gated itself on 582 holding as a pilot. calef's ruling says it has.

## Phase 1: what the migration faces

Measured 2026-09-26 over `main` at `716ae77ef`: 583 numbered blocks, one lettered addendum (20a),
and 18 proposals.

| status | blocks | Built date | gate line | other companion |
|---|---|---|---|---|
| `BUILT` | 258 | 258 of 258 | none | |
| `NOT-STARTED` | 217 | none | 217 | |
| `PARTIAL` | 44 | none | 44 | |
| `REFUSED` | 43 | none | none | 43 of 43 carry `## Revisit` (40 Condition, 4 Unstated bullets) |
| `SUPERSEDED` | 10 | none | 10 | prose names 2 to 5 milestones each |
| `RECORDED` | 6 | none | 6 | |
| `REMOVED` | 3 | 2 of 3 | none | |
| `OPTIONAL` | 2 | none | 2 | |
| `IN-PROGRESS` | 0 | | | none to check for a branch |

The vocabulary is nine tokens, not the proposal's eight: it left out `RECORDED`. Proposals add a
tenth, `PROPOSED`, legal only under `proposals/`.

The gate has already done most of the archaeology 582 had to do. No `BUILT` block lacks its date,
no `REFUSED` block lacks its `## Revisit`, and no `IN-PROGRESS` block lacks a branch, because the
gate fails all three. The expensive field is the one 582 also found expensive, a raise date.

Gates, 283 lines holding 291 tokens: `NONE` 168, bare `DECISION` 41, `DECISION §N` 29, `HARDWARE`
34, `MILESTONE N` 19. Combinations: 8 `HARDWARE` or `DECISION` pairs and 5 with a milestone.

Other facts a parser reads from prose:

- Promoted-from slug: 99 status paragraphs parse through `helpers/roadmap_proposals.py`, and 5 more
  mention a promotion in a shape the parser does not accept.
- The milestones a `REFUSED` status paragraph names, which `--revisit` excludes as self-citations:
  43 blocks, 42 of them naming two or more.
- 88 status tokens carry words inside the bold. 79 are `BUILT <date>` whose date equals the Built
  line, and 9 carry prose that has to move out of the bold verbatim. 46 status paragraphs are only
  the marker and vanish.
- 282 status paragraphs still carry a "number provisional" note from before they landed. Out of
  scope here; it is prose.
- None of §207's five dependency fields appears in any block. §207 (the roadmap is a graph, and the
  block says so in fields a script can walk) is `DECIDED` with no milestone building it.

Raise dates. 295 blocks state a mint or raise date in their first three paragraphs, unverified as to
which event it dates. From git, 87 blocks were first added by milestone 76 (split the roadmap) in
`6192cd358`, or by the backfill that followed it. Their origin is in `design/roadmap.md` history,
the same tracing 582 did through `DECISIONS.md`. 181 blocks were renamed after they were added and
need `--follow`.

Parsers of this prose: `script/roadmap`, `helpers/roadmap_proposals.py`, `script/journeys` (working
tree), and three that read old revisions, `script/catch-up`, `script/citations --moved` and
`script/metrics`. `script/fatal-risks` and `script/audits` read `script/roadmap --index` and need
nothing. The three history readers must keep their prose parser permanently, because the blobs they
read predate the flip. `script/catch-up` already does exactly this for decisions since 582.

## The proposed schema

Flat `key: value` lines, as 582's parser requires: no nesting, and a list is one comma-separated
value. Keys snake_case, values uppercase where they are a vocabulary, dates UTC.

| key | values | required when | from today's | blocks |
|---|---|---|---|---|
| `status` | the nine tokens; `PROPOSED` under `proposals/` | always | `**Status: X` | 601 |
| `raised` | `YYYY-MM-DD` | always | prose, else git's first add, earlier wins | 601 |
| `built` | `YYYY-MM-DD` | `BUILT`; optional on `REMOVED`; else forbidden | `**Built:**` line | 260 |
| `branch` | a branch name | `IN-PROGRESS` | backticked branch in the status paragraph | 0 |
| `promoted_from` | a proposal slug | optional | status-paragraph regex | 99 + 5 to read |
| `superseded_by` | a milestone number | `SUPERSEDED` | prose, chosen by reading | 10 |
| `refused_by` | milestone numbers, or `none` | `REFUSED` | every milestone the status paragraph names | 43 |
| `milestone_dependencies` | numbers, or `none` | where a gate is required today | `MILESTONE N` | 279 |
| `decision_dependencies` | section numbers, `unwritten`, or `none` | the same | `DECISION §N`, bare `DECISION` | 279 |
| `machine_requirements` | capabilities, or `none` | the same | `HARDWARE`, read by hand | 279 |
| `specific_machine` | host and reason, or `none` | the same | `HARDWARE`, read by hand | 279 |
| `needs_person` | `yes` or `no` | the same | `HARDWARE`, read by hand | 279 |

Where it mirrors 582: `status`, `raised` and `superseded_by` are 582's keys with 582's rules. No
`number` or `title` key, since the filename and H1 hold both. The prose may not restate a key:
`**Status:`, `**Built:**` and `**Gate:` at the start of a line outside a fence fail the gate, the
shape of `script/decisions`' `RESTATED`.

Where it does not: 582 refused a `branch` key because a branch is transient. Here `IN-PROGRESS` is
defined as a branch, and the check that it has not merged is the reason the token exists. There is
no `decided` or `ratified_by`: nothing in a milestone is ratified the way a section is.

What stays prose: the H1, the `## Index row` summary (a paragraph, and 582 kept prose out of the
frontmatter), the `## Follow-on` and `## Revisit` bullets, and the dates a block was refused,
superseded or removed, which no script reads. After `built` moves out, `## Index row` holds only the
summary, under a heading named for a table that no longer exists. Renaming it is a naming call and
not this milestone's.

The last five rows are §207's fields, which is the fork below. The smaller alternative is one `gate`
key holding today's tokens verbatim.

## Phase 2 onwards: the rollout, so it does not wreck lanes in flight

On 2026-09-26, 6 of 15 open pull requests touch `design/roadmap/`: #1324, #1321, #1318, #1311,
#1301 and #1289, the last touching 38 blocks. Each edits the top of its own block, which is where
the migration writes, so each would conflict with a hand-made flip.

1. Readers first. One shared parser that reads frontmatter and falls back to prose, used by every
   parser above. It lands like any pull request and breaks nothing, because both forms are legal.
2. A migrator in the tree (`script/roadmap --migrate [FILE...]`, provisional), deterministic and
   idempotent: same input, same output, and a migrated file is left alone.
3. The flip is a script run, not an edit. It is regenerated from current `main` rather than rebased,
   so its own conflicts cost nothing. In the same commit, `--check` starts rejecting the old form,
   with a message naming the migrator. It is enqueued when the queue is drained.
4. A lane rebasing across the flip takes its own side of any conflicted block and reruns the
   migrator on it. That is a new case in `briefs/rebase-onto-main.md`. A lane that rebases clean but
   wrote a new block in the old form fails lint, and the failure names the command.

## The bold touch rule (#1311), measured

The migration does not shrink the bold backlog, because #1311 already stopped counting bold a script
parses. On #1311's branch (`32afdfa2d`) the roadmap blocks carry 8,263 spans over budget in 494
documents. Migrated, they carry 8,270 in 494: seven more, because the removed marker words shrink
each document's allowance. The migration takes 1,126 parsed spans (status, Built and gate markers)
out of the working tree, and #1311 was not counting them. Without #1311's exemption the roadmap
figure would be 10,846. Most of what remains exempt is the 1,535 `## Follow-on` and `## Revisit`
tags, which this schema leaves as prose.

## Decisions owed to an architect

1. Fold §207 in now, or migrate `Gate:` verbatim as a `gate` key and build §207 later?
   Recommended: fold in. A `gate` key is born deprecated, and two migrations mean two rebase storms.
   Folding in also retires the forced edits the stale-gate checks impose, including milestone 591
   (a ruling should make the gate it answers fail until someone updates it), because §207 computes
   blocked-ness instead. It costs reading 34 `HARDWARE` blocks by
   hand. That is effort, and I would choose it at equal cost.
2. Ratify the key names in the table, including the snake_case forms of §207's labels?
3. Bare `DECISION` names no section in 41 blocks. May `decision_dependencies` say `unwritten`, listed
   by a worklist mode? Recommended: yes. Writing 41 sections is not this milestone.
4. Is `raised` required on every block, dated as 582 dated sections? Recommended: yes. It costs
   tracing 87 blocks into `design/roadmap.md` history.
5. A `branch` key for `IN-PROGRESS`, though 582 refused one? Recommended: yes, for the reason above.
6. Do proposals take the same frontmatter, with `status: PROPOSED`? Recommended: yes, so promotion
   is a `git mv` and two key edits.
7. `superseded_by` is one number, as 582's is, and `refused_by` a list copied from today's regex?
   Recommended: yes.

## Index row

A milestone's status, dates and dependencies stop being sentences a regex guesses at and become
fields, the migration milestone 582 piloted on `design/decisions/`, extended to the 583 blocks every
lane edits.
