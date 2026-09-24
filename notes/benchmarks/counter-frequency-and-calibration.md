# Counter frequency and x86 calibration

*An appendix to [`notes/benchmarks.md`](../benchmarks.md), which carries the current numbers and is written so a reader can act without opening this file. This one holds the riscv64 hardcoded counter rate and the x86 TSC calibration that was wrong by up to 12x, with the dates, tables and corrections behind them. Name provisional (`notes/benchmarks/` and this stem), minted 2026-09-24 by the lane that split the note; naming is calef's.*

## 2026-09-21: the userspace counter frequency was a constant on riscv64, and what it did not reach

`user_mode_runtime::cntfrq` returned a hardcoded `10_000_000` on riscv64 from the port until this
date, and the std PAL (`sys/pal/nife/rt.rs`) carried its own copy of the same constant. QEMU `virt`
runs its `time` CSR at exactly that rate, so the number was right on every machine the suite runs on
and wrong by **2.5x on radon**, whose JH7110 states 4 MHz. calef's ruling that ended it: *"We should
ensure that the program only returns accurate numbers versus leveraging hard coded ones."*

**The first thing to establish was whether any committed number was wrong, and the answer is no.**
It is worth writing down why, because the reasoning is the reusable part:

- **The bench baselines are icount**, not time. `bench/baseline-riscv64.txt` holds guest instruction
  counts, which no counter frequency enters.
- **The `ns/iter` column does not come from userspace.** `os_primitives_benchmarker` reports raw
  ticks over IPC and `xtask/src/bench.rs` converts them, dividing by the `bench: cntfrq <hz>` line
  the *kernel* prints from `arch::timer::frequency()`. That has read `/cpus/timebase-frequency` since
  milestone 100 (read the machine's PSCI and its CPU list, not QEMU `virt`'s), so the published EL0
  figures were computed from the machine's own rate, on both
  QEMU and a board.
- **The job mix is the same shape**, from milestone 168 (a multi-tasking workload benchmark).
  `kernel/src/job_mix.rs` prints the kernel's `hz` and
  the tasks report ticks, so the multi-tasking numbers were never exposed to this.
- **CoreMark reports its own `cntfrq`** (`fixtures/src/coremark.rs` sends `[crc, ticks, freq]`) and
  is therefore the one measurement that *did* carry the constant. Nothing published a CoreMark score
  from it: this note's own CoreMark section says the binary "reports correctness, not yet a score",
  and the kernel test asserted only `freq > 0`.

**What the constant did reach was every program's own sense of how long something took**, which is
not a published figure and is not therefore harmless: `Instant`, `thread::sleep`, the shell's `time`,
`uptime`, and the timeouts in `net_stack` all run 2.5x off on radon, in the direction that makes
everything look slower than it is. On `x86_64` the equivalent defect was a 1 GHz fallback read by any
process the userspace ELF loader built, `coremark` included.

**The fix and the test are worth separating.** The fix was to widen a `cfg`: `x86_64` had already
built the mechanism (`counter_frequency_protocol`, a page the kernel fills and maps read-only into
every process) and riscv64 now uses it, with an unknown rate refusing rather than falling back. The
test is the part that would have caught this in July: `kernel/src/user/counter_frequency_tests.rs`
asserts that what a userspace program reports equals what the kernel measured, on all three
architectures. **A test that compares a rate against a constant cannot catch a constant**, which is
exactly why `freq > 0` held for two months.

## 2026-09-21: the x86 boot calibration was wrong by up to 12x, and what that does to the numbers above

The `tscdrift` lane measured the TSC against the CMOS RTC and found the counter ticks at exactly
1000.000 MHz under plain TCG. It also found, without looking for it, that the rate the kernel
*stores* had nothing to do with that: twenty-two boots of one binary wrote down 1001 MHz to 4330
MHz. This lane took that further, fixed it, and this section is what the fix means for anything
already published. `notes/tsc-under-tcg.md` owns the measurement; the fix is
`kernel/src/arch/x86_64/timer.rs` and milestone 571 (the x86 boot calibrates the TSC once,
and can be wrong by 4x).

### The instrument, stated because two instruments wear the same name

**`script/bench --x86` and `script/bench --x86 --real` are different instruments and neither is a
stand-in for the other.** Without `--real`, `bench --x86` adds `-icount shift=0,sleep=off`, and
under `-icount` a guest nanosecond is a function of the instruction stream rather than of real
time: the `tscdrift` lane measured the implied rate moving **37% between two workloads inside one
boot**. That is `-icount` working as designed, it is what makes the tripwire deterministic, and it
means an icount leg's `ns` column is not wall time and never was. **Every section of this file that
quotes x86 numbers should say which one it used, and from here on they do.**

The calibration defect below affects the `--real` instrument only, and within it the `ns` column
only. Ticks are raw `rdtsc` counts and are untouched.

### What was wrong, and by how much

`init_frequency` timed **one** 10 ms PIT window by polling. The poll can only notice the terminal
count *late*, never early, and the whole TSC delta was then divided by exactly 10 ms, so a single
descheduling of the QEMU thread inside that window inflated the stored rate without bound. Two
hundred boots against a counter known to tick at 1000.000 MHz, on a host deliberately saturated to
load 30 on eight cores:

| Windows taken | Median error | 99th percentile | Worst of 200 | Boots wrong by >1% |
|---|---|---|---|---|
| 1 (what shipped) | +0.36% | +884% | **+1153%** | 56 / 200 |
| 3 | +0.00% | +120% | +131% | 24 / 200 |
| 5 | +0.00% | +20.6% | +51.4% | 11 / 200 |
| 9 | +0.00% | +0.49% | +7.1% | 2 / 200 |
| 16 | +0.00% | +0.01% | **+0.47%** | **0 / 200** |

Every one of the 490 boots measured across three host loads was **high**, never low, which is the
signature that named the mechanism rather than merely describing the spread.

### Which published numbers this reaches

- **The 2026-08-24 x86 `ns/iter` table** (`yield_switch`, `tss_iomap_switch`, debug and release) is
  the one body of published x86 wall-clock figures derived from the stored rate. It is marked in
  place, at the table, with what is and is not still safe to quote. The short version: an inflated
  rate makes `ns/iter` too **small**, so those figures may read faster than the runs really were,
  and every conclusion in that section that rests on a ratio between two rows of one boot survives
  untouched, because the calibration cancels exactly in a ratio.
- **The 2026-09-15 lazy-TSS section and the icount baselines are not affected.** They quote
  deterministic ticks under `-icount` and say so; `bench/baseline-*.txt` holds ticks.
- **`coremark`'s self-reported rate on x86** read the stored number, so any x86 CoreMark score
  computed from it carried the same error. No such score is published; the 2026-09-21 section above
  records that CoreMark "reports correctness, not yet a score".
- **`Instant`, `uptime` and the shell's `time` on x86** all read it through
  `counter_frequency_protocol`'s page. Not published figures, and not therefore harmless: every
  process's sense of elapsed time was off by that boot's error.
- **Nothing went red over any of it**, which is why it lasted. `kernel::user::wait_for` takes
  `now()` plus two seconds, so an inflated rate makes a timeout *longer* in real time, never
  shorter. The defect could only ever fail safe.

### Before and after, as a controlled pair

Two `script/bench --x86 --real` runs of the same tree, differing in one constant
(`CALIBRATION_WINDOW_CAP` at 1 and at 16), so the before case is the old estimator exactly:

| | Implied stored rate | Error against 1000.000 MHz |
|---|---|---|
| Before (one window) | 1003.7 MHz | +0.37% |
| After (min of N) | 1000.1 MHz | +0.01% |

**That before figure is one draw and a lucky one**, which is the honest caveat and the reason the
table further up is the real evidence: at this host's load roughly one boot in four was wrong by
more than a per cent, and the worst seen was +1153%. A single before-and-after pair cannot show a
distribution, and quoting it alone would understate the defect by two orders of magnitude.

**The `ticks` columns of those two runs differ by up to 4x, and that is not the calibration.** It
is host load between the two runs, which is exactly what `--real` measures and exactly why
`script/bench --x86` pins the virtual clock by default. A wall-clock bench on a shared Apple
Silicon host running TCG is a magnitude to read, never a number to gate.
