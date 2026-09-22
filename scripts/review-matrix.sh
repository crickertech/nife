#!/bin/sh
# Run milestone 521's whole trial matrix: every model against every bundle, both postures, N times.
#
#     REVIEW_BASE_URL=... scripts/review-matrix.sh <bundle-dir> <out-dir> [replicates]
#
# **Provisional name.** Replicates exist because a single call is not a measurement: a pilot run at
# a lower token cap got a substantive review out of the same model and bundle that later answered
# "NO CONCERNS", and one of those two would have been the reported result.
#
# # BUGS
#
# - **It retries a timeout twice and then gives up**, leaving that cell's file absent. Scoring must
#   treat an absent file as missing, not as a reviewer with nothing to say.
set -eu
[ $# -ge 2 ] || { echo >&2 "usage: $0 <bundle-dir> <out-dir> [replicates]"; exit 2; }
bundles=$1; out=$2; n=${3:-3}
here=$(cd "$(dirname "$0")" && pwd)
mkdir -p "$out"
for m in ${REVIEW_MODELS:-open-lane-qwen open-lane-kimi}; do
    for b in "$bundles"/*.txt; do
        d=$(basename "$b" .txt)
        for p in neutral adversarial; do
            i=1
            while [ "$i" -le "$n" ]; do
                f="$out/$m.$d.$p.r$i.md"
                if [ ! -s "$f" ]; then
                    ( for try in 1 2 3; do
                        "$here/review-trial.sh" "$m" "$b" "$p" "$f" 2>/dev/null && break
                        rm -f "$f"
                      done
                      [ -s "$f" ] || echo >&2 "review-matrix: MISSING $f" ) &
                fi
                i=$((i + 1))
            done
        done
    done
    wait
done
