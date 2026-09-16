# `cycles_per_tick` is riscv64-only, so two of three architectures cannot convert a board bench to cycles

**Status: PROPOSED 2026-09-16.** Written by the maintainer, from calef asking why milestone 74 is
half riscv64 when this project has three architectures. The honest answer was that the milestone has
two halves with different parity answers, and only one of them had a reason.

**Gate: NONE.** Both counters already exist and are already read by this kernel for other purposes.
This is a probe, not a driver.

**In brief.** `kernel/src/bench.rs`'s `cycles_per_tick` is `#[cfg(target_arch = "riscv64")]`. It
spins for a fixed window of timer ticks, reads a cycle counter at each end, and prints the ratio.
That ratio is what converts every tick-denominated row of a board bench into cycles and wall time,
and on 2026-09-16 it turned radon's `bench:` rows into this project's first real-silicon cost table
(`ipc_rtt` 1,046 cycles, `call_reply` 1,256, `ctx_switch` 1,980). **aarch64 and x86_64 cannot
produce that table**, so a three-architecture comparison cannot be made from board benches at all.

## Why this is the parity gap that matters

[DECISIONS §139 part 3](../../decisions/139-cycle-counter-authority.md) already settles the
*capability* question asymmetrically and correctly: x86_64's TSC is ambient (`CR4.TSD` clear at
reset, never written), so there is no grant to test and the negative half of
`a_granted_thread_reads_the_cycle_counter_and_an_ungranted_one_faults` skips there with a reason.
That is a recorded exception, not a hole.

**Measurement inherits none of that.** Reading a counter to compute a ratio needs no authority
decision, because the kernel is reading its own counter in its own bench build. So the reason the
capability half is two ISAs does not extend to the measurement half, and milestone 74's block said
otherwise by omission until this proposal was written.

It is also the half with a consumer waiting: milestone 25's cross-OS comparison and
[§96](../../decisions/96-process-kernel-or-event-kernel.md)'s event-kernel question both want cycles,
and milestone 168's job-mix sweep reports ticks on every architecture it runs on.

## What each one needs

- **x86_64: `rdtsc`, already in the tree.** `kernel/src/arch/x86_64/timer.rs` reads it for the
  calibration this kernel does at boot, including the `rdtscp` serialisation caveat in its own doc.
  The probe is the same shape as riscv64's with `arch::pmu::cycles()` replaced by that read.
  **The caveat to state rather than discover:** the TSC is not a cycle counter on modern parts, it
  is a constant-rate counter (invariant TSC), so on x86_64 the ratio measures TSC ticks against
  timer ticks and **is not cycles per tick** in the sense riscv64's is. That is a real difference in
  what the number means and it has to be in the printed line, not only in a note, or the three
  architectures' outputs will be compared as though they were the same quantity. Whether to use a
  performance counter instead (`IA32_PERF_FIXED_CTR1`, actual unhalted core cycles) is the design
  fork inside this option, and it is the one that would make x86_64's number mean what riscv64's
  means.
- **aarch64: `PMCCNTR_EL0`, and it waits on this milestone's own aarch64 half.** That counter reads
  zero until `PMCR_EL0.E` and `PMCNTENSET_EL0.C` are written, which
  `design/roadmap/proposals/the-aarch64-half-of-74.md` covers. So this proposal's aarch64 part is
  ordered behind that one rather than independent of it.

## What it does not need

**No board time to build, and board time to be worth anything.** The probe compiles and runs under
QEMU on all three, and under emulation the ratio is an artifact by construction (milestone 16a's
"implausibly exact 100.00"). So the deliverable is the probe plus a boot on argon and on xenon, and
neither of those machines has produced a bench boot yet: milestone 87's OptiPlex has never booted
nife at all, and milestone 127's Jetson is unbuilt. **This proposal is therefore worth building
before those boards are ready, so that the first boot of each produces the number rather than
discovering the probe is missing.**

## The recommendation

**Build the x86_64 probe now with the TSC and label the line for what it measures**, and order the
aarch64 probe behind the aarch64 half of 74. The labelling is the part that matters and the part a
lane should not decide alone: a rate printed as `cycles_per_tick` on a machine where it is
TSC-ticks-per-timer-tick is exactly the kind of figure that gets quoted across architectures and
cannot be un-quoted, which is AGENTS.md's *facts that leave the machine* test. Whether x86_64 should
instead read `IA32_PERF_FIXED_CTR1` so the three numbers are the same quantity is calef's call.

**Blocked until it is answered:** nothing. Milestone 74 can close its aarch64 half without this, and
riscv64 already has its number. What waits is any cross-architecture reading of a board bench.
