#!/bin/sh
#
# helpers/queue-stranded-selftest.sh: the merge drain's "enqueue what the platform left behind"
# predicate, checked against fixtures, and its consumer checked for still using it.
#
# The property that matters is the third fixture: **the enqueue path cannot lose the admission
# predicate.** `stranded` is defined on top of `eligible`, so a fork's pull request that is armed,
# CLEAN, old and unqueued is refused here for the same reason it is never armed
# (helpers/queue-eligible.jq). If someone rewrites `stranded` without `eligible` in front of it,
# this test goes red, and so does the drain's own composition check below. script/lint runs it.

set -e
here="$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)"
me="$(basename "$0")"

if ! command -v jq >/dev/null 2>&1; then
	echo "$me: jq is not installed, and the merge drain cannot run without it either." >&2
	exit 1
fi

program="$(cat "$here/queue-eligible.jq")$(cat "$here/queue-stranded.jq")"
# The cutoff is 12:00, which is "now minus one pass interval" from the consumer's point of view,
# so "old" means the last activity was before 12:00 and "fresh" means it was at 12:04. #4 is in the queue already. #5 is not CLEAN. #6 is not armed. #7 is
# a fork. #8 has a check that has not completed (the sentinel gh prints) and is otherwise stranded,
# which a CLEAN pull request cannot really have, so it must still be admitted by the time test.
fixture='[
  {"number": 1, "isDraft": false, "baseRefName": "main", "isCrossRepository": false, "mergeStateStatus": "CLEAN",
   "autoMergeRequest": {"enabledAt": "2026-09-24T11:50:00Z"},
   "statusCheckRollup": [{"completedAt": "2026-09-24T11:55:00Z"}, {"createdAt": "2026-09-24T11:52:00Z"}]},
  {"number": 2, "isDraft": false, "baseRefName": "main", "isCrossRepository": false, "mergeStateStatus": "CLEAN",
   "autoMergeRequest": {"enabledAt": "2026-09-24T11:50:00Z"},
   "statusCheckRollup": [{"completedAt": "2026-09-24T12:04:00Z"}]},
  {"number": 3, "isDraft": false, "baseRefName": "main", "isCrossRepository": false, "mergeStateStatus": "CLEAN",
   "autoMergeRequest": {"enabledAt": "2026-09-24T12:04:00Z"},
   "statusCheckRollup": [{"completedAt": "2026-09-24T11:55:00Z"}]},
  {"number": 4, "isDraft": false, "baseRefName": "main", "isCrossRepository": false, "mergeStateStatus": "CLEAN",
   "autoMergeRequest": {"enabledAt": "2026-09-24T11:50:00Z"}, "statusCheckRollup": []},
  {"number": 5, "isDraft": false, "baseRefName": "main", "isCrossRepository": false, "mergeStateStatus": "BLOCKED",
   "autoMergeRequest": {"enabledAt": "2026-09-24T11:50:00Z"}, "statusCheckRollup": []},
  {"number": 6, "isDraft": false, "baseRefName": "main", "isCrossRepository": false, "mergeStateStatus": "CLEAN",
   "autoMergeRequest": null, "statusCheckRollup": []},
  {"number": 7, "isDraft": false, "baseRefName": "main", "isCrossRepository": true, "mergeStateStatus": "CLEAN",
   "autoMergeRequest": {"enabledAt": "2026-09-24T11:50:00Z"}, "statusCheckRollup": []},
  {"number": 8, "isDraft": false, "baseRefName": "main", "isCrossRepository": false, "mergeStateStatus": "CLEAN",
   "autoMergeRequest": {"enabledAt": "2026-09-24T11:50:00Z"},
   "statusCheckRollup": [{"completedAt": "0001-01-01T00:00:00Z"}, {"completedAt": "2026-09-24T11:55:00Z"}]}
]'
cutoff=$(printf '%s' '"2026-09-24T12:00:00Z"' | jq 'fromdateiso8601')
got=$(printf '%s' "$fixture" | jq -c --argjson queued '[4]' --argjson cutoff "$cutoff" \
	"$program"'[ .[] | stranded($queued; $cutoff) | .number ]')
if [ "$got" != "[1,8]" ]; then
	echo "$me: the stranded predicate admitted the wrong set: expected [1,8], got $got" >&2
	echo "  (2 and 3 are fresh, 4 is queued, 5 is not CLEAN, 6 is not armed, 7 is a fork)" >&2
	exit 1
fi

# The property named at the top, asked directly: a fork that is stranded by every other measure.
got=$(printf '%s' "$fixture" | jq -c --argjson queued '[]' --argjson cutoff "$cutoff" \
	"$program"'[ .[] | select(.number == 7) | stranded($queued; $cutoff) | .number ]')
if [ "$got" != "[]" ]; then
	echo "$me: the enqueue predicate admitted a pull request from a fork; it has lost the admission predicate" >&2
	exit 1
fi

# The consumer splices both files, eligible first, and asks gh for every field the predicate reads.
f="$here/merge-drain.sh"
if ! grep -qF '"$(cat "$ELIGIBLE_JQ")$(cat "$STRANDED_JQ")"' "$f"; then
	echo "$me: merge-drain.sh does not splice queue-eligible.jq ahead of queue-stranded.jq, so the enqueue path has no admission predicate." >&2
	exit 1
fi
for field in autoMergeRequest statusCheckRollup mergeStateStatus isCrossRepository; do
	if ! grep -q -- "--json [^ ]*$field" "$f"; then
		echo "$me: merge-drain.sh does not ask gh pr list for $field, which the stranded predicate reads." >&2
		exit 1
	fi
done
if grep -q 'enqueuePullRequest(input:{[^}]*jump' "$f"; then
	echo "$me: merge-drain.sh enqueues with jump, which puts a pull request ahead of the queue; that is not this script's call." >&2
	exit 1
fi

echo "queue stranded: armed, CLEAN, unqueued, older than the cutoff, and only through eligible; the drain splices both"
