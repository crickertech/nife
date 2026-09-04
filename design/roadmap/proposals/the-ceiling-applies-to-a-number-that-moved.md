# DECISIONS §144's ceiling names two terms and milestone 188 changed both

**Status: PROPOSED 2026-09-04.** Found by the milestone 188 lane the same day §144 landed. Written as
a proposal rather than as an edit to the decision, because `design/decisions/` is not a lane's to
amend; this is the maintainer's or calef's to fold in.

**Gate: NONE.** Nothing is owed and nothing is blocked. The ceiling still fires rarely and 16 KiB
still stands. What is stale is the sentence naming what the ceiling is applied *to*.

## In brief

§144 sets an absolute ceiling of 16 KiB per architecture "on the sum of `ipc_fastpath` and
`syscall_entry`", and derives the number from `notes/target-hardware.md`'s 32 KB L1i requirement. It
records the totals it was measured against as **aarch64 9,156, x86_64 8,404, riscv64 7,174**.

Milestone 188 phases 1 to 3 (pull request #732) moved both terms, on the same day:

- **`ipc_fastpath` measured the SEND/RECV shape**, which essentially no service in this tree runs.
  The gate now reports `ipc_send_recv` and `ipc_call_reply` separately and keeps `ipc_fastpath` as
  the **worse of the two**, since one round trip is one shape or the other.
- **aarch64's `syscall_entry` counted all sixteen exception vector entries** where an `svc` fetches
  one, so 1,892 of its 3,304 bytes were never fetched.
- Phase 3 then took 6 to 10% off both closures by outlining cold arms.

## What the sentence should say

**The ceiling's subject is `max(ipc_send_recv, ipc_call_reply) + syscall_entry`**, which is what
`script/fastpath-footprint` now prints as `total`. That is the same claim §144 already makes,
evaluated on numbers that are true.

| | §144, as recorded | after milestone 188 | fraction of 16 KiB |
|---|---|---|---|
| aarch64 | 9,156 | 8,536 | 52% |
| riscv64 | 7,174 | 7,764 | 47% |
| x86_64 | 8,404 | 9,759 | **60%** |

**x86_64 is the one to watch.** Its total rose, because counting the shape services run adds more
than any other correction removes there, and it is now 60% of the ceiling against the 51% §144
recorded. §144's own reasoning was that the ceiling "sits about 75% above the largest and fires
rarely"; it now sits 68% above the largest. Neither this proposal nor the milestone asks calef to
move the number.

## Why it is worth writing down rather than leaving

§144's `BUGS` already says the measurement is "an upper bound and a loose one" and that "the bytes an
IPC actually touches are fewer and nobody knows by how much". Milestone 188 narrowed that by a
measured amount on one architecture and widened what is counted on all three, and a decision whose
recorded headroom is 9 points optimistic on x86_64 is the kind of stale record this tree keeps
finding by accident.

## Where it came from

`design/roadmap/188-ipc-fastpath.md`'s Follow-on.
