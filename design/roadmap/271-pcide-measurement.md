# 271. Measure `CR4.PCIDE`, and settle the ASID-tagging skip it leaves on x86_64

**Status: NOT-STARTED.** Minted 2026-09-10 by calef, from the live skip inventory. Raised as
**Outstanding** in milestone 161 (the x86_64 kernel port), checked 2026-09-03 and still
unmeasured, and referenced but not answered by milestone 186 (derive the architecture list),
whose only claim on it is that the bench-tooling caller for the x86 baseline is one of its eleven
silent gaps. *(Number provisional until the merge queue lands it.)*

**Gate: DECISION.** Whether to turn `CR4.PCIDE` on is calef's call once the cost is measured; this
milestone's job is to produce the measurement the decision needs, per the seven-questions rule
(*"what does each option cost, measured rather than asserted"*).

## What is skipping, and why it is not a bug

`kernel/src/user/tests.rs:508` skips its ASID-tagging assertion on x86_64, because the kernel runs
with `CR4.PCIDE` clear: **PCID is `CR3[11:0]`, and with `PCIDE` clear those bits are reserved-zero**,
so `arch::x86_64::mmu::ttbr0_value` drops the tag and every `mov cr3` flushes the whole TLB. The test
would pass, but for the wrong reason (nothing was cached, not because tagging works), so it skips
rather than gives a false positive. `kernel/src/arch/x86_64/mmu.rs`'s own `BUGS` records the same
gap: both `PCIDE` and `CR4.PGE` are off with nothing measured against them.

This is a real architectural divergence under §19, not a scope note: aarch64 and riscv64 both tag
address spaces on every context switch today, and x86_64 does not.

## What this needs

1. **Turn `CR4.PCIDE` on** (and, since `script/icount` already only takes the other two
   architectures per the same `BUGS` entry, `CR4.PGE` alongside it if the two are entangled) in a
   throwaway or feature-gated build.
2. **Measure the cost**, the same way milestone 134's register measures everything else: a context-
   switch benchmark with and without PCID tagging, on the same machine, same methodology as E1.
   `script/icount` should take a third leg if the instrument allows it.
3. **Bring the number to calef** as a DECISIONS section, priced rather than argued, with the recorded
   ASID test as the thing that flips from skip to pass if the answer is yes.

## BUGS

- **Nothing here prices the risk side.** PCID reuse across processes needs INVPCID or careful
  reuse-of-tag bookkeeping; this block does not yet name what breaks if tags are recycled wrong,
  which the measurement work should surface rather than assume away.

## Follow-on

- **Milestone 186.** Once this lands, 186's own list of eleven gaps loses the "bench tooling item 3
  has no caller" entry for the x86 leg, since the measurement this milestone produces is that caller.
