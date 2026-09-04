# Bound what one mutant may allocate, so a runaway kills the mutant and not the machine

**Status: PROPOSED 2026-09-03.** Written by the milestone 247 sweep, from milestone 238's block.

**Gate: NONE.** Three shapes are already priced in milestone 238's block, and choosing between them
is engineering rather than a decision owed to calef. It is reversible: all three are a wrapper
around an existing command.

**In brief.** `script/mutation` gives each mutant a per-mutant timeout of 28 to 51 seconds. That
bound is on **time**, and the failure that actually occurs is on **memory**: one mutant goes from
1.4 GB to **15.8 GB in twenty seconds** and takes the whole runner agent with it, comfortably inside
the timeout that therefore never fires. The work is to add a memory bound per mutant so the mutant
dies and the sweep continues.

## Why this matters

The mutation sweep is the refresh for `design/fatal-risks.md`'s third risk, and it has a history of
not producing results: four scheduled runs, four failures, zero reports before milestone 238. A run
that dies because one mutant ate the machine is that same outcome with a new cause, and it is worse
than a timeout because it takes the runner with it rather than reporting a single skip.

The measured shape of the failure is what makes this cheap and worth doing. It is not gradual
pressure; it is one process going ten times its size inside twenty seconds. A hard ceiling turns a
run-ending event into a one-line entry in a report, and a mutant that cannot be evaluated within a
sane memory budget is itself a useful thing to have recorded.

It also has a second victim this tree has already paid for. AGENTS.md records that concurrent heavy
jobs are bounded by memory rather than by the collision surface, and that a day of out-of-memory
kills cost a session, a confusing `ci-build` timing failure, and two `script/verify` runs killed
mid-CBMC. The tell is always the same and is always misread: a heavy job dying with no failing
assertion. An unbounded mutant is one more source of exactly that.

## The three shapes, from milestone 238's own pricing

- **A memory cgroup via `systemd-run --scope`.** The strongest bound and the most Linux-specific.
  Kills reliably at the limit, needs the runner to be Linux with cgroup v2, and does nothing on the
  dev Mac where `script/mutation` is also run by hand.
- **`ulimit -v` ahead of `script/mutation`.** Portable and one line, and it bounds address space
  rather than resident memory, which over-counts for anything that reserves generously. It applies
  to the whole sweep process tree rather than per mutant unless the wrapper re-applies it.
- **A `cargo` runner wrapper.** Applies per test binary, which is the exact granularity wanted, at
  the cost of a small program in the tree that every mutation run then depends on.

Choosing is the work, and the choice should say what happens on the dev Mac as well as on the
runner, because both are places this sweep gets run.

## Where it came from

Milestone 238's `## Follow-on`: *"Bound what one mutant may allocate, so a runaway allocation kills
the mutant instead of the machine. Three shapes are priced in this block (a memory cgroup via
`systemd-run --scope`, `ulimit -v` ahead of `script/mutation`, or a `cargo` runner wrapper) and
choosing between them is the work. Today one mutant goes 1.4 GB to 15.8 GB in twenty seconds and
takes the runner agent with it, inside the 28-to-51-second per-mutant timeout that therefore cannot
catch it."*
