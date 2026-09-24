# `uefi_loader`: a score measured over a file nothing compiled

An appendix of [notes/mutation-testing.md](../mutation-testing.md), moved there verbatim on 2026-09-24.

## 2026-09-04: `uefi_loader`'s 15% was measuring a file nothing compiles

The section above names `uefi_loader` as one of the two crates that carry the tree's fall from 92.4%,
at **3 caught, 17 missed** in the round-robin sample. That crate is the code that boots xenon, on a
machine where a fault has no console and no debugger, so it went first.

**The number was arithmetic, not a finding, and it is the `system_initializer` result again in
different clothes.** Milestone 244 found the largest crate in the tree scored 0 of 191 because the
host suite could not compile a line of it. This is the same failure one level down, at a **target**
rather than a crate: `uefi_loader`'s `[[bin]]` carries `required-features = ["uefi"]`, so
`cargo build --workspace` and `cargo test` never put `src/main.rs` in the build graph at all.
cargo-mutants does not read the build graph. It edits the file, nothing rebuilds, the tests pass, and
every mutant is recorded MISSED **in "0s build + 0s test"**, which is the tell and is printed in the
output nobody was reading.

The whole-crate run on 2026-09-04, before any change:

| | mutants | caught | missed | unviable | killed |
|---|---|---|---|---|---|
| `uefi_loader`, whole crate | 189 | 32 | 156 | 1 | **17.0%** |
| of it, `src/main.rs` (never compiled on the host) | 154 | 0 | 154 | 0 | 0% |
| of it, `src/handoff.rs` + `src/image.rs` (the pure half) | 35 | 32 | 2 | 1 | **94.1%** |

**So the half the design lifted out in order to be host-testable was at 94% the whole time**, and the
crate-level 15% was 154 mutants in a file the tool was alone in reading. That is worth stating
plainly, because the published rate had `uefi_loader` standing for "a subsystem nobody tests" when
what it actually stood for was a measurement bug.

**The residue is real and is not fixed by excluding it.** `src/main.rs` is 790 lines that call
firmware and leave long mode, and the only thing that proves it is `cargo xtask uefi-boot` under
OVMF, on `script/test`'s own leg. `load`, `say_conflict` and `find_screen` carry logic (66, 28 and 9
mutants) that a host test could reach if it were lifted the way `handoff` and `image` were; whether
that is worth doing is
`design/roadmap/381-the-uefi-loaders-firmware-half-is-proved-by-one-boot.md`.
Excluding the file makes the number honest, not the file proved.

### The two real survivors, and the hole beside them

- **`physical_span`'s `first & !(page_size - 1)`, twice** (`- with +`, `- with /`). Every test here
  started its lowest segment on a page boundary, where that mask is the identity and both mutants
  clear bits the inputs never had set. A segment starting mid-page separates them, and the property
  is load-bearing rather than arithmetic: the firmware is asked for this range with one
  `AllocatePages(AllocateAddress)`, which takes a page number, so a span beginning above the
  segment's first byte asks for memory that starts after the bytes about to be written into it.
  Killed by `a_segment_starting_mid_page_pulls_the_span_down_to_its_page`, verified by applying both
  mutations and watching that named test fail.
- **`parse`'s one mutant is unviable, and that hid the fact that nothing called `parse`.** See the
  next section.

**After: 35 mutants, 34 caught, 0 missed, 1 unviable. 100% of viable.** No equivalents claimed and
nothing deferred, which is a small enough crate that the claim is cheap rather than impressive.

**The gate, because none of the above should need remembering.** `script/lint` now derives from
`cargo metadata` that every target with `required-features` is excluded in `.cargo/mutants.toml`,
which is the milestone 244 gate's shape applied to the question its dependency-graph derivation
cannot see. Exactly one target matches today; the gate exists for the second one.

### The unviable mutant, and the case for leaving it unviable

`uefi_loader::image::parse`'s only mutant is `Ok(Default::default())`, and `elf::Elf` has no
`Default`, so cargo-mutants has said **nothing at all** about that function since it was written.
This is milestone 250's shape exactly, and it came with the classic symptom: **nothing in the tree
called `parse` either**, so there was no test for the tool to have failed.

Milestone 246's repair was to derive `Default` on `Verdict` and turn the hole into a kill. **That is
the wrong move here, and the reason is worth recording rather than the verdict.** `Verdict` is a data
struct whose default is simultaneously the fail-safe value and the dangerous wrong answer. `Elf` is a
**validated token**: its own doc says *"an `Ok(Elf)` has nothing left to check; every later accessor
and `segments` iteration step trusts this pass completely."* A `Default` impl makes an unvalidated
`Elf` constructible by every consumer in the tree, which is rung one of AGENTS.md's ladder run
backwards, to buy one mutant on a two-line `map_err` wrapper in one of them. 250's own BUGS section
asks whether a default is a value the code could plausibly be wrong with; here it is, and the
objection is the cost rather than the meaning.

**What was done instead is what the mutant would have asked for.** Two tests now call `parse`, and
`an_accepted_image_arrives_with_its_segments_and_its_entry` asserts precisely what an
`Ok(Default::default())` would violate: the accepted image comes back **carrying its segments and its
entry**, so a wrapper that returned an empty `Elf` fails on the host rather than booting a machine
into a kernel with no segments. The mutant stays unviable and is now a recorded hole with a test
standing where it would have stood, which is the honest disposition rather than a repair.
