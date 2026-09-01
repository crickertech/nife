# The console fixtures, and the fact that they are synthetic

**Every `.log` here was written by hand, not captured from a board.** Nothing in this repository
holds a real VisionFive 2 console capture: the bench sessions of 2026-08-14 and 2026-08-15 are
recorded in `notes/visionfive2.md` as prose and quoted fragments, and the transcripts themselves
were never committed. That is worth saying twice, because a fixture that looks like a capture and
is not one is the exact shape of the fabricated block quote this tree carried for twelve days.

So these are **constructed from documented sources**, and each line is one of three things:

1. A marker quoted from `notes/visionfive2.md`'s bench runbook ("What appears, in order, on a good
   day") or its failure-triage ladder.
2. A line quoted from this tree's own code: `kernel/src/main.rs` for the banner,
   `kernel/src/panic.rs` for `[PANIC]`, `notes/trusted-init.md` for the measured-boot refusal.
3. **Filler in the shape vendor firmware prints**, so a chunk boundary lands somewhere other than
   on a marker. The version numbers, the DRAM size, and the hart table are plausible rather than
   measured, and no test asserts on any of them.

What that means for the tests: they prove the recogniser finds the markers **it was told about**,
in a stream that behaves like a stream. They prove nothing about whether those markers are the text
the board actually prints. Only a bench capture can do that, and the first one taken should replace
these files, at which point this README's first sentence stops being true and should be deleted.

`vf2-good-boot.log` deliberately shows the **manual** boot path (the `StarFive #` commands from the
runbook and `script/board-image`), because that is the path a first boot uses.
