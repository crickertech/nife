# Select E3's padding at boot rather than at compile time, so the experiment is one binary

**Status: PROPOSED 2026-09-04.** Named by calef while planning the first E3 bench session, on
learning it wanted two cards and he has one.

**Gate: NONE.** The mechanism it would copy shipped the same week.

**What the work is.** `fastpath_pad` is a Cargo feature, so E3's padded and un-padded arms are **two
different kernels**. The comparison therefore rests on trusting that two builds differ only where
intended, and the bench session pays for it twice over: six card rewrites to interleave the boots,
and no way for a booted machine to say which arm it is running.

**If the padding were selected at boot instead**, a token on the command line, the way milestone
243's `screen=` now carries the framebuffer geometry from `uefi_loader` to the kernel, the session
becomes **one card, one write, six boots**, and the two arms are provably the same binary.

**Why the mechanism is already proven.** Milestone 243 established the whole path on 2026-09-04:
PVH's `cmdline_paddr` is a field the format has always had, `machine_discovery::x86_64::BootInfo` has
decoded it since milestone 87, and nothing read it until 243 did. On riscv64 the boot script is
milestone 218's `boot.scr`, which is ours to write, so the token has an obvious home.

**What it would cost, and the honest part.** Resident dead code selected at runtime is not the same
experiment as resident dead code linked in: a branch that skips the padding still has the padding in
the image, which is the point (E3 perturbs the *footprint*, and an un-fetched byte still occupies a
cache line only if it is on the same line as a fetched one). **Whoever takes this has to say whether
a runtime-selected pad perturbs what E3 means to perturb**, and the answer is not obviously yes.
That is the design question and it is the whole of the work; the plumbing is a token and a branch.

**What it would buy beyond this one session.** Every future A/B on the board has the same shape, and
this tree now has several build-time flags that select an experiment rather than a product:
`soak`, `jobmix`, `reboot_soak`, `single_hart`, `fastpath_pad`. A proposal already records that
**nothing in CI compiles any of them** (`board-only-features-nothing-compiles.md`), which is the same
brittleness from the other side.
