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
design/naming.md that this file knows nothing about. calef names modules, and has not ratified it.
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
# `recorded (design/naming.md)`. Same shape of check as the date on `ratified`, and the same limit,
# since nothing here follows the citation to see whether it says what the block claims.
CITED = re.compile(r"^recorded \([^()]+\)")

TICKED = re.compile(r"`([^`]+)`")
PAREN = re.compile(r"\([^()]*\)")
# A refused name is a bare identifier: a crate, program or script/ entry that could have existed.
# Anything with a dot, a `::` or a bracket in it is a citation, not a candidate.
NAMEISH = re.compile(r"[A-Za-z][A-Za-z0-9_/-]*$")

# ---- what a READER takes for a header, which is wider than what `block()` can parse -------------
#
# The gap between those two is where two of calef's ratifications went missing (milestone 283). A
# file carried `//! **Name: ratified 2026-09-08 ...**` under a stale `//! Name: provisional` block;
# the bold prefix does not match `head` below, so the parse read the proposal and reported
# `provisional`, which is a legitimate answer nothing disputes. Neither defect was visible alone.
# `strays` closes it by asking the other question: what in this file LOOKS like a header and is not
# the one that was read.

# A surface's block prefix says which language's comments to read, not which marker the block must
# wear: a header written `///` in a `//!` file is exactly the mistake this is looking for, so the
# wider set is scanned and the marker is then part of the answer.
COMMENT_MARKERS = {"//!": ("//!", "///", "//"), "#": ("#",)}

# Markdown a header can wear while still reading as one: bold and italic (`**Name:`), a heading
# (`# Name:`), a block quote. **Backticks are deliberately absent.** `` `Name:` `` at the start of a
# comment line is a MENTION of the convention, which the scripts that implement it write constantly,
# and reading a mention as a claim would make this module's own callers fail their own gate.
_MARKUP = re.compile(r"^[\s*_#>]+")

# A line showing the FORM rather than making a claim: `Name: ratified <YYYY-MM-DD> (<who>, <where>)`.
# `script/names`' own header documents the three spellings that way. An angle-bracket placeholder is
# the tree's existing mark for "substitute something here" and no real block has ever carried one, so
# it is the discriminator, and it leaves the next person writing an example an escape they can see.
# It is applied only to STRAYS: the line `block()` actually read is never dropped by it, so a block
# that somehow did contain a placeholder still reports rather than vanishing.
_TEMPLATE = re.compile(r"<[^<>]+>")

# Why a stray header could not be read, as a token rather than a sentence, for the same reason
# `NO_STATUS` and friends are tokens: `script/names` phrases it for a contributor, and the judgement
# is here.
STRAY_MARKUP = 'markup'   # markdown between the comment marker and `Name:`
STRAY_INDENT = 'indent'   # more whitespace than the single space the parse allows
STRAY_MARKER = 'marker'   # a comment marker other than this surface's block prefix
STRAY_SECOND = 'second'   # it parses; it is simply not the first, so nothing reads it


def _head(prefix):
    """The one header spelling this module reads: the prefix, at most one space, then `Name:`.

    One definition, used by `block` to find the header and by `strays` to say why a line is not it.
    Two copies of this pattern is how a gate and the thing it gates stop agreeing.
    """
    return re.compile(rf"^{re.escape(prefix)} ?Name:\s*(.*)$")


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
    head = _head(prefix)
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


def headers(text, prefix):
    """Every `(line number, line)` a reader would take as this file's provenance header. 1-based.

    Wider than `block` on purpose, and the width is the whole point: a comment line whose content,
    after the marker and any leading markdown, begins `Name:`. That is the question a person answers
    by looking at the file, and it is the question the gate had never asked.
    """
    out = []
    markers = COMMENT_MARKERS[prefix]
    for number, line in enumerate(text.split("\n"), 1):
        bare = line.lstrip()
        marker = next((m for m in markers if bare.startswith(m)), None)
        if marker is None:
            continue
        if _MARKUP.sub("", bare[len(marker):]).startswith("Name:"):
            out.append((number, line))
    return out


def stray_reason(line, prefix):
    """Why `block` could not have read this header-shaped line, as one of the `STRAY_*` tokens."""
    if not line.startswith(prefix):
        return STRAY_INDENT if line.lstrip().startswith(prefix) else STRAY_MARKER
    rest = line[len(prefix):]
    if _head(prefix).match(line):
        return STRAY_SECOND
    return STRAY_INDENT if rest.lstrip().startswith("Name:") else STRAY_MARKUP


def strays(text, prefix):
    """Header-shaped lines that are not the one `block` read: `(line number, line, why)`.

    **Empty is the only healthy answer**, and that is the rule this makes a gate rather than a
    convention 205 files happen to follow: one provenance block per file, in the spelling the parse
    reads. A second one is unreachable by construction, since `block` stops at the first, and a
    first one the parse cannot see hands its file's whole record to whatever is below it.

    Template lines are dropped (`_TEMPLATE`); the line `block` read is never a stray against itself.
    """
    lines = text.split("\n")
    head = _head(prefix)
    read = next((i for i, line in enumerate(lines, 1) if head.match(line)), None)
    out = []
    for number, line in headers(text, prefix):
        if number == read or _TEMPLATE.search(line):
            continue
        out.append((number, line, stray_reason(line, prefix)))
    return out


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
