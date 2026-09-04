# DECISIONS 113's rename reached the code and stopped there

**Status: PROPOSED 2026-09-03.** Found while fixing milestone 133's block for the same reason, which
is how the scale became visible.

**Gate: NONE.** It is a rename with a decided target vocabulary and no open question.

**What the work is.** DECISIONS §113 (eleven kernel object and identifier names move from contraction
or borrowed jargon to the plain, standard term) was decided by calef on 2026-08-23, after he said the
old names *"are clearly not working"* because he had repeatedly had to ask what they meant. The code
was renamed: `kernel/src/cap.rs` carries `ThreadControlBlock(ThreadId)`.

**The prose was not.** Measured 2026-09-03: **115 occurrences of the retired names across 36 files**
under `notes/` and `design/roadmap/`, excluding §113 itself, which documents the rename and should
keep them. `notes/tcb.md` is still named for a retired word and is indexed in `notes/README.md` as
*"The TCB"*.

**Why it matters more than a normal rename.** §113's whole reason was that a reader meets a name and
cannot tell what it is. Renaming the code and not the prose produces the worst of both: the reader
now meets **two** vocabularies, the identifiers say `ThreadControlBlock` and the explanations say
`Tcb`, and nothing tells them these are the same thing. That is a newcomer cost, which is AGENTS.md's
third principle, and it is measurable rather than a matter of taste.

**Why nothing caught it.** No gate reads prose for retired identifiers. `script/lint` checks that
citations are grounded, that links resolve and that names in `script/names` are ratified; none of
those notice a paragraph using a name the code no longer has. Milestone 252's sweep found the same
shape (*"ten stale claims sit in rustdocs, module headers and notes where no gate reads them"*) and
this is a larger instance of it.

**The blind-sed hazard is on the record and applies here.** AGENTS.md records a rename that swept the
tree and rewrote the very row saying a name had been **refused**. §113 itself must keep the old names,
and so must any passage describing the rename or quoting a transcript from before it. A mechanical
pass needs an exclusion list and a reader.

**Not done here, and the reason is scope**: this was found while fixing two files for milestone 133,
and 36 files is a lane rather than an aside.
