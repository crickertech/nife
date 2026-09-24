# Performing a rename: what moves

*An appendix to [`design/naming.md`](../naming.md), which is the rule. This file holds which occurrences of a ratified name move and which stay, the refusal-count gate, and the habits that keep a sweep from rewriting a record. It exists to verify or challenge the main page, and a reader who only needs to name, ratify or rename something should not have to open it. The directory `design/naming/` and this file's stem are provisional names, minted 2026-09-24 by the lane that split the file; naming is calef's.*

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
