**Provisional name.** What a maintainer session does before it briefs anything. You have just opened
a session on patagonia, the machine lanes run on, and you do not yet know what happened while nobody
was here. Do the work; do not ask questions.

**Where this came from.** The two clauses below lived in `AGENTS.md` (the `launchd` watchers, and
reading the queue for what the watchers already found), moved here on 2026-09-23 by the extraction
that milestone 579 (which of the constitution must be carried, and which is a brief) proposed. The
trigger is a specific, recurring, nameable moment, the start of a session, which is the test that
moved them.

**Why it is a checklist rather than a habit.** Both of these are duties that belong to whoever
happens to notice, and that is precisely the arrangement this project has already watched fail. On
2026-08-04 three such duties went unperformed in one evening: two green pull requests sat unmerged
for hours, `main` went red with nobody assigned, and merging one pull request staled eight others
that nothing picked back up.

## 1. Are the two watchers alive

    launchctl list | grep nife

Two entries must be present, `com.nife.merge-drain` and `com.nife.trunk-health`. If either is
missing, start it the old way, which is still how both run on any machine that is not patagonia:

    cd /Users/calef/projects/nife
    scripts/merge-drain.sh &
    scripts/trunk-health.sh &

`notes/merge-queue.md` has the `launchd` plists, the gap calef accepted rather than solved, why this
is deliberately not automated further, and a `BUGS` section honest that neither script reports its
own death.

## 2. Read what they already found

**A running watcher is not the same as a watcher that has been read, and this is the half that gets
skipped.** `merge-drain.sh` posts once per stall and then goes quiet by design, so a stalled pull
request does not re-announce itself every five minutes. Nothing re-announces it to a session that
opens later either. A watcher that reported at 03:00 and a session that opens at 09:00 never meet
unless somebody goes looking.

So go looking, and do it by running the brief that already exists:

    briefs/survey-the-queue.md

That brief is the queue read, in full: it fetches, lists `mergeStateStatus` and `statusCheckRollup`,
distinguishes a `DIRTY`/`CONFLICTING` pull request that needs a rebase from a `BLOCKED` one whose
checks are merely still running, treats a `needs-architect` hold as a deliberate hold rather than a
failure, and names the traps (`auto=false` does not mean unqueued; an absent pull request may have
merged rather than vanished). **Do not restate its commands here or type them from memory.** This
step is one line on purpose: a second copy of those instructions is a second thing to keep correct,
and the copy that drifts is always the one nobody is looking at.

## 3. Treat what it reports as tasks, not as a status line

This is the whole reason step 2 exists, and it is the failure the steward role was meant to cover
and did not, for a reason worth keeping: **it reported and never acted.** A stalled queue announced
in a message is only useful if somebody reads the message and then does something.

Every `DIRTY` or `CONFLICTING` pull request and every `FAILURE` conclusion the survey names is a task
to resolve, ranked the same as keeping lanes full, not a line to skim past. A conflict needs judgment
a watcher does not have: **a queue reports, it does not resolve.**

Two of them already have briefs of their own:

- a rebase needed: `briefs/rebase-onto-main.md`
- a failing check to diagnose: `briefs/triage-a-failing-check.md`

## EXAMPLES

Both watchers alive (real output, patagonia, 2026-09-23):

    $ launchctl list | grep nife
    -	0	com.nife.trunk-health
    -	0	com.nife.merge-drain

The first column is the process id, `-` meaning the job is loaded but not currently running, which is
correct for an interval job between ticks. The second column is the last exit status: **`0` is what
you want, and a non-zero number there is a watcher that ran and failed**, which looks identical to a
healthy one if you only check that the line exists.

A session that finds one missing:

    $ launchctl list | grep nife
    -	0	com.nife.trunk-health
    $ cd /Users/calef/projects/nife && scripts/merge-drain.sh &
    [1] 48213
    $ launchctl list | grep nife
    -	0	com.nife.trunk-health

Note what that last line shows: starting it by hand does **not** make it appear in `launchctl list`,
because it is now an ordinary background process rather than a `launchd` job. Check it with
`pgrep -fl merge-drain` instead, and expect to have to start it again next session.

## Stop, do not improvise

- **A watcher whose `launchctl` exit status is non-zero.** Report it with the number. Do not reload
  the job repeatedly hoping it takes; something is failing and the status is the only evidence.
- **Anything the survey reports that is not a rebase, a failing check, or a `needs-architect` hold.**
  Say what you saw and stop.

## BUGS

- **Neither watcher reports its own death**, which is the gap calef accepted rather than solved. Step
  1 exists only because of that, and step 1 runs when a session remembers to run it, so a watcher can
  be down for as long as nobody opens a session.
- **`launchctl list` shows the job, not whether it is doing anything useful.** A job loaded, exiting
  0, and reading the wrong repository would look exactly like a healthy one here.
- **Nothing measures whether a session actually runs this.** There is no gate, no log, and no record
  that a session started; the honest position is that this is rung three of the constitution's
  ladder, a written record for whoever opens it.
- **Steps 2 and 3 are the same instruction split across two files**, deliberately, and that split
  costs something: a reader who opens only this brief learns that the queue must be read, and has to
  open `briefs/survey-the-queue.md` to learn how. The alternative was a third copy of the queue-read
  instructions, which is the failure mode this extraction was written to remove.
