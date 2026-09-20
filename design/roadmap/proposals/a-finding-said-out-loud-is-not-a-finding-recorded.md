# A finding said out loud is not a finding recorded

**Status: PROPOSED 2026-09-20.** Raised by calef, who asked for *"improved rigor around capturing
all of the action without my prompting"* after a day in which he had to ask twice what had not been
written down, and got four items both times.

**Gate: DECISION.** The substance is an amendment to `AGENTS.md`, which is calef's file and which no
agent may edit. Everything else here is either already built or a few lines.

## The hole, stated precisely

`AGENTS.md` already binds a **lane**: *identified work leaves the lane in a tracked form, or the
merge waits*, in exactly two shapes, a proposed milestone or a recorded limitation where a reader
meets the feature. That rule works. Milestone 448 (a refusal gets a number, a status, and a
condition that would change it) exists because the same rule had no word for a **decision not to**,
and the refusals that named work had nowhere to live.

**There is no counterpart for the maintainer, and the maintainer is the one doing the talking.** A
lane reports once, in writing, to one reader who is processing it. A maintainer session says dozens
of things to calef across a day: a defect met while resolving a conflict, a gate that lied, a claim
of a lane's that turned out to be stale, a deferral. Each is a finding with the same standing as a
lane's, and none of them has a rule attached.

The day this was raised produced at least these, none of which reached the tree until calef asked:

- `script/roadmap --write` emits an index row that `script/roadmap --check` rejects, and refuses to
  run at all while the bad row is present, so the fixer cannot fix its own output.
- `script/metrics --check` reports stale immediately after any commit, because the current week's
  row records HEAD's sha.
- Milestone 447 (a thread's vector registers are its own) found that RISC-V's boot hart never calls
  `arch::init`, and named the class: anything else assuming that hook runs per core has the bug.
  Three call sites, no audit.
- A lane's own correction went stale before its pull request merged, because it measured the tree it
  branched from and `main` moved within the hour.

## What cannot be mechanised, said first

**Nothing can read the conversation.** No gate can know that a maintainer said something to calef
and did not write it down, and a lint that hunted for finding-shaped English in reports and pull
request bodies would be `git grep -w TODO`'s 82% false-positive rate wearing a different hat: a
report *discussing* a limitation reads exactly like one *reporting* a new one. That was measured
here (notes/untracked-work-sweep.md) and the conclusion has not changed.

So the mechanism cannot detect a miss. **It can make the absence visible at the moment it happens**,
which is a different and achievable thing.

## The proposal, three parts, strongest first

### 1. Capture, then report

**Every finding in a message to calef carries the path where it lives.** A finding the maintainer
cannot cite a path for gets one before the message is sent, or the message says plainly that it is
uncaptured and why.

The rung this sits on is honest: it is rung four, a rule somebody has to follow. What makes it
stronger than the rule it replaces is **who checks it and when**. Today a missed capture is
invisible until calef asks, hours or days later, and asking requires him to remember a thing he was
told once. Under this rule the omission is visible **in the same message he is already reading**, as
a finding with no path after it. That converts a memory problem into a proofreading problem, and the
reader is already there.

It is the same inversion the tree applies elsewhere: `script/citations` does not check that a gloss
exists, it checks the ones that are written, and the discipline is to write the gloss while the
target is open rather than to remember later.

### 2. One inbox, which already exists

A finding that is not a `BUGS` entry beside the feature becomes a file in
`design/roadmap/proposals/`, immediately, even at three lines. No new machinery is needed:
`script/roadmap --proposed` lists them, `--unclaimed` finds follow-on work that became one, and
`script/metrics` has counted them weekly since 2026-09-19.

The pile is the point rather than an embarrassment. `notes/project-metrics.md` already says a rising
proposal count is closer to debt than a rising provisional-name count is, and that honesty is what
makes the inbox usable: nobody is tempted to keep the number down by not writing things in it.

### 3. A cadence on disposition, which is the only real gate available

The tree cannot check that a finding was entered. It can check that what was entered does not rot.
Two reports, on the cadence the audit and stranger workflows already run:

- **Proposals past an age with no disposition.** The data is in the file's own
  `**Status: PROPOSED <date>**` line, which `scripts/roadmap_proposals.py` already parses.
- **Refusals whose condition has come true.** Built on 2026-09-20 as `script/roadmap --revisit`, and
  honest about its reach: only 2 of 42 conditions name a milestone at all, and it prints that
  denominator on every run.

## What this is not

**Not a new record type.** Everything above routes into `BUGS` sections and proposals, both of which
exist and both of which have gates. A fourth place to put things would be the defect, not the fix.

**Not a promise to build what is captured.** DECISIONS §71 (a limitation is promoted when it stops
being a fact and becomes a plan) is deliberate and unchanged: a `BUGS` entry is a fact, a roadmap row
is intent, and capture is not commitment. The FreeBSD posture that carries a limitation for years,
honestly, is the one this tree copied on purpose.

**Not a gate on prose.** See above; it was measured and refused.

## What calef is being asked to rule on

Whether part 1 becomes a clause in `AGENTS.md`, beside the lane rule it extends, and in what words.
Parts 2 and 3 need no ruling: the inbox exists, and the first half of the cadence is a few lines
against a parser that is already written.
