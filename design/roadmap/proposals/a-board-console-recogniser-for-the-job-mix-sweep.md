# `board_console` cannot tell a finished job-mix sweep from a wedged one

**Status: PROPOSED 2026-09-04.** Written by milestone 168's lane, from its own block.

**Gate: NONE.** `crates/board_console` is host-tested Rust with no hardware in the loop, and the
sweep it would recognise already runs under QEMU on all three architectures.

**Promoted:** minted as **milestone 324** on 2026-09-18, as one of its parts rather than on its own.
calef promoted the cluster: this proposal and its siblings were each filed by a different lane
against the same surface, and answering them one at a time would have produced one brief per face of
a single finding. The record is
[design/roadmap/324-the-bench-console-is-a-reader-only.md](../324-the-bench-console-is-a-reader-only.md);
the status line above keeps its original date, because that is what makes the pile measurable, and
this file keeps its own argument, because the proposal is the argument as it stood and the milestone
is the account.
**In brief.** `crates/board_console` recognises the boot sequence and, since milestone 219, a soak's
`Stage::Soak` and its heartbeat, so `script/board-console` can return a different exit status for
each way a session ends. It knows nothing about milestone 168's sweep. An operator at radon
therefore reads the log by eye to tell `job-mix: done` from a run that stopped after two of six
points, which is the one thing `script/board-console` exists to stop people doing.

## Why it was not done in milestone 168's lane

The recogniser is shared judgment that the boot sequence and the soak both depend on, and it is the
piece that decides what a hang *is*. Growing it for a run nobody has taken yet would mean guessing
at that run's failure modes. `notes/job-mix.md`'s outcome table is the honest interim: it enumerates
the six things the log can say and what each routes to, written from the code rather than from
experience. **After the first bench evening, that table is evidence rather than a guess**, and the
recogniser should be written from it.

## What it would be

A stage the sweep's `job-mix: started` line reaches, a recogniser for the per-subrun result line
(`job-mix: tasks=N jobs=J ticks=T jpm=R`) so the console can say how many points landed, and the
quiet check re-armed for the duration of a sweep the way `Stage::Soak` re-arms it. The exit statuses
already exist and mean the right things; what is missing is the recognition.

## What is blocked until it is answered

Nothing. The measurement can be taken and read by hand. What it costs is that a sweep which wedges
partway through is noticed by a person rather than by an exit status, and that a repeated-boot
procedure (which `notes/job-mix.md` asks for, five to ten boots) cannot be judged in a loop.
