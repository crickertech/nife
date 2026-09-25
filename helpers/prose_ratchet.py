"""The prose ratchet: §212's word cap and §213's density rules, held to a baseline that only falls.

Milestone 586 (a prose ratchet in lint). calef ratified both decisions on 2026-09-23 with a ratchet
as their enforcement and no gate built, and the budget slipped twice on 2026-09-24 because nothing
checked it: a lane moved facts into notes already over the cap and each note grew. This is the gate.

    python3 helpers/prose_ratchet.py --check          # what script/lint runs
    python3 helpers/prose_ratchet.py --report PATH..  # one document's measures, and why
    python3 helpers/prose_ratchet.py --bank           # lower the baseline to the tree; never raises
    python3 helpers/prose_ratchet.py --init           # write the baseline from scratch (milestone 586 only)

Name: provisional, minted by milestone 586's lane on 2026-09-24. A shared python module under
`helpers/`, which `script/names` puts out of its own scope, so its provenance is this paragraph
rather than a `Name:` block. calef names things; expect this to change. Refused `prose_budget`,
which is §212's half alone and the name `script/metrics` already gives its graph column.

**What it measures, per document.** Four things, each against a ratified limit:

| measure | limit | decision |
|---|---|---|
| words of main body | 3,000 | §212 (a prose budget) |
| median sentence, in words | 20 | §213 (writing standards) rule 1 |
| longest sentence, in words | 40 | §213 rule 2 |
| bold spans per 1,000 words | 4 | §213 rule 3, held as two counts: line-opening and inline |

**The ratchet.** A measure at or under its limit passes. A measure over it passes only if the
committed baseline (`design/prose-baseline.tsv`) records the document as already over on that
measure and the document has not got worse than the baseline says. A document missing from the
baseline, which is every new one, meets the limits outright. Against the merge base as well: a
document that shrank on `main` without anyone banking the shrink may not grow back into the slack,
because §212's rule is "may not grow", not "may not exceed a number written down once".

**Exceptions** are HTML comments in the document, where a reader meets them, and they need a date
and a `Reason:` so that an exception says out loud that it is one (AGENTS.md's ladder):

    <!-- prose-budget: exception. ... Ratified by calef on 2026-09-24 ... Reason: ... -->
    <!-- writing-standards: exception. ... on 2026-09-24 ... Reason: ... -->

`prose-budget` is the syntax `AGENTS.md` and `design/fatal-risks.md` already carried, provisionally,
when this was built, and it is honoured as written. `writing-standards` is its §213 twin and is
provisional. An exception exempts its decision's measures entirely, which is what both decisions
say it does ("passes only if it did not get worse, or carries a marked exception").

**The orphan check.** An appendix is a file under `X/` beside a document `X.md` (§212's default
siting), and it must be linked from `X.md` or from `X/README.md`. `X/README.md` itself is the
directory's provenance page, not an appendix, and is exempt. A thematic appendix directory
(§212's permitted exception, `design/tenets/` today) is named in `THEMATIC_APPENDIX_DIRS` below and
its files must be linked from its own `README.md`. An appendix nothing links to is a lost document.

**What is counted, exactly**, because §213 records a first measurement that was wrong by ten words
of median and a gate built on it would fail documents that pass:

- Fenced code, HTML comments and YAML frontmatter are not prose and are stripped before anything.
- Words are whitespace-separated tokens of what remains, tables included (a table is read).
- Sentences are split on block boundaries first (blank lines, headings, list-item starts, table
  rows, blockquotes), and only then within a block, on `.`, `!` or `?` followed by whitespace and
  anything that can open a sentence, unless the word before the stop is an abbreviation. §213's
  splitter asked for a capital; this tree starts sentences with `calef` and `xenon`, so that one
  ran them together. Headings and table rows are not sentences (§213's own method stripped
  tables). These choices put the corpus lower than §213 measured: 17 words for the median
  document's median sentence, against 20.
- Quoted text is exempt from both sentence limits (586's design note of 2026-09-24): a verbatim
  quote cannot be rewrapped without changing what the speaker said. So a blockquote is skipped and
  a `"..."` span is removed from its sentence before the sentence is measured; the lead-in around
  it is still prose and still counts.
- Inline code is one word in a sentence, since it is verbatim, and all its words in the word count.
  A link counts as its text.
- Bold is `**...**` outside code and outside table rows (§213's table-cell precedent). It opens a
  line when nothing but indentation, a list marker or a blockquote marker precedes it. Bold inside
  a quote still counts: bold is the writer's markup, not the speaker's.
- The median and the bold density are only asked of documents of at least 200 words, the floor
  §213's own per-document statistics used. A median of five sentences or a density over 90 words is
  a coin toss, and a new three-line README with one bold word would otherwise fail at 11 per 1,000.
  The longest-sentence limit applies to every document.

**Bold is ratcheted as counts, not as density, and that is a deliberate departure from §213's
wording ("bold density may not rise").** Density is bold over words, so a lane that condenses a
document without touching its bold RAISES its density, and a density ratchet would fail exactly the
work §212 asks for. That is not hypothetical: five condensation lanes were landing on the day this
was built. So a document over the density limit may not add a line-opening bold or an inline bold
(each count is held separately, since they have different fixes), and cutting words is always free.
Recorded in milestone 586's block for calef, because it reinterprets his ratified words.
"""

import os
import re
import subprocess
import sys

# --- scope -------------------------------------------------------------------------------------
#
# The document scope is `script/metrics`' (milestone 581 (one metrics file per measure)'s prose-budget graph), moved here so the
# gate and the graph cannot drift: every `.md` directly under these five directories, plus
# `AGENTS.md`. Directly under, so `design/roadmap/proposals/`, `design/audit-reports/` and
# `design/journeys/` are out, as the ratified figures had them.
#
# Appendices are added to it, because §212 puts them "under the same cap" and the graph's own
# docstring already said it intended to count them. Before this module the graph's scope could not
# see a single appendix: `notes/benchmarks/` and `design/fatal-risks/` sit one directory down.
PROSE_DIRS = ('design/', 'design/decisions/', 'design/roadmap/', 'notes/', 'briefs/')
PROSE_ROOT_FILES = ('AGENTS.md',)
PROSE_CAP = 3000

# §212's permitted exception to parent-named siting: a thematic directory whose appendices are
# independently citable. Its parent and the reason are in its own README.md, where §212 says the
# exception is marked; this is the machine-readable half of that, and the orphan check needs it
# because a thematic directory cannot be checked by a path rule.
THEMATIC_APPENDIX_DIRS = {'design/tenets/': 'AGENTS.md'}

LIMITS = {'words': 3000, 'median': 20, 'longest': 40, 'bold_per_1000': 4}
SMALL_DOCUMENT = 200  # words; below this, median and density are not asked (see the header)

BASELINE = 'design/prose-baseline.tsv'
COLUMNS = ('words', 'median', 'longest', 'bold_lead', 'bold_inline')
FAMILY = {'words': 'prose-budget', 'median': 'writing-standards', 'longest': 'writing-standards',
          'bold_lead': 'writing-standards', 'bold_inline': 'writing-standards'}


def _direct(path):
    return path in PROSE_ROOT_FILES or (
        '/' in path and path.rsplit('/', 1)[0] + '/' in PROSE_DIRS)


def appendix_parent(path, all_paths):
    """The document `path` is an appendix of, or None. `all_paths` is a set of tracked paths."""
    for d, parent in THEMATIC_APPENDIX_DIRS.items():
        if path.startswith(d):
            return parent
    parts = path.split('/')
    for i in range(len(parts) - 1, 0, -1):
        candidate = '/'.join(parts[:i]) + '.md'
        if candidate in all_paths and (_direct(candidate) or appendix_parent(candidate, all_paths)):
            return candidate
    return None


def documents(all_paths):
    """Every markdown path the prose rules apply to: the graph's scope plus appendices."""
    paths = set(all_paths)
    out = []
    for p in sorted(paths):
        if not p.endswith('.md') or p.startswith(('vendor/', 'target/')):
            continue
        if _direct(p) or appendix_parent(p, paths):
            out.append(p)
    return out


# --- reading a document ------------------------------------------------------------------------

FENCE = re.compile(r'^\s*(```|~~~)')
COMMENT = re.compile(r'<!--.*?-->', re.S)
FRONTMATTER = re.compile(r'\A---\n.*?\n---\n', re.S)
HEADING = re.compile(r'^\s{0,3}#{1,6}(\s|$)')
LIST_ITEM = re.compile(r'^\s*([-*+]|\d+[.)])\s+')
QUOTE_LINE = re.compile(r'^\s*>')
TABLE_ROW = re.compile(r'^\s*\|')
CODE_SPAN = re.compile(r'(`+)(.+?)\1', re.S)
LINK = re.compile(r'!?\[([^\]]*)\]\([^)]*\)')
BOLD = re.compile(r'\*\*(?=\S)(.+?)(?<=\S)\*\*', re.S)
QUOTED = re.compile(r'[*_]*["“][^"“”]*["”][*_]*')
SENTENCE_END = re.compile(r'[.!?][*_)\]"\'\u201d\u2019]*\s+(?=[A-Za-z0-9`"*\'\u201c(\[_\u00a7])')


# Tables a script writes, keyed by the heading that anchors them, and skipped like fenced code.
# `script/decisions` regenerates the table under `## The decisions` in design/decisions/README.md
# from every section's frontmatter, one row per section, and anchors on that heading (its
# TABLE_HEADING) rather than on markers. Counting it made every new section a prose-budget failure
# on a document nobody had edited: the row is the only change, and no lane can cut words to pay
# for it without cutting someone else's prose. Found 2026-09-25 (UTC), the day the ratchet landed,
# when #1273 and #1278 each minted a section and each failed on README.md by one row's words.
GENERATED_TABLES = {'## The decisions'}



def prose_lines(text):
    """(line, in_code) for the text with frontmatter and comments removed. Code lines are dropped."""
    text = FRONTMATTER.sub('', text)
    # Keep the line count stable across a multi-line comment, so nothing downstream miscounts.
    text = COMMENT.sub(lambda m: '\n' * m.group(0).count('\n'), text)
    fence = None
    generated = False
    for line in text.split('\n'):
        if line.strip() in GENERATED_TABLES:
            generated = True
            yield line
            continue
        if generated:
            if not line.strip() or TABLE_ROW.match(line):
                continue
            generated = False
        m = FENCE.match(line)
        if fence:
            if m and m.group(1) == fence:
                fence = None
            continue
        if m:
            fence = m.group(1)
            continue
        yield line


def _code_to_word(s):
    # An inline code span is verbatim, like a quote: it cannot be rewrapped, so in a sentence it is
    # one word, whatever it holds. `Code` is capitalised so a sentence that opens with code still
    # starts a new sentence, which is what the backtick in §213's splitter was for.
    return CODE_SPAN.sub('Code', s)


# Lowercase after a full stop is a sentence start in this tree, because the architect's name is
# `calef` and the boards are `radon`, `argon` and `xenon`. §213's splitter asked for a capital and
# so ran "... has the experiment happened. calef ratified ..." together into one 56-word sentence.
# Any start is accepted unless the word before the stop is an abbreviation.
ABBREVIATIONS = {'e.g.', 'i.e.', 'etc.', 'vs.', 'cf.', 'al.', 'approx.', 'no.', 'fig.', 'ch.',
                 'p.', 'pp.', 'resp.', 'viz.', 'ca.'}


def blocks(lines):
    """Prose blocks, split on block boundaries BEFORE any sentence is split (§213's trap)."""
    cur = []
    for line in lines:
        if not line.strip() or HEADING.match(line) or TABLE_ROW.match(line) or QUOTE_LINE.match(line):
            if cur:
                yield ' '.join(cur)
            cur = []
            continue  # headings, tables and quotes are not sentences
        if LIST_ITEM.match(line):
            if cur:
                yield ' '.join(cur)
            cur = [LIST_ITEM.sub('', line, count=1).strip()]
            continue
        cur.append(line.strip())
    if cur:
        yield ' '.join(cur)


def sentences(block):
    s = LINK.sub(r'\1', _code_to_word(block))
    # Quotes come out BEFORE the split, because a verbatim quote may hold sentence ends of its own
    # ("... It is more honest ...") and splitting inside it would measure half a quote as prose. A
    # quote that closed its sentence leaves the stop behind so the lead-in still ends there.
    s = QUOTED.sub(lambda m: ' \0' + ('. ' if re.search(r'[.!?]\W*$', m.group(0)) else ' '), s)
    out, start = [], 0
    for m in SENTENCE_END.finditer(s):
        before = s[:m.start() + 1].split()
        if before and before[-1].lower().lstrip('(') in ABBREVIATIONS:
            continue
        out.append(s[start:m.end()])
        start = m.end()
    out.append(s[start:])
    lengths = []
    for sent in out:
        n = sum(1 for w in sent.split() if not w.startswith('\0'))  # the lead-in counts
        if n:
            lengths.append(n)
    return lengths


def bold_counts(lines):
    """(line-opening, inline) bold spans. A span may wrap onto the next line of its paragraph, which
    is how a bold lead-in sentence is usually written, so paragraphs are read whole: a line-by-line
    count missed every wrapped span, and wrapped lead-ins are the commonest bold in this tree."""
    lead = inline = 0
    para = []

    def flush():
        nonlocal lead, inline
        if not para:
            return
        text = '\n'.join(para)
        starts = []
        pos = 0
        for line in para:
            m = re.match(r'^\s*(>\s*)*([-*+]\s+|\d+[.)]\s+)?', line)
            starts.append(pos + m.end())
            pos += len(line) + 1
        for span in BOLD.finditer(text):
            if span.start() in starts:
                lead += 1
            else:
                inline += 1
        para.clear()

    for line in lines:
        if not line.strip() or TABLE_ROW.match(line) or HEADING.match(line):
            flush()
            continue
        if LIST_ITEM.match(line) or QUOTE_LINE.match(line):
            flush()
        para.append(CODE_SPAN.sub('code', line))
    flush()
    return lead, inline


def exceptions(text):
    """{family: problem-or-None} for every exception marker in the text."""
    found = {}
    # A marker inside code is being quoted, not asserted (milestone 586's own block spells one).
    text = CODE_SPAN.sub('', re.sub(r'^\s*(```|~~~).*?^\s*\1', '', text, flags=re.S | re.M))
    for m in re.finditer(r'<!--\s*(prose-budget|writing-standards):\s*exception\.(.*?)-->', text, re.S):
        body = m.group(2)
        problem = None
        if not re.search(r'\d{4}-\d{2}-\d{2}', body):
            problem = 'names no date'
        elif 'Reason:' not in body:
            problem = 'gives no `Reason:`'
        found[m.group(1)] = problem
    return found


def granted_words(text):
    """The word count a `prose-budget` exception marker records, or None.

    Milestone 586's design note: the marker's number is the grant, and a file past it has grown
    without a grant. It is the first number before `words` in the marker, compared with the whole
    file's `wc -w`, marker included, because that is how every marker in the tree was measured.
    """
    text = CODE_SPAN.sub('', re.sub(r'^\s*(```|~~~).*?^\s*\1', '', text, flags=re.S | re.M))
    m = re.search(r'<!--\s*prose-budget:\s*exception\.(.*?)-->', text, re.S)
    n = re.search(r'([\d,]+)\s+words\b', m.group(1)) if m else None
    return int(n.group(1).replace(',', '')) if n else None


def median(xs):
    xs = sorted(xs)
    n = len(xs)
    if not n:
        return 0
    return xs[n // 2] if n % 2 else (xs[n // 2 - 1] + xs[n // 2]) / 2


def measure(text):
    lines = list(prose_lines(text))
    words = sum(len(line.split()) for line in lines)
    lengths = [n for b in blocks(lines) for n in sentences(b)]
    lead, inline = bold_counts(lines)
    return {'words': words, 'median': median(lengths), 'longest': max(lengths, default=0),
            'bold_lead': lead, 'bold_inline': inline, 'sentences': len(lengths)}


def over(m):
    """The measures of `m` that are over their limit, as the baseline records them."""
    out = {}
    if m['words'] > LIMITS['words']:
        out['words'] = m['words']
    big = m['words'] >= SMALL_DOCUMENT
    if big and m['median'] > LIMITS['median']:
        out['median'] = m['median']
    if m['longest'] > LIMITS['longest']:
        out['longest'] = m['longest']
    if big and (m['bold_lead'] + m['bold_inline']) * 1000 > LIMITS['bold_per_1000'] * m['words']:
        out['bold_lead'] = m['bold_lead']
        out['bold_inline'] = m['bold_inline']
    return out


# --- the baseline ------------------------------------------------------------------------------

HEADER = """\
# The prose ratchet's baseline: milestone 586 (a prose ratchet in lint), for §212 (a prose budget)
# and §213 (writing standards).
# One row per document over at least one limit when it was written. `-` means that measure was
# within its limit, and so may not cross it. A number is a ceiling that may only go DOWN: lower it
# with `python3 helpers/prose_ratchet.py --bank`, which never raises and never adds. A row may be
# removed; a row may not be added and a number may not rise (script/lint compares against the
# merge base). A document that must exceed its row gets a marked exception, not an edit here.
# Limits: words 3000, median sentence 20, longest sentence 40, bold 4 per 1,000 words (held as the
# two counts). Median and bold are not asked of documents under 200 words.
"""


def fmt(v):
    if v is None:
        return '-'
    return str(int(v)) if float(v).is_integer() else f'{v:.1f}'


def read_baseline(text):
    rows = {}
    for line in text.splitlines():
        if not line.strip() or line.startswith('#'):
            continue
        cells = line.split('\t')
        if cells[0] == 'path':
            continue
        rows[cells[0]] = {c: (None if v == '-' else float(v)) for c, v in zip(COLUMNS, cells[1:])}
    return rows


def write_baseline(rows):
    out = [HEADER, 'path\t' + '\t'.join(COLUMNS) + '\n']
    for path in sorted(rows):
        out.append(path + '\t' + '\t'.join(fmt(rows[path].get(c)) for c in COLUMNS) + '\n')
    with open(BASELINE, 'w') as f:
        f.write(''.join(out))


# --- git ---------------------------------------------------------------------------------------

def git(*args):
    r = subprocess.run(('git',) + args, capture_output=True, text=True)
    return r.stdout if r.returncode == 0 else None


def tracked():
    # Untracked files too (not ignored ones): a lane's new note is judged before its first commit.
    files = git('ls-files', '-z', '--cached', '--others', '--exclude-standard') or ''
    return {f for f in files.split('\0') if f and os.path.exists(f)}


BASE_OVERRIDE = None  # `--base REV`, for falsifying the merge-base half against a chosen commit


def merge_base():
    if BASE_OVERRIDE:
        return (git('rev-parse', BASE_OVERRIDE) or '').strip() or None
    base = (git('merge-base', 'HEAD', 'origin/main') or '').strip()
    head = (git('rev-parse', 'HEAD') or '').strip()
    return base if base and base != head else None


def at(rev, path):
    return git('show', f'{rev}:{path}')


# --- the checks --------------------------------------------------------------------------------

LINK_TARGET = re.compile(r'\]\(([^)#\s]+)')


def links_from(path):
    try:
        text = open(path).read()
    except OSError:
        return set()
    here = os.path.dirname(path)
    out = set()
    for t in LINK_TARGET.findall(text):
        if '://' in t:
            continue
        target = os.path.normpath(os.path.join(here, t))
        out.add(target)
        out.add(target + '/README.md')  # a link to a directory is a link to its README
    return out


def orphans(paths):
    bad = []
    all_paths = set(paths)
    for p in documents(all_paths):
        parent = appendix_parent(p, all_paths)
        if not parent:
            continue
        thematic = next((d for d in THEMATIC_APPENDIX_DIRS if p.startswith(d)), None)
        if thematic:
            readme = thematic + 'README.md'
            if p == readme:
                sources = {parent}
            else:
                sources = {readme}
        else:
            if p == parent[:-3] + '/README.md':
                # The directory's own README is its provenance page (every documentation directory
                # carries its Name: block there since d449f8442), reached by opening the directory.
                # It is not an appendix and is not held to linking.
                continue
            sources = {parent, parent[:-3] + '/README.md'}
        if not any(p in links_from(s) for s in sources):
            bad.append(f'{p}: an appendix nothing links to. Link it from {" or ".join(sorted(sources))} '
                       f'(§212: an unlinked appendix is a lost document)')
    return bad


NAMES = {'words': 'words of main body', 'median': 'median sentence', 'longest': 'longest sentence',
         'bold_lead': 'line-opening bold spans', 'bold_inline': 'inline bold spans'}
LIMIT_TEXT = {'words': 'the 3,000-word cap (§212)', 'median': 'the 20-word median (§213 rule 1)',
              'longest': 'the 40-word limit (§213 rule 2)',
              'bold_lead': '4 bold per 1,000 words (§213 rule 3)',
              'bold_inline': '4 bold per 1,000 words (§213 rule 3)'}


def check():
    paths = tracked()
    docs = documents(paths)
    base = merge_base()
    baseline = read_baseline(open(BASELINE).read()) if os.path.exists(BASELINE) else {}
    bad = []

    # The baseline may only go down. Rows are compared with the merge base's copy of the file.
    if base:
        old_text = at(base, BASELINE)
        if old_text is not None:
            old = read_baseline(old_text)
            renames = {}
            for line in (git('diff', '--name-status', '-M', base, '--', '*.md') or '').splitlines():
                cells = line.split('\t')
                if cells[0].startswith('R') and len(cells) == 3:
                    renames[cells[2]] = cells[1]
            for path, row in baseline.items():
                prior = old.get(path) or old.get(renames.get(path, ''))
                if prior is None:
                    bad.append(f'{BASELINE}: {path} was added. The baseline only shrinks; a new '
                               f'document meets the limits outright, or carries a marked exception')
                    continue
                for c in COLUMNS:
                    was, now = prior.get(c), row.get(c)
                    if now is not None and (was is None or now > was):
                        bad.append(f'{BASELINE}: {path} raised {NAMES[c]} from {fmt(was)} to '
                                   f'{fmt(now)}. The baseline only goes down; an exception belongs '
                                   f'in the document, marked, with its reason')
    for path in baseline:
        if path not in paths:
            bad.append(f'{BASELINE}: {path} is not in the tree. Remove its row (or move it, if the '
                       f'document was renamed)')

    changed = set()
    if base:
        changed = set((git('diff', '--name-only', base, '--', '*.md') or '').split())

    excused = 0
    held = 0
    for path in docs:
        text = open(path).read()
        exc = exceptions(text)
        for family, problem in exc.items():
            if problem:
                bad.append(f'{path}: its {family} exception {problem}. An exception has to say '
                           f'when it was granted and why, or it reads as a design')
        grant = granted_words(text)
        if grant is not None and len(text.split()) > grant:
            bad.append(f'{path}: {len(text.split()):,} words (wc -w) against the {grant:,} its '
                       f'prose-budget exception grants. Cut it back; raising the grant is calef\'s')
        m = measure(text)
        now_over = over(m)
        if not now_over:
            continue
        row = baseline.get(path, {})
        # The merge base's measures, for a document this branch changed: the tight half of the
        # ratchet, which closes the slack an unbanked shrink leaves in the baseline.
        was = None
        if path in changed:
            old_text = at(base, path)
            was = measure(old_text) if old_text is not None else None
        doc_held = False
        for c, v in now_over.items():
            if FAMILY[c] in exc:
                excused += 1
                continue
            ceiling = row.get(c)
            if ceiling is None:
                if path not in baseline:
                    why = 'a document not in the baseline meets the limits outright'
                else:
                    why = ('its baseline row has no ceiling for this measure (it was within the '
                           'limit, or excused by an exception, when the row was written), so it '
                           'may not cross')
                bad.append(f'{path}: {NAMES[c]} is {fmt(v)}, over {LIMIT_TEXT[c]}; {why}')
                continue
            if v > ceiling:
                bad.append(f'{path}: {NAMES[c]} is {fmt(v)}, over its baseline of {fmt(ceiling)} and '
                           f'{LIMIT_TEXT[c]}. A document already over may not get worse')
                continue
            if was is not None and v > was[c]:
                bad.append(f'{path}: {NAMES[c]} went from {fmt(was[c])} to {fmt(v)} on this branch, '
                           f'over {LIMIT_TEXT[c]}. The baseline ({fmt(ceiling)}) has room only '
                           f'because a shrink was never banked, and a document over a limit may not '
                           f'get worse')
                continue
            doc_held = True
        held += doc_held

    bad += orphans(paths)
    return docs, held, excused, bad


def bank(init=False):
    docs = documents(tracked())
    old = {} if init or not os.path.exists(BASELINE) else read_baseline(open(BASELINE).read())
    rows = {}
    for path in docs:
        text = open(path).read()
        exc = exceptions(text)
        o = {c: v for c, v in over(measure(text)).items() if FAMILY[c] not in exc}
        if not o:
            continue
        if init:
            rows[path] = o
            continue
        if path not in old:
            continue  # never adds
        prior = old[path]
        kept = {}
        for c, v in o.items():
            if prior.get(c) is not None:
                kept[c] = min(v, prior[c])
        if kept:
            rows[path] = kept
    write_baseline(rows)
    return rows


def selftest():
    """The reader's traps, each as a document that a wrong reader would measure wrongly."""
    long = ' '.join(['word'] * 30)
    cases = [
        # §213's recorded trap: a heading and an unpunctuated bullet must not join the sentence
        # before them. A naive splitter reads this as one 64-word sentence.
        ('block boundaries', f'{long} ends here\n## A heading of five\n- {long} bullet\n- next',
         lambda m: m['longest'] <= 33),
        # A verbatim quote is exempt and may hold its own sentence ends; the lead-in still counts.
        ('quotes exempt', f'He said: *"{long}. {long} {long}."* And that was all.',
         lambda m: m['longest'] <= 4),
        ('inline code is one word', f'Run `{long}` now.', lambda m: m['longest'] == 3),
        ('lowercase starts split', f'{long} one. calef ruled {long}.', lambda m: m['longest'] == 32),
        ('abbreviations do not split', 'Use a tool, e.g. the linter, here.', lambda m: m['sentences'] == 1),
        ('code fences are not prose', f'Short.\n```\n{long} {long}\n```\n', lambda m: m['words'] == 1),
        ('comments are not prose', f'Short.\n<!-- {long} -->\n', lambda m: m['words'] == 1),
        ('a generated table is not prose',
         '## The decisions\n\n| # | Status | Decision |\n|---|---|---|\n| 1 | DECIDED | [A](a.md) |\n',
         lambda m: m['words'] == 3),
        # Decision files open with YAML frontmatter since #1195; their status is a field, not prose.
        ('frontmatter is not prose', '---\nstatus: DECIDED\nraised: 2026-09-23\n---\n\n# T\n\nShort.\n',
         lambda m: m['words'] == 3),
        ('line-opening and inline bold', '**Lead.** text **inline** more\n- **Item** x\n> **Q** y',
         lambda m: (m['bold_lead'], m['bold_inline']) == (3, 1)),
        ('a bold span may wrap', '**A lead-in that\nwraps.** Then prose.\n\nMore **wrapped\ninline** text.',
         lambda m: (m['bold_lead'], m['bold_inline']) == (1, 1)),
        ('tables hold no sentences or bold', f'| **{long}** | {long} |\n', lambda m: (
            m['longest'], m['bold_lead'] + m['bold_inline']) == (0, 0)),
    ]
    failed = [name for name, text, ok in cases if not ok(measure(text))]
    grant = '<!-- prose-budget: exception. 1,234 words against a 3,000-word cap. 2026-09-24. Reason: x -->'
    if granted_words(grant) != 1234 or granted_words('`' + grant + '`') is not None:
        failed.append('the marker\'s granted word count')
    exc_ok = exceptions('<!-- prose-budget: exception. 2026-09-24. Reason: x -->')
    exc_bad = exceptions('<!-- writing-standards: exception. because -->')
    if exceptions('Spell it `<!-- prose-budget: exception. x -->` in prose.'):
        failed.append('a quoted marker is not an exception')
    if exc_ok != {'prose-budget': None} or exc_bad.get('writing-standards') is None:
        failed.append('exception markers')
    for name in failed:
        print(f'prose ratchet selftest: {name} is measured wrongly', file=sys.stderr)
    return 1 if failed else 0


def main(argv):
    root = git('rev-parse', '--show-toplevel')
    if root:
        os.chdir(root.strip())
    global BASE_OVERRIDE
    if len(argv) >= 3 and argv[0] == '--check' and argv[1] == '--base':
        BASE_OVERRIDE = argv[2]
        argv = argv[:1]
    if argv and argv[0] == '--selftest':
        return selftest()
    if not argv or argv[0] == '--check':
        docs, held, excused, bad = check()
        if bad:
            print('prose ratchet: the tree breaks §212 or §213 against its baseline:', file=sys.stderr)
            for b in bad:
                print(f'  {b}', file=sys.stderr)
            print('\nFix the document (cut restatement, split a sentence, drop or promote a bold '
                  'lead-in), or mark an exception in it with its date and reason. See milestone '
                  '586\'s block, design/roadmap/586-a-prose-ratchet-in-lint.md.', file=sys.stderr)
            return 1
        print(f'prose ratchet: {len(docs)} documents; {held} over a limit and held to the baseline, '
              f'{excused} measures excused by a marked exception, every appendix linked')
        return 0
    if argv[0] == '--report':
        for path in argv[1:]:
            m = measure(open(path).read())
            print(path, ' '.join(f'{k}={fmt(v)}' for k, v in m.items()),
                  'over:', ','.join(over(m)) or 'none')
        return 0
    if argv[0] in ('--bank', '--init'):
        rows = bank(init=argv[0] == '--init')
        print(f'prose ratchet: {len(rows)} rows written to {BASELINE}')
        return 0
    print(__doc__.split('\n\n')[1], file=sys.stderr)
    return 2


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
