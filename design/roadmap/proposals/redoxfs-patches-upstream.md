# Offer the two RedoxFS patches upstream

**Status: PROPOSED 2026-09-03.** Written by the milestone 247 sweep, from milestone 32's block.

**Gate: DECISION, HARDWARE.** DECISION because a merge request is a fact that leaves the machine
under this project's name, which AGENTS.md puts in the irreversible column. HARDWARE in its second
sense, the one the roadmap README names: no board is involved, but a person with an account on
gitlab.redox-os.org has to fork, push and open the request, and no lane can do that.

**In brief.** Two patches against RedoxFS are written, applied and documented, and neither has been
offered upstream. `patches/redoxfs-no-std-vec-import.patch` fixes the `no_std` build (four E0425
sites across `filesystem.rs` and `record.rs`) and adds a `--no-default-features` CI job so the
configuration cannot rot again. `patches/redoxfs-no-std-create-uuid.patch` lets a `no_std` caller
create a filesystem by supplying the disk id, the same way `create` already takes `ctime`.
`patches/README.md` names the submission route for both: fork on gitlab.redox-os.org, `git am` the
file on a branch, push, open the merge request.

## Why this matters

`patches/README.md` opens by saying what the directory is for: *"Each exists to be upstreamed; an
entry leaves this directory when the pin that needed it advances past a release containing the
fix."* Neither entry can ever leave, because nobody has asked. The directory is currently a
permanent home for two patches whose whole design was to be temporary.

The concrete cost is per-bump and recurring. Milestone 203 built the machinery that reports when
upstream moves, so this tree will learn about a new RedoxFS release; each one then re-applies both
divergences by hand and re-checks that they still apply cleanly. `redoxfs-no-std-create-uuid.patch`
is written against the published 0.9.1 rather than master specifically so it applies with zero fuzz
to the pin, and its own README entry says rebasing onto master is the submitter's first step. That
rebase gets more expensive the longer nobody does it.

There is a second cost that is not ours. The `no_std` build of RedoxFS is broken upstream for
everyone, and this tree has the fix sitting in a file.

## What it would take

The mechanical part is small: an account, a fork, `git am`, a push, a merge request per patch. The
`create-uuid` patch wants a rebase onto master first. After that the work is waiting, and answering
whatever upstream asks, which is the part with no schedule.

## Where it came from

Milestone 32's block: *"Offer the two RedoxFS patches upstream.
`patches/redoxfs-no-std-vec-import.patch` and `patches/redoxfs-no-std-create-uuid.patch` are
written and `patches/README.md` names the route,
but no merge request exists on gitlab.redox-os.org. So the pin carries divergences that could have
stopped existing, and every future bump re-applies them by hand."*
