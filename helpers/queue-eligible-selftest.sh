#!/bin/sh
#
# helpers/queue-eligible-selftest.sh: the merge queue's admission predicate, checked against
# fixtures, and its two consumers checked for still using it.
#
# Written by the 2026-09-24 security audit for the finding that `merge-drain.sh` armed auto-merge on
# a pull request from any fork (see helpers/queue-eligible.jq for the path). This is the test that
# fails on the tree before that fix and passes after it: with the predicate absent, or with a
# consumer that stopped splicing it, or with a `--json` list that dropped `isCrossRepository`, one
# of the three checks below goes red. script/lint runs it, so the property is a gate rather than a
# habit (AGENTS.md's ladder, rung two).
#
# Needs jq, which every consumer of the predicate needs too.

set -e
here="$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)"
me="$(basename "$0")"

if ! command -v jq >/dev/null 2>&1; then
	echo "$me: jq is not installed, and the merge drain cannot run without it either." >&2
	exit 1
fi

# 1. The predicate. Four pull requests, one eligible: a same-repository, non-draft head against
#    main. The other three are each refused for exactly one reason.
fixture='[
  {"number": 1, "isDraft": false, "baseRefName": "main",    "isCrossRepository": false},
  {"number": 2, "isDraft": false, "baseRefName": "main",    "isCrossRepository": true},
  {"number": 3, "isDraft": true,  "baseRefName": "main",    "isCrossRepository": false},
  {"number": 4, "isDraft": false, "baseRefName": "feature", "isCrossRepository": false}
]'
got=$(printf '%s' "$fixture" | jq -c "$(cat "$here/queue-eligible.jq")"'[ .[] | eligible | .number ]')
if [ "$got" != "[1]" ]; then
	echo "$me: the admission predicate admitted the wrong set: expected [1], got $got" >&2
	exit 1
fi

# 2. A record missing the field is refused, not admitted. This is what makes a consumer that forgot
#    to ask `gh` for `isCrossRepository` fail closed rather than open.
got=$(printf '[{"number": 5, "isDraft": false, "baseRefName": "main"}]' |
	jq -c "$(cat "$here/queue-eligible.jq")"'[ .[] | eligible | .number ]')
if [ "$got" != "[]" ]; then
	echo "$me: a record with no isCrossRepository field was admitted: got $got" >&2
	exit 1
fi

# 3. Both consumers splice the predicate and ask gh for the fields it reads.
for consumer in merge-drain.sh queue-hold.sh; do
	f="$here/$consumer"
	if ! grep -q 'queue-eligible.jq' "$f"; then
		echo "$me: $consumer no longer uses helpers/queue-eligible.jq; its own copy of the predicate is how a fork got armed." >&2
		exit 1
	fi
	if ! grep -q -- '--json [^ ]*isCrossRepository' "$f"; then
		echo "$me: $consumer does not ask gh pr list for isCrossRepository, so the predicate refuses everything." >&2
		exit 1
	fi
done

echo "queue eligibility: same-repository, non-draft, against main; both consumers splice it"
