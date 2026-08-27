# 177. Wire the graphical terminal stack into the real interactive boot

**Status: NOT-STARTED.** Minted 2026-08-26, from tracing the user story "boot to a login prompt,
land in a `swish` prompt on a real terminal" against the actual code rather than the roadmap's own
framing, and finding no milestone owns the gap this surfaced.

**Gate: NONE.** Nothing here is a design fork. The framebuffer contract (milestone 29), the
compositor (milestone 33), the VT engine and bitmap font (milestone 29's deferred half), and the
virtio keyboard driver are all built and proven, on both ISAs, under the test harness. What is
missing is device attachment and program selection in the one boot path that is not the test
harness.

## What actually blocks this

The real interactive boot (`crates/system_initializer::boot`, reached by
`kernel::user::spawn_init` on aarch64 and `riscv_shell_boot` on riscv64) spawns exactly `console,
input, line_editor, swish, job_undertaker`. `console` and `input` are the plain UART pair (DECISIONS
§26): `input.rs`'s real-boot path reads only the UART register layout, with no code path to a
virtio device at all outside `cargo xtask test`. Checked directly, not assumed: neither the GPU nor
the keyboard is in `system_initializer::BootEndowment`'s device grants, and `NIFE_GPU`/`NIFE_KBD`
join `NIFE_RNG`/`NIFE_NVME` on the list of flags this boot's QEMU invocation never sets, which
`cargo xtask test` sets unconditionally for every leg. This is the same shape DECISIONS §120 already
found and named for the RNG case on milestone 49's boot-wiring fork: a deliberate, existing
minimal-device-surface choice for the interactive/demo boot, not an oversight.

**x86_64 has neither of the two entry functions named above, at all** (found 2026-08-27, tracing
`printenv`'s own "both boards" verification against `script/shell-check`, which names only aarch64
and riscv64: *"x86_64 has no shell leg."* That comment's own justification, "there is no userspace
on that target at all yet," is stale, since x86_64 has had real userspace running for most of this
session; the fact itself is not stale, for a narrower and still-current reason). Nothing boots
straight to a shell prompt on x86_64 today; `kernel/src/user.rs` has no third function beside
`spawn_init`/`riscv_shell_boot`. This is milestone 49's gap as much as this milestone's: neither
currently plans it.

So this milestone is three joined pieces, not two:

1. **Attach the devices.** GPU and keyboard grants added to `BootEndowment`, and the interactive
   boot's QEMU invocation (`scripts/qemu-runner-*.sh`'s non-test path, or a new demo-boot flag)
   attaching the virtio-gpu and virtio-keyboard devices the test harness already exercises.
2. **Swap the programs.** Replace `console`/`input` in `system_initializer::boot`'s spawn list with
   `display_terminal`/`compositor`/the virtio keyboard client, the same components milestone 23's
   own text already names as proven but "not running under the test harness."
3. **Build x86_64's own interactive-boot entry point first**, before either piece above can mean
   anything on that architecture: a third function beside `spawn_init`/`riscv_shell_boot`, and
   `script/shell-check` extended to a third `--arch` leg once it exists. Sequencing note, not a
   design fork: pieces 1 and 2 are provable on aarch64/riscv64 without waiting on this, and this
   piece is provable against the plain `console`/`input` pair before it needs to be provable against
   the graphical stack, so it does not have to land last.

## What this does not decide

Whether the swap is unconditional (the graphical stack becomes the only interactive boot) or a
runtime choice (both paths coexist, selected by a flag); which of milestone 55's storage or
milestone 49's login pieces this should sequence before or after; and whether `line_editor` itself
needs any change to run as a `display_terminal` client rather than a `console` client, or already
does (checked in milestone 29's own text as a client of the framebuffer contract, not re-verified
against the real boot path here).

## What this unblocks

The graphical half of the login-to-`kilo` user story ([DECISIONS
§131](../decisions/131-hold-at-rung-two.md)'s "kick-ass terminal, something I'll love working
with"). Independent of [milestone 169](169-kilo-editor.md) (`kilo`'s raw-keystroke primitive sits
at the `DECISIONS §21` line-discipline contract level, which both `console` and
`display_terminal` already speak identically) and of milestone 49's login-boot-wiring piece
(unblocked 2026-08-26, DECISIONS §120 amended to grant the stopgap; the piece itself is a separate,
ongoing build). All three can proceed in either order; none is a prerequisite for either of the
others, though piece 3 above (x86_64's own entry point) is a real prerequisite for either of *this*
milestone's other two pieces meaning anything on that one architecture specifically.
