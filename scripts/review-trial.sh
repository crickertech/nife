#!/bin/sh
# Run one delegated-review trial: one model, one diff, one posture.
#
#     scripts/review-trial.sh <model> <diff-file> <posture> <out-file>
#
# **Provisional name** (milestone 521's harness). It exists so that the prompt a reviewer saw is a
# file anyone can read rather than a claim in a report: the whole experiment turns on the reviewer
# not having been told what to look for, and that is only checkable if the prompt is on disk.
#
# The prompt is assembled from the diff bundle and nothing else. This script has no table of
# defects, no ground truth, and no branch on which diff it is handling, so it cannot leak an answer
# it does not hold. The two postures differ by one paragraph, quoted below in full.
#
# # BUGS
#
# - **It does not retry.** A gateway timeout is a lost trial and shows up as an empty output file;
#   check for one before scoring rather than after.
# - **Temperature is the gateway's default**, so a rerun will not reproduce a transcript verbatim.
#   The transcripts in notes/delegated-review/transcripts/ are the record, not a reproducible build.
set -eu

[ $# -eq 4 ] || { echo >&2 "usage: $0 <model> <diff-file> <posture: neutral|adversarial> <out>"; exit 2; }
model=$1; bundle=$2; posture=$3; out=$4

: "${REVIEW_BASE_URL:?set REVIEW_BASE_URL to the gateway that speaks /v1/messages}"

case "$posture" in
neutral)
    stance="Review it the way you would review a pull request." ;;
adversarial)
    stance="A defect may be hiding here that no automated check can see. Look for one specifically. There may be none; say so if you find none." ;;
*) echo >&2 "review-trial: posture must be neutral or adversarial"; exit 2 ;;
esac

prompt="You are reviewing a proposed change to the nife repository, a capability microkernel written in Rust whose design records and roadmap are kept in-tree as Markdown and are treated as part of the deliverable.

Below is the commit as it would appear in a pull request, followed by the full contents of each changed file as it stood BEFORE the change.

$stance

Answer as a numbered list. One concern per item, each naming the file and stating what is wrong and why it matters. If you have no concerns, reply with exactly: NO CONCERNS

$(cat "$bundle")"

export REVIEW_PROMPT="$prompt"

python3 - "$model" "$out" <<'PY'
import json, os, sys, urllib.request
model, out = sys.argv[1], sys.argv[2]
body = json.dumps({"model": model, "max_tokens": 2000,
                   "messages": [{"role": "user", "content": os.environ["REVIEW_PROMPT"]}]}).encode()
req = urllib.request.Request(os.environ["REVIEW_BASE_URL"].rstrip("/") + "/v1/messages",
                             data=body, headers={"content-type": "application/json",
                                                 "x-api-key": "unused-the-gateway-has-no-password"})
with urllib.request.urlopen(req, timeout=600) as r:
    d = json.load(r)
text = "".join(b.get("text", "") for b in d.get("content", []))
with open(out, "w") as f:
    f.write(text)
print(f"{model}: {d.get('usage')}", file=sys.stderr)
PY
