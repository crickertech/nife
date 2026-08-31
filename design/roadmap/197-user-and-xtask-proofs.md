# 197. `user/` and `xtask` are out of reach of the prover, for exactly the reason the kernel was

**Status: NOT-STARTED.** Minted 2026-08-30 from milestone 193's (put `kernel/src` within reach of the
prover) lane. *(Number provisional until the merge queue lands it.)*

**Gate: NONE.** The `user/` half needs nothing that does not exist, milestone 193 having established
the mechanics. The timer seam further down is a genuine fork and is marked as one there rather than
gating this whole block on it.

**In brief.** `script/verify`'s header names three things `cargo kani` never compiles: the kernel, the
user programs, and xtask. Milestone 193 removed the first for about ten seconds of run time. The
other two are unchanged, and **`user/` has at least as good a claim on the prover as the kernel did**,
because it holds real parsers over bytes this system did not produce.

## Why `user/` first

`notes/untrusted-input-audit.md` already surveys that surface. A parser over attacker-supplied input
is the case bounded model checking is best at and the case where a defect is worst, which is the
combination that made `dtb::be32`'s unchecked `at + 4` worth catching.

The mechanics should be milestone 193's, and cheaply: `#[cfg(not(kani))]` on what Kani duplicates,
`--ignore-global-asm` where needed, and the stub boundary enumerated where the next author meets it.

## Two smaller things the same lane found

**The timer re-arm seam, and it is a fork rather than work.** Milestone 191 (did the proofs catch the
bugs?) named the milestone 6 timer drift as the sharpest counterfactual in the tree: its property is
already proved in `crates/timetable`'s `next_after`, over already-written code, and the timer does
not call it. Milestone 193's lane could not use it, because `rearm` lives in
`kernel/src/arch/{aarch64,riscv64}/timer.rs` and reads the counter through `asm!`. Lifting the
arithmetic out of the register access would fix that, and **where the seam goes is a design question**:
too high and the arch layer keeps the bug, too low and every ISA restates it.

**`crates/timetable` is in the verify table and the kernel does not depend on it**, which is the other
half of the same observation and is untouched.

## BUGS

- **`xtask` is named here and not argued for.** It is host code with a test suite already, so the
  case for proving it is weaker than for `user/`, and this block does not make it.
- **Nothing here estimates the run-time cost.** The kernel's two harnesses cost ten seconds; a parser
  over symbolic input can cost far more, and `script/verify` is already about 650 seconds.
