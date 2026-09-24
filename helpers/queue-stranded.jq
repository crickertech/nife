# helpers/queue-stranded.jq: which pull requests the merge queue has left stranded, in one place.
#
# On 2026-09-24 #1202, #1200 and #1207 each sat armed (`autoMergeRequest` set), `mergeStateStatus`
# CLEAN, every required check green, and never entered `mergeQueue.entries`; each went in only when
# somebody called the `enqueuePullRequest` mutation by hand. Auto-merge is GitHub's promise to
# enqueue when the checks go green, and that promise is not always kept; this is the drain keeping
# it instead, and this file is the whole of the decision.
#
# `stranded($queued; $cutoff)` admits a pull request that:
#   - passes `eligible` (helpers/queue-eligible.jq, spliced ahead of this file by every consumer),
#     so a head in another repository can never be enqueued by this path, whatever else is true;
#   - is armed (`autoMergeRequest` present) and CLEAN (`mergeStateStatus`), which is the state in
#     which the platform should already have enqueued it;
#   - is not in `$queued`, the numbers `mergeQueue.entries` reports now;
#   - has been in that state since before `$cutoff` (epoch seconds): the later of when auto-merge
#     was armed and when its last check finished. **That is "two consecutive passes" in the only
#     currency a scheduled run has.** Each `merge-drain.yml` run is one `--once` pass in a fresh
#     process, five minutes apart, so "seen stranded on two passes in a row" is "stranded for at
#     least one interval"; the consumer sets `$cutoff` to now minus that interval, and a pull
#     request whose checks finished thirty seconds ago is given the platform its turn first.
#
# The unfinished-check sentinel: `gh pr list --json statusCheckRollup` reports a check that has not
# completed with `completedAt` of `0001-01-01T00:00:00Z`, and `fromdateiso8601` refuses year 1
# ("invalid gmtime representation", found by the self-test), so it is dropped before parsing; a
# CLEAN pull request has no such check anyway, and a status context (no `completedAt`)
# contributes its `createdAt`.
#
# helpers/queue-stranded-selftest.sh checks this against fixtures, including the one that matters
# most: an armed, CLEAN, old, unqueued pull request from a fork is refused, because `eligible`
# runs first. Consumers splice this file after queue-eligible.jq:
#     jq -r "$(cat helpers/queue-eligible.jq)$(cat helpers/queue-stranded.jq)" '...'
def stranded($queued; $cutoff):
  eligible
  | select(.autoMergeRequest != null)
  | select(.mergeStateStatus == "CLEAN")
  | .number as $n
  | select(($queued | index($n)) == null)
  | select(
      ([ (.autoMergeRequest.enabledAt // empty),
         ((.statusCheckRollup // [])[] | (.completedAt // .createdAt // empty)) ]
       | map(select(startswith("0001") | not) | fromdateiso8601) | max) < $cutoff);
