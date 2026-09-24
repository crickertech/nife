# The prose budget

*An appendix to [the register of measures](../register-of-measures.md). The name is provisional.*

calef ratified a prose budget on 2026-09-23 (UTC): 3,000 words of main body per document, enforced
as a ratchet. A document already over may not grow, and one under may not cross. The decision is
[§212 (a prose budget)](../../design/decisions/212-a-prose-budget-for-every-document.md), minted on
2026-09-24; when this panel was written it was still a proposal in pull request #1187. A ratchet is
invisible without a graph, so calef asked for this panel the same day he ratified the cap.

## Two panels

The first chart is the debt: how many words would have to move into appendices for the tree to meet
its own rule. The second is how many documents that work sits in. They are two panels because the
series stack with nothing. They also move independently: splitting one long document cuts the debt
and leaves the count where it was.

## Scope

Both are measured over every `.md` directly under `design/`, `design/decisions/`,
`design/roadmap/`, `notes/` and `briefs/`, plus `AGENTS.md`. Directly under, not recursively, which
is the scope the ruling's own evidence paragraph used. Words are whitespace-separated over the whole
file, so a top-level document's count is its main body.

Every document is counted, including one carrying a marked exception. Exceptions belong to the gate
the ruling asks for, milestone 586 (a prose ratchet in lint), which is not built. A chart that
subtracted exempted documents would hide the debt, and the debt is what the chart is for.

## BUGS

- A word cap rewards moving prose rather than cutting it, as §212's own `BUGS` says. Every file can
  pass while the tree's total grows, and these series would show that as a falling debt. Neither
  chart measures whether anybody reads the words.
- Appendices are not walked. They live in subdirectories such as `notes/benchmarks/` and
  `notes/register-of-measures/`, and the scope is non-recursive, so an appendix over the cap is
  invisible here (found 2026-09-24, when this register was split).
- The numbers include `AGENTS.md`, while the ratified headline figures (569,775 words, 174
  documents, 2026-09-23) were measured without it. `AGENTS.md` alone is 10,967 words, so the two
  reconcile exactly at 7,967 words and one document.
