#!/bin/sh
# Run one delegated-review trial: one model, one diff, one posture.
#
#     scripts/review-trial.sh <model> <diff-file> <posture> <out-file>
#
# **Provisional name**, the harness for milestone 521 (does an AI review of a pull request catch anything the gates and the maintainer do not). It exists so that the prompt a reviewer saw is a
# file anyone can read rather than a claim in a report: the whole experiment turns on the reviewer
# not having been told what to look for, and that is only checkable if the prompt is on disk.
#
# The prompt is assembled from the diff bundle and nothing else. This script has no table of
# defects, no ground truth, and no branch on which diff it is handling, so it cannot leak an answer
# it does not hold. The two postures differ by one paragraph, quoted below in full.
#
# # BUGS
#
# - **A reasoning model spends the budget on thinking**, and LiteLLM returns that as a `thinking`
#   block rather than a `text` one. `REVIEW_MAX_TOKENS` is 16000 for every trial for that reason,
#   and it is one number for all models so the conditions do not differ by model. If a run still
#   returns no text the transcript says so at the top rather than looking like an empty review.
# - **It does not retry.** A gateway timeout is a lost trial and shows up as a missing output file;
#   scripts/review-matrix.sh retries, and a cell that is still missing after that is reported as
#   missing rather than scored as silence.
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
body = json.dumps({"model": model, "max_tokens": int(os.environ.get("REVIEW_MAX_TOKENS", "16000")),
                   "messages": [{"role": "user", "content": os.environ["REVIEW_PROMPT"]}]}).encode()
req = urllib.request.Request(os.environ["REVIEW_BASE_URL"].rstrip("/") + "/v1/messages",
                             data=body, headers={"content-type": "application/json",
                                                 "x-api-key": "unused-the-gateway-has-no-password"})
with urllib.request.urlopen(req, timeout=1200) as r:
    d = json.load(r)
text = "".join(b.get("text", "") for b in d.get("content", []) if b.get("type") == "text")
if not text.strip():
    text = "[no text block returned; the model spent its budget on thinking]\n\n" + \
        "".join(b.get("thinking", "") for b in d.get("content", []))
with open(out, "w") as f:
    f.write(text)
print(f"{model}: {d.get('usage')}", file=sys.stderr)
PY
