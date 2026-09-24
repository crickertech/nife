# 213. Writing standards: three countable rules, one review rule, and a ratchet

**Status: DECIDED.** Ratified by calef on 2026-09-23 (UTC): a median sentence of 20 words or fewer,
no sentence over 40 words, and 4 or fewer bold spans per 1,000 words. Rule 4 stays a review question.
The ratchet is the enforcement, and two proposed rules stay dropped. Raised by calef the same day,
reading a maintainer session's proposal to cap document length. A maintainer session then measured
the tree and proposed four rules, he replied *"Set them."*, and he then changed the median from 25 to
20 and split rule 3 in two. The section number is provisional; the integrator mints it at merge.

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
994 documents of at least 200 words. Fenced code blocks and table rows were stripped first. Then the
text was split into blocks on blank lines, headings and list-item starts, and sentences were split
within each block on `.!?` followed by whitespace and a capital, backtick, quote or asterisk. That
block-aware step is load-bearing, and the next section says why.

| measure | corpus | documents failing the ratified number |
|---|---|---|
| sentence length, median per document | median document is exactly 20 words | over 25: 11%. the share over 20 was not measured |
| sentence length, p90 per document | median document is 42 words | over 40: 61% |
| bold `**…**` per 1,000 words | median 16.1 | over 4: 100%. over 6: 98%. over 8: 95%. over 12: 77% |
| softeners per 1,000 words, narrow set | median 0.00 | over 0.5: 11%. over 1: 5%. over 2: 1% |

`rather than` appears 8,596 times, 4.2 per 1,000 words.

### The first measurement was wrong, and the error is instructive

A first pass split sentences without respecting block boundaries. It read the median document's
median sentence as 30 words and its p90 as 64, and it reported 83% of documents over a 25-word median
and 97% over a 40-word maximum. Those four figures are inflated and the table above replaces them.

The cause: a heading, and a list item without a terminating period, both end without punctuation a
splitter recognises. So the last sentence of a paragraph swallows the heading after it, and one bullet
runs into the next. This section is its own example: read naively its longest sentence measures 73
words, and block-aware, 33. The bold and softener figures are unaffected, because neither depends on
where a sentence ends.

### Most bold in this tree is a heading that lost its syntax

calef's observation, and it is the more useful half of rule 3:

> "I think we bold entirely too much. We use bold for headings at times, which seems like we should
> just use headings. I think we just may be abusing bold."

Measured over the same corpus with code fences stripped, list markers ignored so that a bolded lead-in
inside a bullet still counts:

| pattern | count |
|---|---|
| all bold spans | 29,804 |
| bold that opens a line, the pseudo-heading | 18,712 (63%) |
| of those, ending in `.` `:` `?` `!`, a full claim as a lead-in | 12,474 |
| of those, the entire line is bold, a heading in all but syntax | 816 |
| genuinely inline bold | 10,510 |
| real markdown headings in the same corpus | 9,024 |

The tree has twice as many bold lead-ins as actual headings. The 29,804 figure is lower than an
earlier count of 33,528 because this pass strips code fences.

### What the corrected numbers say the defect is

Sentence length is close to the standard already. The measurable density defect is bold, at 16.1 per
1,000 words against a target of 4, with every one of the 994 documents failing. Two thirds of it is
the pseudo-heading above. The rest is the structural repetition rule 4 names, and no counter can see
that. So the diagnosis is bold and repetition, not long sentences.

### Why a countable rule is worth writing at all

Em-dashes appear zero times in two million words. The `AGENTS.md` banned-word list has two violations
across the corpus. Both are conventions in `AGENTS.md` and nothing gates either one. A style rule
that can be counted has held this tree essentially perfectly, even uncounted. So the density rules
that can be counted should be counted, rather than written as advice and left to hold on their own.

## The standard

Three rules carry numbers. The fourth is the most valuable and no machine can check it.

1. A document's median sentence is 20 words or fewer. The corrected corpus median is exactly 20, so
   the tree already sits on Amazon's 15-to-20 standard, and the proposal's 25 was a floor set above
   where the tree is. A floor above the tree stops nothing. Rule 1's job is regression, not reform.
2. No sentence over 40 words. This one bites: 61% of documents have a p90 over 40, and the median
   document's p90 is 42. A 40-word sentence has to be re-read. Re-reading is the cost this pair of
   decisions exists to cut.
3. Bold marks a claim, not a clause, and it is two rules rather than one. First, the budget: 4 or
   fewer `**…**` per 1,000 words, against today's median of 16.1 with every document over it. Second,
   and this is where the volume is: a bold span that opens a line is a heading that lost its syntax.
   Promote it to a real heading, or drop the bold and let the sentence carry itself. The 816 whole-line
   bolds are unambiguous and mechanically convertible, so they are the first cut.
4. State a finding once, and never comment on your own finding. No number, and this is the rule that
   would cut the most. `design/fatal-risks.md` is the worked example a reader can go check. A finding
   there appears in a status line, again in its own subsection, again in the running-order table.
   Each appearance carries a sentence telling the reader how to feel about it: "which is the point",
   "and that is what makes it credible", "stated once so it is not re-litigated". The finding with
   its caveat is the content. The commentary on the finding is not, and it is often the longest part.

### The budget has no irreducible floor, and an earlier claim that it did was wrong

A maintainer session claimed a document of nothing but claims cannot go below roughly 6.5 bold spans
per 1,000 words. That is false, and `AGENTS.md` is the counterexample. Its split form measures 6.3 per
1,000, and 34 of its 37 spans open a line. Converting or dropping those puts it at 0.5.

So the budget and the heading fix are the same work. Four per 1,000 is reachable because
pseudo-headings are where the volume is, and not because genuine emphasis has to be sacrificed.

## Two rules refused by measurement

The refusals are the valuable half, so both are recorded with their numbers.

A hedging limit was proposed and dropped. It looked justified at 4.8 softeners per 1,000 words. That
count included modal `may` and `might`, which do real work in specification prose (*a lane may
report in two shapes*). On the narrow softener set the corpus median is 0.00 per 1,000 words, and
only 5% of documents exceed 1. This tree does not have a hedging problem. The gate would never fire.

A limit on `rather than` was proposed and dropped. Its 8,596 uses looked like a tic. Read, they are
mostly load-bearing contrast, and they are this tree's actual idiom (*a fact rather than a plan*).

## Enforcement is a ratchet, not a cliff

Rule 3 fails every document in the tree, so a cliff would be 994 instant failures and a migration
project nobody wants. The gate sits at rung 2 of the `AGENTS.md` ladder, a check that fails loudly,
and it wants a milestone. The milestone number is not minted here; the integrator does that at merge.

- A document's median sentence length, longest sentence and bold density may not rise. That is the
  shape of the unsafe-count ratchet and the icount tripwire already in this tree. It turns a wall
  into a direction, and rule 3 needs that more than the other two do.
- The check reports line-opening and inline bold as two counts. They have different fixes, and a
  single density number hides which one a document has.
- A new document meets the standard outright. So does a document being rewritten wholesale, which is
  how the tree converges without a sweep.
- A marked exception in the document, carrying its reason. The ladder permits an exception and
  requires it to say so out loud, because an unmarked exception reads as a design and the next person
  extends it.
- Rule 4 is a review question, and no check can see it. `AGENTS.md` already refuses to gate what a
  lint cannot distinguish from an observation. This section does not pretend otherwise.
- The gate must split on block boundaries before it splits sentences. Otherwise it fails documents
  that pass, for the reason recorded above.

Also owed, and not done here: `AGENTS.md`'s `## Style` section links to this section as the record,
rather than growing a list of numbers. A lane must not edit `AGENTS.md`, so the maintainer does that
at merge.

## What it costs

Two risks, neither softened.

A sentence-length limit can be satisfied by chopping one clear 45-word sentence into two murky
22-word ones. That is worse prose passing a green check, and the check cannot tell the difference. A
bold limit can push emphasis into italics, capitals or a heading. Italics and capitals are the same
evasion wearing different clothes. A real heading is not, which is the point of rule 3's second half,
and it is the one case where the cheap way out is also the fix.

Both are why rule 4 and a human reviewer remain the real mechanism. The countable rules are a floor,
not the standard itself.

The third cost is the ratchet's own: it never finishes. A monotone check on 994 documents converges
only as documents are rewritten for other reasons, so most of the tree stays over the bold number for
a long time. That is the price of not running a migration.

## BUGS

- The corpus numbers were measured on 2026-09-23 with a regular expression, and the first pass of
  that measurement was wrong in the way described above. The corrected figures are one agent's
  second attempt, not an independent check.
- Rule 3's budget is arithmetic from a target, not evidence about readers. Nothing here shows that 4
  bold spans per 1,000 words is better for a reader than 8. Its second half is better grounded: 816
  whole-line bolds are heading syntax written as emphasis, and that is a defect whatever the budget
  is.
- This document's own `**Status:` line is a line-opening bold, which rule 3 deprecates. It is a
  marked exception: the line is a field that `script/decisions` parses, not emphasis, and every
  section in this directory carries one. Changing the format is a separate decision.
- This section meets its own three numbers, which tests that they are livable in a document carrying
  numbers and citations. It does not test them on a note explaining a mechanism, and that is the
  longer half of the tree.
