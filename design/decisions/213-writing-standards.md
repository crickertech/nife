# 213. Writing standards: three countable rules, one review rule, and a ratchet

**Status: PROPOSED.** Raised by calef, 2026-09-23 (UTC), reading a maintainer session's proposal to
cap document length. A maintainer session then measured the tree, proposed four rules with numbers,
and calef replied *"Set them."* The numbers below are therefore the maintainer's, set on his
delegation. They are set, not surveyed. He ratifies, amends or refuses them. The section number is
provisional; the integrator mints it at merge.

His words, which are the whole brief:

> "We should also be clear, there is just a lot of long writing. We could be more dense with our
> writing. Amazon also had pretty strict writing standards to help."

## What is being decided

How densely this tree's prose must be written, in numbers a script can count.

The sibling decision, provisional 212 on branch `maintainer/prose-budget`, bounds how much a
document may be: 3,000 words of main body, with depth in capped appendices. This one bounds how
densely those words must be written. The two are separate on purpose, and this section links rather
than absorbs, which is 212's convention demonstrating itself.

## The evidence

Measured 2026-09-23 over every `.md` under `design/`, `design/decisions/`, `design/roadmap/`,
`notes/` and `briefs/`. Corpus: 1,004 documents, 2,057,764 words. Per-document statistics cover the
994 documents of at least 200 words. Fenced code blocks and table rows were stripped before
sentences were counted. Sentences were split on `.!?` followed by whitespace and a capital,
backtick, quote or asterisk.

| measure | corpus | documents failing the proposed number |
|---|---|---|
| sentence length, median per document | median document is 30 words | over 20: 95%. over 25: 83%. over 30: 44% |
| sentence length, p90 per document | median document is 64 words | over 40: 97%. over 50: 86%. over 60: 61% |
| bold `**…**` per 1,000 words | median 16.1 | over 4: 100%. over 6: 98%. over 8: 95%. over 12: 77% |
| softeners per 1,000 words, narrow set | median 0.00 | over 0.5: 11%. over 1: 5%. over 2: 1% |

Whole-corpus figures: 55,622 sentences, mean 37.0 words, and 17,591 sentences (31.6%) over 40 words.
Bold appears 33,528 times, one bolded phrase every 61 words. `rather than` appears 8,596 times, 4.2
per 1,000 words.

Two further measurements are the argument that a mechanical rule works here. Em-dashes appear zero
times in two million words. The `AGENTS.md` banned-word list has two violations across the corpus.
Both are conventions in `AGENTS.md` and nothing gates either one. A style rule that can be counted
has held this tree essentially perfectly, even uncounted. So the density rules that can be counted
should be counted, rather than written as advice and left to hold on their own.

## The standard

Three rules carry numbers. The fourth is the most valuable and no machine can check it.

1. A document's median sentence is 25 words or fewer. Today's median document is 30, and 83% are
   over. Amazon's own standard is 15 to 20 words, so 25 is already a concession to this tree's habit
   of carrying a citation and a caveat inside the sentence that makes the claim.
2. No sentence over 40 words. Today the median document's p90 is 64, and 97% of documents exceed 40
   somewhere. A 40-word sentence has to be re-read. Re-reading is the cost this pair of decisions
   exists to cut.
3. Bold marks a claim, not a clause: 4 or fewer `**…**` per 1,000 words. Today's median is 16.1, and
   every single document in the tree exceeds 4. That is one bolded phrase every 61 words, which is
   emphasis so constant it has stopped marking anything. At 4 per 1,000, a 3,000-word document
   carries about 12 bolded phrases, roughly two a page. That is enough to mark what a reader would
   quote.
4. State a finding once, and never comment on your own finding. No number, and this is the rule that
   would cut the most. `design/fatal-risks.md` is the worked example a reader can go check. A finding
   there appears in a status line, again in its own subsection, again in the running-order table.
   Each appearance carries a sentence telling the reader how to feel about it: "which is the point",
   "and that is what makes it credible", "stated once so it is not re-litigated". The finding with
   its caveat is the content. The commentary on the finding is not, and it is often the longest part.

## Two rules refused by measurement

The refusals are the valuable half, so both are recorded with their numbers.

A hedging limit was proposed and dropped. It looked justified at 4.8 softeners per 1,000 words. That
count included modal `may` and `might`, which do real work in specification prose (*a lane may
report in two shapes*). On the narrow softener set the corpus median is 0.00 per 1,000 words, and
only 5% of documents exceed 1. This tree does not have a hedging problem. The gate would never fire.

A limit on `rather than` was proposed and dropped. Its 8,596 uses looked like a tic. Read, they are
mostly load-bearing contrast, and they are this tree's actual idiom (*a fact rather than a plan*).

## Enforcement is a ratchet, not a cliff

The numbers are ambitious against today's tree deliberately, because a cliff would be 964 instant
failures and a migration project nobody wants. Propose the gate at rung 2 of the `AGENTS.md` ladder,
a check that fails loudly, and give it a milestone. Do not mint the milestone number here; the
integrator does that at merge.

- A document's median sentence length, longest sentence and bold density may not rise. That is the
  shape of the unsafe-count ratchet and the icount tripwire already in this tree. It turns "nothing
  over 40 words" from a wall into a direction.
- A new document meets the standard outright. So does a document being rewritten wholesale, which is
  how the tree converges without a sweep.
- A marked exception in the document, carrying its reason. The ladder permits an exception and
  requires it to say so out loud, because an unmarked exception reads as a design and the next person
  extends it.
- Rule 4 is a review question, and no check can see it. `AGENTS.md` already refuses to gate what a
  lint cannot distinguish from an observation. This proposal does not pretend otherwise.

Also proposed, and not done here: `AGENTS.md`'s `## Style` section links to this section as the
record, rather than growing a list of numbers. A lane must not edit `AGENTS.md`, so the maintainer
does that at merge.

## What it costs

Two risks, neither softened.

A sentence-length limit can be satisfied by chopping one clear 45-word sentence into two murky
22-word ones. That is worse prose passing a green check, and the check cannot tell the difference. A
bold limit can push emphasis into italics, capitals or a heading. That is the same evasion wearing
different clothes.

Both are why rule 4 and a human reviewer remain the real mechanism. The countable rules are a floor,
not the standard itself.

The third cost is the ratchet's own: it never finishes. A monotone check on 994 documents converges
only as documents are rewritten for other reasons, so most of the tree stays over the numbers for a
long time. That is the price of not running a migration.

## What is blocked until this is answered

- The numbers stay unset, so no gate can be written and `AGENTS.md`'s style rules stay uncounted.
- Every new document is written at the tree's current density, which is the habit this decision
  exists to break, and each one raises the corpus the ratchet would later hold.
- Provisional 212's word cap has no companion. A cap on length with no rule on density is an
  incentive to compress by deleting content rather than by writing better.

## BUGS

- The corpus numbers were measured once, by one agent, on 2026-09-23. The sentence splitter is a
  regular expression. It over-counts length wherever a line lacks a terminating
  period, because the text then joins whatever follows it. Headings and bullets both do this. Read
  naively, this section's longest sentence measures 73 words; block-aware, 32.
- Rule 3's threshold is arithmetic from a target, not from evidence about readers. Nothing here shows
  that 4 bolded phrases per 1,000 words is better for a reader than 8.
- This section meets its own three numbers, which tests that they are livable in a document carrying
  numbers and citations. It does not test them on a note explaining a mechanism, and that is the
  longer half of the tree.
