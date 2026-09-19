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

> **Open decisions live in a file, not in a conversation.**
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

## Slice B, fifteen blocks, resolved 2026-09-19

`224 241 260 271 327 328 330 335 340 341 342 347 350 360 388`, on `milestone/435-slice-b`, with
sections minted in a reserved range of twenty numbers beginning at §177. **Every number here is
provisional until the
merge queue lands it.**

**Five tokens were wrong**, which is a third of the slice and is the answer to the question this
milestone could not ask in advance:

- **224** and **260** wait on a person at a machine, which is `HARDWARE`'s second sense. 224's gate
  had been contradicting the status line directly above it since calef answered it on 2026-09-04.
- **271** confused its output with its gate: the `CR4.PCIDE` ruling is what the milestone delivers,
  and the measurement behind it needs xenon, because icount charges the added gate and credits
  nothing for the removed flush. Milestone 335 already measured exactly that on the riscv64 twin.
- **330** read a provisional name as a blocker, which is the opposite of what `design/naming.md`
  makes a provisional name for. It is now `NONE` and startable.
- **335** carried a second token for a question the tree had answered by practice: board numbers
  live in `bench/<board>-<date>/`.

**Three already had their decision** and gained a citation rather than a file: **241** (calef's own
dated deferral, quoted in the gate), **340** ([§97](../decisions/97-advisory-checks.md), whose
ruleset edit is `DECIDED` and unperformed and whose `BUGS` predicted this block), and **388**
([§154](../decisions/154-the-acronym-test-is-whether-the-phrase-is-spoken.md), which rules four of
that block's five names by name).

**Seven were genuinely owed** and are written up `PROPOSED`: §177 (327), §178 (328), §179 (341),
§180 (342), §181 (347), §182 (350), §183 (360).

**The rate differs sharply between the aged blocks and the promoted ones**, which is a measurement
about milestone 433's drain rather than about these blocks. Of the four numbered under 327, **three
carried a wrong token and one already cited its decision, and none owed a new file.** Of the eleven
numbered 327 or above, **seven owed a decision**, two already had one and two were wrong. The
promotion carried tokens across unchanged and that was right: the proposals it drained were mostly
written by lanes that had met a real fork, while the older blocks had drifted away from gates that
were true when they were set.

**Two premise corrections found on the way**, both recorded in the blocks themselves. xenon has
booted since 2026-09-05, so half of milestone 241's trigger has fired, and milestone 330 cited a
constant, `KERNEL_WRITER_ANCHORS`, that exists nowhere in the tree.

**And one thing the reserved ranges cost, which the integrator has to resolve rather than a lane.**
`script/decisions --check` fails on a gap in the numbering, so a slice minting from §177 upward
while the twenty numbers below it are held by another lane is red on its own branch by
construction. The numbers are
provisional for exactly this reason; closing the gap is a renumber at merge, in the same breath as
every other global name.

## BUGS

- **A written-up fork is still a fork, and this milestone makes calef's queue longer to read before
  it makes it shorter.** Forty-five paragraphs scattered across blocks become some number of files in
  one directory; that is an improvement in findability and not in volume. The honest claim is that a
  queue you can list is cheaper than one you cannot, not that this reduces what is owed.
- **Section numbers are the expensive half.** Each lane holds a reserved range so two cannot collide,
  and every number in them is provisional until the merge queue lands it, which is the standing rule
  for anything global to the tree.
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
