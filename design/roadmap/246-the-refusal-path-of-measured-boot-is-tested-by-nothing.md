# 246. Measured boot's refusal path is tested by nothing, and one mutant turns it off

**Status: NOT-STARTED.** Minted 2026-09-03 by calef, from milestone 244's (the largest crate in the
tree is proved by nothing a mutation can reach) recorded limitation. *(Number provisional until the
merge queue lands it.)*

**Gate: NONE.** The function is eighteen lines and its dependencies already compile for the host.

**In brief.** `system_initializer::measured` decides whether a component the system is about to start
was vouched for by the measured-boot manifest. It is the enforcement point of DECISIONS §104's
measurement, and **no test in this tree ever takes its refusing branch.**

```rust
if measured_boot::verify_in_manifest(table, name, bytes).is_err() {
    return Lookup { elf: None, unvouched: true };
}
```

**Change that `true` to `false` and an unvouched binary starts.** Nothing goes red. Milestone 244
measured that and recorded it beside the code: the function is *"proved only by `script/shell-check`
booting a system whose table happens to be right"*, which exercises the accept path and never the
refuse path.

## Why this is its own milestone rather than part of 244

**244 measured the whole crate and correctly declined to lift it**: 33 of 196 mutants are in pure
leaf functions, about sixty lines of 2,632, and moving them would put the boot path behind a feature
flag for a flattering score. That is the right answer for the aggregate.

**It is the wrong lens for this one function**, and separating them is the point of this block. The
other pure leaves are `hex_password`, `sentence`, `archive_name` and `opt_cap`: a mutant in any of
them produces a wrong string or a wrong option. A mutant here produces **a system that runs code
nobody vouched for**, which is the property `measured_boot` exists to provide. Counting those
mutants as interchangeable is what makes an aggregate fraction the wrong instrument.

## What makes it cheap, which is why it is worth doing now

`measured` takes a `nifefs::Fs`, a `&str` table and a `&str` name. **All three of its dependencies
already compile for the host** (`nifefs`, `measured_boot`, `elf`), and every one of them is already
mutated and covered. The function sits in an excluded crate because of its neighbours, not because of
anything it does.

## The proof that this milestone worked

**A host test that fails when the refusal is removed.** Concretely: a manifest and an archive that
disagree, asserting `unvouched` is set and no `Elf` is returned; and the mutant `unvouched: false`
turning it red. State the mutant that was run and that it was caught, the way milestone 193's block
states its falsification.

Not a test that only takes the accept path, which `script/shell-check` already does better.

## BUGS

- **This does not make `system_initializer` measurable**, and should not be quoted as if it did. 163
  of its 196 mutants stay unreachable by construction; milestone 244's block carries the split.
- **A refusal proved on the host is not a refusal proved on the boot path.** The host test asserts
  the function's contract; that the boot *calls* it, on both ISAs, with the table the kernel measured,
  stays `script/shell-check`'s claim and nothing here strengthens it.
- **`Lookup` may have to move with the function**, which widens a public surface for a test's benefit.
  If that turns out to cost more than the test buys, say so and record the limitation instead: this
  block is not worth a worse tree.
