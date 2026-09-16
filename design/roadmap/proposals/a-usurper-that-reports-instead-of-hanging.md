# A usurper that reports instead of hanging, so row 26 can be falsified

**Status: PROPOSED 2026-09-16.** Found by milestone 305, which wrote the honest defect for
`notes/confinement-claims.md`'s row 26 and could not use the result.

**Gate: DECISION.** It needs a non-blocking or timed receive, which is the syscall surface
(AGENTS.md: anything two programs agree on, and §10/§16's narrow boundary), so it is calef's before
it is anyone's.

## In brief

Row 26 is *"a client of a rendezvous cannot become its server."* Its test,
`kernel::user::live_swap_tests::a_client_of_the_stable_rendezvous_cannot_become_its_server`, hands an
attacker exactly what the honest client holds and requires `NotPermitted` when it tries `RECV_CAP`.
The kernel enforces that in three lines of `kernel/src/syscall.rs`.

**Deleting those three lines, which is the complete break of the claim, does not fail that
assertion.** Measured 2026-09-16 on aarch64: the run came back as a **60-second watchdog reading "no
progress ... a lost-wakeup hang"** with a thread dump, and nothing in it about impersonation.

The reason is structural rather than a flaw in the patch. `RECV_CAP` is a **blocking** receive, so an
attacker the kernel fails to refuse does not come back and report an escape. It takes the message the
honest server was waiting for, or parks on the rendezvous, and the run deadlocks. The assertion that
states the claim is reachable only in the world where the kernel *does* refuse.

So the row is `unfalsified` and the test has never been shown able to fail.

## Why this is worth a block rather than a `BUGS` line

It already has a `BUGS` line, in `notes/confinement-claims.md` and in the test's own
`Falsification: unfalsified` record. §71's promotion trigger is met by the second half: this is not a
limitation of the system, it is a **confinement test that cannot fail**, which is the exact defect
milestone 305 found once already on riscv64 and fixed. One instance is a bug; two is a pattern with a
cause, and the cause here is a missing primitive rather than a mistake anyone made.

## What it would take

Milestone 202 solved the same shape for §31 by adding `wait_for_report`, a bounded wait, so that a
stall is reported where it means something instead of surfacing 234 seconds later as a watchdog. The
equivalent here is that the usurper's attempt must **return**, so `chatty`'s `usurp()` can report
"the kernel let me receive" as a verdict rather than blocking on it.

`swap_protocol::try_recv_cap` is named `try_` and is not one: it invokes `RECV_CAP` straight, and
`RECV_CAP` blocks. That naming is itself worth calef's attention.

Options, costed as far as this lane could without building them:

1. **A non-blocking `RECV_CAP`**, or a flag on it. Smallest kernel change; a new syscall behaviour
   every future program is written against, which is exactly the category AGENTS.md calls expensive.
2. **A timed receive.** §106's block already priced a timed wait and found the mechanism about thirty
   lines, with the authority question the whole problem. That work is adjacent.
3. **Leave the syscall alone and give the attacker a watchdog thread**, so the *test* bounds the wait
   rather than the kernel. No surface change and no benefit to anything but this test, which is the
   argument against it: the same gap will be hit by the next component that wants to ask a rendezvous
   a question it might not be allowed to ask.

**Recommendation: none**, deliberately. AGENTS.md draws the line at the syscall surface, and a
recommendation there is most of the decision already made.

## What is blocked until it is answered

Row 26 of `notes/confinement-claims.md` stays `unfalsified`, and `design/fatal-risks.md`'s risk 7
keeps one claim whose test has never been shown able to fail. Nothing else waits on this.
