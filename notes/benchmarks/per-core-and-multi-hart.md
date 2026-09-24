# Per-core magnitudes and the multi-hart bench

*An appendix to [`notes/benchmarks.md`](../benchmarks.md), which carries the current numbers and is written so a reader can act without opening this file. This one holds why `--real` is single-hart, the 2026-07-29 refresh, and the `smp_*` placement numbers, with the dates, tables and corrections behind them. Name provisional (`notes/benchmarks/` and this stem), minted 2026-09-24 by the lane that split the note; naming is calef's.*

## The one bench that is legitimately multi-hart (DECISIONS §28, the placement win)

Every primitive above is hart-pinned, and it has to be: the icount instrument boots `-smp 1`, because
under `-icount` all vCPUs share one virtual clock and an idle hart's `wfi` jumps that clock forward
(the 2026-07-28 finding above). So the deterministic suite measures per-core path length and is blind,
by construction, to §28's whole job: spreading work across the four harts. The `smp_*` benches
(`kernel/src/bench.rs::smp_throughput`) are the one measurement that shows it, and their methodology is
different on purpose, so this section is where the difference is written down.

Run them with `script/bench --real --smp` (HVF, 4 harts). Plain `--real` is single-hart on purpose
(per-core primitive magnitudes; see the refresh section below), so `smp_throughput` self-skips there.

**They never gate, and never touch `bench/baseline-aarch64.txt`.** Two structural reasons. First, they run
only when `online_count() > 1`, which is only the `--real --smp` boot; under the icount instrument
(`-smp 1`) and the default single-hart `--real` run, `smp_throughput` returns immediately, so no
`smp_*` line is ever emitted there and the committed baseline never sees them (verified: `--check`
output has no `smp_*` rows). Second, a wall-clock throughput number is not even defined under `-icount`
(one shared clock), and TCG serialises all vCPUs onto one host thread, so there is no real parallelism
to measure. Only HVF gives each core its own counter and genuine concurrent execution. These are
statistical HVF magnitudes read by a human with loose bounds, exactly like the other `--real` numbers,
not a tick baseline.

**Two workloads, because they tell opposite and both-true stories.**

| bench | workload | one batch |
|---|---|---|
| `smp_compute_*` | N independent CPU-bound grinders, no syscalls | `solo` = 1 worker; `all` = 16 workers, each the same fixed grind |
| `smp_pipe_*` | N independent synchronous IPC ping-pong pairs | `solo` = 1 pair; `all` = 16 pairs, each 2000 round trips |

The scaling factor for either is the `solo` throughput divided by the `all` throughput, i.e. read it
from the totals (`iters / ticks`), not from the coarse `ns/iter` column. `smp_cores` records the
ceiling (4 on this boot).

**Compute scales, ~3.5x on 4 cores, and that is the §28 placement win.** Numbers (HVF, release,
min-of-4 batches, five boots):

```
smp_compute_solo   ~8,886 ticks / 300,000 iters      (one core's grind rate)
smp_compute_all   ~40,000 ticks / 4,800,000 iters    (16x the work, across the machine)
```

Sixteen workers is sixteen times the work; run one at a time it would take `16 x 8,886 = 142,176`
ticks, and it finishes in ~40,000, a **3.5x speedup** (≈89% of the 4x ceiling). The lost ~11% is real
and expected: 16 does not divide into 4 waves cleanly (the last wave runs four workers where earlier
waves were full), plus spawn, reap, and the barrier. A CPU-bound worker makes no cross-core wake once
placed, so the host keeps every busy vCPU on a real core, and what is left to measure is exactly
placement filling the machine. This is the number no hart-pinned primitive can show.

**Synchronous IPC pipelines do NOT scale under HVF, they go slightly backwards, and the reason is the
host, not the scheduler.** Numbers (same conditions):

```
smp_pipe_solo   ~2,900 ticks / 2,000 rtts    (~59 ns/round trip, one warm core, all local)
smp_pipe_all  ~250,000 ticks / 32,000 rtts   (~322 ns/round trip aggregate)
```

The aggregate per-round-trip is *slower* than a single pair's, a ~0.18x "speedup". That looks alarming
until you see why, and the why is a virtualization property. A single pair, with the other three cores
idle and the main thread blocked, co-locates by §28's local-wake rule and does every rendezvous on one
warm core, no cross-core traffic at all, so it runs at the `ipc_rtt` rate (~59 ns). Sixteen pairs get
scattered across the cores by placement, and whenever placement or stealing splits a pair across two
cores, its next rendezvous is a cross-core wake, an SGI to a vCPU the host has descheduled because the
guest looked idle a moment earlier. Waking a descheduled vCPU costs host reschedule latency that the
co-located pair never pays. So the IPC-heavy parallel workload spends its time in HVF's wake path, not
in the kernel. This is the **same** reason the icount suite is pinned to one hart and the same reason a
same-machine seL4 number is deferred to real hardware: the instrument underneath, not nife, sets
the ceiling. On real silicon with four dedicated cores and no descheduling, the pipelines would scale
the way compute does here; measuring that is a real-hardware follow-up (milestone 16), and the bench is
already written to report it when the wakes are cheap.

Getting the solo baseline honest took one correction worth recording, because it is the same class of
error as the smp=4 counter bug. The first version had the main thread **busy-yield** on a done counter
instead of blocking on a `RECV`. A runnable main plus the pair is three threads the scheduler scatters,
so even the *solo* pair took cross-core wakes and clocked ~60x slower than `ipc_rtt`'s identical pair,
and the derived scaling came out **superlinear** (greater than the core count), which is not physical.
Blocking the main thread (the `ipc_rtt` shape) fixed it: solo returned to the ~59 ns rate and scaling
fell back under the ceiling where it belongs. A non-physical speedup is a bug in the measurement, never
a win; it went in the bin, not the baseline.

## 2026-07-29: real-magnitude refresh on settled main (HVF, release), and the per-core default

The recorded `--real` magnitudes above predated the §22/§26/§27/§28/§30/§31/§32 wave, so they were
rerun on settled `main`. Two harness changes came out of it, and they are the frame for the numbers.

**`--real` is now single-hart by default.** A primitive magnitude is a per-core number, and the
cross-OS table reads it that way (against Linux `fork`, lmbench, seL4, all per-core). The wave made the
default `--real` boot `-smp 4`, and the machine showed why that is the wrong default for a primitive:
the reap-heavy ones inflate and go noisy under cross-core reap lag that has nothing to do with per-core
cost. `spawn_el0` reads **~4.4 us on one hart and ~13.6 us on four** (and swings widely there);
`spawn_reap` is ~1.3 us on one hart and 11-160 us on four. So `--real` now pins `-smp 1` like the icount
instrument, for the same reason, and `--real --smp` boots the whole machine for the throughput bench
above. The single-hart run is the per-core signal; the four-hart run is for scaling, not for reading a
primitive's latency.

**The refreshed per-core numbers** (HVF, `--release`, `-smp 1`, medians of 5 boots, ns/iter):

| primitive | 2026-07-29 (per-core) | previously recorded | what moved, and why |
|---|---|---|---|
| `null_syscall` (EL0) | ~27 | ~27 | unchanged |
| `ipc_rtt_el0` (EL0) | ~361 | ~337 | **+7%**, the milestone-22 §26 mailbox widening 3->5 words; matches the icount +5% exactly, real and expected |
| `ctx_switch` (EL0, round trip) | ~112 | ~28/switch (~56 rt) | ~29 ns/switch derived, unchanged |
| `map_el0` (mechanism, aliased) | ~92 | ~91 | unchanged |
| `map_new` (provision + map) | ~470 | ~524 | within run-to-run noise; still zeroing-bound |
| `spawn_el0` (EL0, build+run+reap+reclaim) | ~4,400 | ~7,700 | **lower**, see below |
| `spawn_reap` (kernel-side) | ~1,300 | ~2,800 (debug) | lower; the old figure was a debug single-run |
| `ipc_rtt` (kernel-side) | ~50 | ~705 (debug) | the gap is the debug->release tax, not a change |
| `call_reply` (kernel-side) | ~66 | ~886 (debug) | same, debug->release |
| `yield_switch` (kernel-side) | ~32 | ~437 (debug) | same, debug->release |
| `coremark` (per iteration) | ~8,700 | n/a | pure compute, invariant across the wave (the smp=4 artifact check) |

Two lines need a word.

`ipc_rtt_el0` is the one clean, real movement: **+7%**, and it lands exactly where the icount baseline
put it (+5%), which is the §26 fault-message carrier widening the mailbox from three words to five so
every send and recv copies five. Small, expected, paid for a feature, and the two instruments agree,
which is the cross-check working.

`spawn_el0` reads **lower** now (~4.4 us) than the recorded ~7.7 us, and honesty demands the caveat
rather than a victory lap. The icount path length for spawn_el0 rose ~11% over the wave (the §31 SPLIT
rights inheritance), so this is **not** a path-length speedup. It is that spawn is the noisiest
primitive (it reaps a child every iteration) and the recorded 7.7 us was a single, busier-machine
sample; the settled per-core median is ~4.4 us with low variance, and the same primitive is ~13.6 us at
four harts. Read 4.4 us as the refreshed stable per-core figure, not as a claim that spawn got faster.
The cross-OS story is unchanged either way: still faster than Linux `fork`+`exit`, with the "a
capability process is a lighter object than a Unix one" caveat that has always stood.

Nothing here needed a path investigated. The only structural change was the harness (`--real` boots
one hart now), and the one real code-attributable movement (`ipc_rtt_el0` +7%) is the mailbox, agreeing
across both instruments.
