# 153. How a two-core x86_64 test earns its place, when two-core x86_64 is not yet trustworthy

**Status: PROPOSED.** Raised 2026-09-17 by the maintainer while minting milestone 315, after asking
calef the same question three times in one session without it reaching a file. That is the failure
AGENTS.md names outright: *open decisions live in a file, not in a conversation*, and five
accumulated in chat scrollback on 2026-08-04 while the milestone abolishing that medium was being
built. This file is the correction.

## What is being decided

**Milestone 315's test needs two cores on `x86_64`, and this project's `x86_64` runs one by
default, on purpose.** The decision is how to sequence that.

## Why one core is the default

`scripts/qemu-runner-x86_64.sh` sets `SMP="${NIFE_SMP:-1}"`, and its comment is specific about why.
The original crash that held it at 1 is **fixed** (a missing cross-core TLB shootdown). Two reasons
remain open, both recorded in `arch::x86_64::ap_boot`'s own `BUGS`:

- **#1**: AP-bring-up flakiness at three or more cores.
- **#3**: a boot-core-identity bug that makes `smp::tests::every_secondary_runs_scheduled_work`
  fail **about half the time at two**.

Separately, `design/roadmap/proposals/the-uefi-boot-gate-asserts-two-cores-that-do-not-always-start.md`
records `script/test`'s UEFI leg failing its two-core assertion **one run in three**.

So the substrate a two-core port test would run on is one where an existing SMP test is already
unreliable.

## The options

**1. Order milestone 315 behind `ap_boot`'s BUGS #1 and #3.** The test lands on a configuration
somebody trusts. A confinement test is only worth having if a red is believed, and a red on a flaky
substrate is not. The cost: 315 waits on SMP work nobody has scheduled, and the one-tick revocation
window stays open in the meantime.

**2. Give milestone 315 an opt-in `NIFE_SMP=2` leg**, separate from the default suite. The test
exists now, and it can see something nothing else can: the three port tests that exist all run on
one core and cannot observe this window at all. The cost: a test that may be red for reasons
unrelated to what it tests, which is the exact condition under which a test gets muted.

**3. Both, ordered.** Build the broadcast and the test now behind option 2's leg, and treat option
1's SMP work as what promotes the leg into the default suite. Costs an explicitly two-tier suite,
which this tree does not have today and would have to be willing to keep honest.

## What makes this genuinely hard rather than a scheduling preference

**This tree has spent two days finding confinement tests that could not fail.** Milestone 305 found
`the_page_tables_say_u_mode_cannot_read_the_kernels_memory` passing with the `U`-bit check removed
outright. Milestone 307 found six more whose quotable assertion cannot run. Milestone 313 found
milestone 299's two port tests hanging rather than going red.

Option 2 adds a test to that family's neighbourhood: one whose failures are ambiguous between "the
claim broke" and "the substrate is flaky". Option 1 avoids that and pays by leaving the claim
untested for longer, which is the same trade that let the `U`-bit test sit vacuous for eight months.

**Neither option is obviously right, which is why this is calef's and not a lane's.**

## Recommendation

**Option 3**, weakly held, and the weakness is worth stating rather than hidden. The reason to
prefer it is that the test's value is diagnostic rather than regression-catching on day one: it is
the first thing in this tree that can observe the revocation window at all, and measuring the
window's width would replace milestone 313's reasoned "at most one tick" with a number. The reason
to distrust the recommendation is that a two-tier suite is a thing somebody must keep honest, and
this tree's own evidence is that the lower tier is where tests go quiet.

## What is blocked until it is answered

Milestone 315, which is `NOT-STARTED` and gated on this. Nothing else.
