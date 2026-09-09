# `initboot` and `shell` are one decision spelled twice, and the decision may not need to be a feature

**Status: PROPOSED 2026-09-09.** Written by milestone 267's lane, which was told to measure this and
explicitly told not to act on the measurement.

**Gate: NONE.** Nothing is blocked. It wants calef because retiring a feature is not reversible in
the way code is: `script/lint` lints each boot-mode feature by name, `xtask` exposes both as
subcommands, and anyone with a note or a script that says `cargo xtask initboot` has already acted
on it.

## The measurement, which is the reason this exists

`script/fastpath-footprint`'s method, release kernels, `.text` symbol bytes, measured at
`8dd9dbf6` before milestone 267 changed anything:

| arch | tour build | `--features shell` | delta |
|---|---|---|---|
| aarch64 | 193,332 | 179,312 | 14,020 (7.3%) |
| riscv64 | 162,964 | 157,136 | 5,828 |
| x86_64 | 140,842 | 140,842 | **0** |

**Two of those three are not what they look like.**

x86_64's boot arm is self-contained and ends in `arch::halt()`, so the shared milestone tour is
unreachable there and LLVM has already deleted it. The feature removes nothing on that architecture
and never has.

riscv64's arm halts too. Its `shell` feature swaps its own arch tour for `user::riscv_shell_boot`,
so 5,828 bytes is the difference between two RISC-V boot paths and not the cost of the aarch64 tour
at all.

So the thing these features exist to remove is **one architecture's 14 KB**, and symbol-level
attribution says almost none of it is the narrative: `console_service` ~3.0 KB, the two
never-yielding spinner closures ~3.3 KB, `virtio_service` ~1.7 KB, `memory_region_service` ~1.3 KB,
and `kernel_main` itself +1,460. Every one of those is a demonstration that needs a kernel privilege
(a kernel thread, a capability mint, a kernel static, the frame count), which is why milestone 267
left them where they are.

## The three things worth deciding

**One, is 7.3% on one architecture worth a compile-time switch at all?** It is not nothing for a
microkernel. It is also not the thing the switch is named after: after milestone 267 the narrative
is a program and the features still remove 14 KB, because they were never removing the narrative.

**Two, if it stays, should it be one feature rather than two?** Six `cfg` sites name these features
and every one names them together; `initboot` appears alone in none. The only real difference is in
`xtask`, where the `initboot` arm prints a different message and passes a differently-named feature
that does the same thing. Milestone 130 exists because a copy outlived its reason, and this is a
copy.

**Three, if it collapses to one, what is it called?** `design/roadmap/proposals/what-the-boot-path-is-called.md`
is holding a related question and **milestone 267 did not retire it**: after milestone 266 every boot
goes to the progenitor, so `progenitor-boot` still distinguishes nothing and `handoff` is still
generic. What changed is that there is now a fourth candidate the old argument did not have, which is
that the feature names *the absence of the demonstrations* rather than a boot path, and a word for
that reads differently from a word for a path.

## What this lane deliberately did not do

Nothing. `initboot` and `shell` are untouched, `script/lint` still lints both, and both `xtask`
subcommands work exactly as before. Milestone 267's brief said a finding is a finding and a feature
is not retired as a side effect, and this file is where the finding went.
