# Appendix to risk 2: The proofs prove trivia, and the real bugs live where Kani cannot reach

*An appendix to [`design/fatal-risks.md`](../fatal-risks.md)'s risk 2. **That entry is the claim of
record**, and it is written so that a reader can decide what to work on next without opening this
file. This one exists to be verified or challenged: it holds the evidence, the dates, the numbers,
the corrections and the refusals behind the verdict, at the length they need rather than the length
the six-pager has. Where a study has its own home in `notes/` this page links it rather than copying
it. **Name provisional** (`design/fatal-risks/` and this file's stem), minted 2026-09-23 by the lane
that split the file; naming is calef's.*

**The claim:** the verification half of DECISIONS §14 (the project's direction: a verified-Rust capability
microkernel that runs real workloads) is real but narrow, and narrow in the direction
that does not matter.

**Evidence today:** 112+ Kani harnesses and `notes/verification.md`. Against: the VisionFive 2's
undelivered-wake bug was found by a bench on three harts, invisible in QEMU, and no proof was
positioned to see it.

**The experiment:** milestone 191 (did the proofs catch the bugs?), against this project's own defect
history, with a second pass over the harnesses asking which prove a property that could plausibly
have been false.

**Verdict of record (see the main entry for the Experiment status field): RUN, 2026-08-30. AMBER, and the red half is structural.** notes/proof-retrospective.md has
the study; PR #589.

- **No Kani harness in this tree has ever caught a defect after the day it was written.** All
  eighteen defects in the corpus were found by something else: a flaky suite, a boot on real
  silicon, a fuzzer, the mutation sweep, loom, a code read, or a CI lint. No red `script/verify` run
  appears anywhere in the record.
- **The cause is one line of `script/verify`'s own header**, verified rather than inferred:
  *"`cargo kani -p <crate>` never compiles the kernel, the user programs, or xtask."* So **64,818
  lines of `kernel/src` are out of reach by construction**, and that is exactly where every
  concurrency, hardware-contract and resource-accounting defect lived. The proofs are not failing to
  catch bugs in the code they cover; they do not cover the code the bugs are in.
- **Why it is amber and not red.** Two real defects were caught *while harnesses were being written*
  (`dtb::be32`'s unchecked `at + 4`, reachable from a corrupt device tree on the boot path;
  `pci::intx_irq`'s pin-0 underflow). That is the survivorship asymmetry this file's rule 1 warned
  about, showing up as evidence rather than as an excuse.
- **The strongest counterfactual is nearly a measurement.** The timer re-arm drift of
  milestone 6 (threads, the context switch, and preemption), 100 Hz configured and ~70 Hz delivered,
  has its property **already proved in this tree**, over already-written code, in
  `crates/timetable`'s `next_after`. The timer does not call it.
- **The numbers were wrong and are now counted:** **145** harnesses, not the roadmap's "112+";
  `script/verify` runs **140**; 31,725 of 206,728 source lines are reachable, though **both sides of
  that ratio count comments**, and `kernel/src` is 40% comment by measurement (25,762 of 64,818
  lines), so any published figure should be in code lines rather than raw ones; 19 `kani::cover!`
  vacuity guards exist, in 4 of 24 harness crates, and a vacuous harness reports `SUCCESSFUL`.
- **The reverse pass found real chaff**, which is what makes the green half credible:
  `capability::subset_is_reflexive` proves `a & !a == 0`, a tautology no plausible implementation
  error breaks, and twelve of the 26 `paging` harnesses are per-ISA restatements of six properties.

**What it changes.** The verification claim should be stated as what it is: proofs over the pure
crates, with the kernel itself largely unverified.

**Milestone 197 (`user/` and `xtask` are out of reach of the prover) closed the second half on
2026-08-31, and half-refuted its own premise.** `user/` was argued to deserve the prover more than
the kernel did, because it holds parsers over bytes this system did not produce. **It mostly does not
any more**: rule 7 and the host-testability discipline already lifted the initrd, ELF, GPT, mDNS,
directory-entry, terminal-escape, shell and glob parsers into crates, **every one already in
`script/verify`'s table**. The prize had been collected under other numbers, which is the tree
working as designed rather than a disappointment.

**It still found a live defect, and found it before any harness ran.** `rmle`'s save buffer was sized
for the document's text while `save` writes rows `\n`-joined, so a full document staged 3,231 bytes
into a 3,200-byte buffer and **panicked the editor on `^S`**. Eight months old and invisible to any
test that does not hit both limits at once. It came out of *writing the property down*, which was the
first time the two constants were compared, and the fix is a `const` assertion beside the buffer
rather than a harness: rung one, because the claim is a relationship between compile-time constants.

**And it measured a trap worth more than the proof.** Three properties were tried and abandoned with
numbers: a sum over 32 symbolic values (no answer in 20 minutes on two solvers), a symbolic index into
a 3.5 KB struct (CBMC out of memory in 3m23s), and any claim downstream of 20 divisions (no answer in
10 minutes). The last is the lesson: **the same harness asserting only a length bound returns in 0.3
seconds, because `--slice-formula` discards the divisions.** A fast harness can be evidence that the
assertion asked nothing, and nothing in this tree currently distinguishes those two cases.

**And the amber moved the same day.** Milestone 193 (put `kernel/src` within reach of the prover) was
minted from this finding and built hours later: two properties proved over `kernel/src/syscall.rs`
with nothing moved into a crate first, **both falsified before being believed** by re-introducing
the real wrapping-multiply defect of milestone 142 (a text display good enough that people use it
instead of a GUI) and watching them turn red. That is the counterfactual this study said the tree
did not have, and it now exists. It cost about 10 seconds of `script/verify`. `kernel/src/arch/`,
`user/` and `xtask` are still out of reach, so the amber stands; what changed is that the reason is
now a worklist rather than a wall.

**And on 2026-09-16 a proof caught a real kernel defect, on an architecture the prover had never
compiled.** Milestone 304 (only ever compiled one architecture, and it was the runner's) found that
`cargo kani -p kernel` selects its `arch/` subtree by `#[cfg(target_arch)]`, which under Kani is the
**host**, so every CI job and the dev Mac had been proving `arch/aarch64/` and nothing of the other
two. The premise was measured rather than assumed: two `assert!(false)` probes placed in
`arch/riscv64/` and `arch/x86_64/` produced *"4 successfully verified harnesses, 0 failures"*,
because neither subtree was compiled.

The first proof ever pointed at `arch/x86_64/irq.rs` went red. `gsi_vector` is a flat
`GSI_VECTOR_BASE.wrapping_add(gsi)`, and `MAX_REDIRECTION_ENTRIES` is documented as the reason it
"cannot silently wrap onto an exception vector" while actually bounding the IO APIC's **entry
count**, not the GSI. A second IO APIC based at global interrupt 200 with the 24 entries every real
part has admits GSI 210, and `gsi_vector(210)` wraps onto **vector 2, the NMI**. Nothing has hit it
because every single-socket PC gives its one IO APIC `gsi_base` 0; a nonzero base is legal ACPI and
exists on multi-socket servers.

**That is a latent defect on hardware this project does not own, found by a model checker, which no
test in this tree could have reached.** It is the class this risk was written to ask about, and it
is the first time this tree has an instance of it. The survivorship caveat above still applies with
full force: the harness caught it *while being written*, like `dtb::be32` and `pci::intx_irq` before
it, so it is evidence that pointing the prover somewhere new pays, not yet evidence that a standing
proof catches regressions. **riscv64 remains unreachable to the prover and nobody here can change
that**: no GitHub image, no Kani cross-target flag, and CBMC needs a goto-binary for its own host.
The fix was deliberately not made in that lane, because it changes a public signature and a
documented policy; it was raised as a proposal with gate `DECISION`, calef chose to route by
redirection index on 2026-09-16, and it was built the same day as milestone 308 (a GSI reaches its
vector by redirection index, not by its own number), whose block is
[roadmap/308-route-gsi-by-index.md](../roadmap/308-route-gsi-by-index.md). **The fix does not add to
this risk's evidence and slightly complicates it**: the harness's `kani::assume(base == 0)` is gone,
so a standing proof now covers the case, but no machine here can execute the path, which
`kernel/src/arch/x86_64/irq.rs`'s module `BUGS` records where a reader meets the feature. A proof
that catches a regression on hardware nobody owns is still the honest shape of what this risk asks
about.
