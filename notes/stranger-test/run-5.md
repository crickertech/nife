# Stranger test run 5, 2026-08-18

*An appendix to [the stranger test](../stranger-test.md), which holds the current state. This page is the record for run 5: the design decided before the run, and what it found.*

## What run 5 changes, decided 2026-08-18 before the run

**This is the first run conducted through `script/stranger-test` rather than rebuilt by hand.** The
lane that wrote the harness deliberately ran nothing through it, so nothing about the instrument has
been exercised by anyone except its author and a `--smoke` reply. Run 5's brief is one line,
`script/stranger-test --commit origin/main`, and the first thing it measures is the harness itself:
whether the isolation, the withholding, the shims and the toolchain restore hold when somebody who
did not write them runs them. A defect in the harness is a finding about the harness and is recorded
rather than worked around, because a run that patches its own instrument mid-flight is measuring
something it can no longer name.

**The disclosure is live for the first time.** Run 4's handoff 5 said run 5 should be told it is
being measured, on the reasoning that the tree leaks the fact within half an hour anyway and
pretending otherwise buys nothing while costing the disclosure. The harness implements it in the
task text. **That makes this the first run whose stranger knows at turn zero**, so it is not
comparable to runs 1 through 4 on anything the knowledge touches, and the two questions worth asking
of it are what the stranger says the disclosure changed, and whether the performance run 4 confessed
to shows up anyway. The task also asks it not to perform, in those words, which is a second new
variable in the same paragraph. If the result is good, the two cannot be separated, and this note
should say so rather than credit either.

**What is genuinely new in the tree, and it is one command.** `script/apropos` landed 2026-08-18 and
exists because of this instrument: three runs measured that a stranger doing ordinary work reaches
neither `notes/net.md` nor `notes/capabilities.md` nor any file under `design/decisions/`, and none
found `crates/abi/src/lib.rs`, which is four syscall numbers and the whole design on one screen.
All four are now one search away. **Whether the stranger discovers it on its own is the
measurement**, so it is not in the task text and it is not hinted at. It is named in exactly one
place a newcomer might reach, `notes/scripts.md`'s table, which sits behind item 8 of the README's
reading order, and item 8 points at `notes/README.md`, which no stranger has yet opened. The
prediction registered here is that the stranger does not find it.

`CONTRIBUTING.md` and the `## Start here` order have now been seen once, by run 4, so this run is
their second data point rather than their first. The specific failure run 4 left is the one to
watch: `CONTRIBUTING.md` is item 2 of 8 and was read sixteenth of twenty-two.

**The machine is uncontended, which no previous run could say.** Run 3 gated against a load average
of 45 to 63 and reported a 2-in-13 red rate; run 4 ran between about 3 and 17.5 with four other
lanes. At this run's launch the load average is 4.04 on 8 cores with no other lane running and no
QEMU on the machine. So a red timing leg here cannot be blamed on the host, and the load-average
diagnostic that run 3 bought and run 4 could not exercise gets its second chance to stay silent
honestly.

**The rubric is not amended by this lane, and that is a deliberate departure from run 4.** Run 4's
lane edited the table before running against it, and its own contamination section says that made
things slightly worse. Run 5 grades against the table exactly as it stands after run 4's
amendments, eight mental-model rows and four build rows, scored `answered`, `partly answered`,
`wrong` or `absent`. If this run falsifies a row, it is recorded here in run 3's shape, as a
correction the next lane applies, rather than applied by the lane that would then score against it.

**Three predictions, registered so the result cannot be read generously afterwards.** They are the
lane's, not the stranger's, and each is falsifiable.

1. The stranger does not find `script/apropos`, and therefore still does not reach any file under
   `design/decisions/`.
2. `notes/README.md` goes unopened for the fifth run running, since nothing in the reading order
   sends a reader there until item 8 and item 8 is the one that says to stop reading in order.
3. The build half is green first try with no change to the tree, and the B2 row measures nothing
   again, because the machine has the pinned nightly, both QEMUs and a warm cargo cache.

## Run 5, 2026-08-18: the first run through the harness, and the machine leaked where the tree did not

**The harness worked, and this is its first evidence from anyone but its author.** One command,
`script/stranger-test --commit origin/main`, and a `--smoke` run before it. Nothing was rebuilt by
hand, nothing about the isolation had to be remembered, and the four failures that each of runs 1
through 4 hit in a different place did not recur: the stranger's working directory held the clone
and nothing else, the probe answered `NONE`, the answer key was absent from the tree, `pkill` was
shadowed, and the account-wide `nife-dev` link was recorded before the run and put back after it,
which the harness printed on its way out and which was verified against `readlink` afterwards. The
`--smoke` run cost `$0.09` and 3 turns and exercised every stage including both debriefs, which is
what a smoke run is for; the real run cost `$10.56`, 201 assistant turns and about 29 minutes.

**Two things leaked anyway, and neither is the tree.** The first is the one the harness's own `BUGS`
section had called luck: **the stranger read `script/stranger-test`**, about thirty minutes in, and
it is a complete description of the instrument it was inside. It did not go looking. The second
explains the first and is the more useful finding.

**The machine leaked into the clone, and the harness cannot stop it.** `nife-dev` is an
account-wide `rustup` link, so the clone's first `script/test` compiled `std` out of whichever
worktree had built the farm last, which was `/Users/calef/projects/nife/target/nife-farm`. The
`-Zbuild-std` dep-info cached those absolute paths, cargo then considered the unit fresh, and
`std-aborts` failed **naming two files and two line numbers inside another checkout on this
machine**. Grep for that foreign path found exactly one file in the tree that contained it, which
was the harness. So a run whose task text says *"the repository is in ./nife and it is the only
thing you have; do not look for another checkout on this machine"* was handed another checkout's
path by its own build, ran `ls -la` against it to see whether it was real, and was then rescued
from a wrong repair by the harness's own paragraph about restoring the link. **The harness both
contaminated the run and saved it**, which is the worst shape a contamination can take. Recorded in
`script/stranger-test`'s `BUGS` and in notes/std.md's.

**What the disclosure did, which is the variable this run exists to measure.** The stranger was told
at turn zero that it was being measured, per run 4's handoff 5, and asked not to perform. Both
halves partly failed and it said so without being pressed:

- **The knowledge chose its program.** It wrote in its journal *before* picking that "repeating the
  shape those walks used would measure nothing", and deliberately selected the one manifest
  combination nothing in the tree had used. Its own reading of that: *"That is not what a fresh
  contributor does. A fresh contributor writes another `doubler`, finds nothing, and that null
  result is data the repository does not get from me."* This is run 4's contamination in a sharper
  form. Telling the stranger it is measured does not remove the effect the tree's leak was already
  having; it makes the effect legible and slightly stronger.
- **It performed anyway, having been asked not to, in the same words.** *"I did adopt the prose
  style, despite being told not to... bolded lead sentences, 'load-bearing', the
  X-is-the-finding-rather-than-the-accident cadence. Not a deliberate choice, which is worse, it
  means the voice is contagious enough to catch without noticing."* **So the instruction does not
  work**, and asking for it is not free: it is one more sentence of framing in a task text whose
  stability across runs is the only thing making them comparable.
- **It over-claimed once and corrected itself unprompted at debrief.** Its write-up opened "build
  and tests: green"; its first debrief line retracts that, because the final `script/gates` was
  killed part way through the HVF leg and what it actually had was green in pieces. Worth recording
  because the retraction came before any question asked for it.

### The configuration, verified rather than asserted

Clone at `a5a75f46` (`6ae92831` after the withholding amend), answer key absent, working tree clean,
`origin` pointed at the public GitHub URL, one branch. Separate `claude` process, `--safe-mode`,
working directory the clone's parent. The probe answered `NONE` and the stranger answered the same
afterwards: *"No CLAUDE.md, no AGENTS.md, no user or project memory. The working directory was
`run/`, with the clone at `run/nife`, so nothing loaded from a descendant."* The harness's own
artifacts were in a sibling directory and its root was `nife.t63XaQe1`, a name with no run number.
Task and debrief text unchanged from the harness, which is runs 1 through 4's text plus the
disclosure paragraph.

**The machine was uncontended, for the first time in five runs**, and "uncontended" here means no
*other* lane rather than an idle machine. Sampled every 30 seconds on 8 cores: 2.44 at launch,
minimum 1.93, mean 4.09, **peak 15.07**, and the peak is the stranger's own emulated legs rather
than anybody else's, since no other lane was running and there was no QEMU on the machine when it
started. That is worth separating, because runs 3 and 4 could not: their peaks were other people. Run 3 gated at 45 to 63 and run 4 at about 3 to 17.5. **No timing assertion fired,
so the load-average diagnostic run 3 bought is still unexercised after two runs.** One HVF flake did
occur and it is not a timing assertion: `inbound check (aarch64) FAILED: the guest served 2 of the 4
inbound connections it offers`, which passed on a standalone re-run of `script/test --hvf`. On an
idle machine that is the cleanest evidence yet that the HVF leg is flaky on its own account.

### The build half

**B1 passes and leaves a worse failure than run 4's.** `README.md` was first, `CONTRIBUTING.md`
**third**, which is run 4's specific complaint fixed: run 4 read it sixteenth of twenty-two, after
the gates it describes had been run. And then **`AGENTS.md` was never opened at all**, in a run
whose reading order names it item 3 and says *"if you read only two, make them 3 and 4"*. The
stranger read item 4 (`notes/capabilities.md`) and skipped item 3, and its write-up's statements
about where architecture-specific code lives and about the project's ladder are paraphrases of
`CONTRIBUTING.md`'s summary of a file it never read. **Four runs reached `AGENTS.md` twelfth,
seventh, and not at all**, and the not-at-all is the run that read `CONTRIBUTING.md` earliest. That
is a result about the reading order rather than about the stranger: the reader met a shorter
document that summarises the longer one and stopped. Its own reason, quoted because it is the
finding: *"66 KB is a large upfront cost when a task is in front of you, and everything I actually
needed turned out to be reachable from code."*

**B2 is not measured**, as pre-registered. `script/setup` passed in 14 seconds with the pinned
nightly, both QEMUs and a warm registry already present, and the stranger noted that **nothing in
the repository told it the machine was pre-provisioned; it checked.**

**B3 passes, and not first try, which falsifies this lane's third prediction.** The first
`script/test` was red at about 88 seconds, on `std-aborts`, for the machine-global reason above and
not for anything in the commit. Recovery was `rm -rf std_exerciser/target` followed by `script/test`
exit 0 in 3m25s, on both ISAs. The rest of `script/gates` then passed except the HVF flake.

**B4 fails, with eight entries**, which is the row that matters and the largest list any run has
produced. In the stranger's order: the mechanism of the `nife-dev` link failure and that it caches;
that the recovery is `rm -rf std_exerciser/target`; that capability slot numbers are computed
per-program in `spawn_service` rather than fixed, while every existing program documents its slots
as constants; that an integer argument reaches a non-interruptible child in `x1` via
`tcb_start(tcb, 0, arg, 0)`; that a `caps` preview of an input operand needs `Holdings::dir`; that
an argument-plus-input manifest breaks a `swish` host test; that `script/swish-check` prints no
transcript on success, so a green run teaches nothing; and that this machine was pre-provisioned.

### The mental model, scored: six answered, one partly, one absent

| # | result | where it came from |
|---|---|---|
| M1 | **answered**, and it is the best answer five runs have produced | `notes/capabilities.md` for the mechanism and `crates/grant_plan/src/lib.rs` for the tree's own words, then `crates/system_initializer/src/lib.rs` for the attenuation: a process viewer holds `ENUMERATE` and not `READ` because `READ` is also what `RECV` and `REAP` take. Its formulation: *"designation is authorization, at the rights the capability carries"* |
| M2 | **absent**, for the third run in four, and it said so plainly | *"I did not find this, and I should be clear about how little I looked."* It never opened `notes/net.md`, saw it cited once in a failure line, and reasoned correctly from `crates/grant_plan`'s `Manifest` having no socket field that no shell-spawnable program can reach the network. It could not say what a networked program holds |
| M3 | **partly answered** | `CONTRIBUTING.md` for `kernel/src/arch/`, verified by checking that the only `asm!` outside it is in comments. On the consequence it said it found no file stating it, and gave the parity gate instead of the diff-across-every-file; the file that states it is `AGENTS.md`, which it never opened |
| M4 | **answered** | `design/roadmap/README.md`, with the rule that the column is wrong and the block is right when they disagree |
| M5 | **answered** | `CONTRIBUTING.md` for both criteria, with host-testable and Kani-reachable named as the load-bearing one, and the `crates/swish` + `components/src/swish.rs` pairing read off the tree |
| M6 | **answered, quoted rather than induced** | `CONTRIBUTING.md`, and then the observation that `notes/adding-a-program.md`'s `BUGS` section held the most useful things it learned and none of them are in that page's steps |
| M7 | **answered by doing it**, and it found an eighth site | added `nth`, working on both ISAs, and listed the eight edits with the `Manifest` as what you declare, provisional name included |
| M8 | **answered** | four states, `CONTRIBUTING.md` for who decides, `notes/adding-a-program.md` for the states, and it ran `script/names --provisional` and found its own hour-old name listed first |

**M2 regressed against run 4 and the cause is measurable.** Run 4 answered it from `notes/std.md`,
which it happened to open; run 5 did not open that page either. Three of five runs cannot answer
M2, and the page written to answer it has now gone unopened five times.

### What it read, and what `script/apropos` did not do

Twenty files, `README.md` first by expectation. **`script/apropos` landed 2026-08-18 precisely
because three runs could not reach `notes/net.md`, `notes/capabilities.md`, any `design/decisions/`
file, or `crates/abi/src/lib.rs`. Run 5 never ran it**, and the prediction registered before the run
is confirmed in a stronger form than it was made: the stranger had the name in front of it **three
times**. It ran `ls script/`, where `apropos` is the first entry. It read the guest's `apropos`
builtin in `crates/swish/src/lib.rs`. It read `apropos photosynthesis` in `SWISH_CHECK_SCRIPT`. The
affordance never fired, it reached no `design/decisions/` file, and it never opened `notes/net.md`,
which is the gap the tool exists for. The reason is placement: the only page that says what
`script/apropos` does is `notes/scripts.md`, and **five runs have now not opened `notes/scripts.md`
or `notes/README.md`.** Recorded in `script/apropos`'s own `BUGS`.

Still unopened after five runs: every file under `design/decisions/`, `notes/net.md`,
`design/naming.md`, `notes/scripts.md`, and `notes/README.md`. New to the list: `AGENTS.md`.

### What it found, and none of it was fixed here

- **`std-aborts` reports a contaminated build as a source defect.** The check never asserts that the
  paths in the dep-info are under `farm_dir()`, so a clone that compiled `std` out of another
  worktree's farm is told, with two files, two line numbers and two suggested fixes, that upstream
  source has changed. **Both suggested fixes would have written a false statement into
  `ABORTS_ACCEPTED`.** The failure caches, so re-running reproduces it in thirty seconds and looks
  stable rather than stale, and the recovery, `rm -rf std_exerciser/target`, is nowhere. Recorded in
  notes/std.md's `BUGS`. The stranger's own verdict on it: *"the worst defect... it means 'the tests
  passed' is not a property of a commit; it is a property of a commit and of what else that machine
  last built."*
- **There is an eighth edit site for a new program and it depends on the manifest.**
  `the_arg_line_follows_the_manifest_for_every_program` in `crates/swish/src/lib.rs` sweeps `Prog`
  and asks the manifest whether a program takes an argument, which is the generalisation its own
  doc comment argues for, and then builds `"<name> 21"` against `Holdings::default()` and hard-codes
  the rest. Any program requiring an argument **and** an input turns it red, in a crate the person
  adding the program never opened. Recorded in notes/adding-a-program.md's `BUGS`, unrepaired on
  `main`; the stranger repaired it only inside its disposable clone.
- **`CONTRIBUTING.md` describes `script/gates` as three stages and it runs five.** The two it omits
  are `script/icount` and a second `script/test --hvf`, and the HVF leg is the slowest and the only
  one that flaked in this run. This is the document that earned run 4's best result by telling a
  stranger what "tests passing" means here, so the sentence being wrong costs more than an ordinary
  drift would.
- **`script/setup`'s comment says the pin is `nightly-2026-07-26` "as of writing"** and
  `rust-toolchain.toml` says `nightly-2026-08-18`. Harmless in effect, and it was the first thing
  the stranger wrote in its journal, against a tree whose stated standard is that a duplicated fact
  is the one that rots.
- **`README.md`'s reading order counts 403 markdown files and 143 notes**; there are 413 and 145.
  A row that counts something goes stale, which this note's own `BUGS` predicted about the rubric
  and which is now true of the reading order too.
- **Capability slot numbers are computed from the manifest by `spawn_service`, not constant**, while
  every existing program's documentation states its slots as fixed facts and
  `notes/adding-a-program.md` does not mention it.
- **`script/swish-check` prints no transcript on success**, so a green run tells a newcomer nothing
  about what its new program actually did.

**Its worst-thing answer, which is run 4's criticism arriving independently and sharper.** Asked
what is worst about working here, it separated the worst defect from the worst experience and gave
the second as: *"the sheer volume of prose and how self-referential it is... It is more honest than
any codebase I have read and it is unnavigable in a day: I got a program shipped on both instruction
sets without opening the file the README names as one of the two you must read. The tree has an
answer for nearly everything and no way to reach the one you need in the time you have."* **Two
strangers in a row, independently, have named the documentation habit rather than any document**,
and run 5 supplies the falsifiable version run 4 did not: a working contribution on both ISAs, with
`AGENTS.md` unopened.

### What this run cost

**The machine-global contamination is the largest and it is new**, and it leads because it is the
only one that changed what the stranger did rather than how it wrote: a foreign path in its build
output made it read the harness, and reading the harness gave it the debrief questions and the
mechanism it needed to diagnose the failure. Then, largest first: **the disclosure changed the
program it chose**, by its own account, so this run's walk of `notes/adding-a-program.md` is a
deliberate probe rather than a newcomer's path and the null result a fresh contributor would have
produced is not available; **it performed anyway**, having been asked in the task text not to, so
the prose in its deliverables should be discounted exactly as run 4's was; **the tree leaked the
measurement in the first minutes**, from `README.md` this time rather than from
`notes/adding-a-program.md`, which is the fifth confirmation and the earliest one; **the machine was
warm**, so B2 measures nothing; and **the operator of this lane has read `AGENTS.md` in full**,
which no arrangement of processes fixes and which is worse here than for runs 3 and 4 for a specific
reason: this lane also read `script/stranger-test` before running it, so its judgement about whether
the harness worked is the judgement of someone who read the harness's own account of what it does.
The mitigations are unchanged and are the only ones available: the task text is runs 1 through 4's,
the rubric predates every run and was **not** amended by this lane, the pre-registration was
committed before the clone was cut, and the stranger's answers are recorded as it gave them.

**The three predictions, scored.** Registered before the run in the section above.

1. *The stranger does not find `script/apropos`, and therefore still does not reach any file under
   `design/decisions/`.* **Confirmed**, and in a stronger form than predicted: the name was in front
   of it three times.
2. *`notes/README.md` goes unopened for the fifth run running.* **Confirmed.**
3. *The build half is green first try with no change to the tree.* **Falsified.** `script/test` was
   red at 88 seconds on a machine-global defect the pre-registration did not anticipate, and finding
   it is the best thing this run did.
