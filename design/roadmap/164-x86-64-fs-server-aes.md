# 164. x86_64 userspace can't build `aes` (and therefore `fs_server`): no SSE, no scalar fallback

**Status: NOT-STARTED.** Minted 2026-08-25, provisional number pending the integrator (mint against
the current index at merge). Found by milestone 161's x86_64 userspace lane (pull request #476),
which named it plainly as "the finding that is not our bug" and proposed it as its own milestone
rather than routing around it. This block gives that debt a home.

**Gate: NONE.** Unlike milestone 163 (the JH7110's PCIe root complex), minted the same day for a
similarly orphaned finding, this is a toolchain and dependency problem, not a hardware one. A lane
can make real progress here without a board.

## What it needs

`fs_server` does not compile for `x86_64-unknown-none` at all, which is **21 of PR #476's 67
skipped tests on its own**. It links the vendored RedoxFS engine, which depends on the `aes` crate
unconditionally (the crypto is not behind a feature flag). `aes` fails at every optimization level,
including zero, with:

```
rustc-LLVM ERROR: Do not know how to split the result of this operator!
```

The cause is the target spec, not the crate: `x86_64-unknown-none` is built with
`-mmx,-sse,+soft-float` (`.cargo/config.toml`'s own comment on the target explains why the kernel
needs a static relocation model, but the SSE-disabling is inherited from the target's `none` base,
not something this tree chose deliberately for this reason). With no 128-bit vector register to
legalize an AES block into and no scalar fallback path, LLVM has nothing to emit. PR #476's own
words: "Nothing on our side fixes it, and it means there is no point attaching a disk to the x86
runner until it is solved."

**A fact worth knowing before scoping either route below**: `x86_64-unknown-none` is not a
userspace-only target the way it might first appear. Reading `.cargo/config.toml`, this is the
*same* target the kernel itself builds under on this architecture (`[target.x86_64-unknown-none]`
carries the kernel's own `relocation-model=static` flag), and `fs_server` builds against it via the
same `TARGET` constant every other `user/` program uses (`xtask/src/main.rs`'s `fs_server_build`).
Unlike aarch64 and riscv64, x86_64 has no separate kernel-vs-userspace split at the target level
today. That matters for Route 2 below.

## The two routes, as named by PR #476, sized here rather than guessed at

### Route 1: patch the vendored `aes` crate

`patches/README.md` is the right home for this: "one file per patch, in `git format-patch` form,
applied with `git am`... each exists to be upstreamed." Two existing patches already carry fixes
against vendored dependencies this tree needs on a bare-metal target (`redoxfs-no-std-vec-import.patch`,
`redoxfs-no-std-create-uuid.patch`), so the mechanism and submission path are proven; this would be
a third entry of the same shape.

The patch itself would need to give `aes` a scalar (non-SIMD) fallback implementation for whatever
operation currently has none: `aes`'s own upstream likely already has a portable/software
backend selectable by feature flag or `cfg`, since SIMD-less targets are not unique to this project;
sizing that precisely (is it a feature flag this tree isn't enabling, or a genuine gap in the
crate's own portable path) is the first thing whoever picks this up should check, before assuming a
patch is needed at all.

### Route 2: an x86 userspace target that keeps SSE

**This is a materially bigger change than it might look, because of the shared-target fact above.**
Simply flipping `x86_64-unknown-none`'s features to re-enable SSE would affect the kernel build
too, not just `fs_server` and other userspace programs: LLVM would then be free to use SSE/vector
instructions anywhere in kernel code (codegen for `memcpy`/`memset` is the usual culprit), which is
unsafe without SSE properly enabled and saved/restored, kernel-side, from the earliest boot code
onward.

The safer shape, matching precedent already in this tree: a **separate** target, distinct from the
kernel's own `x86_64-unknown-none`, for EL0 programs specifically. This tree already has exactly
this kind of custom target for two other architectures: `targets/aarch64-unknown-nife.json` and
its riscv64 twin, both minted for milestone 27/64's `std`-support farm ("nife-os userspace,
aarch64 (softfloat EL0, native capability ABI)", `"std": true`), though those are for the
crates.io-crate-compatibility farm specifically, not for `user/`'s own programs, so a new
`x86_64-unknown-nife.json` (or similarly named) target for `fs_server` and friends would be a new
use of an existing mechanism, not a copy of an existing target.

**The real, currently-unbuilt cost this route needs, checked rather than assumed**: nothing in
`kernel/src/arch/x86_64/` saves or restores FPU/SSE register state anywhere: no `FXSAVE`/`XSAVE`,
no `CR0.TS`/`CR4.OSFXSR` handling, nothing in `kernel/src/sched.rs` or `kernel/src/thread.rs`'s
context-switch path. The other two architectures' own float/vector state handling was not checked
by this milestone (out of scope; note only that x86 currently has none). Enabling SSE for even one
userspace program means every thread's kernel-side state needs an FXSAVE area and the context
switch path needs to save and restore it, or one thread's floating-point registers will silently
corrupt another's the first time two SSE-using threads interleave. This is not a target-spec
change alone; it is new scheduler-adjacent kernel work.

## Why it matters, and what it unblocks

Directly: 21 of PR #476's 67 skips, and, in that lane's own words, "there is no point attaching a
disk to the x86 runner" until this is solved: no real filesystem testing is possible on the
x86_64 leg at all today. Indirectly: any future x86_64 work that needs `fs_server` blocks on this
the same way, which is most real storage or network-service work on that architecture once ported,
matching what milestones 53 and 55 already need on the other two.

## What this does not decide

Which route to take. Route 1 is cheaper if `aes` genuinely has an unused portable fallback; Route 2
is the more general fix (any future crate with the same SSE assumption hits the identical wall) but
carries real, currently-unbuilt scheduler cost that Route 1 does not. Sizing Route 1 precisely
(reading `aes`'s own source for what a scalar path would need) is the cheapest next step before
committing to either, and is not done by this block.
