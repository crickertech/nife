# 235. A command that faults hangs the prompt, because nothing has a word for it

**Status: NOT-STARTED.** Minted 2026-09-02 by calef, from the lane that built milestones 231
(nothing counts how many capability slots a boot actually uses) and 233 (`login` dies on every boot),
which found it while proving its own gate could fail. *(Number provisional until the merge queue
lands it.)*

**Gate: NONE.** The supervision path DECISIONS §26 (the fault endpoint: thread death becomes a message a supervisor holds) already
carries the knowledge; what is missing is a word in the protocol, and nothing external blocks writing one.

**In brief.** **A spawned command that traps hangs the prompt.** `swish` waits on the job's result
endpoint and a killed thread never sends, so the shell waits forever. An ordinary non-zero exit is
handled correctly; it is specifically a **fault** that has no path back.

Found by deliberate experiment rather than by accident: milestone 233's lane patched `worker` to trap
in order to prove its new no-thread-killed assertion could actually go red, and the prompt hung.

## Why it is worth its own block

**The pieces already exist.** DECISIONS §26 (the fault endpoint: thread death becomes a message a
supervisor holds) means somebody already learns that the thread died. What is missing is a word: `grant_plan::spawnproto` has no way to say
*this job faulted* as distinct from *this job has not answered yet*, so the knowledge cannot travel
from the supervisor to the shell that is waiting.

That makes this a protocol gap rather than a bug in `swish`, and it is the kind that gets patched
locally by whoever meets it next unless it is written down.

## What it needs

A faulted job reaching the prompt as a **status**, not a hang. The design question this block does
not settle: whether the shell learns it by asking, by being told through the supervision path it
already has, or by the endpoint itself carrying a death. Those are three different couplings and the
cheapest to build is not obviously the right one.

## BUGS

- **The hang is worse than it looks in a test.** A test harness times out and reports; a person at a
  prompt sees nothing and has no way to know whether the command is slow or dead.
- **Nothing says how common this is.** Every program in the tree that faults under a shell hits it,
  and nobody has counted which ones can.
- **This block does not cover a job that hangs without faulting**, which is a different problem with
  the same symptom and no obvious answer at all.
