# The NVMe end-to-end test has no replayable falsification

**Status: PROPOSED 2026-09-17.** Written by the milestone 318 lane, which falsified the test by
hand and then had nowhere to put the evidence.

**Gate: NONE.** No decision is owed about the defect to inject. What is owed is a judgement about
cost, which is why this is a proposal rather than a line of milestone 318: a fifteenth kernel
falsification record lengthens `script/falsifications`' kernel sweep for every lane that runs it,
and this test needs an NVMe controller attached, so it is not free on any leg.

**Promoted:** minted as **milestone 323** on 2026-09-18, as one of its parts rather than on its own.
calef promoted the cluster: this proposal and its siblings were each filed by a different lane
against the same surface, and answering them one at a time would have produced one brief per face of
a single finding. The record is
[design/roadmap/323-falsification-record-completeness.md](../323-falsification-record-completeness.md);
the status line above keeps its original date, because that is what makes the pile measurable, and
this file keeps its own argument, because the proposal is the argument as it stood and the milestone
is the account.
**In brief.** `kernel::user::nvme_tests::a_confined_el0_process_serves_the_block_interface_end_to_end`
is milestone 261's proof and fatal risk 6's decisive experiment, and nothing replays a defect
against it. Give it a `Falsification:` line and a patch under `kernel/falsifications/`.

## Why this matters

This is the test somebody will stand at a bench and photograph. Its single `ok` is the whole of
risk 6's evidence, and §134's argument applies to it more than to most: a test whose red has never
been seen is a test whose green means less than it looks.

Milestone 318 has already done the experiment twice, by hand, and both defects fired:

| Defect injected | What fired |
|---|---|
| `past_end` computed as `size / BLOCK_SIZE - 1` | `the server read block 2047, which a namespace of 8388608 bytes does not have` |
| both writes sent to block 37 instead of 37 and 38 | `byte 0 of block 37 came back wrong` |

Neither of those is a *code* defect, which is the gap. Both are edits to the test, so they prove
the assertions are not vacuous and prove nothing about whether a real defect reaches them. The
record the convention asks for is a patch against the thing under test.

## The natural defect

An off-by-one in `nvme::Handoff::holds_block`, `block * unit <= self.size_bytes` in place of
`(block + 1) * unit <= self.size_bytes`, which admits exactly the first block past the end. It is
the defect a reader is most likely to write, it is the arithmetic milestone 318 rewrote the
assertion to exercise on any namespace, and it is in the driver rather than in the test.

One thing to check when writing it: that patch also breaks `crates/nvme`'s Kani harnesses, which is
correct and probably desirable (the proof and the boot test agree about the same property), but the
record should say so rather than let a future reader discover it as a surprise.

## What it costs

One patch file, one doc line, and a longer sweep. The sweep is the part to weigh, and nobody has
measured what one more kernel record adds to it; that measurement is most of the work.
