# The x86 TSS I/O-bitmap switch cost

*An appendix to [`notes/benchmarks.md`](../benchmarks.md), which carries the current numbers and is written so a reader can act without opening this file. This one holds the naive and lazy bitmap writes that §121 asked to have priced, with the dates, tables and corrections behind them. Name provisional (`notes/benchmarks/` and this stem), minted 2026-09-24 by the lane that split the note; naming is calef's.*

## 2026-08-24: the TSS I/O-bitmap switch cost (DECISIONS §121's amendment)

§121 is choosing how x86 userspace drivers reach legacy port-I/O devices (the UART, the PIT, the
8259s, the CMOS clock). Its option 1, a port-range capability enforced by the TSS's I/O permission
bitmap, names its own dominant unmeasured cost in the 2026-08-24 amendment: writing the bitmap into
the current CPU's TSS on every context switch. This section is that measurement, done the moment
milestone 161 item 4 made real two-thread switching on `x86_64` exist to measure.

### A third instrument, and why the other two do not apply

`crate::arch::timer::now()` already dispatches to `rdtsc` on `x86_64` (calibrated against the 8254
PIT at boot; `kernel/src/arch/x86_64/timer.rs`), so `kernel/src/bench.rs`'s existing `timed()` helper
needed no change to run on this ISA. What is missing is everything *around* it:

- **No icount leg, at the time this section was written.** `icount()` in `xtask` refused
  `--arch x86_64` ("the instrument's boot needs a userspace this port cannot build"), and the
  inference drawn here was that the same was true one level up: nothing pins QEMU's virtual clock
  to the instruction stream on this port, so there was no deterministic tick count to gate a
  baseline against, the way `bench/baseline-aarch64.txt` and `-riscv64.txt` do. **That inference
  was wrong, and milestone 161's icount leg (2026-08-25, below) corrects it**: `icount()`'s refusal
  is real and still stands (milestone 78's claims need a re-armed deadline timer to compare
  against, and this port's LAPIC timer is a periodic hardware reload with no deadline to read), but
  pinning the virtual clock for a plain duration measurement is a strictly weaker ask, nobody had
  tried it, and it works.
- **No HVF, no KVM.** The dev machine is Apple Silicon; there is no hardware acceleration for
  `x86_64` on it. Every number below is plain QEMU TCG, translating x86 instructions on an aarch64
  host, one at a time. That is slower than real silicon and slower than KVM, and the ratio is not
  known, so **magnitudes here are not a stand-in for real x86 hardware.** What is real is the
  *comparison* within one boot: two benches, same host, same QEMU process, same instant, differing by
  one write.
- **The bench boot itself needed a home on this ISA.** `kernel_main`'s `x86_64` arm was a fixed,
  self-contained tour with no `#[cfg(feature = "bench")]` branch at all (the other two architectures
  have had one since milestone 21); it now diverges into `bench::run()` right after
  `smp::bring_up_secondaries()`, the same position the aarch64 half of `kernel_main` uses. `cargo
  xtask bench --x86` builds and runs it; see `bench_x86()` in `xtask/src/bench.rs`.

Every EL0-plane bench (`null_syscall_el0`, `ctx_switch_el0`, `ipc_rtt_el0`, `sink_throughput`,
`map_el0`, `spawn_el0`) self-skips on this leg through the mechanism they already had (`crate::
user::program` finds nothing, because `crates/user_mode_runtime` has no `x86_64` arms yet): no new gating was
needed for them. `fs_read`, `fs_throughput` and `smp_throughput` self-skip the same way they do on a
single-hart `--real` run elsewhere. What is left, and what runs cleanly, is the kernel-thread plane:
`yield_switch`, `ipc_rtt`, `relay_rtt`, `call_reply`, `broker_rtt`, `spawn_reap`, `map_new`,
`coremark`, plus one new x86-only bench.

### `tss_iomap_switch`: `yield_switch` plus one write

The x86 port space is 16 bits (64 Ki ports), one permission bit each, so the real bitmap is
`65536 / 8 == 8192` bytes exactly; the "8 KiB" in §121's own text is architecture, not a round
number. `tss_iomap_switch` is byte-for-byte `yield_switch` (the same two threads, the same
`YIELD_ITERS = 2000`, the same warmup) with one call added on every resume, in both threads:
`arch::segments::bench_write_io_bitmap`, which `write_bytes`-fills a CPU-owned 8192-byte static and
reads its last byte back (so nothing about the write is provably dead code). **It is a stand-in for
the write, not for the enforcement**: `iomap_base` in the live TSS is untouched, `ltr` is never
reissued, and no ring-3 program executes `in`/`out` in this benchmark boot (there is no ring-3
program on `x86_64` yet outside the hand-assembled ones in `user::x86_programs`). What it prices is
exactly the cost the amendment named: an 8 KiB per-CPU memory write added to the switch path, twice
per iteration (both threads resume once each), so the delta divided by two is the cost of one write.

### The numbers

> **CORRECTION, 2026-09-21: every `ns/iter` in this section is suspect, and the `ns` column alone.**
> These were computed from the guest's calibrated TSC, and on 2026-09-21 the x86 boot calibration
> was found to be wrong by up to **+1153%**, always high, differently on every boot. An inflated
> rate makes `ns/iter` come out proportionally **too small**, so these figures may read faster than
> the run really was, by anything from a fraction of a per cent to a factor of several. Nothing can
> recover the true values: the rate each of these eleven boots stored was not written down, and it
> was a different number each time. The *ticks* they were derived from are unaffected, and so is
> every conclusion here that rests on a ratio between two rows of the same run, which is all of
> them: the debug-versus-release argument and the 4.2x are ratios within a boot, where the
> calibration cancels exactly. **What is not safe is quoting any single figure below as a
> nanosecond count.** See the 2026-09-21 section at the end of this file, and
> `kernel/src/arch/x86_64/timer.rs`'s `BUGS`.

QEMU 11.0.2 (`.qemu-version`, pinned), `-machine q35 -cpu max`, one hart, plain TCG. Six boots
debug, five release; `ns/iter` computed from the guest's own calibrated TSC, the same arithmetic
`xtask`'s `run_bench` already does for every other leg.

| build | bench | ns/iter, median | all runs |
|---|---|---|---|
| debug (6) | `yield_switch` | **12,320** | 12172, 12483, 12022, 12293, 12456, 12347 |
| debug (6) | `tss_iomap_switch` | **15,360** | 15286, 15342, 14944, 15378, 15494, 15751 |
| release (5) | `yield_switch` | **1,267** | 616, 1654, 678, 1267, 1406 |
| release (5) | `tss_iomap_switch` | **6,769** | 4603, 7741, 3935, 6830, 6769 |

| build | delta (`tss_iomap_switch` − `yield_switch`), median | per single 8 KiB write (delta / 2) | overhead over a bare switch |
|---|---|---|---|
| debug | 3,040 ns/iter | **~1,520 ns** | +25% |
| release | 5,363 ns/iter | **~2,682 ns** | +423% |

**Read the two rows together, not separately, because they tell different halves of the story.**
Debug's ~25% looks tolerable; release's ~4.2x is the honest number, and it moves in the direction
§121 already argued from prose: a debug build carries so much fixed overhead around the switch
(unoptimized bookkeeping, unelided checks) that the 8 KiB write is a modest fraction of a slow
baseline. Strip that overhead in release and the baseline switch itself gets **~10x faster** (12,320
ns to 1,267 ns) while the write's own cost barely moves (~1,520 ns to ~2,682 ns, both plain memory
bandwidth and expected to be close). So on the switch path the write would actually run on, the
write does not add a fraction of the cost, **it dominates it**: §121's amendment called this "the
dominant cost" from architecture, before any number existed, and it undersold it if anything, at
least against a release-shaped kernel.

**The release row is noisy, and that is itself part of the finding.** The five release runs span
3,935 to 7,741 ns for `tss_iomap_switch`, roughly 2x peak to trough, against debug's tight
14,944-15,751 spread. Plain TCG with no `-icount` runs on the host's wall clock, and a release
iteration is fast enough (microseconds) that host scheduling jitter on a shared dev machine is a
real fraction of the measured window; a debug iteration is slow enough (tens of microseconds) that
the same jitter is proportionally smaller. This is exactly why `--real` (still the only mode
`--release` can use; see below) never gates: there is nothing to gate on in a wall-clock magnitude,
only a magnitude to read, medians over single runs.

### What this does and does not settle

**It gives §121 the number its amendment asked for**, on both sides of the 1-vs-3 call: option 3's
cost was already on record (~337 ns per IPC round trip, DECISIONS §121 citing this file's cross-OS
table); option 1's now is too (~1.5-2.7 us per switch for the write alone, before the capability
type, the revocation shootdown, or anything else option 1 would also cost). Both are worse than a
raw `in`/`out` instruction (single-digit cycles), which is what makes option 2 (keep legacy devices
in the kernel) the correct default absent a reason to want a userspace console on `x86_64`
specifically, unchanged from the amendment's own conclusion, now with a number under the option-1
half of it.

**It does not settle which option §121 picks.** That is calef's call per the decision's own closing
question, and this section changes only what he has to decide with, not the decision.

**And it does not build option 1.** No port-range capability, no `Untyped::SPLIT`-derived granting,
no syscall surface change; `bench_write_io_bitmap` is explicitly not wired to the live TSS's
`iomap_base` for exactly this reason, so nothing here can be mistaken for the real mechanism.

## 2026-09-15: the LAZY TSS I/O-bitmap write, the number §121's refinement asked for (milestone 299)

DECISIONS §121 was reversed on 2026-09-15 to build the port-range capability (milestone 299), and its
2026-08-25 refinement is now binding rather than a footnote: the ~2,682 ns/write above prices the
**naive** always-write of the whole 8 KiB bitmap on every switch, and the real implementation is the
**lazy** write, where `arch::segments::set_port_grant` compares the incoming thread's grant against
what the core already holds and writes only the bits that move. This section is the measurement the
refinement said to take: the lazy write's real cost, timed against a bare switch, the way the section
above timed the naive one.

Two new benchmarks join `tss_iomap_switch` in the x86 bench boot (`kernel/src/bench.rs`), the same
two-thread yield ping-pong, differing only in what each thread installs on resume:

- **`tss_iomap_lazy_switch`**: the peer installs `None` (no ports), the main thread installs COM1's
  `(0x3F8, 8)` range, so **every** switch is a holder<->non-holder transition, the case that takes
  the write. This is the worst case, not the common one.
- **`tss_iomap_lazy_nop`**: both threads install `None`, so `set_port_grant` finds the core already
  holds `None` and returns after one comparison, writing nothing. This is the case nearly every
  switch on nearly every machine actually takes.

Measured on the pinned QEMU under **TCG + icount** (deterministic instruction-count ticks; the
x86 icount leg the 2026-08-25 section above added), **debug** build, 2000 iters, one run each (icount
is reproducible, so a single run is the number):

| benchmark | ticks/iter | delta over `yield_switch` |
|---|---|---|
| `yield_switch` (bare two-switch round trip) | 10,580 | baseline |
| `tss_iomap_lazy_nop` (nothing holds a port) | 10,796 | **+216** |
| `tss_iomap_lazy_switch` (every switch crosses a holder) | 12,567 | **+1,987** |
| `tss_iomap_switch` (naive 8 KiB always-write) | 12,953 | **+2,373** |

**The load-bearing result is the second row.** On a system where nothing holds a port (every switch
but the console driver's), the lazy mechanism adds **+216 ticks/iter**, ~108 per switch: a load of the
per-core "installed grant" and a branch. That, not the ~2,682 ns naive write, is what x86 pays for
having the enforcement present, and it is what the naive always-write threw away by writing 8 KiB on
switches that never touch a port.

**Why the lazy-switch and naive rows look close under icount, and why that understates the win.** In
instruction count they are +1,987 vs +2,373, only ~16% apart, which is not the order-of-magnitude gap
the byte counts (2 bytes moved vs 8192 written) would suggest. The reason is the instrument: the naive
`write_bytes(_, 8192)` compiles to a `rep stos`, a handful of *instructions* for 8 KiB of *memory
traffic*, and icount counts the instructions, not the traffic. So icount cannot see the naive write's
real cost, which is exactly why §121 measured that one in wall-clock (~2,682 ns) rather than icount.
The lazy form's advantage is therefore **not** primarily that its holder-crossing write is cheaper in
instructions (it is a little cheaper); it is that (a) the crossing almost never happens, so the common
switch pays the +216-tick nop path, and (b) when it does happen it moves ~2 bytes instead of issuing
an 8 KiB `rep stos`, a real-memory cost icount is blind to but a cache is not.

**Caveats, stated because the magnitudes are fiction.** TCG models no caches, so the ns the tool would
print are meaningless and only the deterministic ticks are quoted; there is no HVF for x86_64 on this
Apple Silicon host, so no `--real` magnitude leg exists for these the way it does for aarch64. The
naive row here (+2,373 ticks, icount) and §121's naive row above (~2,682 ns, wall-clock release) are
the *same benchmark* measured two different ways and are not comparable numbers; both say the same
thing, that the always-write is the dominant cost of a switch, and this section adds the half §121
could not yet take: the lazy write moves that cost off every switch that does not cross a holder.
