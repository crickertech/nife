# 166. One boot orchestrator, reached three inconsistent ways: giving `init` a consistent meaning across architectures

**Status: NOT-STARTED.** Minted 2026-08-25, from a naming review that went looking for a small
inconsistency and found a real one instead. calef asked whether aarch64's `init -> hello` archive
mapping should become `init -> builder`, to match riscv64 and the then-unmerged x86_64 port
(PR #476). An investigation lane found the premise false: `hello` and `builder` are not comparable
programs, and repointing aarch64's mapping would have silently broken the real interactive boot plus
six live kernel tests. This milestone is the real question underneath that near-miss.

**Gate: NONE.** This is a software architecture question, not hardware-gated. Whoever picks it up
needs no board, no bring-up, nothing calef has to do by hand first.

## What the investigation actually found

There is exactly **one** real boot orchestrator, `crates/system_initializer::boot()`, and it is
already correctly shared. Its own module doc says so plainly: *"There are two inits, because the two
boards' kernels hand off differently: `user::initrd()` loads the archive entry `init`, which is
`user/src/hello.rs`'s `init_boot` role on aarch64 and `user/src/system_initializer.rs` on riscv64.
There is **one** system they build, and this crate is it."* The same doc also names why this crate
exists at all: before it, the construction and the spawn service were written twice, and *"a fix
that lands in one init and not the other is **a boot that reaches userspace and prints nothing at
all**, with no fault and no message. That shape cost three separate lanes an evening each."* This
milestone is that same risk, one layer up: not the orchestrator's logic diverging, but the *paths
that reach it* already having diverged, silently, in a way nobody had named until now.

**The three paths, precisely** (`kernel/src/user.rs`):

- **aarch64.** `INIT_ROLES_ENTRY = "init"`. `hello` is packed under the archive name `init` because
  on this architecture it genuinely is the boot program: role 27 (`INIT_BOOT_ROLE`) builds
  aarch64's device-grant table (PL011, GIC IRQ, the clock page, the file service when a disk is
  attached) and hands off into `system_initializer::boot()`. `spawn_init`/`boot_via_init` is this
  path. The same archive entry, `init`, and the same `INIT_ROLES_ENTRY` constant, is also how six
  live kernel tests (`kernel/src/user/tests.rs`, the milestone 19d role-dispatch suite: roles 20,
  23, 24, 25, 28, 29) reach `hello`'s other roles. One archive slot, two live jobs.
- **riscv64.** `INIT_ROLES_ENTRY = "hello"` here instead, and the comment at its definition says
  why: *"RISC-V's `init` is the portable `builder` demo, so hello goes in under its own name.
  Reading the wrong one gets a program with no such roles."* The real boot path,
  `riscv_shell_boot`, never touches the `init` archive entry at all: it reads `"system_initializer"`
  by its own name directly, measures it under that name in the trust root, and boots it. `builder`
  (packed as `init`) is a narrow, standalone milestone-20 demo: it parses the archive, builds one
  hardcoded child (`worker`), starts it with a fixed input, and exits. Invoked only by
  `riscv_initrd_demo`, structurally unrelated to the real interactive boot.
- **x86_64** (PR #476, milestone 161 item 4's hand-off, as of this writing still open/unmerged).
  Currently maps `init -> builder` too, via the same `portable_archive_entries()` table riscv64's
  `initrd_riscv` now shares. Per this milestone's own finding, that mapping was never actually
  riscv64's real boot program to begin with, so x86_64 may be inheriting the same legacy-artifact
  choice riscv64 carries, rather than a deliberate one. **Read what PR #476 actually shipped before
  starting this milestone**: it may already need to change, or it may be a clean-slate opportunity
  to give the third architecture the right shape from the start, and that's worth knowing before
  scoping the fix.

**So the inconsistency is not "aarch64 lags a converged standard."** riscv64's `init -> builder`
mapping is itself a legacy artifact, older than the shared milestone 19d role-catalogue tests that
now also depend on the `init` archive slot meaning something specific on aarch64. Three
architectures, three different answers to "what does the archive's `init` entry mean," none of them
wrong on their own terms, all of them different.

## What this milestone is not

**Not a rename.** The near-miss this session was exactly that mistake: reading the surface
inconsistency (three packers disagree on `init -> X`) and assuming the fix is picking one `X` and
repointing the others at it. It isn't: `hello`'s `init_boot` role and `builder`'s standalone demo
do different jobs, and neither can simply replace the other without losing something (aarch64's real
boot device-grant table, in `hello`'s case; nothing load-bearing, in `builder`'s, which is why it's
the one safe to retire from the `init` slot).

**Not a decision this milestone makes.** What (if anything) the archive's `init` slot should hold on
each architecture, once the real boot orchestrator has its own name everywhere, is an open design
question for whoever builds this, not predetermined here.

## The direction the investigation suggested, not yet decided

Give aarch64 a `system_initializer`-under-its-own-name boot entry point, mirroring riscv64's shape:
a dedicated, separately-named archive entry for the real boot orchestrator, decoupled from both the
generic `init` slot and from `hello`'s role-dispatch test-fixture job. Only once that split exists on
all three architectures does it become possible to answer, cleanly, what `init` itself should mean
(a test-harness convenience name naming whichever role-catalogue program a given architecture ships,
nothing at all, or something else) without one archive slot quietly carrying two jobs at once.

## What it touches

- `kernel/src/user.rs`: `spawn_init`, `boot_via_init`, `riscv_shell_boot`, `INIT_ROLES_ENTRY`,
  `INIT_BOOT_ROLE`, and whichever `x86_64` boot path PR #476 lands.
- `user/src/hello.rs`'s `init_boot` role (27) and its other, independently-tested roles
  (`SELF_CHECK`, `UNTYPED_DEMO`, the `VIRTIO_*` probes, `GRANTER`/`RECEIVER`,
  `FRAME_PRODUCER`/`CONSUMER`, `CALL_SERVER`/`CLIENT`, `REVOKE_DEMO`, `ASPACE_BUILDER`,
  `EP_MAKER`/`EP_USER`): these have solid, live coverage today through direct-by-name lookup
  (`HELLO_ENTRY`, ~28 call sites in `kernel/src/user/tests.rs`) and are unrelated to which program
  plays `init`; whoever builds this should confirm that coverage stays intact regardless of how the
  `init` question resolves.
- `user/src/builder.rs`, whose own role in the `init` slot (on riscv64 today, x86_64 pending #476)
  is exactly what this milestone reconsiders.
- `crates/system_initializer`, unchanged in logic: this milestone is about the paths that reach it,
  not the orchestrator itself.

## Why it matters

Because the failure mode is silent and expensive, and this project has already paid for it twice in
different forms: once historically (the two-inits-written-twice era `system_initializer`'s own doc
records, "three separate lanes an evening each"), and once this session, caught only because it was
investigated before being built rather than after. A boot path and a test suite quietly depending on
the same archive slot meaning two different things is exactly the shape of gap `AGENTS.md`'s own
ladder exists to move up a rung: right now it is a fact three people have to independently rediscover
by reading `kernel/src/user.rs` closely, not something the tree states once and any reader can find.

## What is needed to answer it

Whoever picks this up should read PR #476's actual, landed x86_64 choice first (not this milestone's
description of it, which is current only as of 2026-08-25), decide whether aarch64's real boot should
move to a named `system_initializer` archive entry the way riscv64's already is, and only then decide
what (if anything) the `init` slot should mean on each architecture going forward.
