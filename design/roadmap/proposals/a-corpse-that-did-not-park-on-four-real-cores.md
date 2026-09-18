# A corpse that did not park on its supervision rendezvous, on four real cores

**Status: PROPOSED 2026-09-17.** Found on xenon the same evening as
[a kernel that maps one PCI bus](a-kernel-that-maps-one-pci-bus.md), and unlike that one and the two
fixed in PR #933, **this is not an assertion written against QEMU.** Transcript:
`bench/xenon-2026-09-17/nvme-attempt-2-no-controller-found.log`.

**Gate: NONE** to investigate. It may need hardware to reproduce, which is the first thing to find
out.

## What happened

```
test kernel::user::survey_tests::a_dead_child_is_still_in_the_domain_until_it_is_reaped ...
  user thread 17179869238 killed: vector 14 (page fault)
    rip 0x0000000000400005   addr 0x0000000000a50000   user rsp 0x0000000000501000   err 0x00000004
  the kernel is fine.

[PANIC] panicked at kernel/src/user/survey_tests.rs:381:5:
```

The assertion is:

```rust
assert!(
    super::wait_for(|| sched::rendezvous_waiting_senders(ep) == 1),
    "the child never died onto its supervision rendezvous",
);
```

**The child died.** The kill message is in the transcript, three lines above the panic, with the
fault address the test's `FAULT_STUB` is written to produce. What did not happen is the corpse
parking on its supervision rendezvous's sender queue where a survey can see it before anyone has
collected it, which is DECISIONS §26's property and `capability::survey_includes`' proved one.

## Why this is not the timing class the other two were

Both of tonight's other failures were margins that real silicon closed: a spawned thread that needed
8 yields where the code allowed exactly 8, and a chipset id that was q35's. The reflex is to read
this one the same way and widen the bound. **The bound is already two seconds of wall clock:**

```rust
let deadline = crate::arch::timer::now() + 2 * crate::arch::timer::frequency();
```

`wait_for` yields until then and checks once more on the way out. Two seconds is not a thin margin
on a 2.7 GHz machine for a thread that has already been reported dead, so widening it would be
treating a symptom whose cause has not been established. **A test that is made to pass by waiting
longer, when the thing it waits for should already have happened, has been silenced rather than
fixed.**

## What is actually unknown

Three readings, and nothing in the transcript separates them:

1. **The corpse parked and something took it off again**, in which case the survey would also not
   see it and §26's property is false on this configuration.
2. **The corpse never parked**, because death delivery on a core other than the one running the test
   took a path that does not enqueue the sender.
3. **The count was not 1**, because something else was on that queue. The assertion is `== 1` rather
   than `>= 1`, so a second sender fails it in a way that reads identically in the log.

**Reading 3 is the cheap one to eliminate and should go first**, because it is a one-line diagnostic
rather than an investigation: print the count the test actually saw. A test that asserts an equality
and reports nothing about what it got is hard to diagnose from a transcript, which is the general
lesson milestone 318 drew about assertions against constants.

## Why it matters more than its size suggests

This is the supervision mechanism, not a peripheral one. **A corpse a supervisor cannot see is the
failure §26 exists to prevent**, and the test's own doc comment says why it was written as one case:
the domain the viewer reports and the domain the supervisor may collect from are one set. If that
set differs on a multi-core machine, then a property this tree proves and tests is true under
emulation and unverified where it matters.

**x86_64 running four real cores is new**, which is the other reason to take it seriously rather
than to retry. `NIFE_SMP` has been flipped to 2 for the runner (§153) and this machine boots 4. The
combination of real cores and real speed is the condition under which this tree has historically
found its scheduler bugs: the VisionFive 2's undelivered-wake bug was invisible in QEMU and is
cited in `design/fatal-risks.md` risk 2 as the case no proof was positioned to see.

## What a lane does first

1. **Reproduce**, and establish whether it is xenon, four cores, or real cores. `NIFE_SMP=4` under
   QEMU is the cheapest attempt and costs one run.
2. **Print the count**, eliminating reading 3.
3. Only then investigate delivery.

**Do not widen `wait_for`.** If the conclusion turns out to be that two seconds is genuinely not
enough, that conclusion needs its own evidence and is itself a finding worth more than the test.
