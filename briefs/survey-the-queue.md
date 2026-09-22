Report the state of the merge queue. You are in the repository's git worktree. **Change no files.**
This is read-only and its whole output is your final message.

    git fetch origin
    gh pr list --json number,isDraft,mergeStateStatus,title,statusCheckRollup

## What to work out for each open pull request

- **Is it ready or draft**, and is auto-merge already enabled.
- **Is it `DIRTY` or `CONFLICTING`**, which means it needs a rebase before anything else can happen.
- **Does it have a failing check**, and which one by name.
- **Is it `BLOCKED` only by the `needs-architect` label**, which is a deliberate hold rather than a
  problem, and must be reported as such rather than as a failure.

## The traps

- **`BLOCKED` with no failing check usually means checks are still running.** Count the checks with
  no conclusion yet and say so, rather than reporting it as stuck.
- **`auto=false` does not mean unqueued.** GitHub clears the auto-merge request once a pull request
  enters the merge queue, so a clean pull request showing `auto=false` may already be queued. Check
  `gh run list --event merge_group` before claiming anything is idle.
- **A pull request absent from the default listing may have merged**, not vanished. Confirm with
  `--state all` before reporting it as gone.

## What to report

A table: number, ready or draft, state, and either the failing check's name or the reason it is
held. Then one short list of **what needs a human**: rebases needed, failures to investigate, and
anything held on `needs-architect`.

Do not rebase anything. Do not enqueue anything. Do not edit or commit.
