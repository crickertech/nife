# Briefs a rented model can be handed

Each file here is a complete instruction for one recurring piece of **maintainer** work, written so
`scripts/open-lane.sh` can hand it to a rented open-weight model:

    scripts/open-lane.sh <worktree> briefs/<brief>.md

**Why these exist as files rather than as something a maintainer types each time.** On 2026-09-22 a
delegated rebase cost **$0.055**, was correct first time, needed no redo, and replaced about
thirteen of a maintainer's tool calls with four. It worked for one reason: **the brief encoded
judgement that had already been made.** Take `main`'s baselines, never hand-merge them, accept the
retired index's deletion, and stop rather than guess at anything else. A model following that is
doing lookup, not judgement, and lookup is what a cheap model is good at.

Written from memory each time, that brief would lose a clause a month, and the clause it loses is
the one that stops a wrong conflict resolution shipping as housekeeping.

## What belongs here

Work that recurs, whose failure modes are already known, and whose correctness a gate can check.
Rebases, triage, sweeps. **Not** design, not prose a reader will trust, not anything touching the
syscall surface or a wire format: DECISIONS §202 (mechanical work goes to a cheaper model) has the
routing rule, and the decision recording the measurements behind it was taken the same day and is
not yet on `main`.

## The rule every brief here follows

**Say what to do about the failures that are known, and say "stop" about everything else.** A brief
that tries to anticipate every case teaches a model to improvise; a brief that names three
resolutions and demands an abort on the fourth gets a model that hands back a clean worktree when it
meets something new. The rebase brief is the worked example.

## BUGS

- **`--bare` skips `AGENTS.md`**, so a brief inherits none of this project's rules. Every brief has
  to carry what it needs: that citations want glosses grounded in the target's H1, that a lane never
  edits another milestone's block, that invented names are provisional.
- **Nothing here measures the redo rate**, which is the number that decides whether any of this
  saves anything. A delegated task that passes its gates and must be redone by hand costs more than
  never having delegated it. One sample so far, and it needed no redo.
