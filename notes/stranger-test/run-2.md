# Stranger test run 2, 2026-08-16

*An appendix to [the stranger test](../stranger-test.md), which holds the current state. This page is the record for run 2: what it withheld, decided before the run, and what it found.*

## What run 2 withholds, and what it cannot, decided 2026-08-16 before the run

Run 1's first `BUGS` entry said run 2 must not have this note in the tree it is given. Trying to
obey it exactly is what showed the entry was asking for something the tree cannot supply, so the
rule this run used is narrower and is written here rather than left as an intention:

**Withhold the answer key, not the fact that a test exists.** Each stranger got a clone of
`d6a16b7` with `notes/stranger-test.md` and its `notes/README.md` entry removed and the deletion
amended into the tip, so the working tree is clean and no `git status` line advertises it. Nothing
else was touched.

**The mentions of run 1 stay, because removing them would fabricate a different repository.**
`README.md`, `DECISIONS.md`, `notes/README.md` and the milestone 117 block all cite the first run,
and three of them do it while making a point a newcomer needs (why `CLAUDE.md` is misnamed, where
`§N` resolves, what `adding-a-program.md` is for). A tree with those cut is not the tree under test,
and the milestone's own sentence is *with only this repository*. So a run 2 stranger can discover
that this project tests its onboarding; what it cannot discover is the rubric's "pass means" column,
which is the part that made run 1's answers uncountable.

**The stranger writes its log as it goes, and that is a change to the instrument.** Run 2's first
attempt died part way and left nothing at all: the findings lived in the stranger's own context and
went with it, which is rung four of the ladder wearing a different hat. So the second attempt keeps
an append-only journal outside the tree, written before and after each step, with the questions and
the had-to-work-it-out items recorded at the moment they are hit rather than assembled at the end.
The cost is real and belongs in `BUGS`: a stranger told its confusion is the deliverable is watching
itself, which run 1's stranger was not.

**The machine is no longer cold, and the build half is weaker for it.** The maintainer ran
`cargo --version` inside the repository while getting oriented, which installed the pinned nightly
from `rust-toolchain.toml`, and the attempt that died had already installed `qemu-system-aarch64`
and `qemu-system-riscv64`. So B2 and B4 measure a partly warmed machine: whatever those two steps
would have cost a newcomer, this run cannot see. A cold measurement wants a fresh container and is
worth doing separately rather than pretending this one is it.

**The strangers were subagents of a maintainer session whose working directory is the repository**,
which is a weaker isolation than run 1's container and may hand them `CLAUDE.md` before they choose
to read anything. That would contaminate the reading-order row and four of the eight mental-model
questions, all of which `CLAUDE.md` answers directly. It is measured rather than assumed: each
stranger is asked afterwards what it read and in what order, and what was in front of it before it
chose. A run whose answer is "the constitution was already there" reports no B1 and no M3, M5, M6 or
M8, rather than reporting them generously.

## Run 2, 2026-08-16: a stock Linux box, and a stranger that was not one

**Task given:** the same words run 1 got, plus a journal. "Get the project building and its tests
passing, then write up what this system is and how you would add a new user program to it," with an
append-only log written before and after each step, because run 2's first attempt died part way and
left nothing behind at all.

**It got to green, and the build half is the finding.** `script/test` exits 0 on both ISAs (260
aarch64 and 263 riscv64 kernel assertions under QEMU, on top of the host crates), `script/lint`,
`script/fmt --check`, `script/names --check` and `script/swish-check` all pass, and `script/verify`
reached 91 harnesses across 17 of 19 crates with no failures before the stranger stopped waiting on
`calendar` and `glob`. It also added a program, `doubler`, and answered `doubler 21` at the prompt on
both instruction sets, which is the only way to test the how-to page rather than read it.

**`script/setup` cannot complete on a stock Linux box, and had not been able to for weeks.** No
Ubuntu release ships a QEMU with `riscv-iommu-pci`, so `script/qemu-check` hard-fails by design;
`script/bootstrap` is `set -e`, so it ended there, before the clang step; and the remedy the error
named, `script/ci-qemu`, described itself in its own first line as **"CI only"**. The reader was told
to run a script that told them not to. Every contributor's machine was already warm, so nobody had
met it.

> **Correction, 2026-09-13 (milestone 287): the fix recorded here was half a fix, and the sentence
> that claimed it worked was false about the tree for twenty-eight days.** What this paragraph used
> to end with was: *"Fixed: bootstrap now prints the whole sequence on Linux, and `ci-qemu` no longer
> claims to be for CI alone."* The `ci-qemu` half was true. The bootstrap half was not, three times
> over.
>
> **It never printed.** The new branch was guarded by `[ "$os" = linux ]` and `uname -s` says
> `Linux`, so no reader ever saw the message. Nothing caught that, because nothing on Linux ran
> bootstrap from cold again.
>
> **Printing would have been rung four anyway.** A script that knows the next two commands and asks
> a human to type them is describing a mechanism rather than being one, which is the ladder's floor.
>
> **And the two printed commands looped.** `script/ci-qemu` installs into
> `$HOME/.cache/nife-qemu`, and the only thing in the entire tree that ever put that prefix on PATH
> was `.github/workflows/ci.yml`. Nothing in `script/`, nothing in `scripts/`, nothing in `xtask`. So
> a Linux developer following the instructions verbatim spent twelve minutes building the right QEMU,
> re-ran `script/setup` as told, and `qemu-check`'s `command -v qemu-system-aarch64` found
> `/usr/bin`'s 8.2.2 again: same failure, same message, same remedy, forever.
>
> **Why nobody met any of it, which is the same reason as the first time.** Every contributor's
> machine was already warm, and CI sets the PATH itself in `ci.yml`, so the only configuration that
> exercises this path is the one nobody has: a cold Linux clone. Run 2's whole value was being that
> configuration, and the fix it prompted was never run against it.
>
> Milestone 287 is the actual fix: bootstrap runs `script/ci-qemu` instead of printing how to,
> `scripts/qemu-path.sh` is the PATH half, and `script/lint` gates that every entry point resolves
> it. Reproduced and verified on a stock Ubuntu box with apt's 8.2.2 on `/usr/bin`.

**The four corrections to `notes/adding-a-program.md`** are the second half, and they are the page's
own BUGS entry coming true: it asked the first person to add a program against it to correct whatever
it got wrong. The aarch64 initrd has a newer list the page did not know about, so following it wrote
eight lines of dead boilerplate. The riscv64 side is two edits rather than one, and missing the
`--bin` half fails the build on a file cargo was never asked to produce. The `grant_plan` step is six
edits rather than four, and the two the page omitted are `from_name()` (without it the program is
unreachable from the prompt) and `PROG_COUNT` (whose own comment says forgetting it is an
out-of-bounds panic in init rather than a compile error). And the page did not warn that the build
will fail in `crates/swish`, which is the design working.

**One real code defect, found because the manifest already knew the answer.** `swish`'s `caps`
preview printed the `arg` line under `matches!(e.prog, Prog::Worker)`, a hand-maintained second copy
of a fact the manifest holds. `Worker` is the only argument-taking program today, so the tree could
not see it; the next program to take an argument would have previewed `arg (none)` and then been
handed the argument anyway. That is the worst direction for that line to be wrong in, since the next
thing it prints is that reading the command is reading its whole authority. Fixed to read
`manifest().arg`, with a test that sweeps every `Prog` rather than naming one, because a test naming
`Worker` would have passed against the bug.

**The `provisional` trap**, which is §89: AGENTS.md tells a lane to ship a provisional name,
`script/names` accepts only `ratified`/`recorded`/`unrecorded`, and writing the rules' own word gets
you a red gate. Two programs already work around it in prose. The decision is calef's; the page now
documents the trap either way.

### What run 2 cannot claim, and it is most of the mental-model half

**The stranger was not a stranger, and it said so itself.** Asked afterwards what had been in front
of it before it chose to read anything, it answered that `AGENTS.md`'s full contents arrived in its
context at turn zero, from the maintainer's checkout it had been told not to read, and that it used
that document throughout while **never opening the copy in its own tree**. Every "AGENTS.md says X"
in its write-up is really the other checkout's copy, asserted as though read.

So the pre-registered discount applies, and the rows it covers are reported rather than scored:
**B1** (a reading order exists) and **M3, M5, M6, M8**, all of which `AGENTS.md` answers directly.
What survives is what the tree had to supply on its own: **M1** and **M2** it answered from
`notes/capabilities.md` and `crates/abi`, quoting the note's own "you did not *hold* anything, you
**said a name**"; **M7** it answered by doing it, which is the strongest evidence in either run and
is exactly where the tree turned out to be wrong in four places. **M4** is weak: it read
`design/roadmap/README.md` only in part.

**It also saw that the repository instruments stranger runs, three times, and did not conclude it was
being measured.** `README.md`, `notes/adding-a-program.md` and `DECISIONS.md` all cite run 1 by name;
its own grep surfaced `design/roadmap/117-newcomer-onboarding.md` beside the page it wanted, and it
chose the page and never opened the block. That is the withholding rule working exactly as written
and no better: the fact leaked, the answer key did not, and the run stayed honest because the
stranger disclosed rather than because the tree hid anything.

**The one thing this run measured better than run 1** is the build from cold on Linux, and even that
is partial: the machine had the pinned nightly already, and a dead first attempt had installed both
QEMUs before the real run started.
