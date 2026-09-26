"""The milestone block's fields, read from frontmatter or from the prose form it replaces.

**Why this exists**: milestone 596 (the roadmap blocks get frontmatter too). calef ruled on
2026-09-26 (UTC) that the roadmap follows milestone 582 (a decision's status becomes a field, and
the index becomes generated) onto frontmatter, and the same day that the block carries the
dependency fields of §207 (the roadmap is a graph, and the block says so in fields a script can
walk) as keys, retiring the `**Gate:**` line. Six scripts read a block's status out of a sentence, each with
its own regex. They all read it through here now, in both forms, because three of them
(`script/metrics`, `script/catch-up`, `script/citations --moved`) read blobs from revisions that
predate the switch and will go on reading them for as long as the history is walked. The prose
reader here is therefore permanent, not transitional.

**The key names are provisional.** The ruling took the dependency fields; the names themselves are
still before calef. Every key is spelled once, in `KEY` below, and every reader and the migrator
(`helpers/roadmap_migrate.py`) go through that table, so a ratified rename is a one-line change here
plus rerunning the migrator. `VALUE` does the same for the fixed words a value may take.

**The format is 582's, exactly**: `---`, flat `key: value` lines, `---`. No lists and no nesting; a
list is one comma-separated value. A value may not contain `: ` or ` #`, because GitHub renders
frontmatter as YAML and either would make it something other than a string.

Name: provisional, minted by milestone 596's lane on 2026-09-26. A shared python module under
`helpers/`, which `script/names` puts out of its own scope, so this paragraph is the record. calef
names modules, and has not ratified it.
"""
import re

# ---- the schema, spelled once ------------------------------------------------------------------
# The role on the left is what the code says; the key on the right is what a block says. A ratified
# rename changes the right-hand side only.
KEY = {
    'status': 'status',
    'raised': 'raised',
    'built': 'built',
    'branch': 'branch',
    'promoted_from': 'promoted_from',
    'superseded_by': 'superseded_by',
    'refused_by': 'refused_by',
    'milestone_dependencies': 'milestone_dependencies',
    'decision_dependencies': 'decision_dependencies',
    'machine_requirements': 'machine_requirements',
    'specific_machine': 'specific_machine',
    'needs_person': 'needs_person',
}
# The order the migrator writes them in, which is the order a reader asks the questions: where does
# it stand, since when, what it came from, and what it waits on.
ORDER = list(KEY)
# §207's five, which travel together: a block carries all of them or none (`--unmodelled`).
DEPENDENCIES = ['milestone_dependencies', 'decision_dependencies', 'machine_requirements',
                'specific_machine', 'needs_person']
VALUE = {
    'none': 'none',            # an empty dependency field: §207 writes it out, silence means nothing
    'unwritten': 'unwritten',  # a decision dependency on a fork nobody has written up yet
    'yes': 'yes',
    'no': 'no',
}

# The status vocabulary, as `script/roadmap` defines it; `PROPOSED` belongs to proposals alone.
STATUSES = ["BUILT", "REMOVED", "SUPERSEDED", "PARTIAL", "IN-PROGRESS", "NOT-STARTED", "OPTIONAL",
            "RECORDED", "REFUSED"]
PROPOSED = "PROPOSED"

DATE = re.compile(r"\d{4}-\d{2}-\d{2}")
_LINE = re.compile(r"([a-z_]+): (.+)")
# The prose form, as every reader spelled it before 596. Kept for old revisions.
PROSE_STATUS = re.compile(r"\*\*Status: ([A-Z-]+)\b")
PROSE_BUILT = re.compile(r"^\*\*Built:\*\*\s*(\d{4}-\d{2}-\d{2})\s*$", re.M)


def key(role):
    """The key a block spells for `role`."""
    return KEY[role]


def frontmatter(lines):
    """(fields by ROLE, index of the first body line, [(line number, problem)]), or (None, 0, [])
    when the text does not open with `---`, which is the prose form.

    Fields come back keyed by role, not by spelling, so a caller never sees a key name. A key that is
    not in the schema is a problem and is dropped; so is a key given twice.
    """
    if not lines or lines[0] != "---":
        return None, 0, []
    try:
        end = lines.index("---", 1)
    except ValueError:
        return {}, len(lines), [(1, "the frontmatter is never closed by a second '---'")]
    role_of = {v: k for k, v in KEY.items()}
    fields, problems = {}, []
    for n, line in enumerate(lines[1:end], start=2):
        m = _LINE.fullmatch(line)
        if not m:
            problems.append((n, f"{line[:40]!r} is not '<key>: <value>'. The frontmatter is flat "
                                f"key-value pairs and nothing else: no lists, no nesting"))
            continue
        spelled, value = m.group(1), m.group(2).strip()
        role = role_of.get(spelled)
        if role is None:
            problems.append((n, f"{spelled!r} is not in the schema ({', '.join(KEY.values())}). "
                                f"The schema is calef's and a lane does not extend it"))
        elif role in fields:
            problems.append((n, f"{spelled!r} twice. One fact, one field"))
        elif ": " in value or " #" in value:
            problems.append((n, f"{spelled}'s value carries ': ' or ' #', which YAML would read "
                                f"as something other than a string"))
            fields[role] = value
        else:
            fields[role] = value
    # One blank line after the closing fence, as design/decisions/ writes it, is layout.
    start = end + 1
    while start < len(lines) and not lines[start].strip():
        start += 1
    return fields, start, problems


def tokens(value):
    """A comma-separated value as a list; `none` is the empty list."""
    if value is None or value == VALUE['none']:
        return []
    return [t.strip() for t in value.split(",") if t.strip()]


def render(fields):
    """The frontmatter lines for `fields` (by role), in ORDER."""
    return ["---"] + [f"{KEY[r]}: {fields[r]}" for r in ORDER if fields.get(r) is not None] + ["---"]


def status_and_built(text):
    """(status, built) of a numbered block in either form, or (None, '') if neither parses.

    For the readers that want two facts and no validation: `script/catch-up`, `script/citations
    --moved`, `script/metrics` and `script/journeys`. `script/roadmap --check` is what judges a
    block; these read what it passed, at whatever revision they were asked about.
    """
    lines = text.split("\n")
    fields, start, _ = frontmatter(lines)
    if fields is not None:
        return fields.get('status'), fields.get('built', '') or ''
    first = next((l for l in lines[1:] if l.strip()), "")
    sm = PROSE_STATUS.match(first)
    if not sm:
        return None, ''
    bm = PROSE_BUILT.search(text)
    return sm.group(1), bm.group(1) if bm else ''


def body(text):
    """The lines after the frontmatter, or all of them for the prose form. Line 1 is the H1."""
    lines = text.split("\n")
    fields, start, _ = frontmatter(lines)
    return lines[start:] if fields is not None else lines
