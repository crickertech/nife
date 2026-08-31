# 196. A physical address on `elf::Segment`, or a second ELF reader forever

**Status: NOT-STARTED.** Minted 2026-08-30 from milestone 87's (the x86_64 bare-metal machine) lane.
*(Number provisional until the merge queue lands it.)*

**Gate: DECISION.** `crates/elf` is a shared definition, which AGENTS.md rule 7 makes global to the
tree and therefore calef's, not a lane's.

**In brief.** The UEFI loader carries its own forty-line ELF reader for one reason: `crates/elf`'s
`Segment` exposes no `p_paddr`, and in this kernel `p_vaddr` and `p_paddr` are genuinely unrelated
(`.ap_trampoline` is at `0x8000` against `0x165000`). A loader placing an image in physical memory
needs the physical address, so it re-implements the parse.

**Two readers of one format is the defect**, and it is the shape rule 7 exists to prevent: the
agreement between them is a comment, not a type.

## What is being decided

Whether `elf::Segment` widens to carry `p_paddr`. It is a small change to a crate the kernel's own
loader depends on, which is what makes it calef's rather than small.

- **Widen it.** One field, one parse site, and the loader deletes forty lines. Everything reading
  `Segment` keeps working.
- **Leave it.** The duplicate reader stays, and the next thing that loads an image physically writes
  a third.

**Recommend widening**, on the grounds that the field is in the format and the crate is simply not
exposing it, which makes this less a design change than a gap. Recorded as a recommendation because
the fork is reversible; the call is still his.

## BUGS

- **Nobody has checked whether other `Segment` consumers would be confused** by a field that is
  equal to `p_vaddr` in every case they handle. A field that is almost always redundant is a field
  someone will eventually use wrongly.
