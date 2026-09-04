# A `Reply` capability names a thread and not a call, and only a sweep keeps that sound

**Status: PROPOSED 2026-09-04.** Written by the milestone 133 lane, from that milestone's block and
from notes/blocked-thread-teardown.md's first open question.

**Gate: NONE.** No decision is owed for the proving half. Giving the capability a call identity
*would* be a fork (it changes what `cap::reply_cap` mints and what `ipc_reply` checks), so a lane
should prove first and bring the widening as a proposal with the proof in hand.

**In brief.** `cap::reply_cap` mints `Object::Reply(tid)`, whose payload is a generational thread
name and nothing else: no call sequence number, no rendezvous, no nonce. `sched::ipc_reply`'s guard
checks the caller's `WaitRole` and **discards the rendezvous**. Nothing in this tree proves that
combination is sound; what stands in for a proof is an argument by exhaustion over the exits from a
reply park, written by hand in a note. Lift the guard into a pure function the way
`sched::region_reap_verdict` was lifted, and prove it.

## Why the argument is worth converting into a proof

The soundness claim is *"nothing can leave a reply park and enter a second `CALL` while an
unconsumed `Reply` names it"*, and it is a claim about every path in the kernel rather than about
one function. The research established it by reading `cap::reply_cap`, `crates/generational_table`'s
naming, and `ipc_reply`, and by enumerating the three ways out of a reply park; its own BUGS section
is honest that this is an argument and that **no test in this tree would catch a regression**,
because no path can currently produce the dangerous shape at all.

Milestone 133 is why that matters now rather than eventually. It ships the first code that ends a
reply-parked caller, and it defends the claim twice over: the victim is never woken (seL4's
`ThreadState_Inactive`, so no second `CALL` can exist), and every outstanding `Reply` naming it is
swept out of every capability table anyway (seL4's `cteDeleteOne(callerCap)`).
`user::force_kill_tests::tearing_down_a_reply_parked_caller_sweeps_the_reply_capability` pins the
sweep. **What no test pins is the guard the sweep is protecting**, and a later change that made a
caller resumable would re-open the hazard somewhere the sweep does not reach. L4Re documents the
identical hazard as a consequence of its own finite receive timeouts, and Zircon documents it for
`zx_channel_call`; milestone 106's timed wait would bring this tree the same trigger.

## The two pieces, and they are separable

1. **Lift and prove the guard.** Extract the decision `ipc_reply` makes about whether a `Reply` may
   be delivered into a pure function over the caller's recorded wait, and give it Kani harnesses.
   That makes the rule statable, testable and falsifiable, which the argument in the note is not.
2. **Then ask whether the payload should carry a call identity**, which is the actual widening and
   the actual fork. The cost is a wider `Object::Reply` and a check that costs a comparison; the
   benefit is that the guard stops depending on a whole-kernel reachability argument. Answer it with
   the proof from piece 1 in hand, not before.
