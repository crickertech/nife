# 193. Put `kernel/src` within reach of the prover, because today the proofs cannot see it

**Status: NOT-STARTED.** Minted 2026-08-30 by calef, from milestone 191's (did the proofs catch the
bugs?) finding. *(Number provisional until the merge queue lands it.)*

**Gate: NONE.** Nothing here needs deciding to start. The scoping fork below is real and arrives
after the measurement rather than before it, which is the order that makes it answerable.

**In brief.** Milestone 191 asked whether the Kani harnesses had ever caught a defect and found that
none has, after the day it was written. The cause is not the harnesses. It is one line in
`script/verify`'s own header:

> `cargo kani -p <crate>` never compiles the kernel, the user programs, or xtask.

**So 64,818 lines of `kernel/src` are out of reach by construction, and that is exactly where every
concurrency, hardware-contract and resource-accounting defect in the corpus lived.** DECISIONS §14
(the demonstration OS) promises a verified core. Today the verified part is the pure crates and the
core is not in it.

## The distance is much shorter than anyone assumed, and this is measured

Run on 2026-08-30, `cargo check -p kernel --target aarch64-apple-darwin`, which is the host on the
development machine. **Three errors, all shallow:**

1. `unwinding panics are not supported without std`, which is a profile setting (`panic = "abort"`).
2. `invalid Mach-O section specifier` at `kernel/src/interrupt_stack.rs:96`, an
   `#[unsafe(link_section = ".interrupt_stacks")]` on a static.
3. The same at `kernel/src/smp.rs:71`, `.secondary_stacks`.

That is the whole list. **The kernel very nearly compiles for a host target already**, and the
reason is DECISIONS §4 rule 1 working exactly as designed: `asm!` appears outside
`kernel/src/arch/` in only two files and three sites (`cpu.rs` once, `user.rs` twice), and
`kernel/src/arch/` is 14,744 of the 64,818 lines, leaving roughly 50,000 lines of ordinary Rust.

**Take that as an encouraging measurement and not as a schedule.** `cargo check` is not `cargo
build`, neither is `cargo kani`, and the three errors above are the ones that surface first rather
than the ones that are hardest.

## What is actually going to be hard, named rather than discovered

- **The host in CI is not the host here.** This machine is aarch64, so the `aarch64-cpu` dependency
  compiled. CI is x86_64 Linux and it very likely will not, which probably forces that dependency
  behind a target `cfg`. Check this early; it can change the shape of everything else.
- **Kani cannot model `asm!`.** Three sites outside `arch/` and all of `arch/` are unreachable to
  the prover no matter what compiles. The answer is a stub boundary, not a rewrite, and where that
  boundary sits is the interesting design question in this milestone.
- **MMIO and fixed addresses.** A raw pointer to a device register is not something CBMC can reason
  about, and the same stub argument applies.
- **CBMC does not scale to 50,000 lines wholesale.** Nobody should imagine a proof over the kernel.
  The deliverable is *reachability*: harnesses can be written against the kernel's own code, one
  property at a time, the way they already are for the crates.
- **`script/verify` already costs about 42 minutes.** Anything added here lands on that budget, and
  the `--affected-since` machinery exists precisely because it is expensive.

## The fork, which the measurement should decide rather than precede

**Option A: make the `kernel` crate itself Kani-reachable**, with the arch layer stubbed. Direct,
and it puts the prover where the defects are.

**Option B: keep lifting pure logic into host-compilable crates**, which is what AGENTS.md already
prescribes (*"Pure logic ... belongs in crates that compile for the host"*) and what the tree has
been doing incrementally. Milestone 191's own worklist has an instance: lifting `login`'s cspace
bookkeeping into a crate answers two recorded leaks.

**These are not exclusive and the honest answer is probably both**, with the split decided by where
a property naturally lives. A scheduling policy wants to be a crate. `syscall.rs`'s dispatch does
not, and never will be one.

## The proof that this milestone worked

**One property, over code that lives in `kernel/src` today, proved by a harness `script/verify`
runs.** Not a survey, not a plan, and not a proof of something that was moved into a crate first,
because that would be option B wearing this milestone's name.

The candidate worth trying first is milestone 191's sharpest counterfactual: the milestone 6 timer
re-arm drift (100 Hz configured, ~70 Hz delivered) has its property **already proved** in
`crates/timetable`'s `next_after`, over already-written code, and the timer does not call it. That
is a case where the property, the proof and the defect all already exist and only the reachability
is missing.

## BUGS

- **This block does not price the work**, and the three-error measurement above is the kind of
  number that invites a bad estimate. What was measured is that the front door opens, not that the
  building is empty.
- **It says nothing about the user programs or `xtask`**, which the same `script/verify` line puts
  out of reach for the same reason. `user/` holds real parsers over untrusted input and has at least
  as good a claim on the prover as the kernel does.
- **A stub is a hole in a proof, and a proof with an unexamined stub is worse than no proof**,
  because it reads as coverage. Whatever boundary this lands on needs the stubs enumerated where a
  reader meets the harness.
- **`kernel/src/arch/` stays unreachable under every option here**, which means the architecture
  layer, where the VisionFive 2's undelivered-wake defect actually lived, is not what this fixes.
  Saying so plainly is what keeps this milestone from being quoted as more than it is.
