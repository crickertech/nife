# The roadmap: one file per milestone

`script/roadmap` is the index, computed from these files, so nothing here lists them. A block opens
with its number, title, status and gate, and its `## Follow-on` bullets each carry a disposition;
`script/roadmap --check` says which forms it accepts, and
[`notes/follow-on-work.md`](../../notes/follow-on-work.md) says why. Work with no number yet goes in
[`proposals/`](proposals/README.md).

For the shape rather than the list: milestone 7 (user mode: EL0, capabilities, the ELF loader, and
IPC) is the dividing line between a Rust program that boots and an operating system.

*Name: recorded (milestone 294). Milestone 294 (`design/roadmap/README.md`'s index is generated,
not hand-maintained) retired the hand-kept index this file used to be. The stems are milestone
slugs and are not tracked by `script/names`: calef ruled milestone titles drafts on 2026-08-04.*
