# `map_everything` fails on a machine large enough for RAM to reach the framebuffer

**Status: PROPOSED 2026-09-05.** From milestone 87's first light on xenon.

**Gate: NONE.** It reproduces from a memory map; no board is needed to fix it, and one is needed to
confirm the fix.

**What happened.** nife booted on xenon, a Dell OptiPlex 7050 with **17,119 MiB of RAM**, got as far
as ACPI, SMP, PCI and the timers, and then:

```
[PANIC] panicked at kernel/src/arch/x86_64/mmu.rs:325:33:
failed to build the kernel page tables: AlreadyMapped
```

Photograph: `art/bench/xenon-2026-09-05-first-light.jpg`. It is the only transcript, because the
machine has no serial cable attached and milestone 243's screen console is what carried the tour.

**The leading hypothesis, and it is a hypothesis.** Milestone 243 added a framebuffer mapping to
`map_everything` on 2026-09-04, after every RAM region has been direct-mapped:

```rust
if let Some((base, size)) = memory::framebuffer() {
    direct_map(m, base, base + size, Flags::device())?;
}
```

The aperture on this machine is at **`0xd0000000`** and it has 17 GB of RAM. Under OVMF at 2 GiB the
aperture sits far above every RAM region and the two mappings cannot meet; with this much RAM they
can, and `direct_map` refuses a frame it has already mapped. **The failure would then appear only
where RAM is large enough to reach the aperture**, which is a machine no emulator run in this tree
has modelled: `notes/x86-uefi-boot.md`'s own table is written at `-m 256M` and the runner defaults to
2 GiB.

**It is not confirmed, and the first thing the fix owes is the evidence.** `map_everything` maps
several other things (the low megabyte, the AP trampoline's LMA, the VT-d window), and
`AlreadyMapped` names the symptom without naming the two ranges that collided. **A message that says
which two would have made this diagnosable from the photograph**, and is worth more than the fix.

## What to do, in order

1. **Make the error say what collided.** `direct_map`'s refusal should carry the range being mapped
   and the mapping already there. That is the difference between a bench session that ends in a
   diagnosis and one that ends in a hypothesis.
2. **Reproduce it without the board.** The memory map is on the photograph and `map_everything`'s
   inputs are `memory::ram_regions()` and `memory::framebuffer()`, so a host test over a synthetic
   17 GB map with an aperture inside it should fail today and pass after.
3. **Then decide what the right behaviour is**, which is not obvious and is the real work. An
   aperture that overlaps a RAM region is not a contradiction: firmware reports the region as RAM and
   the adapter answers at the same physical addresses. The choices are to skip frames already mapped,
   to remap them device-typed, or to exclude the aperture from the direct map, and **they differ in
   what a stray kernel pointer into that range does.**

## BUGS

- **The hypothesis is from reading, not from running.** If the collision is between two other ranges
  the whole account above is wrong and only item 1 survives.
- **A fix confirmed under QEMU is not confirmed on xenon.** The runner's memory sizes do not reach
  this case, and `-m 17G` on a machine with 16 GB of host RAM is its own problem.
- **This is milestone 243's regression on a machine 243 could not see**, which is an argument for
  something rather than a reproach: the tree has one x86_64 machine and it had never been booted.
