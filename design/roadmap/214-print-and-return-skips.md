# 214. A test that prints "skipping" and returns is counted as passed

**Status: NOT-STARTED.** Minted 2026-09-01 by milestone 164's lane, which made eleven tests move
from the skip column to the pass column without running anything. **Provisional number**: the
integrator mints it at merge.

**Gate: NONE.** A sweep of test code in this repository; nothing outside it is involved.

## What it needs

`kernel/src/testing.rs` has a `skip!` macro. It stores a reason and returns, and the runner prints
`skipped: <reason>` and counts the test in a separate column, which is exactly right: a suite that
reports "200 passed, 55 skipped" is telling the truth about what it proved.

**Forty-six sites across sixteen files do not use it.** They `crate::println!("    (no RedoxFS disk
attached; skipping)")` and `return`, and a test that returns without setting a skip reason is
indistinguishable from one that passed. The line is in the transcript, so a human reading the
scrollback can see it; the counts cannot, and neither can anything that reads the counts.

The fix is mechanical: those sites become `skip!` with the same words. What makes it a milestone
rather than a `sed` is that the shape is not uniform (some print inside a `let ... else`, some
after a partial assertion, and at least one, `sink_tests`, prints something that is not simply
"skipping" but "the pipe arm stands alone", meaning the test **did** prove something and only part
of it was skipped). A blind sweep would flatten that distinction, which this tree has a scar about.

## Why it matters, with the case that found it

Milestone 164 packed the FS server into the x86_64 archive. Eleven tests that had been skipping
with "no fs_server in this archive" moved to skipping with "no RedoxFS disk attached", and because
the second arm is a `println!` and the first was a `skip!`, `script/test --arch x86_64` went from
"200 passed, 55 skipped" to "211 passed, 44 skipped" **while running exactly as much code as
before**. A reader comparing those two lines would conclude eleven tests were recovered. None were.

That is the failure this project's first principle is about: a number that cannot be gamed is the
point, and this one can be moved without touching a test. It is also rung two of the ladder going
missing where rung two already exists and is one macro away.

## What to check while sweeping

- **Does the reason still name the real cause?** Milestone 164 found `rmle_tests` giving
  `NO_FS_SERVER` for what is actually an absent disk, and widened that constant rather than let it
  become a false statement. Others may be similarly stale.
- **Partial skips want their own shape.** `sink_tests::one_reader_two_sources_and_the_same_answer`
  runs one arm and skips the other. "Skipped" is wrong for it and "passed" is wrong for it, and
  deciding which is right (or whether the two arms want to be two tests) is the judgment this
  milestone owes.
