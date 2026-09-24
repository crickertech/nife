# 586. A prose ratchet in lint

**Status: NOT-STARTED.** Minted by the maintainer on 2026-09-24, at the merge of the two decisions it
enforces: [§212 (a prose budget)](../decisions/212-a-prose-budget-for-every-document.md) and
[§213 (writing standards)](../decisions/213-writing-standards.md). calef ratified both on 2026-09-23.

**Gate: NONE.** Nothing blocks it. Both decisions are DECIDED and describe the check.

## Why one milestone serves both decisions

Each section asks for a gate, and the two gates are one mechanism with different counters. Both read
a document's main body with code fences stripped and blocks split before anything else is counted.
Both keep a per-file baseline that may not rise, the shape of the unsafe-count ratchet already in the
tree. Both let a new document, or one rewritten wholesale, meet the standard outright. And both
accept a marked exception carrying its reason. Building them as two milestones would build the
markdown body reader and the baseline file twice, or sequence one lane behind the other for no gain.

The decisions stay separate because they bound different things. The gate does not need to be.

## What it would be

A check in `script/lint` that measures every markdown document in the tree and compares it with a
committed baseline:

- Words of main body, against §212's 3,000-word cap. Appendices are measured under the same cap.
- Median and longest sentence, against §213's 20 and 40 words.
- Bold spans per 1,000 words, against §213's 4, reported as two counts: bold that opens a line and
  bold inline. They have different fixes.
- An orphan check: an appendix nothing links to fails, since §212's convention manufactures exactly
  that failure if nothing watches.

A document at or under every limit passes. A document over a limit passes only if it did not get
worse against its baseline, or carries a marked exception with a reason.

## The trap it inherits

Sentence splitting must respect block boundaries first. §213 records a naive pass that let a
paragraph's last sentence swallow the next heading. It reported a corpus median of 30 words where the
truth was 20. A gate built on that splitter fails documents that pass.

Markup a script parses, such as the `**Status:` line, counts as bold under §213. That is a marked
exception in §213's `BUGS`, not a carve-out the gate should invent.

## Design note, 2026-09-24: quoted text is exempt from the sentence limits

The sentence-length check must not count text inside quotation marks against the 40-word limit or
the median. A verbatim quote cannot be rewrapped. Splitting it changes what the speaker said, and
the tree's quotation convention (and `script/citations`, for attributed quotes) holds quotes word
for word.

The evidence is `notes/stranger-test/run-5.md`, condensed to §213 on 2026-09-24. Its only sentence
over 40 words is a stranger's worst-thing answer, quoted verbatim in `*"..."*`. Every other
sentence in that file and its seven sibling appendices was brought under the limit. A gate without
this exemption would fail that file for the one sentence it must not change.

The exemption covers the quoted span, not the sentence around it. A lead-in wrapped around a quote
is still prose and is still measured with the quote removed. Bold inside a quote still counts, since
bold is the writer's markup rather than the speaker's.

## Design note, 2026-09-24: an exception marker's count must match the file

A marked exception records the word count calef granted. Nothing compares that number with the
file afterwards, and the 2026-09-24 documentation audit found both marked exceptions past it the
same day they were granted. `design/fatal-risks.md` was granted 4,235 words and measured 4,440, after
a correction to risk 2 landed. `AGENTS.md`'s marker says 6,279 and the file measured 6,292. The gate
should treat the marker's number as the file's baseline: growth past it fails like growth past any
other baseline, and raising it is a new grant, which is calef's.

## What would make it not worth doing

If the baseline file churns on every pull request, the ratchet costs more attention than it saves.
Measuring that churn over the first week is part of the milestone.

## Index row

Decisions 212 (a prose budget) and 213 (writing standards) were ratified with a ratchet as their enforcement and no gate built. This builds one check in `script/lint` for both: main-body word count, median and longest sentence, and line-opening and inline bold density, each held to a per-file baseline that may not rise, plus an orphan check for appendices. One milestone because both gates share the block-aware markdown reader and the baseline mechanism.
