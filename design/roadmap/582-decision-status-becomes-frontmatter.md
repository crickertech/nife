# 582. A decision's status becomes a field, and the index becomes generated

**Status: BUILT.** Raised by calef on 2026-09-23: *"I wonder if the key values should be front
matter instead."* He ratified the schema the same day. *(Number provisional until the merge queue
lands it. The brief said 580, which was taken on `main` at this lane's base commit `bfc91c8d2` by
`580-nobody-reads-branches.md`. This lane then took 581 and collided with another lane that could
not see it, which is the collision `AGENTS.md` gives the integrator to resolve at merge; pull
request #1193 keeps 581 and this block moved to 582.)*

## Why a prose field was the defect

`**Status: DECIDED.**` is a data field written as a sentence, and `script/decisions` read it with
`re.match(r"\*\*Status: (.+?)\.\*\*", first)`. Two failures on record come from exactly that shape.

A file said `**Status: NOT YET`, the regex captured `NOT`, dropped `YET`, and the report printed a
word nobody had written. That is §211 (what a fatal-risk verdict says, and what the chart can plot as a result)'s finding.

`design/fatal-risks.md`'s risk 7 carried two status lines eighteen lines apart. The script takes the
first match per section, so `AUDITED` was invisible to every consumer for weeks.

A schema removes that class. A tighter regex would have fixed the two instances. That is rung one of
`AGENTS.md`'s ladder against rung two, and the ladder says to take the higher rung when it fits.

## The second payoff, which is the larger one

`design/decisions/README.md` repeated every section's number, status and title, and part of what
`script/decisions` did was check that a file and its row agreed. Generated from the files, that
index stops being hand-maintained. `briefs/rebase-onto-main.md` names it as the worked example of an
additive-index conflict, case 4, because every lane minting a section edited it. That conflict class
is now gone, and the brief's example is stale. Naming it is this block's job; editing that brief is
not, since it sits on another branch in flight.

## The schema, ratified by calef on 2026-09-23

```yaml
---
status: DECIDED
raised: 2026-09-23
decided: 2026-09-23
ratified_by: calef
---
```

| key | values | required when |
|---|---|---|
| `status` | `PROPOSED`, `DECIDED`, `AMENDED`, `SUPERSEDED` | always |
| `raised` | `YYYY-MM-DD`, UTC | always |
| `decided` | `YYYY-MM-DD`, UTC | `status` is `DECIDED` or `AMENDED` |
| `ratified_by` | GitHub username | `status` is `DECIDED` or `AMENDED` |
| `superseded_by` | a section number | `status` is `SUPERSEDED` |

Values are uppercase, matching the vocabulary already in readers' heads. Keys are snake_case.
`superseded_by` splits today's compound `SUPERSEDED BY 42` token into a value and a pointer, so a
gate can check the target exists, which it could not before. There is no `number` key and no `title`
key, because both already live in the filename and the H1, and a third copy is a third thing to
disagree. There is no `branch` key, because a branch is transient and the pull request records it.

The rule that ships with the schema: frontmatter is authoritative, and the prose may not restate the
status. Two copies of one fact in one file is the defect this removes.

## What the gate can and cannot say about a restated field

The gate refuses a `**Status: ...**` token anywhere outside a code fence, and it refuses an opening
sentence of the field-shaped forms `Ratified by <user> on <date>` and `Decided <date>`. It does not
refuse an attributed sentence like `calef, 2026-09-18: <ruling>`, which 130 files use and which is
voice rather than a duplicated field. A gate that stripped those would make 130 files worse to read
in exchange for removing no ambiguity, since the frontmatter is the field either way. That limit is
recorded in the script's own `BUGS` section, where a reader meets it.

## The provenance the tree never wrote down

This is the finding calef should see, because the schema was ratified before anyone had counted.
Migrating 211 files recovered `status` for all of them and `superseded_by` for all four that need
it. The historical corpus records the rest far more thinly:

| key | filled from the file's own prose | of |
|---|---|---|
| `status` | 211 | 211 |
| `ratified_by` | 79 | 168 |
| `decided` | 107 | 168 |
| `raised` | 59 | 211 |

Every value was taken from the prose or left out. A date was filled only where a keyword anchored it
to a role, never from a bare date whose role was ambiguous and never from git history, because the
date a decision was committed is not the date it was raised.

`ratified_by` is the key where that rule was hardest to keep, so it is worth stating why it was kept.
The first version of this migration wrote `ratified_by: calef` on all 168, reasoning that this tree
has one architect, that every decision is his by the constitution, and that no other name appears as
a ratifier anywhere in the directory. All three are true. The key is still absent on the 89 files
whose prose does not name him, because a machine-readable field holding a rule rather than a record
is one a later reader cannot tell from evidence, and because a lane that refuses to infer 213 dates
and then infers 89 ratifiers has a preference rather than a standard. Nobody should fill them in from
that inference, which is why the gap list says so where a reader meets it.

So 152 files record no raise date, 61 record no decision date, and 89 name no ratifier. Inventing any
of them would put a false fact in the field whose job is not lying, and `script/names` already treats
a missing ratification date as a defect rather than a blank to fill. The 302 gaps are written to `design/decisions/PROVENANCE-GAPS.md`, which
`script/decisions --check` enforces as a ratchet: a gap already on the list is a worklist item, a
gap that is not is a failure. 302 entries. Every new decision file therefore carries the full
schema, and the backlog can only shrink. It is the shape of `script/names --unratified`, a worklist rather than a
wall.

## Index row

**Built:** 2026-09-23

A decision's status stops being a sentence a regex has to guess at and becomes a field, and
`design/decisions/README.md` stops being hand-maintained, which deletes the additive-index conflict
every lane minting a section used to pay.

## Follow-on

- **Proposed.** `design/roadmap/proposals/the-roadmap-blocks-get-frontmatter-too.md`, the same
  migration for `design/roadmap/`. It is 578 files against this one's 211, its status vocabulary is
  eight tokens rather than four, and `script/roadmap` reads far more out of prose than
  `script/decisions` did. It should wait until this pilot has held.
- **Recorded.** The gate cannot refuse an attributed sentence that repeats a date the frontmatter
  holds, and the reason is in `script/decisions`' `BUGS` section.
