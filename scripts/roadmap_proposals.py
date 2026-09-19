"""Reading an unnumbered roadmap proposal, a record form this tree no longer writes.

**The form is historical as of 2026-09-19** (milestone 434). `design/roadmap/proposals/` existed
from 2026-09-03 to 2026-09-19, holding work a lane had identified and could not number, because
concurrent lanes cannot see each other and two reaching for the same number collide. calef retired
it by removing the constraint instead of routing around it: a lane now writes the numbered block
itself, `NOT-STARTED`, with the number provisional, and the integrator renumbers at merge. Milestone
433 drained all 106 files out of the directory and 434 cut the machinery that read it.

**So why this module is still here, with one caller.** `script/metrics` counts the pile per week
(milestone 276), and that dashboard is a RESTATEMENT: `--backfill` rewrites every week's row by
applying today's definitions to old commits, reading blobs at revisions nobody has checked out. The
`proposals_unnumbered` column is 74 at 2026W36 and 93 at 2026W38, and both numbers are true about
the tree at those revisions. Delete this parse and the next backfill writes zero into both, which is
a dashboard quietly lying about a fortnight that happened. That rise and fall is the measurement
`design/decisions/140-follow-on-disposition-vocabulary.md` and milestone 433 both rest on, so it is
the last thing to erase.

It stays a module rather than fifty lines folded into `script/metrics` because this paragraph has to
be somewhere a reader meets it: the column is permanently zero going forward, and a column that can
only be zero looks like a bug unless something says why. Milestone 236's rule (a derivation two
scripts each copy will drift while both look authoritative) is what put it here and no longer
applies, since `script/roadmap` stopped calling it.

**What went with the directory.** `promoted_from`, which read a numbered block's claim that it had
been promoted out of the pile, and the ordering check in `script/roadmap` that was its only caller.
Promotion is not an act anybody performs now.

**What is deliberately not here**, the same line `scripts/rust_source.py` and
`scripts/name_provenance.py` draw: only the derivation over a filename and a file's text. The
directory LISTING stays at the caller, because `script/metrics` reads blobs at revisions nobody has
checked out.

Name: provisional, minted by milestone 276's lane on 2026-09-11, and it is a shared python module
under `scripts/`, which `script/names` puts out of its own scope, so this paragraph is the record
rather than a `Name:` block. `proposals` alone would not say which proposals (this tree also has
`**Status: PROPOSED**` decisions in `design/decisions/`, a different record with a different form);
`roadmap_records` would promise the index rows too, which live in two different parses that this
does not touch. calef names modules, and has not ratified it. The name is now a claim about a form
that no longer exists, which is a rename and therefore his too.
"""

import re

DIRECTORY = 'design/roadmap/proposals'

# A proposal filename is a lowercase hyphenated slug carrying NO number. The slug is the readable
# half of a numbered block's name, so nothing new has to be learned, and it collides only if two
# lanes pick the same words, which is visible rather than silent.
_SLUG = re.compile(r"[a-z][a-z0-9-]*\.md")

# The date is what makes the pile measurable. A proposal nobody promotes is the same burial in a new
# place, and age is the only tell a script has.
_STATUS = re.compile(r"\*\*Status: PROPOSED (\d{4}-\d{2}-\d{2})\.\*\*")

_TITLE = re.compile(r"# [^0-9]")
_GATE = re.compile(r"\*\*Gate: ([A-Z0-9, ]+)\.\*\* (\S)")

# What a proposal can be wrong about, as tokens rather than sentences, because the callers say it
# differently: `script/roadmap` prints a paragraph telling a lane how to fix the file, and
# `script/metrics` only needs to know this file is not a proposal it can count.
NOT_MARKDOWN = 'not-markdown'
NOT_A_SLUG = 'not-a-slug'
NO_TITLE = 'title'
NO_STATUS = 'status'
NO_GATE = 'gate'


def filename_problem(filename):
    """Why this directory entry is not a proposal file, or None."""
    if not filename.endswith('.md'):
        return NOT_MARKDOWN
    if not _SLUG.fullmatch(filename):
        return NOT_A_SLUG
    return None


def classify(text):
    """(date, title, problem) for one proposal's text; `problem` is None when it is well formed.

    The three checks are the ones a numbered block also has to pass, in the same order: a title with
    no number in it (an unpromoted proposal has none), a dated `PROPOSED` status, and a gate, because
    a proposal a lane cannot start is worth as little as a milestone a lane cannot start.
    """
    lines = text.split('\n')
    if not lines or not _TITLE.match(lines[0]):
        return None, None, NO_TITLE
    first = next((line for line in lines[1:] if line.strip()), '')
    status = _STATUS.match(first)
    if not status:
        return None, None, NO_STATUS
    paragraphs, current = [], []
    for line in lines:
        if line.strip():
            current.append(line)
        elif current:
            paragraphs.append(current)
            current = []
    if current:
        paragraphs.append(current)
    gate = ' '.join(paragraphs[2]) if len(paragraphs) > 2 else ''
    if not _GATE.match(gate):
        return None, None, NO_GATE
    return status.group(1), lines[0][2:], None
