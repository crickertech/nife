# 201. Is multicore reliability converging? A defect-discovery curve, not a stress run

**Status: NOT-STARTED.** Minted 2026-08-31 by calef, scoping `design/fatal-risks.md`'s risk 5, which
its own `BUGS` recorded as unowned. *(Number provisional until the merge queue lands it.)*

**Gate: HARDWARE.** Three boards, and the sense of the gate that means somebody has to sit at them.

**In brief.** Risk 5 is *"it cannot be made reliable on multicore, and the bugs appear only on
silicon."* The experiment written against it was "sustained multi-core stress on all three boards",
which is a plan and not a test: it can never come back green, because you either stop finding bugs
or you run out of time.

**So measure the rate rather than the absence.** Track defects found per hour of stress. A curve that
flattens is evidence that the concurrency is converging. **A curve that stays linear is the red
result**, and it is the only framing of this risk in which "we cannot make this reliable" is a
possible outcome rather than an admission.

## Why this risk needs the reframe more than the others

It already fired once, and by luck. The VisionFive 2 produced a receiver woken with nothing
delivered, on three harts, that no emulator run had ever shown; the fix came out of a bench session
rather than a test. x86_64 carries two unresolved `ap_boot` bugs (milestone 161), which is the same
class arriving on the third architecture.

Those are the curve's first three data points, and they should be entered from the record rather than
waiting for new ones.

## What the run needs

- **The load-sensitive assertions live** (`notes/load-sensitive-assertions.md`), plus
  `script/repeat-under-load` and `script/interleaving-check`, since a defect nothing asserts on is a
  defect nobody counts.
- **Hours logged per board**, because the denominator is the whole measurement and is the thing most
  likely to be recorded badly.
- **Every defect classified** as concurrency or not. A curve polluted with unrelated failures answers
  a different question.

## BUGS

- **A flattening curve is a confidence, not a verdict.** It cannot prove the concurrency is right,
  only that this project is finding fewer problems per hour than it used to, which is also what
  running out of imagination looks like.
- **Three boards is not three samples of the same thing.** aarch64, riscv64 and x86_64 have different
  memory models, so a per-board curve is three measurements, and pooling them would hide the
  architecture where the trend is worst.
- **The denominator is manipulable without anyone meaning to.** Hours of stress on an idle workload
  find nothing and flatten the curve for free, so the workload has to be stated with the number.
- **This is expensive and slow**, and its own answer arrives over weeks. It is on the fatal-risk list
  because it could be fatal, not because it is efficient.
