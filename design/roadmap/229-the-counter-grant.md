# 229. Build the cycle-counter grant DECISIONS 139 decided

**Status: NOT-STARTED.** Minted 2026-09-02 by the maintainer, the moment calef finished answering
DECISIONS 139 (who may read the cycle counter, and by what authority). *(Number provisional until
the merge queue lands it.)*

**Gate: NONE.** The decision is made, the default is now known rather than assumed (milestone 228,
the cycle counters are closed by assumption), and nothing here needs a board.

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

## BUGS

- **Nothing verifies the granted state on silicon.** QEMU's `PMUSERENR_EL0` is almost certainly zero
  either way, which is the same blindness milestone 228 recorded, so a green run says little. The
  bench item belongs beside 127's existing `PMCCNTR_EL0` one.
- **The riscv64 half is `scounteren.CY` and is not symmetrical with aarch64's**, because one CSR
  carries both the coarse and fine permissions there while aarch64 splits them across two registers.
  Whether that asymmetry deserves a shared abstraction or two honest implementations is a real
  question this block does not answer.
- **No consumer exists yet.** Milestone 74's aarch64 half is the first, and until it lands this
  grant is a mechanism nothing asks for, which is worth saying plainly rather than discovering.
