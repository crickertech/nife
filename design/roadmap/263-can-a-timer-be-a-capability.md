# 263. Can a userspace process hold a timer, on all three architectures?

**Status: NOT-STARTED.** Minted 2026-09-05 by calef, as a spike under his decision that a timed wait
should be served by a userspace timer service rather than by a new kernel blocking primitive.
*(Number provisional until the merge queue lands it.)*

**Gate: NONE.** Reading three specifications and pricing one kernel addition. No hardware, no
decision waiting.

## Why a spike, and the precedent is milestone 106's own

**Milestone 106 was already priced this way**, and it worked: the pricing lane built a prototype (a
`deadline` word on `Thread`, a cached earliest, an expiry walk in `on_tick`, a blocked-thread
census), measured it, and **threw it away on purpose**, because *"shipping it would have settled the
fork by accident."* Those numbers retired the cost objection that had stood since 2026-08-04.

**This is not proposal-shaped procrastination**, which AGENTS.md warns is an available way to avoid
deciding. The direction is decided. What is unpriced is a prerequisite underneath it, and nobody can
currently say what it costs, which is the condition that earns a lane.

## The decision this sits under

calef, 2026-09-05, on milestone 106's fork: **serve the timed wait from a userspace timer service
signalling a notification, rather than from a new kernel blocking primitive.** The reasoning is in
[§101](../decisions/101-notification-objects.md), which anticipated it:

> A notification object lets a *userspace timer process* wake a thread at a deadline, which is how
> seL4 does it, but it requires that timer process to exist and to hold a clock capability.

**All four of milestone 106's named consumers are userspace** (`thread::sleep`, `Endpoint::RECV`'s
callers, the shell's `^C` poll, and `net_stack`), and §101's carve-out for a kernel timed wait names
kernel needs (a watchdog, a scheduling deadline, an in-kernel retransmit) of which **the tree has no
instance**. Both of `sched.rs`'s no-timeout complaints are about userspace callers being hung.

## The problem this spike exists to settle

**A userspace timer service needs a timer of its own**, because it cannot use a timed wait to
implement a timed wait. seL4's driver holds the hardware timer's registers and interrupt, programs
it for the next deadline, and blocks on that interrupt.

**On two of three architectures there appears to be nothing to hold**, from this tree's own headers
rather than from the specifications, which is exactly what the spike must correct or confirm:

| ISA | what the kernel's tick uses | why it may not be grantable |
|---|---|---|
| aarch64 | the ARM Generic Timer | `kernel/src/arch/aarch64/timer.rs`: *"It is **not** an MMIO device. It is **part of the CPU**, reached through system registers... There is no base [address]."* `CNTKCTL_EL1` can open EL0 access, but that is ambient across EL0 rather than held by one process |
| riscv64 | SBI TIME (`sbi_set_timer`, an `ecall` to OpenSBI in M-mode) | one timer per hart, write-only, and U-mode cannot issue SBI calls at all: only S-mode can |
| x86_64 | the local APIC timer | the exception. HPET exists with multiple comparators in MMIO, so a spare grantable timer plausibly exists here |

**[§19](../decisions/19-architectural-parity.md) makes parity a gate**, so an answer that works on
x86_64 alone is not an answer.

## What the spike must produce

1. **Confirm or refute the two negatives from the specifications**, not from this tree's doc
   comments. Can aarch64 grant timer access **per thread** rather than to all of EL0? Is there
   genuinely no U-mode path to arm a timer on riscv64, including Sstc's `stimecmp`?
2. **Price the fourth shape**, below, in the kernel. The pricing lane already measured that any
   deadline structure costs **one comparison per tick** (1.000 comparisons and 0.000 writes over
   100,000 idle ticks, for a scan and for a sorted list), so what is unpriced is the object, its
   methods and the signalling, not the bookkeeping.
3. **Say plainly whether the userspace-service answer survives parity.** If it does not, calef's
   decision changes, and it is better to know that in a day than three days into a lane.

## The fourth shape, which neither milestone 51 nor 106 lists

If the kernel must own the timer on two of three architectures, the service cannot hold one, but the
kernel can signal on its behalf:

```
Timer::ARM(deadline, notification) -> the kernel signals that notification at the deadline
```

A thread then blocks in `RECV` on its endpoint with the notification **bound** to its TCB
([§101](../decisions/101-notification-objects.md), decided 2026-08-20, unbuilt, milestone 151), and
wakes on **either** a message or the deadline. That is milestone 106's own title met.

**It is smaller than any of milestone 51's three shapes.** Not "block until a deadline", only "signal
this at T". `RECV` keeps its signature, nothing becomes ambient, and the authority to wait on time is
a capability like everything else. It is also close to what seL4 does, with the timer's ownership
moved to the only place two of our three architectures allow.

**This block does not recommend it.** It is named so the spike prices it beside the userspace
service, and because a fork should not be settled by discovering an option late.

## BUGS

- **Three headers are not three specifications.** The table above is read from this tree's own
  comments, which is how the spike was scoped and is not evidence about the hardware. Item 1 exists
  because the maintainer's reading is the thing most likely to be wrong here.
- **It prices nothing about a timer service itself**, only whether one can hold a timer. If the
  answer is yes on all three, the service is still unbuilt, unpriced, and unnamed.
- **The fourth shape is a syscall-surface addition** whatever its size, so it is calef's under §10
  and this spike may only measure it.
- **Milestone 151 is unbuilt**, so every option here that composes with a notification composes with
  something that does not exist yet, and the spike should say what it assumes about it.
