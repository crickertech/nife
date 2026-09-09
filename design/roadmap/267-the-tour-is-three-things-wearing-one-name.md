# 267. The milestone tour is three things wearing one name, and only one of them belongs in the kernel

**Status: NOT-STARTED.** Minted 2026-09-09 by calef, from one question: *"Does the milestone tour
need to be part of the build at all? Can the milestone tour just be a userspace program that we run
if we want the milestone tour?"* *(Number provisional until the merge queue lands it.)*

**Gate: NONE.** Everything here is code this project owns, and milestone 266 (one progenitor) landed
first, which is what makes `kernel/src/main.rs` safe to open.

## The three things, which want opposite treatment

Boot output is currently one undifferentiated stream, compiled in or out as a unit. It is three
audiences.

**The machine description** is diagnostics and belongs on **every** boot, never compiled out. Paging,
ISA, firmware, cores, ACPI tables, PCI ECAM, timers, memory map. **xenon proved this is not
decoration**: at first light there was no serial console this project could read, so the boot output
*was* the transcript, read off a photograph, and it is what diagnosed both the local-APIC collision
and the PCI BAR window landing in RAM. A port is verified by reading these lines.

**The milestone narrative** is a demonstration. *"milestone 1: we are running our own code on a CPU
with nothing underneath it."* It is what `script/server` shows a person and what makes this project
legible to a stranger, and it is the 230 lines (`kernel/src/main.rs` 1538 to 1768) that the
`shell` and `initboot` features exist to remove.

**And a genuine kernel remainder** that cannot move anywhere, because it demonstrates facts only
kernel code can establish. It calls `sched::spawn`, reads `sched::preemptions()`, spins the timer,
calls `sched::ipc_recv` directly, and takes the address of the kernel static `USER_FAULTS` to hand to
a userspace program that then faults trying to read it, incrementing the very counter it reached for.
**That demo is the thesis in miniature and no EL0 program can perform it about itself.**

## What this milestone does

1. **Separate the machine description from the narrative**, so the first is unconditional and the
   second is not. Today one `#[cfg(not(any(feature = "shell", feature = "initboot")))]` governs both
   halves of a boot that serve different readers.
2. **Move what can move to userspace.** The EL0-observable demos (a program ran at U-mode, init
   loaded a program and built it as a child, the IPC exchanges) are driven from the kernel because
   that is where the tour lives, not because they need to be. A program that runs them is a program
   somebody can run on purpose.
3. **Keep the remainder in the kernel and say why each survivor is there**, in one place rather than
   spread through 230 lines. A reader should be able to see that the list is short and that each
   entry needs a privilege userspace does not have.
4. **Measure what it costs and what it reclaims.** Kernel symbol bytes before and after, by
   `script/fastpath-footprint`'s method, and the `.text` delta. That number is the deliverable that
   decides item 5.

## What it dissolves, and this is the reason it is worth doing now

**`initboot` and `shell` are the same feature to the kernel.** Six `cfg` sites name them, and every
one names them together; `initboot` appears alone in none. Both mean exactly one thing: *compile out
the milestone tour*. The only real difference is in `xtask`, where the `initboot` arm prints a
different message and passes a differently-named feature that does the same thing.

So the boot-mode features are not three modes. They are **one decision, spelled three ways**, and
that decision is only compile-time because the narrative was expensive to leave in.

**If the narrative leaves the kernel, the reason for the features leaves with it.** That would retire
the naming question `design/roadmap/proposals/what-the-boot-path-is-called.md` is holding, which is
currently a choice between `progenitor-boot` (which distinguishes nothing, since after milestone 266
every boot goes to the progenitor) and `handoff` (generic). **A name is hard to choose here because
the thing may not be a distinct thing**, and this milestone is the test of that.

**Do not retire a feature as a side effect.** If the measurement says the remainder is small enough
that no build needs to exclude it, that is a finding to report and a proposal to file, not a change
to make in passing. `script/lint` lints each boot-mode feature by name and milestone 130 exists
because a copy outlived its reason.

## What must not break

- **The machine description prints on every boot, on all three architectures.** A bring-up on new
  silicon reads it off a photograph. If this milestone makes any of those lines conditional, it has
  failed regardless of what else it achieved.
- **`xtask` has a `Reached(Tour)` checkpoint.** At least one harness treats reaching the tour as a
  state. Find every such consumer before moving anything.
- **`script/shell-check` runs the real boot on both legs** and is the gate that would catch a
  half-moved tour.

## The proof that this milestone worked

**A boot that prints the machine description and nothing else, and a program you can run that prints
the milestone narrative**, with the kernel-only remainder listed in one place and each entry carrying
the reason it cannot leave. Plus the byte count.

## BUGS

- **The narrative is the project's front door and this milestone risks it.** `script/server` is what
  a stranger runs first, and a demonstration nobody sees is worth less than one compiled into every
  boot. Whatever replaces it must be as easy to reach as the tour is today, and "run this other
  program" is a worse default than "it prints".
- **Nobody has measured the 230 lines.** They may cost little enough that the compile-time exclusion
  was never worth its complexity, or enough that item 5 is obvious. This block does not know, and
  the measurement is item 4 for that reason.
- **The split between machine description and narrative is not currently marked in the source.** It
  is one function with a `cfg` in the middle, and deciding which line belongs to which audience is a
  judgement per line rather than a boundary somebody drew.
- **Preemption counts are not proposed for exposure.** A supervisor or benchmark might want them one
  day, and the shape is already established by §139 and milestones 229/237: a per-thread grant, not
  an ambient fact. Nothing needs it today, so nothing is built.
