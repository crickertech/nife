# The icount claims: the sixth round, 2026-08-17

*(An appendix of [notes/load-sensitive-assertions.md](../load-sensitive-assertions.md), which holds
the register and the rules. Moved there verbatim on 2026-09-24; "this page" and "above" in the text
below meant the single note it came from.)*

## The sixth round, 2026-08-17: the instrument, and the two claims it was owed

The five rounds above re-aimed everything that could be re-aimed. What they could not re-aim, they
deferred to the same place every time, in six separate paragraphs across this page and the block:
**the icount instrument, recommended and not built.** This round built it. It is `script/icount`,
a boot mode rather than a `#[test_case]`, and notes/instruction-clock.md is its note.

### What was actually unaskable, and why no margin was ever going to do

Both remaining claims fail on the same sentence, which this page states twice: *from inside the
guest, a slow handler and a descheduled emulator are the same observation.* That is not a
sensitivity problem, so it does not have a sensitivity fix. Widening hides the defect (DECISIONS
§61, and the block forbids it by name); deleting the assertion is what round one already refused.

The third option is to change the **unit**. Under `-icount shift=0,sleep=off` virtual time advances
by exactly one nanosecond per guest instruction retired and by nothing else, so a claim denominated
in instructions has no host term in it at all. Both claims are now stated that way:

| claim | aarch64 | riscv64 | bound |
|---|---|---|---|
| deadline to handler observing it | 1,008 instructions | 300-400 | 2,000 / 1,500 |
| deadline to next one armed (the whole handler) | 1,056 | 800-900 | 2,500 / 2,500 |
| ticks missed over 64 sampled | 0 | 0 | 0 |

**The aarch64 numbers are identical across all 64 ticks**, minimum equal to maximum. That is the
instrument demonstrating itself: the measurement has no variance, so a bound on it is a statement
about this kernel and about nothing else. riscv64's pair differ by one counter tick because that
ISA's `rdtime` reads in steps of 100 instructions where aarch64's counter reads in steps of 16.

### The injection, which is the only part of this that proves anything

The claim that matters is riscv64's, because it is the one this milestone was left holding: SBI's
`set_timer` is write-only, so `DEADLINE` is our own array and reading it back proves only that the
kernel remembers what it meant to write. The block names the exact residual: *"an implementation
that maintains `DEADLINE` correctly and arms SBI with something else"*.

So that implementation was built and run, twice, each a single line in `rearm` with the grid store
left untouched beside it. Both were reverted.

| injection | `script/icount --arch riscv64` | the riscv64 leg of `script/test` |
|---|---|---|
| `sbi_set_timer(now + interval())`: re-anchor every tick | **red**, arrival 420,400 instructions against a bound of 1,500 | **red**, `the_handler_keeps_up_when_no_lock_is_held` |
| `sbi_set_timer(next + interval() / 4)`: a fixed offset, no drift, no misses, 100 Hz still exactly delivered | **red**, arrival 2,500,400 on every one of 64 ticks | **red**, `ticks_arrive_at_the_configured_rate` |

**The prediction was that the suite would miss them, and it did not. That is the useful part.** What
the suite cannot do is say what is wrong. The first injection fails as *"the timer handler is taking
longer than a whole tick period, with no lock held... Late by less than one interval means the
handler itself is slow, which is this kernel's bug"*, which is false, and which is the assertion that
broke #204, #210 and #215 in one afternoon. The second fails as *"no miss-free measurement window in
eight tries: either the host is too contended to observe the grid, or the handler is slower than a
whole tick period"*, whose first clause is an invitation to re-run.

The icount message names the actual defect: *"either the trap path grew, or the timer was armed with
something other than the deadline the kernel recorded"*, on a number that has no host term in it.

**So what the instrument buys is diagnostic certainty rather than detection**, which is this
milestone's own thesis rather than a lesser result. The block's cost line is that every red check
here needs a human to decide "known or real", and that on 2026-08-03 the judgement was made six times
and got the wrong answer twice. Both injections produce exactly that judgement call on the test path,
and none of it on the instrument.

A third injection asked what the instrument can *see* rather than whether it fires: exactly 200
instructions added to the aarch64 `tick`. Arrival went 1,008 -> 1,216 and the handler 1,056 -> 1,264,
both **+208** on every one of 64 ticks, with the eight-instruction residual (the loop's operand
setup) smaller than one counter tick. That is the resolution measured, and it is what makes the
instrument able to answer milestone 106's pricing question on aarch64.

### What the instrument cost, and a correction it forced

The block states the cost as *"icount is slower and changes what the suite measures"*, and the
first half of that had never been measured. **It is wrong**:
the same bench boot took 2.47-2.61 s under `-icount shift=0,sleep=off` and 2.62-2.80 s without it,
three runs each, on the same binary. `sleep=off` fast-forwards virtual time through idling, which
covers icount's per-instruction overhead.

The two real reasons are different and better. Every vCPU shares **one** virtual clock, so the
instrument is `-smp 1`, and a suite run there would not fail: it would silently stop proving every
cross-core property it exists for. And a clock-bound wait stops costing host time and starts costing
instructions, at roughly five to one. The first of those is the same fact that keeps the placement
probe on the wall clock, arriving from the other side.

### What this round did not do

**The other sites in the scope note were still not audited.** Five rounds have now said this. The
reading order (three greps: a global count as a baseline, a loop with no clock, a frame count
standing in for a mechanism) is the accumulated answer and nobody has run it across the remaining
files.

**Only the timer is instrumented.** `tick_trace` is three relaxed counters and one call site, not a
framework, and any other path wanting an instruction-denominated claim needs its own. That is
deliberate: the block asked for two claims, not for infrastructure.
