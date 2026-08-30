# 134. A harness carries a machine-replayable falsification record, or it is not evidence

**Status: PROPOSED.** Raised 2026-08-30 by calef, from milestone 191's (did the proofs catch the
bugs?) finding. The direction is his (*"It sounds like [option C] is where we want to land if we want
to state that nife is proven"*); what remains open is the record's **format**, which is the only
irreversible part and is his to settle. *(Section number provisional until the merge queue lands
it.)*

## What is being decided

Whether every Kani harness in this tree must carry evidence that it can fail, in a form a machine
can replay, and what that form is.

## Why now

`notes/verification.md` already states the rule, and states it well:

> **Falsify a property before believing it.** Break the code the harness guards and confirm the harness
> fails. Every milestone 35 property was falsified this way, and one falsification corrected a claim in
> the code (the load-bearing guard was not the one the comment pointed at). A harness that cannot be
> made to fail is not evidence.
>
> -- notes/verification.md

**It is rung four**, honoured by whoever remembers, and milestone 191 measured what that produces:
145 harnesses, **23 `kani::cover!` sites across 4 of 24 harness crates**, and **no harness anywhere
recording what was done to falsify it**. The reverse pass found the predictable result:
`capability::subset_is_reflexive` proves `a & !a == 0`, a tautology no plausible implementation error
breaks, and twelve of the 26 `paging` harnesses restate six properties once per ISA.

The forcing question is calef's, and it is about a claim rather than about code: if the project is
going to say **proven**, the evidence that the proofs are load-bearing cannot itself be a written
claim that they are load-bearing. That is one level of indirection away from evidence, and it is the
same shape as the nine misrecorded roadmap statuses and the fabricated block quote that survived
twelve days of gates.

## Two failures, and they are not the same problem

Conflating them produces a convention that half works.

**Vacuity is mechanical.** Assumptions contradict, the input set is empty, and Kani reports
`SUCCESSFUL`. `kani::cover!` is the check that catches it, which `notes/verification.md` already
says.

**Tautology is not.** A property true of every plausible implementation passes every cover check
ever written. Only falsification catches it: if nobody can describe a change that turns the harness
red, the harness is decorative.

## Prior art, and it is a field rather than an idea

Read rather than recalled, because this tree carried a fabricated block quote for twelve days.

The model-checking literature has done this since roughly 2004, under two names that belong
together. **Vacuity detection** and **coverage metrics** are the standard sanity checks, and the
survey work makes the unifying observation explicit: both work by *repeating the verification on a
mutated input*, with vacuity mutating the **specification** and coverage mutating the **system**.

What this section proposes has a name there: **mutation coverage for model checking**, where an
element of the system is considered covered by the specification if changing that element falsifies
the specification. That is "break the code, confirm the harness fails", stated formally, with two
decades behind it.

**§39's protected class applies**: a term a reader already knows from outside costs a newcomer
nothing, so this convention should be spelled in the field's vocabulary rather than in ours.

There is also a cheaper family, **Inductive Validity Cores**, which compute the minimal set of model
elements a proof actually needed and so give coverage without re-running anything. **Whether Kani or
CBMC can produce one is unverified and is the first thing the implementing milestone should check**,
because a yes makes most of this section cheaper or unnecessary.

Sources: [Coverage Metrics for Formal
Verification](https://link.springer.com/chapter/10.1007/978-3-540-39724-3_11), [Sanity Checks in
Formal Verification](https://www.cs.huji.ac.il/~ornak/publications/concur06b.pdf), [Mutation Coverage
Estimation for Model Checking](https://link.springer.com/chapter/10.1007/978-3-540-30476-0_29).

## The options

| | what | catches | cost | prior art |
|---|---|---|---|---|
| **A** | a lint requiring `kani::cover!` in every harness | vacuity only | cheap, per pull request | vacuity detection, standard practice |
| **B** | a **prose** falsification record, presence lint-enforced | tautology, by human review only | cheap, and it is a claim nobody re-runs | none |
| **C** | a **machine-replayable** falsification record, replayed on a schedule, each harness required to go red | both, mechanically | about one `script/verify` run per sweep | mutation coverage for model checking |
| **D** | Inductive Validity Cores | coverage, with no re-run | unknown, may not exist in Kani | IVC literature |

## The recommendation: A and C, with B refused on the record

**A**, because vacuity is mechanical and a cheap gate catches it. It is also already the tree's
stated practice, so the lint is making an existing rule enforceable rather than adding one.

**C**, because it is the only option that lets the word carry weight. **The price is smaller than it
looks**, and the arithmetic is the reason this is recommended rather than admired: a falsification
runs **one** harness, not all of them. `script/verify` costs about 42 minutes for 140 harnesses, so a
full sweep of one recorded falsification per harness is the same order of magnitude as a single
verify run, not 145 times it.

**Its posture is `script/mutation`'s, deliberately copied**: a scheduled report with a baseline, not
a per-commit gate, because the run costs too much to block a commit on and a regression is a worklist
entry rather than a defect in whatever landed that day. A **survivor** here is a harness whose own
recorded falsification failed to make it fail, which is exactly the signal nothing in this tree can
currently detect.

**B is refused rather than skipped**, because it is the tempting middle and it is the precise failure
this section exists to end: a written claim standing in for evidence. It would also pass its own lint
forever while meaning nothing.

**Machine-generated mutation is refused too, and the reason is cost rather than preference.**
`cargo-mutants` produced **5,551** mutants over the host crates. At test-run cost that is hours; at
proof-run cost it is not a job. So the affordable form is one recorded falsification per harness,
written by whoever writes the harness.

## What this does not claim, which is the half that matters

**Parity with seL4 on the proof is not available and should never be implied.** seL4 proves
functional correctness by refinement to an abstract specification, plus integrity and information
flow. That is a different category and a decade of work. nife does bounded model checking of selected
properties over selected crates, and after milestone 193 (put `kernel/src` within reach of the
prover) it will do that over some of the kernel too.

**Parity on the assumption discipline is available now, and it is most of what makes seL4 trusted.**
The seL4 Foundation publishes *What the Proofs Assume* as a first-class page with eight numbered
assumptions: assembly code (about 340 lines, assumed correct), hardware functionality, cache and TLB
management, boot code (about 1,200 lines, outside the proof), virtual memory (where they say plainly
that the proof is not from first principles and there is potential for human error), DMA devices,
information side-channels, and the compiler except where binary verification covers it. They also
keep test suites specifically for the code left as an assumption.

And they name this section's exact problem as their hardest assumption: **one can never be sure that
a formal specification means what we think it should mean.**

So the claim this section buys is not "nife is proven". It is the sentence this tree already writes
about benchmarks, applied to proofs: **these specific properties are proved, each is falsifiable and
the falsification is replayed on a schedule, and here is what is not covered.** That sentence is
honest at any size. It gets larger from milestone 193, not from this decision.

## What calef must settle, because it is the irreversible part

**The record's format.** Everything else here is reversible; the format is not, because it lands in
145 places and in the head of everyone who writes the 146th.

The requirement that decides it: **a script must be able to apply the mutation without a human
reading it.** A prose record ("I broke the bounds check") makes option C impossible without redoing
all 145. A precise, machine-applicable spec makes C a script that replays them and can sample a
rotating subset per cycle.

Open, and his:

1. **The carrier.** A doc-comment section, a custom attribute, or a sibling file per harness.
2. **The mutation's spelling.** A unified diff, or a structured trio of (file, symbol, edit), or a
   `cargo-mutants`-style operator name. A diff replays trivially and rots against refactors; an
   operator name survives refactors and constrains what can be expressed.
3. **The names.** The attribute or keyword, and the lint check's name.
4. **Whether an exemption exists**, and how it is spelled. Some harnesses may be honestly
   unfalsifiable; §75's posture says an exception must say it is one.

## BUGS

- **This adds friction to writing a harness**, at the moment harness-writing is about to increase
  sharply if milestone 193 lands. That is a real cost and this section does not pretend the
  convention is free.
- **A recorded falsification proves the harness catches *that* defect**, not that it catches the
  class. It is a floor, and a low one.
- **Nothing here addresses the 145 existing harnesses**, which would all need records written
  retroactively, by someone who did not write them. The implementing milestone owns that sweep and it
  is the largest piece of work in this section.
- **The `kani::cover!` lint can be satisfied vacuously too**, by covering something trivially
  reachable. A gate that counts `cover!` sites is weaker than a human asking what the cover is for.
- **`kernel/src/arch/` stays out of reach under every option here**, so the architecture layer, where
  the VisionFive 2's undelivered-wake defect actually lived, gains nothing from this section.
