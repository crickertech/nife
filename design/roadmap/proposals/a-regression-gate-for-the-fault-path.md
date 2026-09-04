# Nothing stops the lost prompt returning, because the fault path has no regression gate

**Status: PROPOSED 2026-09-03.** Written by the milestone 247 sweep, from milestone 235's block.

**Gate: DECISION.** It needs a program that faults on purpose, and a new program is a new name,
which is calef's. Everything else is a lane's work.

**In brief.** Milestone 235 fixed a shell that hangs forever when a spawned command traps. The
evidence it was fixed was a **scaffold**: milestone 233's lane patched `user/src/worker.rs` to trap,
watched the prompt hang, and the patch was removed afterwards. So the defect is fixed and the proof
is gone. The work is a program whose job is to fault, wired into `script/shell-check` so a hung
prompt fails a gate, plus teaching milestone 233's no-thread-killed assertion to except that one
program.

## Why this matters

The fix is real and nothing holds it in place. A faulting command reaching the prompt is a property
of three couplings that milestone 235's block describes, any one of which a future change can break
quietly, and the symptom is a hang rather than an error. A hang under a shell test looks like a slow
test, which is the failure mode most likely to be waited out and rerun.

This is also the whole class rather than one command. Every program in the tree that can fault took
this path, so the gate is not protecting one binary; it is protecting the shell's answer to any of
them.

## What it would take

A small program that traps deliberately (a name calef gives), an entry in the shell test that spawns
it and asserts the prompt comes back with a fault reported, and an exception in
`script/shell-check`'s no-thread-killed assertion, which milestone 233 added and which this program
exists to violate on purpose. The exception has to be narrow: excepting the assertion generally
would retire the check that found this defect in the first place.

## Where it came from

Milestone 235's `## Follow-on`: *"A regression gate for the fault path. It needs a program that
faults on purpose, which is a new name and therefore calef's, and milestone 233's no-thread-killed
assertion in `script/shell-check` has to learn to except it. The scaffold that proved this milestone
was a patch to `user/src/worker.rs` and was removed afterwards, so nothing stops the lost prompt
returning."*
