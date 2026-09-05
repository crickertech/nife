# A BAR window sized by the bus rather than by a constant

**Status: PROPOSED 2026-09-04.** Written by milestone 256's lane. Provisional slug; the integrator
mints the number.

**Gate: NONE.** Both halves are reachable from code that already runs: the sizing pass exists in
`pci::read_bars` and needs moving earlier, and the leaf size is a value in `crates/paging`'s format
trait. Nothing here needs hardware, and the trigger for starting is a boot line rather than a
machine (see "What would force it").

## What

`arch::x86_64::mmu::PCI_BAR_MAPPED` is 2 MiB. Milestone 256 made the window's **placement** the
machine's answer and left its **size** a constant, so a machine whose genuinely unassigned BARs want
more than 2 MiB exhausts it. `pci::place_bars` says so and returns false, which fails the device
rather than corrupting anything, so this is a limitation and not a defect.

## Why it is not urgent

Adoption (milestone 256) is what took the pressure off. On real firmware almost every BAR arrives
already placed and is adopted where it stands, so the window only has to hold what the kernel itself
assigns: on xenon, two functions of fifteen. Under QEMU every BAR arrives unassigned and 2 MiB has
covered them all since milestone 161. **Nothing measured has come close to exhausting it**, which is
why this is a proposal rather than a milestone.

## What would force it

A machine with several unassigned functions, or one unassigned function with a large aperture (a
discrete GPU's 256 MiB BAR is the obvious case). Either would print `BAR window exhausted` on the
boot line, which is the trigger to read this file.

## What it would cost, and the part worth deciding

Widening the constant is one character and the wrong answer, because **every byte of the window is
mapped with 4 KiB leaves at boot** (`arch::x86_64::mmu`'s first recorded BUG). A 256 MiB window is
65,536 leaves, half a megabyte of page tables, for space that mostly decodes nothing.

So the two halves are:

1. **Size the window from the bus**: sum what the unassigned BARs actually ask for, which needs the
   sizing pass to run before the fine map is built rather than at bring-up time.
2. **Map it with 2 MiB leaves**, which is a change to `crates/paging` (a leaf size in the format
   trait) that all three architectures want and that the same BUG already names.

The second is the larger piece and is not x86's alone, which is the thing to decide first: whether
this is one milestone or the x86 half of a paging one.
