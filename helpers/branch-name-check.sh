# scripts/branch-name-check.sh: the one shape rule for a milestone-claim branch name.
#
# There is no shebang because this file is sourced, not run, the same reason
# scripts/qemu-path.sh has none: it exists to be read into another script's shell rather than
# spawn its own, so ShellCheck is told which dialect to assume instead of inferring one.
# shellcheck shell=sh
#
# **Why this moved out of script/lint** (2026-09-22). The rule lived as a `case` statement inline
# in script/lint's check 4, which only ever sees a branch that already exists: the check runs after
# `git worktree add`, after the work is written, sometimes after commits. On 2026-09-22 the
# maintainer hit it there and paid the late price: delete the branch, delete the worktree, redo the
# `git worktree add` with the right name, re-apply the commit. A rung-two gate delivering a
# rung-four experience, because the only place the rule was checkable was the last place it was
# cheap to fail.
#
# script/claim exists to run the same refusal BEFORE any of that: before the worktree, before the
# branch, before a single commit. That means two callers now need the same answer for the same
# name, and copying the four-line `case` statement into a second file is exactly the failure this
# project's naming rule calls out elsewhere, a second copy that quietly stops agreeing with the
# first. So the `case` statement lives here once, and script/lint and script/claim both source it.
#
# Name: provisional, minted 2026-09-22 pulling this out of script/lint's check 4. Hyphenated and
# unadorned like scripts/qemu-path.sh, the other sourced-not-run helper in this directory; a verb
# phrase because it names an action (check a branch name) rather than a thing. calef has not ruled
# on it.

# nife_check_branch_name_shape <branch> [prefix]
#
# Returns 0 for a correctly spelled `milestone/N-slug` claim, and for every branch that claims no
# milestone at all (a prefix nothing reads is a convention, not a gate; script/lint's check 4 has
# the reasoning for why that half is deliberately unbounded).
#
# Returns 1 and prints the explanation, on a near-miss: `milestone-126-pgrep`, `milestones/9-foo`,
# `Milestone/3-x`. A near-miss is refused rather than let through because it would sail past
# script/lint's check 4b in silence: that check moves a milestone's roadmap status and matches only
# `milestone/*`, so a branch that LOOKS like a claim but does not parse as one would skip the
# status-moves-at-merge check with nothing to say so. See DECISIONS §77 (the branch-prefix list now
# describes the tree) for why the prefix taxonomy is otherwise unbounded, and DECISIONS §90 (the
# claim is a draft pull request; the status flip is a gate) for what the roadmap-block check protects.
#
# `prefix` names the caller in the printed message (`lint`, `claim`, ...) and defaults to
# `branch name`; the substance of the explanation is the same for every caller, only the byline
# differs.
nife_check_branch_name_shape() {
    # $1 and $2 are read directly rather than copied into named variables: POSIX sh has no `local`,
    # and this function is sourced into a caller's own shell, so a plain assignment here would leak
    # into whatever called it.
    case "$1" in
        milestone/[0-9]*-*) return 0 ;;
        milestone/*|milestone-*|milestones/*|Milestone*)
            echo "${2:-branch name}: branch '$1' looks like a milestone claim but does not parse." >&2
            echo "Spell it milestone/<N>-<slug>, so the roadmap-block check can read it." >&2
            echo "A near-miss is refused because it would skip that check in silence (§77, §90)." >&2
            return 1
            ;;
        *) return 0 ;;
    esac
}
