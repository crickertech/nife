# The migration drain and the tick budget, 2026-08-18

*(An appendix of [notes/load-sensitive-assertions.md](../load-sensitive-assertions.md), which holds
the register and the rules. Moved there verbatim on 2026-09-24; "this page" and "above" in the text
below meant the single note it came from.)*

## The migration drain, 2026-08-18: a budget denominated in what the guest actually got

The section above found a new member of the family, said what the fix was, and did not take it:
`smp.rs`, `a_migrated_kernel_thread_keeps_its_hart_pointer`, red twice in eighteen loaded runs with
`migration workers never drained (59/60 done)`. This lane took it. The verdict is the predicted one,
which is worth saying out loud because on this page the predicted verdict has been wrong twice: the
failure is honest positive-direction load sensitivity, the assertion wants the right thing, and only
its **unit** was wrong.

### What it proves, what the budget was, and what the budget is

The claim is one line, re-checked on every turn of the drain: no worker ever read a per-CPU pointer
naming a hart other than the one it was running on. Everything else in the test exists to *drive*
that check, by keeping more cpu-bound kernel threads alive than there are harts so that idle harts
steal already-preempted ones.

The drain loop has to end, so it carried a second assertion nobody wrote down as a claim: **60
workers finish inside two seconds of `timer::now()`**. That is a claim about the host. `timer::now()`
is the guest's view of a counter that follows the host clock and advances whether or not the guest
is executing, so under contention the budget is spent in wall clock the guest never received. Each
worker is about 40 ms of it. One worker short of sixty is exactly what that deficit looks like at
the margin.

The budget is now **200 delivered timer ticks** (`2 * TICK_HZ`, the same two seconds on a quiet
machine). A descheduled emulator *misses* deadlines rather than gaining them, so fewer ticks arrive
per second of wall clock and the budget stretches under precisely the condition that made the old
one wrong. This is the drift twins' move (assert the law, not the rate) spent on a wait, and the
mechanism was already in the tree: `sched::tests::within_ticks` had done it for
`a_thread_that_never_yields_is_preempted_anyway` in the second round.

**What was shared is the arithmetic, not the loop**, and the distinction is why this is a
`testing::TickBudget` rather than a second `within_ticks` caller. Those two waits poll differently
and both are right to. `within_ticks` must not yield, because the thing it waits for is a preemption
of the waiting thread. This drain must yield, because it shares a hart with the workers it is
draining, and it checks a *second* assertion (`BAD`) on every turn, so it cannot be expressed as a
condition handed to somebody else's loop at all. What the two have in common is the budget's
arithmetic and its re-anchoring on migration, and that is now written once.

### The injections, and the first one was too coarse to reach its own target

Every injection was reverted. The middle row is the useful one.

| injection | what it is | result |
|---|---|---|
| `trap_return` restores the frame's `tp` on an S-mode return, unconditionally | the pre-fix stale-tp bug, exactly (DECISIONS §28) | **never reached the target test**: red ~40 tests earlier at `a_finished_thread_is_reaped_and_its_memory_returned`, with a rank-45-over-rank-60 lock-order violation |
| the same, gated on a `no_mangle` flag the migration test arms and nothing else | the defect arranged inside this assertion's own window | **red**, `a migrated kernel thread read a per-CPU pointer for the WRONG hart: a stale tp rode a hart migration`, at the drain loop's own line |
| a wave whose workers stop counting themselves done at five | a drain that genuinely cannot finish | **red**, `migration workers never drained (5/12 done) within 200 delivered ticks` |

**The first row is the fifth round's lesson arriving in a new subsystem.** That round found that a
whole-region frame injection "is too coarse to test anything downstream of it, so a leak injection
aimed at a late test has to be one frame wide". The same is true one layer down: a per-CPU pointer
that goes stale on *every* S-mode return corrupts the scheduler from the first migration onward, and
the suite dies of it long before the test written to name it. It is a red run, and it is a red run
that says "lock order" rather than "stale tp", which is the diagnosis cost this milestone exists to
remove.

So the useful injection is the narrow one, and narrowing it took a flag in the assembly. That is
worth knowing before anyone tries to prove this test again: **the honest injection for a
whole-machine invariant is the invariant broken inside one test's window**, not the invariant broken.

**The third row is the half that a budget change most needs and is easiest to skip.** Re-denominating
a bound is one edit away from widening one, and the difference is only visible if you have watched
the new bound fail. It fails, it fails on the first wave, and it prints the count and the unit.

### What the drain actually costs, measured

Six waves per leg, both ISAs, four full loaded runs, with the drain instrumented to print what it
spent (the instrument was reverted; it is not in the shipped tree). The measurement was taken on
**this branch merged with `milestone/62-assertion-disposition`**, because on `main` at these loads
the suite dies at `ticks_arrive_at_the_configured_rate` before it ever reaches `smp.rs`: 3 of 3 runs
failed there in about 60 s each. That is the sibling lane's assertion and its fix, not this one's,
and it is stated here so the tree the numbers came from is not a guess.

| leg | worst wave, wall clock | worst wave, delivered ticks | margin on the old budget (2000 ms) | margin on the new (200 ticks) |
|---|---|---|---|---|
| aarch64 | 316 ms | 41 | 6.3x | 4.9x |
| riscv64 | **1005 ms** | 30 | **2.0x** | 6.7x |

**The riscv64 row is the whole argument, and it is one wave rather than an average.** That wave took
a second of wall clock and was charged **ten ticks** in the run it happened in: at 100 Hz a second is
a hundred tick periods, so the emulator was descheduled through roughly nine tenths of it and the
guest executed for about a tenth of the second the old budget was spending. Two more of those and
the old bound is gone, which is what `59/60 done` was. The tick bound does not move, because the
same starvation that stretches the wall clock is what stops the ticks arriving.

**And the instrument caught the hazard the budget type is built around, without being asked to.**
Four of the 48 wave measurements printed a *wrapped* tick delta (`18446744073709551587`, which is
-29): the test thread was stolen onto another hart between the two reads, and `ticks()` is per core
(§11), so the subtraction compared two unrelated counters. That is the second round's `ticks_on`
finding arriving in a print statement, and it is exactly why `TickBudget` re-anchors on a change of
core rather than subtracting across one. A naive tick budget would have had the bug it was written
to avoid, in 8% of its samples.

### BUGS

- **Under load this test loses coverage quietly rather than going red, and that is the cost of the
  fix rather than a defect it removes.** Each worker's lifetime (`life`) is still `frequency() / 25`,
  a span of *counter* time, so a worker whose vCPU is descheduled through most of its 40 ms burns
  the span without executing much: it is preempted fewer times, acquires a saved frame less often,
  and drives less of the migration the test exists to provoke. The failure direction is safe (the
  test passes having proven less, it does not fail having proven nothing), which is why this is
  recorded rather than fixed here, and it is the same "an assertion whose sensitivity is set by its
  neighbours" shape the fifth round found in the reaper test, arriving through the host instead of
  through an upstream test. Denominating `life` in delivered ticks too is the obvious fix and is not
  obviously right: it would stretch the workers *and* the drain under load, and the product of the
  two is what the harness's 90 s per-test ceiling has to hold, where today only one factor grows.
  Whoever takes it should measure against the table above rather than reason about it.
- **A `TickBudget` that re-anchors on every check never expires**, by construction: a thread that
  changed core is given its budget back. Nothing in the tree can make that happen (a steal moves a
  thread far less often than once per poll), and the type is deliberately not the last line of
  defence: the harness's per-test wall-clock ceiling fails the run whatever the ticks say. The
  alternative, subtracting across a migration, compares two unrelated per-core counters, which is
  the defect `ticks_on` was added for in the second round.
