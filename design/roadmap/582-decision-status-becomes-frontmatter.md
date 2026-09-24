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

## The provenance the tree never wrote down, and where it was found

The schema was ratified before anyone had counted, and the corpus predating it states its own
provenance thinly. Of 211 files, 59 state a raise date and 106 a decision date.

The rest came from the repository, on calef's ruling of 2026-09-24 that a decision's date is the
date its commit went to decided, and that a raise date is the date it was minted. `raised` is the
file's first commit, found with `--follow` so a renumbered file keeps its origin, merged with a
plain walk because `--follow` truncates at the rename. `decided` is the first commit whose text
reads DECIDED or AMENDED, in either spelling, since history holds both. Author dates, UTC.

| key | from the prose | from git | total |
|---|---|---|---|
| `status` | 211 | 0 | 211 |
| `raised` | 59 | 152 | 211 |
| `decided` | 106 | 62 | 168 |
| `ratified_by` | 168 by ruling | 0 | 168 |
| `superseded_by` | 4 | 0 | 4 |

Where both sources speak, the earlier wins. Each is an upper bound on when the thing happened, so
the earlier is the tighter bound, and the difference is not small: git dates the 77 files created by
milestone 114 (split `DECISIONS.md`, and give a decision a status)'s split commit to 2026-08-04, while §10 (process model: capability-based, microkernel)
says in its own words that it was decided 2026-07-14. Twenty-nine files then claimed a decision date
before their raise date, which is what an upper bound taken from a later commit looks like beside a
stated fact, so `raised` is clamped to `decided` in those.

A derived date therefore means "no later than", and that is recorded once in
`design/decisions/README.md` and once in `script/decisions` rather than per file. A second key or a
per-file marker would be machinery for a caveat that applies to two thirds of the corpus uniformly.

## Why `ratified_by` came out and went back

This block records the round trip because the value is identical and its provenance is not, and
because the difference is the one a later reader cannot reconstruct.

The migration first wrote `ratified_by: calef` on all 168 decided sections, reasoning that this tree
has one architect, that every decision is his by the constitution, and that no other name appears as
a ratifier anywhere in the directory. A reviewer refused it, correctly on the evidence available:
this lane had just declined to infer 213 dates on the grounds that the field whose job is not lying
must not hold a reconstruction, and a lane that then infers 89 ratifiers has a preference rather
than a standard. The key was blanked on the 89 files whose prose does not name him.

calef then ruled, on 2026-09-24: *"ratified_by: calef on all 168. [He is] the only ratifier to
date."* That is the architect stating a fact about his own role, which is not the act the refusal was
aimed at. The key is back on all 168, and nobody should read it as the inference that was refused.

## The gap ledger, and why there is none

There was one, `design/decisions/PROVENANCE-GAPS.md`, holding 302 entries and enforced as a ratchet:
a gap named there passed, a gap that was not failed. It was the right answer while the tree held
facts nobody had written down, and it is the shape of `script/names --unratified`, a worklist rather
than a wall.

Deriving the dates emptied it. An allowlist with nothing on it is weaker than no allowlist, because
it offers an escape a lane can reach for instead of doing two `git log` calls, so it is gone and the
required-when rules are hard failures. That is rung one where the ledger was rung two.

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
