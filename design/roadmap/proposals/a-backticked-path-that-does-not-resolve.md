# A gate for a backticked in-tree path that does not resolve

**Status: PROPOSED 2026-09-05.** Found by milestone 259's notes sweep, which spent more than half
its corrections on this one shape and would have spent none of them if a gate existed.

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
