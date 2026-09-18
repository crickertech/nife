# 321. A corpse that did not park, on four real cores

**Status: NOT-STARTED.** Minted 2026-09-17 by the maintainer, from
`design/roadmap/proposals/a-corpse-that-did-not-park-on-four-real-cores.md`, which this block
replaces. *(Number provisional until the merge queue lands it.)*

**Gate: NONE.** Investigating needs no hardware. Whether *reproducing* needs it is the first thing
to find out, and is itself the finding.

**This is the one failure from xenon's 2026-09-17 bench evening that is not an assertion written
against QEMU.** The other three were: a q35 chipset id, an eight-yield window, and a one-bus PCI
scan. This one is about what the kernel did.

## What happened

```
test kernel::user::survey_tests::a_dead_child_is_still_in_the_domain_until_it_is_reaped ...
  user thread 17179869238 killed: vector 14 (page fault)
    rip 0x0000000000400005   addr 0x0000000000a50000   user rsp 0x0000000000501000   err 0x00000004
  the kernel is fine.

[PANIC] panicked at kernel/src/user/survey_tests.rs:381:5:
```

`kernel/src/user/survey_tests.rs:381`:

```rust
assert!(
    super::wait_for(|| sched::rendezvous_waiting_senders(ep) == 1),
    "the child never died onto its supervision rendezvous",
);
```

**The child died.** Its kill message is in the transcript three lines above the panic, at the fault
address `FAULT_STUB` is written to produce. What did not happen is the corpse parking on its
supervision rendezvous's sender queue, where a survey can see it before anyone has collected it.
That is DECISIONS §26's property and `capability::survey_includes`' proved one.

Transcript: `bench/xenon-2026-09-17/nvme-attempt-2-no-controller-found.log`.

## Do not widen the bound

The reflex, after an evening in which two other failures were margins real silicon closed, is to
read this the same way. **`wait_for`'s bound is already two seconds of wall clock:**

```rust
let deadline = crate::arch::timer::now() + 2 * crate::arch::timer::frequency();
```

It yields until then and checks once more on the way out. Two seconds is not a thin margin on a
2.7 GHz machine for a thread already reported dead. **A test made to pass by waiting longer, for
something that should already have happened, has been silenced rather than fixed.** If the
conclusion turns out to be that two seconds is genuinely not enough, that conclusion needs its own
evidence and is a larger finding than this test.

## Three readings, and the transcript separates none of them

1. **The corpse parked and something took it off again.** Then the survey cannot see it either, and
   §26's property is false on this configuration.
2. **The corpse never parked**, because death delivery on a core other than the one running the
   test took a path that does not enqueue the sender.
3. **The count was not 1.** The assertion is `== 1`, not `>= 1`, so a second sender on that queue
   fails it in a way that reads identically in the log.

## The order of work

1. **Print the count the test actually saw**, eliminating reading 3. One line, and it makes every
   future failure here diagnosable from a transcript. This is the general lesson milestone 318 drew
   about assertions against constants, applied to an equality.
2. **Reproduce.** `NIFE_SMP=4` under QEMU is one run and costs nothing. It establishes whether this
   is xenon, four cores, or four *real* cores, and those are three different findings.
3. **Only then investigate delivery**, and only with a reproduction in hand.

## Why it is worth a lane rather than a retry

This is the supervision mechanism. **A corpse a supervisor cannot see is the failure §26 exists to
prevent**, and the test's own doc comment says why both halves live in one case: the domain the
viewer reports and the domain the supervisor may collect from are one set. If that set differs on a
multi-core machine, a property this tree proves and tests holds under emulation and is unverified
where it matters.

**x86_64 running four real cores is new.** §153 flipped `NIFE_SMP` to 2 for the runner and this
machine boots 4, and real cores at real speed is the condition under which this tree has
historically found its scheduler bugs. `design/fatal-risks.md` risk 2 cites the VisionFive 2's
undelivered-wake bug for exactly this: found on a bench, invisible in QEMU, and no proof was
positioned to see it.

## Done when

Either a defect is found and fixed with a test that fails without the fix, or the investigation
establishes that the kernel is correct and the test was wrong, **with the reason written down**.
"It passed on a retry" is not an outcome this block accepts.

## Index row

A child died, the kernel reported it dead, and its corpse never reached its supervision rendezvous
within two seconds. DECISIONS §26's property, on four real cores, and the one failure from xenon's
bench evening that was not an assertion written against QEMU.

## Follow-on

- **None.** This block is the identified work; it has not run yet.
