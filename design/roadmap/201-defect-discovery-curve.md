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

## Why this risk needs the reframe more than the others, and why its seeds needed re-deriving

**Found 2026-09-23: all three of this section's original seeds were wrong or stale, and they are
corrected here rather than quietly replaced.**

The section as minted (2026-08-31) said the risk "already fired once, and by luck," citing the
VisionFive 2's "receiver woken with nothing delivered" plus x86_64's two then-unresolved `ap_boot`
bugs (milestone 161 (the x86_64 kernel port: bring up the HAL's third architecture)) as the curve's
first three data points.

**The VisionFive 2 reading is void, not merely outdated.** `notes/visionfive2.md`'s fifth bench stop
(2026-08-15, the day *before* this milestone was minted) overturned it with five independent
identifications: the dumps the fourth stop read as a stranded receiver were the terminal state of a
*completed* boot tour. `notes/scheduler.md:70` records the consequence plainly: the wake-refusal gate
built against this reading "has never fired on a field failure," and `refuse:` has never appeared on
a board ring. It is hardening against a reachable state, proved by a loom harness, not a repair of an
observed one. There was no silicon-only, QEMU-invisible defect here. It never happened, and treating
it as risk 5's fired instance is exactly what risk 5 needs not to have.

**That matters more than it looks, because it was risk 5's only candidate for the risk's own central
claim.** Risk 5 is specifically that "the bugs appear only on silicon" (QEMU cannot show them). With
the VisionFive 2 reading gone, **risk 5 currently has zero confirmed instances of its own defining
claim**, not one.

**The two `ap_boot` bugs are real defects, but "unresolved" is stale.** Both were open when this
milestone was minted; both are now closed: the boot-core-identity bug (`arch::x86_64::ap_boot`'s
`BUGS` #3) was fixed by milestone 316 (which core booted: making `NIFE_SMP=2` mean something on
x86_64) on 2026-09-17, and the third-or-later-secondary race (`BUGS` #1) was fixed by milestone 161's
own closing work (2026-09-19). A third x86_64 SMP bug from the same
bring-up, a missing cross-core TLB shootdown (`BUGS` #2), was found and fixed the same day,
2026-08-25, before this milestone existed, and this section never counted it.

**None of the three was found on silicon.** All three were found and reproduced under QEMU TCG, never
on real x86_64 hardware (xenon). They are genuine concurrency defects and belong in a defect history,
but they do not speak to risk 5's specific claim either, since that claim is about defects QEMU
cannot show, and QEMU showed all three of these.

**So the corrected seed set, as of 2026-09-23:**

1. **x86_64: a missing cross-core TLB shootdown** (`ap_boot`'s `BUGS` #2). A dead thread's kernel
   stack, unmapped and remapped onto a different frame for reuse, was still translated by another
   core through a stale TLB entry, so a recycled thread's context read back as stack paint.
   Reproduced 10 of 10 runs at `NIFE_SMP=2`; found and fixed 2026-08-25.
2. **x86_64: a boot-core-identity mixup** (`ap_boot`'s `BUGS` #3). `boot_cpu_id` answered "which core
   am I" instead of "which core booted," so a test body DECISIONS §28 (SMP placement: two random
   choices at spawn, message-shaped stealing, local wakes)'s placement migrated onto a secondary
   mistook that secondary for the boot core. Found 2026-08-25 (while verifying #2 above); fixed by
   milestone 316, 2026-09-17.
3. **x86_64: a lost-checkin race in secondary bring-up** (`ap_boot`'s `BUGS` #1). `cpu_start`
   re-read the online count after issuing `STARTUP` IPIs and waited for it to move again, so a core
   that checked in during the settle delay was counted absent. 26 of 40 plain boots failed at
   `-smp 4` before the fix; fixed by milestone 161's own closing work, 2026-09-19.

**All three are QEMU-only findings, dated and closed, not the silicon-only defect risk 5 exists to
catch.** The curve therefore has three real data points for ordinary multicore concurrency bugs, and
**zero for the specific failure mode this risk names**. Presenting the three above without saying
that would be exactly the overclaim this milestone's own first `BUGS` entry warns against ("a
flattening curve is a confidence, not a verdict").

## What the run needs

- **The load-sensitive assertions live** (`notes/load-sensitive-assertions.md`), plus
  `script/repeat-under-load` and `script/interleaving-check`, since a defect nothing asserts on is a
  defect nobody counts.
- **Hours logged per board**, because the denominator is the whole measurement and is the thing most
  likely to be recorded badly.
- **Every defect classified** as concurrency or not. A curve polluted with unrelated failures answers
  a different question.

## BUGS

- **The original seed set was wrong, and the correction is recorded in the section above rather than
  by silently replacing it.** Minted 2026-08-31 citing a VisionFive 2 finding that `notes/
  visionfive2.md`'s fifth bench stop had already retracted the day before (2026-08-15), and two
  x86_64 `ap_boot` bugs that were open then and are both closed now (milestones 316 and 161). Found
  2026-09-23 by the lane sweeping the retraction's propagation. The lesson worth keeping is not the
  specific facts, it is that a retraction landing one day before a milestone is minted is exactly the
  timing most likely to be missed, and nothing caught it for 23 days.
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

## Index row

Scopes `design/fatal-risks.md`'s risk 5, which its own BUGS recorded as unowned. "Sustained
stress" is a plan, not a test: it can never come back green. Measuring defects found per hour can:
a flattening curve is evidence, **a linear one is the red result**. **Its original three seeds were
wrong** (found 2026-09-23): the VisionFive 2 reading was retracted 2026-08-15, before this milestone
was even minted, and it never happened; the two "unresolved" x86_64 `ap_boot` bugs are now both fixed
(milestones 316 and 161). The corrected seed set is three closed, QEMU-only x86_64 concurrency
defects; risk 5's own defining claim, a defect QEMU cannot show, **has no confirmed instance**.
Gate: HARDWARE, in the sense that somebody must sit at three boards.
