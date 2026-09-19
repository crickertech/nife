# A checked direct-map reader for the x86 ACPI walk

**Status: PROPOSED 2026-09-17.** Found by milestone 319, which proved the parsing half and so
narrowed what the volatile half is actually for.

**Gate: DECISION.** It is a new accessor at a trust boundary, so its shape and its name are calef's.

## In brief

Milestone 304's lane named `kernel/src/arch/x86_64/machine.rs`'s ACPI walk as its strongest
remaining target and declined to take it: 836 lines over firmware-supplied lengths, checksums and
counts, whose volatile half reads raw pointers into the direct map. It called finding that seam a
design question rather than a mechanical one, and was right.

**Milestone 319 changed the size of the question rather than answering it.** With
`crates/machine_discovery` proved, every decision about what those bytes *mean* now lives in a crate
Kani reaches. What is left on the kernel side is one operation repeated: read N bytes at a physical
address through the direct map and hand them to a parser. The 836 lines are mostly the walk's
sequencing, and the part that is a trust boundary is a single accessor.

## The question

**Does that accessor take a bound, and what does it do when the bound is exceeded?**

Today each read is a raw pointer plus a length the table itself supplied. An RSDT whose `length`
field says 64 KiB is a 64 KiB read at whatever physical address the RSDP pointed at, and nothing
between the firmware's number and the dereference says how much of the direct map is legitimately
readable.

The analogous question is answered on the other two architectures. `device_tree_blob::DeviceTreeBlob::from_ptr` takes the
blob's own length, validates the header before anything else is read, and every reader below it is
proved total (`be32_is_total`, `be64_is_total`, milestone 18). The x86 walk has no equivalent
because ACPI is not one blob: it is a linked structure of independent tables, each reached from a
physical address inside another one.

## What is in scope

- A bounded read against the direct map's own extent, so a firmware length cannot name memory
  outside it.
- Whether the bound is per-read or a region capability the walk holds, which is the part that could
  go either way and is why this is a decision.
- The accessor's name, which is calef's and which this proposal deliberately does not guess at.

## What is out of scope

The parsers. They are proved (milestone 319, fifteen harnesses), and three defects came out of that;
nothing here should re-litigate what a table means.

## What is blocked until it is answered

Nothing is blocked. This is the last unproved reach of the x86 discovery path, and it is the one
place left where a firmware number reaches a dereference with no proved step between.
