# Falsification records: what a harness proves it can catch

Milestone 194, building DECISIONS §134. The convention, the tooling, and what running it on six
real harnesses actually found, which was more than the mechanism was expected to show.

`notes/verification.md` has said the rule since milestone 35:

> **Falsify a property before believing it.** Break the code the harness guards and confirm the harness
> fails. Every milestone 35 property was falsified this way, and one falsification corrected a claim in
> the code (the load-bearing guard was not the one the comment pointed at). A harness that cannot be
> made to fail is not evidence.
>
> -- notes/verification.md

It was rung four, honoured by whoever remembered, and milestone 191 measured the result: 141
harnesses and not one recording what was done to falsify it. This note is the record's home.

## Inductive Validity Cores: checked first, and they are not available

§134 named IVCs as the cheaper family and said the implementing milestone should look before
building anything, because a yes would make most of the section unnecessary. The answer is no, and
the reason is structural rather than a feature Kani has not got round to.

An IVC is the minimal set of model elements a proof actually needed. It falls out of the
**unsatisfiable induction step** of an inductive proof, which is why the technique belongs to
k-induction and IC3 model checkers over Lustre models: JKind, Kind2, Sally. **Kani is a bounded
model checker.** It unrolls loops to a depth, compiles the whole thing to one SAT formula, and hands
it to CaDiCaL. A bounded proof has no induction step, so there is nothing for a core to be minimal
with respect to.

What was actually looked at, on 2026-08-31, against Kani 0.67.0 and the CBMC 6.8.0 it ships:

| looked at | what it gives | why it is not an IVC |
|---|---|---|
| `cargo kani --coverage` | source-based **reachability** coverage: which regions symbolic execution touched | catches some vacuity (an unreachable region), no tautology at all. Kani's own RFC-0011 calls it reachability analysis and not proof-theoretic core extraction |
| `cbmc --slice-formula` | drops assignments unrelated to the property | a solver optimisation that reports nothing to the user |
| `cbmc --cover`, `--symex-coverage-report` | test-suite generation against a coverage criterion | again reachability, and about generating tests rather than explaining a proof |
| anything unsat-core shaped in either tool | nothing. `cbmc --help` has no such flag; `cargo kani --help` has no such flag | |

So §134's option C is the affordable option that exists, and this is it.

## The convention

A `Falsification:` block in the comment run immediately above `#[kani::proof]`, past any other
attributes. At the thing, never in a registry, which is milestone 115's argument reused rather than
a new one: two lanes touching two crates cannot collide, and a computed report cannot drift from the
tree.

```rust
/// **Userspace cannot forge a right.** `from_bits` takes an attacker-controlled syscall
/// register (any u32) and the result holds only defined rights, for every possible input.
/// Falsification: replayable 
#[kani::proof]
fn from_bits_cannot_forge_a_right() {
    let raw: u32 = kani::any();
    assert!(Rights::from_bits(raw).is_subset_of(Rights::ALL));
}
```

Three states, and the unknown one is first-class, for exactly the reason `script/names` insists
`unrecorded` stays one: inventing evidence to fill a row puts a false claim in the record whose only
job is saying what is known.

| state | means | what the sweep does |
|---|---|---|
| `replayable <path>` | a patch exists that turns this harness red (backtick the path in a `///` comment; `clippy::doc_markdown` rejects a bare one) | applies it, runs that one harness, **requires red**, reverts |
| `attested <date>` | a person broke the code and watched it fail; nothing can re-check it | counts it, and it is a worklist entry |
| `unfalsified` | nobody has | counts it, and this is the claim's honest denominator |

A patch lives at `crates/<crate>/falsifications/<module.path>.<harness_fn_name>.patch`
(`crates/capability/falsifications/verification.subset_is_reflexive.patch`). **Backtick the path in
the record.** These harnesses live in `#[cfg(kani)]` modules, which `script/lint`'s kani-shim pass
compiles with `-D warnings`, and `clippy::doc_markdown` rejects a bare dotted path in a `///`
comment as an unmarked item; that pass is what found it. The reporter strips backticks rather than
requiring them, so a `//` comment, which has no such rule, can carry the same form. Each patch
**opens with prose above its first `diff --git` line**, which `git apply` ignores and a reader does not. The prose says
which property the patch expects to break, because a falsification that makes a harness fail for the
wrong reason proves nothing. `script/falsifications --check` requires that header to be non-empty;
it cannot check that it is true, which is the same limit `script/names` records for its own
citations.

## Running it

```
script/falsifications                       the table, by crate, and the ratio
script/falsifications --unfalsified         the worklist, biggest crate first
script/falsifications --check               form only; script/lint runs this
script/falsifications --sweep [crate...]    apply every recorded patch, require red, revert
script/falsifications --affected-since <base>   sweep only what a diff can reach
```

**The gate is presence, never `replayable`.** `script/lint` fails a build only when a harness does
not say which of the three states it is in, for the reason `script/names --check` gates on presence
rather than on `ratified`: a lint demanding the queue be drained would hold every unrelated merge
behind proof work nobody can hurry.

**The sweep is a report, weekly** (`.github/workflows/falsifications.yml`, 06:00 UTC Monday), with
`script/mutation`'s posture deliberately copied. A **survivor** is a harness that stayed green with
its own falsification applied, which is exactly the signal nothing in this tree could previously
detect. A **stale** patch is one that no longer applies, which §134 argues is the mechanism working:
the covered code moved, so the falsification must be redone rather than trusted.

**The per-pull-request half is a gate**, in `verify.yml`, and the split is `script/lint`'s own: a
patch that a commit staled is a defect in that commit, where a weekly survivor is a fact about the
tree that morning. It costs seconds because a falsification runs **one** harness.

### The cost, measured

`script/verify` is ~650 seconds and this milestone adds **nothing to it**: the sweep is a separate
script and a separate workflow, and `script/verify` was not touched.

| what | cost |
|---|---|
| `script/falsifications --check` (in `script/lint`) | 0.15 s, no build |
| `script/falsifications --sweep`, six records, warm | 3.0 s of solving on top of the rebuilds |
| one harness alone, `capability`, warm | 0.35 s |

The sweep's cost is dominated by the **rebuild each patch forces**, so it scales with the number of
distinct crates carrying records rather than with the record count. That is why the weekly workflow
is unsharded, and it is a measurement to re-take rather than a rule.

## What six real records found

The six were chosen against the cases that would embarrass the mechanism rather than a toy: milestone
191 named `capability::subset_is_reflexive` as a tautology and the `paging` harnesses as
per-ISA duplicates, so those are where a falsification convention has to earn its keep.

### The capability crate's central theorem is blind to the predicate underneath it

`derive_never_widens_rights` is the crate's flagship: *"the central theorem, on the real
`CapabilityTable::derive`."* Applying the single most plausible defect in `Rights::is_subset_of`, its
two operands swapped (`other.0 & !self.0` for `self.0 & !other.0`), and running the whole crate:

```
Complete - 10 successfully verified harnesses, 2 failures, 12 total.
Verification failed for - verification::subset_matches_allows
Verification failed for - verification::from_bits_cannot_forge_a_right
```

**Ten of twelve stayed green, including the central theorem.** The reason is worth stating plainly
because it generalises: `derive_never_widens_rights` states its property *through the predicate it is
testing*. It asserts `derived.rights.is_subset_of(src_rights)` while `derive` guards on
`rights.is_subset_of(src.rights)`, so a consistently wrong `is_subset_of` satisfies both sides and
the harness cannot see the break. It is load-bearing against the **guard** being dropped, which is
what its recorded falsification does, and blind to the comparison the guard calls.

That is a real coverage gap in the crate this whole project rests on, and it was invisible to
everything: the harness passes, `kani::cover!` would pass, the tests pass, mutation testing does not
reach `cfg(kani)` code. It took applying a defect and looking at which harnesses noticed.

**Not fixed here, deliberately.** Closing it means rewriting the harness to state the property in
terms that do not call the predicate (comparing raw bits, or asserting against `allows`), and this
lane was measuring the proofs rather than rewriting them. It wants a lane, and the proposal is in
milestone 194's report.

### `subset_is_reflexive` is falsifiable and still adds nothing

It can be turned red: drop the `!` from `is_subset_of` and `a.is_subset_of(a)` fails. So it is not
decorative in the strict sense, and §134's "no plausible implementation error breaks it" is slightly
too strong.

The sharper statement is the one the sweep can support: **its falsification set is a strict subset of
another harness's.** Every mutation of `is_subset_of` that reaches it also reaches
`subset_matches_allows`, and the swapped-operand case above reaches `subset_matches_allows` and not
it. A harness whose red cases are a subset of another's adds no discrimination to the suite. Nothing
here proposes deleting it; the record is what lets somebody make that argument with evidence instead
of taste.

### The same shape, one crate along

`filesystem_proto::a_grandchild_is_bounded_by_the_root` is `attenuate_never_widens` at depth two.
`attenuate` is a single `&`, so the only claim the second harness adds is that AND is associative,
and there is no plausible defect that breaks it while leaving the depth-one harness green. Both go
red on the same one-character patch (`&` becoming `|`). Recorded in the patch prose rather than acted
on.

### The patch path was wrong, and `paging` broke it on first contact

§134's first spelling was `crates/<crate>/falsifications/<harness_fn_name>.patch`, which assumes a
harness function name is unique inside its crate. In `paging` it is not. Six properties are stated
once per ISA, in `aarch64.rs`, `sv39.rs` and `x86_64.rs`, under six shared function names:

```
paging::{aarch64,sv39,x86_64}::verification::distinct_pages_take_distinct_paths
paging::{aarch64,sv39,x86_64}::verification::index_is_always_in_bounds
paging::{aarch64,sv39,x86_64}::verification::the_indices_and_offset_tile_the_address
paging::{aarch64,sv39,x86_64}::verification::the_leaf_keeps_address_and_permissions_apart
paging::{aarch64,sv39,x86_64}::verification::the_two_halves_are_disjoint
paging::{aarch64,sv39,x86_64}::verification::the_user_va_gate_admits_only_the_aligned_low_half
```

Eighteen of that crate's twenty-six harnesses, three to a filename. `cargo kani --harness
index_is_always_in_bounds` cannot separate them either, so the sweep could not have run one of them
if the file had existed.

**calef amended §134 on 2026-08-31**: the module path is always included, with no branch. Refused,
and it is the refusal worth keeping, was *unqualified when unique and module-qualified when not*,
which is a branch keyed on an **unstable** property, since a harness added elsewhere would
retroactively invalidate an existing path. That is the same shape as the two-tier program-naming rule
calef rejected on 2026-08-01, one domain over.

**The amendment buys more than uniqueness, and this is why it was the right answer rather than the
adequate one.** The qualified path is Kani's own fully qualified harness name with the separators
changed, so the sweep now filters with `--harness <qualified> --exact` instead of a substring match
on a bare function name. Under the old filter, `paging`'s three `index_is_always_in_bounds` harnesses
would all have run, and "one of them went red" would have proved nothing about the one being
falsified. The path and the filter are one fact written twice, which is the shape of a convention
that cannot drift.

`script/falsifications --check` keeps a collision check, but it now guards **this script's own
module-path tracking** rather than the tree: two harnesses cannot share a qualified name in Rust, so
if two ever compute the same patch path, the brace counting that derives module paths is wrong and
the sweep would prove the wrong harness.

## BUGS

- **Nothing forces the ratio upward.** 6 of 141 today. Every remaining harness may sit at
  `unfalsified` for ever while `script/lint` stays green. That is the honest cost of making the
  convention shippable against an existing tree at all, and it is why the number that matters is the
  ratio the reporter prints rather than the gate's exit code.
- **A recorded falsification proves the harness catches *that* defect, not the class.** It is a
  floor, and a low one. `derive_never_widens_rights` above is the worked example of exactly this: it
  now carries a green record and a documented blind spot at the same time, both true.
- **A patch rots against refactors.** §134 argues that is correct rather than harmless, and the
  sweep reports a stale patch as a failure rather than a skip. It is still churn, and a heavily
  refactored crate re-falsifies often.
- **`--sweep` refuses to run on a dirty working tree.** It applies and reverts with `git apply`, and
  a failure mid-run would otherwise leave a deliberate defect in somebody's source. So you cannot
  sweep while you work, which is a real limit and not a preference.
- **`--affected-since` is crate-granular, not harness-granular.** A change anywhere in a harness
  crate's dependency closure re-falsifies every replayable harness in that crate, because nothing in
  this tree knows which lines a harness covers. That is what an IVC would have bought. It fails
  toward doing too much, which for a run measured in seconds is the right direction.
- **Module paths come from brace counting, not from a Rust parser.** A brace inside a string or a
  comment at module scope would miscount and produce a wrong patch path. Nothing in this tree does
  that, and the failure surfaces as a path `--check` reports rather than as a silently wrong sweep,
  but it is a real limit of a 30-line derivation standing in for `syn`.
- **`kernel/src/arch/` carries no harnesses and gains nothing here.** The architecture layer, where
  the VisionFive 2's undelivered-wake defect actually lived, is outside this record entirely, the
  same scope gap §134 names.
- **Vacuity is still unguarded.** §134 recommended option A (a lint requiring `kani::cover!`)
  alongside option C, and this milestone built only C. 23 `cover!` sites across 4 of 24 harness
  crates is the current state, from milestone 191. That wants a lane.
