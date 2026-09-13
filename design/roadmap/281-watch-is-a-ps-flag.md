# 281. `watch` holds exactly what `ps` holds, so it is a flag rather than a program

**Status: NOT-STARTED.** Minted 2026-09-13 by calef, from his own question while ratifying
`watch`'s name: should the refresh behaviour just be a command option for `ps` instead of a
program? *(Number provisional until the merge queue lands it.)*

**Gate: NONE.** calef decided the shape when he raised it. What is still his at build time is the
flag's spelling and whether `crates/watch` folds into `crates/ps` or stays a crate that `ps`
consumes, like every other naming decision.

**In brief.** In a capability system the question "should these be one program or two" has an
answer that is not a matter of taste: two programs are two programs when they hold different
authority. These hold the same authority, from the same named constants, and one is a loop of the
other.

## The measurement that decides it

Both programs take exactly three slots, and not merely three of the same shape. The same
`grant_plan` constants:

| slot | `user/src/ps.rs` | `user/src/watch.rs` |
|---|---|---|
| report | `REPORT = 0` | `REPORT = 0` |
| the process domain | `grant_plan::DOMAIN_SLOT` | `grant_plan::DOMAIN_SLOT` |
| diagnostics | `grant_plan::DIAGNOSTICS_SLOT` | `grant_plan::DIAGNOSTICS_SLOT` |

**The refresh needs no capability of its own**, which is the fact that could have gone the other
way and did not. `wait_interval()` is a yield-spin over `monotonic_nanos()`, because this kernel has
no timed wait (`user/src/timetable.rs`'s module docs name the gap). Had the interval needed a clock
capability, a flag would mean `ps` always carries authority it usually does not use, and the
separate program would be the least-authority answer. It does not, so it is not.

## And the second program is the first one's loop

`watch.rs` calls `ps::collect`, uses `ps::Row` and `ps::MAX_ROWS`, follows the same refusal rule and
the same diagnostics-before-output ordering. Its `diag_slot()` fallback carries the comment
**"`ps`'s own fallback, verbatim"**. Verbatim duplication across two binaries is the shape rule 7
exists to prevent, arrived at from the other direction: not a `#[path]` module, but a copy.

## The flag precedent is already in the planner

`budgeter --mem N` is a flag the *shell's planner* reads and acts on, by splitting untyped off its
own budget before the spawn. `ps --watch N` is a flag the *program* reads, which is strictly less
than the planner already does. `grant_plan`'s command table and `crates/swish`'s help text are the
two places that change.

## What it retires

One program, one `[[bin]]`, one archive entry in each of `xtask`'s manifests, one typed command, and
**one unratified name** — the one whose ruling raised this question. `ps` is already ratified.

## The refusals

- **Leave them as two programs.** Refused because the only argument for it was least authority and
  the measurement above removes it. What is left is that a reader types `watch` rather than
  `ps --watch`, which is a preference about typing and not about the system.
- **Keep `watch` and delete the duplication into `crates/ps`.** This is the smaller change and it
  fixes the verbatim copy, but it leaves two binaries with identical cspaces, which is the thing
  that makes the split arbitrary. It is a phase of this milestone rather than an alternative.

## BUGS

- **A merged `ps` ships a busy-wait.** The interval burns a core for roughly two seconds per
  refresh, which `watch`'s own `BUGS` already admits. Nobody triggers it without the flag, but `ps`
  would then contain a spin loop where today it does not, and the honest fix is the timed wait
  `user/src/timetable.rs` names as a gap rather than anything in this milestone.
- **`watch` is bounded by an iteration count, not by `^C`.** That is unchanged by this milestone and
  is the reason the loop is finite at all: an interruptible child is built with no capabilities in
  its cspace, so a `watch` that ran until interrupted could not survey anything.

## Follow-on

- **None.**
