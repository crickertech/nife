# 191. Did the proofs catch the bugs? A retrospective of every real defect against the harness that should have found it

**Status: BUILT 2026-08-30.** Minted the same day by calef, from the fatal-risk sweep
(design/fatal-risks.md), and run the same day: `notes/proof-retrospective.md`, PR #589.
**Corrected 2026-09-10**: this block sat `NOT-STARTED` for eleven days after the work landed and
merged, the exact status-in-two-places defect §76's own sweep found before. Caught by a maintainer
review of `design/fatal-risks.md` against the tree, not by any gate. *(Number provisional until the
merge queue lands it.)*

## What it found, in brief

**No Kani harness in this tree has ever caught a defect after the day it was written.** All eighteen
defects in the corpus were found by something else (a flaky suite, a boot on real silicon, a fuzzer,
the mutation sweep, loom, a code read, or a CI lint), because one line of `script/verify`'s own header
meant `cargo kani` never compiled the kernel, the user programs, or `xtask`, so 64,818 lines of
`kernel/src`, exactly where the concurrency and hardware-contract defects lived, were out of reach by
construction. Two real defects were caught while harnesses were *being written* (`dtb::be32`'s
unchecked `at + 4`; `pci::intx_irq`'s pin-0 underflow), which is survivorship showing up as evidence
rather than as an excuse. The counted numbers were also wrong: 145 harnesses, not "112+"; `script/verify`
runs 140; 19 vacuity guards exist across 4 harness crates. Full account, including the reverse pass
that found real chaff (a proved tautology, twelve per-ISA restatements of six properties), is
`design/fatal-risks.md` risk 2 and `notes/proof-retrospective.md`; this block does not repeat it.

**In brief.** DECISIONS §14 (a verified-Rust capability microkernel) promises a verified core. There are 112+ Kani
harnesses and `notes/verification.md` explains how they work. **Nothing in this tree asks whether
they caught anything.** This milestone answers that against the only evidence that cannot be
arranged: the project's own record of real defects.

## The question, stated so it can come back red

**For every real bug this project has found, could a proof have caught it, and did one exist?**

A red result is a finding that the harnesses cluster on pure algorithmic properties while every
defect that actually cost time was a concurrency, hardware-contract, or integration bug that no
proof was ever positioned to see. That would mean the verification claim is real but narrow, and
narrow in the direction that does not matter, which is a serious wound to half the thesis.

A green result is at least as valuable and is the more likely one: proofs caught a class of thing
early enough that it never became a bug, which is invisible in a defect list precisely because it
worked. **That asymmetry is the hard part of this milestone**, and the method below is what keeps it
from being a study that can only confirm.

## The method, because the shape decides whether the answer means anything

**One pass, four fixed questions per defect**, the same discipline `notes/arch-audit.md` used and
milestone 187 (read the x86_64 arch tree through the lens the first arch audit used) is repeating:

1. **What was the defect, stated as a property that was false?** Not "the board hung", but the
   invariant that did not hold.
2. **Was that property provable at all?** Some are not: a property about what real silicon does with
   a store buffer is not a property of our source.
3. **Did a harness exist that covered it?** If yes and it passed anyway, that is the most
   interesting outcome on this list and deserves its own writeup.
4. **What would it have cost to have one?** This is the question that turns the study into a
   worklist rather than a scoreboard.

**And one pass in the other direction**, which is what makes the result honest: walk the harnesses
and ask which ones constrain something that could plausibly have gone wrong, and which prove a
property that could not have been false. A harness over an algorithm nobody could have written
incorrectly is a passing check that buys nothing, and counting it is how a verification claim
inflates.

## The corpus, which already exists and is unusually good

This project writes its failures down, so the defect history is real rather than reconstructed:

- **The VisionFive 2's undelivered wake** (notes/visionfive2.md): a receiver woken with nothing
  delivered, found on three harts on real silicon, invisible in QEMU. The single most important
  entry, because it is the shape most likely to be the red result.
- **The load-sensitive assertions** (notes/load-sensitive-assertions.md), including the run that
  went red on a clean kernel.
- **The arch audit's own bug class** (notes/arch-audit.md): state staged in single-copy hardware
  registers across more than one instruction while an exception can land in the middle.
- **The FS-server stack bug**, the PLIC hart lottery, the timer drift (notes/instruction-clock.md),
  the two-core crash (milestone 161's lane), the `std-src` toolchain race, and the record-level
  re-run that corrected stale counts (notes/fs-server.md).
- **The nine misrecorded roadmap statuses** and the fabricated block quote that survived twelve days
  of gates. These are not code defects and they belong in the study anyway, because they are the
  same question one level out: what did the mechanisms fail to see?

## Why this is first on the fatal-risk list

It costs an afternoon, needs no hardware, blocks nothing, and aims at half the thesis. Every other
experiment on that list needs a lane, a board, or both. **A cheap test that can return red is worth
more than an expensive one that probably will not**, and this is the cheapest one available.

## BUGS

- **A retrospective cannot prove a counterfactual, and the study said so rather than presenting
  judgement as measurement.** Held.
- **Survivorship runs both ways.** The two defects caught while harnesses were being written are the
  measured half of this; bugs nobody has found yet are still not in the corpus, and nothing here
  closes that.
- **This block itself was the worklist-going-stale risk, realized.** It named milestone 94's inventory
  as the precedent for what happens when a finding has no gate; its own status sat wrong for eleven
  days as exactly that. The fix was a person reading `design/fatal-risks.md` against the tree, which
  is rung zero and is not a mechanism.

## Follow-on

- **Milestone 193.** Put `kernel/src` within reach of the prover. Built the same day.
- **Milestone 197.** `user/` and `xtask`, for the same reason. Built.
- **Recorded.** `notes/proof-retrospective.md` carries the full study: the eighteen-defect corpus,
  the four fixed questions asked of each one, and the reverse pass over the harnesses that found the
  proved tautology and the per-ISA restatements.
