#!/bin/sh
# Build the diff bundle a delegated reviewer sees: the commit, then every changed file's pre-image.
#
#     scripts/review-bundle.sh <worktree> <commit> > bundle.txt
#
# **Provisional name** (milestone 521's harness). This is the half of the experiment that enforces
# blindness, so it is deliberately dumb: everything it emits comes out of `git show` and `git cat-file`
# in the worktree named on the command line. It knows nothing about which commits are defective, and
# takes no argument that could tell it.
#
# The pre-image matters. A record whose defect is a *removed* true sentence is invisible in a diff
# read alone unless the reviewer can see what the paragraph used to say in place, so withholding the
# pre-image would make arm 1 unfair rather than blind.
#
# # BUGS
#
# - **It caps each pre-image at 1200 lines**, so a reviewer given a very large changed file sees a
#   truncated one and the truncation is announced in the bundle. No file in milestone 521's corpus
#   hit the cap; a later corpus might.
set -eu

[ $# -eq 2 ] || { echo >&2 "usage: $0 <worktree> <commit>"; exit 2; }
w=$1; c=$2

cd "$w"

echo "=== THE COMMIT ==="
echo
git show --find-renames "$c"
echo
echo "=== THE CHANGED FILES AS THEY STOOD BEFORE THE COMMIT ==="

git show --find-renames --name-status --format= "$c" | while read -r status paths; do
    case "$status" in
        R*) old=$(printf '%s' "$paths" | cut -f1) ;;
        *)  old=$(printf '%s' "$paths" | cut -f1) ;;
    esac
    [ -n "$old" ] || continue
    git cat-file -e "$c^:$old" 2>/dev/null || continue
    echo
    echo "--- $old (before) ---"
    git show "$c^:$old" | awk 'NR<=1200; NR==1201 { print "[... truncated at 1200 lines by scripts/review-bundle.sh ...]"; exit }'
done
