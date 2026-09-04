# The boot tour prints nothing between `pcie` and `hw entropy`, so the last half of fatal risk 6 can only be measured by eye

**Status: PROPOSED 2026-09-04.** Written by milestone 159's third lane, from that milestone's own
bench procedure.

**Gate: NONE.** The tour already reads a timebase elsewhere and the change is to the same file the
step lives in (`kernel/src/main.rs`). Confirming the number on radon is `HARDWARE`, but the work
does not start there: the instrumentation can be written and exercised against QEMU's virtio-rng
backend, where the same tour prints the same shape of line.

**In brief.** `design/fatal-risks.md` risk 6 is *"a capability-confined userspace driver cannot
drive real hardware at real speed"*. On 2026-09-04 its **confined** half and its **drives real
hardware** half were both demonstrated on radon. **At real speed** is the remainder, and nothing in
the tree can currently answer it, because the only clock available to a bench session is a person
watching a serial console.

Concretely: the riscv64 tour emits `pcie` and then, with nothing in between, `hw entropy`. The wall
time between those two lines is roughly one TRNG bring-up (a reseed, then a generation) plus the
eight `entropy_proto` round trips the two 32-byte draws now take. A person with a stopwatch resolves
that to about a second, which answers "is this milliseconds or minutes" and nothing finer. A
bytes-per-second figure worth publishing needs the machine to time itself.

**What to build.** Read the timebase around the `hw entropy` step and print the elapsed time in the
step's own line, separating the two costs that are interesting for different reasons: the bring-up
(a once-per-boot cost, which is what a slow reseed would show up in) and the per-draw cost (which is
the rate). The tour already has a clock: `uptime` and `timebase_proto` are in the tree and the
riscv64 tour reads the timebase for other steps.

**Why it is worth a proposal rather than a line in the driver.** Two reasons it should be decided
rather than assumed. The number is a **fact that leaves the machine** in the *move fast on what can
be undone* sense: a rate quoted against a capability microkernel is exactly the kind of figure a
stranger repeats, so what it includes has to be stated (does it count the IPC, the poll loop, the
process spawn?) before it is printed rather than after. And the honest comparison is not obvious:
the interesting claim is against Linux's `jh7110-trng.c` on the same silicon, which is interrupt-
driven where this driver polls, so a like-for-like number needs the two to be measured over the same
thing. `notes/benchmarks.md`'s standard applies: say where it is not apples-to-apples.

**Blocked until it is answered:** nothing. Milestone 159 can close its other questions without this,
and fatal risk 6's first two halves are already demonstrated. This is the third half, and it stays
open in the risk file until somebody measures it.
