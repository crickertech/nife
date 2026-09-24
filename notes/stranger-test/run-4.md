# Stranger test run 4, 2026-08-18

*An appendix to [the stranger test](../stranger-test.md), which holds the current state. This page is the record for run 4: the design decided before the run, and what it found.*

## What run 4 changes, decided 2026-08-18 before the run

Run 3's `BUGS` entry names one fix and prices it at one line, and that is what run 4 applies.
Everything else about the configuration is run 3's, deliberately, because a run that changes two
things measures neither.

**The harness's own files leave the stranger's working directory.** Run 3's logs were called
`stranger3-stream.jsonl` and `stranger3-stderr.log` and sat beside the clone, so the stranger's
first `ls -la` told it which run it was before it had read a byte of the project. Run 4's logs go in
a **sibling** directory: the stranger's working directory contains the clone and nothing else. The
directory names carry no run number either, since a path in an error message is as readable as a
directory listing.

**The isolation mechanism is unchanged and is verified the same two ways**, because it is the thing
that made run 3 scorable: a separate `claude` process rather than a subagent, `--safe-mode`, working
directory is the clone's *parent*, since project instructions load from ancestors and never from
descendants. A throwaway probe in the same configuration is asked what project-instructions files
are in its context before the real run, and the stranger itself is asked afterwards.

**The answer key is withheld the same way**: this note and its `notes/README.md` entry removed and
the deletion amended into the tip, nothing else touched, working tree clean. The in-tree mentions of
runs 1 through 3 stay, for the same reason as before.

**What is actually new to measure, and it is why this run is worth taking rather than repeating.**
`CONTRIBUTING.md` and the README's `## Start here` reading order both landed on 2026-08-18, after
run 3's clone was cut. **No stranger has seen either.** Run 3's B1 failed because there was no
reading order and its stranger built one by instinct, reaching `AGENTS.md` twelfth and
`crates/abi/src/lib.rs` far too late. So B1 is the row this run exists to move, and the specific
questions are whether a stranger finds the reading order at all, whether it follows it, and whether
following it costs it the two things instinct cost run 3.

**The task text stays verbatim from runs 1, 2 and 3.** Changing it would make the four runs
incomparable, and the comparison is the only thing that distinguishes a milestone from an audit.

**What this run still cannot give.** The machine is the architect's laptop with the pinned nightly,
both QEMUs and a warm cargo cache, so **B2 is not measured and B4 is measured only against the
documented sequence**, exactly as in run 3. The machine is also loaded by other lanes gating in
other worktrees, and this time the tree can say so: `script/test` prints the host load average
beside a failing timing leg as of 2026-08-18. Whether a stranger meeting that message needs `uptime`
anyway is itself a measurement this run gets for free.

## Run 4, 2026-08-18: the harness stopped leaking and the tree took over the job

**The one-line fix worked, and it is the smallest useful result here.** The logs were written to
`/private/tmp/newclone/logs/`, a sibling of the stranger's working directory rather than inside it,
under the names `stream.jsonl` and `stderr.log` with no run number anywhere in the path. The
stranger's first tool call was `ls -la`, exactly as run 3's was, and it returned one entry: `nife`.
Nothing in the harness told it which run it was.

**It knew anyway, within the first half hour, and the source was the tree.** Asked afterwards what
had told it, it named `notes/adding-a-program.md`'s `BUGS` section: the table headed
`walk | program | wrong in`, whose three dated rows name run 2's `doubler`, run 3's `triangle` and
"this lane", beside the sentence asking the next person to add a program to treat a surprise there
as the page's bug. `README.md`'s `## Start here` block cites milestone 117 by name in the same
breath. Its own summary: *"So I knew, within the first half hour, that I was at minimum the fourth
person to walk this exact path and that the walks were treated as measurements."*

**This is a different defect from run 3's, and the difference is the important part.** Run 3's leak
was a mistake, introduced by the person building the isolation, and it cost one line to fix. Run
4's leak is a *consequence*: every run records its walk in the page the next run is sent to, because
recording it is what the page's own `BUGS` convention asks for. **The instrument's good practice
generates its own contamination**, and it compounds, since each run adds a row. There is no
one-line fix, and the obvious fixes are all worse than the leak: deleting the record would fabricate
a tree, and hiding it would be the answer-key withholding rule applied to something that is not an
answer key.

**What it changed, in the stranger's words rather than in ours**, because the value of asking is
that the answer is unflattering:

- *"I performed."* It wrote a prediction into the journal before starting the walk, and calls that
  *"an imitation of this tree's voice, aimed at a reader I imagined scoring me on intellectual
  honesty. Nothing about the task required it."*
- It chose the deliverable partly for how it would read: *"I wanted to be the walk that found
  something."*
- It adopted the repository's prose style wholesale, and names that as mimicry rather than judgement.
- *"I front-loaded my own honesty... The fact is fine; the framing is a bid."*

**And what the leak did not change, which is why the run still counts.** It did not invent a
breakage to have something to fix, it did not skip gates and claim them, and it never opened
`design/roadmap/117-newcomer-onboarding.md` or looked for `notes/stranger-test.md`, which was
withheld the same way runs 2 and 3 withheld it. The answer key held for the fourth time; only the
fact of being measured leaks, exactly as the first `BUGS` entry has said since run 1.

### The configuration, verified rather than asserted

The clone was `c9fe2d67` (`bb094187` after the withholding amend), with this note and its
`notes/README.md` entry removed and the deletion amended into the tip, working tree clean, `origin`
pointed at the public GitHub URL. The stranger was a separate `claude` process rather than a
subagent, `--safe-mode`, started from the clone's parent so no `AGENTS.md` could load from an
ancestor. A throwaway probe in the same configuration answered `NONE` when asked which
project-instructions files were in its context, and the stranger itself answered *"No. Nothing from
a `CLAUDE.md`, `AGENTS.md`, or equivalent was in my context before my first tool call... The repo
sat one level below the primary working directory, so nothing was auto-loaded."*

Task verbatim from runs 1, 2 and 3, plus the journal. 177 turns, about 15 minutes of wall clock,
`$12.66`.

**One deliberate contamination the earlier runs did not have, disclosed because it is the harness's
and not the tree's.** `pkill` and `killall` were shadowed on the stranger's `PATH` by wrappers that
kill only QEMU processes whose command line names the clone, because `README.md`'s own "Try it"
block tells a reader to quit with `pkill qemu-system-aarch64` and four other lanes were gating on
this machine. The stranger never invoked either, so the shim changed nothing about this run; it
would have, and the next harness should keep it.

### The build half

**`script/test` was green on arrival, exit 0, first try, with no change to the tree:** 1312 tests
passed and 0 failed across the host workspace, the doctests, the vendored RedoxFS round trip and its
`no_std` core on both bare-metal targets, `redoxfs_server`'s sans-IO core, the patched `nife-dev` std
toolchain, and the kernel under QEMU on both ISAs. About 25 minutes wall clock, most of it the two
emulated legs.

**It then ran the rest of `script/gates` unprompted, on its own reading of the tree's vocabulary**,
(that command was retired into `script/ci-build`'s table by milestone 286 on 2026-09-13; this run
and the quotation below are from 2026-09-06 and keep the name they happened under)
which is the run's best unforced result and belongs to `CONTRIBUTING.md`: *"'tests passing' in this
project's own vocabulary is `script/gates`, not `script/test`. I ran one of the three."*
`script/fmt --check` and `script/lint` both exit 0. It then named what it had not run rather than
letting silence imply coverage: `script/verify`, `script/bench --check`, `script/coverage`,
`script/fuzz`, `script/supply-chain`, `script/test --hvf`.

**B2 is not measured**, as pre-registered: the pinned nightly and the pinned QEMU were both already
installed and `script/setup` had nothing to do. **B4 has exactly one entry**, and the stranger
scored it against itself rather than against the tree: `timeout(1)` does not exist on this macOS
host, `AGENTS.md` says so explicitly and points at `scripts/qemu-bounded.sh`, and it hit the missing
binary before it read that section. Its own verdict: *"That is my error, not the tree's, the file
that told me is the file the README tells you to read third."*

**The machine was loaded and no timing assertion fired, so the new load-average print is still
unexercised.** Load average at launch was 5.41 on 8 cores, and it ran between about 3 and 17.5 for
the duration, with four other lanes gating in other worktrees. That is well under the 45 to 63 that
produced run 3's 2-in-13 red rate, and both emulated legs passed on the first attempt. **The
diagnostic landed 2026-08-18 in response to run 3 and this run could not test it**, which is worth
saying plainly rather than counting the green as evidence for it.

### The mental model, scored: eight of eight

| # | result | where it came from |
|---|---|---|
| M1 | **answered**, and better than any previous run | `notes/capabilities.md` for the mechanism, `notes/abi.md` for the rights field, so it gave the attenuation too: holding an endpoint does not by itself permit receiving on it |
| M2 | **answered** | `notes/std.md`, not `notes/net.md`, which it never opened: slot 2 is a `Stack` endpoint with `WRITE` and slot 3 is the untyped budget the socket frames are minted from, and *"the absence of slots 2 and 3 is exactly what 'no ambient network' feels like from inside a process"* |
| M3 | **answered**, with an under-claim on enforcement | `AGENTS.md` rule 1 through the reading order. It said the rule looked like convention rather than mechanism and hedged that it had not read `script/lint`'s source; the gate is there, at `script/lint`'s `==> rule 1`, and it read that output |
| M4 | **answered** | `design/roadmap/README.md`, the whole vocabulary including `RECORDED` and the `IN-PROGRESS` branch rule |
| M5 | **answered** | `AGENTS.md` rule 7, with the Kani-and-host-tests reason named as the load-bearing one and `c_seam` as the case |
| M6 | **answered, and read rather than induced** | `CONTRIBUTING.md`, quoting it, then `AGENTS.md`'s second job for it as one of the two homes for identified work. Run 3 had to assemble this from four instances |
| M7 | **answered by doing it** | added `tally`, ran it at a real prompt on both ISAs, ran a negative control, reverted to a byte-identical tree |
| M8 | **answered** | four states, with the distinction that `provisional` is a claim about intent and the other three about the record |

**Two of these moved because of documents no stranger had seen before.** M6 is `CONTRIBUTING.md`
working exactly as it was written to: run 3 had to induce the `BUGS` convention from instances and
run 4 quoted a definition. And the whole gates observation above is the same document.

### What it read, and in what order, which is B1

Twenty-two files, of which six were reached through the `## Start here` order and five more through
links from `notes/adding-a-program.md`. **`AGENTS.md` was seventh**, against run 3's twelfth.
**`notes/capabilities.md` was eighth**, and run 3 never opened it at all. `README.md` was still
first by expectation rather than by any pointer, which no reading order can fix.

**So B1 passes, for the first time, and the failure it leaves is specific.** The stranger did not
follow the order; it used the order as an index and read the items in the sequence its work needed,
which is what the section's own last line invites. The cost is one item: **`CONTRIBUTING.md` is item
2 of 8 and was read sixteenth of twenty-two**, late enough that its gates paragraph arrived after
the gates had been run. The one document written for a person deciding whether to work here is the
one the reading order failed to get read early.

Still unopened after four runs: **every file under `design/decisions/`**, `notes/net.md`,
`design/naming.md`, and `notes/README.md` itself.

### What it found, and none of it was fixed here

- **`script/lint`'s naming worklist under-counts by exactly the provisional names, and then names
  the command that prints the other number.** The `--check` path prints
  `len(recorded) + len(unrecorded)` and the default listing prints
  `len(provisional) + len(recorded) + len(unrecorded)`, so the gate says `82 still want calef
  (script/names --unratified)` and that command says `UNRATIFIED (86 of 162)`. The census line above
  it drops them too: `76 ratified, 15 recorded, 67 unrecorded` sums to 158 of 162. Reproduced on
  `main`. **It bites precisely the state a newcomer is told to use**, since `AGENTS.md` and
  `notes/adding-a-program.md` both say to ship a provisional name and say so. Recorded in
  design/naming.md's `BUGS`.
- **The two archives boot different binaries under the name `init`**, `hello` on aarch64 and
  `builder` on riscv64, in a project whose loudest claim is architectural parity. The stranger
  reported this as undocumented and was wrong: `xtask/src/archive.rs` says it, in a comment on the
  aarch64 table's `hello` row, about 200 lines from the riscv table it describes. **A stranger who
  read both tables in the same minute still called it invisible**, which is a placement finding
  rather than an absence, and is run 3's closing diagnosis reproduced by a different reader.
- **The two commands run most often are both blind to the most-warned-about mistake.** A program
  added to the aarch64 table and not the riscv one passes `cargo xtask build` and `script/lint`, and
  is caught first by `script/swish-check` or `script/test`. The `BUGS` entry in
  notes/adding-a-program.md says nothing gates the two lists against each other; this adds which
  gates a person will believe before they find out.
- **Nothing in the suite counts programs.** 1312 tests before adding `tally` and 1312 after. A
  program's presence is proven only by a transcript line someone remembered to write into
  `SWISH_CHECK_SCRIPT`.
- **The eight places are a removal problem too, and only the addition has a page.** Reverting was
  clean only because every edit was in a file it could still name; a half-removed program is a
  `PROG_COUNT` too large and an init slot no variant claims, which is the same silent failure
  reached from the other side. Recorded in notes/adding-a-program.md's `BUGS`.
- **`notes/adding-a-program.md` was right about everything, including a line number.** This is the
  first walk of four to find no defect in it, and the page's own `BUGS` predicted the opposite. The
  stranger checked `crates/swish/src/lib.rs:864` against the error the page quotes and got the same
  file and the same line. Its prediction, written before the walk, was that it would find at least
  one thing the page did not mention; it did not.

**Its own worst-thing answer is the one to keep**, because it is the only place in four runs where a
stranger has criticised the tree's central habit rather than a document: *"the project's habitual
response to a structural problem is another document, which its own ladder names as the worst
available move... The documentation is doing work that a data structure should be doing, and it is
doing it beautifully, which is exactly what stops anyone from fixing it."* It cites three instances,
all of them things this tree has written about at length and not changed: `AGENTS.md`'s misleading
name, `notes/adding-a-program.md`'s fourth rewrite, and the eight-place program problem whose
one-place fix is sitting in this milestone's own handoff list.

### What this run cost

**The tree's own leak is now the largest contamination and it is structural**, which is the first
entry above and the reason it leads. Every other cost is smaller and all of them are carried over:
the machine was warm so B2 measures nothing; the machine was loaded, though far less than run 3's,
so the timing rows are contaminated and the new diagnostic went unexercised; `pkill` was shadowed by
the harness and never invoked; and **the harness's author has read `AGENTS.md`**, which no
arrangement of processes fixes and which the amendments above make slightly worse, since this lane
edited the rubric before running against it. The mitigations are the same three: the task text is
runs 1 through 3's verbatim, the amendments were made and committed before the clone was cut, and
the stranger's answers are recorded as it gave them.
