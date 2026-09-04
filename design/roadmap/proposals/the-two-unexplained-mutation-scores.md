# `uefi_loader` at 15% and `manual` at 52% are unexplained holes in the published score

**Status: PROPOSED 2026-09-03.** Written by the milestone 247 sweep, from milestone 238's block.

**Gate: NONE.** The measurement runs with tooling already in the tree, milestone 244 is the worked
example of doing it for one crate, and nothing external blocks it.

**In brief.** The first mutation report the sweep ever published put the tree at 83.4%, down from
92.4%. Three crates carry nearly all of that fall. Milestone 244 took `system_initializer`, measured
it, and closed `RECORDED` on the honest reason that the pure fraction a mutation can reach in that
crate is small. The other two, **`uefi_loader` at 15% and `manual` at 52%**, have nobody on them.
The work is to measure each properly and give each an answer: tests that kill the surviving mutants,
or a `BUGS` entry that accepts the score and says why.

## Why this matters

The published number has two holes in it and nothing anywhere admits to them. No `BUGS` entry
accepts either score, so a reader who finds 83.4% has no way to learn that two crates account for
most of the gap or that nobody has looked at them. That is the failure mode this tree's `BUGS`
convention exists to prevent, and it currently applies to a figure that is about to be quoted in a
fatal risk's verdict.

15% is low enough to mean something specific rather than to be noise. Either `uefi_loader` is barely
tested, or almost all of its mutants are unreachable by a test that cannot boot firmware, and those
two conclusions have opposite consequences. Only measuring tells them apart. `manual` at 52% is the
milder case and the more likely to be a real testing gap, since it is ordinary host-testable Rust.

Milestone 244 is also the template for the acceptable outcome, which is worth saying because it
keeps this from being open-ended. It did not chase the score; it measured, found the reachable
fraction small, and recorded that. Either of these two crates may end the same way, and a recorded
reason is a complete answer.

## Where it came from

Milestone 238's `## Follow-on`: *"Measure and answer for `uefi_loader` at 15% and `manual` at 52%,
the two crates this block names beside `system_initializer` as carrying nearly all of the fall from
92.4% to 83.4%. Milestone 244 took `system_initializer` alone. Nothing tracks these two, and no
`BUGS` entry anywhere accepts their scores, so the published number has two unexplained holes in
it."*
