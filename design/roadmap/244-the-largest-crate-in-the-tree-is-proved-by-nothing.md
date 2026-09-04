# 244. The largest crate in the tree is proved by nothing a mutation can reach

**Status: NOT-STARTED.** Minted 2026-09-03 by calef, from milestone 238's (the scheduled checks that
never run) first published mutation report. *(Number provisional until the merge queue lands it.)*

**Gate: NONE.** Nothing is missing; what is missing is a way for a test to reach the code.

**In brief.** `crates/system_initializer` is **2,632 lines with zero `#[test]`**, and milestone 238's
report scored it **0 caught of 191 mutants**. Every mutation of every function in it survives.

**That is not negligence, and reading it as negligence would produce the wrong lane.** `script/lint`
excludes the crate from the host pass on purpose, and says why beside the exclusion: it takes an
unconditional dependency on `user_rt`, which is EL0 `asm!`, so it cannot compile for the host at all.
Four crates are in that position (`supervision_proto`, `swap_proto`, `virtio`, and this one) and this
is much the largest. `cargo mutants` builds the crate and then runs a workspace suite that
structurally cannot reach it, so the zero is arithmetic rather than a discovery.

The crate's own head comment already names what proves it instead, honestly:

> the thing that actually proves this code is `script/shell-check`, which boots both ISAs and types
> at the prompt.

**That gate is real and it is not nothing**, which is the fact that makes this milestone a judgement
rather than an alarm. `script/shell-check` is the only thing in the tree that runs a real init, and
milestone 96 exists because a fix landing in one init and not the other produced *a boot that reaches
userspace and prints nothing at all*, with no fault and no message, three times. What it cannot do is
tell which of 191 mutations it would have caught, and a gate whose coverage nobody can state is a
gate nobody can improve.

## What this milestone is, in one sentence

**AGENTS.md's own rule, applied to the largest place the tree breaks it:**

> Pure logic (allocator algorithms, page-table math, scheduling policy, filesystem parsing) belongs
> in crates that compile for the **host**, so most tests run in milliseconds without an emulator.

This is milestone 193's (put `kernel/src` within reach of the prover) **option B** with a name and a
number. 193 chose option A for the kernel and said the honest answer is probably both, with the split
decided by where a property naturally lives. This block is that sentence cashed out for the one crate
where the cost of not doing it is measured rather than argued.

## What is actually in there, because the split is the whole question

The crate holds two kinds of code and they are not mixed evenly:

- **Logic with a right answer that a host can check.** Reading the archive, decoding a `grant_plan`
  off the spawn channel, checking a `measured_boot` manifest against what it is about to load,
  choosing addresses to map an image at. All of this is `no_std` arithmetic and parsing over bytes.
- **Syscalls on capabilities the kernel granted at spawn.** `boot` returns `!`, and every step it
  takes is an `svc`. There is nothing to assert and nowhere to assert it.

**The deliverable is the first kind moved somewhere a test can reach, not the second kind
simulated.** A mock kernel is the failure mode to avoid here: it would produce a large green suite
proving that the mock behaves the way the code expects, which is the thing already assumed.

## The proof that this milestone worked

**A mutation run over the new host-reachable crate catches most of what it generates**, reported the
way milestone 238's does, plus the number that shows the split was worth making: how many of
`system_initializer`'s 191 mutants now live in code a host test can reach.

Not a line count moved, and not a test count. Either of those is satisfiable by moving the easy half.

## What would make this milestone wrong

Worth stating in advance, because a lane that finds it should say so rather than build anyway:

**If the pure fraction turns out to be small.** The crate may be mostly syscall sequencing with
arithmetic threaded through it, in which case lifting it produces a crate of fragments, a wider
public surface, and a reader who now has to hold two files. That is a worse tree than a 2,632-line
crate with an honest note saying `script/shell-check` is what proves it. **Measure the fraction
before moving anything**, and if it is small, say so and stop; the finding is worth more than the
lane.

## BUGS

- **This does not close the other three excluded crates.** `supervision_proto`, `swap_proto` and
  `virtio` are excluded for the same reason and are not in this block. Whether the same argument
  applies to them is not answered here.
- **A mutation score over the lifted crate is not a claim about the init.** What stays behind stays
  unreachable, and the fraction that stays is exactly what this block asks a lane to measure and
  report rather than assume.
- **`script/shell-check` remains the only thing that runs a real init**, and nothing here changes
  that or should be read as reducing its standing.
