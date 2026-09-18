# 323. The falsification record is incomplete in five ways, and each was found by a different lane

**Status: NOT-STARTED.** Minted 2026-09-18 by calef, promoting a cluster rather than its members:
five proposals, written by five lanes between 2026-09-03 and 2026-09-17, are all about the same
record. *(Number provisional until the merge queue lands it.)*

**Gate: DECISION.** Part 5 only, which is milestone 188's shape rather than a blocked block:
**parts 1 to 4 need nobody and a lane can take them today.** Part 5 asks whether
`kernel/falsifications/` is an exception to
[§134](../decisions/134-harness-falsification-record.md)'s per-crate rule or a mistake, and a sweep
cannot be written against either answer until somebody says which. The token is `DECISION` because
`NONE` stands alone and would claim nothing is owed, which is false for part 5; it is not a claim
that the block cannot start.

**Part 4 was decided on 2026-09-18 and is no longer calef's**, which is recorded here rather than
silently dropped: he ratified `Expected to fail:` as the line a record carries, in
[§134](../decisions/134-harness-falsification-record.md)'s own spellings section. The decision turned
out to be a ratification rather than a minting. This block filed it as *"what to call a new field"*
on the proposal's observation that four patches already did it in prose; counted on the day, **all 66
falsification records carried the line and 65 spelled it exactly that way**. The one outlier
(`Expected red`, in `crates/nifefs`) was corrected in the same change, so the convention the gate will
read is now uniform across all 66.

**Why a cluster and not five proposals.** Each was filed by the lane that tripped over it, from a
different direction: milestone 313's security audit, 307's sweep of all 26 confinement rows, 318's
NVMe work, 319's device-tree argument, and 247's original sweep. Read together they are one finding
with five faces: **`script/falsifications` is the instrument this project uses to tell a test that
can fail from a test that cannot, and the instrument has holes.** That is `design/fatal-risks.md`
risk 3's own subject, and risk 3 is the entry whose verdict is still calef's.

Promoting them one at a time would produce five briefs, five lanes and five partial answers to a
question that wants one.

## The five parts, each with the proposal that found it and what it claims

1. **The device-tree parser's four harnesses are unfalsified.** `crates/dtb` carries `be32_is_total`,
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
5. **Six kernel confinement claims have records nothing can replay**, because `script/falsifications`
   replays through `cargo kani` and these are kernel tests. Found by milestone 247's sweep, from milestone 210's block.

## What it is not

**Not a rewrite of `script/falsifications`.** Four of the five parts are records the tree does not
have; only part 4 changes the format, and that is the part gated on calef.

**Not a claim that the instrument is broken.** It works: milestone 202 enumerated 26 confinement
claims with 25 replayable falsifications, and milestone 305 used it to find a confinement test that
could not fail. This is about the edges it does not reach yet, which is why every part was found by
a lane doing something else.

## Index row

`script/falsifications` is how this project distinguishes a test that can come back red from one
that cannot, which `design/fatal-risks.md` risk 3 calls the difference between quality and the
illusion of it. Five lanes each tripped over a different hole in it between 2026-09-03 and
2026-09-17: two sets of harnesses with no record at all, a record that names one architecture for a
portable claim, a record that does not say which assertion it expects, and six kernel claims nothing
can replay. Three parts need nobody; two are calef's, on a field name and on whether
`kernel/falsifications/` is an exception to §134 or a mistake.
