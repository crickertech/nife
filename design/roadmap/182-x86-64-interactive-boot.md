# 182. x86_64's own interactive-boot entry point

**Status: NOT-STARTED.** Split off from [milestone 177](177-graphical-interactive-boot.md),
2026-08-27, once that milestone's build lane found piece 3 (originally scoped as "build x86_64's
own interactive-boot entry point first") needs a from-scratch ELF-loading boot path, not wiring: a
substantially larger, separate undertaking than pieces 1-2's device attachment and program swap.

**Gate: NONE.** Nothing here is a design fork; `kernel::user::spawn_init` (aarch64) and
`riscv_shell_boot` (riscv64) are the worked examples to follow, not a new design.

## What is actually missing

x86_64 has no third function beside `spawn_init`/`riscv_shell_boot`. What exists instead,
`kernel::user::x86_userspace_demo`, is a fixture: it builds two children directly from a hand-built
region, each carved from a budget with every kernel object built in place, with no ELF parsed and
no archive loaded (its own doc: "the loader-shaped path minus the ELF"). It proves the scheduler and
the fault path can build and run real EL0-equivalent (ring 3) processes; it does not prove a real
`init` reached from a real archive, the thing `spawn_init`/`riscv_shell_boot` each already are for
their own architecture.

**Milestone 177 corrected its own earlier claim that this piece was independent of the others.**
x86_64 has no fallback UART path at all (DECISIONS §121, permanently kernel-resident), so this
milestone's only possible route to an interactive shell is through the graphical stack milestone
177 builds, not a plain-console alternative the way aarch64/riscv64 each have one. This milestone
therefore starts from wiring the graphical path directly, not a two-step "console first, graphical
later" the other two architectures got to take.

## What this needs

1. **A real ELF-loading boot entry**, matching `spawn_init`/`riscv_shell_boot`'s shape: parse the
   initrd archive, measure and verify `init` against the boot's own measurement table (DECISIONS
   §104's discipline, the same check the other two architectures' entries already make), build its
   address space, grant it the boot's own capability set, and dispatch it to ring 3.
2. **The x86_64-specific capability grants** the other two boots each hand-assemble for their own
   architecture: whatever this boot's own device set turns out to be (informed by milestone 177's
   now-built kernel-side graphical-stack pattern, `kernel::user::boot_graphical_terminal`, which
   this milestone should call the same way aarch64/riscv64 now do rather than re-deriving it).
3. **A third `script/shell-check` `--arch` leg**, extending the graphical leg milestone 177 already
   built (`shell_check_leg_graphical`) rather than inventing a fourth verification shape.

## What this does not decide

Whether x86_64's own boot path needs anything architecture-specific beyond the ELF-loading
mechanism itself (interrupt routing, device discovery specifics already covered by milestone
176's own work) is not assessed here; check that milestone's own text and `kernel/src/arch/x86_64/`
before assuming parity with aarch64/riscv64 on every point.

## What this unblocks

x86_64 joining aarch64/riscv64 as a real interactive-boot target, which milestone 177's own text
already names as the graphical half of the login-to-`kilo` user story. Depends on milestone 177's
own remaining blocker (the display-driver flush hang, recorded in its BUGS) resolving first, since
this milestone has no plain-console fallback to prove against in the meantime.

## BUGS

Not started; nothing built yet to carry its own BUGS section.
