# 254. A caller stranded by a server that died is stranded forever, and nothing records that as intended

**Status: NOT-STARTED.** Split out of milestone 133 (ending a permanently blocked thread, and
deciding who may) on 2026-09-03 by calef, who took this half first on the argument that it is a
defect rather than a fork. *(Number provisional until the merge queue lands it.)*

**Gate: NONE.** No new authority, no new syscall, and the authorities it rides on are already
exercised.

**In brief.** `abi::Error::Gone` does not reach a reply-parked caller. The abort machinery walks an
endpoint's wait queues, but a caller whose request was *taken* was popped off at the rendezvous; it is
woken by `sched::ipc_reply` and by nothing else. **So a caller stranded by a server that merely died
stays blocked for the life of the machine.**

**Nothing in this tree records that as intended**, which is what makes it a defect rather than a
design. QNX Neutrino has not permitted it since the 1990s:

> If the server thread fails, exits, or disappears, the client thread becomes READY, with MsgSend()
> indicating an error

Zircon reaches the same outcome by a different route (closing the channel, rather than touching the
thread). Both are in notes/blocked-thread-teardown.md's survey, and **nobody in that survey blocks
forever with no way out.**

## Why this is separable from milestone 133, and why it goes first

Milestone 133 is a genuine fork: *which held capability expresses the right to end a thread*, with
four proposals conceding different things, and it touches the syscall surface under one of them. This
is proposal C's first piece and it is the only part of that milestone that **asks for no authority
anybody does not already hold.** It rides on destroying a region or destroying an endpoint, both of
which the kernel already witnesses, and changes only their *reach*.

The note says so in as many words:

> It is the only proposal here that is plainly complementary to the others rather than an alternative
> to them

So it can be taken with A, with B, or alone, and taking it first costs milestone 133 nothing.

**It does not solve 133's capacity problem and must not be quoted as if it did.** A hung component's
own region stays unreclaimable. What this halves is the compounding: today one hang strands every
caller in flight and each stranded caller is itself an unreclaimable region, so a service with many
callers costs one region per caller. After this, one hang costs one region.

## The hazard this cannot ship without solving

**Waking a reply-parked caller is exactly the path that opens the stale reply capability**, and
milestone 133's block states it as a hazard for every proposal:

`cap::reply_cap` mints `Object::Reply(tid)`, whose payload is a generational thread name with **no
call identity**. `ipc_reply`'s guard checks the `WaitRole` and discards the endpoint, and that is
sound **only** because nothing today can leave a reply park and enter a second `CALL` while an
unconsumed `Reply` names it. This milestone creates that path. A hung server's stale reply capability
would then forge an answer to a later, unrelated call, and L4Re documents the identical hazard as a
consequence of its own finite receive timeouts.

**Two known fixes, both seL4's**, and this milestone must take one and say which: delete the
outstanding reply capability at abort (a cspace sweep, the pattern `sched::delete_frame_caps` already
establishes), or do not wake the victim at all. The second is not available here, because waking the
victim is the entire point.

**The sweep is not optional and is the larger half of the work.** A block that shipped the wake
without it would trade a permanent block for a forgeable reply, which is a worse defect in a
capability system than the one it fixes.

## What the mechanism already provides

Milestone 133's research established that the kernel can do this and only the authority was open.
For this half the authority question does not arise, so what remains is mechanical:

- **Finding the reply-parked callers of an endpoint.** Their `wait_on` records
  `(ep, WaitRole::Reply)`, so a scan over `MAX_THREADS` answers it.
- **Waking them.** `set_ipc_aborted` plus `wake` is the existing abort-and-resume pair, already used.
- **The trigger.** The destruction of the server's region or of the endpoint the call went to, both
  already witnessed.

## The proof that this milestone worked

**A caller whose server is destroyed mid-`CALL` returns `Gone` rather than blocking forever**, proved
by a test that hangs today, and **a test that a stale `Reply` capability cannot answer a later call**,
proved by attempting exactly that and being refused.

The second is the one that matters. Without it this milestone has traded one defect for a worse one,
and a test asserting only the first would pass while that were true.

## BUGS

- **It does not reclaim the hung component's region**, which is milestone 133's capacity problem and
  stays open. This halves the leak; it does not close it.
- **A scan over `MAX_THREADS` per destruction is a linear cost** on a path that is not hot but is not
  free either, and nothing here measures it. `script/bench`'s icount tripwire is where it would show.
- **It says nothing about a server that is alive and simply never replies**, which is the case that
  motivated milestone 133 and is answered by a deadline on `CALL` (milestone 106's fork, not this
  one's) or by ending the thread (133's).
- **Milestone 133's own block and note both still say `Tcb`**, a name DECISIONS §113 retired on
  2026-08-23 in favour of `ThreadControlBlock`. They predate the rename and were never swept, because
  `script/roadmap`'s staleness gates reach `BUILT`, `REMOVED` and now `PARTIAL` blocks, and 133 is
  `NOT-STARTED`.
