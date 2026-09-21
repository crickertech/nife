# 521. Does an AI review of a pull request catch anything the gates and the maintainer do not

**Status: NOT-STARTED.** Minted 2026-09-21 by calef, who asked for an experiment rather than an
opinion. *(Number provisional until the merge queue lands it.)*

**Gate: NONE.** The corpus, the harness precedent and the defects all exist in this tree.

## The question, stated so it can come back no

**Does an AI review of a pull request's diff catch defects that this tree's gates and its maintainer
do not, at a false-positive rate worth paying?**

Both halves are load-bearing. A reviewer that catches real defects and also produces three plausible
findings per clean pull request costs more attention than it saves, and this tree has refused gates
on exactly that ground before: `git grep -w TODO` measured **82% false positives**, and a proposed
gate on the lane line measured **two false positives in four hundred pull requests with zero true
ones**.

## Why the obvious experiment is the wrong one

**Turning review on and reading what it says produces plausible findings with no way to separate the
true ones from the confident wrong ones.** That is the failure mode this tree keeps meeting: a lane
reported a `script/names` blind spot that had been closed for a month; a maintainer measured an hour
of benchmarks against a tree they had clobbered; a census misattributed 77 survivors to a pull
request that introduced four. **Every one of those was plausible.** An experiment whose output is
prose that somebody has to believe is not an experiment.

## The design: retrospective, with ground truth, pre-registered

### Arm 1: can it catch what actually got through?

Take defects whose **introducing commit is identifiable**, reconstruct the diff as it was **without
the fix**, and run the review blind. Score one question: did it name the defect?

`notes/proof-retrospective.md`'s corpus is eighteen entries, each with what found it, and this week
alone added more with known introduction points:

- `drain_sink` ending only on a marker a dying program never sends, so a panic message was delivered
  and discarded.
- All three link scripts discarding `*(.eh_frame*)`, throwing away the compiler's unwind information.
- Milestone 447 (a thread's vector registers are its own)'s stack-frame regression, where a 544-byte
  field cost 1,120 bytes of frame.
- `arch::init` not running on every core on RISC-V, which was green on two architectures and silently
  wrong on the third.
- `compositor`'s `Rect::area`, caught in August and surviving in September.

**The reviewer must not be able to see the future.** Reconstruct at the parent commit, strip anything
that references the fix, and say in the record how that was enforced.

### Arm 2: what does it cost in noise?

Run the same review on pull requests with **no subsequent fix commit touching them**, and adjudicate
every finding as true, false, or unfalsifiable. **This is the arm that decides the milestone**, and
it is the one that will be tempting to skip because it is tedious and its result is likely to be
unflattering.

### The threshold, registered before the runs

**Write it down before running anything**, because a threshold chosen afterwards is a rationalisation
with a number in it. A starting proposal, to be argued with before the first run rather than after:

- **Adopt** if it names a third or more of arm 1's defects at under one false positive per clean
  pull request.
- **Refuse** if it names under a fifth, or exceeds two false positives per clean pull request.
- **Anything between is a null result**, which is a legitimate outcome and must be reported as one.

## The variable worth building in

**The reviewer is the same model family that wrote the code**, so its blind spots may be the lane's
blind spots, and a clean result might mean agreement rather than correctness. Run a subset with an
**adversarial posture** ("find the defect this diff introduces") against the same diffs reviewed
neutrally ("review this diff"), and report whether detection moves. If it does not, that is a fact
about the ceiling rather than about the prompt.

## The honest prior, recorded in advance so the result can contradict it

**The defects this tree actually found were found by running things**: a QEMU boot, a mutation sweep,
a GDB backtrace, a bench on three harts. `design/fatal-risks.md`'s risk 2 already records the
strongest version of this, that no Kani harness has caught a defect after the day it was written.

So the maintainer's prior is that review's plausible niche is the class the gates cannot see: a
comment that is wrong, a record that has gone stale, a gloss that resolves to the wrong thing. **That
is also precisely the class where a plausible-but-wrong reviewer does the most damage**, because
nothing downstream checks it. The experiment exists to find out whether that prior is right.

## Prior art in this tree

- **`notes/stranger-test.md`** already runs an agent against this tree as an experiment, with a
  protocol, a journal and costs recorded per run. It is the shape to copy, including that it records
  what the harness itself cost.
- **Milestone 191 (did the proofs catch the bugs?)** is the retrospective-against-a-corpus shape, and
  it returned an uncomfortable answer that the tree kept.

## BUGS

- **The corpus is small and not evenly distributed.** `notes/proof-retrospective.md` says so of its
  own eighteen: five are concurrency. A result on this corpus is a result about this project's defect
  history, not about code review in general.
- **Adjudicating arm 2 is a judgement call**, made by the same maintainer whose attention the
  reviewer would be spending. State who adjudicated and record the disputed cases rather than only
  the totals.

## Index row

An experiment rather than an opinion: whether an AI review catches defects the gates and the
maintainer miss, scored against defects whose introducing commits are known, with the false-positive
arm that decides it and a threshold registered before the first run.
