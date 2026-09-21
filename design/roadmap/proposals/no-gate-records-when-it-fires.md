# No gate in this tree records when it fires, so its own retirement rule cannot be applied

**Status: PROPOSED 2026-09-21.** Raised by a research lane calef briefed to ask what gate practices
this tree lacks, with the instruction that any gate must prove its worth, existing ones included.
The lane read `script/lint`'s 47 named checks, the thirteen workflows, `notes/check-inventory.md`,
`design/fatal-risks.md` and the failure records in `notes/`, and measured the git history for
evidence that each gate has caught a real defect. The 47 is derived rather than copied
(`grep -c '^echo "==>' script/lint`), because the number written into `notes/check-inventory.md` has
already gone stale once and that file says to derive it.

**Gate: DECISION.** Two of the three additions below change what every contributor runs, and the
third is a watcher rather than a gate. None should be built before calef rules on the first, because
the first is what would decide whether the others are worth keeping a year from now.

## The short version

**Nothing in this tree should be deleted on the evidence available, and that is the finding rather
than a reprieve.** This tree retires a check on a stated criterion, "a check that only ever rejects
valid work is measuring the wrong thing", which §61 (a lint is adopted on evidence from this tree,
not on its description) supplies the adoption half of and nothing supplies the retirement half of.
Applying it needs to know what a check has rejected. **No gate here records a single firing.** Three
checks have been retired under that criterion and each time the evidence was somebody's memory of
having been rejected, reconstructed after the fact.

So the ranked recommendation is one cheap instrument, one convention already proven on proofs and
not yet applied to gates, and one watcher for the hazards that sit outside the gate surface
entirely. Two refusals are recorded with their measurements, because a refusal with a number in it
is worth more than an addition without one.

## What was measured, and how

- **Firing evidence per gate, from the git history.** Every `script/` entry point and every
  `script/lint` check was searched for commits that fix a real defect it found, as distinct from
  commits that add or repair the gate and commits that fix a cosmetic violation to satisfy it. The
  method works here only because `allow_squash_merge` is false on this repository, so a lane's
  own commits survive on `main` and a gate that fired pre-merge usually leaves a fix commit behind.
- **`script/lint` end to end, from a lane worktree, warm cache: 84 seconds wall, exit 0.** The
  record-parsing legs are the cheap end: `roadmap --check` 0.27 s, `decisions --check` 0.08 s,
  `citations --check` 1.06 s, `fatal-risks --check` 0.65 s, `names --check` 0.13 s,
  `falsifications --check` 0.61 s. The ten clippy passes are the other 80 seconds.
- **The live state of the machine**, because two candidates are about state no file describes.

### What could not be established

**Sixteen of the 47 `script/lint` checks have no trail at all**, in either direction. They are
ratchets: each encodes a fix already made, and no commit says any of them later caught a new
instance. That is not evidence against them, because a ratchet that holds leaves no trace by design,
and it is exactly the case the first recommendation below exists to settle.

**`script/icount --check` is the one existing gate whose worth this lane could not establish and
would not defend.** It is a required check. It was demoted from a 2% gate to a 10% coarse tripwire in
2026-07 as the honest fix, because it could not attribute a wiggle to the commit in front of it; no
commit since records it catching a regression; and it generates baseline churn after toolchain bumps,
which already has a proposal of its own. The narrower `script/fastpath-footprint` covers the hot path
and **has** paid, loudly, on real work. This lane does not recommend retiring `icount` on that,
because "no commit says so" is exactly the weak evidence this proposal is about. It recommends
letting the ledger below answer it within a few weeks.

## Finding 1: the deletion criterion has no record to read

`script/lint` is one required check carrying 47 sub-checks, and it exits on the first failure, so a
slow or wrong one stalls every lane at once. Three have been deleted for the "only ever rejects
legitimate work" signature. The branch-prefix check is the documented case and it is instructive:
four separate false rejections of legitimate work, each one discovered when something broke, each
fixed by widening an allowlist, until calef asked what the taxonomy was for and the answer was that
nothing consumed it.

**Every one of those four was learned by being hit.** Nothing counted them. The same is true of the
cost side generally: the exact-count relations that became a merge hotspot were weakened twice,
reactively, each time after a lane paid; the unsafe-fn gate needed a trait-impl exclusion that was
twelve false positives out of thirteen without it, measured by hand at authoring time and never
since.

And the benefit side is no better instrumented. The em-dash check is the clearest case, because it
can be measured from history and the result is not what the intuition says: **it fires constantly**,
roughly twenty-odd times since 2026-08-01, with commit subjects like "an em-dash the style gate
caught, in a block about gates", and **it has never caught a defect**, because it is a house-style
rule and not a correctness property. Both halves of that sentence are worth having. Neither was
available without a pickaxe run over 3,000 commits.

### Addition A: a gate-firing ledger

**What it is.** Every gate that renders a verdict appends one line on failure: UTC timestamp, gate
name, sub-check name, branch. Local file, git-ignored, never read by any gate. `script/lint --fired`
(or its own entry point) reports firings per check over a window and, more usefully, the checks that
have never fired.

**Which recorded failure it would have caught.** None directly, and that is the honest statement: it
is an instrument, not a gate. What it would have prevented is the shape this whole audit ran into.
Milestone 191 (did the proofs catch the bugs? a retrospective of every real defect against the
harness that should have found it) had to assemble an eighteen-row corpus by hand to establish that
no Kani harness has ever caught a defect after the day it was written, and that finding is the single
most load-bearing fact in `design/fatal-risks.md`. The equivalent fact about the 47 lint checks is
currently unobtainable at any price under a lane.

**What it costs a contributor per run.** One append on failure, zero on success. Nothing on the
success path, which is the path that runs.

**False-positive risk.** None available: it renders no verdict and cannot fail a build. The risk is
the opposite one, a misread. Two biases have to be written at the report rather than discovered:
`script/lint` exits on first failure, so a check late in the file is under-counted relative to an
early one; and a lane that re-runs the same failure five times produces five rows.

**What it cannot check.** It counts firings, never value. On this ledger the em-dash check would sit
near the top and the `sh -n` check, which caught a genuine bash 3.2 parser bug that ShellCheck had
already passed, would sit near the bottom with one row. **A firing count is the cheap half of §61's
question and not the whole of it**, and it should be labelled that way where it prints. It also sees
only local runs: CI runners are ephemeral, so a check that fires mostly in CI reads as quiet unless
the workflow uploads its rows, which this proposal deliberately does not ask for in the first
version.

## Finding 2: the tree proves that a proof can fail, and does not prove that a gate can

The most frequently recurring failure in this tree's whole record is **an absent failure signal read
as a pass**, and it is not close. The instances are not variations on a theme, they are the same
defect in twelve places: milestone 214 (a test that prints "skipping" and returns is counted as
passed); a vacuous Kani harness reporting `SUCCESSFUL`; an assertion that answered "U-mode cannot
read the kernel" by refusing to look, green through every gate since milestone 41 (dead code: triage
the suppressions, and un-blindfold the gate); confinement tests that hang instead of going red; every
`uefi_loader` mutant recorded MISSED in "0s build + 0s test"; `cargo kani -p kernel` selecting
`arch/` by the host's `target_arch` and so proving one architecture on all three, which two
`assert!(false)` probes exposed as "4 successfully verified harnesses, 0 failures"; `script/fmt`
silently ignoring `--check` for months; a scheduled step piped through `tee` and exiting with tee's
status; `--shard 4/4` dying as an argument error in twenty seconds every week for a month;
`script/citations` reading exit 0 on a file `git ls-files` could not see.

**This tree already has the answer to that shape and applies it in one direction only.** §134 (a
harness carries a machine-replayable falsification record, or it is not evidence) makes a proof carry
a demonstration that it can come back red, and `script/falsifications --check` gates it. The same
discipline has been applied to gates three times, each time by hand and each time by somebody who
happened to think of it: `script/fatal-risks --selftest` runs eleven fixtures of which eight must go
red and three must stay quiet; `script/image-permissions` was proved both ways before it was wired
up, green on three images and red on the pre-fix x86_64 one; milestone 233 (`login` dies on every
boot, and the boot says it is ready) proved its killed-thread assertion able to fail before believing
it. All three are among the newest checks in the tree, and all three are the good practice arriving
as an instinct rather than as a rule.

### Addition B: a gate carries a selftest, or it is not evidence

**What it is.** §134's convention, moved from proofs onto checks. Each `script/` entry point that
renders a verdict grows a `--selftest` that runs it against fixtures which must fail and fixtures
which must stay quiet, and `script/lint` gains one check: an entry point with a verdict and no
`--selftest` is refused. The shape is already in the tree and does not need designing;
`script/fatal-risks --selftest` is the worked example and it costs 0.65 s combined with its
`--check`.

**Which recorded failures it would have caught.** `script/fmt` ignoring `--check`, which the record
calls the same defect as the `tee` fail-open and which went unnoticed for months. The `--shard 4/4`
argument error, which is a fixture that must produce a shard and did not. `script/citations`' input
and reading patterns disagreeing about lettered citations, which is precisely a "must fail" fixture
nobody wrote: three files in the tree are unreachable to that gate today and it was found by
accident. `script/shell-check` red on `main` on both architectures with nothing saying so. And
prospectively, the class `notes/check-inventory.md` closes on: *"Milestone 233's `login` was found by
somebody asking what a passing check proved, and that remains the only known way to find the next
one."* A selftest is that question, asked once, in a form that keeps being asked.

**What it costs a contributor per run.** A fixture sweep is milliseconds; the measured combined cost
of the one that exists is 0.65 s. The real cost is at authoring time, and it is the point rather than
a side effect: a gate is roughly a third more work to write, and the third buys the evidence that it
works.

**False-positive risk, and the method.** Near zero by construction, since a failing selftest means
the gate is broken rather than the tree. The residual risk is a selftest that rots into a nag, and
the estimate is from the one instance: `script/fatal-risks --selftest` has eleven fixtures and no
recorded false rejection since 2026-09-11. One data point, stated as one.

**What it cannot check.** That the fixtures cover the failure space. A selftest proves a gate *can*
fire, never that it fires on everything it claims, and the record already holds the sharper version
of this: a harness that restates the reader's own inequality cannot detect that the inequality is
wrong. A selftest written by the same person in the same hour as the gate inherits the same blind
spot, which is what milestone 191's survivorship finding says about proofs and says equally here. It
also cannot reach a gate whose subject is the live machine: `script/crate-probes` needs the network,
`script/qemu-check` needs an emulator, and a fixture for either is a fixture about a stand-in.

**And it wants a bounded adoption, not a sweep.** §61's corollary applies exactly: nothing goes in
"to see what it finds", because adding this check is a commitment to write every missing selftest
first. The defensible shape is the tree's own ratchet: the gate refuses a **new or modified** entry
point without one, the existing surface is a worklist, and the worklist is `script/`'s own listing.

## Finding 3: the hazards that destroy work are all outside the gate surface

`AGENTS.md` names three hazards that destroy work rather than delaying it: uncommitted work in a lane
worktree ("the one thing no part of this system protects"), disk ("the only pressure here that
destroys work rather than delaying it"), and the shared stash stack. All three are at rung four
today. **Nothing in `script/` or `scripts/` runs `git worktree list`, and nothing anywhere reads
`df`** outside one CI resource trace; both were grepped rather than assumed.

Measured on patagonia while writing this, 2026-09-21:

- **35 worktrees.** The 2026-07-31 incident that took the volume to zero bytes free was 42.
- **The main checkout's `target/` is 30 GB.** `AGENTS.md` records it at 7.2 GB and names it as the
  one nobody watches because it is not a lane and does not appear in `git worktree list`. It has
  quadrupled since, and nothing reported that.
- **134 GiB free of 460.** Not urgent, and that is the point: the number is fine today and no
  mechanism would have said otherwise on the day it was not.
- **Two worktrees carry uncommitted work.** `maintainer/metrics-2026-09-19` holds seven modified
  files, its last commit a day old. That is the 2026-08-04 failure verbatim, seven modified files and
  nothing looking, reproduced today and found only because this lane ran the command.

### Addition C: one watcher over the lane fleet

**What it is.** One script, run from `scripts/trunk-health.sh` the way `script/cadence-check` already
is, reporting three readings: worktrees with uncommitted work and no commit inside a window; free
space on the volume against a floor; worktree count and total `target/` footprint including the main
checkout's.

**Explicitly not a gate**, and the precedent decides it rather than taste.
`scripts/lane-claim-check.sh` faced the identical question and answered it in its own header: nothing
should fail a build over this, because the lane that most needs telling is mid-work, and `script/lint`
was refused for exactly that reason. The same applies here twice over, since the subject is another
worktree's state and a gate that fails a lane over a sibling's dirt would be the "only ever rejects
legitimate work" signature acquired on purpose.

**Why one script and not three.** They are one question, "is any lane's state at risk right now",
and they share their whole mechanism: one `git worktree list`, one `status --porcelain` per entry,
one `df`. Three scripts would be three cadences to keep alive and three headers to keep true.

**What it costs.** A poll, on a watcher that already polls. The `du` leg is the only expensive
reading and it is the one that can be sampled rather than run every pass.

**False-positive risk, estimated with a stated method.** Run today against the live fleet, the
uncommitted-work leg names 2 of 35 worktrees and both are true positives on inspection. The
shape to design out is the one `lane-claim-check.sh` already priced: a lane legitimately mid-edit.
A window measured from the last commit rather than from the last write handles it, and the same
script's five-times-measured grace period is the precedent for choosing the number.

**What it cannot check.** Whether uncommitted work is *wanted*: generated artifacts a lane has
deliberately not committed look identical to work about to be lost, and one of today's two hits is
regenerated SVG output. It cannot see growth during a long run, so a `df` reading is a snapshot and a
build that fills the volume in ten minutes passes it and then fails. It reports only the machine it
runs on. And it dies the way every watcher here dies, which `notes/merge-queue.md` records as the
unfixed defect one level up: neither existing watcher reports its own death.

### Addition D, smaller: refuse `git stash` in a shared checkout

`AGENTS.md` records the 2026-08-26 case where one session's stash entry was popped into another
lane's tree, and prescribes a patch file instead. That is rung four for a hazard with a rung-two
mechanism available, and the mechanism was verified here rather than recalled: git's
`reference-transaction` hook fires on `refs/stash` and can abort the transaction. Tested in a scratch
repository: the stash is refused, **the working tree is left exactly as it was**, and ordinary commits
are unaffected. Cost measured over 20 commits with and without the hook: 0.364 s against 0.809 s,
so about 22 ms per ref transaction, which is process spawn rather than the check. A fetch is one
transaction and one invocation, not one per ref.

It is ranked last because the recorded incident lost nothing, and it is included because it is the
only place in this audit where a rung-four rule can move up a rung for ten lines of shell. It cannot
reach the other two clobbers in the record, `git reset --hard` and `git checkout <file>`, because
both are legitimate commands whose damage is indistinguishable from their intent.

## What to weaken, with evidence

**`script/cadence-check`'s DEAD verdict is currently 67% false positive**, measured today by running
it: three workflows reported dead, of which one is true (`undefined-behavior check` has never had a
successful scheduled run, from 2026-08-10 to now, through two milestones that repaired it, and a
manual dispatch succeeding on 2026-09-17 is not a cadence). The other two are known-false by
construction. `audit-cadence`'s red **is** its signal, which the script's own header already records
as a case it cannot tell apart, and milestone 311 (the audit cadence tripwire, and the month of
correct alarms nobody acted on) is the record of what that costs. `vendor-watch` was monthly until
2026-09-17 and is now weekly, so its gap is a transition and not a death, which is the second half of
the false positive the header says will recur for the next non-weekly workflow anyone adds.

This is the tree's own "cries wolf" diagnosis applied to the watcher built to cure it, and both fixes
are already owed in the script's own header: derive each workflow's expected interval from the cron
it already reads, and let a workflow declare that red is its signal so the tripwire asks "did it run"
rather than "did it pass". Nothing else in this audit should be weakened.

## What this lane refuses to add, and the measurements that refuse it

**A gate on `AGENTS.md`'s lane line.** Every pull request an agent writes must open with it, and the
mechanism is rung four until milestone 128 (the automation gets its own identity, and the agents get
their own voice) delivers one. Compliance was measured over 400 pull requests since 2026-08-16: **398 carry it, and the two that do not are dependabot's**,
which are not agent-written and correctly should not. A gate would have zero true positives and two
false ones in four hundred, which is the retirement signature acquired at birth.

**A gate requiring a gloss on every citation.** Refused by `script/citations` already, on a sweep of
2,911 sites, and the 2026-09-19 census makes the number worse rather than better: 505 of 9,483
scheme-number pairs are named, 5.3%. The ratchet is the right shape and this lane found no argument
against it.

**More checks generally.** The tree considered this question on 2026-09-02 after four findings in one
day and wrote the answer down: the answer to four bad checks is not six more. This lane agrees, and
two of its three recommendations are not checks.

## What cannot be gated

This is not a short section because the list is thin. It is short because the tree has already
written it: a grep for the declarative forms of "nothing checks this" returns **509 lines** across
`notes/`, `design/` and `script/`, most of them deliberate, each sitting where a reader meets the
thing it is about. That density is the practice working, not a backlog.

The three that bound this proposal:

- **A premise being overtaken.** `design/fatal-risks.md` states it best about itself: a green
  `script/fatal-risks` means no status word contradicts the record it names, and is not a warrant
  that the arguments still hold. Milestone 275 (a gate that diffs `design/fatal-risks.md` against the
  roadmap it cites) closed the mechanical half and found four live disagreements on its first run;
  the larger half has no mechanism and this lane found no candidate for one. The alternative is the
  audit cadence, and the audit cadence is a tripwire whose red nobody acted on for five consecutive
  Mondays.
- **Whether a comment, a gloss or a name is *true*.** The tree's own worked example is exact: a
  `SAFETY:` comment describing capability validation sat above a `write_volatile` into a DMA page and
  passed `clippy::undocumented_unsafe_blocks` for as long as the file existed, because that lint asks
  whether a comment is present and never whether it is about the thing underneath it. The alternative
  is the promotion triggers of §71 (a limitation is promoted when it stops being a fact and becomes
  a plan), and the audits, both of which are reading.
- **Whether a green result means anything.** `notes/check-inventory.md` answered that question by
  opening each file and asking, found six checks whose green is narrower than their name, and closed
  by saying there is no reason to believe six is the whole set. Addition B is the nearest a gate gets
  to it and it is not the same question: a selftest asks whether a check *can* fail, and this asks
  whether it fails on the thing its name promises. That gap is where the next finding will come from.

## The ranking

1. **The ledger (A).** It is the only item that changes what anyone can know, it costs nothing on the
   success path, and it is what lets calef ask this question again in three months and get an answer
   instead of a lane. It also settles `icount` without anyone having to argue about it.
2. **The selftest convention (B).** The highest-value of the three by defects prevented, and the
   most expensive, which is why it goes second and why it wants the ratchet shape rather than a
   sweep. It is a decision rather than a build: the code is already in `script/fatal-risks`.
3. **The lane-fleet watcher (C).** The only one with a live failure standing in the tree as this was
   written. Ranked third because it protects against loss rather than against wrongness, and because
   a watcher nobody reads is this tree's most repeated disappointment.
4. **The stash refusal (D).** Ten lines, verified, and honestly the smallest thing here.

**If only one is taken, take the first.** The other three are opinions about where the next defect
will come from. The first is the instrument that would tell us whether this proposal was right.
