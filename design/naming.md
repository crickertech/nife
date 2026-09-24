# Naming things in nife

What the tree's names mean, which conventions are rules, and which of those a machine checks.

Written at milestone 46, alongside the rename that made four of the names honest. The headline rule
and its argument are [DECISIONS §39](../design/decisions/39-component-names.md); this note is the working reference and covers
the parts §39 does not: crates, scripts, where a document goes, and the two numbering schemes that
look alike and are not.

**This document is the authority for naming conventions** (`design/decisions/` §155, 2026-09-18),
and it moved here from `notes/` on the same ruling, because a file that is normative is not a note.
`AGENTS.md` keeps only the authority itself, which is the part a lane must act on without opening
anything: **names are calef's, ship a provisional one and say so, never rename on your own
initiative.** Everything else (the spelling conventions, the acronym test, nouns over verbs, the
failure modes, how to perform a ratified rename) is here. **Where the two disagree, this file is the
rule for conventions and `AGENTS.md` is the bug**, which is the reverse of what was true before §155.

**This overturns milestone 262 by finishing what it started.** On 2026-09-05 the rule was written
out twice, in full, in both files, and 262 shrank that to one statement per rule in `AGENTS.md` with
the case for it here. The residue was still duplication: when the acronym test changed on 2026-09-18
it again had to be edited in both places in one commit, and nothing compares the two. §155 removes
the duplication rather than shrinking it, and buys back 58 lines of a constitution that is on a
budget (milestone 118). What makes that safe is the **provisional name**: a lane that has never read
this file invents a name that is expected to change, so the conventions are needed at ratification
and at rename, which are the unhurried moments, not at invention.

## The rules a lane applies

Moved here verbatim from `AGENTS.md` by §155; the argument for each is further down this document.

new crate, program or module ships a **provisional** name, says so in its report, and expects it to
change; the integrator surfaces it. Never rename on your own initiative, because a rename is a naming
decision with extra steps. A function name is more reversible than a crate's, typically fewer call
sites and all inside one crate, so the "recommend on reversible forks" latitude applies more freely
there than one level up. **Performing a ratified rename has its own rules**, because the cheap
edit is what destroys the expensive record: status decides what moves (a `BUILT` block is an account
and keeps the old name, a `PROPOSED` one is live intent and moves), a quotation never moves, and you
enumerate before sweeping. "Performing a ratified rename" below has the worked example and what is not gateable.

**The three failure modes to name against.** **Abbreviations** that need a decoder (`capsh`,
`uheap`, `vt`). **Generic words** that could name almost anything in an operating system (`compose`,
`measure`, `regions`, `slots`, `caps`, `frames`). And, on the other side of the line, **standard
terms a reader already knows from outside**, which are the best names available (`elf`, `pci`,
`paging`, `glob`): this rule is not a licence to rename everything.

**An acronym is spelled out where its expansion is a phrase people actually say, and stays whole
where nobody says it** (calef, 2026-09-18, §154, which supersedes two earlier tests). Ask it again
of any acronym inside the expansion. What decides it is whether anybody *uses* the expansion, not
whether one exists, because only a spoken one is free to the expert. So `device_tree_blob`
and `globally_unique_identifier_partition_table` go, `pci` and `elf` stay (nobody says "peripheral
component interconnect"), `pcie` becomes `pci_express` with no special case, and this **deratifies
`dma_validator`, `nvme`, `gpt`, `dtb`, `ipc`, `asid`** while re-ratifying `pci` and `elf` under it.

**Name things with nouns** (calef, 2026-08-01). A crate, a program or a module is a *thing*, so it
takes the name of a thing: `capability`, `grant_plan`, `user_heap`, `video_terminal`, `line_editor`,
`fs_subtree_caretaker`. A verb names an action and a namespace is not one, which is audible at the
call site: `line_edit::expand_output` reads as an instruction where `line_editor::expand_output`
reads as a location. The exception is a **term of art that happens to be a verb**, where the word is
the one the field already uses: `bind` (§50) is Plan 9's, and respelling it as a noun would assert
novelty where there is none.

**A crate and a program may share a name, and it says something when they do**: the crate is that
program's logic, lifted out so it can be host-tested and Kani-reachable while the program keeps the
IO. `coremark`, `line_editor` and `compositor` are all this pair, and splitting the names would hide
a relationship worth seeing.

### The convention: one rule per domain, and each domain's own

**`snake_case` is the rule for Rust things, not for everything.** Six domains, each keeping its own:

| Domain | Form | Because |
|---|---|---|
| Crates, programs, modules | `snake_case` | Rust's own convention, and what the tree already does |
| `script/` and `helpers/` entry points | `hyphens` | shell commands are hyphenated everywhere (`apt-get`, `pkg-config`, `docker-compose`); an underscore in a command name reads as a mistake |
| Ordinary markdown (`notes/`, `design/`) | `hyphens` | filenames become URL slugs in every static site generator, and hyphens are word separators in a URL where underscores are joiners |
| Repo-root markdown | `SCREAMING_SNAKE_CASE` | **GitHub behaviour, not style.** It recognises `README.md`, `SECURITY.md`, `CODE_OF_CONDUCT.md`, `CONTRIBUTING.md` and links them in its UI; get the name wrong and the Security tab does not find your policy |
| A directory holding a Rust package | named **exactly as the package**, so `snake_case` | the directory and the package are one thing with one name |
| Any other directory | `hyphens` if it needs two words | a directory is a path element, and paths are hyphenated outside this repository |

These are splits *across* domains on a **stable** property: a file either is a Cargo target or is an
executable in `script/`, and `script/test` will never become a `[[bin]]`. **There is no second tier
*within* a domain**, because a split inside one would key on something unstable, which is the two-tier
rule calef rejected. A short name for a typed command is then a *choice its author makes* rather than
a convention to apply, and nobody needs a rule to know `wc` beats `word_count`.

**One constraint to know:** `nifefs` caps archive names at `NAME_LEN = 32` bytes, which bounds a

## The rule everything else is a corollary of

**A name is a claim, and it is made before a reader sees a line of code.** That is the whole
argument. A wrong name is the same defect as a stale comment, except that a comment can be skipped
and a name cannot: every reader of every call site reads it.

Two ways a name can be a false claim, and the tree had both:

- **It claims a model we rejected.** `netd`, `compd`, `gpud`, `termd`. The `-d` suffix says "Unix
  daemon", and a daemon is defined by what it detaches from: no controlling terminal, inherited
  ambient authority, a pid file, started by a privileged init. This OS has none of those. `netd` held
  five explicit capabilities, could not name its own callers, was supervised, and could be reaped by
  something that lacked the authority to build it. About as far from a daemon as a long-running
  process gets.
- **It claims a reader who does not exist.** `linedisc` was the correct Unix term of art. calef did
  not recognise it, and he built the system. That is evidence about the name, not about him. It
  became `lineedit` and then `line_editor`, which someone who has never opened a tty manual
  understands immediately.

So: **name a component for what it is, and prefer a word that parses without prior Unix exposure.**
`spawner`, `console`, `input`, `painter`, `window` were always right, and were always the majority.
The four `-d` names were the outliers.

(`blk` and `kbd` stood in this list until 2026-08-28, when they were renamed to `block_driver` and
`keyboard_driver`. They belonged to the second failure below, not to this one: they parse fine to a
Unix reader and badly to anyone else, which is the whole point the sentence above makes.)

The shell is the one exception, and it is deliberate: it is called **`swish`**, not `shell`, because
shell names are identities rather than descriptions (`bash`, `zsh`, `fish`, `rc`). The argument is in
milestone 63's roadmap block. `capsh` was the obvious candidate and is unavailable: Linux's libcap
ships `capsh(1)`, a capability shell wrapper, so a reader arriving from Linux would assume ours is
that tool.

## Components

A **component** is the shippable unit: one binary in `components/src/`, one `[[bin]]` in
`components/Cargo.toml`, one entry in the initrd archive. A **service** is what a component offers. A **contract** is the wire
protocol it offers it over. "Server" is a fine role word inside a component (`redoxfs_server` serves
the FS service). "Daemon" appears nowhere.

- Lowercase, `snake_case`, no suffix. `net_stack`, `compositor`, `gpu_driver`, `line_editor`,
  `fs_subtree_caretaker`. One word where one word will do, an underscore where the name is a
  qualifier applied to a thing; the 2026-08-01 rule below retired the older "no separators" wording.
- **Never `-d`.** Not `netd`, not a future `logd` or `authd`. Checked.
- **`c_` means "written in C", and it spans two unrelated milestones.** `c_shim`, `c_seam` and
  `c_confiner` are milestone 36's foreign-language seam (DECISIONS §31); `c_swappable` is milestone
  23's replacement demo, the C half of the `rust_swappable` / `c_swappable` pair. The prefix means
  the same thing in both places and the milestones have nothing to do with each other, so do not
  read the four of them as one family.
- **Abbreviate only where the abbreviation is what the field itself calls the thing**, not merely a
  shortening that reads as obvious to whoever typed it: `pci`, `elf`, `dtb`, `gpt`, `ipc`, `asid`.
  If you have to expand it in the doc comment to make the file readable, it was not the ordinary
  name.

  **This clause used to cite `blk` and `kbd`, and they were renamed on 2026-08-28 for failing it**,
  to `block_driver` and `keyboard_driver`. That is worth keeping rather than quietly deleting,
  because a rule whose own examples got renamed is telling you something: the test as first written
  ("is this the ordinary name?") was answered from inside Unix, where `blk` and `kbd` obviously are.
  The survivors are not shortenings at all. `pci` and `elf` are the names of standards, `dtb` and
  `gpt` name formats, `asid` is an architectural term of art. A reader meets each of them outside
  this project and arrives already knowing it. Nobody meets `blk` outside a Unix source tree.

  So the sharper question, and the one to ask of a new name: **would a competent stranger who has
  never read this tree recognise it?** `capsh`, `uheap` and `vt` fail that and are named in
  CLAUDE.md as the abbreviation failure mode. `pci` passes it. Truncating a word you happen to be
  tired of typing is not abbreviation, it is shorthand, and shorthand is what the third principle
  ("a newcomer must be able to succeed without asking anyone") exists to refuse.
- The binary name, the source file name and the archive entry name are the same string. `xtask`'s
  `initrd_aarch64` (`mkinitrd` before 2026-08-27) pairs them positionally in a flat array, so a mismatch is a runtime "program not found"
  rather than a compile error, which is exactly the kind of thing to keep boring.
- **There used to be one deliberate exception**, and milestone 266 closed it: `builder` was packed
  as `init` on riscv64 and `hello` was packed as `init` on aarch64, because `init` was the entry the
  kernel loaded by name. The archive name was a role and the `user/src/` name was the program, which
  meant one string named two binaries. The role is now a program of its own, `progenitor`, and the
  three names agree in every row.

Fixtures and benchmarks (`interrupt_heeder`, `interrupt_ignorer`, `flaky`, `allocator_exerciser`,
`coremark`, `os_primitives_benchmarker`) live in `fixtures/`, which is where milestone 175 separated
them from the real components on 2026-09-13. The naming rule is the same either way: a fixture is a
program and takes a program's name. **`worker` was on that list by repetition and is not a
fixture**: calef ruled it the canonical minimal program on 2026-09-13, so it is
`components/src/least_authority_demo.rs`.

**Two suffixes carry a category, and the distinction between them is real** (milestone 63).
An **`_exerciser`** puts a capability of the system under load and sees whether it holds, with no
contract being probed from outside: `allocator_exerciser` interleaves allocation and free and then
demands a large allocation fit in pages already committed, and `std_exerciser` is an ordinary Rust
program on the native ABI whose three behaviours are chosen by the authority it was granted. A
**`_test_client`** exercises a service contract from outside, with a server on the other end:
`fs_test_client`, `socket_test_client`, `credentialer_test_client`. The `test` is not noise. The
unqualified names (`fs_client`, `socket_client`, `credentialer_client`) belong to the real clients
milestones 54 and 55 will need, and giving them to test programs squats them.

## Shell builtins

A builtin is a word the shell answers itself, and it is the most reader-facing name in the tree
after a program's: it is typed, and nothing but this file records why it is spelled the way it is.
`script/lint` cannot check them, because a builtin is a match arm in `grant_plan::parse` rather than
a file.

The rule is the one the crates already follow with the guard rail intact: **a term of art a reader
already knows from outside this project is the best name available**, so `cd`, `pwd`, `ls`, `mkdir`,
`echo`, `time` and `xargs` are Unix's and were never candidates for renaming. `caps` is ours.

- **`apropos`** (milestone 40 phase 2, 2026-08-16). **Provisional.** Search the installed
  documentation store: `apropos capability` names the pages that mention the word. It is Unix's, and
  it is the same word for the same job in the same architecture (`man` plus `apropos` plus `mandb`
  is the split this whole milestone borrowed), so a reader arriving from anywhere else already knows
  what it does before they run it.

  The roadmap block proposed **`doc search <term>`**, and it was refused for a mechanical reason
  rather than a stylistic one: `doc` is a **program** and builtins are matched before program names,
  so a builtin whose first word is `doc` would shadow the viewer for every line beginning with it.
  A shell where `doc search` and `doc page.md` take different paths through the parser is one where
  a person has to know which class a command is in before they can type it, which is the thing
  milestone 47 deleted the `run` verb to avoid.

  `search` alone was refused as a generic word that could name almost anything in an operating
  system (crates §"generic words", where `compose` and `measure` were caught by the same test), and
  `find` because it is Unix's name for walking a directory tree, which is the one thing this system
  cannot do and this milestone exists because it cannot.

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
expansion anywhere: not in §41, not in [live-replacement.md](../notes/live-replacement.md), not in the commit
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
directories that violated them are in [Directories](#directories-milestone-63) below.

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
[nifefs.md](../notes/nifefs.md) for the numbers. The rule that survives the raise: **do not let the
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

## Scripts

Two directories, on purpose, and the split is by audience.

- **`script/`** is the front door: [Scripts to Rule Them All](https://github.com/github/scripts-to-rule-them-all)
  names, one short file each, **no extension**, lowercase, hyphenated if more than one word
  (`qemu-check`, `ci-qemu`, `toolchain-bump`, `vendor-verify`, `supply-chain`). These are what a
  person types. The canonical set (`setup`, `test`, `server`, `console`, ...) keeps its standard
  names even where a different word would be more descriptive: the entire value is that the command
  is the same in every repo that follows the pattern.
- **`helpers/`** is the helper drawer: `.sh` extension, called by other scripts and by `xtask`, not
  by people (`qemu-bounded.sh`, `qemu-runner-aarch64.sh`, `qemu-runner-riscv64.sh`). **It was
  `scripts/` until calef ratified `helpers/` on 2026-09-23**, and the reason is this file's own
  subject: the two directories differed by one trailing `s`, which cannot carry the difference
  between a command a person types and a file something else calls, so the distinction survived
  only in the head of whoever already knew it. The split was right and is unchanged; the name was
  the defect. See [scripts.md](../notes/scripts.md).

Every `script/` entry needs a row in [scripts.md](../notes/scripts.md); `script/lint` fails without one, and
fails in the other direction too if `README.md` names a script that does not exist.

## Where a name's provenance lives (milestone 115)

**The refusals are the valuable half, and they used to live nowhere.** A ratified name is visible in
the tree, because it *is* the name. A refused one is visible in no file at all, and the person who
most needs it is the person about to propose it again.

That is not hypothetical. A lane proposed `system_builder` for the crate milestone 96 extracted, the
maintainer endorsed it, and calef overruled it to `system_initializer`. Only afterwards did anyone
find that **milestone 63 had already refused `system_builder`**, for a reason still true:
`components/src/builder.rs` called itself "a minimal init: the system builder" then, so two
programs would claim one phrase. The refusal existed, in one table cell inside one milestone block, invisible at the
moment it was needed. A blind rename then swept the old name out of that very row, and the record of
the refusal was nearly destroyed by the rename it should have prevented.

That program was retired on 2026-09-14 (milestone 295) and the refusal stands unchanged, which is
the same point one turn later: a refusal records why a name lost on the day it lost, and one
rewritten every time the tree moves is one nobody can check.

**The record is derived, not maintained.** The first draft of the fix was one ratified-names table,
here in this file. calef rejected it on 2026-08-04 for scaling the way the original `DECISIONS.md`
and `design/roadmap.md` scaled, and size is the smaller half of that argument. The **conflict shape**
is the real one: every lane that adds a name would edit one file, which is exactly what produced
three section-number collisions in a day.

So:

1. **Provenance lives at the name.** A crate's `lib.rs` header, a program's module doc and a
   `script/` entry point's comment block each carry a `Name:` block saying when the name was
   ratified and what was refused. Adding a name touches exactly one file, so two lanes naming two
   things cannot collide. It also puts the refusal where the next proposer is already reading.
2. **`script/lint` checks presence, never content.** 126 names today, and a name with no block
   fails the build.
3. **`script/names` is the table**, computed on demand, so it cannot drift from the tree. Same
   family as `script/roadmap`, `script/decisions` and `script/catch-up`.
4. **The maintainer writes the block at ratification**, in the same commit that applies the name,
   while the alternatives are still in mind.

### The three states

calef works back through the existing names over time, so the record has to hold **which ones still
want him**, not only what is known. The state is the first word of the block:

| State | Means | What it costs to clear |
|---|---|---|
| **unrecorded** | nothing in the tree or its history says why this name was chosen | research, and then a ruling |
| **recorded** | the tree argues the name somewhere (a milestone block, a decision, a header) and calef never ruled | a ruling |
| **ratified** | calef ruled, with the date and what was refused | done |

The first cut of this mechanism had two states, and `unrecorded` was doing both of the first two
jobs. "Nobody in this tree can say why this is called that" and "here is the argument, nobody ever
signed it" are different amounts of calef's time, and a worklist that cannot tell them apart is a
list rather than a plan.

**The criterion, stated so that a reader can disagree with a case rather than with a mystery: a name
is `recorded` when something *outside its own block* argues for the name it has.** Three corollaries
did most of the work in the 2026-08-04 triage:

- **A `Name:` block is never its own evidence.** All 126 were written in one week by this milestone.
  Reading them as history would make the record prove itself and every name `recorded` by
  construction, which is why a `recorded` block must cite somewhere else and the citation is checked
  for being present.
- **"It got here first" is not a reason.** `design/naming.md` exempts `abi` from the `*_proto` rule
  because it "predates the suffix", which explains why the crate is not called `syscall_proto` and
  says nothing about why it is called `abi`. Counting an exemption as an explanation would let every
  old name in the tree explain itself.
- **An assertion is not an argument.** The BUGS section below says of `virtio` that "the crate keeps
  its name, which is right", with no reason attached, so a reader learns that somebody agreed rather
  than why.

**The gate never keys on `ratified`, and that is deliberate rather than a weakness to tighten
later.** 54 of the 126 names were unratified when milestone 115 (the names that were ratified, and
the ones that were refused) wrote this; the tree is at 76 of 229 on 2026-09-20, so the ratio held
while the tree nearly doubled. A lint that demanded the queue be drained would hold every unrelated
merge behind a review nobody can hurry, which is a wall this milestone was written specifically not
to build. `script/lint` insists only that a name say which state it is in. **There are four states
rather than these three**: §89 (`provisional` becomes the fourth provenance state) added
`provisional` on 2026-08-16, a claim about intent where the other three are claims about the
record, and it is the largest of the four today (48 of 229). The section *Those two captures print the worklist two
different ways* below is where it is argued; this table is milestone 115's and is left as it was
written.

### The three forms

```
Name: ratified <YYYY-MM-DD> (<who>, <where>). Refused `x` (why), `y` (why).
Name: recorded (<where>). <what the tree already argues, and what it does not settle>
Name: unrecorded. <what the history does and does not say>
```

**`recorded` carries a citation for the same reason `ratified` carries a date.** The claim is that
the reasoning lives somewhere else, so a block that will not say where has not made the claim. Both
are checked as form and neither as truth: nothing follows the citation to see whether it says what
the block says it says.

A block runs from its `Name:` line to the next empty comment line, so it may wrap over as many
lines as the reasons need. Two conventions make the refusals machine-readable without a syntax
anybody has to remember: **a reason goes in parentheses**, and **the refusal clause ends at its
sentence**. Both exist because the alternative misfires. `capsh(1)` is cited as the Linux tool that
made `capsh` unavailable, so a parser that read every backtick would record the citation as a
refusal of its own; `grant_plan` explains after its list that it is deliberately not named for
`swish`, and neither `swish` nor the `dwarden` it compares itself to is a refused name.

**`unrecorded` is a first-class answer.** Most of this tree's vocabulary arrived before anyone was
writing naming decisions down. Inventing a ratification to fill a row would put a false claim in the
one record whose entire job is saying who claimed what, so where the history does not say, the entry
says it does not say and cites the commit that introduced the name.

### The numbers, and the one that was not expected

Of 126 names: **72 ratified, 10 recorded, 44 unrecorded.** So 43% of this tree's most reader-facing
vocabulary arrived without a recorded decision, and only a fifth of that backlog is the cheap kind
where the argument exists and wants a signature.

Every name a rename ever touched is ratified, because a rename is an argument and somebody wrote it
down. What is left over is what nobody objected to at the time.

**The distribution is the finding, and it runs opposite to exposure:**

| Surface | ratified | recorded | unrecorded |
|---|---|---|---|
| programs (54) | 36 | **0** | 18 |
| crates (43) | 23 | 8 | 12 |
| `script/` (29) | 13 | 2 | 14 |

**Programs are 0 recorded of 18.** The surface a person types at the prompt, which the worklist puts
first precisely because a wrong name there is read by everyone who uses the system, is the one with
no argued reasoning anywhere in the tree: not in a header, not in a milestone block, not in an
introducing commit. `budgeter` and `heeder` are cited *as* an established agent-noun family when
milestone 63 argues for `benchmarker`, and neither was ever argued for itself. (`budgeter` was
argued and ruled on 2026-09-13, and is `memory_grant_depleter`; the word stays in this sentence
because the sentence is about what 63's text says, and 63 is BUILT and keeps it.) `sink` is used
throughout DECISIONS §51 and defended nowhere in it. The one program whose record says anything
useful is `disk_partitioner`, whose introducing commit calls the name provisional in as many words.

The 10 `recorded` are almost all one rule doing the work: seven `*_proto` crates, where milestone
46's spelling decision plus the service the stem names produces the whole string. The three
outliers are `intrusive` (its own header grounds the term in Linux's `list_head` and seL4's TCB
queues), `script/fmt` (the name was itself the fix, and the header cites §39 for it), and
`script/supply-chain` (the naming tenet cites this name and says not to respell it).

**Two `*_proto` crates were not recorded, and the split was the criterion working.** `gfx_proto` and
`cred_proto` had abbreviated stems, which is the first of the three failure modes the tenet lists
for crate names, and the rule that yields `<service>_proto` does not pick which word goes in front
of the underscore. `gfx_proto` was ratified 2026-08-23 (a kernel-dependency crate naming review) as
`graphics_proto`, spelling the abbreviation out in full. `cred` was the sharper case, and was renamed to `credentialer` on 2026-09-08: milestone 63
expanded `credcli` and argued `credentialer` in full, then left two crates spelled `cred` without
saying why. `user_mode_runtime` fails the same way twice over, since the only thing establishing `user_` as a
prefix is `user_mode_runtime` itself.

### EXAMPLES

Has this name been refused before? This is the query the incident above needed and nothing could
answer:

```
$ script/names system_builder
REFUSED for crate system_initializer  (crates/system_initializer/src/lib.rs)
  ratified 2026-08-04 (calef, milestone 96), and it is the ratification that raised milestone 115.
  Refused `system_builder` (milestone 63 had already refused it, for a reason still true:
  `builder.rs` calls itself "a minimal init: the system builder", so two programs would claim one
  phrase) and `system_bootloader` ...
```

Everything that has been turned down, and where the reason lives:

```
$ script/names --refused
REFUSED (85), and what holds each refusal

  allocdemo                    program allocator_exerciser
  allot                        crate grant_plan
  ...
  job_killer                   program job_undertaker
  sanitize                     script undefined-behavior-check
  sheesh                       crate swish
```

**What is left, in the order worth working through.** This is the deliverable, and the ordering is
exposure rather than alphabet or count, because exposure is what makes a wrong name expensive: a
program is typed at the prompt, a crate is what a newcomer greps before opening anything, a
`script/` entry point is typed by whoever works on the tree rather than in it. Within a tier, a name
nobody can justify comes before one whose reasoning merely lacks a signature.

```
$ script/names --unratified
UNRATIFIED (54 of 126), in the order worth working through
...
  programs, provisional
    address_space_builder        fixtures/src/address_space_builder.rs
    ...
  crates, unrecorded
    abi                          crates/abi/src/lib.rs
    ...
  crates, recorded
    clock_protocol               crates/clock_protocol/src/lib.rs
    ...
  scripts, recorded
    fmt                          script/fmt
    supply-chain                 script/supply-chain

44 unrecorded (research, then a ruling), 10 recorded (a ruling only).
```

That capture keeps `address_space_builder`, which is `address_space_witness` since calef's ruling of
2026-09-18. It is a transcript with measured counts in it, so it is evidence and stays; and sweeping
it would have been doubly wrong, because the new name is ratified and so appears on no
`--unratified` listing that command will ever print.

**The tier is the kind, and not "programs a person actually types".** That second split is the
two-tier rule calef rejected on 2026-08-01, keyed on a property that is not stable: `wc` went from
internal plumbing to a prompt-typed pipeline stage inside a day. Every program in `components/src/`
and `fixtures/src/` is in the initrd and can be typed, so the kind is the honest tier and needs no
classification anybody
could get wrong. This is a sort order rather than a naming convention, so the cost of being wrong
about one entry is that it is read in the wrong minute.

Then one name at a time, with what the history does and does not say about it:

```
$ script/names bitmap_font
crate bitmap_font  (crates/bitmap_font/src/lib.rs)
  ratified 2026-08-23 (calef, a kernel-dependency crate naming review). Renamed from `bitfont`:
  spell out the contraction fully, consistent with this session's other renames, even though
  `bitfont` was already transparent.
```

The narrower slice, for the names where the research is still owed:

```
$ script/names --unrecorded
UNRECORDED (44 of 126): nothing outside the block says why
```

The whole table, and the gate:

```
$ script/names | tail -3
total: 126 names, 72 ratified, 10 recorded, 44 unrecorded, 85 refusals
54 still want calef: script/names --unratified
refused but live: video_terminal

$ script/names --check
names: NOTE 'video_terminal' is recorded as refused and is also a live name
names: 126 names carry provenance (43 crates, 54 programs, 29 scripts)
names: 72 ratified, 10 recorded, 44 unrecorded, 85 refusals recorded beside them
names: 54 still want calef (script/names --unratified), which is a worklist and not a failure
```

That `video_terminal` line is the mechanism working rather than a defect. The name was **refused for
the program** (`display_terminal` is named for its role) and is **live as the crate** (named for the
protocol it implements). Both facts are true, the pair is deliberate, and a reader who meets only
one of them would get it wrong. The check reports the contradiction and never fails on it, because a
refused word can legitimately survive as ordinary English and a gate that fires on prose is a gate
people learn to skip.

**Those two captures print the worklist two different ways, and that was a defect, fixed
2026-09-19.** The `--check` line counted `recorded + unrecorded`, the table's last line and
`--unratified` counted `provisional` as well, and neither census line listed `provisional` at all,
so the three numbers shown did not add up to the total. The captures above keep their numbers
because they are evidence of what the tool printed; at 126 names there happened to be no
provisional ones, which is why the two lines agree there and why nobody saw it until milestone
117's fourth stranger added a program with a provisional name and watched it vanish from the
gate's count. **Every line now prints the one worklist, which is exactly what `--unratified`
lists**, `provisional` included, because that command sorts provisional names first in every tier
(their author has already said they are wrong, §89) and a count that dropped them hid the part of
the worklist worth reading first. The census names all four states and all four kinds, so it sums:

```
$ script/names --check 2>/dev/null
names: 222 names carry provenance (68 crates, 89 programs, 10 packages, 55 scripts)
names: 153 ratified, 41 provisional, 27 recorded, 1 unrecorded, 273 refusals recorded beside them
names: 69 still want calef (script/names --unratified), which is a worklist and not a failure

$ script/names --unratified | head -1
UNRATIFIED (69 of 222), in the order worth working through
```

### BUGS

- **It checks that a name carries a reason, never that the reason is still true.** A block whose
  argument was overtaken looks exactly like one whose argument holds. That is the same limit
  `script/decisions --check` records for `§N` citations, and it is not closeable by a script,
  because a reason is prose and prose is checked by reading. Milestone 97 is the neighbouring case.
- **It cannot tell an honest `unrecorded` from a lazy one.** The 44 above were each researched
  against the git history, and nothing stops the forty-fifth from being a shrug. The only defence is
  that an `unrecorded` entry cites the commit that introduced the name, so the next reader starts
  where this one stopped rather than from nothing.
- **`recorded` is the state easiest to claim and hardest to check**, which is the price of splitting
  it out. The citation is checked for being present and never followed, so `recorded (milestone 46)`
  on a name milestone 46 never mentions passes the gate exactly as well as a true one. Read the
  citation before trusting the state, and note that a wrong one costs less than the alternatives: it
  demotes the entry from "research owed" to "signature owed" in a worklist, rather than putting a
  false claim in the tree.
- **Seven of the ten `recorded` lean on a rule that was derived from the names it now explains.**
  `*_proto` won in milestone 46 partly *because* "it is what the actual crates already were", so
  saying `fs_proto` is recorded by that rule is not fully independent of `fs_proto`. It is not
  circular either, since the decision adjudicated four live spellings and `script/lint` has enforced
  the winner since, but a reader deciding how much weight the state carries should know it leans on
  one decision and that the decision partly ratified the status quo.
- **The tree has more kinds of name than the table covers, and this bullet is the one place that
  says which.** Everything else that states the worklist's scope (`script/names`' own comment,
  `helpers/name_provenance.py`, `helpers/roadmap_proposals.py`) cites this bullet rather than
  restating it, on purpose: the last two copies of this claim went stale for a month after the
  coverage grew and nothing compared them against the tool. Crates, programs, `script/`
  entry points and Cargo packages carry blocks. Directories, types and `helpers/` helpers
  do not, and at least one ratified name had no home as a result:
  **`design/audit-reports/`** (calef, 2026-08-04), where `audit-trail` was refused because
  `design/decisions/35-scanner-findings.md` already uses that phrase for a chronological record of
  dismissals and it is also what an operating system means by it (`auditd`), and bare `audits` was
  passed over because every file in the directory is a report. Recorded here rather than stretched
  into a schema that does not fit it. **Closed for directories on 2026-08-16 by §75**: a directory
  under `design/` or `notes/` now carries its provenance in its own `README.md`, applied that day
  to `design/decisions/`, `design/roadmap/` and `notes/`, with `design/audit-reports/`'s line owed
  by the milestone-92 commit that creates it. **Closed for Cargo packages on 2026-08-18**, when calef found
  `script/names std_exerciser` answering "neither a name in the tree nor a recorded refusal" and the
  `package` kind was added: `kernel`, `xtask`, `redoxfs_server` and `tools/redoxfs_host` now carry
  blocks in their manifests, and milestone 276's weekly series shows the hole closing in 2026W34.
  Types are still uncovered. **`helpers/` is uncovered on purpose**, priced and refused on
  2026-09-20 by milestone 446 (the naming worklist says what it covers, and stops saying what it
  used to); its own bullet is below, because it is a decision rather than a gap.

  **That blind spot has a live casualty, found while triaging.** `disk_partitioner`'s introducing
  commit (2026-08-03) named two provisional things: itself, and `fs_maker`. The first is on a
  covered surface, so it is in the worklist with the word "provisional" quoted in its block. The
  second is at `redoxfs_server/src/bin/mkfs.rs`, where nothing looks, so it was resolved to `mkfs` by
  whoever was mid-task and no record anywhere says a decision was owed. That is the exact failure
  this milestone exists to prevent, still happening one directory over.
- **`helpers/` helpers are deliberately outside the worklist, and the numbers are why.** There are
  17 files in `helpers/`, of which **9 already carry a `Name:` paragraph** that nobody asked them
  for, written by the lane that added the file. So the question is not whether a helper may argue
  its own name (it may, and more than half do) but whether the worklist should **enumerate** them,
  and enumerating costs **about 15 rows on a worklist that is 76 deep today**, a fifth again of the
  only queue in this tree whose sole consumer is calef's attention. Three things decided it against.
  **The worklist is ordered by exposure**, and its own header says so: a program is typed at the
  prompt, a crate is what a newcomer greps, a `script/` entry point is typed by whoever works on the
  tree. The **Scripts** section above defines `helpers/` as the drawer that is *not* typed by
  people, so enumerating it would add a tier below the bottom tier of a list whose whole ordering is
  exposure. **Nothing is being lost today**: not one of those 9 paragraphs records a refusal, so
  milestone 115's "the refusals are the valuable half" claim gives up nothing by leaving them out,
  and that is the number to re-measure if this is ever revisited. And **the 8 without a paragraph
  are the machine-invoked ones** (`qemu-runner-*.sh`, `qemu-bounded.sh`, `memory-bounded-runner.sh`,
  `build-ripgrep.sh`, `rust_source.py`), so a gate demanding blocks would mostly manufacture
  rulings on names nobody types. **If it is ever built, order it last**, after `script/`, for the
  same exposure reason. What the refusal did buy: `script/names <helper>` no longer answers
  "neither a name in the tree nor a recorded refusal" about a file that argues its name at length,
  and instead says the name is out of scope and points at the file.
- **A type's name is a naming decision the mechanism does not see.** `BootEndowment` was ratified on
  2026-08-04 (replacing `Grants`) and is mentioned inside `system_initializer`'s block only because
  its crate happens to export it. `supervision_protocol::Endow` is an open naming question (§69) and
  appears nowhere in this record.
- **The `Name:` marker is a string in a comment**, so a header that never had one is caught by the
  gate while a header that loses one to an edit is caught only if the edit removes the whole line.
- **A correctly formatted `ratified` is never checked against calef, and on 2026-08-14 two lanes
  contradicted each other about the same name on the same day.** `crates/cpu_set` was introduced
  twice in one stack of pull requests. #176 carried `Name: ratified (calef, 2026-08-14, the same day
  the first-silicon online-set sweep introduced it)`. #178, one branch later in the same stack,
  carried `Name: unrecorded, provisional (introduced 2026-08-14 by the first-silicon online-set
  sweep; calef has not seen it)`. The second is the true one, and the two sat in the queue together
  asserting opposite things about whether a ruling had happened. (Both headers said `Chris` when
  they were written; they are quoted here in the referent this tree adopted on 2026-08-15, which is
  also how the surviving one now reads in `crates/cpu_set`.)

  **The gate caught the false one, and caught it for the wrong reason.** `script/names --check`
  rejected #176 because the date sits inside the parenthetical where `ratified (\d{4}-\d{2}-\d{2})`
  cannot reach it. Written as `Name: ratified 2026-08-14 (calef, …)`, the identical false claim would
  have passed every check in CI and landed on `main`. What stopped it was punctuation.

  **This follows from the design and is not a defect in it.** The gate checks that a block names one
  of the three states and never that the state is `ratified`, because a gate keyed on ratification
  holds every unrelated merge behind a queue only one person can drain, which is the wall milestone
  115 was written not to build. The cost of that choice is what this entry records: **claiming
  calef's ruling is as cheap as claiming anything else, and nothing downstream disagrees.** The
  `recorded` entry above says the citation is never followed; this is the same hole one state up,
  where the claim is not a citation anybody could follow but an assertion about a person.

  A lane that does not know this will write `ratified` meaning "this name seems settled". Write
  `unrecorded, provisional` instead and say so in the report. It costs nothing, `script/names
  --unratified` is a worklist rather than a wall, and an unratified name has never failed a build.

  **What would help without building the wall**, if this recurs: surface newly added `ratified`
  blocks in a diff for the integrator rather than blocking on them. `ratified` is the one state a
  lane structurally cannot be entitled to assert, so a new one appearing in a pull request is worth a
  human glance even though it must not be worth a red check. That is rung two of AGENTS.md's ladder
  applied to the one state that currently sits on rung zero. Not built, and not obviously worth
  building for a hazard observed once.

## Directories (milestone 63)

The tenet covered crates, programs, modules, shell entry points and markdown, and said nothing about
**directories**, so the tree carried three spellings. Two rules, and neither is a new tier:

- **A directory that holds a Rust package is named exactly as the package**, so `snake_case`. The
  directory and the package are one thing with one name.
- **Any other directory is lowercase, and hyphenated if it needs two words**, the same convention as
  markdown filenames and `script/` entry points, because a directory is a path element and paths are
  hyphenated in the world outside this repository.

Three directories violated the first rule and all three moved in milestone 63: `fs-server/` (package
`fs-server`) is `fs_server/`, `tools/redoxfs-host/` is `tools/redoxfs_host/`, and `user-std/`, whose
package was called `hellostd` and matched neither, is `std_exerciser/` twice over.

A hyphenated package name is not wrong in the wider ecosystem. `wasm-bindgen` and
`tracing-subscriber` are ordinary and Cargo normalises a hyphen to an underscore for `use`, so
nothing was broken. The case was internal consistency, 36 crates against 3, and it should be read
that way rather than as a correctness fix.

**Two things that look alike and are not:** `target/` is gitignored build output and `targets/` is
the tracked custom target JSON (`aarch64-unknown-nife.json`). Nothing enforces the distinction.

## Where a document goes

Three places, and the distinction is what the document is *for*, not what it is about.

| | holds | shape |
|---|---|---|
| `design/` | the option space, before a decision | "here are four answers and three are bad" |
| `design/decisions/` | the decision, and the argument that settled it | numbered `§N`, append-only |
| `notes/` | what exists, and what building it taught us | a running glossary, indexed in `notes/README.md` |

`design/roadmap/` is the exception that proves the split: it lives in `design/` because a milestone
block is an argument for doing something, not a record of having done it, even after the milestone
ships and the block gains a "Built" line.

A note is not optional. Every concept and every finding gets one, indexed in
[notes/README.md](../notes/README.md), because for a demonstration OS the documentation is part of the
deliverable rather than a courtesy to the author.

## `§N` is not milestone N, and they collide

**DECISIONS section numbers and roadmap milestone numbers are separate schemes over the same small
integers.** There are 41 sections and 39 milestone blocks, so almost every number means two things:

| N | DECISIONS `§N` | roadmap milestone N |
|---|---|---|
| 24 | the two-tier Ctrl-C | a Virtualization.framework board |
| 28 | SMP placement | the line discipline |
| 31 | the foreign-language C seam | the capability shell |
| 39 | this naming rule | components, services, and the directory layout |

This has already produced a wrong citation in the tree, not a hypothetical one: milestone 50's block
cited "§31's FileSpec", which points at the C seam and has nothing to do with file grants. The thing
it meant was milestone 31 phase 2, granting against the §27 filesystem contract. Fixed in c0643bc.

So:

- **Write `§N` only for DECISIONS.** Never for a milestone.
- **Write "milestone N" in full.** Never bare `N`, and never `§N`.
- Prefer number **and** name on first mention in a block ("milestone 31, the capability shell"),
  which is what makes the wrong one visible.

`script/decisions --check` verifies that every cited `§N` resolves to *some* section. It cannot
verify that it resolves to the *right* one: a well-formed wrong citation is indistinguishable from a
correct one from the outside. Worth knowing before trusting that gate for more than it claims.

### Both schemes are sparse: a duplicate is the defect, a gap is not (milestone 443)

**Two files claiming one number is fatal in both schemes**, because every citation to that number is
then ambiguous and no gate can tell which one a sentence meant. `script/decisions --check` and
`script/roadmap --check` each fail on it twice over, once for two index rows and once for two files.

**A hole in the numbers is reported and passes.** `§157` with nothing behind it misleads nobody: the
number appears in no citation, every citation that does exist still resolves, and the sequence was
never a promise that it is dense. The roadmap has always worked this way, and 441 and 442 are unused
today. `script/decisions` failed on a gap until 2026-09-19 and no longer does.

**The reason is that the gate was buying a cosmetic property with the tree's most dangerous edit.**
A gap is closed by renumbering, and [§194](decisions/194-sessions-interleave-rather-than-serialize.md)
records what renumbering costs: a citation rewritten by number can be silently wrong and still pass
every gate, because the section it now names exists. On 2026-09-19 one branch was renumbered four
times in two and a half hours, from §156-§189 to §160-§194, because another session was minting from
the same range and a contiguous scheme made every collision displace the whole run.

**So when two sessions collide on a number, the later lander takes the next free ones and leaves the
hole.** Only the colliding sections move, not the run behind them, which is four files rather than
thirty-four. The rule above it is unchanged and is `AGENTS.md`'s: a number is provisional until the
merge queue lands it, and anything global to the tree is the integrator's at merge.

**What no longer has a gate, stated rather than discovered.** A decision file deleted outright, with
its index row deleted in the same commit and no citation to it anywhere in the tree, now leaves
nothing behind to notice. Every partial shape still fails: a file with no row, a row with no file, a
row pointing at a missing file, and a `§N` in the tree that resolves to nothing.

## Branches

Eight prefixes were in use when this was written, including both `feature/` and `feat/` for the same
idea. One spelling, and `feature/` is the older one:

`milestone/` (a roadmap milestone), `fix/` (a bug with a name), `bench/` (measurement work),
`audit/` (reading rather than writing), `integration/` (joining lanes), `finalize/` (landing them).
Plus `main`, and the tooling's own `worktree-agent-*`, which no person types.

**This is a convention and not a gate, since 2026-08-18.** It was an enforced allowlist, and calef
asked what the taxonomy was for. The answer, checked rather than argued: **nothing consumes it except
the check itself.** A grep across `script/`, `helpers/`, `.github/workflows/` and `xtask/` for any
other reader of a branch prefix returns only false positives. Only `milestone/N-` is read by
anything, and `script/lint`'s milestone-branch-touches-its-block check is what reads it.

**Every observed failure of the allowlist was a false rejection of legitimate work**, four times:
`roadmap/` (the repository's *second* commonest prefix, refused while about thirty-five merges using
it were already on `main`), `gh-readonly-queue/*` (which failed every group build's clippy job, so the
queue evicted and rebuilt forever), `dependabot/*`, and `claude/*` (a harness names its own branches
and a lane cannot rename them from inside). Each was fixed by widening the list after something broke.
That is the signature §61 used to drop three lints and milestone 78 used on three assertions: **a
check that only ever rejects valid work is measuring the wrong thing.**

What survives as a gate is the one prefix that carries a mechanism, and it survives as a branch name
rather than a label for a specific reason: the check that reads it runs **locally and offline**, from
`symbolic-ref`, with no network and no GitHub context. A label cannot be read without an
authenticated round trip, and would not exist yet anyway, because a lane runs `script/lint` before it
runs `gh pr create`. A branch name is also **fixed when the lane is cut**, where a label is editable
at any time, and for a check whose job is "you claimed 126, so move 126's block" the claim has to be
the immutable half.

## What is checked, and what cannot be

`script/lint`'s `naming conventions` block enforces seven things, and the first five are cheap greps
because lint runs constantly. (An earlier version of this sentence said four and then listed five,
which is the ordinary way a hand-kept count drifts; take it from the script.)

1. **No name ending in `-d`**, over `components/src/*.rs` and `fixtures/src/*.rs`, both
   `Cargo.toml`s' `[[bin]]` names, and `crates/*`. Four characters or more, so a three-letter name ending in `d` is read as an
   abbreviation rather than a daemon (`kbd` was this rule's worked example until its 2026-08-28
   rename). Words that
   genuinely end in `d` go in `naming_allow` **with a reason**, the same shape as a per-item
   `#[allow]`; `uuid` (RFC 9562's own term) is the one there today. `asid` was the other until
   2026-09-18, and §154's rename removed the entry rather than re-argued it: an exemption that
   exists only because a name is an unreadable acronym is one the expansion deletes.
2. **The word "daemon" appears nowhere**, outside `design/decisions/` and `design/`, which are where the
   argument about the word lives and therefore have to be able to name it.
3. **Contract crates spelled `*_proto`.**
4. **The current branch carries a recognised prefix.**
5. **No `#[path]` module is shared by two or more binaries** (CLAUDE.md rule 7). This is the newest
   and the one with teeth: it counts consumers per include target and fires at two. A module with a
   single consumer is an ordinary submodule and is fine, because the rule is about *agreement between
   binaries*, not file layout. The allow-list is **empty**, which is the intended steady state.
   `virtio` was its one entry for about an hour: it could not be a crate while it reached back into
   whichever binary included it for `check`, and the resolution was to **delete `check`** rather than
   pass it in. Rust already has the per-binary "how this program dies" hook, `#[panic_handler]`, and
   both binaries already had one executing the same instruction by two different routes. An entry
   here needs a reason of that calibre.
6. **`notes/` and `design/` filenames are lowercase and hyphenated**, because they are URL slugs the
   moment any of this is published. `README.md` is the only exception, and it is GitHub behaviour
   rather than style.
7. **Every crate, program and `script/` entry point carries a `Name:` block** (milestone 115, and
   the section above). Presence only: it cannot check that the reason is still true. It checks that
   the block names one of the three states, that `ratified` carries a date and that `recorded`
   carries a citation, and it **never checks that the state is `ratified`**, so a name waiting on
   calef does not fail anybody's build. `script/names --unratified` is how that queue gets worked.

   **And exactly one such block per file, in the spelling the parse reads** (milestone 283). That
   was a convention 205 files happened to follow until two did not, and the two ways of breaking it
   compound into silence: the parse stops at the **first** `Name:` line, so a stale proposal block
   above a ratified one is what gets read, and the parse matches `^<prefix> ?Name:`, so a header
   wearing markdown (`//! **Name: ratified ...**`) is invisible even when it is the only one. Both
   at once is how two of calef's ratifications sat on the worklist for a week while the gate
   reported `provisional`, which is a legitimate answer nothing disputes.

   So the check asks the question a person asks by looking at the file: does anything here read as a
   provenance header without being the one that was read. A comment line whose content, after the
   marker and any leading markdown, begins `Name:` counts, at `///` and `//` as well as the
   surface's own prefix, and the failure names the file, the line, and which of four things is wrong
   with it (markup, indentation, a different comment marker, or a second block nothing reads past).
   **Two things are deliberately not headers**: `` `Name:` `` in backticks is a *mention* of the
   convention, which the scripts implementing it write constantly, and a line carrying an
   angle-bracket placeholder (`Name: ratified <YYYY-MM-DD>`) is showing the *form*, which is how
   `script/names`' own header documents the three spellings.

   **The parse was not widened to admit bold**, deliberately: 205 files use the plain form, so
   admitting a second spelling would make both legal, which is the opposite of the fix.

Everything else here is prose because it needs judgement and no checker can supply it. In particular
**a checker cannot catch the jargon half of §39**: `linedisc` would have passed all four rules above.
It ends in `c`, contains no daemon, is not a proto crate, and had a perfectly good branch. What
caught it was a person reading the name and not knowing what it meant, and that remains the test.

Two limits worth stating rather than discovering: the checks read the filesystem for names and use
`git grep` for the word, so an **untracked** file with "daemon" in it is invisible until it is added
(the same blind spot the conflict-marker check has), and check 1 sees the *names* of things rather
than the things, so a component whose name is fine and whose behaviour is a daemon is not its
problem.

## A half implies two; a third of anything is an arm

**calef, 2026-09-19**, reading a lane's workflow that called macOS, Linux and Windows each a "half"
of one program. A count above two in front of the word is not a strong claim or a loose one; it is
arithmetic that cannot be true, and a reader who meets it stops trusting the sentence around it.
(This section writes that shape as "three <halves>" wherever it must show it, because `script/lint`
gates on the literal and a rule whose own text trips its gate is a rule nobody can land.)

**The rule, and it costs nothing to follow.** "Half" is for a genuine two-way split and is often
exactly right: this tree has honest halves everywhere (a crate's pure half and its host-tools half,
milestone 74's aarch64 half and riscv64 half). For one branch of a split with three or more, this
tree's own word is **arm**: `components/src/console.rs` speaks of "its x86 arm", and the boot
ladder, the console server and the shell-check legs all read that way. "Part", "piece" and "leg" are
the other honest choices; a "leg" in this tree already means one architecture's run of a gate.

**What is gated and what is not.** `script/lint` reads only the shape that cannot be argued with: a
count word immediately in front of the word ("three h...", "four h...", and so on). It does not
judge a "half" whose siblings are a paragraph away, because that needs a reader, and a gate that
guesses at prose is how this tree lost three checks. The sweep that came with the rule fixed four
(milestone 22's deliverable, milestone 54's landings, a `filesystem_protocol` doc comment and a
`timetable_tests` one) and left the honest halves alone.

## An abbreviation we receive rather than author

calef, 2026-09-13, asking what it would take to rename `initrd` to `initial_ramdisk`. The answer is
that it cannot complete, and the reason generalises past this one word.

**The acronym rule points at it, correctly.** *An acronym is spelled out unless its expansion teaches
nothing* (2026-09-05). *Initial ramdisk* teaches a great deal: it says the thing is RAM-resident and
readable before storage exists, which is the entire point and is not recoverable from the five
letters. By that test `initrd` should go, the same way `dma` went.

**It cannot, because about ninety of its 1,302 occurrences are somebody else's spelling:**

| what | count | whose |
|---|---|---|
| `"linux,initrd-start"` | 19 | the Devicetree spec's property name, in a blob QEMU generates |
| `"linux,initrd-end"` | 8 | the same |
| `-initrd` | 64 | QEMU's command-line flag |

The kernel finds the region by parsing that property; every run passes that flag. Rename our 840
identifiers and the tree says `initial_ramdisk` in the code and `initrd` at the two points where a
reader most needs the words connected: where we read the property, and where we launch the machine.
**A rename that cannot reach the boundary makes a newcomer learn two words instead of one**, which is
the opposite of what the acronym rule is for.

**This is the `Guid` case one level out.** `crates/gpt` keeps `Guid` rather than following
`user/src/uuid.rs`'s ratification because the name is load-bearing at an interface boundary: a GUID
is mixed-endian on disk where RFC 9562's UUID is big-endian, so the two words name different things
and collapsing them would assert a byte order the code does not produce. `initrd` is the same shape
with the authority reversed: not a distinction we are preserving, but a name we do not own.

**So the rule this adds, stated so it can be disagreed with: the acronym test applies to names this
tree authors.** Where a name arrives across an interface somebody else defines, the tree keeps their
spelling and pays the cost at the reader's expense once, in an expansion written where the reader
meets it. `crates/user_rt/src/initrd.rs` carries that expansion as of 2026-09-13.

**And the defect the pricing found was not the name.** `initrd` appeared about 1,300 times and was
expanded in full **exactly once**, in `crates/dtb` (`crates/device_tree_blob` since 2026-09-19), a crate about device trees rather than the one
named for the thing. The abbreviation was never the problem; an unexplained abbreviation was, and
that is rung three rather than a sweep.

## A terminus that is structural, or one that is merely current

calef, 2026-09-13, asking after ruling `audit_sink` -> `login_audit_receiver`: *"Are there other
sinks that should be named receivers?"* The sweep found three and renamed none of them, which is
what makes the distinction worth writing down rather than leaving in one block.

**The test, in one question: does the name claim an end-of-stream that is a property of the design,
or one that is an accident of what has not been built yet?**

`components/src/audit_sink.rs` receives one message per successful login on `login`'s `AUDIT` endpoint and
discards it. "Sink" was accurate about today and wrong about the program: the discard exists because
printing the record would need a `WRITE` view of the terminal, and handing that to a third process
was refused *for now*. The moment somebody grants it, the program keeps records and its name says it
does not. A name that has to change when a capability is granted is naming the gap rather than the
thing.

The three that survived the same question, and each for its own reason:

| Name | Why the terminus is structural |
|---|---|
| `byte_sink_protocol` | A wire contract named for what it carries. It makes no disposal claim at all |
| `terminal_sink_caretaker` | It holds the terminal endpoint, which also carries `OP_READLINE`, and hands out a sink that **cannot read**. `sink` names what it hands out, `caretaker` names what it is. calef already caught this class once here, ratifying the longer form over `terminal_sink` on 2026-08-03 |
| `sink` (the program) | Not a terminus at all. Three roles, and `ROLE_FILE` is a real file behind a sink: the process can open, read, write at offsets, truncate and stat, while its client can only say *here are sixteen bytes, append them*. Renaming it `receiver` would name one end of a three-role program |

**The third row's program no longer exists in that form**, and the ruling is unaffected: milestone
292 split it into `sink_transcript_writer`, `file_sink` and `file_source` on 2026-09-14. The row's
*reason* was that one name covered three jobs, and that reason retired itself. What survived the
split is the answer: `file_sink`'s terminus is structural, because its client holds a capability over
which no message but *append* is expressible, and no grant anybody could make would change that.
That is the strongest form of this test passing, and it is why `sink` stays the contract's word
rather than becoming `receiver`. The row stands as the account of what was ruled on 2026-09-13.

**The second half of the ruling is the part that is easy to lose.** `audit_sink` failed on two
counts and only one of them is about "sink". The `audit` half promised a record that does not exist,
which is `flaky`'s fault (borrowed recognition the program contradicts) applied to a payload rather
than to a behaviour. A reader meeting `audit_sink` in a process listing concludes the system records
logins. Nothing does.

So `receiver` won because it is true in both states: it receives today and it will receive when it
records, and **`login_audit_recorder` is then an honest successor rather than a correction**. That
successor is written into the program's own block as a condition rather than left to whoever
notices, which is §71's shape borrowed for a name: say what would change the answer, beside the
thing it would change.

**What this does not license.** It is not an argument against `sink`, which is this tree's word for
the end of a stream nobody reads further and is right three times out of four. It is an argument
against naming a program after a state that a single capability grant would end.

## An identity is what you present; a principal is what you become

**`principal` is ratified** (calef, 2026-09-14) as this tree's term of art for an authenticated
actor holding a capability set. It was the last word in the login vocabulary with no ruling, and it
is ratified as a **term**, not as a filename: `script/names` walks crates, programs and modules, so
nothing gates this and the record is the gate.

**Why it needed settling at all.** `components/src/login.rs` could not be named until the words it
operates on were. Asked what that program authenticates, the honest answer turned out to be
*nothing*: it holds `WRITE` on the credential service's verify endpoint and **relays**, and
`components/src/credentialer.rs` is what checks the secret. What `login` does is mint a session's
worth of capabilities on the answer. So the sentence the program needs a name for is *turns an
identity into a principal*, and two of those three words were unsettled.

**The four words, and why only one was open.** Measured on 2026-09-14, tree-wide:

| word | code | prose | already names |
|---|---|---|---|
| `identity` | 546 | 362 | `identity_provisioner`, `MAX_IDENTITY`, `identity_hint` |
| `session` | 256 | 472 | `session_reviver` |
| `credential` | 163 | 129 | `credential_proto`, `credentialer`, `credentialer_test_client` |
| `principal` | 36 | 46 | **nothing** |

`identity` is fixed by an interface rather than by taste: DECISIONS §117 names a principal's subtree
by the identity string **used directly**, with no lookup table, and `MAX_IDENTITY` caps it on the
wire. `credential` was ratified with `credentialer` on 2026-08-01. `user` means **a person**, which
calef settled when the `user_` prefix became `user_mode_`, and is otherwise spoken for: 1079 occurrences in code, essentially all of them the
kernel's `user::` module or the `user_mode_` prefix. That left `principal`, which the tree leans on
for the thing that matters most and had never given a name to.

**The distinction the ratification keeps.** An **identity** is the string a client presents
(`chris`, `corinne`). A **principal** is the authenticated actor that results, holding a fresh
capability set. Collapsing them into one word was considered and refused: it is cheaper to read and
it loses exactly the difference `login` exists to perform, which is the difference between what you
present and what you become. §109's attribution model is written in the second word, not the first
("nameable only by the principal that established it"), and two successful logins are two different
endpoint *objects* rather than two views of one.

**One ambiguity recorded rather than fixed.** `session` carries two senses in this repository: a
login session, and an **agent** session in AGENTS.md and the process notes. The prose count above is
mostly the second. Nothing in code confuses them, and no rename is proposed here; a reader of the
process docs should know the word is doing two jobs.

## The `login` stem stays

**Ratified 2026-09-15 by calef, for the whole family**: `components/src/login.rs`,
`crates/login_protocol`, `fixtures/src/login_test_client.rs`, and the kernel's `login_service` and
`login_tests`. About 700 occurrences across 94 files keep the word.

**The case against was real, which is why this was parked on 2026-09-14 rather than signed.** Two
facts undercut the reason first recorded for the name. **Nothing types `login`**: the kernel starts
it, and only other programs reach it, by `CONNECT` on its front door, so an argument resting on a
person meeting the Unix name did not hold. **And it does not authenticate**: it relays to
`credentialer`, which checks the secret, and what it does itself is turn an identity into a
principal (the section above). By milestone 63's own test, which refused to name the credential
service for its resource because it "never hands you a credential", a login service never hands
you a login.

**Why the stem stays anyway.** `login` is the field's name for this role whoever speaks it, and a
reader arriving from Unix lands in the right place; the program's docs say plainly where it departs
(capabilities instead of a mutated user ID). And the stem is carried by `login_protocol`, a wire
vocabulary two programs agree on, which is the expensive kind of name to move.

**Considered and refused**, as a program name: `authenticator` names the half this program does not
do; `principal_minter` and `session_granter` are accurate and are new words for what everyone already
calls logging in; `powerbox` is the right term of art for the pattern and one almost no reader would
recognise.

**The cost that ruling removed.** Milestone 265 renamed `login_proto` to `login_protocol` with the stem
open and accepted a second rename when it was ruled. There is no second rename.

## Performing a ratified rename

`AGENTS.md` carries the three rules. This is the argument, the worked example and what is not
gateable.

**The asymmetry that makes this worth writing down.** A rename is trivial mechanically and expensive
in every other way, which the *move fast on what can be undone* tenet already says. What it does not
say, and what this adds, is that the expensive half is not only the name in a reader's head. It is
the **records**, and a sweep edits those at the same cost as it edits code while destroying
something a revert cannot restore.

### What a ratified name drags with it, and what it does not

A name is ratified for a crate, a program or a module. Some other things in the tree carry that word
and the question is which of them move with it. calef ruled both halves on 2026-09-18, during the
names review that performed six renames.

| Carries the name | Moves? | Why |
|---|---|---|
| The crate directory, package name, dependency entries | **Yes** | They *are* the name |
| A **note named for the crate** (`notes/asids.md`, `notes/gpt.md`) | **Yes** | A note is an interface: a reader meets it by name, and `script/apropos` and every citation address it that way |
| A **note named for the concept or for another thing** (`notes/ipc-naming.md`, about inter-process communication; `notes/ipc-tables-lock-inventory.md`, about the `IPC_TABLES` lock §118 named) | **No** | The ownership test below: it keeps its name when our crate is deleted. The earlier wording of the row above said only "a note filename", and read that way it would have renamed both of these |
| A **roadmap slug** (`design/roadmap/15-asids.md`) | **No** | Exempt, standing rule: roadmap titles and slugs are drafts, and the number is what people cite |
| A **hardware field or wire name** (`satp.ASID`, `NVMe 1.4 §3.1`) | **Never** | A citation of somebody else's specification |
| A **public type named for the acronym** (`Gpt`, `Dtb`) | **Yes** | calef, 2026-09-19: a reader meets the type far more often than the crate, so leaving it short leaves most of the acronym in place. `Nvme` had already moved with its family. **`Guid` stays** under its own 2026-09-13 ruling, which is about byte order rather than length |
| A **fuzz target named for the crate** (`gpt_table`, `dtb_walk`) | **Yes** | calef, 2026-09-19: named for what it fuzzes |
| A **`BUILT` block, a transcript, a dated account** | **Never** | The status table above |

**The note half has a cost the crate half does not: every citation of the old path breaks.**
`notes/gpt.md` was cited by 18 files when it moved. `script/lint` check 4c verifies that
a markdown *link* target resolves, so it catches those; it does **not** catch a path written in prose
outside a link, and both forms exist in this tree. Grep for both.

**And the relative depth is where it actually goes wrong.** A citation from `design/roadmap/*.md`
reads `../../notes/...`; one from inside `notes/` is a bare sibling with no directory at all. Moving
`notes/naming.md` to `design/naming.md` on 2026-09-18 rewrote 65 citations correctly and still left
one sibling link broken, because the grep that found the others could not see a path with no
directory in it.

**Filenames under `notes/` and `design/` are hyphenated**, per the domain table above, so a
`snake_case` crate becomes a hyphenated note: `address_space_identifier` and
`address-space-identifiers.md`.

### The refusal count is the gate, and it is one command

**Take `script/names | tail -3` before the rename and again after. The refusal count must not
move.** A rename neither adds nor removes refusals, so any change is a mistake: a refusal swept into
a name that no longer exists, or one reformatted out of the block the parser reads.

**It is sharper than reading the diff**, and it is the only thing that would have caught the
2026-09-18 incident, where a maintainer reformatting a `Name:` block pushed three refusals out of
the parsed paragraph and the tree-wide count fell from 171 to 168. Nobody was reading for that; the
number was what spoke. The first performed rename under this procedure
(`capability_demo_protocol` to `capability_witness_protocol`) held at 273 across the change, which
is the result to expect.

Contributed by that lane, which was asked what the next rename should do differently and answered
with this first.

### Enumerating is cheap; classifying is the whole cost

**Hand the next lane the classification, not the file list.** The same lane measured it: enumerating
27 occurrences across 16 files took two minutes, and deciding whether **one line** was an account or
a pointer took twenty. A larger rename has proportionally more of the second and not much more of
the first, so a brief that supplies the list and stops has helped with the cheap half.

**The case the status table does not cover, and it is the one that costs the twenty minutes: a
dated account inside a live document.** `design/roadmap/proposals/`'s refusals proposal is
`PROPOSED`, so the table says it moves; the specific sentence in it recorded three measured counts
taken against a crate under the name it had that morning, so that sentence is evidence and stays.
Read the paragraph, not the heading. The resolution is to keep the old name **and add a clause
saying why**, which is what that file now carries.

### The ownership test: does it keep its name when our crate is deleted?

**If yes, the name is not ours and does not move.** One question, and it settles the class that
accounts for most occurrences in every rename so far.

It is what kept `satp.ASID` and `TTBR0_EL1.ASID` (fields in two instruction-set manuals), and what
kept `crates/pci`'s `CLASS_NVME` and `find_nvme_device` when the `nvme` crate was renamed around
them: `01:08:02` is a class code the NVM Express specification defines, and it keeps that name
whatever this tree calls its driver. Waiting for the renames still to come: `.dtb` is a file format
`dtc` writes and QEMU reads, and `GUID Partition Table` is UEFI's phrase.

The test is ownership rather than subject matter. A thing can be *about* our crate and still not be
named by us.

### Counting is not classifying, and only reading finds the rest

**Take the census twice, and read the after-census line by line.** The `nvme` rename went 615 to 477
and the classification pass over those 477 caught three sites that `script/lint`, `script/names` and
a green four-leg `script/test` had all passed over: a module header still saying `Provisional` after
ratification, a `println!` prefix, and a path inside two QEMU runner scripts.

**None of the three is a link, a symbol, or a name any parser reads**, which is exactly why no gate
saw them and why the count alone would not have either. The number tells you the sweep ran; only
reading tells you it was right.

### A path is navigation; a name in an account is a claim

A `BUILT` block keeps the old *name*, because it is an account of what was built under it. It does
**not** keep a broken *path*. The `asid` rename hit this on 2026-09-18: milestone 15's block cites
the note that moved, and the lane updated those citations while leaving every use of the name
itself. **The account rule protects names-as-used-then; it does not protect a link that now goes
nowhere.**

The test is whether a reader follows it or reads it. A path is followed.

### Expanding an acronym changes what clippy sees

`crates/asid` passed `doc_markdown` for a year. `crates/address_space_identifier` has underscores in
it, so clippy demands backticks, and three doc comments failed `-D warnings` on text nobody meant to
touch. **Every acronym expansion under §154 will hit this**, because every expansion introduces
underscores where there were none.

**Backtick every `crates/<name>` inside a `///` or `//!` before running the gate**, rather than
discovering it when the gate is red and the diff is large.

### A sweep on `name::` does not catch `](name)`

`kernel/src/arch/riscv64/mmu.rs` carried an intra-doc link whose target was the bare crate name.
A sweep looking for `asid::` never saw it, and **no gate reports it**: rustdoc warns only when it
runs, which is not on every build. It is the stale-pointer-upgrade class one level down, so grep
`](<name>)` as its own pass.

### `components/` is a second workspace, and `cargo check` is blind to it

The main workspace's check does not compile `components/`, so a rename that breaks a consumer there
is green until something builds it. `gpt` has consumers in it (the `gpt` rename built and ran them); `dtb` has none, which this line wrongly said it had until the `dtb` lane checked with `git grep`, and `asid` had only the
kernel, which is why the first three renames never exercised this. **Build both workspaces, or run
`script/test`, which does.**

### Two mechanical tells worth ten seconds each

**`git status` must say `R`, not `A`.** A crate rename moves a directory, and `RM crates/old ->
crates/new` is what you want. An `A` means the directory was copied rather than moved, which is how
757 lines of duplicate source once sat outside every workspace and surfaced only as a stray
`script/names --unratified` entry.

**Never let a sweep near a `Name:` block's `replacing` clause.** `perl -pi` will eat the very
sentence that records what the name replaced, which is the blind-`sed` scar in miniature. Sweep the
code against a file list you typed out, then open every `crates/*/src/lib.rs` block by hand. That is
a small enough set to read.

### Status decides what moves, not directory

| Kind | Moves? | Why |
|---|---|---|
| `BUILT` roadmap block | No | An account of what happened, under the names it happened under |
| Dated audit report | No | Same, and the date is on the file |
| Closed decision | No | What was decided, in the words used then |
| `PROPOSED` proposal | **Yes** | Live intent; a reader picks it up and goes looking |
| `PARTIAL` roadmap block | **Yes** | Its outstanding scope is work somebody will do |
| Any quotation | **Never** | See below |

Same rule the nife rename follows (`AGENTS.md`'s header: older records keep the old name where they
describe the past), at the granularity a person performing a rename needs.

**It was got wrong on the first pass of the `manual` rename**, 2026-09-13, which is why it is here.
Twelve `design/` files were correctly left alone and three were not: `the-two-unexplained-mutation-scores`
carried `manual` at 52% **in its title**, `colour-and-the-pager` was about `doc` being unable to
page, and `fatal-risk-3-against-the-new-number` cited the crate in a score table. All three
`PROPOSED`. The directory looked like history; the status said otherwise.

**Status is a property of the passage, not of the file, and the `jh7110` rename found the other
direction of the same error** (2026-09-13). `notes/model-attribution-review.md` announces itself as a
**plan**, so by the table above it moves. Two of its tables do not: they count the crates and
programs created inside one measured commit window, and a crate created in that window under the name
`jh7110_trng` did not exist under any other. Sweeping them made a live plan carry a false
measurement, which nothing checks and no reader can spot. The same shape hit
`notes/proof-retrospective.md`, where a captured shell transcript reading `in no shard: jh7110_trng`
was rewritten into output that command had never produced, and `notes/unsafe-obligations.md`, where a
block count measured against a named base commit was restated under a file name that commit does not
contain.

So read the **paragraph**, not the heading: a live document routinely contains dated accounts, and a
dated account routinely contains pointers that must still resolve. The repair in all three cases was
the one this section already prescribes, which is to restore the measured name and put a sentence
beside it saying what the thing is called now.

### A quotation never moves

Put a note beside it saying the thing was named differently when it was measured, so a number stays
traceable to the run that produced it.

**This is the tree's oldest naming scar.** A blind `sed` swept a rename across the tree and rewrote
the very row recording that a name had been *refused*, and the refusal it destroyed was the one that
would have prevented the rename. Milestone 115 and `script/names` exist because of it.

### Enumerate before sweeping

The match count is not the rename. Renaming `crates/manual`:

- **89 files** matched the word
- **37** were the crate, its note or its program
- The rest were the English word ("manual resume", "not intended for manual editing"), a captured
  boot log (`vf2-2026-09-01-manual-boot.log`), and **`manual_let_else`**, a clippy lint in the root
  `Cargo.toml` a sweep would have silently broken

List the true positives, read them, then edit. `git grep -l` over a narrowed pattern is the whole
technique, and the cleverness is the hazard.

### Renaming a crate is compiler-checked; renaming a program is not

**This is the clause the other three do not cover, and it is a different error.** "Enumerate before
sweeping" guards false *positives*: most matches are not the name. This guards false *negatives*, and
the two want opposite habits. One says do not trust the match count; the other says the match count
is not the whole set and nothing will tell you.

Change `crates/manual` and `cargo check` finds every site missed. Change the program `doc` and the
compiler is silent, because a program's name reaches the running system as a **string literal** in
tables nothing type-checks.

Where it hides, from the two renames that found it:

| Site | Example |
|---|---|
| The shell's command table | `b"doc" => Some(Prog::Doc)` in `crates/grant_plan` |
| ...and its reverse map | `Prog::Doc => "doc"` |
| Archive tuples in `xtask` | `("doc", "doc")`, **once per architecture** |
| A gate's expectation row | `("doc", &["name a file"])` |
| `program(...)` lookups | `user::program("jh7110_trng")` in `kernel/src/main.rs` and the entropy tests |
| `[[bin]]` name and path | `user/Cargo.toml` |
| Shell command strings inside tests | `parse(b"heeder report.txt")` |
| Fixture strings in other crates | `crates/timetable`'s `"every 5s heeder"` |
| A configuration file the tree ships | `components/timetable.conf`'s `at-boot budgeter --mem 4` |
| Identifiers derived from the program's name | `saw_budgeter_grant`, `budgeter_reports`, four test function names |
| A provenance block's "replacing" clause | `Name: ... replacing the provisional jh7110_trng` |
| Another project's file name, URL, version string or identifier | `$NetBSD: jh7110_trng.c,v 1.2 ...`, the fetch URL beside it, and `jh7110_trng_init` in its text |

**The configuration-file and derived-identifier rows were added by the `budgeter` rename on
2026-09-13, and both hide in a way the others do not.** A `.conf` is invisible to the habit that makes this technique cheap: `git grep`
narrowed with `--include=*.rs --include=*.md --include=*.toml` is how most of these sweeps are
scoped, and it misses a shipped configuration file entirely, while `crates/timetable` compiles that
one in with `include_str!` and the kernel asserts on it firing. Derived identifiers hide for the
opposite reason, which is that they are *not* string literals and the compiler does find them:
`saw_budgeter_grant` and `budgeter_reports` would have compiled fine under the old spelling and left
the tree naming a program that no longer exists, in the one place a sweep's own grep still finds
them. Neither is exotic; both were hit by the `worker` rename earlier the same day and recorded only
in its commit message, which is rung four.

**The last two rows, the provenance clause and the foreign citation, are the first that mark a site a
sweep must *not* touch, and the table earns them anyway** (the `jh7110` rename's repair,
2026-09-13). Every row above them is a false negative, a place the sweep missed. These two are false
positives. They belong here because the technique that catches both is the same one, which is
enumerating the matches and reading them rather than counting them.

**The provenance sentence is the worst place in the tree to sweep blind, because it is the one
occurrence of the old name the standard exists to protect.** `95db4a3e` renamed `jh7110_trng` to
`jh7110_entropy_source` and `jh7110_crg` to `jh7110_clock_and_reset`, and in both crates it rewrote
the clause naming the predecessor. Each block came out saying it replaced **itself**, and the old
name was then unrecoverable from the block: it had to be read back out of `git log`.

**The only reason that was caught is that the resulting sentence is self-referentially absurd.**
"Replacing the provisional `jh7110_entropy_source`" inside `jh7110_entropy_source` reads as nonsense
to anyone who looks at it. A rename between two less similar words produces a sentence that reads
perfectly and is false, and nothing here would say so: `script/names` parses the block and prints it
and has no opinion about whether the name inside is the one being replaced. So treat the `replacing`
clause exactly as a quotation is treated above, because that is what it is. It quotes a decision.

**A citation to another project is a quotation wearing a path.** The same commit rewrote
`sys/arch/riscv/starfive/jh7110_trng.c` to `jh7110_entropy_source.c` in three places in one file: the
source bullet, its `raw.githubusercontent.com` fetch URL, and the reference-link definition at the
bottom. One of them carried NetBSD's own RCS keyword string, `$NetBSD: jh7110_trng.c,v 1.2 2025/02/09
09:09:49 skrll Exp $`, which is a verbatim line out of somebody else's source file. The tree then
cited a file that does not exist upstream and a version string nothing ever printed, which is the
fabricated-quote failure this project has already carried once for twelve days. The seven questions
say prior art is read rather than recalled; a swept citation is a citation recalled, with `sed` doing
the recalling.

The sibling that survived shows it was luck rather than care. `crates/jh7110_clock_and_reset` cites
Linux's `starfive%2Cjh7110-crg.h` and is still right only because upstream spells that one with a
hyphen where the sweep matched an underscore.

**There was a fourth site in that same file, and it outlived the repair**, found 2026-09-14 by the
rename that replaced `jh7110_entropy_source` with `jh7110_entropy`. `95db4a3e` also rewrote the
**function name inside** NetBSD's driver, so the crate's bring-up section credited the sequence to
`[netbsd]'s jh7110_entropy_source_init`, a symbol that exists in no tree anywhere. `0cfb6f63`
restored the two `replacing` clauses and did not look for this, because the three sites it knew
about were all paths and this one is an identifier. It was repaired by fetching the file
(`raw.githubusercontent.com/NetBSD/src/trunk/sys/arch/riscv/starfive/jh7110_trng.c`, which also
re-confirmed the `$NetBSD: jh7110_trng.c,v 1.2 2025/02/09 09:09:49 skrll Exp $` line the crate
quotes) and reading the name out of it: `jh7110_trng_init`.

So the row above reads **file name, URL, version string, or identifier**. The general shape is that
a foreign name does not have to look like a path to be somebody else's, and the sweep's own pattern
is what decides which of them it eats: an underscore-spelled rename matches an
underscore-spelled foreign symbol and leaves a hyphen-spelled one standing, which is why the
survivor above survived and this one did not.

**The evidence is one failure and one success, a commit apart.** Renaming `doc` to `mdr` left
`grant_plan` still saying `doc`, so the shell could not spawn the binary and the archive did not hold
what the gate looked for; `cargo check` passed and three CI jobs failed for that one cause. The
`jh7110` rename the same day enumerated strings first, found all four sites, and pushed green.
**That success was real and partial**, which is why the paragraphs above exist: the same commit
broke four records, fabricated three external citations, and left a whole crate behind. Enumerating
the *program* strings is one clause of this standard and not the standard.

So: for a program, grep the **quoted** name as well as the identifier, and treat `cargo check`
passing as no evidence at all.

### A crate copied rather than moved is invisible to every gate but one

`95db4a3e` moved `kernel/src/drivers/jh7110_crg.rs` and `user/src/jh7110_trng.rs` properly, and git
records both as renames. `crates/jh7110_crg` it **copied**: the new directory was added and the old
one was never deleted, leaving 757 lines of duplicate source behind with no `Cargo.toml` at all.

**Nothing compiled it.** It was not in `Cargo.toml`'s workspace members, no manifest referenced it,
and this tree's gates are compile-driven almost everywhere, so there was no clippy over it, no test,
no coverage, no mutation sweep, and no `cargo check` that could notice the duplicate at all. A
directory outside the workspace is outside all of them at once.

The one gate that did see it is `script/names`, because it walks `crates/*/src/lib.rs` on disk rather
than the package graph. So the defect surfaced as a **worklist entry**: `jh7110_crg` went on
`script/names --unratified`, queueing a name nobody could compile into the one queue whose entire
purpose is to spend calef's attention well. Nothing red happened anywhere. The cost of this failure
was paid in the scarcest thing in the project rather than in a build.

**The general fact is worth more than the incident: an on-disk walker and a package-graph walker
disagree, and the disagreement is information.** `script/verify` and `script/falsifications` both
moved to `cargo metadata` because a hand-kept list went stale silently. `script/names` walks the disk
because a name exists whether or not it compiles. Neither is wrong, and a name present to one and
absent to the other is a thing to go and look at rather than reconcile.

So, after any rename that moves a directory: `git status` showing an **add** where you expected a
rename is the whole tell, and `git diff --stat -M` on the commit says which it was.
### A crate is compiler-checked only where a compiler is looking

**The clause above says renaming a crate is the easy case, and milestone 285 found the sentence too
generous.** `cargo check` finds every `use` and every `[dependencies]` key, which is most of the
work and all of the reassurance. It finds nothing where the crate's name has left Rust and become a
**path on disk** or an **argument to a build tool**, and those sites fail at link time, at gate time,
or not at all.

The tree already had the scar and had not generalised it. When milestone 175 moved `user/link.ld` to
`crates/user_rt/link.ld`, two `build.rs` files were missed because they live in **separate Cargo
workspaces**; nothing errored until a cross-compiling link, and CI's QEMU leg went red with nothing
before it saying a word. That is the same shape as everything below.

| Site | Example | Why the compiler is blind to it |
|---|---|---|
| A linker script referenced by path from a `build.rs` | `../crates/user_mode_runtime/link.ld`, in **four** build scripts, two of them in separate workspaces | it is a string handed to `rust-lld`, and the main workspace never builds the other two |
| `--exclude <crate>` in a gate | `script/lint`, `script/coverage`, and two lists in `xtask/src/main.rs` | cargo takes an unknown `--exclude` name silently, so the gate keeps passing while covering less |
| A mutation-testing exclusion glob | `.cargo/mutants.toml`'s `"crates/user_mode_runtime/**"` | a glob that matches nothing is not an error |
| A crate-keyed row in a measurement baseline | `.cargo/mutants-baseline.txt`'s `user_mode_heap 20 3 5 7` | a plain data file, keyed by crate name, that no build reads |
| An identifier derived from the crate name inside a gate's embedded script | `reaches_user_mode_runtime()` in `script/lint`'s python | it compiles and runs either way; only the reader is misled |
| A shell script that derives an artifact from the crate's directory | `helpers/build-ripgrep.sh` seds `crates/user_mode_runtime/link.ld` into a high-load variant | shell, and it runs only when somebody builds ripgrep |
| A generated module in the patched-`std` overlay | `sys/alloc/nife/user_mode_heap.rs`, written by `xtask` from the crate and declared `mod user_mode_heap;` in the overlay | it compiles only when the `std` farm is rebuilt, in a source tree outside every workspace |
| `Cargo.lock` in each separate workspace | `redoxfs_server/Cargo.lock`, `tools/redoxfs_host/Cargo.lock` | regenerated on their own next build, not on the main workspace's |
| A **glob that selects the set a gate then judges** | `script/lint` check 3 looped over `crates/*proto` and rejected any name not ending `_proto` | after milestone 265 that glob matches no directory, so the loop body never runs and the check passes by checking zero crates |

**The glob row is worse than the `--exclude` row above it and belongs beside it anyway.** Both fail
by going quiet, but an `--exclude` that has gone stale still covers everything else; a selector that
has gone stale covers nothing, and the gate's whole subject vanishes at once. The tell is the same
and it is not a failure: **a gate that passed before your change and passes after it, on a change
that is precisely its subject, has probably stopped looking.** Run it against a deliberately wrong
name once and confirm it still says no.

**The habit that catches all of them is the same one the program clause asks for**, applied a
directory wider: grep the **path** (`crates/<name>`) as well as the identifier, and then build every
workspace, not the one `cargo build` means by default. `find . -name Cargo.toml -maxdepth 3 | xargs
grep -l '\[workspace\]'` is the enumeration; there are five.

### A suffix rename is one decision and N provenance blocks

**The cost of a rename scales with the names it touches; the cost of *repairing* one scales with the
records those names appear in, and a family rename makes the second number much larger than the
first.** Milestone 265 renamed fifteen crates by changing one suffix, which is one decision, and then
had to read every one of their provenance blocks, because a provenance block's job is to record what
the name was.

Two shapes recur and are worth expecting:

- **The same account, copied into several crates.** Four crates (`clock_proto`, `entropy_proto`,
  `supervision_proto`, `swap_proto`) carried a byte-identical sentence about the wire contract having
  been spelled four ways on 2026-07-30. A sweep breaks all four the same way, so the repair is also
  four copies, and finding one is no evidence you have found them all. Grep the sentence, not the
  name.
- **An account whose subject is the spelling itself.** Those four sentences list four spellings in
  order to contrast them, and two of the four ended in the suffix being renamed. Swept, the sentence
  still parses and is now nonsense: it contrasts four spellings, two of which no longer contain the
  thing being contrasted. Nothing catches that but reading it.

**The general rule this is a case of**: when the thing being renamed is a *convention* rather than a
single name, every passage arguing the convention is an account, and there are as many of them as
there were names.

### The census is the checklist, and it is taken twice

Contributed by the fourth performed rename (`nvme` to `non_volatile_memory_express`, 2026-09-18),
which is the first one large enough that no person could hold the file list in their head: 615
occurrences across 93 files, against the `asid` rename's 446 and the two before it in the dozens.

**Take the full census before and after, and classify every line of the second one.** Not the
count, the lines. The after-census is the only artifact that asks "why is this one still here?"
of each survivor individually, and on this rename it caught **three** sites every other check had
passed over: a module header still calling itself "the volatile half of the `nvme` crate" and
still saying `Provisional` after the ratification, a `println!` prefix naming the driver in the
one place a user actually reads it, and `kernel/src/nvme.rs` written into two QEMU runner scripts
as a comment. `script/lint`, `script/names` and a green `script/test` on four legs had all passed
with those in the tree, because none of them is a link, a symbol, or a name a gate parses.

**The classification is also the report and the block's own evidence.** Sorting the survivors into
kinds (the standard as a proper noun, the emulator's device name, an account, a spec citation, a
slug, a neighbouring crate's identifiers) takes one script and turns "most occurrences are not the
crate" from a thing a brief asserts into a number the next lane can check. Here it was 241 / 71 /
68 / 30 / 26 / 26, and that distribution is now in the crate's own provenance block.

### A neighbouring crate's identifiers are the hardware's, not yours

`crates/pci` holds `CLASS_NVME`, `PciNvmeDevice` and `find_nvme_device`, and none of them moved.
They name the **PCI class code the specification defines** (`01:08:02`), the same way `satp.ASID`
and `flush_asid` named a hardware field through §154's first rename. The tell is ownership rather
than spelling: if the thing keeps its name when this tree's crate is deleted, the name is not
this tree's to change.

The same test disposes of the rest of that family in one pass, and it is worth listing because a
sweep's pattern matches every one of them: QEMU's `-device nvme`, the `NIFE_NVME` environment
variable, `target/nife-nvme.img`, and `clippy.toml`'s `doc-valid-idents` entry. The clippy entry
is the interesting one, because the instinct on a rename is to delete it: it exists so
`doc_markdown` tolerates the **proper noun**, the proper noun is what survives the rename, and
deleting it would turn 30 spec citations red for a word nobody renamed.

### A refusal the parser invented is not a refusal, and the count is why you leave it

`script/names --check` reports `non_volatile_memory_express` as "refused but live", and it is
neither refused nor a bug in the rename. A refusal clause runs to the end of its sentence; that
crate's sentence refusing `nvm_express` names the winning spelling while arguing against the
loser, so the parser records the winner too. It is the `video_terminal` NOTE's shape without
`video_terminal`'s real reason behind it.

**It was left alone deliberately**, and the reasoning generalises: rewording the sentence would
have moved the tree-wide refusal count, which is the one gate a performed rename is measured by,
and spending that signal to silence a NOTE that never fails a build is a bad trade. Record the
artifact beside the block instead, which is what that crate now does.

### A sweep can turn a stale pointer into a fabricated one

Contributed by the second performed rename (`address_space_builder` to `address_space_witness`,
2026-09-18), which found one site the sweep would have made worse rather than wrong.
`kernel/src/user.rs` carried ``See fixtures/src/hello.rs `address_space_builder()` ``, and
`hello.rs` has held no such function since milestone 291 split the role out into its own fixture.
The pointer was already stale, which nothing notices, because a prose reference resolves in a
reader's head rather than in a compiler.

**A sweep does not fix that and does not leave it alone; it upgrades it.** Swept, the line would
have read `` hello.rs `address_space_witness()` ``: a symbol that has never existed anywhere, cited
by its current name, and so indistinguishable from a true reference. The stale version at least
names something that used to exist and can be traced. This is the internal cousin of the foreign-
identifier row in the table above, and it hides better, because there is no upstream tree to check
it against.

So when a match is a *pointer* rather than a declaration, resolve it before rewriting it. The cost
is one grep per site and it is not optional: the two neighbouring doc comments on the same kernel
module point into `hello.rs` for `ep_maker()`, `ep_user()`, `call_server()` and `call_client()`,
none of which are there either, and they were left alone only because this rename did not touch
them.

**The seventh rename found the same shape at scale** (`ipc` to `inter_process_communication`,
2026-09-19). §113 renamed `Endpoint` to `Rendezvous` on 2026-08-23 and nothing moved the prose, so
nine sites still said `ipc::Endpoint` a month later. A `ipc::` sweep would have turned every one
into `inter_process_communication::Endpoint`, a type that has never existed. They were classified
instead: the ones in decided sections are accounts and kept their words with the current name beside
them, and the ones in notes describing today's code were repointed to `Rendezvous`. Reading those
lines found two more pointers of the same age (`Rendezvous<Tid>`, and `crates/intrusive` for a
crate now called `intrusive_fifo`). **A type rename leaves a trail of stale prose, and the next
crate rename walks straight into it.**

### A tool that does not understand `\b` does not say so

Contributed by the `dtb` and `ipc` renames (2026-09-19), which each lost a count to it. On macOS,
`git grep` does not support `\b` in its default pattern syntax and **matches nothing**, so
`git grep -c '\bdtb\b'` reports zero across a tree with eighty-six hits in it. The macOS `sed` drops
`\b` the same way, so a substitution meant to be word-bounded silently rewrites nothing, or with a
different pattern rewrites too much. Neither prints a warning.

**Treat a zero as a claim to re-check, never as a result.** Re-run it with `grep -rE` and an
explicit class (`(^|[^a-z_])ipc::`), or with `git grep -w` where a word match is what you want.

### A `Name:` block can move its own census

The `dtb` block described the crate's files as `.dtb` files, so the rename that wrote the block
counted it: 88 hits against a census of 86, and two phantom survivors to classify. The block now
says "the blob files' extension". When the after-census is off by a small number, check the
provenance block you just wrote before the tree.

### A hand rewrap needs a width check afterwards

Expanding a name lengthens lines, and every rename in this series rewrapped paragraphs by hand or by
script. Two failures were both invisible to the gates: lines left past the file's hundred columns,
and a list marker given a second space by a wrap script. After a rewrap, list the added lines longer
than the file's width (`git diff -U0 | grep '^+[^+]' | awk 'length > 101'`) and read a
`--word-diff` of the result; the word diff should show only the names you meant to change.

### The generated roadmap index was not a sweep target, and running the generator proved it

*The index this section is about was retired on 2026-09-21 and `--write` went with it, so there is
no generated file left in a rename's path. The lesson is kept because the next generated file will
raise the same question.*

The same lane was briefed to run `script/roadmap --write` after editing, on the reasoning that
`design/roadmap/README.md` is generated and would pick the rename up. It reported **"index already
current"** and wrote nothing, which is the right answer: the README's one occurrence sits inside
milestone 295's summary, mirrored from a `BUILT` block that keeps the name it was written under.

**A generated file inherits its sources' status rather than having one of its own**, so it needs no
classification at all. Running the generator is still worth the ten seconds, because it is the one
command that decides the question: a generator that writes nothing has confirmed the sources were
classified correctly, and one that writes something has found a source you missed.

### What is checked, and what is not

`script/names --check` catches one member of this family: a name recorded as refused that is also
live. Nothing catches a rewritten quotation, a stale `PROPOSED` proposal, or a lint that shared a
substring, and a check that tried would be guessing at intent.

**So this is rung three**, a written record at the thing a person is about to do, and it says so.
The higher rung is not available: no gate can tell an account from an intention in prose, which is
the reason `AGENTS.md` gives for not gating identified work either.

## BUGS

What milestone 63 did **not** rename, each on purpose, so the next reader does not "fix" one of them
by mistake.

- **Two records under `design/` still spell names milestone 175 retired, and one of them is a
  present-tense claim.** `design/capsicum-and-the-retrofit-question.md`'s honest comparison says the
  system confines "`worker`, `budgeter`, `heeder`, `spinner`, a C component, and a filesystem we
  vendored", and `AGENTS.md`'s rule 7 section describes `user/src/` as a live directory. Both were
  left where they are because a developer lane edits its own milestone's roadmap block and nothing
  else under `design/`, and never `AGENTS.md`. Every *other* occurrence of the old names in
  `design/` is a dated narrative and correctly keeps them. Neither is load-bearing; both are one
  line for whoever next has the standing to make the edit.

  **Half of the first one closed on 2026-09-13**, and the way it closed is the point rather than the
  tidiness. The `memory_grant_depleter` rename lane was already editing that sentence's `budgeter`,
  because a present-tense claim moves whatever directory it sits in, so the word it was there to
  correct went with the sweep that had to touch the line anyway. `worker` is still there, and this
  entry is still open for it: performing *that* ruling was a different lane's, and a rename is not a
  thing to do on the way past. The general shape, worth more than either word: **a stale record gets
  fixed when something else brings a writer to the line**, not when somebody schedules a pass over
  it, which is why the entry names the line rather than filing a task.

- **The boot mode is still called `shell`, and the program is `swish`.** `cargo xtask shell` and the
  kernel's `--features shell` name a *configuration* (boot straight to a prompt, milestone tour
  compiled out), not the binary. Renaming them would have been a naming decision nobody made. The
  cost is that a reader who greps `shell` in `xtask` and in `kernel/Cargo.toml` meets a word that no
  longer names a program.
- **`caps` is still a shell builtin**, and it no longer shares a name with anything. It was the
  larger half of the 285 occurrences of `caps` in the tree before the rename, and it means "print
  this process's endowment". The crate that used to share the spelling is `capability`.
- **`crates/virtio`'s first line still says "A virtio-blk driver".** It also drives net, serves
  blocks through `run_blk_server`, and carries two deliberate attack roles. The crate keeps its name,
  which is right, but the sentence under it is wrong. Out of scope for 63 and not yet filed anywhere
  else.
- **A rename can reach into the vendored tree, and `script/supply-chain` is what says so.** Our one
  divergence comment in `vendor/redoxfs/Cargo.toml` names `redoxfs_host`, so the rename had to touch
  the vendored file **and** `vendor/redoxfs.divergence.patch` together or
  `script/vendor-verify` fails with "differs from upstream+patches". Milestone 63 changed the patch
  first and the vendored file not at all, and the gate caught it. Edit both, in the same commit.
- **The measured-boot manifest is still `target/init-measure-<arch>.txt`.** It is a build artifact
  name, not a crate reference; `kernel/build.rs` reads it and turns it into `TRUST_ROOT`.
- **Note filenames did not move**, and that is the rule rather than an oversight:
  [fs-server.md](../notes/fs-server.md), [shell.md](../notes/shell.md), [shell-navigation.md](../notes/shell-navigation.md) and
  [line-discipline.md](../notes/line-discipline.md) are markdown, so they stay lowercase-hyphenated even
  though the things they describe are now `redoxfs_server`, `swish` and `line_editor`.

## The casing of `nife`, considered and settled

Raised 2026-08-15, the day of the rename: should prose write `Nife` (ordinary proper noun) or
`NiFe` (the chemically exact form, how Suess and Edison's batteries spell it)? **Lowercase
`nife` everywhere, kept.** Identifiers are lowercase regardless (crates, the triple, the repo),
so any other choice splits the spelling per context and drifts to three forms in practice. The
camel seam in `NiFe` also fights the ratified pronunciation (said like *knife*) by inviting
"nye-fee". The refusal record matters more than the choice: the sial/sima rule applies, since a
name that needs a casing note is the pronunciation-note tax in different clothes. The chemistry
lives in the README's one line, which is where it costs nothing.
