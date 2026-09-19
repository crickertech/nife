# 209. Whether the job mix reports the spread rather than the best, and whether `REPEATS` varies by sweep point

**Status: PROPOSED.** Raised 2026-09-19 by milestone 435's slice-c lane, which found milestone 419's
`DECISION` gate naming no section. Filed 2026-09-16 by the maintainer from the five-boot job-mix
session on radon, where the fourth and fifth boots each landed outside the range the first three had
established. *(Section number provisional until the merge queue lands it.)*

## What is being decided

Milestone 168's sweep reports the **best of three** repeats per sweep point. At `tasks=4` the
underlying distribution is wide enough that the best of three is itself a coin flip. The decision is
what the instrument reports and how many samples it takes, and it is not a patch because **the
`job-mix:` line is output two programs read**.

## Is the premise true

Checked 2026-09-19 in this worktree:

- `crates/job_mix/src/lib.rs:271` still reads `pub const REPEATS: usize = 3;`, one constant for the
  whole sweep, and line 276 still reads `pub const ECHO_SERVERS: usize = 2;`.
- `kernel/src/job_mix.rs:214` prints `job-mix-repeat: tasks={tasks} repeat={repeat} ticks={ticks}`,
  and `xtask/src/main.rs:9061` and `9067` parse the `job-mix` family by prefix.
- Milestone 168 is still `PARTIAL`, and its status says in its own words that it does not turn
  `BUILT` until `tasks=4` has a number. It still does not have one.

## The measurement

Across five boots of an identical image the reported figure at `tasks=4` ranged from **766,361 to
991,671 jobs per minute, a 29.4% spread**, while boot 3's three repeats *on their own* spanned
132,148 to 181,408 ticks, which contains the whole boot-to-boot range.

**The variance is within a boot, so power cycling does not reduce it.** That is the finding that
decides the shape of the fix: more boots buy nothing, more samples might.

`tasks=4` is where `ECHO_SERVERS = 2` first produces contention (2:1) with too few samples to
average it. That contention is deliberate and correct; what is missing is enough samples at the
point it bites.

## Why the obvious fix is not the cheap one

**The cost is not symmetric across sweep points.** `tasks=32` already takes about 930,000 ticks per
repeat and is stable at 2.7%; tripling its repeats buys nothing and lengthens every board session
noticeably. `tasks=4` takes about 150,000 ticks and is the one that needs them.

So "raise `REPEATS`" is the wrong shape and "vary repeats by sweep point" is the right one, and that
turns one constant into a table, which is a different thing for a reader to hold and for the
transcript recogniser to parse.

## What this tree already does in the analogous case, and where it does not apply

**`REPEATS`' own doc states the rule this tree uses**: the minimum is the least host-contended
sample and everything above it is somebody else's load. That reasoning is sound for a
micro-benchmark on a busy host, which is what §25's icount work and `script/bench` are.

**It is questionable for a workload whose whole subject is contention between its own tasks.**
There, the spread is the signal rather than noise to be minimised away, and reporting only the best
discards it. This tree's standing posture on benchmarks is that an honest tie or loss recorded
plainly is worth more than an overclaimed win, and a best-of-three on a 29.4% distribution is
neither honest nor a win.

## The options

| | what | cost |
|---|---|---|
| **1** | raise `REPEATS` uniformly, to 5 or 7 | simplest, one constant, no format change. Costs the most board time and spends it mostly where it is not needed |
| **2** | a per-sweep-point repeat table | cheap in board time and targets the problem. Costs a constant becoming a table, and the recogniser has to stop assuming a fixed count |
| **3** | keep three repeats and report the spread rather than the best: min, max and median on the line | no extra board time at all, and it makes the instability visible rather than averaged away. **Costs a wire-format change to the `job-mix:` line** |
| **4** | do nothing and record `tasks=4` as a range | free, honest, and leaves milestone 168 `PARTIAL` for a reason nobody can close without one of the above |

## Recommendation

**3, with 4 as the interim**, and the reason is the one this section opened with: the job mix exists
to measure what contention costs, and a report that keeps only the least-contended sample is
answering a different question than the one §96 asked.

**Option 3 is the most expensive on AGENTS.md's own test and that is why it is here rather than in a
commit.** It changes a line two programs read, which is the category that cannot be un-shipped, so
it is calef's rather than a lane's. Until it is decided the honest thing is option 4, which is what
milestone 168's block now does.

**Option 2 should be refused if 3 is taken**, rather than stacked on it: reporting the spread makes
the sample count legible in the output, so varying it per sweep point stops being a hidden fact and
becomes one more column, which is a second format change for the same problem.

## Would we still choose 3 if all four cost the same

Yes, and more clearly. 3 is the *most* expensive option on this list and is still the recommendation,
so the argument does not rest on effort at any point.

## How reversible, and who has acted on it

**The `REPEATS` constant is reversible; the line is not.** `script/job-mix`'s rehearsal and
`xtask/src/main.rs`'s parser both read the `job-mix` family by prefix, and board transcripts already
captured on radon and xenon carry the current spelling, so a format change re-means every transcript
in `bench/` that a later reader compares against.

## What is blocked until this is answered

**Milestone 168 turning `BUILT`.** Its own status line says it does not until a number exists, and
`tasks=4` does not yet have one.
