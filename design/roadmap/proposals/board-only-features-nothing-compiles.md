# The board-only kernel features nothing compiles, until somebody is standing at the board

**Status: PROPOSED 2026-09-04.** Found by the maintainer/e3-on-radon lane, which added the fifth
one.

**Gate: NONE.** It is a build matrix, and the expensive question (which of these should also *run*
somewhere) is deliberately left out of it.

## In brief

`kernel/Cargo.toml` carries feature flags whose only consumer is a microSD card:
`board`, `soak`, `jobmix`, `reboot_soak`, `single_hart`, and `fastpath_pad` beside them. **Nothing
in CI compiles any of them.** `script/lint`, `script/test`, `script/bench` and every workflow build
the default and test kernels; a card build happens when a person runs `script/board-image`, which is
minutes before they walk to the bench.

So an ordinary refactor in `kernel/src/sched.rs` or `kernel/src/smp.rs` can break a card build, and
the tree stays green for as long as nobody writes a card. The failure surfaces as a bench session
that starts with a four-minute cargo error instead of a boot, which is the most expensive place in
this project to discover a compile error: the board is powered, the card is out, and the person is
not at their desk.

## What it would be

A CI job, or a leg of `script/ci-build`, that runs `cargo build -p kernel --release --target
riscv64imac-unknown-none-elf` once per feature set the tree actually writes to a card:

```
board
board,soak
board,soak,reboot_soak
board,jobmix
board,bench,single_hart
board,bench,single_hart,fastpath_pad
```

Six release builds of one crate. It proves compilation and nothing else, which is the point: it is
the cheap half, and it catches the failure that costs the most.

## What it is not

**It is not "run the soak in CI".** These features exist because their workloads need a board;
`reboot_soak` will not even compile off riscv64 by deliberate design, and a `jobmix` sweep under TCG
is the rehearsal `script/job-mix` already provides. Compiling is the part that generalizes.

**And it is not a fourth architecture's problem.** `fastpath_pad` has no x86_64 module and that is
already recorded in `script/fastpath-footprint`'s BUGS; a matrix would make the gap visible rather
than fix it.

## The same shape one level out

The list above is hardcoded, which is finding 11 of notes/architecture-list-sweep.md wearing new
clothes: the seventh card feature somebody adds is measured here only if they remember to type it.
Deriving it from `[features]` is possible and is a second decision, because not every feature in
that file belongs on a card.

## Where it came from

notes/footprint-perturbation.md's BUGS, and the observation that `single_hart` shipped into a tree where
nothing but a person's own `script/board-image` run has ever compiled it.
