# 365. `xtask/src/main.rs` is 6,785 lines with no module structure

**Status: NOT-STARTED.** Filed as a proposal on 2026-09-03 by the milestone 247 sweep, from
milestone 130's block; promoted by milestone 433 on 2026-09-19. Checked that day: `xtask/src/` still
holds exactly one file and `xtask/src/main.rs` is now **10,680 lines**, not the 6,785 the title and
body name. The file grew 57% in the sixteen days this sat in the pile, which is the argument for
doing it rather than against.

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

## Index row

`xtask/src/main.rs` is one file with no module structure, 6,785 lines when this was filed on
2026-09-03 and 10,680 on 2026-09-19. The split runs along seams the file already has, which are the
commands, and the compiler verifies it completely, so the edit is mechanical and fails loudly. What
it needs is a scheduled slot rather than a permission: this is one of the three merge hotspots every
lane wires its test into, so a wholesale restructure conflicts with every branch in flight and
should run when the board is thin. The recurring merge cost is the reason it is more than tidiness,
and milestone 130's refused `kernel_main` split is the reason to run it as its own commit: that one
came out byte-identical and still broke the build.
