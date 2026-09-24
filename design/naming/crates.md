# Naming crates

*An appendix to [`design/naming.md`](../naming.md), which is the rule. This file holds the crate rule and how far it reaches, nouns over verbs, the acronym history, and what `crates/` holds. It exists to verify or challenge the main page, and a reader who only needs to name, ratify or rename something should not have to open it. The directory `design/naming/` and this file's stem are provisional names, minted 2026-09-24 by the lane that split the file; naming is calef's.*

## Crates

### The one rule, and who applies it (2026-08-01)

**`snake_case`, everywhere, with no second tier.** Crates already did this (`fs_proto`, `user_mode_runtime`);
programs did not, and **0 of 57** carried an underscore, so multiword names were squished. The three
worst were `fsclient`, `sysinit` and `credcli`; they are `fs_test_client`, `system_initializer` and
`credentialer_test_client` now (milestone 63).

An earlier draft had two tiers: short names for programs a user types, underscores for programs only
the system spawns. It was rejected, and the reason generalises. **The category is not a stable
property of a program.** `wc` was internal plumbing and became a prompt-typed pipeline stage inside a
day, and a convention keyed to something that changes produces renames. It is also not how Unix got
its names: the terseness of `ls` is emergent pressure on words people type constantly, not a rule
anyone wrote down, and codifying an emergent property turns it into a classification chore every
contributor has to get right.

So one rule, no branch. A short name for a typed command is a *choice its author makes*, not a
convention to apply; nobody needs a rule to know `wc` beats `word_count`.

**calef names the crates, the programs, and the shared modules.** Same shape as
`design/decisions/` section numbers: global to the tree, so decided by the person who can see the whole tree. A lane
ships a **provisional** name, says so in its report, and expects it to change. Nobody renames on
their own initiative either, because a rename is a naming decision with extra steps. The reason is
that names are what make this OS legible to humans and to LLMs, and in a capability system a name is
often the only thing that says what a program can *do*.

#### Why it needed a rule, and how far the rule reaches

The evidence is the tree itself, and every one of these was a locally reasonable choice by whoever
was mid-task. `dwarden` is named for what it **holds** while its two siblings are named for what they
**serve**, so a reader who correctly infers the scheme gets it wrong. `conx` has no recorded
expansion anywhere: not in §41, not in [live-replacement.md](../../notes/live-replacement.md), not in the commit
that introduced it. `cseam.rs` sat among 48 programs and was not one; it was a shared module.

**Crates came into scope on 2026-08-01**, and they are the most reader-facing names in the tree: a
newcomer greps `crates/` before they ever open `components/src/`, and a crate name appears in every
`Cargo.toml` that depends on it, in every `use` statement, and in the dependency graph an outsider
reads to understand the shape of the system.

**Shared modules came in for a reason of their own.** `user/src/`, the directory milestone 175
split into `components/src/` and `fixtures/src/`, used to hold 48 `[[bin]]` programs
and a handful of modules compiled into them with `#[path = "..."] mod ...`, with **nothing in the
naming distinguishing them**, so a reader who tried to run `cseam` was misled by the directory.
`AGENTS.md` rule 7 retired that category the same day: what two binaries share is a crate, and what
remains beside the programs is single-consumer submodules (`net_transport`,
`socket_test_client`), which
are ordinary Rust. `script/lint` counts consumers per `#[path]` target and fails at two. A shared
module's name still has to answer a question a program's name never raises, which is *"where does
this get compiled into?"*, and that makes it a naming problem of its own rather than a smaller
version of the program one.

The count in an earlier draft of that paragraph said "three modules" and was wrong: the grep that
produced it matched only single-line includes, and several were two lines. **Take a count from the
merged tree, with a pattern you have checked against the real shapes.**

**Public function and method names came in on 2026-08-23** (milestone 160), after a crate-naming pass
across everything the kernel depends on raised the natural next question. A function name is more
reversible than a crate's, typically fewer call sites and all of them inside one crate, so the
"recommend on reversible forks" latitude applies more freely there than it does one level up.

#### Nouns, not verbs, and the three crates that were settled by it

A crate, a program or a module is a *thing*, so it takes the name of a thing. A verb names an action
and a namespace is not one, which is audible at the call site: `line_edit::expand_output` reads as an
instruction where `line_editor::expand_output` reads as a location. The exception is a **term of art
that happens to be a verb**, where the word is the one the field already uses; `bind` (§50) is Plan
9's, and respelling it as a noun would assert novelty where there is none, which is the
standard-terms guard rail rather than a hole in this rule.

Three crates predated the rule and were settled by it the day it was written: `compose` became
`compositor`, `measure` became `measured_boot`, and `dma_validate` became `dma_validator`. **Each had
named itself a noun in its own first line while carrying a verb as its name**, which is what makes
the test worth applying rather than merely stating. (`dma_validator` has since been deratified by the
acronym test above, on the other half of its name.)

#### The domain table's own argument

`AGENTS.md` carries the table of which form each domain takes. Two things about its shape are worth
keeping here, because both were argued.

**The splits are *across* domains on a stable property**, not within one on an unstable property.
That is the difference calef identified when he rejected the two-tier rule: `wc` moved from internal
plumbing to prompt-typed pipeline stage inside a day, where a file either is a Cargo target or is an
executable in `script/`, and `script/test` will never become a `[[bin]]`.

**And it is the standard-terms guard rail applied to form rather than to vocabulary.** We do not
rename `elf`, and we should not respell `supply-chain` either: a name whose *shape* a reader already
knows from outside this project costs them nothing.

The directory rows are that same principle one level out rather than a new tier, and the three
directories that violated them are in [Directories](programs-scripts-and-directories.md#directories-milestone-63) below.

**Standard terms are already right and must not be touched.** `elf`, `pci`, `paging`, `glob`,
`socket_protocol` are names a reader knows from outside this project, so they cost nothing to learn.
This tenet is a naming authority, not a renaming mandate, and renaming `elf` would destroy the
recognition the whole thing exists to buy.

**But an acronym is spelled out unless its expansion teaches nothing** (calef, 2026-09-05), and this
paragraph used to protect four names that fail that test. **The test is an asymmetry**: a reader who
knows the term recognises its expansion instantly, so spelling it out costs the expert nothing and
saves the newcomer a bounce. The acronym only ever helps the reader who was already fine.

**It was settled by the architect not knowing one.** `mmu::bar_window` was defended on 2026-09-05
with the argument that `BAR` was the vocabulary the retired `PCI_BAR_PHYS` already used. calef asked
what BAR stood for, which is the whole answer: **an argument from this project's own history is not
an argument that a reader knows the word.** He then named the general form, and deratified `dma` in
the same breath: *"we want accessible names which means spelling out acronyms except in the most
obvious cases because the knowledgeable reader would recognize the expansion of an acronym they
knew."*

**What survives and what does not.** `pci` expands to peripheral component interconnect and the
reader is no wiser; `elf`, `paging` and `glob` are the same. **`dma`, `dtb`, `gpt`, `ipc` and `asid`
all expand into something more informative than themselves** (direct memory access, device tree
blob, GUID partition table, inter-process communication, address space identifier) and are therefore
deratified. `dma_validator` was ratified by calef on 2026-08-01, before the test existed, and this
overturns that rather than quietly dropping it from a list.

**The sweep is its own milestone**, because `ipc` in particular is load-bearing across the tree and
a rename that size is not a side effect of a rule change.

**One constraint:** `nifefs` caps archive names at `NAME_LEN = 32` bytes, so a program's name is
bounded. Crates are not in the archive and are unbounded.

It was 24 until 2026-08-01, when it had started deciding names rather than bounding them: two settled
names were within four bytes of it and `os_primitives_benchmarker` exceeded it. Raising it costs
directory entries per block, and nothing else now that `Fs` no longer holds an entry array. See
[nifefs.md](../../notes/nifefs.md) for the numbers. The rule that survives the raise: **do not let the
limit pick a name, and do not spend a format change on bytes nothing needs.** 32 clears the longest
settled name by seven bytes, which is a budget rather than the three bytes that were left before.

## What `crates/` holds

`crates/` holds four audiences under one directory, and **naming does not distinguish them**, which
is a known gap rather than a decision.

- **Kernel logic**, host-tested and Kani-reachable: `capability`, `paging`, `page_frames`,
  `memory_regions`, `generational_table`, `address_space_identifier`, `intrusive_fifo`,
  `inter_process_communication`, `dma_validator`, `measured_boot`, `user_mode_heap`.
- **Wire contracts**, spelled `*_protocol` and checked for it by `script/lint`:
  `filesystem_protocol`, `socket_protocol`, `byte_sink_protocol`, `credential_protocol`,
  `clock_protocol`, `entropy_protocol`, `graphics_protocol`, `environment_protocol`,
  `login_protocol`, `network_time_protocol`, `supervision_protocol`,
  `swap_protocol`, `counter_frequency_protocol`, `capability_witness_protocol`. Plus `abi`, which is
  the syscall boundary and predates the suffix.

  **The suffix was `_proto` until milestone 265** (calef, 2026-09-05, on being shown
  `timebase_proto`: *"I think `_proto` was lazy on my part. It should have been `_protocol` globally
  to differentiate from prototype."*). `proto` is a truncation rather than an abbreviation, and it
  is equally short for `prototype`, which this tree uses for a real thing. Wherever a dated passage
  below spells a crate `_proto`, that is what it was called then and the passage is left alone.
- **Format and hardware parsers**: `elf`, `device_tree_blob`, `pci`,
  `globally_unique_identifier_partition_table`, `nifefs`.
- **Userspace libraries**: `user_mode_runtime`, `grant_plan`, `virtio`, `video_terminal`, `line_editor`,
  `bitmap_font`, `glob`, `calendar`, `credentialer`, `compositor`, `coremark`, `c_seam`.

**`compositor` and `line_editor` are the two that look like contracts and are not**, and an earlier
version of this section listed them as such. Both are *logic* crates that happen to contain a
protocol module: `compositor` is the scene, the clipping and damage-rectangle arithmetic, and the
composition itself; `line_editor` is a sans-IO editor with a `line_editor::proto` inside it. Renaming
either to `*_proto` would promise a wire definition and deliver an algorithm, which is exactly the
kind of claim §39 is about. The `*_proto` check is right to leave them alone.

What the names actually do, over the 39 directories under `crates/` **on 2026-08-01**, when this
census was taken. It has not been re-taken (there were 68 on 2026-09-19), and the names in it are
the ones those directories had that day:

- **One word where one word will do**, which is 21 of the 39: `abi`, `capability`, `compositor`,
  `elf`, `frames`, `ipc`, `paging`, `regions`, `slots`, `virtio`.
- **Underscore when the two halves are separate concepts** and the name reads as a qualifier applied
  to a thing, which is the other 18: `filesystem_protocol` is the protocol *for* the filesystem,
  `graphics_protocol` the protocol *for* graphics, `dma_validator` the validation *of* DMA, `user_mode_runtime` the runtime *for* user mode,
  `user_mode_heap` the heap *for* user mode, `measured_boot` the measurement *of* boot.

**Milestone 63 deleted the third bullet, which used to read "run together when the result is one
word".** It was a real observation (`capsh`, `lineedit`, `uheap`, `crickerfs`, `bitfont`), and it was
the rule that produced every abbreviation a reader had to decode. **Two of the five survive it, not
one**, and this sentence said otherwise until milestone 115 checked the history: `crickerfs` (now `nifefs`, milestone 120) stayed
with a reason, because `procfs` is the shape of a filesystem name outside this project and nobody
writes `proc_fs`, and **`bitfont` stayed with none**, having never been renamed at all, until the
kernel-dependency crate naming review ratified it as `bitmap_font` on 2026-08-23. Three moved,
to `grant_plan`, `line_editor` and `user_mode_heap`. The boundary that
remains, between one word and two, is judgement, and the guard rail is that a **standard term keeps
its standard spelling** (see above).

The one place it became a real inconsistency is worth fixing and is checked: **the wire contract was
spelled four ways** (`fs_proto`, `gfx_proto`, `netproto`, `line_editor::proto`) for one concept,
`gfx_proto` at the time; the 2026-08-23 review made that one `graphics_proto`, and milestone 265
made it `graphics_protocol` on 2026-09-14.
`*_proto` won for crates, because it is what the actual crates already were, and `socket_proto` has
since graduated from a module inside `net_stack` into a crate under that name. The suffix itself
then lost, three weeks later and to its own author: see the `_protocol` note above.

**A crate that is a component's engine takes the component's name** (`line_editor` the sans-IO
editing crate, `line_editor` the binary that wires it to endpoints; `compositor` and `coremark` are
the same pair). They are the same thing at two layers, and giving the engine a second name is how
`termd`/`linedisc` happened in the first place. Where a note needs to tell them apart it says "the
`line_editor` crate" and "the `line_editor` binary".

**One pair deliberately does not share a name**, and the reason is worth keeping: the crate is
`video_terminal` and the program that wires it is `display_terminal`. The crate is named for the
**protocol** it implements (the VT standard, bytes in and a character grid out) and the program for
its **role** (the terminal on the display, next to `gpu_driver`, the virtio-gpu driver it is a client
of). Both facts are true and neither name says the other.
