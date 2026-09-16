# A toolchain bump leaves the icount baselines stale, and nothing says so

**Status: PROPOSED 2026-09-15.** Decision 2 of the finding milestone 299's lane surfaced (originally
`design/roadmap/proposals/icount-baselines-drift-after-a-toolchain-bump.md`, on PR #883). Milestone
300 executed Decision 1 (decompose and re-baseline) and carries the finding; this file holds the
half that is calef's to decide.

**Gate: DECISION.** It changes what a toolchain-bump PR is obliged to do, which is a workflow rule,
not a lane's call.

## The finding, and why it survives its own correction

`script/toolchain-bump` / `toolchain-bump.yml` raises the pinned nightly and changes only
`rust-toolchain.toml`. It does not re-save `bench/baseline-{aarch64,riscv64,x86_64}.txt`, on the
theory that a new nightly's codegen may move the deterministic icount counts the baselines gate
against. When the bump moves the numbers and the floor is not moved with it, the tripwire's headroom
erodes silently, and the next unrelated PR that adds a few percent trips the +10% gate for a reason
that is not its own.

Milestone 300 measured this window and found the drift was **not** the nightly: `nightly-2026-08-27`
and `nightly-2026-09-15` emit byte-identical icount on the same code, and the whole ~+6.5% move was
one code change (milestone 139's cycle-counter grant at the context switch). So the specific alarm
that motivated the proposal was a false attribution. **The proposal survives anyway**, because the
mechanism it names is real and unguarded: a nightly that *does* move codegen would erode the headroom
exactly as feared, and nothing in the tree would notice until a lane chased an unrelated regression
into it months later. The fix should not depend on which cause happens to bite.

## What to decide

Two shapes, calef's pick:

1. **A bump re-baselines in the same PR.** `toolchain-bump.yml` runs `cargo xtask bench --save` for
   all three architectures after it raises the pin, and commits the re-saved baselines alongside
   `rust-toolchain.toml`. The floor then tracks the toolchain by construction. Cost: the bump PR now
   depends on the bench runners, and a re-save is a committed performance floor made by automation
   rather than by a human reading the diff, which is the thing baseline saves have always been
   deliberate about.
2. **A bump fails loudly if the baselines are stale for its nightly.** A check that says "the pinned
   nightly changed and the baselines were not re-saved for it" turns the silent erosion into a red
   gate a human clears deliberately. Cheaper and keeps the human in the save, at the cost of a manual
   step on every bump that moves the numbers (and a no-op ceremony on every bump that does not).

## BUGS

- **Option 1 lets automation commit a floor without a human reading the diff**, which is exactly the
  care a baseline save has always carried (`bench/baseline-aarch64.txt`'s own header: "a statement
  that a performance change is intended and understood"). A re-save that also launders a real
  regression into the floor is the failure milestone 300 exists to prevent, one level out; option 1
  would need the decomposition 300 did by hand to be at least partly automatic, or it re-opens the
  hole.
- **This is measured on one machine, one QEMU.** The numbers are TCG icount on the dev Mac. A
  different runner moves them again, which is the argument *for* mechanizing the re-baseline rather
  than trusting a human to eyeball a percentage, and also the argument that the mechanized floor is
  only as portable as the runner that saved it.
