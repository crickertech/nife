# The weekly falsification sweep replayed nothing and reported success

**Correction of error, 2026-09-23.** The first one this tree has written, under
§210 (a correction of error, and its action items are decisions, proposals or milestones), which
sets the shape below and rules that an action item is a decision, a proposal or a milestone.

**The path is provisional.** §210 proposes `notes/corrections/<date>-<slug>.md` and names it as a
suggestion rather than a ratified name, because calef names what a reader meets. This file follows
the suggestion and says so. `notes/corrections.md` stays the index and now points here.

**Blameless, and that is load-bearing here rather than polite.** Both halves of this defect were
written deliberately, by people who explained their reasoning in the file, and both were right on
their own. Nobody was careless. What follows is about the interaction and about a habit, and it
would be a worse document if it had a name in it.

## What happened

`.github/workflows/falsifications.yml` runs the weekly falsification sweep: it replays every
recorded falsification patch against its own Kani harness to prove the harness can still be made to
go red. §134 (a harness carries a machine-replayable falsification record, or it is not evidence) is
the convention; milestone 194 (build §134: the falsification record, its lint, and the sweep that
replays it) built it.

The sweep step is

    script/falsifications --sweep 2>&1 | tee sweep.txt

`tee` creates `sweep.txt` in the repository root, and it creates it before the script it is piped
from has read anything. `script/falsifications --sweep` refuses to run on a dirty working tree,
because it applies and reverts patches with `git apply` and a failure mid-run would otherwise leave
a deliberate defect in somebody's source. That refusal is its own `BUGS` entry, and it is correct:
you cannot sweep while you work. On a runner, the untracked `sweep.txt` is the work.

So the script printed its refusal and exited non-zero. The step carries `continue-on-error: true`,
which was put there for a stated reason: a survivor is a finding to publish rather than a red check
somebody has to silence. The non-zero exit was swallowed, the next step read `sweep.txt` and
published its contents to the job summary, and the job went green. The published report was the
refusal.

Three lines from run 35592675887, the scheduled run of 2026-09-21, in full:

    script/falsifications --sweep refuses to run on a dirty working tree: it applies and
    reverts patches with `git apply`, and a failure mid-run would leave a deliberate
    defect in your source. Commit or set the change aside first.

    ?? sweep.txt

`?? sweep.txt` is the whole of the dirty tree. The file the sweep's own output was being captured
into is the file that stopped the sweep.

## Timeline

All times UTC. The workflow's whole life fits in one table.

| When | What |
|---|---|
| 2026-08-31 18:18 | `24a1e0a7b` lands the lint hook, the weekly sweep, the per-pull-request gate and the note. The `tee` and the `continue-on-error` are both in that first commit, and the file has not been touched on `main` since. |
| 2026-09-07 10:47:57 | First scheduled run, 34113314569. Refuses at 10:48:35.396, 0.5 s after the step starts. Reports success. |
| 2026-09-14 11:01:12 | Second scheduled run, 34836043991. Refuses at 11:01:46.017. Reports success. |
| 2026-09-21 11:10:46 | Third scheduled run, 35592675887. Refuses at 11:11:24.014. Reports success. |
| 2026-09-23 20:30:04 | `falsify/the-single-harness-crates` dispatches the workflow by hand to check its new records. Refuses at 20:30:52. That lane reads the log rather than the green tick and finds it. |
| 2026-09-23 20:32:30 | The same lane pushes `401203294`, which sends the transcript to `$RUNNER_TEMP` instead of the checkout. Pull request #1156. |
| 2026-09-23 20:32:35 | A dispatch on the fixed commit starts. It is the only run of this workflow that would have swept anything. |
| 2026-09-23 20:36:38 | `falsify/crates-paging` dispatches from its own branch. The workflow's `concurrency` group is the bare string `falsifications`, with no ref in the key, so `cancel-in-progress` cancels the fixed run above. It never replays a patch. |
| 2026-09-23 20:37:53 | That dispatch refuses, on a branch without the fix. |
| 2026-09-23 20:49:25 | `falsify/machine-discovery` dispatches. Refuses. |
| 2026-09-23 20:50:57 | `falsify/crates-paging` posts the same finding as a comment on pull request #1159, deliberately not fixing it, because `.github/` is shared and two siblings were dispatching that workflow at the time. |
| 2026-09-23 21:16:38 | This record's own pull request, #1166, trips a fifth instance of the same shape while being written. `coe-architect-label.yml`, merged hours earlier to put `needs-architect` on every COE by default, detects the new file correctly and then fails to apply the label, because `gh pr edit` was called without `--repo` in a job that never checks the repository out. The step's `if` takes the else arm as designed, the job reports **pass**, and no label appears. |

**Six completed runs, six refusals. The count of falsification patches this workflow has replayed in
its lifetime is zero.** The one run that carried the fix was cancelled by its own concurrency group
before it got there.

## Impact

**Every scheduled sweep the mechanism has ever had produced no evidence, and published a refusal as
though it were a report.** That is 23 days from the workflow landing to the finding, covering the
three Mondays of 2026-09-07, -14 and -21.

**The population it was not checking was growing the whole time**, which matters because the value
of a sweep is in the records it did not see last week:

| Date on `main` | Falsification patches | of which kernel-test | `#[kani::proof]` harnesses |
|---|---|---|---|
| 2026-09-07 | 40 | 4 | 156 |
| 2026-09-14 | 40 | 4 | 156 |
| 2026-09-21 | 74 | 18 | 184 |
| 2026-09-23 | 78 | 18 | 184 |

Nineteen commits touched `*/falsifications/*` inside that window. **Three of them redid a patch
against code that had moved**: `651a3fa43` regenerated the x86 port-revoke patch that pull request
#1059 staled, `00189fefa` redid the W^X record against the encoder as it then stood, and
`8470caa21` refreshed the span falsification against a simplified harness. Stale patches are exactly
what the weekly sweep exists to report. All three were found by a lane already working in that
crate, which is luck rather than mechanism, and is the coverage the sweep was supposed to replace.

**The kernel-test half had no second checker at all.** The per-pull-request half,
`script/falsifications --affected-since` in `ci.yml`, did run throughout and is unaffected, but it
covers only the Kani half and only harnesses a diff can reach. `script/falsifications`' own `BUGS`
section already says a kernel `#[test_case]` falsification is re-checked by a full `--sweep` and by
nothing else. Four such records existed for the first two weeks of the window and 18 for the third.
Nothing looked at any of them.

**What is not claimed.** No survivor and no stale patch is on record for this window that the sweep
would have caught, and none can be, because the instrument produced no reading either way. The
honest statement is an absence of evidence rather than evidence of absence, and the three redone
patches above show the class was live.

**The population moved again on the day of the finding.** `main` today carries 63 of 180 Kani
harnesses replayable, 35%, plus a separate 15 of 16 kernel-test records. Three open falsification
lanes roughly double the first number. The first sweep that actually runs will be the first reading
ever taken, against a record set that has changed more in one day than in the three weeks before it.

**And the worst of it is what the mechanism is for.** `design/fatal-risks.md`'s risk 2 is that the
proofs prove trivia and the real bugs live where Kani cannot reach. This sweep is part of how that
risk is answered. The instrument built to ask whether our proofs are evidence was producing none
itself, and saying so in green.

## Root cause: five whys

**1. Why did three weekly sweeps replay nothing?**
Because `script/falsifications --sweep` refused to run and its non-zero exit was discarded by
`continue-on-error: true`, so the job's only assertion was one that had been deliberately switched
off.

**2. Why did it refuse?**
Because `tee sweep.txt` wrote an untracked file into the checkout before the script read the tree,
and the script's dirty-tree guard is a guard on the whole tree rather than on the files it patches.
The guard is right to be that broad: it cannot know in advance which files a patch will touch.

**3. Why did nobody notice a report about 40 records that contained no records?**
Because the report has no reader with an expectation. It is published to a job summary, consumed by
a human on a Monday if anyone opens it, and nothing downstream reads its contents. Its emptiness was
not a signal that anything consumed, so it was not a signal at all. This is the first half of what
went wrong, and it is a property of the design rather than of anyone's attention.

**4. Why was a job with no assertion on its content built that way on purpose?**
Because `continue-on-error` was implementing a genuine ruling about the **verdict**: §134 says a
survivor is a worklist entry and not a defect in whatever commit happened to precede the cron, so a
survivor must not go red. The mistake is that a step's exit status was carrying two different claims
at once. "The sweep ran and found something to look at" and "the sweep did not run" are not the same
fact, and once they share one channel, suppressing the first necessarily suppresses the second.
Nothing in the workflow could tell them apart afterwards, because the only thing that could have is
the report's own contents, which nothing reads.

**5. Why does this tree keep building mechanisms that cannot tell "checked and found nothing" from
"checked nothing"?**
Because a gate is written against the defect it hunts, and the empty-input case is the one state its
author is not thinking about while writing it. From outside, checking nothing is indistinguishable
from finding nothing, and it is what every new gate does on the day it lands, before its subject
exists. The review that follows asks whether the gate catches the defect; nothing asks what it does
when its input set is empty. **This was the fourth instance of the shape found in a single day, and a fifth arrived while
this document was being written**; that recurrence, not the `tee`, is the finding:

- **Milestone 401 (a gate that selects the set it judges can pass by checking nothing)**, pull
  request #1130, went looking for the class in `script/` and found it. Eight selectors now assert
  they selected something, enumerated in `notes/empty-selectors.md`.
- **`script/ci-build`'s tier selector.** It picks its checks by matching a literal tier string in
  its own table with `awk`. Retagging every `local` row to `default` makes the command a developer
  runs before pushing run zero checks and print `ci-build: all pass`, exit 0. Reproduced twice
  independently, by 401's lane and by pull request #1134.
- **The draft gate.** `ci.yml`'s own comment names it: a skipped job still posts a conclusion and
  still satisfies a required check, so skipping is never the safe default. That file's `gate` job
  exists because it had to learn it more than once.
- **This sweep**, where the skipped work is the whole subject of the workflow.
- **And a fifth, found by this document's own pull request**, which is the strongest evidence in it
  that the shape is a habit rather than four coincidences. The workflow that labels a COE for
  calef's attention reported success while applying no label. Its detection step names the
  repository explicitly and worked; its labelling step inferred the repository from a git remote
  that a checkout-free job does not have, and its deliberate report-and-continue arm turned the
  failure into a pass. Every individual choice there is one this tree argues for on purpose, in
  that file's own header: best-effort, never fatal, a COE must not fail to exist because a label
  API call failed. The result is still a green tick over nothing.

The ladder in `AGENTS.md` ranks how hard to make a rule hold. It has nothing to say about a
mechanism proving it had something to hold. **A gate that reported clean should have to say over how
many units**, and zero should be loud. That is a rung-two artefact this tree does not have a
convention for, and building one is what the second action item below is.

## Action items

- **Done.** The transcript goes to `$RUNNER_TEMP` instead of the checkout, so the sweep's own
  logging can no longer dirty the tree it is about to patch. Carried by pull request #1156, commit
  `401203294`, on branch `falsify/the-single-harness-crates`; that pull request owns
  `.github/workflows/falsifications.yml` and this record deliberately does not touch it.
- **Proposed.** `design/roadmap/proposals/a-mechanism-reports-its-denominator.md` (name provisional):
  survey the 5 `continue-on-error: true` steps and 19 `|| true` constructs across the 14 workflows,
  decide for each whether it suppresses a verdict or an outcome, and give every job that publishes a
  report a denominator it must assert is non-zero. It is the workflow-level counterpart of what
  milestone 401 (a gate that selects the set it judges can pass by checking nothing) built inside
  `script/`.
- **Done.** `coe-architect-label.yml`'s labelling step now passes `--repo "$GITHUB_REPOSITORY"`, so
  it no longer depends on a git remote that a job without a checkout does not have, and the reason
  is written beside the flag. Carried on this branch, `maintainer/the-sweep-that-swept-nothing`.
  The label on this pull request was applied by hand in the meantime, and it will stay hand-applied
  here: the labeller's `synchronize` path diffs only the push that raised it, deliberately, so that
  a label a human removed does not silently return. No later push to this branch adds a file under
  `notes/corrections/`, so nothing on this pull request will exercise the fixed step. The next COE
  is the first run that can.
- **Recorded.** The `concurrency` group in `falsifications.yml` is the bare string `falsifications`
  with no ref in the key, so a hand dispatch from any branch cancels a running one from any other.
  That is what killed the only run of this workflow that would have swept anything. The limitation
  now sits beside the feature in `notes/falsification.md`'s `BUGS` section, where a lane about to
  dispatch the sweep will meet it.
