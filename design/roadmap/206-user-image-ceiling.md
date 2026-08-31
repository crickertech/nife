# 206. A program image has under 896 KiB, and the failure names an overlap rather than a size

**Status: NOT-STARTED.** Minted 2026-08-31 from milestone 121's (`ripgrep`: enumeration as a
capability) lane, which hit it the hard way. *(Number provisional until the merge queue lands it.)*

**Gate: DECISION.** `USER_STACK_VA` is a protocol constant in two `_proto` crates, so moving it is a
change two programs agree on, which AGENTS.md puts in the expensive category.

**In brief.** `user/link.ld` links a program at `0x40_0000`, `USER_STACK_VA` is `0x50_0000`, and 32
std stack pages sit below it. **So a program image has under 896 KiB.** `ripgrep`'s `.text` alone is
1.37 MiB.

**The failure is `Unmappable(AlreadyMapped)`**, which names an overlap and not a size, so nobody
hitting it would learn what was wrong. Milestone 121's lane worked around it by relinking at
`0x100_0000`, derived from `user/link.ld` by substitution so the two cannot drift.

## Why it is not a one-line change

`USER_STACK_VA` appears in `supervision_proto`, `timebase_proto`, `c_seam`, `user/src/builder.rs` and
several tests. **Measured by that lane: moving it alone breaks `authority_tests` at stage 10.** It is
a layout two programs agree on rather than a private kernel detail.

## What this is really about

**896 KiB is small for anything with a dependency tree**, and the corpus milestone 123 (the
demonstration: somebody else's software, running narrow) wants is made of such things. This ceiling
is the second thing a foreign program meets, right after milestone 205 (how a foreign program is told
what to do).

## BUGS

- **The error message is the worst part and is the cheapest to fix.** `AlreadyMapped` could say that
  the image overlaps the stack and name both addresses, independently of any layout decision.
- **This block proposes no new layout.** Picking one is the work, and it interacts with the std
  heap's base at `0x4000_0000` and the shared pages above it.
