# Stranger test run 3, 2026-08-18

*An appendix to [the stranger test](../stranger-test.md), which holds the current state. This page is the record for run 3: the design decided before the run, and what it found.*

## What run 3 changes, decided 2026-08-18 before the run

Run 2's second `BUGS` entry names the one thing run 3 must fix, and it is not the rubric leak:
**the stranger must be a process that cannot see this repository except through the tree it is
handed.** Run 2's strangers were subagents of a maintainer session whose working directory was the
repository, so `AGENTS.md` arrived in their context before they chose to read anything, and five of
the eight rubric rows went unscored as a result. Everything below is the design that closes that
hole, written down before the run so it cannot be graded generously afterwards.

**The stranger is a separate process, not a subagent.** A fresh `claude` CLI process, started from
a shell, with no conversation history and no brief beyond the task. A subagent inherits its parent
session's project instructions; a separate process does not have a parent.

**Its working directory is the clone's parent, not the clone.** This is the whole mechanism and it
is one line of shell. Project instructions are loaded from the working directory and its
*ancestors*, never its descendants, so a process started in `.../run3/` with the repository at
`.../run3/nife/` is handed no `AGENTS.md` at turn zero. The file is still in its tree, exactly as a
stranger who cloned from GitHub would find it, and the stranger can open it the moment it decides
to. **That is the difference the milestone is about**: whether the tree tells a newcomer to read
the constitution, rather than whether the constitution is good once read. Run 2 could not ask that
question because the answer was already in front of its stranger.

**The answer key is withheld the same way run 2 withheld it**, because that rule worked as far as
it claimed: the clone has `notes/stranger-test.md` and its `notes/README.md` entry removed, amended
into the tip so the working tree is clean and no `git status` line advertises the deletion. Nothing
else is touched, and the in-tree mentions of runs 1 and 2 stay, because a tree with those cut is not
the tree under test.

**The journal stays**, with its cost restated rather than rediscovered: a stranger told its
confusion is the deliverable is watching itself. It writes an append-only log outside the clone,
before and after each step, because run 2's first attempt died and took its findings with it.

**What this design still cannot give, stated now rather than after the result.** The machine is
warm, and warmer than run 2's: this is the architect's own laptop, with the pinned nightly, both
QEMUs, and a populated cargo registry cache already present. **B2 and B4 are therefore not
measured on the install path at all**, only on whether the documented sequence runs and says true
things. A cold build half wants a container and is a separate run; claiming this one is it would be
the generous grading the rubric exists to prevent. The stranger also inherits the operator's
*global* user preferences file, which mentions no operating system and no project in this tree, and
the per-project memory directory is keyed to the repository's own path so a clone under `/tmp` does
not load it.

**And the harness's author is not a stranger, which is the residual and it is not small.** This
lane's developer has read `AGENTS.md` in full. It wrote the task, chose the isolation, and reads the
result, so its judgement about what counts as a defect is contaminated even though the stranger's
answers are not. The mitigations are that the task text is run 1's and run 2's verbatim, the rubric
predates all three runs, and the stranger's questions are recorded as it asked them rather than
summarised into findings.

## Run 3, 2026-08-18: a process that could not see the repository, and a harness that told it anyway

**The isolation worked, and that is the deliverable.** Run 2's unresolved `BUGS` entry demanded a
process that cannot see this repository except through the tree it is handed. Run 3 was a separate
`claude` process rather than a subagent, started with `--safe-mode` (which disables project
instruction discovery, skills, plugins, hooks and MCP) in a directory whose *child* was the clone,
so no `AGENTS.md` could be loaded from an ancestor. **It was verified rather than assumed**, twice:
a throwaway probe in the same configuration answered `NONE` when asked what project-instructions
files were in its context, and the stranger itself, asked afterwards, answered *"No. Before my first
tool call my context was the system prompt and your task message... I had not read a byte of the
project's prose."* Five rubric rows that run 2 had to report unscored are scored here.

**The clone was `9495d6ba` with the answer key withheld the same way run 2 withheld it**: this note
and its `notes/README.md` entry removed and the deletion amended into the tip, so the working tree
was clean, nothing else touched, and `origin` pointed at the public GitHub URL. The stranger never
fetched, never used the network at all, and never opened another checkout on the machine.

**Task, verbatim from runs 1 and 2**, plus the journal: *"Get the project building and its tests
passing, then write up what this system is and how you would add a new user program to it."* 132
turns, about 41 minutes, `$12.08`.

### The build half, measured properly for the first time

**`script/test` is green and intermittently red on a contended host, and the difference is host load
average.** The stranger's first `cargo xtask test` failed 22 tests into the aarch64 leg on
`the_handler_keeps_up_when_no_lock_is_held`, with six cascading host-side probe failures behind it
that it correctly judged downstream and did not chase. It then did what neither previous run did: it
built a rate rather than an anecdote. **2 red in 13 aarch64 legs**, plus two reds of the sibling
`ticks_arrive_at_the_configured_rate`, whose eight-retry loop also exhausts under sustained
contention. It closed the diagnosis with `script/icount`, which passed on both ISAs at load 46.77
with 1,056 and 800 instructions against a 2,500 bound.

**The load was 45 to 63 on eight cores, caused by other lanes gating in other worktrees, and
nothing told it.** It assumed the machine was its own for five journal entries, built a theory on
that premise, and caught itself only by running `uptime` an hour in. Its own summary is the finding:
*"Check the environment before theorising about it... One `uptime` at the moment of the first failure
would have replaced twenty minutes of inference with a fact."* The tree had already measured this
exact condition, to the digit, in notes/load-sensitive-assertions.md, four hundred lines below the
section the assertion's comment points at.

`script/setup` had nothing to do: the pinned nightly and the pinned QEMU were already installed, so
**B2 is not measured and B4 is measured only against the documented sequence**, exactly as
pre-registered. `script/lint`, `script/swish-check` on both ISAs and `cargo xtask build` were green
on arrival. **Nothing in the tree was changed to reach green**, and the stranger says so plainly:
*"the work turned out to be establishing that they do... The temptation to make a change so there is
a change to show is exactly the trap the assertion's own message sets."*

### The mental model, scored

| # | result | where it came from |
|---|---|---|
| M1 | **partly**, and it corrected the question | the concept from `crates/abi` and `swish`'s banner; it flagged the rubric's phrase as imported and never opened notes/capabilities.md |
| M2 | **mostly absent** | it never reached notes/net.md; what it had was `user/Cargo.toml`'s per-program prose, and it said so rather than filling the gap |
| M3 | **answered** | `AGENTS.md` rule 1, reached through the README's pointer |
| M4 | **answered** | `design/roadmap/README.md`, including the rule that the block wins over the column |
| M5 | **answered** | `AGENTS.md` rule 7, with the reason the file says matters most |
| M6 | **induced, not read** | no stated definition found; assembled from four instances and mapped to the ladder's rung three by inference |
| M7 | **answered by doing it** | added `triangle`, got it answering at a real prompt on both ISAs, ran the negative control, reverted |
| M8 | **answered, and it corrected the rubric** | four states, not three; see the amendments above |

**The strongest single result is M7 and it is the same shape as run 2's.** The stranger walked
notes/adding-a-program.md and found a fifth wrong place: milestone 130 deleted the `mkinitrd()`
shape step 4 describes, and `manifest()` is a seventh `grant_plan` edit the list of six omits. The
page was corrected by run 2 on 2026-08-16 and was wrong again by 2026-08-18. It is recorded in that
page's own `BUGS` rather than fixed here.

**The weakest results are M2 and M6, and they share a cause.** The tree answers both, well, in
`notes/net.md` and in `AGENTS.md`'s comments section, and the stranger reached neither. Its own
closing sentence is the diagnosis and it is worth quoting because it is what the milestone is
about: *"this tree is unusually good at writing down what it has learned, and unusually bad at
putting the writing where the person who needs it will trip over it."* Three instances, its own:
the timer taxonomy documented in a note and contradicted by the panic message a failing developer
actually reads; `notes/adding-a-program.md` stale again on arrival; `CLAUDE.md` holding the rules
behind a name the README says out loud makes people walk past it, and then does nothing about.

### What it read, and in what order, which is B1

Fifteen documentation files. `README.md` first, by expectation rather than by any pointer.
`AGENTS.md` **twelfth**, reached from the README's pointer and read as four targeted sections rather
than whole, on the explicit reasoning that 66 KB is expensive. `notes/adding-a-program.md` ninth,
found by grep *after* reconstructing most of it from `xtask` and `grant_plan`, because it grepped
the identifier before the concept. `crates/abi/src/lib.rs` late, which it calls its own biggest
mistake: *"Four syscall numbers. It is the whole design on one screen, and I read it far too late."*
**It opened no file under `design/decisions/` at all**, having read the signpost and stopped there.

**So B1 fails, and it fails in an interesting direction.** There was no reading order, the stranger
built a good one by instinct, and the two places that instinct cost it most were a file the README
does not mention (`crates/abi`) and a pointer that sits at line 226 of 285.

### What this run cost, and the largest cost was the harness's own

**The stranger knew it was stranger 3 from its first tool call, because the harness's log files were
in its working directory.** `ls -la` returned `stranger3-stream.jsonl` and `stranger3-stderr.log`
beside the clone. Asked afterwards, it named that first and was specific about the consequence:
*"It is the direct reason I added a program instead of only describing one"*, and *"both deliverables
are long and rhetorically polished, and some of that is performance for a reader I knew was
watching."*

**That is the same class of defect as run 2's, one level out, and it was introduced by the person
building the isolation while building it.** Run 2 leaked the constitution; run 3 leaked the fact of
being measured, louder than the in-tree references leak it, and before the stranger had read
anything. The in-tree leak fired too, exactly as the `BUGS` entry predicts it always will: the
README cites runs 1 and 2 by name and `notes/adding-a-program.md` is saturated with them. The
stranger met both and still never opened `design/roadmap/117-newcomer-onboarding.md`, so the answer
key held. **The fix for run 4 is one line: the log files go in a sibling directory, not the parent.**

**Three smaller costs, all pre-registered except the second.** The machine was warm, so B2 measures
nothing. The machine was also *loaded*, by other lanes, which contaminated every timing result and
produced the run's best finding, so it is a cost and a dividend at once. And the harness's author
had read `AGENTS.md`, which no arrangement of processes fixes: the task text was runs 1 and 2's
verbatim and the rubric predates all three, but the judgement about what counts as a defect is
still a contaminated judgement.

### What a stranger still cannot do, after three runs

- **Reach `notes/net.md`, `notes/capabilities.md` or any `design/decisions/` file** by following the
  tree from the front page while doing ordinary work. All three were unopened; two of them carry
  rubric answers.
- **Learn what a `BUGS` section is for from a sentence.** It is the tree's most distinctive
  convention and a stranger has to induce it from instances.
- **Know that the machine is shared with other lanes**, which is the single most load-bearing fact
  about any timing result and is stated nowhere a person running `script/test` will meet it.
- **Find `crates/abi/src/lib.rs`**, four syscall numbers and the whole design on one screen, without
  luck.
