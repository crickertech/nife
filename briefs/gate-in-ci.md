Gate a branch's work in GitHub Actions rather than on this laptop. You are in the branch's git
worktree, your work is written, and you are ready to prove it. Do the work; do not ask questions.

**Why this brief exists rather than "run `script/test`".** The gates are the heaviest thing this
project does: three architectures of QEMU, and `script/verify`, whose own header records a Kani
harness reaching **3.5 GB**. Running them here is what caps concurrent lanes at three or four, and
it has killed a session outright. CI has more memory than this machine and nobody is waiting on a
lane, so the wall-clock a dispatched run costs is free and the memory it saves is not.

## Step 1: run the cheap gates locally, because a 25-minute run must not die on a typo

These take seconds, need no emulator, and catch most of what fails:

    script/lint
    script/roadmap --check
    script/fmt
    script/citations --ratchet

`script/citations --ratchet` reads the **committed** state, so commit before running it. Fix what
they report and commit again. Do not push until all four are clean.

## Step 2: push, then dispatch the suite explicitly

    git push -u origin HEAD
    gh workflow run ci.yml --ref "$(git branch --show-current)"
    gh workflow run verify.yml --ref "$(git branch --show-current)"

**The second and third commands are not optional, and this is the whole trap.** §90 (the claim is a draft pull request; the status flip is a gate) opens every lane
as a **draft** pull request, and both workflows begin with a `draft gate` job that asks the API
whether the pull request is a draft right now and **skips the entire suite when it is**. So a push to
a lane branch raises `synchronize`, the gate reads `draft: true`, and all thirteen checks report
"skipped". **A skipped check still posts a conclusion and still satisfies a required check**, which
is how #567 once sat mergeable with nothing verified. Pushing is therefore not gating. A
`workflow_dispatch` run is not a `pull_request` event, so the gate returns `run=true` and everything
actually executes.

## Step 3: watch it, and do not dispatch twice

    gh run list --branch "$(git branch --show-current)" --limit 5
    gh run watch <run-id> --exit-status

Both workflows set `cancel-in-progress` for any ref that is not `main`, so **a second dispatch
cancels the first**. If you think a run is stuck, read it before re-running it; a cancelled run
reports as a failure and tells you nothing.

Expect **23 to 29 minutes** for `ci.yml` on real code, 2 to 3 on a documentation-only branch where
nearly every job skips on scope. `verify.yml` takes about **47 minutes** and is the long pole; it is
a separate workflow precisely so a flaky three-minute job can be retried without waiting for it.

## Step 4: read a failure with `briefs/triage-a-failing-check.md`

That brief has this repository's traps: that `test exited abnormally` is the consequence rather than
the cause, that a `bench: CHECK FAIL` line carries both numbers, that `fastpath-footprint` names an
ISA, and that a cancelled job is not evidence of a defect. Use it rather than reading the log raw.

Fix, commit, push, dispatch again. **Each round costs about half an hour of wall-clock and nothing
else**, which is the trade this brief is making: rounds are cheap because no person is waiting, and
the laptop's memory is not spent at all.

## What you must not do

- **Do not run `script/verify` or `script/test` locally to "check first".** That is the cost this
  brief exists to avoid, and two of them at once is the out-of-memory kill.
- **Do not mark the pull request ready** to make CI run. Dispatch instead. Ready means the work is
  finished and asking to merge; a lane still gating is neither.
- **Do not push to a branch that is in the merge queue.** GitHub refuses it with `GH006`, and
  `auto=false` does not mean unqueued.
- **Do not enqueue anything.** Gating proves the work; merging is the maintainer's.

## BUGS

- **A dispatched run and a push-triggered run share a concurrency group**, so pushing immediately
  after dispatching cancels your own run. Push first, then dispatch, in that order.
- **Nothing here measures what a CI round costs against a local one.** The claim that rounds are
  free rests on lanes being asynchronous, which is true today because one person reviews them. It
  stops being true the moment a lane blocks something a person is waiting for.
- **The scope guard can skip jobs on a branch that genuinely needed them** if the change only
  touches paths it considers documentation. A suite that finishes in three minutes on a code change
  has skipped, not passed.
