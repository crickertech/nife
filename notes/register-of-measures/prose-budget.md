# The prose budget

*An appendix to [the register of measures](../register-of-measures.md). The name is provisional.*

## The prose budget

calef ratified a prose budget on 2026-09-23 (UTC): 3,000 words of main body per document, enforced
as a ratchet, so a document already over may not grow and one under may not cross. The decision
section is proposed in pull request #1187 and has no number yet. A ratchet is invisible without a graph, which is why
this panel exists and why calef asked for it the same day he ratified the cap.

The first chart is the debt: how many words would have to move into appendices for the tree to meet
its own rule. The second is how many places that work sits in. Two panels rather than one, because
the series stack with nothing. They also move independently: splitting one long document cuts the
debt and leaves the count where it was.

Both are measured over every `.md` directly under `design/`, `design/decisions/`,
`design/roadmap/`, `notes/` and `briefs/`, plus `AGENTS.md`. Directly under, not recursively, which
is the scope the ruling's own evidence paragraph used. Words are whitespace-separated over the whole
file. Main body and whole file are the same number until a document here has an appendix.

Every document is counted, one carrying a marked exception included. The exception mechanism belongs
to the gate the ruling asks for, which does not exist yet. A chart that subtracted exempted
documents would hide the debt rather than measure it, and the debt is what the chart is for.

**BUGS.** A word cap rewards moving prose rather than cutting it, which the ruling says in its own
`BUGS` section. Every file can pass while the tree's total grows, and these two series will show
that as a falling debt against a rising document count. Neither chart measures whether anybody reads
the words. And the numbers here include `AGENTS.md`, while the ratified headline figures (569,775
words, 174 documents, 2026-09-23) were measured without it. `AGENTS.md` alone is 10,967 words, so
the two reconcile exactly at 7,967 words and one document.
