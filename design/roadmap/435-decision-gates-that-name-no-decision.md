# 435. Forty-five milestones are gated on a decision nobody wrote down

**Status: IN-PROGRESS** on `maintainer/drain-the-proposal-pile`, with per-slice lanes branching from
it. Minted 2026-09-19 by calef, from a question he asked about one block that turned out to be true
of forty-five. *(Number provisional until the merge queue lands it.)*

**Gate: NONE.** Reading a block and deciding whether its token is wrong or its decision is owed needs
nobody's permission. Writing a decision up is not the same as answering it, and this milestone does
not answer any of them.

## Where it came from, which is one question about one block

calef ruled milestone 421's gate from `NONE` to `DECISION` on 2026-09-19, then asked: *"Does the
prior one have a numbered decision?"* It did not. Its ask lived in the block's own *What is needed
from calef* section, which is one rung above a chat message and below where AGENTS.md puts an open
decision:

> **Open decisions live in a file, not in a conversation.** A decision waiting on calef that exists
> only in chat scrollback is in exactly the medium milestone 94 was written to abolish, and on
> 2026-08-04 five of them accumulated there in one day while that milestone was being built.
>
> -- AGENTS.md

A roadmap block is not chat scrollback, so this is rung three rather than rung four. It is still not
the file AGENTS.md names, and the measurement says the exception is the rule.

## The measurement

**58 blocks carry a `DECISION` token in their gate. In 45 of them the gate paragraph names no
`design/decisions/` section and no `§`.** The other 13 do, so the convention exists and is followed
about a quarter of the time.

```
52 66 95 102 105 131 142 147 178 180 188 198 205 206 207 224 241 260
271 327 328 330 335 340 341 342 347 350 360 388 391 394 395 397 398
400 403 404 406 407 408 413 415 419 423
```

**Twenty-four of the forty-five are numbered 327 or above**, so they arrived hours earlier from
milestone 433's drain of the proposal pile. Promotion carried each block's gate token across
unchanged, which was right for that pass and means a quarter of this backlog is fresh rather than
aged. It is also the second thing 433 surfaced that it did not cause.

## What this is, and what it is not

**It is not forty-five missing decision files.** A block whose gate says `DECISION` may be wrong
about itself, which is what 421's turned out to be in reverse: it said `NONE` while owing a decision.
Some of the 45 will be tokens to correct rather than decisions to write, and a lane that assumes
otherwise will mint decisions nobody needs.

**It is not an attempt to answer any of them.** Writing up a fork is not deciding it. Every decision
this milestone creates lands `**Status: PROPOSED.**` and waits, which is the state that makes calef's
queue readable instead of scattered.

**And it is not a claim that the blocks are wrong to be gated.** `script/roadmap --ready` correctly
excludes all 58. What is wrong is that for 45 of them a reader cannot find out *why* without reading
the whole block, and then finds only a paragraph addressed to one person.

## What a lane does, per block

1. **Read the block and decide which of three it is.** The token is wrong and should be `NONE`,
   `HARDWARE` or `MILESTONE n`; or a decision is genuinely owed; or the decision already exists and
   the gate paragraph simply does not cite it.
2. **Correct the token** with the reason written where the gate is, the way 421's correction records
   that it carried `NONE` for two days and why that was the failure the vocabulary exists to prevent.
3. **Or write the decision up**, `**Status: PROPOSED.**`, with what is being decided, the options,
   a recommendation and its reason, and what is blocked until it is answered. AGENTS.md's seven
   questions are the standard, and questions 2 to 5 are lookups rather than arguments: what this tree
   already does in the analogous case, what the prior art is, whether the premise is true, and what
   each option costs measured rather than asserted.
4. **Or cite the decision that exists**, in the gate paragraph, where a reader meets the gate.
5. **Cite the new section from the block's gate paragraph** either way, so the two records agree.

## What the slices found

**Slice c: 391 394 395 397 398 400 403 404 406 407 408 413 415 419 423**, resolved 2026-09-19 on
`milestone/435-slice-c`. Fifteen blocks, all of them promoted hours earlier by milestone 433, which
makes this slice the cleanest available measurement of whether that drain carried bad tokens across.

**It did not.** Fourteen of the fifteen owed a decision that nobody had written down, and became
§§197-210 (`PROPOSED`, numbers provisional). **Zero tokens were wrong.** One, milestone 391, was the
third outcome: a decision already existed and the gate did not cite it, and the decision is §149,
which refused this exact question on the ground that the general case must not be settled on the
narrow one's momentum and said to answer it when milestone 269 gives it a consumer. So 391 gets a
citation and deliberately gets no new section.

**Two blocks arrived with a problem already recorded, and both readings held.** Milestone 406's
title is false as written and its narrower half is the work; that lane judged the gate and left the
retitling to calef. Milestone 415 was suspected of carrying a token that had gone too strong, and
the suspicion turns out to be backwards: the item that needed nobody's permission is the one that
landed, so what remains is more purely `DECISION` than when the gate was written. A **different**
argument that the token is too strong does exist, on the block's own reversibility paragraph, and
§208 puts it to calef as its first question rather than a lane deciding it.

**What that rate means for the other two slices, which hold aged blocks rather than fresh ones.**
Fourteen of fifteen is a high owed-decision rate, and the reason is visible in the blocks: a
promoted proposal was written by a lane that hit the fork, so its `DECISION` token was minted by
somebody who had just met the question. An aged block's token is more likely to have been inherited,
which is the condition under which it goes stale, so slice c's rate should be read as a floor for
the promoted quarter rather than as a prediction for the other 21.

## BUGS

- **A written-up fork is still a fork, and this milestone makes calef's queue longer to read before
  it makes it shorter.** Forty-five paragraphs scattered across blocks become some number of files in
  one directory; that is an improvement in findability and not in volume. The honest claim is that a
  queue you can list is cheaper than one you cannot, not that this reduces what is owed.
- **Section numbers are the expensive half.** Each lane holds a reserved range so two cannot collide,
  and every number in them is provisional until the merge queue lands it, which is the standing rule
  for anything global to the tree.
- **The reserved ranges make every slice branch fail `script/decisions --check` on its own**, and
  this was measured rather than predicted: slice c holds §197-216, so with §157-196 living on two
  branches it cannot see, its own gate reports *"gap in the numbering: §157 ... §196"* and exits 1.
  The check has no escape hatch and is right not to have one, since a hole is almost always a
  renumber on merge that nobody meant. **It resolves at merge and only at merge**, which puts two
  duties on the integrator: land the three slices in one merge-queue group so the group build sees a
  contiguous tree, and compact the numbering if a lane did not fill its range, because three ranges
  sized 20 against slices that minted 14, 20 and 20 would leave the same hole in the merged tree.
  A lane that renumbered into another lane's range to make its own branch green would be claiming a
  global name, which is the rule this scheme exists to keep.
- **The three-way judgement is the part no gate can check.** Whether a token is wrong or a decision
  is owed is a reading, and a lane that guesses wrong either mints a decision nobody needs or puts a
  block on the ready list that a lane will stall on. The second failure is worse and lanes are told
  so.
- **Nothing here stops a future block gating on an unwritten decision.** A gate that tried would have
  to read prose for intent, which AGENTS.md already priced at `git grep -w TODO`'s 82% false-positive
  rate. What this milestone buys is one sweep and a convention stated where the gate vocabulary
  lives; keeping it true is a habit.

## Index row

Fifty-eight milestones carry a `DECISION` gate and forty-five of them name no decision anywhere a
reader can open, so the ask exists only as a paragraph inside the block, addressed to one person.
AGENTS.md says open decisions live in a file rather than in a conversation, and a roadmap block is
rung three rather than the rung four of chat scrollback, which is why this went unnoticed. calef
found it by asking whether milestone 421's freshly corrected gate had a decision behind it; it did
not, and §156 was minted on the spot. Twenty-four of the forty-five arrived the same evening from
milestone 433's drain, which carried each proposal's gate token across unchanged. The work is a
reading per block and one of three outcomes: correct a token that is wrong, write up a fork that is
genuinely owed, or cite a decision that already exists.
