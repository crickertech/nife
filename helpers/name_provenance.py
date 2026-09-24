"""Reading a `Name:` provenance block, for the `script/` entry points that count them.

**Why this file exists.** Milestone 115 put a name's provenance in the header of the thing it names
(`//! Name: ratified 2026-08-04 (calef, milestone 63). Refused `x` (why).`), and `script/names`
derives the table from it. Milestone 276 put the same four-word vocabulary on the metrics dashboard
as a weekly series, which is a second reader of the same blocks. The derivation is shared rather
than copied for milestone 236's reason: a gate and a dashboard that each carry their own copy of a
definition will drift, both will look authoritative, and nothing will fire. `helpers/rust_source.py`
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
`helpers/`, which `script/names` puts out of its own scope, so it carries no `Name:` block of its
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
# Anything with a dot, a `::`, a bracket or a SLASH in it is a citation, not a candidate. The slash
# joined that list on 2026-09-18, when widening `block` surfaced `crates/gpt`, `fixtures/` and
# `script/repeat-under-load` as refusals: those are places a thing was refused, not names anybody
# proposed, and `script/names <name>` is asked about names. A `script/` entry point is recorded
# under its bare command (`board-console`), never under its path, so nothing real is lost.
NAMEISH = re.compile(r"[A-Za-z][A-Za-z0-9_-]*$")

# A line that RECORDS a refusal, as against one that mentions the word: `Refused` followed by a
# backticked candidate name. Used only by `refusals_outside`, which is asked whether a record
# sits where the parse cannot reach it, not whether a word appears.
RECORDS_REFUSAL = re.compile(r"\bRefused\s+`[A-Za-z][A-Za-z0-9_-]*`")

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

    A continuation is the next comment line at the same prefix, **across paragraph breaks**: an empty
    comment line is a break, and the end of the comment run is the terminator.

    **It used to stop at the first empty comment line, and that hid refusals** (DECISIONS §155's
    sibling finding, 2026-09-18). A block written as one paragraph per argument, which is how the
    tree's best ones are written, put every refusal after the first break and therefore out of
    reach: `crates/screen_console` recorded four and `script/names --refused` could see none of
    them. That defeats the one query milestone 115 exists to serve, *has this name been refused
    before*, whose worked example is milestone 63 having already refused `system_builder` for a
    reason nobody could find.

    **The widening was measured before it was made**, because this parse is shared with
    `script/metrics` and a change here moves a dashboard as well as a gate. Across all 165 surfaces
    carrying a block, the wider read changes **no** status, date or well-formedness verdict, and
    makes **91** more refusals visible in `.rs` surfaces alone.

    The cost is that a block now runs to the end of its comment run, so prose after it is read as
    part of it. Four files put a heading after their block and were moved to the tree's own
    convention (139 of 143 already had the block last); `script/lint` fails a `Refused` written
    outside what this reads, which is the remaining shape.

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
                parts.append("\n")   # a paragraph break: kept, because it bounds a clause
                continue
            c = cont.match(later)
            if not c:
                break                 # the comment run ended, and so does the block
            parts.append(c.group(1).strip())
        joined = " ".join(p for p in parts if p)
        # Collapse the runs a kept break leaves, so the newline is exactly the boundary and never
        # stray whitespace inside a sentence.
        return re.sub(r" *\n *", "\n", joined).strip()
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
    **the refusal clause ends at its sentence or at a paragraph break**, so the prose that follows it
    can name other things freely: `grant_plan` explains after its list that it is deliberately not
    named for `swish`, and neither `swish` nor the `dwarden` it compares itself to is a refusal.

    The paragraph half arrived with the 2026-09-18 widening of [`block`]: once a block spans
    paragraphs, a clause that ended only at a sentence ran on into the next argument and swept its
    examples up. A paragraph break is a stronger boundary than a sentence and it is now treated as
    one.

    **What this still gets wrong, measured rather than guessed.** A name mentioned in backticks
    *inside* a refusal's own sentence is read as refused too, so `outlaw`'s block, whose clause runs
    `Refused ...: it collides with ... module also drives `hello`, `flaky` and `worker``, reports
    four refusals where it made one. Eight of the 273 refusals on 2026-09-18 were of that shape.

    **Ending the clause at a colon as well was tried and refused**: it removes all eight, and 26
    real refusals with them, including `pci`, `elf` and `mdr`, because `Refused `x`: why` is a form
    the tree uses constantly. Losing a real refusal is the failure this record exists to prevent;
    reporting a spurious one is noise a reader can see through by opening the block. So the noise
    stays, named here rather than discovered.
    """
    out = []
    for chunk in re.split(r"\bRefused\b", text)[1:]:
        flat, prev = chunk, None
        while flat != prev:
            prev = flat
            flat = PAREN.sub(" ", flat)
        clause = re.split(r"\.\s|\n", flat)[0]
        for token in TICKED.findall(clause):
            if NAMEISH.fullmatch(token) and token not in out:
                out.append(token)
    return out


def refusals_outside(text, prefix):
    """`(line number, line)` for every `Refused` in this file's comments that [`block`] cannot read.

    The sibling of [`strays`], asking the other half of the same question. `strays` finds a line that
    reads as a provenance *header* and was not the one parsed; this finds a *refusal* recorded where
    the parse does not reach, which is the failure milestone 115 exists to prevent: a refused name
    that is in the file, read by a human, and invisible to `script/names --refused`.

    It was worth a gate because the old parse stopped at the first empty comment line, so a block
    written one argument per paragraph, which is how the tree's best ones are written, put every
    refusal out of reach. `crates/screen_console` recorded four and the tool could see none. Nobody
    noticed for as long as the convention had existed, and it surfaced on 2026-09-18 only because a
    maintainer edit made the tree-wide count go DOWN by three.

    [`block`] now reads to the end of the comment run, so the remaining shape is narrow: a refusal
    after the doc comment ends, or in a file whose block is not the last thing in it. Narrow is
    exactly when a gate earns its keep, because nobody will catch it by eye.
    """
    # **This surface's own block marker only**, unlike `strays`. A header wearing the wrong marker is
    # exactly what `strays` is looking for; a refusal is not, and an item's `///` doc discussing a
    # refusal elsewhere in the crate is prose rather than a provenance record. Scanning the wider set
    # reported 35 lines, all of them item docs hundreds of lines below the block.
    read = block(text, prefix) or ""
    out = []
    seen_name = False
    for number, line in enumerate(text.split("\n"), 1):
        s = line.strip()
        if not s.startswith(prefix):
            continue
        body = s[len(prefix):].strip()
        if body.startswith("Name:") or _MARKUP.sub("", body).startswith("Name:"):
            seen_name = True
            continue
        # **The recording form, not the word.** `Refused` also names a return value in this tree
        # (`regions::destroy_outcome` returns `Refused`), and a module doc that mentions it is prose.
        # What records a refusal is the word followed by a backticked candidate name, which is the
        # form `refused_in` reads and the form the convention documents.
        if not seen_name or not RECORDS_REFUSAL.search(body):
            continue
        # A line whose text the parse actually took is fine wherever it sits.
        if body and body[:40] in read:
            continue
        out.append((number, line))
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
