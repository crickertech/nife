# 210. No kernel test can be run by name, so one falsification costs a whole suite

**Status: NOT-STARTED.** Minted 2026-08-31 from milestone 202's (falsify the confinement claims)
lane, which paid the cost twenty-five times. *(Number provisional until the merge queue lands it.)*

**Gate: NONE.** The runner already knows the test names; nothing has to be discovered, only passed
through.

**In brief.** `kernel/src/testing.rs`'s runner takes no filter, `cargo xtask test` parses only
`--arch`, `--cpu` and `--hvf`, and arguments after `--` go to QEMU. **So there is no way to run one
kernel test.**

Every host crate has this for free: `cargo test <name>` and `cargo kani --harness <name> --exact`
both work, which is why DECISIONS §134's falsification sweep is thirty seconds over twenty-five
records. **The kernel half of that table is not affordable at all**: one falsification costs a full
suite run of about four minutes, and milestone 202 measured 234 of those seconds as a single hung
test whose break manifested as a watchdog timeout.

## Why it is worth its own block

It is not only 202's problem. **Every lane that has waited out a whole suite to see one test has paid
this**, and the tree records the shape without naming the cause: the load-sensitive assertions, the
scanout checks, the two-core crash hunt, and this session's own `ci-build` runs that failed on timing
under load and told us nothing about the change under test.

It also gates something specific. `kernel/falsifications/` exists on `main` as of milestone 202 and
**nothing sweeps it**, because a sweep that runs the whole suite per record is not a sweep. Six
kernel confinement claims have no mechanism for that reason.

## What it needs

A filter reaching `kernel/src/testing.rs`'s runner from the command line, and an `xtask` argument to
carry it. The runner already has the test names; nothing has to be discovered, only passed through.

**The interesting constraint is the boot.** A kernel test is not a function a harness calls; it runs
inside a booted kernel under QEMU, so "run one test" means "boot, run one, exit", and the boot is
most of the four minutes. A filter makes the *selection* cheap and does not by itself make the *run*
cheap, which is worth knowing before anybody promises a fast inner loop.

## BUGS

- **This block does not price the boot.** If booting dominates, a filter turns four minutes into
  something closer to the boot time rather than into seconds, and the win is real but smaller than it
  sounds.
- **It says nothing about the three architectures.** A filter that runs one test on one ISA is what a
  lane wants and is exactly the shape DECISIONS §19 (architectural parity is a tenet) distrusts, so
  the default should stay all three and the narrowing should be explicit.
- **`kernel/falsifications/` is a provisional path** that milestone 202 created and nothing reads.
  Whoever builds the sweep decides whether it stays there or moves.
