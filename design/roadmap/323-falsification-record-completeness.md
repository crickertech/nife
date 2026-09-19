# 323. The falsification record is incomplete in five ways, and each was found by a different lane

**Status: NOT-STARTED.** Minted 2026-09-18 by calef, promoting a cluster rather than its members:
five proposals, written by five lanes between 2026-09-03 and 2026-09-17, are all about the same
record. *(Number provisional until the merge queue lands it.)*

**Gate: NONE.** It was `DECISION` on parts 4 and 5 when this block was minted. Both were reviewed
with calef on 2026-09-18 and 2026-09-19, and **both turned out to be already answered by the tree**,
which is the finding at the bottom of this block rather than a footnote to it. A lane can take every
part today.

**Part 4 was decided on 2026-09-18**, and it was a ratification rather than a minting. calef ratified
`Expected to fail:` as the line a record carries, in
[§134](../decisions/134-harness-falsification-record.md)'s own spellings section. This block filed it
as *"what to call a new field"* on the proposal's observation that four patches already did it in
prose; counted on the day, **all 66 falsification records carried the line and 65 spelled it exactly
that way**. The one outlier (`Expected red`, in `crates/nifefs`) was corrected in the same change, so
the convention the gate will read is uniform across all 66.

**Part 5 was decided on 2026-09-19, and the answer was neither of the two offered.** It asked whether
`kernel/falsifications/` is an exception to §134's per-crate rule or a mistake. Both halves of the
premise were false, and both had been closed before this block promoted the proposal:

- **The path was settled on 2026-09-01.** §134 carries a paragraph headed *"Clarified 2026-09-01: the
  path is beside the package, not under `crates/`"*, which names `kernel/falsifications/` as
  conforming. There is no per-crate rule to be an exception to. Counted on 2026-09-19, twelve
  packages carry a `falsifications/` directory (ten under `crates/`, plus `kernel/` and
  `components/`) and every one obeys `<package>/falsifications/<module.path>.<harness_fn>.patch` with
  no special case.
- **The replay was built on 2026-09-16.** The part says six kernel claims have records nothing can
  replay. Milestone 305 added a `KernelTest` class to `script/falsifications` on top of milestone
  210's `cargo xtask test --test <substring>`, and `notes/confinement-claims.md` struck the `BUGS`
  bullet that said the mechanism did not exist. Measured on 2026-09-19: **14 kernel tests carry a
  record and 13 are replayable.** The fourteenth is `unfalsified` on purpose, because a real escape
  there hangs the run instead of failing the test.

**calef widened §134's wording on 2026-09-19** rather than leaving the one thing part 5 did surface,
which was vocabulary rather than design: the section said "harness" throughout while the mechanism
had covered kernel `#[test_case]`s since 305. `script/falsifications` had flagged that gap against
itself and handed it over; §134's *"Harness" covers a kernel test too* section is the answer.

**Why a cluster and not five proposals.** Each was filed by the lane that tripped over it, from a
different direction: milestone 313's security audit, 307's sweep of all 26 confinement rows, 318's
NVMe work, 319's device-tree argument, and 247's original sweep. Read together they are one finding
with five faces: **`script/falsifications` is the instrument this project uses to tell a test that
can fail from a test that cannot, and the instrument has holes.** That is `design/fatal-risks.md`
risk 3's own subject, and risk 3 is the entry whose verdict is still calef's.

Promoting them one at a time would produce five briefs, five lanes and five partial answers to a
question that wants one.

## The five parts, each with the proposal that found it and what it claims

1. **The device-tree parser's four harnesses are unfalsified.** `crates/device_tree_blob` carries `be32_is_total`,
   `be64_is_total`, `be32_reads_big_endian_when_in_bounds` and `align4_rounds_up_to_a_multiple_of_four`,
   and none has a record. Four patches against code that already exists. Found by milestone 319.
2. **The NVMe end-to-end test has none either**, and milestone 318's lane falsified it by hand with
   nowhere to put the evidence. What is owed is a cost judgement, not a decision: a fifteenth kernel
   record lengthens the sweep for every lane, and this one needs an NVMe controller attached. Found by milestone 318's lane.
3. **A record names one architecture, so a portable claim is evidenced on one leg.** Milestone 313's
   finding 5. Two spellings fit the existing convention and either would do. Milestone 313's finding 5.
4. **A record does not say which assertion it expects to fire**, so a patch that turns a test red for
   the wrong reason is indistinguishable from one that works. Milestone 307 swept all 26 rows of
   `notes/confinement-claims.md` asking exactly this. Found by milestone 307.
5. **Prose that still describes the mechanism as it was before milestone 305 built the kernel
   replay.** The part as filed (six kernel claims nothing can replay, found by milestone 247's sweep
   from milestone 210's block) was closed on 2026-09-16; what survives is rot around it, and it is a
   lane's afternoon rather than a design question. Two instances are known and a sweep should look
   for more. The patch
   `components/falsifications/proofs.push_never_writes_past_the_buffer_it_was_given.patch` tells its
   reader that no script will apply it and that the sweep *"walks `crates/` only"*; both were true
   when milestone 197 wrote them and neither is true now, and the record reports `replayable` today.
   Milestone 197's own `BUGS` carries the `crates/`-only claim while its `## Follow-on` two screens
   below already records milestone 212 closing it, which is the shape worth grepping for: a `BUGS`
   entry nobody struck when the work landed. The sweep earns its keep because
   `notes/confinement-claims.md` went a fortnight saying a primitive did not exist that had already
   shipped.

## What it is not

**Not a rewrite of `script/falsifications`.** Parts 1, 2 and 3 are records the tree does not have,
part 4 adds one line the sweep may read, and part 5 is stale prose. Nothing here changes how the
sweep works.

**Not a claim that the instrument is broken.** It works: milestone 202 enumerated 26 confinement
claims with 25 replayable falsifications, and milestone 305 used it to find a confinement test that
could not fail. This is about the edges it does not reach yet, which is why every part was found by
a lane doing something else.

## What the two decisions cost, which is this block's second finding

Both parts filed as calef's were already answered by the tree, and neither lane could have known.

Part 5's proposal was written on **2026-09-03**. The path question it raised had been settled on
**2026-09-01**, two days earlier, in a paragraph its author had no reason to re-read. The replay it
asked for was built on **2026-09-16** by milestone 305, thirteen days later. The proposal was
promoted into this block on **2026-09-18**, two days after that, by a maintainer who did not re-check
its premise against the tree. Part 4 has the same shape with a smaller gap: it reported *"four
patches in the tree already solve this in prose"*, which was a sample of 66.

**A proposal's premise decays while the proposal sits, and promotion is where that is cheapest to
catch.** Filing is the wrong moment, since the lane is looking at one thing and the tree moves
underneath it afterwards. Reading the brief is too late, because by then a lane has been spent. The
promotion step already reads every proposal in the cluster and already costs the maintainer's
attention, so re-running the proposal's own claims there is the smallest addition that would have
caught both of these.

**This is deliberately not a gate**, for the reason §134's own `BUGS` gives about prose: no check can
tell a stale premise from a live one. It is rung three, written where the next promotion happens.

## Index row

`script/falsifications` is how this project distinguishes a test that can come back red from one
that cannot, which `design/fatal-risks.md` risk 3 calls the difference between quality and the
illusion of it. Five lanes each tripped over a different hole in it between 2026-09-03 and
2026-09-17: two sets of harnesses with no record at all, a record that names one architecture for a
portable claim, a record that does not say which assertion it expects, and six kernel claims nothing
could replay. The last two were filed as calef's and both turned out already answered by the tree,
which is this block's second finding: a proposal's premise decays while it sits in the pile.
