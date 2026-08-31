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

**Decided 2026-08-31: widen it, as a plain `u64`.** The field is in the format and the crate is
simply not exposing it, which makes this a gap rather than a design change.

**And the typed version is deliberately not done here.** Making the virtual/physical confusion
unrepresentable is milestone 200 (a virtual address and a physical address stop being the same
type), which is tree-wide and ratified its own names. Typing `elf::Segment`'s two fields alone would
claim a distinction the rest of the tree does not make, and a newtype whose value every consumer
immediately unwraps is not a mechanism. So this milestone adds the field with the hazard in its doc
comment, and milestone 200 makes the mistake unsayable everywhere at once.

## BUGS

- **The field is redundant in every case but one**, and that is the hazard rather than a comfort:
  `paddr == vaddr` for every user program in this tree, so a consumer that reaches for the wrong one
  behaves correctly in testing and wrongly on the one path that matters, which is the kernel image
  where `.ap_trampoline` sits at physical `0x8000` against virtual `0x165000`. The doc comment has
  to carry that until milestone 200 lands, and a doc comment is rung three.
