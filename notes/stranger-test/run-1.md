# Stranger test run 1, 2026-08-14

*An appendix to [the stranger test](../stranger-test.md), which holds the current state. This page
is the record for run 1.*

## Run 1, 2026-08-14: an x86_64 container, no QEMU

The task: "Get the project building and its tests passing, then write up what this system is and
how you would add a new user program to it." No brief, no pointers, no answers.

### The headline finding, which nobody in the tree could see

The workspace had not built on an x86_64 host since 2026-08-03. `--exclude` removes a package from
the test *selection*, not from the dependency graph. So excluding `user_mode_runtime`, while four
crates depended on it unconditionally, left it in the build. CI moved to `ubuntu-24.04-arm` the same
day those dependencies landed, and there the EL0 assembly compiles by accident. The one gate that
would have caught the bug ran on the only architecture where it is invisible. `script/lint`'s
comment asserted the opposite and was wrong in both directions. Fixed, with a gate that derives the
bare-metal set from `cargo metadata` rather than maintaining a list.

The stranger rejected its own brief to find it. It was told the container had no QEMU and that the
kernel tests could not run. It installed QEMU anyway to test that premise, and watched `script/test`
fail with the same errors *before* QEMU was invoked. It reported: *"The machine overruled the
brief."* That is this project's own rule, applied by an agent that had not read it.

### Its other findings, all fixed in the same pass

- `DECISIONS.md` did not exist while the whole tree cited `§N`. A signpost now does.
- No document described how to add a user program. `adding-a-program.md` now does.
- `notes/program-manifest.md` listed five fields against the code's ten, so following it produced a
  struct that would not compile.
- `CLAUDE.md` is the project's constitution in a filename that tells a human it is not for them. The
  README now says so out loud.

### Where the instrument failed

The stranger found this itself. While grepping for "designation is authorization" it hit the
stranger-test note and read the rubric, including the "pass means" column. It disclosed that
unprompted. It named questions (g) and (h) as contaminated, re-derived both from primary sources,
cited those, and told the reader to discount them anyway. The rubric was in the repository the
stranger is told to read, and nothing in the protocol anticipated that. See
[the BUGS history](bugs-history.md).

No score is recorded. Two of the eight answers are contaminated, so a score would be a number with a
footnote, and the questions it asked are worth more than that number. The full report is in the pull
requests that carry the fixes.

Run 2 was owed after these fixes, with a different stranger. One pass measures; two show whether the
fixes worked.
