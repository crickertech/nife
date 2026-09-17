# This kernel keeps only the first IO APIC the MADT lists, so half a multi-socket machine's interrupts have nowhere to go

**Status: PROPOSED 2026-09-16.** Written by milestone 308's lane, from that block, while writing the
`BUGS` entry recording that 308's fix ships unexecuted. The gap is larger than the defect 308 closed
and was found by asking what "unexecuted" actually covers.

**Gate: NONE.** Nothing blocks a lane starting this except that no machine here has two IO APICs, so
it would be built against QEMU's `-machine q35` with more than one `ioapic`, or not at all until
xenon or a borrowed two-socket box can boot it. That is a real constraint on the *evidence*, not on
the work.

**In brief.** `arch::x86_64::machine`'s MADT walk records the first IO APIC it finds and discards
every other one:

```rust
if into.io_apic.is_none() {
    into.io_apic = Some((id, address, gsi_base));
}
```

`irq.rs` then keeps exactly one part's base, entry count and MMIO address in three statics, and
`route_gsi` panics for a GSI outside that one part's range. So on a machine with two IO APICs, every
global system interrupt owned by the second is not misrouted, which is the good news, and is also not
routable at all. A device on those lines cannot be given to a driver.

**Milestone 308 is a precondition for this and is not this.** Routing by redirection index is what
makes the vector arithmetic correct once a second part exists; it does not make one exist. A reader
of 308's block could reasonably come away thinking the multi-IO-APIC case is now handled, which is
why this is written down rather than left as a sentence in a `BUGS` entry.

## What the work is

- A table of IO APICs rather than three statics, each with its address, `gsi_base` and entry count.
- `redirection_index` answers *which part* as well as *which entry*, and `route_gsi`/`mask_gsi` write
  through the part that owns the GSI.
- The vector assignment has to be decided rather than inherited. `GSI_VECTOR_BASE + index` is
  unambiguous for one part and ambiguous for two, because both have an entry 0.
  `MAX_REDIRECTION_ENTRIES` is 144 and two real parts have 48 entries between them, so the band is
  wide enough for a flat concatenation; that is a policy question and the numbers say it is not a
  tight one.
- `is_device_vector` follows whatever that decision is, and `proofs::
  an_owned_gsi_routes_inside_the_io_apic_band` is the harness that would have to be restated over a
  table instead of over one part. It is already assumption-free over one, which is a good starting
  shape.

## Why it might not be worth doing

**Nothing in this project can boot it, and that is not a detail.** AGENTS.md ranks by the shortest
path to a system a customer runs, and this is machinery for hardware nobody here has, proved by a
model checker and exercised by an emulator. The honest alternative is the proposal's own option 2
from 308, one level out: **refuse a machine whose MADT describes more than one IO APIC, at boot, with
a named reason**, so the failure is a refusal a person can read rather than a device whose interrupts
never arrive. That is perhaps twenty lines and it makes the current limitation a checked fact instead
of a comment, which is what §76's ladder would ask for.

A lane taking this should probably do the refusal first regardless, since it is correct under either
answer.
