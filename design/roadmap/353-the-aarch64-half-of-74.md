# 353. The aarch64 half of milestone 74: `PMCCNTR_EL0` counts nothing, because nobody starts it

**Status: NOT-STARTED.** Filed 2026-09-03 as an unnumbered proposal by the
`milestone/74-cycle-counters-riscv` lane, which built 74's riscv64 half and was scoped out of this
one; numbered 2026-09-19 by milestone 433. **Premise re-checked 2026-09-19 and the defect is
unchanged.** `PMCR_EL0` and `PMCNTENSET_EL0` still appear nowhere in this kernel except in comments
saying they are not written, so `PMCCNTR_EL0` is still a stopped counter, and
`kernel/src/bench.rs:77` still carries that fact as the reason a cycle number cannot be quoted.
**One detail below has moved**: milestone 291 split `fixtures/src/hello.rs` into programs on
2026-09-14, so the EL0 reader this block calls `hello`'s `cycle_counter_child` is now
`fixtures/src/cycle_counter_reader.rs`, and the "QEMU leaves `PMCR_EL0.E` clear" comment the proposal
cites at `fixtures/src/hello.rs:358` is in that file and in `crates/capability_witness_protocol`.
`kernel/src/user/tests.rs:2609` still carries the same note. The sizing is otherwise as written.

**Gate: DECISION.** Not the authority question, which is answered: `design/decisions/139-cycle-counter-authority.md`
chose option 4 and the mechanism is built and tested on all three architectures. What is owed is
smaller and is still calef's, because it is a public API and a published number: what
`user_mode_runtime`'s cycle-counter function is called and what it promises, and whether the counter counts in
EL1 and EL2 as well as EL0.

**In brief.** `PMCR_EL0.E` and `PMCNTENSET_EL0.C` are never written by this kernel, so
`PMCCNTR_EL0` is a stopped counter. A thread holding the milestone 229 grant can read it, legally,
and gets the same number every time. Turning it on is a handful of register writes, and the
milestone is what goes around them.

## What is already built, so this is sized honestly

Everything the riscv64 half needed from the authority side, aarch64 already has, and the parts
below were read in the tree on 2026-09-03 rather than recalled.

- **The per-thread grant exists and is enforced at the context switch.**
  `kernel/src/sched.rs`'s `install_cycle_counter_grant` (near line 1734) calls
  `arch::timer::set_cycle_counter_grant` on every switch, behind the `cycle_counter_grant` feature
  that milestone 237 introduced when it measured what the always-on version cost the IPC fastpath.
- **The aarch64 register work is done and is careful about the part that has no PMU.**
  `kernel/src/arch/aarch64/timer.rs` writes `PMUSERENR_EL0` per core at init (milestone 228, so the
  architecturally UNKNOWN reset value is not inherited), gates every access on
  `ID_AA64DFR0_EL1.PMUVer` because the register is UNDEFINED without `FEAT_PMUv3`, and caches what
  it last wrote rather than reading back for the same reason.
- **The EL0 read is proven, including the negative half.**
  `kernel::user::tests::a_granted_thread_reads_the_cycle_counter_and_an_ungranted_one_faults` passes
  on aarch64, riscv64 and `x86_64`, and `fixtures/src/cycle_counter_reader.rs` is the EL0 side
  (`hello`'s `cycle_counter_child` until milestone 291 split it out on 2026-09-14): one `mrs` from
  `PMCCNTR_EL0`, with an ungranted thread faulting rather than reading.

So milestone 75's mechanism is not what stands in the way, and a plan that assumed it was would be
sizing the wrong work.

## What is missing, and it is the same defect the riscv64 half found the hard way

**The counter is not running.** `PMCR_EL0` and `PMCNTENSET_EL0` appear nowhere in this kernel except
in comments explaining that they are not written. Two of those comments already say what the
consequence is: `kernel/src/user/tests.rs` and `fixtures/src/cycle_counter_reader.rs` both record
that QEMU leaves `PMCR_EL0.E` clear, so `PMCCNTR_EL0` reads zero however many times you read it, and both
tests deliberately carry the values without checking them.

That is exactly the hazard the riscv64 half met on QEMU's `rva23s64` model and now defends against:
firmware handed back a counter that `counter_get_info` described as a working 64-bit hardware
counter and that read zero forever. A benchmark reporting 0 cycles for everything is worse than one
that says there is no cycle counter, so `arch::riscv64::pmu::init` reads the counter twice across a
timed spin and refuses it if it has not moved, and records **why** as a value the boot line prints.
**The aarch64 half wants the same shape**, and it wants it more, because on aarch64 there is no
firmware call to blame: the kernel is the thing that failed to start the counter.

## What the work is

- Enable the counter: `PMCR_EL0.E` (bit 0) and `PMCNTENSET_EL0.C` (bit 31), per core, gated on
  `ID_AA64DFR0_EL1.PMUVer` the way the existing writes already are.
- Decide `PMCCFILTR_EL0`. Its `P` and `NSH` bits say whether EL1 and EL2 cycles are counted, and the
  answer changes what a published number means: a count that excludes the kernel is not comparable
  to seL4's, and one that includes it is not comparable to a userspace-only profile. **This is the
  decision, and it is calef's**, because it is a fact that leaves the machine.
- Verify it is counting, and record why not when it is not, in the shape
  `arch::riscv64::pmu::CycleCounter` established.
- The portable read. `fixtures/src/cycle_counter_reader.rs`'s `read_cycle_counter` is deliberately
  *not* in `crates/user_mode_runtime`, and its own comment says why: *"A portable userspace cycle-counter API is
  milestone 74's deliverable, and it will want to say what the number means."* That is the naming
  and semantics question, and it is the second thing calef owes here.
- The harness probe, matching the riscv64 half: one `bench-probe: cycles_per_tick` line, which
  converts every tick-denominated row in `bench/baseline-aarch64.txt` at once.

## Why it matters

**§19 (architectural parity) is a gate, and this is a gap in the one subsystem whose entire purpose
is cross-machine comparison.** The riscv64 half can now say what a tick costs in cycles and the
aarch64 half cannot, which is the wrong way round for the ISA that runs on the machine every merge
uses.

And it is the half milestone 25's `sel4bench` needs. seL4's published 413 and 426 are single-shot
PMU measurements taken from user level on ARM; reproducing that instrument needs `PMCCNTR_EL0`
running and readable, which is this proposal and nothing else now that the grant exists.

## What it cannot do here

**Nothing can be measured on Apple silicon.** notes/pmu.md's last section is why: the generic timer
is architected state a hypervisor must present and the PMU is not, so neither QEMU-TCG nor HVF
offers a real cycle counter. A green test under either proves the register writes and says nothing
about the number, which is the same honest limit the riscv64 half records. The machine that would
settle it is argon, the Jetson TX1, with a person at it.

## Where it came from

The `milestone/74-cycle-counters-riscv` lane, which was scoped to the riscv64 half on the ground
that milestone 75 gated the aarch64 one. That gating turned out to be a stale index row rather than
missing work, and the lane was told mid-flight not to expand into it. This file is that handoff, so
the sizing above does not have to be re-derived by whoever takes it.

## Index row

`PMCR_EL0.E` and `PMCNTENSET_EL0.C` are never written by this kernel, so `PMCCNTR_EL0` is a
stopped counter: a thread holding the milestone 229 grant can read it, legally, and gets the same
number every time. Turning it on is a handful of register writes gated on `ID_AA64DFR0_EL1.PMUVer`,
and the milestone is what goes around them. Everything the riscv64 half needed from the authority
side aarch64 already has: the per-thread grant is installed at every context switch, the register
work is careful about parts with no PMU, and the EL0 read is proven on all three architectures
including the ungranted thread faulting. What is missing is the same defect the riscv64 half met the
hard way on QEMU's `rva23s64` model, where firmware described a working 64-bit counter that read zero
forever, so this wants the same shape: verify the counter is moving and record why not when it is
not. Two things here are calef's, and both are facts that leave the machine: what
`user_mode_runtime`'s cycle-counter function is called and what it promises, and whether
`PMCCFILTR_EL0` counts EL1 and EL2, because a count that excludes the kernel is not comparable to
seL4's and one that includes it is not comparable to a userspace-only profile. §19 makes this a
parity gap in the one subsystem whose entire purpose is cross-machine comparison, and milestone 25's
`sel4bench` needs it. Nothing can be settled on Apple silicon: the PMU is not architected state a
hypervisor must present, so the machine that decides it is argon, with a person at it.
