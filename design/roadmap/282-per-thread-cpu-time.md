# 282. A thread's CPU time, and the `top` it makes possible

**Status: NOT-STARTED.** Minted 2026-09-13, when calef ruled milestone 126's eighteen-day-old fork.
The decision is `design/decisions/150-per-thread-cpu-accounting.md`; this is the build.
*(Number provisional until the merge queue lands it.)*

**Gate: NONE.** §150 settled the semantics, the sampling point, the unit and the wire shape. What
remains his is the program's name, like every name, and the one sub-choice §150 marks as expensive
to un-ship if he wants it back before this ships.

**In brief.** There is no per-thread CPU accounting anywhere in this kernel, dead or live. `Thread`
carries no time-on-CPU field, `sched::on_tick()` touches no per-thread state, and the only counter
is a machine-wide `preemptions()`. So this is new kernel state on the scheduler's hottest path, a
widened wire contract, and then a program that reads it.

## What to build

1. **A `u64` per `Thread`**, incremented by one tick in `sched::on_tick()` for the thread running on
   that core. One branch and one increment in code that already runs every tick on every core.
2. ~~**A fourth word on `abi::rendezvous::SURVEY`'s return**~~, **superseded 2026-09-21 by §204 (how
   userspace asks where a thread runs)**: `SURVEY` now takes a **selector** naming which record the
   caller wants, so CPU time becomes a new record value rather than a new return register. What this
   milestone adds is one constant, one arm in the kernel's match, and one line in the known-record
   check; nothing that reads an existing record is touched. §150 (a thread's CPU time)'s ruling on *what* is measured
   (tick-sampled, per-thread, scheduled on-CPU time) is unchanged; only how it reaches a reader moved,
   inside the window §150 named for overturning it. The reasoning below still applies,
   gated by the `ENUMERATE` right `SURVEY` already requires. This widens an existing method rather
   than adding a syscall number, the shape §114 used for `pmap`.
3. **The consumers.** `crates/ps` gains the column and `ps` prints it. The live-refresh view
   `watch` used to give was **cut** on 2026-09-13 rather than folded into a flag, on the grounds
   that a table of `TID` and `STATE` has nothing worth re-reading; this milestone is what makes one
   worth building again, and whether it returns as `top` or as a `ps` flag is decided then.
4. **`top` itself**, once there is something to be top *of*. See the naming note below.

## The name is not available until step 1 lands, and that is the point

`top` means *the top N by resource consumption*; it is defined by ranking. Today `ps`'s table is two
columns, `TID` and `STATE`, so there is nothing to rank by and the name would overclaim, which is
the fault `flaky` was renamed for on the same day this was minted. Under §150's option 2 the name
becomes both available and correct. Under the two refused options it would not have: wall-clock age
is not `%CPU`, and a sampled estimate is not a measurement.

**So `top` is earned by this milestone rather than assumed by it**, and whether the ranking view is
`top`, a `ps` flag, or something else is calef's call at build time.

## BUGS

- **The reader races the writer, by design.** Each core's tick touches only its own running thread,
  so the write needs no cross-core synchronisation, but a reader on one core observing a counter
  another core is incrementing is a relaxed-load race. The same shape the per-CPU `TICKS` array
  already accepts, and rule 4 says state it rather than assume it.
- **Tick-sampling is coarse and systematically so.** A thread that runs entirely between two ticks
  is charged nothing, which is what Linux's `jiffies`-based `utime` also does. That is the accepted
  cost of not touching `schedule()`'s hot path, recorded here rather than discovered by whoever
  first compares two numbers that should have matched.
- **CPU time is an aggregate statistic and capabilities do not close that side channel.** A
  confined viewer holding `ENUMERATE` learns something continuous about threads it cannot name,
  where today it learns only a state word. §150 says the authority question has an answer and that
  this is the part that is new.

## Follow-on

- **None.**

## Index row

Minted 2026-09-13 when calef ruled milestone 126's eighteen-day-old fork; the decision is §150.
There is no per-thread CPU accounting anywhere in this kernel, dead or live: `Thread` carries no
time-on-CPU field, `on_tick()` touches no per-thread state, and the only counter is a machine-wide `preemptions()`. Build a `u64` per `Thread` incremented one tick at a time in `on_tick()`, a
selector record on `SURVEY`'s return gated by the `ENUMERATE` right it already requires (widening a
method rather than adding a syscall, §114's `pmap` shape), then the `ps` column. `top` is *earned*
by this rather than assumed: the name means ranking by resource use and today's table is `TID` and `STATE`, so under the two refused options the name would have overclaimed.
