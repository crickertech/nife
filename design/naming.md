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
| `script/` and `scripts/` entry points | `hyphens` | shell commands are hyphenated everywhere (`apt-get`, `pkg-config`, `docker-compose`); an underscore in a command name reads as a mistake |
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

## Appendices

Each former section of this page now lives in one of these files, verbatim, under its old heading.

| appendix | holds |
|---|---|
| [programs-scripts-and-directories.md](naming/programs-scripts-and-directories.md) | the argument that a name is a claim, the component and program conventions, shell builtins, `script/` against `scripts/`, and directories |
| [crates.md](naming/crates.md) | the crate rule and how far it reaches, nouns over verbs, the acronym history, and what `crates/` holds |
| [provenance.md](naming/provenance.md) | milestone 115's mechanism: why the record is derived, the states, the `Name:` block forms, the numbers, and worked `script/names` examples |
| [provenance-limits.md](naming/provenance-limits.md) | the known limits of `Name:` blocks and `script/names`, including the kinds of name the worklist does not cover and why `scripts/` helpers are left out |
| [documents-numbers-and-gates.md](naming/documents-numbers-and-gates.md) | where a document goes, why `§N` is not milestone N, why a numbering gap passes, the branch convention, and what `script/lint` checks |
| [vocabulary-rulings.md](naming/vocabulary-rulings.md) | the individual words calef has ruled on and the tests they set: halves and arms, abbreviations we receive, structural termini, identity and principal, the `login` stem, and the casing of `nife` |
| [rename-what-moves.md](naming/rename-what-moves.md) | which occurrences of a ratified name move and which stay, the refusal-count gate, and the habits that keep a sweep from rewriting a record |
| [rename-where-names-hide.md](naming/rename-where-names-hide.md) | the sites a compiler cannot see when a program or crate is renamed, and why the census is taken twice |
| [rename-traps.md](naming/rename-traps.md) | the smaller failures past renames hit: foreign identifiers, parser artefacts, stale pointers, broken word boundaries, rewraps and generated files |
| [what-milestone-63-left-alone.md](naming/what-milestone-63-left-alone.md) | the names milestone 63 left in place on purpose, so the next reader does not "fix" one by mistake |
