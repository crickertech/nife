# 175. Where the kernel's own output goes once userspace owns the console

**Status: PROPOSED.** Raised 2026-09-19 by milestone 435's slice B, which read milestone 342's
`DECISION` gate and found it naming no section. Milestone 230 named the fork while fixing something
else, and the 2026-09-03 proposal sweep carried it forward. *(Section number provisional until the
merge queue lands it.)*

## What is being decided

Once the `console` server owns the console, **two address spaces drive one UART with nothing
arbitrating**. The kernel writes directly, because a kernel that cannot print during a fault is a
kernel nobody can debug. The server writes on behalf of userspace. The streams interleave at byte
granularity.

The decision is where kernel output goes: a second port, a buffer the server drains, a claim the
server takes and the kernel respects except in a panic, or something else.

## Why it is not a bug to fix

It corrupts every bench session on argon, radon and xenon, where a serial log is the only thing
those machines can say, and milestone 216 built a tool whose contract is recognising a boot sequence
in that stream. Interleaved bytes break that contract in the least visible way available: the log is
present, it looks like output, and the line being matched has a kernel message spliced through the
middle of it.

**Nothing in `design/decisions/` answers it**, checked 2026-09-19.
[§149](149-kernel-served-console-endpoint.md) is the nearest and is a different question: it asked
whether the kernel may *answer on an endpoint* where §121 left x86 without a userspace holder, and
it was resolved on 2026-09-15 by dissolving that premise, so x86's console is a userspace driver like
the other two. That makes the interleaving question **more** live rather than less, because all three
architectures now reach the shape that produces it.

## What the kernel actually writes after the handoff, measured

Milestone 342's block says this is *"a measurable list rather than an opinion"*. The tree has already
measured it, and the answer is in `xtask/src/main.rs`:

> Text the **kernel** prints only in a user-fault report, which is the only thing it writes after
> the userspace console has started.
>
> -- `KERNEL_FAULT_TOKENS`'s doc comment

Six tokens (`user thread `, ` killed: `, `the kernel is fine`, `stval 0x`, `esr 0x`, ` sp 0x`), and
the neighbouring `SHELL_CHECK_MARKER_SLACK` prices the intrusion in the same file: *"one kernel
fault report, three lines and about 150 characters"*, with 400 bytes of slack allowed *"with room to
spare"*.

**That narrows the problem sharply and it should be the first thing calef is told.** This is not a
kernel that chatters over userspace. In normal operation it writes **nothing**; the entire collision
surface is one three-line fault report per faulting user thread, plus whatever a panic produces.
Options that would be absurd for a chatty writer are reasonable for this one.

## The options

| | shape | what it costs, and where the cost is unmeasured |
|---|---|---|
| **A** | **A second port.** The kernel keeps a UART of its own; the server owns the other. | Zero coupling, nothing to arbitrate, and a panic path that cannot be starved. It is a hardware fact per board whether a second usable port exists, and this tree has not established it: `notes/uart.md` records two UARTs on the Pi and nothing equivalent for argon, radon or xenon. A bench session answers it; nothing here does. |
| **B** | **A buffer the server drains.** The kernel appends; the server interleaves at line granularity. | Serves the ordinary fault report well and serves the case that matters worst. A panic is exactly when the draining server may be the thing that died, so a buffered panic is a panic nobody reads. Any B has to carry a direct-write escape, at which point it is C with extra machinery. |
| **C** | **A claim the server takes, which the kernel respects except in a panic.** | One flag and one exception, and it matches what the tree already measured: the kernel writes nothing until a fault, so "respect the claim" costs nothing in the common case. The exception is where every argument will be, since a fault report is not a panic and the two want different answers. |
| **D** | **Leave it, and record the limitation where the reader meets it.** | Free today and it is what the tree does. The cost is already being paid by every bench log and by `script/shell-check`'s own `BUGS`, which describes the interleaving as a live defect in the system rather than in the script. |

**No recommendation, deliberately.** The choice *"binds every architecture and every future console
consumer"*, which is the irreversible column, and the one measurement that would decide between A
and C (whether each board has a second usable port) has not been taken. A proposal that recommended
without it would be arguing where a bench session would do.

## What would settle it, and it is two evenings rather than a lane

1. **Whether argon, radon and xenon each have a second usable serial port**, read off the boards.
   That is what makes A real or removes it.
2. **What a panic costs under B and C**, which is the constraint that probably decides it and which
   can be reasoned from the fault path without hardware.

## What is blocked until this is answered

**Milestone 342.** And it is already load-bearing somewhere it cannot be fixed: milestone 243's
`BUGS` points at a home for this question, which is why this section exists, since a citation to
nothing is the tell AGENTS.md names for being on too low a rung.
