"""Reading a `Name:` provenance block, for the `script/` entry points that count them.

**Why this file exists.** Milestone 115 put a name's provenance in the header of the thing it names
(`//! Name: ratified 2026-08-04 (calef, milestone 63). Refused `x` (why).`), and `script/names`
derives the table from it. Milestone 276 put the same four-word vocabulary on the metrics dashboard
as a weekly series, which is a second reader of the same blocks. The derivation is shared rather
than copied for milestone 236's reason: a gate and a dashboard that each carry their own copy of a
definition will drift, both will look authoritative, and nothing will fire. `scripts/rust_source.py`
is the same move for the `unsafe` census and the harness count, and its docstring is the longer
argument.

**What is deliberately NOT here, and it is the same line `rust_source.py` draws.** Only the
derivation over text is shared; the file *sourcing* stays in each caller. `script/names` walks the
working tree with `os.listdir` and `os.walk`, because a gate is asked about the tree in front of it.
`script/metrics` enumerates the same four kinds out of a `git ls-tree` at a historical revision and
never checks anything out, because a report is asked about the past and AGENTS.md forbids the
checkout that would be needed. The two enumerations cannot be one function; the two parses can, and
are.

**Form only.** Nothing here can tell whether a reason is true, whether a date is right, or whether a
`recorded` citation leads anywhere. That is `script/names`' own recorded limit and it is not
closeable by a script: a reason is prose, and prose is checked by reading.

Name: provisional, minted by milestone 276's lane on 2026-09-11. It is a shared python module under
`scripts/`, which `script/names` puts out of its own scope, so it carries no `Name:` block of its
own and this paragraph is the record instead. `provenance` alone was considered and reads as the
mechanism rather than the subject, which is the same objection `script/names`' own header records
against `provenance` as a command name; `naming` names the whole topic, including the conventions in
notes/naming.md that this file knows nothing about. calef names modules, and has not ratified it.
"""

import re

# The four words, in the order a reader meets them in `script/names`' own report. `provisional` is a
# claim about INTENT (whoever chose this expects it to change) where the other three are claims about
# the RECORD; §89 added it on 2026-08-16 and the states are orthogonal in principle. The tree spends
# one word on the common case rather than modelling the cross product.
STATUSES = ['ratified', 'recorded', 'provisional', 'unrecorded']

_HEAD = re.compile(r"(ratified|recorded|unrecorded|provisional)\b")
_DATE = re.compile(r"ratified (\d{4}-\d{2}-\d{2})\b")

# `recorded` claims the reasoning is somewhere else, so it has to point: `recorded (milestone 46)`,
# `recorded (notes/naming.md)`. Same shape of check as the date on `ratified`, and the same limit,
# since nothing here follows the citation to see whether it says what the block claims.
CITED = re.compile(r"^recorded \([^()]+\)")

TICKED = re.compile(r"`([^`]+)`")
PAREN = re.compile(r"\([^()]*\)")
# A refused name is a bare identifier: a crate, program or script/ entry that could have existed.
# Anything with a dot, a `::` or a bracket in it is a citation, not a candidate.
NAMEISH = re.compile(r"[A-Za-z][A-Za-z0-9_/-]*$")


def block(text, prefix):
    """The provenance block inside one file's text: the `Name:` line and its continuations, joined.

    A continuation is the next comment line at the same prefix; an EMPTY comment line ends the
    block. That is why the convention puts the block in a paragraph of its own: it is the only
    terminator a header can carry without inventing punctuation nobody would remember.

    Returns None when the file carries no `Name:` line at all, which every caller reports as a
    problem rather than as a status: a surface with no block has not answered, and reading silence
    as `unrecorded` would invent the one claim this record exists to make explicit.
    """
    lines = text.split("\n")
    head = re.compile(rf"^{re.escape(prefix)} ?Name:\s*(.*)$")
    cont = re.compile(rf"^{re.escape(prefix)} (\S.*)$")
    empty = re.compile(rf"^{re.escape(prefix)}\s*$")

    for i, line in enumerate(lines):
        m = head.match(line)
        if not m:
            continue
        parts = [m.group(1).strip()]
        for later in lines[i + 1:]:
            if empty.match(later):
                break
            c = cont.match(later)
            if not c:
                break
            parts.append(c.group(1).strip())
        return " ".join(p for p in parts if p)
    return None


def refused_in(text):
    """Every name recorded as refused, in order, without duplicates.

    Two rules make this parseable without a syntax nobody would remember. **A reason goes in
    parentheses**, so parenthesized spans are removed before the names are read (otherwise
    `capsh(1)`, cited as the Linux tool that refused `capsh`, reads as a refusal of its own). And
    **the refusal clause ends at its sentence**, so the prose that follows it can name other things
    freely: `grant_plan` explains after its list that it is deliberately not named for `swish`, and
    neither `swish` nor the `dwarden` it compares itself to is a refusal.
    """
    out = []
    for chunk in re.split(r"\bRefused\b", text)[1:]:
        flat, prev = chunk, None
        while flat != prev:
            prev = flat
            flat = PAREN.sub(" ", flat)
        clause = re.split(r"\.\s", flat)[0]
        for token in TICKED.findall(clause):
            if NAMEISH.fullmatch(token) and token not in out:
                out.append(token)
    return out


def parse(text):
    """(status, date, [refused names]). Form only: this cannot judge a reason."""
    m = _HEAD.match(text)
    status = m.group(1) if m else None
    date = None
    if status == "ratified":
        d = _DATE.match(text)
        date = d.group(1) if d else None
    return status, date, refused_in(text)


# What a block can be wrong about, as a token rather than a sentence, because the two callers say it
# differently: `script/names --check` prints a paragraph telling a contributor how to fix it, and
# `script/metrics` only needs to know that this surface's status did not parse and so counts toward
# no column. The messages stay at the caller; the judgement is here.
NO_STATUS = 'status'        # the block does not start with one of the four words
NO_DATE = 'date'            # `ratified` without a date is a ratification nobody can place
NO_CITATION = 'citation'    # `recorded` without a citation has not made its own claim


def classify(text):
    """(status, date, refused, problem) for one block's text; `problem` is None when it is well formed.

    The two refinements are the reason this is a function rather than a call to `parse`. A
    ratification carries a date because a ratification nobody can place is prose, and a `recorded`
    carries a citation because the claim IS that the reasoning is somewhere else. A block failing
    either has not said which of the four it is, so it holds no status for anyone to count.
    """
    status, date, refused = parse(text)
    if status is None:
        return None, None, refused, NO_STATUS
    if status == "ratified" and date is None:
        return None, None, refused, NO_DATE
    if status == "recorded" and not CITED.match(text):
        return None, None, refused, NO_CITATION
    return status, date, refused, None
