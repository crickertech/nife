# nife: user journeys

*Name: calef's own word, 2026-08-26 ("I'd like to start tracking user journeys"), not a lane's
choice to make provisional.*

A journey is an end-to-end, user-facing story, told as an ordered bundle of the milestones (and,
where a step is blocked by one, the decisions) that have to be true at once for someone to actually
live it. `design/roadmap/` tracks work item by item; a journey is the read that answers a different
question than any single milestone can: *could a person actually do the whole thing today, start to
finish?* The login-to-`kilo` conversation that minted this directory is the worked example: no
single milestone was "blocked," but tracing the story end to end found a real, unowned gap
(milestone 177) that nothing surfaced by scanning the roadmap milestone by milestone.

## How this directory works

One journey, one file, named `slug.md` (hyphenated, this project's convention for ordinary
markdown outside the repo root). No number: a journey is not cited by other files the way a
milestone or a decision is (`§N`, `milestone N`), because a journey names no scope of its own to
be cited *for*; it only ever points at scopes that already have one.

**A journey has no status field of its own, and that is deliberate.** A hand-written "70% done" is
exactly the kind of claim this tree's own `counted-claims` mechanism exists to replace: a number
nobody re-derives drifts the moment one of its steps moves. `script/journeys` computes a journey's
state by reading `design/roadmap/README.md` and `design/decisions/README.md`, the same indices
`script/roadmap` and `script/decisions` already treat as ground truth, so a journey file records
only what a human has to write: which steps, in what order, and why each one is on the path.

## The format

A journey file is a short narrative (what the story is, why it matters) followed by one table:

| step | milestone | decision | what this step needs |
|---|---|---|---|
| 1 | 33 | | one line naming what this step of the story needs from that milestone |

- **step.** Ordinal, for reading order only; `script/journeys` does not enforce that a lower step
  finishes before a higher one starts; some journeys have steps that are genuinely independent (the
  login-to-`kilo` journey's own steps 3 and 4 are unordered with respect to each other, and its own
  prose says so).
- **milestone.** The milestone number this step's progress is read from. Required.
- **decision.** A `§N` this specific step is additionally blocked by, when the milestone's own
  status does not tell the whole story (a `PARTIAL` milestone can still have the one piece a journey
  needs sitting behind a separately-decided hold, DECISIONS §120 on milestone 49's login-boot-wiring
  piece is the worked example). Blank when the milestone's own status is the whole answer.
- **what this step needs.** One line, human-written, naming the specific slice of that milestone
  this journey's story needs, not the milestone's full scope. This is the column a milestone's own
  coarse status cannot carry: `PARTIAL` says nothing about *which* part is missing.

## Reading a journey's state

`script/journeys` prints, per step, the milestone's status from the roadmap index and (when a
decision is cited) that decision's status from the decisions index, then a one-line rollup: how many
steps sit on a `BUILT` milestone with no additional decision blocking them. A step whose milestone is
`BUILT` but whose cited decision is not `DECIDED` in the direction the step needs still reads as
blocked; the tool does not try to parse *which way* a decision went; it says what a step needs and
lets a reader hold the decision's own text against it.

**This is a report, not a gate.** Nothing fails a build over a journey's state, the same way
`script/roadmap --ready` reports rather than blocks. A journey can regress if a milestone it depends
on gets re-scoped down to `PARTIAL`, and that is information, not a defect for CI to catch.
