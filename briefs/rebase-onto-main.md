Rebase a branch onto `origin/main` and resolve its conflicts. You are in the branch's git worktree.
Do the work; do not ask questions.

    git fetch origin
    git rebase origin/main

## The three conflicts you will hit, and exactly how each is resolved

These are known. Do not improvise; resolve them this way.

**1. `design/roadmap/README.md` shows as `DU` (deleted by us).** That file was deliberately retired
from the repository. The resolution is to accept the deletion:

    git rm -f design/roadmap/README.md
    git rebase --continue

It may recur on several commits in a row. Each time, same answer.

**2. A file under `bench/` conflicts (`baseline-aarch64.txt`, `baseline-riscv64.txt`, or
`baseline-x86_64.txt`).** These are benchmark floors. **Take `origin/main`'s version, never this
branch's, and never hand-merge the numbers**: a baseline is a measurement of one specific binary, so
a merged file describes a binary nobody ever built.

    git checkout origin/main -- bench/
    git add bench/
    git rebase --continue

**3. `notes/benchmarks.md` conflicts.** Both sides are *additions* of different dated sections;
neither replaces the other. Keep **both**, one after the other, and delete only the `<<<<<<<`,
`=======` and `>>>>>>>` marker lines.

## If you hit a conflict that is none of those three

**Stop.** Run `git rebase --abort`, leave the branch exactly as you found it, and say in your final
message which file conflicted and what the two sides were. Do not guess. A wrong resolution here is
worse than no resolution, because it looks like housekeeping and ships silently.

## After the rebase completes

Run these and fix what they report:

    script/lint
    script/roadmap --check
    script/citations --ratchet

Two failures are common after a rebase and both are yours to fix:

- **A milestone number collision** (`milestone N is claimed by two files`). The file *this branch*
  introduced moves to a free number; the one already on `origin/main` keeps it. Pick the lowest free
  number **above 557**. Rename with `git mv`, change the H1's number to match, and add a sentence to
  the status paragraph saying it was renumbered on 2026-09-22 because a concurrently merged lane had
  taken the old number.
- **A compile error about a function taking more arguments than were supplied.** Some function grew
  a parameter on `main` while this branch sat. Find another call site of that function on `main` and
  pass what it passes.

`script/citations --ratchet` reads the **committed** state, so commit before running it.

Do **not** push. Do **not** force-push. Leave the branch rebased locally and stop.
