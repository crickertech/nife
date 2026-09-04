# A block site that writes `Blocked` by hand opts out of teardown, silently

**Status: PROPOSED 2026-09-04.** Written by the milestone 133 lane, from that milestone's block.

**Gate: NONE.** No decision is owed. It is a lint or a type change in
`crates/thread_wake_handshake` plus its callers in `kernel/src/sched.rs`, and it wants a lane rather
than a hotfix because the interesting half is deciding which rung of the ladder it can reach.

**In brief.** `Handshake::park` writes `state = Blocked` and `wait_on = Some(..)` in one statement,
and the pair is what every teardown path reads to find the queue a thread is linked on. The fields
are `pub`, because the kernel has legitimate out-of-protocol writers, so **a future block site can
write `state = Blocked` directly and leave `wait_on` holding whatever the last wait left there.**
`thread_wake_handshake`'s own BUGS says nothing prevents it. Make it impossible, or make it fail
loudly at the write rather than at the read.

## Why this matters more since milestone 133

Before that milestone the consequence of a stale `wait_on` was a diagnostic one: a hang dump saying
the wrong thing. `sched::finish_blocked_resident` now *acts* on it. It resolves the recorded
rendezvous name and unlinks the thread's TCB from that rendezvous's queues by pointer, and then the
region's reclaim frees the page the TCB sits on. If the name is stale, the unlink runs against the
wrong rendezvous, finds nothing, and the reclaim leaves **a freed page still linked into a live wait
queue**, which the next `recv` on that rendezvous follows. That is a use-after-free in the IPC path,
and it is the sharpest failure mode the research (notes/blocked-thread-teardown.md) named for every
one of its four proposals.

Milestone 133 bought what a caller can buy on its own and no more. It asks **both** queues rather
than trusting the recorded `WaitRole`, which it must, because a `CALL` caller that met no server is
recorded `Reply` and is genuinely on the sender queue; and it carries a `debug_assert!` pairing
`Blocked` with a recorded wait. Neither defends the *name*, and a `debug_assert` is rung two on a
release board.

## What the options look like, so a lane starts from the fork rather than at it

- **Rung one, and it is the reason to look:** make the pair unrepresentable. `state: RunState`
  and `wait_on: Option<W>` are two fields that must agree; one field carrying `Blocked(W)` cannot
  disagree with itself. The cost is that `RunState` is loom-searched and matched on in many places,
  and the out-of-protocol writers the fields are public *for* would all have to be looked at, which
  is the work.
- **Rung two:** a `script/lint` check that no file outside `thread_wake_handshake` assigns
  `RunState::Blocked`. Cheap, greppable, and it fires without being remembered; it is also exactly
  the shape AGENTS.md prices at `git grep -w TODO`'s false-positive rate, so it wants checking
  against the real writes before it is written.
- **Rung two, narrower:** promote the `debug_assert!` in `finish_blocked_resident` to a real refusal,
  so a resident whose wait is unrecorded is refused rather than unlinked from a guess. That is the
  smallest honest change and could ship on its own, ahead of either of the above.

## Where it came from

Milestone 133's Follow-on, and before that `crates/thread_wake_handshake`'s own BUGS section and
notes/blocked-thread-teardown.md's, which names it as the failure mode proposals A, B and C all
inherit.
