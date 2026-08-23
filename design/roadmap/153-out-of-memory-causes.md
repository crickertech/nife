# 153. `OutOfMemory` collapses three distinct causes into one error code

**Status: NOT-STARTED.** Found 2026-08-22 while costing milestone 49's attribution fork: `Untyped::SPLIT`'s
own doc comment already admits it (`crates/abi/src/lib.rs`), but nothing downstream lets a caller,
or a person debugging one, tell the three apart.

**Gate: DECISION.** Distinguishing the causes touches the syscall ABI's `Error` enum, which
`crates/abi` and every caller share; whether the fix is new `Error` variants, a separate diagnostic
query, or something else is a fork worth deciding rather than guessing at.

## The gap, checked at the source rather than assumed

`Untyped::SPLIT` (create a new region) returns `Error::OutOfMemory` for three unrelated reasons,
by its own documentation: *"`OutOfMemory` when the budget or region table is exhausted or the
cspace is full"* (`crates/abi/src/lib.rs`, the `SPLIT` doc comment). `RETYPE_OBJ` collapses a
different pair the same way: *"when the untyped is exhausted, the object registry is full, or the
cspace is."* At the syscall layer (`kernel/src/syscall.rs:523`), the kernel-side `Option` from
`untyped::split()` becomes `.ok_or(Error::OutOfMemory)?` with no cause attached; the caller sees one
error code, full stop.

**The three causes are not shades of the same fact.** They point a fix in different directions:

1. **Your own region's budget is exhausted.** A fact about the caller: ask for a smaller grant, or
   free something first.
2. **Your cspace (capability slot table) is full.** Also a fact about the caller: it has too many
   capabilities already.
3. **`MAX_REGIONS` (256, `kernel/src/untyped.rs`), the system-wide concurrently-live region table,
   is full.** A fact about *everyone else on the machine*. Nothing the caller did is wrong, and
   nothing it can do locally fixes it.

A process, or a person debugging one, gets `OutOfMemory` and cannot tell which of these three is
true, which means it cannot tell "I asked for too much" from "some unrelated process elsewhere
exhausted a table I have never heard of."

## Why this is the same shape of defect this tree has already named once

`crates/timetable`'s `Unbacked`/`Refusal` split exists for exactly this reason, in a different
subsystem: *"A `Refusal` is a fact about **the line**... An `Unbacked` is a fact about **the
scheduler**... Collapsing the two would tell a person to edit a line that has nothing wrong with
it."* `OutOfMemory` collapses three facts (two about the caller, one about the whole system) the
same way, and nobody has named it here yet.

## What surfaced this

Not found by auditing error codes. Found while pricing milestone 49's attribution fork (DECISIONS
§109) and estimating where the channel model's own cost eventually bites: `MAX_REGIONS` exhaustion
is the real number that matters for "how many durable sessions can this system support," and tracing
what a caller actually sees when it happens found the three-way collapse.

## What has to be decided

Not answered here, on purpose: this touches the syscall ABI's `Error` enum, shared by every caller,
which is the same category the *move fast on what can be undone* tenet puts on the expensive side.

- **New, more specific `Error` variants** (e.g. splitting `OutOfMemory` into a per-cause set) is the
  most direct fix, but it is an ABI change every existing caller's match arms would need to account
  for, and it grows the syscall surface's vocabulary permanently.
- **A separate diagnostic-only query** (a way to ask "why did my last call fail" without changing
  what the failing call itself returns) keeps the hot-path return value exactly as narrow as today,
  at the cost of a second syscall a caller has to know to make.
- **Leave it collapsed, and document the ambiguity where a reader meets it** (this milestone's own
  `BUGS` entry in `kernel/src/untyped.rs` is that, today) is the cheapest option and may be the
  right one until `MAX_REGIONS` pressure is a measured, real problem rather than a traced
  possibility.

## What it unblocks

Nothing else is gated on this. It is named because a durable-session-heavy future (milestone 152)
is exactly the scenario where cause 3 (system-wide exhaustion, not the caller's own fault) would
start being hit by ordinary use rather than only by a misbehaving process, and a caller that cannot
tell that apart from its own mistake will be debugged as if it were one.
