# 227. A GICv3 driver, because GICv2 boots and silently loses every interrupt

**Status: NOT-STARTED.** Minted 2026-09-02 by the maintainer, from milestone 222's (the one command
a person runs before pushing has a leg that fails instead of skipping) measurement. *(Number
provisional until the merge queue lands it.)*

**Gate: NONE.** Nothing external blocks it. It is large, which is different.

**In brief.** `kernel/src/drivers/gic.rs` speaks GICv2 only. What nobody had measured until 2026-09-02
is what happens under GICv3, and the answer is worse than an unsupported build:

**The kernel boots the whole tour, brings four cores online over PSCI, prints
`timer: 100 Hz tick, interrupts ON`, and then takes zero interrupts and zero preemptions, with
nothing faulting.**

The cause is one line of assumption. `memory::gic_regions()` matches the device tree node by the name
prefix `intc@` and takes its first two `reg` blocks. **A GICv3 node's second block is the
redistributor, not a CPU interface**, so `gic::init_this_cpu` writes `GICC_PMR` and `GICC_CTLR` into
registers that are not there.

So the honest statement is not "GICv3 is unsupported". It is **"GICv3 boots and silently loses every
interrupt"**, which makes the aarch64 runner's `gic-version=2` pin load-bearing rather than tidy, and
which is the most dangerous shape a hardware assumption can take.

## Two independent reasons, which is why it is its own milestone

- **It is what puts accelerated testing back.** HVF requires GICv3, so as of milestone 222 there is
  **no accelerated coverage on the development machine at all**: every gate a contributor can run is
  TCG until this exists.
- **It is the largest item barring most modern aarch64 boards**, which `notes/aarch64-board-survey.md`
  recorded independently and long before the HVF question came up. Milestone 127 (the seL4 machine)
  asked from its own side that this be minted deliberately rather than folded into something else,
  noting argon's generation is the last GICv2 silicon anyone will buy.

## What it is, priced from the failure rather than guessed

Milestone 222's lane priced it while measuring:

- **Version discovery at init.** Nothing reads `compatible` today.
- **A redistributor frame per core**, which is the structure GICv2 does not have.
- **A system-register CPU interface** (`ICC_*`), so `msr`/`mrs`, so it belongs in
  `kernel/src/arch/aarch64/` by DECISIONS §4 rule 1 rather than in `drivers/`. That split is the
  interesting design question here and this block does not settle it.
- **Affinity routing** replacing the eight-bit `GICD_ITARGETSR` mask.
- **Both drivers coexisting**, since GICv2 is what the runner, the CI matrix and argon's silicon use.

About ten call sites reach `drivers::gic::` directly.

## BUGS

- **This block does not decide where the driver lives**, and the answer is not obvious: rule 1 puts
  system-register access under `arch/`, while the existing GIC driver is under `drivers/`. Whoever
  takes it should decide that first and record why.
- **It is not on any fatal risk's critical path**, which is worth saying plainly so it is not
  promoted past work that is. It restores a convenience and opens future hardware.
- **Nothing checks that a GICv2 assumption stays true.** The silent-loss failure above was found by
  someone deliberately asking; no gate would have noticed.
