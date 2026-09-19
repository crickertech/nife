# 434. Retire the `Proposed.` disposition and the machinery that read its directory

**Status: NOT-STARTED.** Minted 2026-09-19 by this lane under the convention it exists to finish:
the ruling of the same day says a lane that identifies work writes the numbered block itself, with
the number **provisional**, so this is that rule's first use. *(Number provisional. Milestone numbers
are the integrator's at merge; on a collision the newer file moves and the older number stands, per
`design/roadmap/README.md`.)*

**Gate: NONE.** The decision is already made and written down in two places. What is left is the
code that has not caught up.

## The contradiction this closes

calef ruled on 2026-09-19 that `design/roadmap/proposals/` should not exist, milestone 433 drained
all 106 files out of it, and two records were rewritten to say so:
`design/decisions/140-follow-on-disposition-vocabulary.md` retired `Proposed.` and left six words,
and `design/roadmap/README.md` says a lane writes the numbered block itself.

**The scripts said otherwise.** `script/roadmap` still accepted `Proposed.` in a follow-on bullet,
still required that bullet to name a file under a directory that no longer exists, still carried a
`--proposed` mode that listed an empty pile and a `--unclaimed` mode whose only input was the
`Proposed.` branch, and still spent about sixty lines of header and section prose explaining a
queue nobody is in. `script/fatal-risks` still carried a check keyed on the same directory.
`script/metrics` still counted it.

A tree where the prose and the gate disagree teaches the gate, because the gate is what fails a
build. This block is the other half of the same afternoon's work.

## What was cut, and what was kept

**The reasoning was kept and the remedy was replaced**, which is the distinction that decided every
paragraph here. `Proposed.` was not a mistake: it named a real hole in the vocabulary, and the
account of that hole is still true. Three lanes on milestone 247's sweep found work a block had
named honestly and nobody had taken, found that `Recorded.` lies about intent and `Refused.` lies
about the decision, and **left the item out**, which is the burial arriving through the gate that
exists to stop it. What changed is only where the work is written down: the word needed a file to
point at, the file needed a number, and the number is what a lane could not mint. The 2026-09-19
ruling removed that constraint instead of routing around it.

So every cut below either carries its argument forward to the thing that superseded it or says
plainly that the argument went with the directory.

### `script/roadmap`

- `Proposed` is out of the `DISPOSITION` regex, out of the `elif kind ==` chain, and out of the six
  error messages that spelled the vocabulary. The header's account of why the word existed is kept,
  compressed, under a heading that says it is retired and names what replaced it.
- `--proposed` and `--unclaimed` are gone. `--unclaimed`'s only source was the `Proposed.` branch,
  so it could only ever print nothing; the `--check` line that reported its count is gone with it.
- `PROPOSED_DIR` is gone, and with it the `os.listdir` of the pile, the five per-file diagnostics,
  and the import of `scripts/roadmap_proposals.py`.
- **The directory can no longer come back in silence, and that is a stronger guard than the one
  removed.** The file loop used to skip `proposals` by name. It no longer does, so a
  `design/roadmap/proposals/` that reappeared would fail the check that already says nothing but
  milestone files lives here. That is rung one wearing an existing check's clothes: the wrong state
  is not gated, it is unrepresentable in a passing tree. Proved by making the directory and reading
  the red.

### The promoted-proposal check, which is deleted rather than kept as a guard

Added 2026-09-18, it failed a numbered block claiming promotion from a proposal that was still
sitting in the pile. **It is deleted**, for this script's own recorded reason rather than for
tidiness: its whole input was `os.listdir` of a directory that cannot exist, so it could not come
back red on any tree. `script/roadmap`'s header already says what that costs, about its own
merged-branch check, which *"spent a day passing in CI while being unable to fail"*: a check that
cannot fail is worse than one that is absent, because its green is read as an answer.

Keeping it as a guard against the directory returning would have been the softer call and the worse
one, because the file loop above now guards that outright, and two checks answering one question is
the shape where the second stops being updated.

### `scripts/roadmap_proposals.py` survives, cut to its parse half

Deleting it was the expected answer and it is the wrong one, for a reason that is specific to how
`script/metrics` works. That dashboard is a **restatement**: `--backfill` rewrites every week's row
by applying today's definitions to old commits, reading blobs at revisions nobody has checked out.
The `proposals_unnumbered` column is 74 at 2026W36 and 93 at 2026W38, and those two numbers are
true about the tree at those revisions. Delete the parse and the next backfill writes **zero** into
both, which is a dashboard silently lying about a fortnight that happened. The rise and fall of
that directory is the measurement §140 and 433 both rest on, so it is the last thing to erase.

What went is `promoted_from` and the promotion commentary around it, whose only caller was the
check above. What stays is `DIRECTORY`, `filename_problem` and `classify`, with a header that says
the form they parse is historical and says why one caller is enough to keep it.

Milestone 236's rule (a derivation two scripts copy will drift while both look authoritative) no
longer applies, since there is one caller now. The module stays a module anyway: it is the place a
reader meets the explanation of why a column on a live dashboard is permanently zero going forward,
and folding fifty lines of a dead record form into the middle of the dashboard would bury that.

### `script/fatal-risks`

Check 6b asked whether a proposal cited by a risk entry had changed after the entry was last dated,
using git rather than a recorded date because a proposal had no completion to record. `design/fatal-risks.md`
cites no proposal today and can never cite one again, so 6b and its two fixtures
(`proposal-moved`, and the `no-history` refusal that existed to keep 6b from passing vacuously in a
shallow clone) are gone, along with the `last_touched` argument that fed only them. The
`missing-path` fixture kept its check and lost its proposal-shaped path.

**The question 6b asked is still good and is deliberately not generalised here.** "A cited artifact
that would answer this risk changed after the entry was written" applies to numbered blocks too,
and check 6a only reaches the ones that turned `BUILT`. Widening it to every cited path would trade
the one-path noise surface that script's header deliberately chose for a tree-wide one, which is a
judgement about what a gate should accept rather than a consequence of this ruling. So it is
recorded where a reader meets the check, in that script's own header beside check 6, rather than
decided here.

## BUGS

**Nothing re-reads a script header.** The prose cut here had been wrong since the moment 433's last
lane merged, and no gate in this tree can tell a stale paragraph from a current one. What caught it
was a person reading the file to change it, which is rung four, and this block does not fix that.

**About forty citations of `design/roadmap/proposals/<slug>.md` still dangle**, in milestone blocks,
notes, two audit reports and one shell script. They are untouched here and are milestone 435:
433 chose deliberately to keep the slug on promotion so that a reader can resolve one with a single
`ls design/roadmap/ | grep <slug>`, which makes them readable but does not make them true.

**`script/metrics --check` is red on this branch and was red on its base.** The CSV is regenerated
by `.github/workflows/metrics.yml` on a schedule, this week's `proposals_unnumbered` will fall to 0
when it next runs, and nothing here touches the generated file.

## Index row

The tree contradicted itself for an afternoon. calef retired the `Proposed.` disposition and the
`design/roadmap/proposals/` directory on 2026-09-19, milestone 433 drained all 106 files out of it,
and the records were rewritten to say so, while `script/roadmap` still accepted the token, still
required it to name a file under a directory that no longer existed, still carried `--proposed` and
a `--unclaimed` mode that could only print nothing, and still spent sixty lines of header prose on
a queue nobody is in. `script/fatal-risks` and `script/metrics` each kept a piece of the same
machinery. This block cuts the token, the two modes and the two checks that can no longer fail,
keeps the argument for why the word existed beside what superseded it, and turns the file loop's
existing "nothing else lives here" check into the guard that the directory cannot come back in
silence. `scripts/roadmap_proposals.py` survives on one caller, because `script/metrics --backfill`
restates every week from git history and deleting the parse would rewrite two true weeks to zero.
It is also the first use of the convention it finishes: this block was minted by the lane that
wrote it, with a provisional number.
