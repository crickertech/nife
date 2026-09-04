# Three of AIM7's job categories are missing from the multi-tasking workload

**Status: PROPOSED 2026-09-04.** Written by milestone 168's lane, from `crates/job_mix`'s own `BUGS`.

**Gate: NONE.** For the design; the *result* it improves is still milestone 168's `HARDWARE`. The
jobs themselves are ordinary userspace work and develop under QEMU.

**In brief.** `crates/job_mix` keeps AIM7's four methodological properties and stands in for its
categories with five jobs: a compute grind, a working-set walk, a null syscall, a yield burst and an
IPC round trip. **Three AIM7 categories have no representative**, each refused for a stated reason
rather than overlooked:

| category | why it is absent |
|---|---|
| disk-file operations | needs a disk attached, which would make the instrument's availability depend on the runner's storage; the same reason `script/bench`'s `fs_*` rows are not in the gated set |
| process creation | costs an address space per iteration, and `spawn_el0` exists to reclaim exactly one at a time; at 32 concurrent tasks it would measure the memory-region allocator rather than the scheduler |
| user page mapping | needs a per-task address-space capability the spawn path does not currently hand out |

## Why it matters

DECISIONS §96's question is how much of this kernel's time goes into process-kernel overhead under
multi-tasking load, and the cost lives in kernel stacks left behind by threads that **block inside
the kernel**. Two of the three missing categories are exactly that: a filesystem call and a spawn
both block deep in a kernel path, where a null syscall and a yield do not. So the missing categories
are not a fidelity nicety; they are plausibly where the effect is largest, and a flat result from the
present mix is weaker evidence than a flat result from a mix that had them.

## What it would take

The map job is the cheapest and should go first: it needs the spawn path to hand a task a capability
on its own address space, or a shared target space the way `kernel/src/bench.rs`'s `map_el0` already
builds one. The spawn job wants the reclaim `spawn_el0` already does, done per task rather than per
supervisor. The file job wants the sweep to run with a disk attached and should probably be a second
mix rather than an addition to the first, so a run without a disk is still a comparable run.

## What is blocked until it is answered

Nothing. Milestone 168's instrument works and its result is interpretable, with this limitation named
in `crates/job_mix`'s `BUGS` and in `notes/job-mix.md` where a reader meets the number. What is at
stake is how much weight a flat curve can carry.
