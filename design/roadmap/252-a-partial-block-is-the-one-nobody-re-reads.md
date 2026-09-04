# 252. A `PARTIAL` block claims work is remaining and nobody re-reads it

**Status: NOT-STARTED.** Minted 2026-09-03 by calef, the same evening milestone 247 shipped, after a
`PARTIAL` block was found describing as remaining two things the tree had finished.
*(Number provisional until the merge queue lands it.)*

**Gate: NONE.** Milestone 247 built the machinery; this widens what it covers.

**In brief.** Milestone 247 made every `BUILT` and `REMOVED` block carry a `## Follow-on` section
saying what became of the work it named, and `script/roadmap --check` fails a block that does not.
**`PARTIAL` is not in that set, and it is the status where this failure is most likely.**

Milestone 16 (real hardware + IOMMU-backed driver isolation, RISC-V first) is the worked example and
the reason for this block. It listed three things as remaining. **Two of them were done:** the
on-board test-suite exit is the UART marker plus SBI SRST that every board run is judged by, and the
DTB-driven UART IRQ printed `source 32` on all nine boots of the 2026-09-03 series. The block said
otherwise for weeks, and it was caught by calef asking what was left rather than by anything in the
tree.

**Why `PARTIAL` is the worse case, not the lesser one.** A finished block is written once and closed.
A `PARTIAL` block is a *standing claim about the future* that gets edited as pieces land, and nothing
re-reads what it still asserts. Its prose is also the thing lanes read when deciding what to pick up,
so a stale one does not merely misinform: it offers work that does not exist.

**22 blocks are `PARTIAL` today.** Milestone 16 is one. Nobody knows how many of the other 21 are in
the same state, and that number is the first thing this milestone should produce.

## What "follow-on" means for an unfinished block, which is the design question

For a `BUILT` block the question is *what happened to the work this named*. For a `PARTIAL` block it
is **what is still outstanding, and is that still true**, which is a different sentence and may want a
different section or different words. Milestone 247's seven dispositions may fit as they are: `Done.`
already exists, and it is exactly what milestone 16's first two items needed.

**Resolve that before sweeping**, and say which way it went. Reusing `## Follow-on` unchanged is the
cheaper answer and probably the right one, since a second section name is a second thing to remember;
but if the words do not fit, forcing them would produce dispositions that lie in a new way.

**What the gate can and cannot do is unchanged from 247** and should not be overclaimed: it can check
that the claims are enumerated and that each resolves to something that exists. It cannot check that
a claim is still true. What actually catches staleness is a person writing the dispositions out, which
is what 247's sweep demonstrated when it found three items already built, two of them stale inside a
`## Follow-on` section for a month.

## The proof that this milestone worked

**All 22 `PARTIAL` blocks carry dispositions for their outstanding work, and the gate refuses a
`PARTIAL` block that does not**, demonstrated by removing one and watching it go red.

And the count that makes it worth having done: **how many of the 22 were describing work already
finished.** Milestone 16 was one; report the rest rather than quietly fixing them.

## BUGS

- **It cannot tell a stale claim from a live one**, which is 247's limitation inherited whole. The
  value is that the claims become enumerable and that somebody reads them once.
- **`IN-PROGRESS` and the rest of the status vocabulary are still outside this.** Whether they want
  the same treatment is not answered here, and a status that means "a lane is on it right now" has a
  much shorter staleness window than `PARTIAL`, which can sit for months.
- **A `PARTIAL` block's outstanding work is often the milestone itself**, not a follow-on, so there
  is a real risk of ceremony: writing `**Milestone 74.**` under a heading when the block's own prose
  already said it. If that is what the sweep finds, the honest outcome is to say the section buys
  little for `PARTIAL` blocks and to record that rather than shipping a gate nobody gains from.
