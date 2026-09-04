# Sample `compositor`'s five remaining full-screen sweeps under Miri

**Status: PROPOSED 2026-09-03.** Written by the milestone 247 sweep, from milestone 238's block.

**Gate: NONE.** The pattern already exists in four crates, the one worked example in `compositor`
itself is measured, and nothing external blocks it.

**In brief.** `script/undefined-behavior-check` runs the test suite under Miri, which is roughly
three orders of magnitude slower than native. `glob`, `ntp_proto`, `calendar` and `gpt` already
handle this by sampling their exhaustive loops under `cfg(miri)` rather than running them whole.
`compositor` has **six full-screen per-pixel sweeps** and only one of them has been strided; it fell
from over 44 minutes to **57 seconds**. The other five are untouched and are now the whole remaining
cost of the check. The work is to stride them the same way, then measure the real end-to-end run.

## Why this matters

The check is currently affordable only because its budget was **raised to 240 minutes**, not because
its cost was reduced. Milestone 238 did that deliberately to get the workflow green after three
weeks red, and said so. The consequence is that the true end-to-end cost of the Miri run has never
been measured: the budget is a ceiling nobody has pushed against, so nobody knows whether the check
finishes in ninety minutes or in two hundred and thirty.

A four-hour budget also decides how the check can ever be used. At that cost it is a scheduled job
and can never be anything else. One data point says the reduction available is large: 44 minutes to
57 seconds on the one sweep that was done. If the other five behave similarly the check moves into a
range where running it more often is a real option, and that is a different check.

There is a second cost to the current state, which is the one that bites a lane. A red Miri run
takes up to four hours to tell anyone anything, and milestone 232's audit already recorded what
happens to a slow check that cries wolf: the only available response is to stop reading it.

## What it would take

The pattern is copy-work with judgment in one place: choosing the stride. Sampling every Nth pixel
under `cfg(miri)` preserves the shapes of the memory accesses Miri is checking while cutting the
count, and the four existing crates show what N looks like in practice. The judgment is that a
stride must not accidentally skip an edge case (the first row, the last pixel, a wrap) that the
exhaustive loop was covering, since Miri finds bugs at the boundaries as often as in the middle.
Afterwards, run the check end to end and record the real number, which is the part the budget raise
skipped.

## Where it came from

Milestone 238's `## Follow-on`: *"Sample `compositor`'s six full-screen per-pixel sweeps under
`cfg(miri)`, the way `glob`, `ntp_proto`, `calendar` and `gpt` already sample theirs. One was
strided and fell from 44+ minutes to 57 seconds; the other five are untouched and are now the whole
remaining cost of `script/undefined-behavior-check`. The budget was raised to 240 minutes instead of
tuned, so the true end-to-end cost has never been measured."*

The same block **refused** excluding `compositor` from the Miri run to make it fit, and the reason
bounds this work: the crate has no `unsafe` today but is central system logic under active
development, and an excluded crate is one where a future `unsafe` block is silently uncovered. So
the answer has to be striding, not exclusion.
