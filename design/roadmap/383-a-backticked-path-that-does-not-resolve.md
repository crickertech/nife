# 383. A gate for a backticked in-tree path that does not resolve

**Status: NOT-STARTED.** Filed 2026-09-05 as an unnumbered proposal by milestone 259's notes sweep,
which spent more than half its corrections on this one shape; numbered 2026-09-19 by milestone 433's
drain of the proposal pile. **Premise re-read against the tree on 2026-09-19 and still true**:
`script/lint`'s markdown section still checks relative link targets and the `notes/README.md` index
and nothing else, and its own comment still says backticked repo paths are "deliberately NOT
checked". That comment has already retracted half its own justification: it was re-measured by
milestone 93's documentation sweep at 31 unresolvable backticked paths over 379 markdown files,
where it used to claim a checker would be 100% false positives, and it now rests on the
false-positive *rate* rather than on perfection. This block is the proposal that the rate is
enumerable. *(Number provisional until the merge queue lands it.)*

**Gate: NONE.** It reads the tree and needs nothing.

## In brief

**262 citations in `notes/` pointed at a crate, a file or a Rust path that had been renamed away**,
and every gate in this repository passed them. `script/lint` check 4c verifies that a markdown
*link* target exists. `script/citations` verifies that a `§N` or a `milestone N` resolves to the
thing the author meant. A path in backticks is neither, so `` `crates/fs_proto` `` was
unfalsifiable prose for the two weeks after §75's naming pass renamed the crate.

`notes/follow-on-work.md` recorded the same rot one directory over, in the roadmap's own blocks, and
its sentence is the whole argument for this: *"Nothing had been checking a path cited in a roadmap
block."* Nothing is checking one cited in a note either, and there are more of them.

## What it would check

For each `` `path` `` in a tracked markdown file where the path starts with a directory that exists
at the repository root (`crates/`, `kernel/`, `user/`, `script/`, `design/`, `notes/`, `fuzz/`,
`bench/`, `patches/`, `xtask/`, `tools/`), assert the path exists, after stripping a `::symbol`
suffix and an anchor.

**The false positives are the whole design problem, and they are enumerable**, which is what makes
this rung two rather than a `git grep -w TODO` at 82%. Milestone 259 hit exactly four kinds:

1. **Verbatim transcripts.** A quoted panic said `crates/frames/src/lib.rs:315` because that is what
   it said, and rewriting it would falsify the evidence. Two instances.
2. **Deliberate past tense.** `notes/ntlm.md` and `notes/smb.md` describe code removed on
   2026-08-30 and say so in their first line; `notes/heap.md` carries a banner recording that
   `crates/heap` was deleted. Fifteen instances across four files.
3. **A path named because it does not exist.** `notes/register-of-measures.md` says "there is no
   wrapper and there should not be one" about `script/measures`;
   `notes/footprint-perturbation.md` says `kernel/src/arch/x86_64/fastpath_pad.rs` does not exist.
   Three instances.
4. **Elided or illustrative paths.** `patches/std-nife/.../pal/nife/rt.rs`, and
   `notes/documentation-audit.md`'s `design/dec/` in an argument about spelling words out.

So the check needs an escape, and the cheapest honest one is an allow-list file carrying **the
reason per entry**, the shape `xtask`'s `ABORTS_ACCEPTED` already uses. Twenty-odd entries is a
readable list, and a new one arriving with no reason is what the gate is for.

## The trap this must not fall into, which the sweep found by falling into it

**A crate is named three ways in this tree**: as a path (`crates/fs_proto`), as a Rust path
(`fs_proto::PAGE`), and as a bare name in prose (`` `fs_proto`'s own BUGS section ``). The sweep's
first pass matched the first, reported itself clean, and left 167 instances of the other two. The
same thing happened one size smaller in `notes/documentation-audit.md`, whose own path check matched
`` `user/src/virtio.rs` `` and missed `` `user/src/virtio.rs::write_block` `` on the next line.

A check that only knows one spelling reports a clean tree and is worse than none, because it retires
the worry.

**The bare-name form is the hard one and probably should not be gated.** `` `slots` ``, `` `frames` ``
and `` `regions` `` are ordinary English words, and every occurrence in `notes/` turned out to be a
crate reference only because those were bad crate names. A gate over bare identifiers would fire on
prose. The path forms are unambiguous and are where a reader actually goes.

## Scope

**Not scoped to `notes/`.** `fuzz/seeds/README.md` carries the same dead path as
`notes/fuzzing.md` (`crates/elf/tests/fuzz_seed.rs`, deleted by `acc2338a` and restored by
milestone 259), and `design/roadmap/` has the disease `notes/follow-on-work.md` already recorded.
Every tracked markdown file, or the check is the sweep it is trying to replace.

## What it would have caught

Every one of milestone 259's 262 path corrections, and one thing worth more than the prose: the
`crates/elf/tests/fuzz_seed.rs` deletion. That commit was about adding `Segment::p_paddr` and never
mentioned the 62-line test file it removed, so **a documented safeguard disappeared and two
documents kept describing it in the present tense for five days.** A path check would have failed
that commit's own CI run.

## The symbol half, which this check strips and a rename walks into

*Added 2026-09-19 by the maintainer, from the `ipc` to `inter_process_communication` rename.*

The check above strips `::symbol` and asserts only the path, so `` `ipc::Endpoint` `` passes as long
as the crate exists. §113 renamed `Endpoint` to `Rendezvous` on 2026-08-23, and nine prose sites
still cited `ipc::Endpoint` a month later. A crate-rename sweep would have turned every one into
`inter_process_communication::Endpoint`, a type that has never existed: design/naming.md's "A sweep
can turn a stale pointer into a fabricated one" has the case. A symbol that does not resolve is the
same defect as a path that does not, one `::` further along.

**Measured with a throwaway prototype, 2026-09-19**: over every tracked `.md` file, match
`` `crate::Symbol` `` where `crate` is a directory in `crates/`, and grep that crate's `src/` for a
definition of `Symbol`.

- **992** such references; **122** do not resolve (66 in `design/roadmap/`, 9 in
  `design/decisions/`, the rest in `notes/`).
- **The precision is poor, and for a reason this proposal's escape does not cover.** The sample
  mixes true stale pointers (`capability::CSpace`, which §113 renamed) with **kernel modules that
  share a crate's name**: `pci::find_block_device` and `virtio::register` live in
  `kernel/src/pci.rs` and `kernel/src/virtio.rs`, not in `crates/pci` or `crates/virtio`.

So the symbol check wants two things the path check does not: **resolve against both namespaces**
(`crates/<c>/src` and `kernel/src/<c>.rs` or `kernel/src/<c>/`), then remeasure before deciding
whether the remainder is an allow-list or a gate. Resolution by grep will still miss a symbol made
by a macro or reached through a re-export, which is a BUGS line for the check rather than a reason
not to write it. Nothing was fixed from the 122; they want classifying first, by status, as the
rename procedure says.

## Index row

262 citations in `notes/` pointed at a crate, a file or a Rust path that had been renamed away, and
every gate in this repository passed them: `script/lint` check 4c verifies a markdown *link* target,
`script/citations` verifies that a `§N` or a `milestone N` resolves to the thing the author meant,
and a path in backticks is neither, so `` `crates/fs_proto` `` was unfalsifiable prose for the two
weeks after §75's naming pass renamed the crate. The check is one pass over every tracked markdown
file, asserting that a backticked path rooted in a directory that exists really exists, with an
allow-list carrying a reason per entry in the shape `xtask`'s `ABORTS_ACCEPTED` already uses.
Milestone 259 hit exactly four kinds of legitimate exception (verbatim transcripts, deliberate past
tense about deleted code, a path named *because* it does not exist, and elided illustrative paths),
which is what makes the escape a readable list rather than a shrug. The trap is that a crate is
named three ways here, as a path, as a Rust path and as a bare word, and the sweep's own first pass
matched one, reported itself clean and left 167 instances: a check that knows one spelling is worse
than none, because it retires the worry. What it would have caught, beyond the prose, is the
deletion of `crates/elf/tests/fuzz_seed.rs` by a commit about `p_paddr` that never mentioned it,
which left two documents describing a safeguard in the present tense for five days.
