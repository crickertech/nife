# 213. A harness that re-implements the code instead of calling it proves nothing about the code

**Status: NOT-STARTED.** Minted 2026-09-01 by milestone 211's sweep, which was looking for a
different defect and found this one beside it. *(Number provisional until the merge queue lands
it.)*

**Gate: NONE.** Same reason as 211's: no check tells a deliberate model apart from an accidental
copy.

## In brief

Milestone 211 asked whether a harness can see the function under test break. The answer for
`nifefs::the_validation_implies_reads_slice_is_in_bounds` is no, for a reason 211's class does not
cover: **it never calls the function.** Neither `Fs::parse` nor `Fs::read` appears in it. The
harness recomputes both functions' arithmetic inline and proves a property of that recomputation:

```rust
// Exactly parse's acceptance condition, no more.
let start = start_block as usize * BLOCK;
...
// Exactly read's slice arithmetic.
let r_start = start_block as usize * BLOCK;
```

The two comments are honest, which is what makes this a design to reconsider rather than a mistake
to fix quietly. But "exactly" is a claim in prose, and a claim in prose is what DECISIONS §134
exists to stop standing in for evidence. Rewrite `read`'s slice arithmetic and the harness stays
green.

## Why it is worth a milestone rather than a one-line fix

The obvious repair, calling `parse` and `read` from the harness, is what the harness's author
avoided, and the reason is probably tractability: `parse` walks a directory over a symbolic image.
So this is a **restructuring** question rather than a harness question, and it is the shape
DECISIONS §46 rule 1 already describes: factor the acceptance condition and the slice bounds into
functions small enough for the prover, which both the implementation and the harness then call.
`elf`'s `check_segment_bounds` is the worked example already in the tree: a leaf factored out of a
loop precisely so a bounded model checker can quantify over it, with the real parser calling the
same function.

## The sweep

- Confirm the shape in `nifefs` and factor the two conditions out so the implementation and the
  harness share them.
- Ask the same question of the rest: which harnesses assert over values they computed themselves
  rather than over values the crate produced? This is mechanically narrowable in a way 211's class
  was not, because it is the *absence* of a call to the crate under test.

## BUGS

- **Some duplication is correct.** `intrusive_fifo` keeps a model queue on purpose and compares
  the real one against it, which is the good version of this and must not be swept up. The
  difference is whether the crate's own function is on the other side of the comparison.
- **The repair can be worse than the defect.** Factoring a condition out to make it provable adds
  a function whose only caller-visible reason is the proof, and §46 refuses machinery taken for
  tidiness. Whether the `nifefs` split earns its keep is the first thing this milestone should
  decide, not assume.
