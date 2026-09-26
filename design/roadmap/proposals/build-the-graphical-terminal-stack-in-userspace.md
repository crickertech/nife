# Build the graphical terminal stack in userspace now that a frame names a run

**Status: PROPOSED 2026-09-26.** Raised by the lane for milestone 23 (a capability-routed component
OS with live replacement), checking why `display_terminal` cannot be swapped: the kernel builds it,
so no userspace supervisor holds its endowment. notes/interactive-stack-swap.md.

**Gate: NONE.** A question answered by reading the kernel, then ordinary work if the answer is yes.

## The finding

`kernel/src/user.rs`'s `boot_graphical_terminal` builds the gpu driver, `display_terminal` and the
keyboard driver kernel-side because a virtio-gpu driver needed eleven capability-table slots, one
`PageFrame` per DMA page. §102 (a Frame names a run of pages) was decided a week before that comment
was written, and the `MAP_INTO` handler in `kernel/src/syscall.rs` already maps a whole run in one
call. So the premise looks expired.

## What to find out, then build

1. Are the gpu's DMA pages minted as one contiguous run, and does its `virtio` registration accept
   one? If not, record why and stop.
2. If so, move the three spawns into `system_initializer`, which then holds `display_terminal`'s
   endowment and supervision endpoint, and correct `boot_graphical_terminal`'s doc comment.

## What it unblocks

Swapping `display_terminal` live (milestone 23's last `Outstanding` line, its second component), and
one fewer kernel-side spawn that exists for a budget reason.
