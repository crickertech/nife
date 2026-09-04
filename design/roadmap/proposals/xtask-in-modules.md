# `xtask/src/main.rs` is 6,785 lines with no module structure

**Status: PROPOSED 2026-09-03.** Written by the milestone 247 sweep, from milestone 130's block.

**Gate: NONE.** No decision is owed. What it needs is a scheduled slot rather than a permission:
`xtask/src/main.rs` is one of the three merge hotspots every lane wires its test into, so a
wholesale restructure conflicts with every branch in flight and should run when the board is thin.

**In brief.** Split `xtask/src/main.rs` into modules along the seams it already has, which are the
commands. The compiler verifies the split completely, so the edit is mechanical and its failure
mode is a build error rather than a subtle one. The work is not the thinking, it is the timing.

## Why this matters

The honest answer is that most of this is a readability win, and a readability win in a build tool
is worth less than one in the kernel. Two things make it more than tidiness.

The first is the merge cost, which cuts both ways and is the reason to do it deliberately rather
than never. Today every lane that adds a command or wires a test edits the same 6,785-line file,
and AGENTS.md's own measurement of the merge queue found that lane collisions scale through files
rather than through numbers, with this file named as one of the three hotspots. Modules turn most
of those edits into edits of different files. The restructure itself is one large conflict, paid
once, in exchange for the recurring ones.

The second is that a file this size stops being read. Milestone 130 was a duplication sweep and
found its material by reading; the next such sweep pays the same cost again.

Against that: it is genuinely low risk and genuinely low urgency, which is why it has sat. If it
stays unpromoted for a long time, that is a defensible outcome and not a failure of this file.

## Where it came from

Milestone 130's Follow-on: *"Split `xtask/src/main.rs`, 6,785 lines with no module structure, into
modules. The compiler verifies the split completely so the edit is mechanical; what it needs is a
scheduled slot, because that file is one of the three merge hotspots every lane wires its test into
and a wholesale restructure conflicts with every branch in flight."*

The same block refused a related split for a measured reason worth carrying: splitting
`kernel_main` came out byte-identical and still broke the build, because four `cfg`-gated blocks
park early in `arch::halt()` and one divergent function absorbs the unreachable tail where two do
not. Nothing like that is expected here, since `xtask` builds for the host and has no early-park
pattern, but it is the reason to run the split as its own commit rather than beside other work.
