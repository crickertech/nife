# 204. A pushed lane branch with no draft pull request is a claim nobody can see

**Status: NOT-STARTED.** Minted 2026-08-31 by calef, after two lanes in one session pushed their
branches and never opened the draft pull request their briefs named as the first act. *(Number
provisional until the merge queue lands it.)*

**Gate: NONE.** The two commands it needs both exist and the rule it enforces is already written in
AGENTS.md; what is missing is anything that looks.

**In brief.** AGENTS.md §90 says a lane's first act is a draft pull request, and says why: **the draft
is the claim.** It is how two lanes cannot silently take the same milestone, the board is
`gh pr list --draft`, it costs one command, and a draft cannot be stuck in the merge queue because a
draft cannot be merged.

**Nothing checks it.** On 2026-08-31 the lanes for milestones 121 and 194 both pushed
`milestone/*` branches and opened nothing. The board was empty while two milestones were being
worked, and it was noticed only because calef asked.

## Why prose was never going to hold this

The instruction was in both briefs, in the section headed *First act*, with the exact command to run.
Both lanes read it and neither did it. **That is rung four behaving exactly as AGENTS.md says rung
four behaves**, and it is the second instance of the same shape in this project's short history: the
first was lanes ending their turn mid-gate, which that file already records as a recurring failure
needing a standing rule rather than a per-brief reminder.

A brief is prose. Prose in a brief is not a weaker version of a mechanism; it is the same rung as
prose anywhere else.

## The check

Compare the pushed lane branches against the open drafts, and report the branches with no claim:

```sh
git ls-remote --heads origin 'milestone/*'   # what is being worked
gh pr list --draft --json headRefName        # what has claimed it
```

**A report rather than a gate**, in `scripts/merge-drain.sh`'s family: nothing should fail a build
over it, because the lane that most needs telling is one that is mid-work and about to open its pull
request anyway. What it must do is be visible without anyone asking, which is the property the
current arrangement lacks entirely.

## What it must not do

- **It must not fire on a branch that has just been pushed.** A lane pushes and then opens the
  pull request, so there is a legitimate window of seconds. A grace period is required, and picking
  it badly makes the check either useless or noisy.
- **It must not treat a merged lane's leftover branch as a live claim.** A branch whose pull request
  merged and which nobody deleted is hygiene, not a missing claim, and reporting it as one trains
  people to ignore the report.
- **It must not be a gate.** See above.

## BUGS

- **It detects the absence of a claim, not a collision.** Two lanes on the same milestone with two
  drafts is the case §90 actually fears, and this check would call that fine. Detecting the real
  collision needs the milestone number parsed out of the branch name, which is a convention nothing
  enforces.
- **It cannot see a lane that has not pushed at all**, which is the more dangerous state, because
  AGENTS.md says a lane's pushed branch is the only ledger another session can read and uncommitted
  work in a worktree is the one thing no part of this system protects.
- **Where it runs is not decided here.** `script/lint` is wrong (it is a gate), and the `launchd`
  watchers die with the session that started them; a GitHub Actions cron cannot see local worktrees
  but can see pushed branches, which is all this check needs.
