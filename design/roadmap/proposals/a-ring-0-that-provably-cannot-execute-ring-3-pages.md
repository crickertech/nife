# A ring 0 that provably cannot execute ring-3 pages, and SMAP with a number

**Status: PROPOSED 2026-09-17.** Raised by milestone 313's security audit (findings 3 and 6).

**Gate: NONE.** Nothing has to land first; the SMAP half owes a syscall-path measurement before it
ships, and that measurement is part of the work rather than a precondition on starting it.

## What is being proposed

Two halves, one lane, because they need the same new thing.

1. **A kernel test that ring 0 cannot fetch from a user page**, on all three architectures, with a
   falsification record each. On aarch64 the defect is a user page without `PXN`; on riscv64 there
   is no defect to write (the hardware refuses unconditionally, and the test documents that); on
   `x86_64` the defect is not setting `CR4.SMEP`, which was the tree's state until 2026-09-17.
2. **`CR4.SMAP` on `x86_64` and `PSTATE.PAN` on aarch64**, so a kernel bug that strays into the low
   half faults instead of succeeding quietly, with the syscall-path cost measured first, which is
   what `mmu::permit_kernel_access_to_user_pages`'s `BUGS` has asked for since it was written.

## Why

Milestone 313's audit found the claim "the kernel cannot execute a confined component's code"
stated nowhere, true on aarch64 and riscv64 by the encoders and the hardware, and false on `x86_64`
because nothing set SMEP. The bit is set now and a boot line says so, and that is rung three: a
setting with a transcript, not a claim with a test. A test would need ring 0 to take a page fault
on purpose and recover, and this kernel has no fault-recovery path for its own faults, which is also
the thing a SMAP test needs. Building that path once serves both.

## What it costs, honestly

The recovery path is the expensive half and it is a design question: a per-CPU "expected fault"
slot the trap handler consults before it panics is the small shape, and it has to be written so
that nothing but a test can arm it. SMAP itself is `stac`/`clac` around every kernel access to
user memory, of which there are few by design (the shared-page contracts carry no lengths and the
syscall surface moves words in registers), so the number may be small, but it is a number this tree
asks for before turning a protection on.
