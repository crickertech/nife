#!/bin/sh
#
# Report pushed lane branches that have no pull request claiming them.
#
#     scripts/lane-claim-check.sh          # one pass, then exit
#
# PROVISIONAL NAME. Minted 2026-08-31 by milestone 204's lane; not put to calef. See `Name:` below.
#
# # Why this exists
#
# AGENTS.md §90: a lane's first act is a draft pull request, and the reason is that **the draft is
# the claim.** It is how two lanes cannot silently take the same milestone, the board is
# `gh pr list --draft`, it costs one command, and a draft cannot be stuck in the merge queue because
# a draft cannot be merged.
#
# **Nothing checked it.** On 2026-08-31 the lanes for milestones 121 and 194 both pushed
# `milestone/*` branches and opened nothing; the board was empty while two milestones were being
# worked, and it was noticed only because calef asked. The instruction was in both briefs, in a
# section headed *First act*, with the exact command. That is rung four behaving exactly as
# AGENTS.md says rung four behaves, and it was the second instance of the shape in this project's
# history: the first was lanes ending their turn mid-gate.
#
# A brief is prose, and prose in a brief is not a weaker mechanism than prose anywhere else. It is
# the same rung.
#
# # What it deliberately is not
#
# **Not a gate.** Nothing should fail a build over this, because the lane that most needs telling is
# one that is mid-work and about to open its pull request anyway. `script/lint` was refused for
# exactly that reason. What was missing was never enforcement; it was anything that looks.
#
# **Not a nag.** Three false-positive shapes were designed out, because a report that cries wolf
# gets ignored and then the real case goes unread:
#
#   - **The legitimate window.** A lane pushes and then opens the pull request, and GitHub refuses a
#     pull request with no commits between the branch and `main`, so a lane must produce something
#     committable first. Measured on this script's own branch: 3 minutes from branch creation to
#     draft, and that included writing the file that made the branch non-empty. `GRACE_MINUTES` is
#     five times that, and well under `merge-drain.sh`'s 75-minute stale-draft threshold, which is
#     the neighbouring report and the one this must not duplicate.
#   - **A merged lane's leftover branch.** A branch whose pull request merged and which nobody
#     deleted is hygiene, not a missing claim. It is reported on its own line, with the word
#     `LEFTOVER` and the pull request number, so nobody has to read it as an accusation.
#   - **A branch someone opened a non-draft pull request for.** A ready pull request is a louder
#     claim than a draft, not a quieter one.
#
# # The clock, and why it is the branch's birth rather than its last commit
#
# The grace period runs from **branch creation**, taken from the repository activity feed, and a
# later push does not reset it. That is the whole point: a lane that keeps committing is precisely
# the lane whose missing claim matters, and a last-commit clock would go quiet for exactly the
# branches that are being worked hardest. `merge-drain.sh`'s `stale_drafts` uses the opposite clock
# for the opposite reason, and the pair is worth reading together.
#
# Name: unrecorded. Provisional. `lane` and `claim` are both AGENTS.md's own words for these things
# (`§90`: "the draft is the claim"), and `-check` matches `script/qemu-check` and
# `script/stack-frame-check`. It lives in `scripts/` rather than `script/` for `merge-drain.sh`'s
# reason: it is a maintainer's tool, not a front door a contributor types.
#
# # BUGS
#
#   - **It detects the absence of a claim, not a collision.** Two lanes on the same milestone with
#     two drafts is the case §90 actually fears, and this calls that fine. Detecting the real
#     collision needs the milestone number parsed out of the branch name, which is a convention
#     nothing enforces.
#   - **It cannot see a lane that has not pushed at all**, which is the more dangerous state:
#     AGENTS.md says the pushed branch is the only ledger another session can read, and uncommitted
#     work in a worktree is the one thing no part of this system protects.
#   - **`milestone/*` only.** A lane on `fix/`, `roadmap/` or `maintainer/` is invisible here, and
#     those are legitimate lane prefixes that `script/lint` accepts. Widening the pattern would also
#     sweep in short-lived maintainer branches that are not claims and are not meant to be, so the
#     narrow version ships and the gap is written down rather than guessed at.
#   - **The activity feed is read one page deep.** A `milestone/*` branch created more than 100
#     repository events ago has no visible birth, and is reported rather than skipped: an old branch
#     with no claim is the case worth seeing, so the fallback errs loud instead of silent.
#   - **It reports to stdout only.** `merge-drain.sh` can comment on the pull request it is
#     complaining about; a branch with no pull request has nowhere to be told. Whoever reads the
#     drain's log reads this, and nothing reaches a lane that is not looking.

set -e
cd "$(dirname "$0")/.."

REPO="crickertech/nife"
GRACE_MINUTES=${GRACE_MINUTES:-15}

branches=$(git ls-remote --heads origin 'milestone/*' 2>/dev/null |
	sed 's|.*refs/heads/||' | sort)
[ -z "$branches" ] && exit 0

# Every pull request that has ever named one of these branches as its head, in any state. `--state
# all` is what separates a missing claim from a merged lane's leftover branch, and getting that
# wrong is the failure mode this whole report is designed around.
prs=$(gh pr list --repo "$REPO" --state all --limit 200 \
	--json number,headRefName,state,isDraft 2>/dev/null || echo '[]')

# Branch birth times. `branch_creation` is the only event that answers "when did this claim become
# due"; commit dates cannot, because a branch pushed empty carries `main`'s commit date and would be
# reported the instant it existed.
activity=$(gh api "repos/$REPO/activity?per_page=100" 2>/dev/null || echo '[]')

for branch in $branches; do
	pr=$(printf '%s' "$prs" | jq -r --arg b "$branch" \
		'[.[] | select(.headRefName == $b)] | sort_by(.number) | last // empty' 2>/dev/null)

	if [ -n "$pr" ]; then
		state=$(printf '%s' "$pr" | jq -r '.state')
		num=$(printf '%s' "$pr" | jq -r '.number')
		case "$state" in
		OPEN) continue ;;  # claimed, draft or ready; nothing to say
		*)
			echo "lane-claim-check: LEFTOVER. $branch's #$num is $state; delete the branch"
			continue
			;;
		esac
	fi

	born=$(printf '%s' "$activity" | jq -r --arg b "refs/heads/$branch" '
		[ .[] | select(.ref == $b) | select(.activity_type == "branch_creation")
		  | .timestamp | fromdateiso8601 ] | min // empty' 2>/dev/null)

	if [ -n "$born" ]; then
		age=$(( $(date -u +%s) - born ))
		if [ "$age" -lt $((GRACE_MINUTES * 60)) ]; then
			continue  # the legitimate window between a push and a create
		fi
		mins=$((age / 60))
	else
		mins="?"  # older than the activity page; see BUGS
	fi

	echo "lane-claim-check: UNCLAIMED. $branch has no pull request after $mins minutes." \
		"AGENTS.md §90: gh pr create --draft"
done
