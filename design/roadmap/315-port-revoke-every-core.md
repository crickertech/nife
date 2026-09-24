# 315. A port revoke that reaches every core

**Status: NOT-STARTED.** Promoted from
`design/roadmap/proposals/a-port-revoke-that-reaches-every-core.md` by calef on 2026-09-17, the day
milestone 313's security audit raised it as finding 4. *(Number provisional until the merge queue
lands it.)*

**Gate: NONE.** It was `DECISION` until 2026-09-18, when calef answered
[§153](../decisions/153-two-core-x86-test-sequencing.md): **close this milestone first, then default
`NIFE_SMP` to 2.** A lane can start today.

**And the scope shrank while the decision was open.** This block said the test was the deliverable
and would be the first port test able to observe the revocation window. **That test already exists.**
Milestone 316 fixed `ap_boot`'s BUG #3 (the boot-core-identity defect that failed
`every_secondary_runs_scheduled_work` about half the time at two cores), and with a working
substrate `a_revoked_holder_faults_on_its_next_port_write` became the observer: of 12 two-core runs,
**all seven failures are that one assertion**, `left: 2, right: 1`, the revoked holder's `out`
succeeding.

So this milestone owes **the broadcast, and the default flip**:

1. `PortRange::REVOKE` and `sched::delete_current_cap`'s port half broadcast
   `revoke_installed_port_grant` to every online core after clearing the cached grants.
2. **Flip `NIFE_SMP`'s default from 1 to 2 as this milestone's closing step.** §153 is explicit that
   the flip is not a separate judgment: the two-core suite going green *is* the verification that
   the broadcast worked. Leaving it to be remembered is how it would not happen.

## What this is

`PortRange::REVOKE`, and `sched::delete_current_cap`'s port half, broadcast
`arch::x86_64::segments::revoke_installed_port_grant(base, count)` to every online core after
clearing the cached grants, so a revoked holder running on another core faults on its very next
`in`/`out` rather than on its next context switch.

**The pieces already exist.** The IPI that runs a core-local step on every online core and waits is
the TLB shootdown's (`notes/x86-tlb-shootdown.md`). The core-local step was written for exactly this
broadcast. What is missing is the call between them, and the test.

## Why it is open at all

[DECISIONS §152](../decisions/152-port-range-capability.md)'s `BUGS` said x86 ran one core, so the
core-local reset was the whole machine. `smp::seat_cpus_from_acpi` made that false: the tour boots
two cores under OVMF and **xenon booted four on 2026-09-17**. The reset stayed core-local.

So a revoked holder on a second core keeps that core's TSS bitmap until its next context switch,
**at most one tick**, during which its `in`/`out` succeed against a capability that no longer
exists. Milestone 313 **accepted** that window with its reason (bounded, cannot reopen, no consumer
holds a port on two cores today) and recorded it at the function. §152's entry was corrected the
same day. This is the closing move.

## The decision this is gated on

**The test is the deliverable, not the broadcast**, and the test needs two cores on x86_64. That is
where the problem is.

`helpers/qemu-runner-x86_64.sh` sets `SMP="${NIFE_SMP:-1}"` **deliberately**, and its comment gives
two open reasons: AP-bring-up flakiness at three or more cores, and a boot-core-identity bug that
makes `smp::tests::every_secondary_runs_scheduled_work` fail **about half the time at two**. Both
are recorded in `arch::x86_64::ap_boot`'s own `BUGS` (#1 and #3). Separately,
milestone 412 (design/roadmap/412-the-uefi-boot-gate-asserts-two-cores-that-do-not-always-start.md)
records the UEFI leg's two-core assertion failing one run in three.

So a new two-core port test would ride on a configuration where an existing SMP test is already
flaky. **Two ways to sequence it, and they are not close enough to pick without calef:**

1. **Order this behind `ap_boot`'s BUGS #1 and #3.** The test lands on a two-core configuration
   somebody trusts, which is what makes a confinement test worth having. The cost is that this
   milestone then waits on SMP work nobody has scheduled, and the window stays open meanwhile.
2. **Give it an opt-in `NIFE_SMP=2` leg**, separate from the default suite. The test exists now and
   can see a window nothing else can. The cost is a test that may be red for reasons unrelated to
   what it tests, which is the exact condition under which tests get muted, and this tree has spent
   2026-09-16 and 2026-09-17 finding three separate confinement tests that could not fail.

**Recorded rather than decided** because the maintainer asked three times in one session and it
never reached a file, which is the failure AGENTS.md names: *open decisions live in a file, not in a
conversation.*

## What the test has to do, whichever way it is sequenced

Written down now because it is the part most likely to go wrong, and because milestone 313's own
finding 2 is the warning.

- **The assertion is the fault arriving**, positively observed. Not the absence of a successful
  `out`. Milestone 299's two port tests were found to hang rather than go red precisely because a
  wrongly-permitted `out` was followed by a `SEND` nobody received, and a draft of 313's finding 1
  hung the same way.
- **Three outcomes must be distinguishable**: fault seen, no fault before the deadline, and the
  other core never scheduled the holder. Only the first two are about revocation, and a test that
  cannot tell the third apart will be read as flaky when it is uninformative.
- **A bounded wait, not a race.** The window is one tick. One shape that converts the race into an
  observation: have the holder report a monotonic count of successful `out`s and assert the count
  **stops advancing** within a bounded window after the revoke. That also measures the width of the
  window rather than only its existence, which would put a number on the "at most one tick" the
  audit accepted on reasoning.

## Follow-on

- **Decision.** `design/decisions/153-two-core-x86-test-sequencing.md`, **DECIDED 2026-09-18**:
  close this milestone first, then default `NIFE_SMP` to 2, so the flip verifies the broadcast
  rather than being a judgment somebody has to keep making.

## Index row

`PortRange::REVOKE` clears the capability table and the cached grants, but the TSS I/O bitmap reset
is core-local, and `smp::seat_cpus_from_acpi` made x86 multi-core after DECISIONS §152 recorded that
it was not. So a revoked holder running on another core keeps COM1 for at most one tick. Milestone
313's audit accepted that window with its reason and recorded it at the function; this broadcasts
the existing core-local reset over the existing TLB-shootdown IPI so the fault arrives on the next
`in`/`out` instead. The test is the deliverable rather than the broadcast: it would be the first
port test to run on two cores and the first able to observe the window at all, which is why its
sequencing against `ap_boot`'s open SMP bugs is a decision rather than a scheduling detail.
