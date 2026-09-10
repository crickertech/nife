# The boot tour prints nothing between `pcie` and `hw entropy`, so the last half of fatal risk 6 can only be measured by eye

**Status: PROPOSED 2026-09-04.** Written by milestone 159's third lane, from that milestone's own
bench procedure.

**Gate: HARDWARE.** Only for what is left. The gate was `NONE`, correctly: the instrument was written
and exercised under QEMU on 2026-09-10 (branch `milestone/159-time-hw-entropy`) and the section at
the bottom of this file records what it printed. What remains is one boot of radon, which nothing
but radon can do.

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

## Built under QEMU, 2026-09-10, and the number the tour prints

**The instrument exists and the tour prints three figures instead of none.** The change is in
`kernel/src/main.rs`, where the step lives, and it uses the clock the tour already reads
(`arch::timer::now()`, the `time` CSR, whose rate came out of this machine's device tree). Nothing
new was introduced to read a clock with.

The three figures, and each is a different question:

- **`since the pcie line`**, which is the quantity a person with a stopwatch at the serial console
  was measuring: the whole gap between the two adjacent tour lines, console output and all. It is
  the one that makes the old measurement obsolete rather than merely finer.
- **`bring-up`**, the once-per-boot cost: the register window mapped, the driver spawned as an EL0
  process, its reseed, its first generation. **The `hw clock` line's own console time is excluded
  from it**, and that exclusion is load-bearing rather than fussy: a `println!` here is a polled
  UART, and on radon at 115200 baud that one line is roughly 30 ms of the kernel doing nothing but
  shift bits out, which is the same order as the bring-up it would otherwise be added to.
- **the draws**, in bytes and microseconds with a rate: the eight `entropy_proto` round trips two
  32-byte draws take.

**What the rate counts is stated at the function that computes it**, per this proposal's own reason
for existing (a number that leaves the machine has to say what is in it). It counts everything
between the readiness report and the last byte landing: the round trips, the context switches each
one costs, the driver's poll loop, and the device. It does not count the spawn or the bring-up.
**It is therefore not comparable to a Linux `hwrng` throughput figure**, which is a read from an
already-running in-kernel driver with no IPC in it at all.

### What QEMU measured, which is the IPC and not a device

QEMU's riscv64 `virt` has no JH7110, so the tour takes the skip arm there and always will. What it
can do is the **same client path with a different device at the end**: virtio-rng, the same
`entropy_proto`, the same `Wiring::fill`, the same eight round trips, the same confined userspace
process holding the same two rendezvous capabilities. So the skip arm now prints a **reference**
measurement when the machine has one, and says in the line itself that it is neither a TRNG nor
hardware.

Six boots, `NIFE_RNG=1 NIFE_SMP=4`, riscv64 debug build under QEMU TCG on patagonia:

| boot | since the pcie line | bring-up | 64 bytes in | bytes/s |
|---|---|---|---|---|
| 1 | 15154 us | 13036 us | 2060 us | 31067 |
| 2 | 12388 us | 10464 us | 1866 us | 34297 |
| 3 | 10198 us |  8069 us | 2067 us | 30962 |
| 4 | 12767 us | 10793 us | 1913 us | 33455 |
| 5 | 14913 us | 12893 us | 1957 us | 32703 |
| 6 | 15204 us | 13057 us | 2084 us | 30710 |

**The two halves behave completely differently, which is the finding rather than either number.**
The draws are tight (1866 to 2084 us, an 11% spread, about 250 us per round trip) and the bring-up
is not (8069 to 13057 us, a 62% spread). The bring-up is dominated by spawning a userspace process
out of the archive, which is scheduling work on four TCG harts and is exactly the kind of thing an
emulator is bad at reproducing. So the rate figure is the one worth carrying forward and the
bring-up figure is the one to distrust.

**None of this is a speed claim.** It is QEMU TCG on a laptop: no icount, four harts sharing a host
core, and a debug build. Its only use is as the denominator radon's number gets read against, which
is what the proposal asked for: with an emulated device that costs nothing, the path itself costs
about 250 us per 8-byte exchange, so whatever radon spends beyond that is the JH7110's.

For contrast, a boot with no virtio-rng at all prints `since the pcie line 2647 us`, which is the
device-tree query and the two console lines and nothing else.

### What is left, and it is one boot

**The `HARDWARE` half.** Follow milestone 159's bench procedure and read the `hw entropy` line: it
now carries the three figures above. Nothing else is needed, and the stopwatch that step 5 asks for
is no longer the instrument.

The comparison the original proposal named is still open and is a separate piece of work: Linux's
`jh7110-trng.c` on the same silicon, which is interrupt-driven where this driver polls, so a
like-for-like number needs both measured over the same thing. `notes/benchmarks.md`'s standard
applies and the tour's line already says where it is not apples-to-apples.

### One thing this found on the way

`NIFE_RNG=1` without `NIFE_DISK` panicked the kernel, on both riscv64 and aarch64, and had done
since virtio-rng was added. `-global virtio-mmio.force-legacy=false` lived inside the runners' disk
block, so **every mmio device was modern only when a disk happened to be attached**; the RNG came up
legacy (VERSION 1) and `virtio.rs`'s scan asserts 2. The whole test suite always builds disks, which
is why nothing met it. Fixed in both runners by hoisting the global. The latent twin on
`virtio-net-device` under `NIFE_NET` went with it.

The scan's own behaviour is a separate question and has its own proposal:
`design/roadmap/proposals/a-legacy-virtio-mmio-slot-panics-the-scan.md`.
