# 225. Run the soak on radon, argon and xenon, which is the only place its answer means anything

**Status: NOT-STARTED.** Minted 2026-09-02 by the maintainer, carrying forward the follow-on that
milestones 219 (the boot tour ends and the kernel halts, so there is nothing to soak) and 221 (the
soak never crosses cores, so build the hook that makes it) both proposed. *(Number provisional until
the merge queue lands it.)*

**Gate: HARDWARE.** In milestone 53's sense: the boards are on the desk and this needs hands on
them. Nothing else blocks, and nothing more can be built for it.

**In brief.** Fatal risk 5's entire premise is that the defects appear only on silicon. Its one
recorded instance is exactly that: **radon** produced a receiver woken with nothing delivered, on
three harts, in a bench session, and no emulator run had ever shown it.

Everything needed to run it now exists, and none of it existed on 2026-09-01:

- **A workload that lasts** (milestone 219), with a heartbeat on the wall clock so a crawling machine
  still reports on time.
- **A hook that makes it cross cores** (milestone 221), on the real `irq_notify` to `wake_load_aware`
  path, which is where the recorded defect lived.
- **A console that watches and judges** (milestone 216), with a sustained mode and a stage that
  re-arms the quiet check a completed boot tour suppresses.
- **A boot that needs nobody typing** (milestone 218), unconfirmed on the board itself.

## What it needs

**Bench evenings, one per machine, and the discipline to read the first heartbeat before walking
away.** Milestone 221's procedure is explicit about this and it is the part most likely to be
skipped: `wakerate` should be about `100 * harts`, and `crossings` must be **rising** between beats
rather than frozen. Eight hours of a non-crossing soak is eight hours of milestone 219's experiment
rather than 221's, and the difference is invisible afterwards.

Record `rounds`, `rate`, `wakes` and `crossings` for every run, in `notes/soak.md`'s table.

## What an answer would and would not be

**A clean run licenses one sentence**, which milestone 219's tooling prints on every green result:
this machine did N cross-core round trips without the wake gate refusing one, without a wrong reply,
and without a worker stalling. It is not proof the concurrency is correct, and the risk's own text is
honest that this class of question "produces a confidence rather than a verdict".

**A failure is worth far more**, and is the outcome to hope for. It would be the second defect this
risk has produced and the first found by an instrument rather than by somebody watching a bench.

## BUGS

- **No duration is prescribed**, because nobody knows what would be persuasive, and milestone 219's
  block says the same thing for the same reason.
- **A hung board needs a person**, since nothing can power-cycle radon remotely (milestone 224) and
  `script/board-console` reads without writing.
- **The crossing count varies by more than 2x between identical runs**, recorded in milestone 221's
  BUGS, so it is not a figure to compare machines on without more care than a single run affords.
- **argon has never booted nife at all**, so its soak sits behind milestone 127 (the seL4 machine)
  rather than beside radon's.
