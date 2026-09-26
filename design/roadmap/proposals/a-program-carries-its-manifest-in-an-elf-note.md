---
status: PROPOSED
raised: 2026-09-26
milestone_dependencies: none
decision_dependencies: unwritten
machine_requirements: none
specific_machine: none
needs_person: no
---
# A program carries its manifest in an ELF note

Raised by the `maintainer/m2-ruling` lane, which recorded calef's
ruling on DECISIONS §197 (a package is one archive file)'s manifest question and built none of it.
The slug is a lane's coinage and provisional, like every name below.

The owner string, the type number and the descriptor's encoding are names and a
wire format two programs agree on, so they are an architect's. The build cannot start until they
are ruled. Everything else here is reversible code.

## What was ruled

calef, 2026-09-26 (UTC): *"M2 is right."* A program's manifest travels inside its executable, as an
ELF note found through a `PT_NOTE` program header. §197 has the ruling and its reasons. Draft pull
request #1319 (the price of an ELF-note manifest) is the evidence, and it carries a working
prototype of the reader.

## What is still open, and whose

- The note's owner string. #1319 proposes `nife`. Nothing has chosen it.
- The note's type number.
- The descriptor's encoding: what bytes carry `grant_plan::Manifest`, and how a version is told
  apart from the next one.

## The build, once those are named

1. `crates/user_mode_runtime/link.ld` keeps the note instead of discarding it. Today its
   `/DISCARD/` line drops `*(.note*)`. #1319 measured the fix as two lines: a `PT_NOTE` entry in
   `PHDRS`, and an output section that keeps the manifest note inside the rodata load segment.
   `helpers/build-ripgrep.sh` derives its own copy of this script and needs the same change.
2. A way for a program to emit its note: a macro or a `#[used]` static in a named link section,
   whose bytes are the ruled encoding of the program's manifest.
3. The reader from #1319 (`crates/elf/src/note.rs` on `lane/elf-note-price`), landed with its
   three Kani harnesses, three falsifications, host tests and the `elf_parse` fuzz extension. It
   refuses the same owner and type appearing twice, so the shell and the progenitor cannot read
   different manifests from one file.
4. The shell and the progenitor read an installed program's manifest from its note, in place of
   `grant_plan::INSTALLED_MANIFEST_OF`. That constant is the provisional ceiling #1320 (run an
   installed program by its bytes) shipped: every installed program is endowed as `uptime` is. Its
   callers are in `crates/system_initializer` and `crates/grant_plan/src/spawnproto.rs`, and
   `crates/activation_set` names it in a comment.

## What it retires

`notes/packages.md`'s two BUGS entries that say an installed program's manifest is `uptime`'s, and
the one that says the writable activation set stops being harmless "the day a manifest travels".
That last one gets worse, not better, when this lands, so the installer that moves the table out of
a session's reach should land first or with it. Milestone 198 (a package manager) rung 3a is the
consumer.
