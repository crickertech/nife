# 208. The x86_64 kernel image ships an RWX segment, and it is the reason a second ELF reader exists

**Status: NOT-STARTED.** Minted 2026-08-31, found by milestone 196's (a physical address on
`elf::Segment`) lane while trying to delete a duplicate ELF parser. *(Number provisional until the
merge queue lands it.)*

**Gate: NONE.** The change is one linker script and the parser that refuses the image already
exists; nothing here needs a ruling to start.

**In brief.** `kernel/link-x86_64.ld` folds `KEEP(*(.text.boot))` and `KEEP(*(.data.boot))` into one
output section, so the 32-bit trampoline ships as a **single `RWX` `PT_LOAD` at `0x101000`.**

**`crates/elf::Elf::parse` refuses that image outright**, with `Error::WritableAndExecutable`. So the
kernel's own image violates the W^X rule that `crates/elf` and `paging::Flags` enforce everywhere
else in the tree.

## How it was found, which is the useful part

Milestone 196's premise was that `uefi_loader` carries its own forty-line ELF reader because
`elf::Segment` exposed no physical address. **That was true and it was not the blocker.** The lane
added the field, then measured rather than assumed: it built the kernel for `x86_64-unknown-none`,
dumped the program headers, and ran the real bytes through `Elf::parse`.

`REFUSED: WritableAndExecutable`. Patching that one segment's `p_flags` from `RWX` to `RX` in a copy
made **`Elf::parse` accept the whole file** — all ten `PT_LOAD`s, the three `NOLOAD` reservations,
and the trampoline's split addresses. Nothing else in the validating parser objects.

So the duplicate reader exists because of a W^X violation, not because of a missing field, and
nobody knew that until somebody tried to delete it.

## The fix, and why it is worth doing for its own sake

**Split `.boot` into an RX text section and an RW data section.** One linker-script change.

It is independently a **security fix**: this project's whole claim is about what a confined component
can reach, and an image that ships writable-executable memory is a claim the tree makes everywhere
and breaks in its own boot path. The parser refusing it is the tree's own rule catching the tree.

Two things it unlocks, which is why it is filed rather than folded into 196:

- **`uefi_loader` can then delete its forty-line reader**, which is AGENTS.md rule 7's target: two
  readers of one format, agreeing by comment rather than by type.
- It removes the temptation to add a **non-validating** entry point to `crates/elf` to work around
  the refusal, which would be new public API plus a name and would defeat the parser's purpose.

## Why its own block rather than reopening 196

Milestone 196 is a `crates/elf` struct-field change. This is a kernel linker script and boot-path
change with security value of its own, and folding a W^X image fix inside a struct-field milestone
would hide it. It also lands in files milestone 87's (the x86_64 bare-metal machine) work touches, so
it wants sequencing rather than a merge conflict.

## BUGS

- **Nothing checks that the shipped image is W^X clean.** `crates/elf` refuses such an image when
  something parses it, and nothing parses the kernel's own image in any gate, which is why this
  survived. A check would be the mechanism; this block does not build one.
- **Only x86_64 was measured.** Whether `link-aarch64.ld` and the riscv64 script have the same shape
  is unchecked, and DECISIONS §19 (architectural parity is a tenet) says the answer matters.
- **The trampoline's addresses are unusual and worth reading before editing**: `p_vaddr` `0x8000`
  against `p_paddr` `0x12b000`, because its bytes ship high (`AT()` places them after `.rodata`) and
  execute low (a STARTUP IPI can only name a page below 1 MiB). The image has three different
  vaddr/paddr relationships and this is the strangest.
