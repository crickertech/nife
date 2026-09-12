"""Reading an unnumbered roadmap proposal, for the `script/` entry points that count them.

**What a proposal is** (milestone 247, calef 2026-09-03). Anybody may add to the roadmap;
prioritising it is a different act. A lane that finds work it is not doing writes
`design/roadmap/proposals/<slug>.md` with no number in it, because the thing concurrent lanes
collide over is the NUMBER and not the authority. The integrator promotes one at merge: give it a
number, `git mv` it up a directory, add the index row.

**Why this file exists.** `script/roadmap` gates the pile and lists it; `script/metrics` counts it
per week (milestone 276). Milestone 236's rule is that a derivation two scripts each carry a copy of
will drift while both keep looking authoritative, and the pile's whole defence against becoming a
graveyard is that a script can see it, so a second definition that quietly disagreed about what
counts is the one failure this record cannot afford.

**What is deliberately not here**, the same line `scripts/rust_source.py` and
`scripts/name_provenance.py` draw: only the derivation over a filename and a file's text. The
directory LISTING stays at the caller, because `script/roadmap` reads the working tree and
`script/metrics` reads blobs at revisions nobody has checked out.

Name: provisional, minted by milestone 276's lane on 2026-09-11, and it is a shared python module
under `scripts/`, which `script/names` puts out of its own scope, so this paragraph is the record
rather than a `Name:` block. `proposals` alone would not say which proposals (this tree also has
`**Status: PROPOSED**` decisions in `design/decisions/`, a different record with a different form);
`roadmap_records` would promise the index rows too, which live in two different parses that this
does not touch. calef names modules, and has not ratified it.
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
