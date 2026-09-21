# The x86 boot calibrates the TSC once, and can be wrong by 4x

**Status: PROPOSED 2026-09-21.** Raised by the `tscdrift` lane, which was sent to measure whether
TCG's TSC drifts and found that it does not, while the number the boot writes down does.

**Gate: NONE.** The fix is inside `kernel/src/arch/x86_64/timer.rs`, touches no syscall surface, no
wire format and no dependency. It is one function.

## The measurement

`notes/tsc-under-tcg.md` has the method. Against a counter measured to tick at exactly 1000.000 MHz
under plain TCG on patagonia, twenty-two boots of the same binary reported:

| Host state | Stored rate, per boot |
|---|---|
| Ordinary session load | 1001, 1001, 1002, 1002, 1003, 1003, 1003, 1004, 1004, 1005, 1005, 1042 MHz |
| Saturated (16 spinners) | 1003, 1003, 1006, 1422, 1703, 2349, 2364, 2616, 3421, 4330 MHz |

**Twenty-two of twenty-two were above the truth and none below**, which is the shape that names the
mechanism rather than merely describing the spread.

## Why it is one-sided, which is what makes it cheap to fix

`init_frequency` arms PIT channel 2 for a 10 ms one-shot and polls port 0x61 until the output line
goes high, reading the TSC either side. The poll can only ever notice the terminal count **late**,
never early, and the entire elapsed TSC delta is then divided by exactly 10 ms. A single descheduling
of the QEMU thread inside that window inflates the answer without bound: 4330 MHz is a 43 ms window
reported as a 10 ms one.

So the error is **non-negative by construction**, and the minimum of several windows converges on the
truth from above. That is not an argument from taste; it is why averaging would be the wrong
estimator here and the minimum is the right one.

## What to do

**Take the smallest of N windows rather than the only one.** Five 10 ms windows cost 50 ms of boot
and turn an unbounded tail into a bounded one, because a run of five that are all descheduled is the
thing that has to happen for the answer to stay wrong. The one-sidedness above is what makes the
minimum a sound estimator; with a two-sided error it would be a biased one.

Two things to decide while doing it, both small:

- **Whether the boot should say how confident it is.** The spread between the best and worst of the
  N windows is free once they are taken, and a boot line that printed it would make a bad
  calibration visible instead of silent. That is the same posture milestone 524 (the three x86_64
  boot gates: NX, SYSCALL, and the invariant TSC) took for the invariant-TSC bit.
- **Whether the local APIC timer's rate should be derived from the same windows.** Today it is
  measured in the same single window and inherits the same error, which is why the `apic timer NN
  MHz` figure moves in lockstep with the TSC one in every transcript.

## What it is worth

`bench --x86 --real`'s ns/iter, `Instant` and `uptime` through `counter_frequency_protocol`'s page,
and `coremark`'s self-reported rate all read the stored number, so on a loaded machine any of them
can be off by a factor of four with nothing saying so. `wait_for`'s deadlines fail safe (an inflated
rate makes a timeout longer in real time, never shorter), which is why this has never shown up as a
red test and why it would have gone on not showing up.

It also decides how much a **D'**-style marking (carrying "unpromised" in the type, so consumers must
acknowledge the rate is unpromised) is really worth: a marked number that is still 4x wrong is
honest about the wrong thing.
