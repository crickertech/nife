# A card written by any means but ours cannot be checked without booting it

**Status: PROPOSED 2026-09-03.** Written by the milestone 247 sweep, from milestone 217's block.

**Gate: NONE.** A lane can start today. Both artifacts are files on disk, the expected digest is the
one the kernel image was built against, and the check runs on a mounted directory without any
board.

**In brief.** A tool that reads a mounted card and reports whether the kernel on it and the archive
beside it come from the same build. Milestone 217 added a `--card` option that writes a matched
pair, which narrows who needs this rather than removing the need. A card written by `dd`, by a
graphical imager, by a colleague, or by an earlier version of this tree carries no guarantee at all,
and today the only way to find out is to power the board and read the refusal.

## Why this matters

The mismatch this catches is not hypothetical, and the record of it is what minted milestone 217.
The VisionFive 2 booted, reached U-mode, and then halted with `MEASURED BOOT REFUSED: 'init' is not
what this kernel image was built against`, two sha256 digests, and nothing to do about it except
walk back to the Mac. That is the cost of finding a mismatch after a power cycle instead of before
one: a bench trip, a card pulled and re-seated, and a serial session spent on a question a file
comparison answers in a second.

The gate that caught it is a runtime refusal by design, and that is correct: the kernel must refuse
an archive it was not built against. But a runtime refusal is the last possible moment to learn it,
and it is the moment when the person is furthest from the tools that can fix it.

## What it would take

Read the kernel image and the archive from the mounted path, compute the archive's sha256, and
compare it against the digest the kernel image was built against, which is the same comparison
`measured_boot` performs at boot. Report the two digests on a mismatch, the way the kernel's refusal
does, so the output of the tool and the output of the board are the same thing said in two places.

The honest limit to record beside it is the one milestone 217 already wrote down: nothing in this
tree has touched a real microSD card. The card option has only ever written to a directory on a
Mac's own disk, and a verifier reading a mounted card inherits that same untested boundary.

## Where it came from

Milestone 217 (the card carries a kernel and an archive from different builds) named it: *"A tool
that reads a mounted card and reports whether its kernel and its archive match. `--card` narrows who
needs one rather than removing the need: a card written by any other means stays unverifiable
without booting it, so a mismatch is found after a power cycle at the bench instead of before one."*
