# The stranger test: the instrument, the rubric, and the runs

Milestone 117 turns the third principle into a measurement. The principle is CLAUDE.md's own
sentence: *could a competent stranger, with only this repository, reach a passing build and a correct
mental model without opening a chat window?* Where the answer is no, that is a bug in the tree and
not in the stranger.

This note is the instrument. **It is written before any run**, deliberately, because a rubric written
afterwards grades generously: every answer looks close enough once you know what it was supposed to
be.

## Why the tree cannot grade itself

calef cannot take this test; he wrote the system. Nor can any agent that has worked in this tree, and
by 2026-08-14 that is most of them. An agent that spent a night merging pull requests here knows why
`nife-dev` is a symlink, what a lane is, and that `script/lint` fails on a branch prefix.
**Knowing the answer disqualifies you from being the instrument.**

So the run needs a fresh context that has never seen the repository, handed the repository and a
task, and nothing else. No brief explaining the conventions. No pointer to the right note. No answer
to any question it asks.

## The protocol

1. **Fresh context, not a summarised one.** A handoff that says "read CLAUDE.md first" has already
   given away the finding a newcomer would not know to make.
2. **One task, stated the way a new contributor would receive it.** Not "evaluate the docs", which
   invites a review rather than an attempt.
3. **Every question it asks is a defect**, recorded verbatim. The questions are the deliverable, more
   than any score is.
4. **Every confident wrong answer is a worse defect**, because a document that misleads costs more
   than one that is silent.
5. **No help mid-run.** If it is stuck, that is the measurement finishing, not a prompt to intervene.

## The harness

The history of how the harness was built is in [the instrument appendix](stranger-test/instrument.md).

## The rubric, written 2026-08-14, before the first run

Two halves. Only the first is mechanical.

### The build

From a clean clone: does `script/setup` then `script/test` reach green, following only what the
repository says?

| # | checked | pass means |
|---|---|---|
| B1 | a reading order exists | the stranger knows where to start without guessing among `README.md`'s sections |
| B2 | `script/setup` completes | or fails with a message that says what to install |
| B3 | `script/test` reaches green | on at least one architecture |
| B4 | nothing undocumented was needed | no step the reader had to know that no file states |

**Record what it actually took**, including anything the reader had to know that no file said. That
last row is the one that matters; the first three are hygiene.

### The mental model

Each question below is one the tree *claims* to answer. **Grade against what the tree actually says,
not against what a maintainer knows.** A question the tree answers only in a commit message is a
question the tree does not answer.

| # | question | pass means |
|---|---|---|
| M1 | What is a capability here, and what is the relationship between holding a reference to a thing and being permitted to use it? | names that holding the reference *is* the permission, with no separate check |
| M2 | Why is there no ambient network, and what must a program hold to reach one? | names a held capability rather than a config flag or a permission bit |
| M3 | Where does architecture-specific code live, and what breaks if it lives elsewhere? | `kernel/src/arch/`, and that the port becomes a diff across every file |
| M4 | What does `BUILT` mean on a roadmap row, and `PARTIAL`? | that it is a claim about the tree, and that the index and the block must agree |
| M5 | Why is there a `crates/` and a `components/src/`, and what decides which? | shared-by-two-binaries goes in `crates/`; host-testable and Kani-reachable is the reason |
| M6 | What is a `BUGS` section for? | a promise about known limits, not an apology, and next to the feature |
| M7 | How would you add a program, and what must you declare about it? | the grant manifest, and that a provisional name is expected |
| M8 | Who decides a name, and what provenance states can one be in? | calef; the states `script/names` accepts, which is four as of §89: `ratified`, `recorded`, `unrecorded`, `provisional` |


## The honest limits, stated before the result exists

**An agent is not a person.** It will not get bored, will not give up out of frustration, and will
read further before asking than a human would. So every number this instrument produces is a **lower
bound** on the friction a real newcomer would meet, and any report of it must say so.

**One run measures; two show whether the fixes worked.** A single pass is an audit. The milestone is
the second run, with a different stranger, after the worklist is fixed.

**The rubric can be wrong.** These eight questions are what the tree claims to answer as of
2026-08-14. If a run shows a stranger falling down somewhere the rubric does not ask about, the
finding is real and the rubric is what needs amending.

## The cadence: monthly, decided 2026-08-18


**A run is due every 30 days.** Thirty rather than a calendar month because a month is not a number
of days and the difference cannot matter to a signal that is asked once a week. The sentence above
is the cadence in the literal sense: `script/stranger-test --due` matches that line and reads the
integer out of it, so the interval a reader meets and the interval the tripwire uses are the same
characters. Changing the cadence is editing that sentence, and there is nowhere else to edit.

**The mechanism is notification, and it cannot be anything else.** A run spawns a `claude` process,
spends real budget, needs a machine with the pinned toolchain and both QEMUs, and ends in a debrief
that a person scores against the rubric above. No part of that is a thing CI can do, and pretending
otherwise would be worse than the gap it closed. So:

- `script/stranger-test --due` exits 1 when a run is due and prints the command to run, plus what a
  run costs. It runs nothing.
- `.github/workflows/stranger-cadence.yml` asks it every Monday, in its own workflow rather than in
  `script/lint`, because a run coming due is information about the tree and not a defect in whichever
  commit happened to be pushed that week. Same split, and the same argument, as the audit cadence.
- `script/stranger-test --check` is the structural half and does run in `script/lint`: the cadence
  sentence appears exactly once, the run headings are numbered from 1 without a gap, and their dates
  are in order. A malformed record **is** a defect in the commit that malformed it, and it silently
  moves the date the tripwire reads.

**Red means run the test.** Closing it by editing a heading is available, cheap, and the one thing
that makes the whole mechanism a lie.

## Runs

### Run 1, 2026-08-14: an x86_64 container, no QEMU

Recorded in full in [run 1](stranger-test/run-1.md).

### Run 2, 2026-08-16: a stock Linux box, and a stranger that was not one

Recorded in full in [run 2](stranger-test/run-2.md).

### Run 3, 2026-08-18: a process that could not see the repository, and a harness that told it anyway

Recorded in full in [run 3](stranger-test/run-3.md).

### Run 4, 2026-08-18: the harness stopped leaking and the tree took over the job

Recorded in full in [run 4](stranger-test/run-4.md).

### Run 5, 2026-08-18: the first run through the harness, and the machine leaked where the tree did not

Recorded in full in [run 5](stranger-test/run-5.md).

### Run 6, 2026-09-19: a different model, a program in two edits, and a stranger that provisioned the machine

Recorded in full in [run 6](stranger-test/run-6.md).

## BUGS

Every entry, with its history, is in [the BUGS history](stranger-test/bugs-history.md).
