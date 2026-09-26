"""Rewrite prose-form roadmap blocks into the frontmatter form, for milestone 596 (the roadmap blocks
get frontmatter too).

    script/roadmap --migrate                 every block and proposal still in the prose form
    script/roadmap --migrate FILE...         only these (what a lane runs after rebasing)
    script/roadmap --migrate --dry-run ...   say what would change, write nothing

**What it is for.** calef ruled on 2026-09-26 (UTC) that the roadmap follows milestone 582 (a
decision's status becomes a field, and the index becomes generated) onto frontmatter, with the five
dependency fields of §207 (the roadmap is a graph, and the block says so in fields a script can
walk) in place of `**Gate:**`. 583 blocks is too many for a hand edit, and nearly every open pull request
edits its own block, so the switch is a run of this program rather than a diff: it is regenerated
on current `main` instead of rebased, and a lane that crosses it takes its own side of a conflicted
block and runs this on that one file.

**Deterministic and idempotent, which is the whole contract.** The same tree and history give the
same bytes, so the switch and a lane's rerun agree; a block already in frontmatter is left alone.
Every fact comes from the block's own prose, from git, or from the hand-read tables below, and
nothing from the clock except the raise date of a block no commit has seen yet.

**Where each field comes from.**

- `status`, `built`, `branch`: the prose status line, the `**Built:**` line and the backticked
  branch an IN-PROGRESS status names, moved verbatim.
- `raised`: the earliest of three sources, as milestone 582 dated sections. The block's own status
  paragraph where it says when it was minted, raised, filed, written or proposed; the first commit
  that added the file, followed through renames; and for the blocks milestone 76 (split the roadmap)
  cut out of `design/roadmap.md`, or backfilled from `DECISIONS.md`'s first milestone table, the
  first revision of that file that carried the milestone. Clamped to `built`, because prose written
  before 2026-09-13 dates by calef's local day and a Pacific evening lands on the next UTC one.
- `promoted_from`: `helpers/roadmap_proposals.py`'s parse, plus five blocks read by hand.
- `superseded_by`: read by hand. Four of the ten name more than one milestone, and one is answered
  by a decision rather than a milestone, so the value is a list and a `§N` is legal in it.
- `refused_by`: every milestone the REFUSED status paragraph names, which is exactly the set
  `script/roadmap --revisit` has always excluded as the refusal's own origin.
- The five dependency fields: from the gate. `NONE` writes `none` in all five and `no`;
  `MILESTONE N` and `DECISION §N` become entries, bare `DECISION` becomes `unwritten`, and the 34
  `HARDWARE` gates were read by hand into the three machine fields below, because §207 found that
  word hides four different things.

**What happens to the prose.** The status token leaves its paragraph; a date inside it that equals
`built` goes with it, and anything else it held stays as an ordinary sentence. The gate token
leaves its paragraph and the reason after it stays. The `**Built:**` line leaves `## Index row`.
That rewrite is `helpers/roadmap_block.py`'s `without_fields`, the same one `helpers/prose_ratchet.py`
measures a prose-form block through and `script/citations --ratchet` compares a changed line
against, so a migrated block measures what it measured before and cites nothing new.

BUGS

**The hand-read tables are this program's, not the tree's.** They are keyed by milestone number and
read once, on 2026-09-26. A block that gains a `HARDWARE` gate or a SUPERSEDED status on a branch
after that date is refused by name rather than guessed at, and its lane adds a row.

**A raise date read from prose is a date near a word, not a parse of the sentence.** "Minted
2026-08-23" is read correctly and so is "filed 2026-09-24 by the lane", but a sentence that says a
proposal was written on one date about an event on another will give the earlier one. Earlier wins
anyway, which is the direction 582 ruled on.

Name: provisional, minted by milestone 596's lane on 2026-09-26, and it goes away once no branch
in flight predates the switch. calef names modules, and has not ratified it.
"""
import datetime
import difflib
import os
import re
import subprocess
import sys

sys.dont_write_bytecode = True
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import roadmap_block                                                        # noqa: E402
import roadmap_proposals                                                    # noqa: E402

K = roadmap_block.VALUE
NONE, UNWRITTEN, YES, NO = K["none"], K["unwritten"], K["yes"], K["no"]

# ---- read by hand, 2026-09-26 -------------------------------------------------------------------
# The 34 HARDWARE gates, as (machine_requirements, specific_machine, needs_person). A machine
# requirement is a capability any host with it satisfies (radon satisfies "riscv64 silicon"; it is
# not the requirement). A specific machine is named only where one machine is the point, with the
# reason. needs_person is §207's field that earns the section: the board is here and somebody has
# to be at it, which is most of these.
HARDWARE = {
    16: ("riscv64 silicon", NONE, YES),
    25: ("aarch64 silicon; PMU cycle counter",
         "argon (the Jetson TX1 under seL4's published aarch64 numbers, so the comparison is "
         "like for like)", YES),
    53: ("riscv64 silicon with an NVMe controller and a NIC", NONE, YES),
    74: ("aarch64 and riscv64 silicon; PMU cycle counter", NONE, YES),
    88: ("rented aarch64 metal (Oracle Ampere, AWS Graviton .metal)", NONE, YES),
    89: ("a rented Scaleway Elastic Metal RV1", NONE, YES),
    101: ("riscv64 silicon; PMU cycle counter", NONE, YES),
    127: ("aarch64 silicon",
          "argon (bought as the seL4 machine, so identical silicon referees the comparison)", YES),
    143: ("riscv64 silicon with a RISC-V IOMMU (v1.0 or later)", NONE, NO),
    157: ("a board booted by U-Boot with a display attached", NONE, YES),
    163: ("JH7110 silicon", NONE, YES),
    168: ("real silicon; PMU cycle counter", NONE, YES),
    201: ("aarch64, riscv64 and x86_64 silicon", NONE, YES),
    224: (NONE, "radon (the work is radon's own outlet and its network path)", YES),
    225: ("aarch64, riscv64 and x86_64 silicon", NONE, YES),
    239: (NONE, "radon (the device tree radon's own firmware hands over is the subject)", YES),
    249: (NONE, "radon (the boot lottery being sampled is radon's)", YES),
    260: (NONE, "xenon (its firmware settings and the house network it boots from are the work)",
          YES),
    261: ("x86_64 silicon with VT-d and an NVMe drive", NONE, YES),
    271: ("x86_64 silicon with PCID", NONE, YES),
    320: (NONE, "xenon (whether the walk finds its Micron drive is the question)", YES),
    321: ("four-core x86_64 silicon", NONE, YES),
    335: ("riscv64 silicon with an ASID-tagged TLB", NONE, YES),
    347: (NONE, NONE, YES),
    363: ("silicon with a tagged TLB", NONE, YES),
    374: ("riscv64 silicon; PMU cycle counter", NONE, YES),
    378: (NONE, "xenon (its own ACPI DMAR table is what is read)", YES),
    400: ("an x86_64 UEFI machine with a display attached", NONE, YES),
    493: ("riscv64 silicon with a disk", NONE, YES),
    503: ("aarch64 and riscv64 silicon; PMU event counters", NONE, YES),
    524: ("an x86_64 CPU reporting invariant TSC", NONE, YES),
    558: ("aarch64, riscv64 and x86_64 silicon", NONE, YES),
    592: (NONE, "radon (the fault is in radon's own OpenSBI and PMIC)", YES),
    594: ("x86_64 silicon with two VT-d units and RMRRs", NONE, YES),
}
# SUPERSEDED, and what answered each: milestones, or a section where a ruling did it, as §177
# (whether AGENTS.md quotes measured numbers at all) answered milestone 350 (the comment ratio
# AGENTS.md quotes is wrong).
SUPERSEDED_BY = {
    301: "166", 350: "§177", 354: "250", 361: "188", 362: "420", 364: "303, 420",
    399: "405", 431: "319, 423", 556: "89", 577: "300, 302, 415",
}
# Promotions whose status paragraph says so in a shape the proposal parser does not accept; the
# slug is the proposal file git shows each was moved from.
PROMOTED_FROM = {
    295: "retire-the-builder-program",
    306: "time-the-hw-entropy-step",
    441: "a-program-that-makes-the-stick",
    541: "map-new-times-a-window-too-short-to-mean-anything",
    592: "radons-reboot-dies-in-opensbis-pmic-write",
}
# The commits that created files for milestones that already existed elsewhere: milestone 76's
# split of design/roadmap.md, and the backfill of 1 to 11 from DECISIONS.md's first table. Neither
# date is a raise date.
PRE_SPLIT = {
    "6192cd358e2c0c5c648cc1139e546cbbc8ec02cc",
    "048c486de767e801c33f54cd7a05b46bcd61adb9",
}

FNAME = re.compile(r"(\d+)-[a-z0-9][a-z0-9-]*\.md")
RAISE_WORDS = re.compile(r"\b(?:minted|raised|filed|written|proposed)\b"
                         r"[^.;]{0,60}?(\d{4}-\d{2}-\d{2})", re.I)
STATUS_HEAD = re.compile(r"\A\*\*Status: ([A-Z-]+)(.*?)\*\*[ \t]*", re.S)
GATE_HEAD = re.compile(r"\A\*\*Gate: ([A-Z0-9§, ]+)\.\*\*[ \t]*")
BRANCH = re.compile(r"`([a-z][a-z0-9]*(?:/[A-Za-z0-9._-]+)+)`")

def utc(epoch):
    return datetime.datetime.fromtimestamp(int(epoch), datetime.timezone.utc).strftime("%Y-%m-%d")


def git(root, *args, data=None):
    return subprocess.run(["git", "-C", root, *args], input=data, capture_output=True,
                          check=True).stdout


# ---- git: when each path first existed, followed through renames --------------------------------
def lineage(root):
    """({path: (epoch, sha)} of the earliest commit each live path's lineage appears in, and the
    same for every path that ever existed, deleted or not). The second answers when a proposal a
    block was promoted from was written, since promotion deletes it."""
    out = git(root, "log", "--reverse", "--format=C\t%H\t%at", "--name-status", "-M",
              "--", "design/roadmap").decode()
    origin, ever, cur = {}, {}, None
    for line in out.split("\n"):
        if not line:
            continue
        f = line.split("\t")
        if f[0] == "C":
            cur = (int(f[2]), f[1])
        elif f[0] == "A":
            origin[f[1]] = min(origin.get(f[1], cur), cur)
            ever[f[1]] = min(ever.get(f[1], cur), cur)
        elif f[0][0] in "RC" and len(f) == 3:
            origin[f[2]] = min(origin.get(f[2], origin.get(f[1], cur)), origin.get(f[1], cur))
            ever[f[2]] = min(ever.get(f[2], origin[f[2]]), origin[f[2]])
        elif f[0] == "D":
            origin.pop(f[1], None)
    return origin, ever


def first_in_old_files(root, wanted):
    """milestone -> (date, title) where it first appeared as a heading or index row in
    design/roadmap.md, or as a row of DECISIONS.md's `## Milestones` table."""
    found = {}
    for path, scope in (("DECISIONS.md", "table"), ("design/roadmap.md", "roadmap")):
        revs = git(root, "log", "--reverse", "--format=%H %at", "--", path).decode().split()
        pairs = list(zip(revs[0::2], revs[1::2]))
        if not pairs:
            continue
        blob = git(root, "cat-file", "--batch",
                   data="".join(f"{h}:{path}\n" for h, _ in pairs).encode())
        pos = 0
        for h, at in pairs:
            end = blob.index(b"\n", pos)
            head = blob[pos:end].split()
            if head[-1] == b"missing":
                pos = end + 1
                continue
            size = int(head[2])
            text = blob[end + 1:end + 1 + size].decode("utf-8", "replace")
            pos = end + 1 + size + 1
            if scope == "table":
                m = re.search(r"^## Milestones\n(.*?)(?=^## )", text, re.M | re.S)
                text = m.group(1) if m else ""
                rows = re.findall(r"^\| *(\d+) *\| *(.*?) *\|", text, re.M)
            else:
                rows = (re.findall(r"^#{2,4} (\d+)\. (.*)$", text, re.M) +
                        re.findall(r"^\| *(\d+) *\| *[A-Z-]+ *\| *(.*?) *\|", text, re.M))
            for n, title in rows:
                n = int(n)
                if n in wanted and n not in found:
                    found[n] = (utc(at), title.strip("* "))
            if all(n in found for n in wanted):
                break
    return found


# ---- one block ---------------------------------------------------------------------------------
def paragraphs(lines, start):
    """[(first, last)] line spans of the non-blank runs from `start`."""
    spans, i = [], start
    while i < len(lines):
        if lines[i].strip():
            j = i
            while j + 1 < len(lines) and lines[j + 1].strip():
                j += 1
            spans.append((i, j))
            i = j + 1
        else:
            i += 1
    return spans


def migrate(text, num, raised_git, pre_split_date, today, ever=None):
    """(new text, None) or (None, reason it cannot be migrated). `num` is None for a proposal."""
    lines = text.split("\n")
    if lines and lines[0] == "---":
        return text, None
    spans = paragraphs(lines, 1)
    if not spans:
        return None, "no status paragraph"
    s0, s1 = spans[0]
    para = "\n".join(lines[s0:s1 + 1])
    sm = STATUS_HEAD.match(para)
    if not sm:
        return None, "no '**Status:' at the head of the paragraph under the title"
    token, inside = sm.group(1), sm.group(2)
    status_line = " ".join(l.strip() for l in lines[s0:s1 + 1])
    f = {}

    if num is None:
        m = re.fullmatch(r"\s*(\d{4}-\d{2}-\d{2})\.?\s*", inside)
        if token != "PROPOSED" or not m:
            return None, "a proposal's status is '**Status: PROPOSED <date>.**'"
        f["status"], built = "PROPOSED", ""
        inside = ""
        prose_dates = [m.group(1)]
    else:
        f["status"] = token
        bm = roadmap_block.PROSE_BUILT.search(text)
        built = bm.group(1) if bm else ""
        prose_dates = RAISE_WORDS.findall(status_line)
    promoted = roadmap_proposals.promoted_from(status_line) or PROMOTED_FROM.get(num)
    proposal = (ever or {}).get(f"design/roadmap/proposals/{promoted}.md") if promoted else None
    candidates = [d for d in prose_dates + [raised_git, pre_split_date,
                                            utc(proposal[0]) if proposal else ""] if d]
    raised = min(candidates) if candidates else today
    if built and raised > built:
        raised = built
    f["raised"] = raised
    if built:
        f["built"] = built
    if token == "IN-PROGRESS":
        b = BRANCH.search(status_line)
        if not b:
            return None, "IN-PROGRESS names no branch"
        f["branch"] = b.group(1)
    if promoted:
        f["promoted_from"] = promoted
    if token == "SUPERSEDED":
        if num not in SUPERSEDED_BY:
            return None, "SUPERSEDED, and not in the hand-read SUPERSEDED_BY table"
        f["superseded_by"] = SUPERSEDED_BY[num]
    if token == "REFUSED":
        named = sorted({int(x) for x in re.findall(r"[Mm]ilestone (\d+)", status_line)})
        f["refused_by"] = ", ".join(map(str, named)) or NONE

    # The gate paragraph, if there is one: the paragraph after the status, as script/roadmap reads it.
    gate_span = None
    if len(spans) > 1:
        g0, g1 = spans[1]
        gm = GATE_HEAD.match(lines[g0])
        if gm:
            gate_span = (g0, g1, gm)
    if any(l.startswith("**Gate:") for i, l in enumerate(lines)
           if not (gate_span and i == gate_span[0])):
        return None, "a '**Gate:' line somewhere other than the paragraph under the status"
    if gate_span:
        spec = gate_span[2].group(1).split(", ")
        mdeps, ddeps, hardware = [], [], False
        for t in spec:
            if t.startswith("MILESTONE "):
                mdeps.append(t.split(" ")[1])
            elif t.startswith("DECISION §"):
                ddeps.append(t.split("§")[1])
            elif t == "DECISION":
                ddeps.append(UNWRITTEN)
            elif t == "HARDWARE":
                hardware = True
            elif t != "NONE":
                return None, f"gate token {t!r} is not in the vocabulary"
        machine, specific, person = NONE, NONE, NO
        if hardware:
            if num not in HARDWARE:
                return None, "a HARDWARE gate that is not in the hand-read HARDWARE table"
            machine, specific, person = HARDWARE[num]
        f["milestone_dependencies"] = ", ".join(mdeps) or NONE
        f["decision_dependencies"] = ", ".join(ddeps) or NONE
        f["machine_requirements"] = machine
        f["specific_machine"] = specific
        f["needs_person"] = person

    # ---- the body -----------------------------------------------------------------------------
    # The same rewrite `helpers/prose_ratchet.py` reads a prose-form block through, so a migrated
    # block measures what it measured before, and a line that only lost its token is not new prose.
    out = roadmap_block.without_fields(text).split("\n")
    # A paragraph emptied of its token leaves two blank lines; one is a paragraph break.
    tidy = []
    for l in out:
        if not l.strip() and tidy and not tidy[-1].strip():
            continue
        tidy.append(l)
    # The H1 follows the closing fence directly. design/decisions/ puts a blank line there; here it
    # would let git's diff pair the old H1's blank line with the new one and report the H1 itself as
    # rewritten, which `script/citations --ratchet` then reads as an added line.
    return "\n".join(roadmap_block.render(f) + tidy), None


def main():
    root, args = sys.argv[1], sys.argv[2:]
    dry = "--dry-run" in args
    args = [a for a in args if a != "--dry-run"]
    d = os.path.join(root, "design", "roadmap")
    if args:
        targets = [os.path.relpath(os.path.abspath(a), root) for a in args]
    else:
        targets = sorted(f"design/roadmap/{f}" for f in os.listdir(d) if FNAME.fullmatch(f))
        pdir = os.path.join(d, "proposals")
        targets += sorted(f"design/roadmap/proposals/{f}" for f in os.listdir(pdir)
                          if roadmap_proposals.filename_problem(f) is None)
    today = datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%d")
    origin, ever = lineage(root)
    todo = []
    for rel in targets:
        text = open(os.path.join(root, rel)).read()
        if text.startswith("---\n"):
            continue
        todo.append((rel, text))
    wanted = {}
    for rel, text in todo:
        m = FNAME.fullmatch(os.path.basename(rel))
        if m and "/proposals/" not in rel and origin.get(rel, (0, ""))[1] in PRE_SPLIT:
            wanted[int(m.group(1))] = rel
    old = first_in_old_files(root, set(wanted)) if wanted else {}
    changed, refused, doubtful = 0, [], []
    for rel, text in todo:
        is_prop = "/proposals/" in rel
        m = FNAME.fullmatch(os.path.basename(rel))
        num = None if is_prop else int(m.group(1)) if m else None
        if not is_prop and num is None:
            continue
        o = origin.get(rel)
        raised_git = utc(o[0]) if o and o[1] not in PRE_SPLIT else ""
        pre = ""
        if num in wanted:
            if num in old:
                pre, then = old[num]
                now = text.split("\n")[0].split(". ", 1)[-1]
                if difflib.SequenceMatcher(None, then.lower(), now.lower()).ratio() < 0.3:
                    doubtful.append(f"{rel}: first seen {pre} as {then[:50]!r}")
            else:
                raised_git = utc(o[0])
        new, why = migrate(text, num, raised_git, pre, today, ever)
        if new is None:
            refused.append(f"{rel}: {why}")
            continue
        if new != text:
            changed += 1
            if not dry:
                with open(os.path.join(root, rel), "w") as fh:
                    fh.write(new)
    for r in refused:
        print(f"roadmap --migrate: refused {r}", file=sys.stderr)
    for r in doubtful:
        print(f"roadmap --migrate: check by hand, the title differs: {r}", file=sys.stderr)
    verb = "would migrate" if dry else "migrated"
    print(f"roadmap --migrate: {verb} {changed} file(s), {len(refused)} refused, "
          f"{len(targets) - len(todo)} already in frontmatter")
    sys.exit(1 if refused else 0)


if __name__ == "__main__":
    main()
