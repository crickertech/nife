# 439. A job-mix sweep goes quiet for a whole subrun, so a wedge timer has to guess

**Status: NOT-STARTED.** Minted 2026-09-19 by the integrator, from milestone 324's parts 2 and 3
lane, which built the sweep recogniser and found the limitation it could not fix from where it
stood. *(Number provisional until the merge queue lands it.)*

**Gate: NONE.** It is a `println!` from a timer in the supervisor, and the marker it prints is
already a shared constant. Nothing here is anybody's decision.

## The problem, and it is a comparison the tool cannot make

`crates/board_console` can tell a finished job-mix sweep from a wedged one, since milestone 324's
part 2. **What it cannot do is tell a wedged sweep from a slow one**, and the reason is that a sweep
emits nothing between subruns.

A soak does not have this problem. `kernel/src/soak.rs` prints a heartbeat on a wall clock, so the
watcher's quiet timer is sized against *the heartbeat interval*, which is a number the kernel
chooses. A sweep's quiet timer has to be sized against **the slowest subrun anyone has seen**, which
is a number nobody chooses and which changes with the board. The lane measured 2.6 seconds for the
longest subrun on QEMU (163,224,570 ticks on a 62.5 MHz counter) against a 60-second default: a
ratio of twenty to one, picked to be safe rather than to be right.

**So a board slower than twenty times QEMU reads as wedged when it is merely slow**, and there is no
way to tell the two apart from outside. radon is the case that matters, because it is the board the
sweep exists to run on and nobody has watched one there yet.

## What the work is

A heartbeat from the supervisor on a timer rather than only between subruns, the same shape
`kernel/src/soak.rs` already ships. Then `board_console`'s quiet timer is sized against a chosen
interval instead of against a measured worst case, which is the whole point.

**It is a kernel change**, which is why milestone 324's lane recorded it rather than taking it: the
supervisor has to print from a timer, and that is outside a milestone about a console tool.

## What it is not

**Not a new exit status.** `board_console` already reports a quiet run as `2`; what changes is the
threshold's justification, not the vocabulary. Milestone 324's lane deliberately added no status and
this should not either.

**Not a change to the sweep's output format.** The point and subrun lines are output two programs
read, and 2026-09-19 is the day that cost got paid once already: milestone 168's lane changed those
fields on `main` while milestone 324's lane wrote a parser for the old ones, and every gate stayed
green on both branches. A heartbeat is a new line rather than an edited one for exactly that reason.

## Follow-on

- **Recorded.** *The limitation as it stands today*, in the `BUGS` sections of `script/job-mix`,
  `crates/board_console/src/lib.rs` and `notes/job-mix.md`, each naming the twenty-to-one ratio and
  the fact that no sweep has been watched on a board.

## Index row

`crates/board_console` can tell a finished job-mix sweep from a wedged one since milestone 324, but
not a wedged one from a slow one, because a sweep prints nothing between subruns. Its quiet timer is
therefore sized against the slowest subrun anyone has measured, 2.6 seconds on QEMU against a
60-second default, which is twenty to one and is a guess rather than a choice. `kernel/src/soak.rs`
already solves this by printing a heartbeat on a wall clock, which makes the watcher's threshold a
number the kernel picks. The work is that heartbeat in the supervisor, and it is a kernel change,
which is why the lane that found it recorded it instead. The board it matters for is radon, where no
sweep has been watched at all.
