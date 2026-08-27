# 1. Boot to a login prompt, into `swish`, to editing a file with `kilo`

calef, 2026-08-26: boot to a login prompt, enter a username and password, land in a `swish` prompt
on a terminal worth using, and create and edit a file with `kilo`. The first journey tracked here,
and the reason the directory exists: no single milestone was blocked when this was asked, but
tracing the whole story end to end found a real, unowned gap (milestone 177) that scanning the
roadmap milestone by milestone never surfaced.

`swish` itself is not a step below: it already exists and is already what the real interactive boot
spawns unconditionally today (`crates/system_initializer::boot`), so nothing on this journey is
waiting on it. The gaps are everywhere else.

| step | milestone | decision | what this step needs |
|---|---|---|---|
| 1 | 29 | | the framebuffer contract, bitmap font, VT engine, and virtio keyboard driver: proven, but only under the test harness |
| 2 | 33 | | the compositor: multiplexing the screen, proven under the test harness |
| 3 | 177 | | attach the GPU/keyboard devices to the real interactive boot and swap `console`/`input` for `display_terminal`/`compositor`; on x86_64 this also needs a real interactive-boot entry point first, which does not exist on that architecture at all yet |
| 4 | 49 | 120 | wire `login` into the real interactive boot; unblocked 2026-08-26 (§120 amended, grants the QEMU-only virtio-rng stopgap), the piece itself is still unbuilt |
| 5 | 169 | | `kilo`: the raw-keystroke input primitive nife's terminal layer does not have today, and the editor itself |

Steps 3, 4 and 5 are independent of each other; none is a prerequisite for either of the others
(traced directly: `kilo`'s raw-input primitive sits at the `DECISIONS §21` line-discipline contract
level, which both the plain UART console and `display_terminal` already speak identically, so step 5
does not wait on step 3; step 4's blocker was a device grant chain unrelated to either).

## What "done" looks like

Every step above at `BUILT`. At that point the story in the title is something a person can
actually do, not something proven in five separate test harnesses.
