# 427. The documentation sweep the worklist already ranks

**Status: PROPOSED 2026-09-17.** Raised by milestone 311 alongside its security sibling; the same
repaired tripwire reports both.

**Gate: NONE.** `script/audits --worklist` runs today and needs nothing built.

## In brief

```
documentation  last 2026-08-17 (The ABI surface as documented, read from the wire outward: e)
  milestones built     76 -> 188  (+112, fires at 10)   <-- FIRED
  components           110 -> 155  (+45, fires at 10)   <-- FIRED
  ABI constants        50 -> 52  (+2, any change fires)   <-- FIRED
  external packages    108 -> 108  (+0, fires at 30)
  calendar             31 days since (12 weeks)
```

Overdue by eleven times the milestone trigger. Unlike its security sibling this one arrives with its
scope already computed, which is why it is the cheaper of the two to brief.

## The worklist, which is the scope

`script/audits --worklist` ranks every document by how many of the files it cites have moved since
the document was last edited. Its head on 2026-09-17:

```
 moved/cited  commits  last edit   document
   16/16           72  2026-09-14  design/roadmap/284-finish-the-progenitor-rename.md
   14/19           53  2026-09-14  design/roadmap/158-kernel-object-rename-build.md
```

**Read `notes/documentation-audit.md` first.** It is the procedure, it says what counts as a finding,
and it is honest that the ranking has never read a sentence: a document at the top may be perfectly
true, and one absent from the list may be a year out of date. 138 of 269 documents cite no resolvable
code path and are simply not on it.

## One scope this sweep can start from, measured rather than guessed

Milestone 311 swept for paths that milestone 175's `user/` split moved, and found the sweep is
larger than its own lane. **Eight distinct `components/` and `fixtures/` paths cited in backticks in
markdown across the tree do not resolve today.** Six of them are historical prose and correct under
AGENTS.md's rename rule, which says a record of the past keeps the old name; two were live rot and
are fixed in 311. Separating those two categories needs a reader, which is the whole argument for
routing this to a sweep rather than to a gate, and `script/lint`'s own comment on the matter says so.

`design/roadmap/proposals/a-backticked-path-that-does-not-resolve.md` is the standing proposal for
gating this class tree-wide. It is worth reading alongside, and it is worth noting that **its own
enumeration of root directories still lists `user/`**, three days after that directory stopped
existing, which is a small exhibit for why the sweep is due.

## What closes it

A report in `design/audit-reports/`, both index rows, every finding dispositioned. `script/audits
--due` stops naming `documentation` on its own.
