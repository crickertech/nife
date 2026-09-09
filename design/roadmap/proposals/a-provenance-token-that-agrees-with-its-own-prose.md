# A provenance token that agrees with its own prose

**Status: PROPOSED 2026-09-05.** Found by milestone 264, whose largest single finding was that a
third of its worklist was this bug rather than missing research.

**Gate: NONE.** It reads the tree.

## The bug it would have caught

`script/names` was built with two states, `ratified` and `unrecorded`, and gained `recorded` and
`provisional` afterwards. Its own header records why, and the reason is the whole point: "nobody in
this tree can say why this is called that" needs research *and* a ruling, while "here is the
argument, nobody ever signed it" needs only the ruling, and a worklist that cannot tell them apart is
a list rather than a plan.

**Nothing swept the blocks written before the third state existed.** On 2026-09-05, **21 of the 60
names reported as `unrecorded` already carried a complete argument, refusals included, and said
"provisional" or "not yet put to calef" in their own prose.** Their leading token was simply the old
word.

So the report over-stated its hardest tier by roughly a third, and it interleaved prepared names with
unprepared ones in the one record whose entire job is telling them apart. A reader of `--unratified`
could not see which twenty-one were a read and which were a research task.

## Why no existing gate could see it

`script/names --check` asserts that a block is present and that its first token parses. The header's
BUGS section is already honest that it "cannot check that the recorded reason is still true" or "that
an `unrecorded` entry is honest rather than lazy". This is the narrow, mechanical corner of that
honest gap: not whether the prose is true, but whether the prose and the token are saying different
things about the same block.

## What it would check

For a block whose token is `unrecorded`, fail if its own text contains `provisional`, `not yet put to
calef`, `not put to calef`, or `Refused`. Each of those is the block asserting that an argument
exists, which is what `provisional` means and what `unrecorded` denies.

**The false positives are enumerable, which is what keeps this on rung two.** A block may legitimately
say "nobody recorded a decision because nobody had to make one", and none of the four phrases appears
in it. The risk runs the other way: a block that argues its name at length without using any of the
four phrases is missed, and that is acceptable, since a gate that catches the common shape is worth
more than one that tries to read prose.

## The general form, which is worth more than the check

**A vocabulary change swept the tool and not the records the tool reads.** That is not specific to
`script/names`: the same shape is available anywhere this tree derives a report from tokens embedded
in headers, which by 2026-09-05 is `script/roadmap`, `script/decisions`, `script/falsifications` and
`script/audits` as well. When a status vocabulary grows a state, the existing entries do not move,
and no gate compares an entry against the vocabulary it was written under.

## BUGS

- **It cannot tell a stale `provisional` from a live one.** A name calef has since ratified in
  conversation and nobody transcribed reads exactly like one he has not seen. That is the same limit
  the parent tool records and is not closeable by a script.
