#!/bin/sh
#
# Drain the merge queue: enqueue every pull request that does not need calef.
#
#     scripts/merge-drain.sh              # run until nothing is left to enqueue
#     scripts/merge-drain.sh --once       # one pass, then exit (for a cron or a check)
#
# PROVISIONAL NAME. Minted 2026-08-04; not put to calef. See the `Name:` block below.
#
# # Why this exists
#
# The maintainer holds merge authority, and merging is the one duty it is structurally worst at:
# when it is busy it is busy, and merging happens between conversations rather than during them. On
# 2026-08-04 two green pull requests sat unmerged for hours because nobody armed auto-merge, and the
# steward, which exists precisely to compensate for the maintainer being busy, only ever *reported*
# a stalled queue and never acted on one.
#
# # What this used to do, and why almost all of it is gone (2026-08-16)
#
# This script used to decide the merge ORDER: at most one pull request in flight, in flight before
# current before oldest, and one "Update branch" click per pass. That brain took four shapes and
# three of them starved something (the history is in notes/merge-queue.md, kept because it is
# evidence about the up-to-date rule rather than about this script).
#
# **GitHub's merge queue is now enabled on this repository, and it is that brain, one rung up.** It
# serializes candidates, tests each against the tip, and rejects what fails, which is exactly what
# the ordering logic was reconstructing from outside. Three consequences, all load-bearing:
#
#   - **Ordering is not ours any more.** Enqueue everything eligible; the queue decides.
#   - **Updating a branch is neither needed nor allowed.** The queue builds the merge candidate
#     itself, and GitHub answers `update-branch` on a queued pull request with a 422.
#   - **"Arm exactly one" is now the wrong answer**, not merely a redundant one: it leaves ready work
#     idle for a cycle when enqueueing costs nothing and the queue would have ordered it.
#
# # What is left, and why it is not nothing
#
# Two duties survive, and neither is something the platform knows:
#
#   - **The admission policy.** The queue merges what is enqueued; something has to decide what gets
#     enqueued. Drafts are not asking to be merged, and `needs-architect` means the work is outside
#     standing merge authority: it touches the syscall surface, adds a dependency, or owes a
#     `DECISIONS` section. CLAUDE.md describes the label and the `## What I need from you` comment
#     that goes with it.
#   - **Saying what stalled.** A queue never resolves a conflict (two were resolved by hand on
#     2026-08-16), and a pull request whose checks fail is ejected rather than fixed. Both need a
#     person, so both are reported and neither is retried.
#
# A proposal to move the first duty onto a required check, which would make this script smaller
# still, is in design/decisions/ (`needs-architect` as a check rather than as a script's restraint).
#
# Name: unrecorded. Provisional, minted 2026-08-04 and not yet put to calef. Named for what it does
# to the queue rather than for the mechanism, in the family of `qemu-bounded.sh`. It lives in
# `scripts/` rather than `script/` because it is a maintainer's tool and not a front door a
# contributor types; `script/` is the normalised "Scripts to Rule Them All" set (notes/scripts.md).
# See notes/merge-queue.md.

set -e
cd "$(dirname "$0")/.."

REPO="crickertech/nife"
HELD_LABEL="needs-architect"
once=""
[ "$1" = "--once" ] && once=1

# A watcher must run from the MAIN checkout, never from a lane worktree, and this refuses rather
# than trusting anyone to remember. Measured cause, twice: `/bin/sh` reads a script LAZILY, so
# deleting the file under a running shell can kill it mid-loop. The merge drain died that way on
# 2026-08-18 when the worktree it was launched from was pruned, and `trunk-health.sh` died the same
# way later the same day during a 24-worktree cleanup, silently, while `main` was red on the
# fastpath gate for hours. The drain survived that second sweep only because it happened to have
# been relaunched with an absolute path into the main checkout.
#
# Only the watching form is refused. `--once` is a check anybody may run anywhere, including a lane
# gating its own work, and it exits long before a prune could reach it.
#
# `--git-dir` resolves to `.git/worktrees/<name>` in a linked worktree and to `.git` in the main
# checkout, which is the cheapest true test available; `--git-common-dir` points at the shared
# `.git` from both and cannot tell them apart.
if [ -z "$once" ] && [ "$(git rev-parse --git-dir 2>/dev/null)" != ".git" ]; then
	echo "$(basename "$0"): refusing to watch from a lane worktree." >&2
	echo "  A watcher outlives the lane that started it, and pruning that lane's worktree kills" >&2
	echo "  it silently, because /bin/sh reads a script lazily. Run it from the main checkout:" >&2
	echo "    cd <main checkout> && scripts/$(basename "$0") &" >&2
	echo "  ('--once' is fine from anywhere; only the watching form is refused.)" >&2
	exit 2
fi


# The unheld queue, lowest number first. Drafts are excluded: a draft is not asking to be merged.
queue() {
	gh pr list --repo "$REPO" --state open \
		--json number,mergeStateStatus,labels,isDraft,title,body,headRefName 2>/dev/null |
		jq -r --arg L "$HELD_LABEL" '
			[ .[]
			  | select(.isDraft == false)
			  | select((.labels | map(.name) | index($L)) | not) ]
			| sort_by(.number)' 2>/dev/null || echo '[]'
}

# **A report only this script's own stdout can see is not a report calef will find in time.**
#
# # Why this exists
#
# On 2026-08-26 three armed pull requests (#530, #531, #532) sat "3 armed, 0 stalled" for hours
# while the queue drained nothing, because each carried a GitHub Actions run stuck `queued` with
# no job ever started. Once that stall shape was named (`stuck_checks` below), calef asked the
# obvious next question: could the script itself tell him, instead of a log file on patagonia that
# nothing prompts anyone to open. It can: `gh pr comment` is one API call, same shape as
# `gh pr merge --auto`.
#
# **The one real risk is spam, not correctness.** A stall that persists gets re-detected every
# five-minute pass, and posting a fresh comment every pass would bury the one useful comment under
# duplicates within the hour. So `notify` checks the pull request's own comments for a marker
# (an HTML comment, invisible when rendered) before posting, and posts once per marker per pull
# request, ever, not once per stall *episode*. A stall that clears and recurs later does not get a
# second comment. That is a real, accepted limitation rather than a solved problem: closing it needs
# either a timestamp-based cooldown or deleting the marker comment when a stall clears, and neither
# was worth building for a first cut. The five-minute log line still fires every pass regardless;
# only the PR comment is deduplicated.
notify() {
	num="$1"
	marker="$2"
	message="$3"
	existing=$(gh pr view "$num" --repo "$REPO" --json comments 2>/dev/null |
		jq -r --arg m "$marker" '[.comments[] | select(.body | contains($m))] | length' 2>/dev/null)
	if [ "${existing:-0}" = "0" ]; then
		gh pr comment "$num" --repo "$REPO" --body "$message

<!-- $marker -->" >/dev/null 2>&1
	fi
}

# A workflow run stuck at `queued`, no job ever started, no conclusion: the third stall shape, and
# neither DIRTY nor a FAILURE conclusion catches it, because both read false while a run sits in
# this state. `gh pr merge --auto` on a pull request in this state is not wrong, only useless: it
# re-arms a check that was never going to move, silently, forever.
#
# The threshold is generous for the same reason `STALE_DRAFT_MINUTES` is: this repository's own
# check suite (Kani proofs, fuzz targets, a full three-architecture boot) legitimately takes
# `in_progress` a long time. `queued` with zero jobs started for this long is a different thing:
# GitHub Actions ordinarily assigns a runner within seconds to low minutes, not tens of minutes.
STUCK_CHECK_MINUTES=${STUCK_CHECK_MINUTES:-20}

stuck_checks() {
	num="$1"
	head="$2"
	gh run list --repo "$REPO" --branch "$head" --json status,conclusion,createdAt --limit 5 2>/dev/null |
		jq -r --argjson mins "$STUCK_CHECK_MINUTES" --arg n "$num" '
			(now - ($mins * 60)) as $cut
			| .[]
			| select(.status == "queued")
			| select((.createdAt | fromdateiso8601) < $cut)
			| "merge-drain: STALLED. #\($n) has a workflow run stuck queued for over " +
			  "\($mins) minutes with no job ever starting (GitHub infra, not this pull " +
			  "request). Push an empty commit to retrigger, or check the Actions tab."
		' 2>/dev/null || true
}

# A draft that has stopped moving is probably a finished lane that forgot to mark it ready.
#
# # Why this exists
#
# On 2026-08-19 PR #348's lane finished, reported, and left its pull request a draft. The drain
# excludes drafts **by design** ("a draft is not asking to be merged"), so it sat unmergeable for
# hours while every observer saw exactly what a healthy working lane looks like. It was found by
# calef asking why two pull requests were drafts, not by anything in this system.
#
# **That is the recurring shape rather than a one-off**: a state whose silence is indistinguishable
# from healthy operation. The same day, a watcher died and nothing said so while `main` was red, and
# a CI gate went red on a check that could not block a merge. In all three the observer was missing,
# not the signal.
#
# **Rung four is "tell lanes to mark ready", and that is what failed.** The mechanism has to be
# something that notices, so this reports a draft whose branch has stopped receiving commits. A live
# lane commits as it works, per AGENTS.md ("commit whenever a piece works and push whenever a commit
# exists"); a finished one goes quiet. Quiet for longer than a full gate takes is the signal.
#
# It **reports and does not act**. Marking somebody else's draft ready would be a judgement about
# whether their work is done, which is exactly the thing the draft is claiming. This says the words a
# person needs and leaves the decision.
#
# The threshold is generous on purpose. `script/test` runs both ISAs and a slow leg is tens of
# minutes, so anything tighter would fire on lanes that are working and teach everyone to ignore it.
STALE_DRAFT_MINUTES=${STALE_DRAFT_MINUTES:-75}

stale_drafts() {
	gh pr list --repo "$REPO" --state open --json number,isDraft,title,commits 2>/dev/null |
		jq -r --argjson mins "$STALE_DRAFT_MINUTES" '
			(now - ($mins * 60)) as $cut
			| .[]
			| select(.isDraft == true)
			| select((.commits | length) > 0)
			| select((.commits[-1].committedDate | fromdateiso8601) < $cut)
			| [.number, ("merge-drain: STALE DRAFT. #\(.number) has not committed in over " +
			  "\($mins) minutes (\(.title[0:60])). If its lane is finished: gh pr ready \(.number)")]
			| @tsv
		' 2>/dev/null |
		while IFS="$(printf '\t')" read -r num msg; do
			[ -z "$num" ] && continue
			echo "$msg"
			notify "$num" "merge-drain:stale-draft" "$msg"
		done
}

# `Blocked-by: #N` in a pull request body: a SELF-RELEASING hold for a mechanical ordering
# constraint, as opposed to `needs-architect`, which means a person must decide something.
#
# # Why this exists, and why a plain "held" label was refused
#
# On 2026-08-18 #329 and #324 each carried a file named `97-*.md` in `design/decisions/`. Both were
# green alone; a merge-queue group containing both fails the decisions gate, because two sections
# cannot share a number. #329 was evicted as UNMERGEABLE while reporting CLEAN on its own page, and
# the only lever available to keep the drain from re-arming it was `needs-architect`, which says a
# person must rule on something. Using it here would have put a false entry on calef's queue, which
# is the one queue in this project that must not accumulate noise.
#
# That is the same shape as #274, which was enqueued and evicted **29 times, 26 of them in a
# 3.5-hour loop**: #271 landed a doctest calling a method whose arity #274 was changing, git merged
# both without a conflict marker, and the pair was red only together. Green alone, green alone, red
# together is not a state any per-branch check can see.
#
# **A generic hold label was considered and refused**, and the reason is the failure mode rather
# than tidiness: a manual label has to be REMOVED by whoever remembers, and this project has a
# recorded history of exactly that going wrong. `needs-architect` was left on #320 and on #329 after
# both had been answered, on the same day, and calef found both. A hold that outlives its reason is
# a false blocker, and a false blocker is worse than none because it is believed.
#
# So the hold names its own release condition and evaporates without anybody acting: when #N merges,
# the next pass arms this pull request. Nothing to remember, and `gh pr list` shows the reason.
#
# The blocker being CLOSED rather than merged is reported loudly instead of silently released,
# because that is an anomaly: it means the thing this was sequenced behind is not coming.
blocked_by() {
	# The first `Blocked-by: #N` in the body. Case-insensitive on the key, because a person typing
	# it in a pull request body will not match a regex's idea of capitalisation.
	printf '%s' "$1" | sed -n 's/.*[Bb]locked-by:[[:space:]]*#\([0-9][0-9]*\).*/\1/p' | head -1
}

# Arming is one API call and changes nothing until the checks pass, so every eligible pull request
# is armed on every pass. Under the merge queue that is the whole job: an armed pull request enters
# the queue when its checks go green, and the queue lands them one at a time against the tip.
pass() {
	q=$(queue)
	n=$(printf '%s' "$q" | jq -r 'length' 2>/dev/null || echo 0)
	if [ "$n" = "0" ] || [ -z "$n" ]; then
		echo "merge-drain: queue empty; nothing open that does not need calef"
		return 1
	fi

	armed=0
	stalled=0
	attempted=""
	for num in $(printf '%s' "$q" | jq -r '.[].number'); do
		state=$(printf '%s' "$q" | jq -r --arg n "$num" '.[] | select(.number == ($n | tonumber)) | .mergeStateStatus')
		title=$(printf '%s' "$q" | jq -r --arg n "$num" '.[] | select(.number == ($n | tonumber)) | .title')

		# A declared ordering constraint, checked before anything else, because arming a pull
		# request that is sequenced behind another wastes a group build and can evict it.
		body=$(printf '%s' "$q" | jq -r --arg n "$num" '.[] | select(.number == ($n | tonumber)) | .body')
		blocker=$(blocked_by "$body")
		if [ -n "$blocker" ]; then
			bstate=$(gh pr view "$blocker" --repo "$REPO" --json state -q .state 2>/dev/null)
			case "$bstate" in
			MERGED) ;;  # released, and nobody had to do anything
			CLOSED)
				msg="merge-drain: STALLED. #$num is blocked by #$blocker, which was CLOSED without merging ($title)"
				echo "$msg"
				notify "$num" "merge-drain:blocker-closed" "$msg"
				stalled=$((stalled + 1))
				continue
				;;
			*)
				echo "merge-drain: holding #$num until #$blocker merges ($title)"
				continue
				;;
			esac
		fi

		# A conflict is the one state that cannot be waited out: the queue will not resolve it and
		# neither will another pass. Say which pull request it is and move on to the rest, because
		# one conflict must not stop the others being armed.
		if [ "$state" = "DIRTY" ]; then
			msg="merge-drain: STALLED. #$num has conflicts a person must resolve ($title)"
			echo "$msg"
			notify "$num" "merge-drain:conflict" "$msg"
			stalled=$((stalled + 1))
			continue
		fi

		# A failing check is the other. `--auto` on a failing pull request is harmless but says
		# nothing, so the failure is named instead: the queue ejects what fails, and nothing here
		# should retry it and burn CI.
		failed=$(gh pr view "$num" --repo "$REPO" --json statusCheckRollup \
			-q '[.statusCheckRollup[] | select(.conclusion == "FAILURE") | .name] | join(", ")' 2>/dev/null)
		if [ -n "$failed" ]; then
			msg="merge-drain: STALLED. #$num is failing $failed ($title)"
			echo "$msg"
			notify "$num" "merge-drain:check-failure" "$msg"
			stalled=$((stalled + 1))
			continue
		fi

		# A third stall shape: neither DIRTY nor FAILURE, a run just never started. See
		# `stuck_checks`'s own comment for why this needs a person rather than a retry.
		head=$(printf '%s' "$q" | jq -r --arg n "$num" '.[] | select(.number == ($n | tonumber)) | .headRefName')
		stuck=$(stuck_checks "$num" "$head")
		if [ -n "$stuck" ]; then
			echo "$stuck"
			notify "$num" "merge-drain:stuck-check" "$stuck"
			stalled=$((stalled + 1))
			continue
		fi

		# NO `--delete-branch` HERE, and this is not a style preference. With a merge queue
		# enabled GitHub refuses the whole command with "Cannot use `-d` or `--delete-branch`
		# when merge queue enabled", so passing it enqueues NOTHING. The flag was also always
		# redundant: this repository sets `delete_branch_on_merge`, so the platform deletes the
		# head branch itself. `gh` prints "the merge strategy for main is set by the merge
		# queue" and enqueues anyway; that line is a notice, not a failure.
		#
		# The failure this cost: on 2026-08-17 the drain reported "9 armed" every pass for
		# three hours while the queue stayed empty and nothing merged, because the error went
		# to /dev/null and `|| true` swallowed the exit code. A count of ATTEMPTS was being
		# printed as a count of RESULTS.
		if ! gh pr merge "$num" --repo "$REPO" --auto --merge >/dev/null 2>&1; then
			msg="merge-drain: STALLED. #$num would not enqueue ($title)"
			echo "$msg"
			notify "$num" "merge-drain:would-not-enqueue" "$msg"
			stalled=$((stalled + 1))
			continue
		fi

		attempted="$attempted $num"
	done

	# **Verify, and know that "armed" has two shapes, because neither field alone covers both.**
	#
	#   - Checks still running: the pull request carries an `autoMergeRequest` and reports
	#     `BLOCKED`. It enters the queue by itself when the last check goes green.
	#   - Checks green: it is IN the queue, and it reports `mergeStateStatus: CLEAN` with a
	#     **null** `autoMergeRequest`, because arming became membership.
	#
	# So a queued pull request looks unarmed on the pull request object, which is the same trap
	# that produced the bug above one level along: the obvious field looks authoritative and is
	# not. `mergeQueue.entries` is the only thing that knows about the second shape, and it is
	# asked once per pass rather than once per pull request.
	queued=$(gh api graphql -f query='{repository(owner:"'"${REPO%/*}"'",name:"'"${REPO#*/}"'"){mergeQueue{entries(first:50){nodes{pullRequest{number}}}}}}' \
		--jq '.data.repository.mergeQueue.entries.nodes[].pullRequest.number' 2>/dev/null)
	for num in $attempted; do
		if printf '%s\n' "$queued" | grep -qx "$num"; then
			armed=$((armed + 1))
		elif [ "$(gh pr view "$num" --repo "$REPO" --json autoMergeRequest \
			-q '.autoMergeRequest != null' 2>/dev/null)" = "true" ]; then
			armed=$((armed + 1))
		else
			msg="merge-drain: STALLED. #$num took the call but is neither queued nor armed"
			echo "$msg"
			notify "$num" "merge-drain:not-armed" "$msg"
			stalled=$((stalled + 1))
		fi
	done

	echo "merge-drain: $armed armed, $stalled stalled, of $n unheld"
	stale_drafts

	# Nothing left to do on a pass where everything open is stalled: the remaining work needs a
	# person, and looping only re-prints the same lines.
	[ "$armed" -gt 0 ]
}

if [ -n "$once" ]; then
	pass || exit 0
	exit 0
fi

while pass; do
	sleep 150
done
