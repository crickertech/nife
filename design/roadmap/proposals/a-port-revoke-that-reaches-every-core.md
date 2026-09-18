# A port revoke that reaches every core

**Status: PROPOSED 2026-09-17.** Raised by milestone 313's security audit (finding 4).

**Gate: NONE.** The mechanism it needs, an IPI that runs a core-local step on every online core and
waits, already exists for the TLB (notes/x86-tlb-shootdown.md); the core-local step already exists
(`arch::x86_64::segments::revoke_installed_port_grant`, written for exactly this broadcast). What is
missing is the call between them and a two-core test.

**Promoted:** minted as **milestone 315** by calef on 2026-09-17, the day milestone 313's security
audit raised it as finding 4. The record is
[design/roadmap/315-port-revoke-every-core.md](../315-port-revoke-every-core.md), which is where the
work is tracked; it is `NOT-STARTED`, so this is a promotion and not a completion. The status line
above keeps its original date because that is what makes the pile measurable.

## What is being proposed

`PortRange::REVOKE` (and `sched::delete_current_cap`'s port half) broadcast
`revoke_installed_port_grant(base, count)` to every online core after clearing the cached grants, so
a revoked holder that is running on another core faults on its very next `in`/`out` rather than on
its next context switch.

## Why

DECISIONS §152's BUGS, and two kernel comments beside the revoke path, said x86 runs one core and the
core-local reset was therefore the whole machine. `smp::seat_cpus_from_acpi` made that false (the
tour boots two cores under OVMF), and the reset stayed core-local. So today a revoked holder running
on a second core keeps that core's TSS bitmap until its next switch, at most one tick, during which
its `in`/`out` succeed against a capability that no longer exists. The audit accepted the window with
its reason (bounded, cannot reopen, no consumer holds a port on two cores) and recorded it at the
function; this is the closing move.

## What it costs, and the test that is the point

The IPI path is the TLB shootdown's and the reset is a `#[cold]` function, so the cost lands only on
a revoke, which is a driver-replacement event and not a fast path. The test is what earns the
milestone: a holder pinned to a second core spinning on `out`, a revoke from the first, and the fault
arriving before the second core's next tick. That is the first port test that runs on two cores, and
the first that can see the window at all; the three that exist run on one.
