# A dispatched CI run scopes against the branch, not against its last commit

**Status: PROPOSED 2026-09-26.** Raised by milestone 47 (navigation and naming)'s lane
`milestone/47-navigation`, gating in CI as `briefs/gate-in-ci.md` says. Its dispatched `ci.yml`
run reported every job green, and the build and boot had not run at all. The stem is a lane's
coinage and provisional.

**Gate: NONE.** A change to `.github/workflows/ci.yml` alone.

## What happened

`ci.yml` decides per job whether a change needs the build. It diffs against
`github.event.pull_request.base.sha` or `github.event.merge_group.base_sha`. A `workflow_dispatch`
run has neither, so it falls back to `HEAD~1`, the branch's last commit only. Six jobs carry that
line.

The lane's tip commit touched only `notes/`. So run 36255430502 printed "documentation only;
nothing a build or a boot can catch" and skipped `script/test` and `script/swish-check`. The
branch under it changed four crates, a component and `xtask`. Every job concluded success.

The brief tells every lane to dispatch, because a draft pull request skips the suite. A lane that
ends on a note or a roadmap edit, which the brief's own step 1 invites, gets a green run that
tested nothing. The merge queue is not affected: `merge_group` carries its base.

## Exit criterion

A dispatched run diffs against the merge base with `main`, fetched if the checkout is shallow. A
branch whose tip is documentation over a code change runs the build. A test of the scope step
shows both cases.

## Index row

A dispatched CI run scopes its build check against `HEAD~1`, so a branch ending on a docs commit
skips the build and reports green. Proposed: scope against the merge base with `main`.
