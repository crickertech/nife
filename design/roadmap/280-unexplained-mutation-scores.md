# 280. `uefi_loader` at 15% and `documentation` at 52% are unexplained holes in the published score

**Status: NOT-STARTED.** Written by the milestone 247 sweep as a proposal on 2026-09-03, from
milestone 238's block. **Promoted out of the proposal queue on 2026-09-13 by calef**, who asked what
would progress fatal risk 3: this is step three of four, and the only one that can be worked before a
clean sweep exists. *(Number provisional until the merge queue lands it; 278 is in flight ahead of
it.)*

**Gate: NONE.** The measurement runs with tooling already in the tree, milestone 244 is the worked
example of doing it for one crate, and nothing external blocks it.

**Half of this was already answered, and the promotion did not notice.** `uefi_loader` was
diagnosed on **2026-09-04**, the day after this proposal was written, and the record is
`notes/mutation-testing.md`'s dated section plus the exclusion and its derived gate in
`.cargo/mutants.toml`. The 15% was arithmetic rather than a finding: `src/main.rs` carries
`required-features = ["uefi"]`, so `cargo test` never puts it in the build graph, and all 154 of
its mutants came back MISSED in *"0s build + 0s test"* because nothing rebuilt. The pure half the
design had lifted out to be host-testable was at **94.1%** the whole time. Milestone 244's result
one level down, at a target rather than a crate.

**So what is left of this milestone is `documentation` at 52%**, and one residue that is already
tracked elsewhere: excluding `src/main.rs` made the number honest, not the file proved, and whether
its 790 firmware lines get lifted the way `handoff` and `image` were is
`design/roadmap/proposals/the-uefi-loaders-firmware-half-is-proved-by-one-boot.md`.

**Where it sits on that path.** Milestone 277 built the memory bound so a sweep can survive a runaway
mutant; the first green scheduled run is what proves it. This milestone is what stops the number that
run produces from having two unexplained holes in it, and
`design/roadmap/proposals/fatal-risk-3-against-the-new-number.md` is the reading calef then does. It
can start now: both crates can be measured against the existing 83.4% shard without waiting for a
full run.

**In brief.** The first mutation report the sweep ever published put the tree at 83.4%, down from
92.4%. Three crates carry nearly all of that fall. Milestone 244 took `system_initializer`, measured
it, and closed `RECORDED` on the honest reason that the pure fraction a mutation can reach in that
crate is small. The other two, **`uefi_loader` at 15% and `documentation` at 52%**, have nobody on them.
(That crate was measured under its then-name `manual`; calef ratified `documentation`
on 2026-09-13. The quoted `Follow-on` below keeps the old name, as a quotation must.)
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
two conclusions have opposite consequences. Only measuring tells them apart. `documentation` at 52% is the
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
