# 229. Build the cycle-counter grant DECISIONS 139 decided

**Status: BUILT 2026-09-02.** Minted 2026-09-02 by the maintainer, the moment calef finished answering
DECISIONS 139 (who may read the cycle counter, and by what authority). *(Number provisional until
the merge queue lands it.)*

It was minted `Gate: NONE`, and that held: DECISIONS 139 was already answered, the default was made
a fact rather than an assumption by milestone 228 (the cycle counters are closed by assumption), and
nothing here needed a board. What only a board can settle is in `BUGS` below.

**In brief.** DECISIONS 139 is `DECIDED` in three parts, and this milestone is the first two of them:

- **Option 4**, a per-thread grant enforced at the context switch. A granted thread runs with the
  counter open, every other thread runs with it closed, and the kernel writes the enable on the
  switch the way it already writes the address-space root.
- **The grant is a field in the spawn manifest**, not a method on a live thread, so a program cannot
  acquire a timing side channel it was not given at creation. DECISIONS §28 chose the same shape for
  thread placement at ratification.
- **`x86_64` is not part of this.** It keeps its ambient counter, for reasons 139 measured rather
  than assumed, and §19 (architectural parity is a tenet) should read that as a stated exception and
  not a gap.

## Why the cost is known before the work starts

139 priced it: **one comparison, and one `msr` only when the value changes.** That is exactly the
shape `kernel/src/arch/aarch64/mmu.rs`'s `switch_user_root` already has, which early-returns when
`TTBR0_EL1` already holds the wanted value. If no thread is granted, the value never changes and the
whole cost is a compare.

**`script/fastpath-footprint` has a 5% bound and it fired on a lane today** (milestone 221's soak
counters grew the aarch64 IPC path 5.7%). The switch is not the IPC fastpath, but it is adjacent
enough that the gate should be run rather than assumed.

## What it is not

**It is not milestone 147** (a profiler that holds exactly the counters it was granted). This says
*this thread may read the counter*, not *this profiler may read that subtree's counters*. 147 wants
cross-thread authority with a named target, which neither this nor any option in 139 provides, and
139 records that explicitly so nobody reads this as having built it.

**It does not buy timing confinement**, and `notes/confinement-claims.md` now carries the row saying
so: two threads and a shared word reconstruct a 6.8 ns clock on any of the three architectures. What
this buys is accountable authority, meaning the cheap accurate path is granted rather than ambient
and the kernel knows which threads hold it. Anything this milestone prints or documents must not
imply more.

## What was built

2026-09-02. Four pieces, and the third is the one that had to be measured rather than argued.

**The grant is a bool on the TCB, set through a new `ThreadControlBlock` method.**
`abi::thread_control_block::GRANT_CYCLE_COUNTER` (method 3), `WRITE`-gated like its three
siblings, and it **refuses a thread that is not an embryo**, which is the whole security property
rather than housekeeping: `CONFIGURE` and `CAP_INSERT` make the same refusal, so the set of things
you can do to a TCB before it runs is exactly the spawn manifest DECISIONS 139 part 2 asked for,
and there is no later. One-way: an embryo starts closed and nothing but this opens it.

**A new method rather than a field on `CONFIGURE`, and that was a choice a lane made in a space
139 marked as calef's.** 139's recommendation section listed three shapes for expressing the grant
(a field on TCB configure, a new spawn input, a badge on an existing capability) and said a lane
should not pick one; part 2 of the answer then settled the half that matters, which is that it is
set at creation and not on a live thread. What remained was mechanical and had one answer:
`invoke` takes three argument registers, `CONFIGURE` already spends all three (entry, user stack,
address-space slot), and `START` spends all three on the child's first registers. So the field had
nowhere to go without widening `invoke` itself, which is a syscall-surface change of a much larger
kind. A new method inside the established object model is what AGENTS.md and 139 both already
permit (`MemoryRegion::SPLIT` and `DESTROY` arrived this way). **The method number, the method
name and `Thread::cycle_counter_grant` are all provisional.**

**The switch writes it beside the address-space root.** `sched::schedule` reads the incoming
thread's bool out of the same locked block that reads its `ttbr0`, and calls
`arch::timer::set_cycle_counter_grant` immediately after `mmu::switch_user_root`, which is the
`adopt_address_space` shape one register over. The arch call compares before it writes, so a
machine where nothing is granted never writes the register at all.

**Two honest implementations, not a shared abstraction**, which is the question the block below
left open. The two registers disagree on both halves, and each disagreement decides one
implementation:

- **aarch64 caches what it last wrote.** `PMUSERENR_EL0` does not exist without FEAT_PMUv3, so
  "read back what the hardware holds" (which is exactly how `switch_user_root` compares
  `TTBR0_EL1`) is not available on the architecture. A per-core `AtomicBool`, plus a per-core
  record of whether the register exists at all, which `timer::init` already had to work out for
  milestone 228's write. The grant is `CR` (bit 2) alone: `EN` would open every PMU register and
  `ER` the event counters, and 139 granted neither.
- **riscv64 reads `scounteren` back and changes one bit.** The CSR always exists, so the read is
  available; and it is *required*, because `TM` shares the word, is open for every thread by
  design, and `crates/user_rt`'s `now()` breaks if it is ever cleared. A cached implementation
  would have had to model `TM` too, which is the more moving parts, not fewer.
- **`x86_64` is an `#[inline(always)]` empty function.** 139 part 3, and the measurement below
  says it costs literally nothing.

One call site, three implementations behind one name, which is what `switch_user_root` itself
already is. A trait would have added a vocabulary without removing a difference.

### What it costs, measured

`script/fastpath-footprint`, this branch against `8a418981` built in a separate worktree:

| ISA | `ipc_fastpath` | `syscall_entry` |
|---|---|---|
| aarch64 | 5852 -> 5988 (+136 bytes, +2.3%) | 3304 -> 3340 (+36) |
| riscv64 | 5132 -> 5148 (+16 bytes, +0.3%) | 1870 -> 1898 (+28) |
| `x86_64` | 6687 -> 6687 (**unchanged**) | 1637 -> 1637 (**unchanged**) |

The gate is green on all three. **`x86_64` is byte-identical**, and that is the no-op compiling to
nothing rather than a coincidence, which is the closest thing to the "an ungranted kernel is
unchanged" claim that is actually true: on the other two the mechanism has to exist in the image,
and what an ungranted thread pays at runtime is a load, a compare and a return.

**The gate fired once, and the fix is recorded because the number is the argument.** Written as an
ordinary arm inside `invoke`, the new method put **224 bytes** on riscv64's `syscall_entry` set,
12% against the 5% bound, because `invoke` inlines into the dispatcher every syscall fetches. The
arm was extracted into an `#[inline(never)]` helper exactly as milestone 156 extracted the other
spawn-path bodies, and it lands at +1.5%. A grant a loader calls once per child does not belong in
the bytes a round trip fetches.

### How it was verified

**`script/test` green on all three architectures**, with three new tests.

- **A live thread cannot be granted the counter** (`sched::tests`). It asks on its own behalf,
  which is the strongest available form of the question: the caller is `Running` by definition of
  executing the line, and holds every authority a kernel thread has. `WrongObject`, the same
  refusal `CONFIGURE` makes on a started thread.
- **The write is idempotent and does not disturb its neighbours** (`sched::tests`). Four calls
  where two change anything, then on riscv64 the assertion that carries this milestone's asymmetry
  argument: `scounteren.CY` is clear and `scounteren.TM` is still set.
- **A granted EL0 child reads the counter** (`user::tests`). init builds a child, grants the
  embryo, starts it, and the child reads `PMCCNTR_EL0` (aarch64) or the `cycle` CSR (riscv64) and
  reports. **The report arriving is the whole assertion**: milestone 228 closed both registers at
  init on every core, so without the grant that read traps and the thread dies with nothing sent.
  The counter values are carried and not checked, because QEMU leaves `PMCR_EL0.E` clear and
  `PMCCNTR_EL0` reads zero there forever; asserting on the number would be asserting on the
  emulator.

The `x86_64` run passes the same EL0 test for a reason it should not be read as evidence: `rdtsc`
is ambient there, so the child would have reported with or without the grant.

## BUGS

- **Nothing verifies the granted state on silicon.** QEMU's `PMUSERENR_EL0` is almost certainly zero
  either way, which is the same blindness milestone 228 recorded, so a green run says little. The
  bench item belongs beside 127's existing `PMCCNTR_EL0` one.
- **The riscv64 half is `scounteren.CY` and is not symmetrical with aarch64's**, because one CSR
  carries both the coarse and fine permissions there while aarch64 splits them across two registers.
  Answered above, with the reason: two implementations behind one call site, because the registers
  disagree about whether they can be read back and about whether anything else lives in the word.

- **Nothing checks that an *ungranted* thread is refused.** The EL0 test proves the open side; the
  closed side would need the fault caught and reported rather than ending the run, which is the
  supervision path and a bigger fixture than this earned. The evidence for the closed side stays
  milestone 228's, which is the register written to zero at init on every core with the write
  confirmed by disassembly.

- **The method number and every name here are provisional**, including
  `GRANT_CYCLE_COUNTER`, `Thread::cycle_counter_grant`,
  `arch::timer::set_cycle_counter_grant` and `cycle_counter_grantable`. Names are calef's, and this
  one is on the syscall surface, which is the expensive category.
- **No consumer exists yet, and that is still true after this.** The only caller of the grant in
  the tree is `hello`'s own test role. Milestone 74 (cycle counters) is the first real consumer, and
  it is also the owner of a portable userspace read: the raw `mrs`/`csrr` here lives in the one test
  program that needs it rather than in `crates/user_rt`, so that 74 designs that API instead of
  inheriting one from a test vehicle.
