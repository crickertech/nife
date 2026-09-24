# E1 to E4: the cache experiments and what each reading says

*An appendix to [the register of measures](../register-of-measures.md). The name is provisional.*

**E1, E3 and E4 re-taken on radon, 2026-09-04, which is the board all three were designed
against.** Six boots on a `board,bench,single_hart` card, interleaved unpadded and padded, one
microSD card written six times. Every boot printed `cntfrq 4000000` and reached `bench: done`;
nothing self-skipped. The capture is `bench/radon-2026-09-04/` and the reading is
notes/footprint-perturbation.md.

- **E1 found a knee at 16 threads**: 1.00x, 1.03x, 1.20x, **1.68x**, then flat to 96 (the last four
  points span 1.2%). The prediction was a knee in the low tens, computed against radon's 32 KB L1i
  by name. Recorded against `design/decisions/96-process-kernel-or-event-kernel.md`'s input 3, with
  the caveat that E2's customer path runs 4 to 8 threads, below the knee.
- **E4 found displacement that is real and load-dependent**: 0 to 1% at ordinary IPC load, **5 to
  8% at 96 threads**, peaking at a 32 KiB working set, which is radon's L1d exactly. Three times
  the magnitude the dev Mac read, with a peak on the cache size rather than at an end of the sweep.
- **E3 separated cleanly and cannot be attributed.** The padded build is 1.49% slower on
  `call_reply` and **3.01% faster** on `ipc_rtt_el0`, both non-overlapping across three boots each.
  Resident dead code cannot make anything faster, so the experiment is measuring code layout summed
  with footprint and reporting the total. **Do not quote an E3 number as a footprint result**: the
  control milestone 370 specifies was built on 2026-09-19 and the evening that uses it has not been
  run, so E3 has no attributable number at either date. This flaw
  was present in the 2026-08-22 dev-Mac reading too; the small cache made it visible rather than
  causing it.

**The per-IPC kernel stack depth, taken 2026-09-19, retires E1's one estimated input.** E1's
prediction assumed "roughly 1 to 2 KiB" of kernel stack per IPC. Measured by painting each
thread's own stack around each operation: in the **release** kernel radon boots, one round trip
reaches about **600 bytes** of each kernel thread's stack (riscv64: 608 client, 576 server, E1's
SEND/RECV shape), and 0.5 to 1 KiB for EL0 threads, trap frame included; the debug build reaches
2 to 4 KiB. It is `dated` rather than gated because it had never existed before and nothing
depends on its value yet. **What it changes about E1 is the reading, not the curve**: at 600 bytes
a thread, stacks fill a 32 KB L1d near 54 threads, not 16, so capacity does not explain radon's
knee at 8 to 16. Page-aligned stack tops sharing set indices would (at most 8 lines per page offset
in radon's 32 KiB 4-way L1D), and so would page-aligned TCBs. notes/stack-high-water.md carries the
arithmetic and the colouring experiment that separates the two.

**E1 through E4, taken 2026-08-22 (milestone 134's Tier A lane).** All four ran on the dev Mac
under HVF; none of the four needed silicon, which is what the block promised, but three of them
(E1, E3's latency half, E4) need a *real cache* to say anything, and this tree's only accelerator
with one is HVF, so they self-skip under TCG (both the default icount instrument and every riscv64
run, which has no HVF equivalent) rather than print a number that would be fiction. E3's static
footprint measurement is not in that boat: it is `objdump`-based and runs on both ISAs with no
QEMU at all.

- **E2 (cheapest, taken first, and it is close to decisive).** The naive reading of
  `sched::thread_count()` inside the SMB/FS gate test read 95 (aarch64) and 82 (riscv64), and both
  numbers were wrong for what E2 asks: the full kernel test suite runs 279 `#[test_case]`s in ONE
  continuous boot, so an absolute count taken partway through includes whatever earlier tests left
  allocated. The delta against a baseline taken at the top of the SAME test (before it wires
  anything) is the number that isolates the topology's own cost, and it is **4 new threads on both
  ISAs**: `net_stack`, the echo client, the SMB adapter, the mDNS responder. The FS service (block
  server + FS server) and the credential service add nothing to the delta because they are already
  running, latched from earlier tests in the same boot and reused rather than re-spawned, which is
  itself informative: a from-scratch boot would add roughly three more (a generous estimate is 7 to
  8 total). Either number is deep in single digits, nowhere near E1's knee (below). **§96 is moot
  for this workload as currently shaped**, which is the finding E2 was raised to check for before
  spending on E1 at all.
- **E1.** `ipc_scale_N` (N = threads, 2 to 96, via `tp_batch`/`tp_best` pinned to one hart) is flat
  at roughly 1,270 to 1,310 ns/iter from 2 through 16 threads across three repeated runs, then
  rises, reproducibly, to roughly 1,360 to 1,420 ns/iter (8 to 11%) by 64 to 96 threads. The knee
  starts where the prediction said it would (the low tens) and the magnitude is small because the
  dev Mac's L1d is far larger than the 32 KB `SiFive` U74 the prediction was built against: a
  reproducible cost at 96 threads on a large-cache machine is the "positive result is conclusive"
  case the block's own BUGS names, and it argues for taking this to a small-cache board rather than
  against the mechanism. Read against E2: the customer path (4 to 8 threads) sits inside the flat
  region, well below where any cost appears on this machine.
- **E3.** The static half: padding roughly doubles `ipc_fastpath` on both ISAs (aarch64 5,792 to
  11,628 bytes, 2.01x; riscv64 5,088 to 10,152 bytes, 2.00x), confirming the padding mechanism does
  what it claims before asking whether it costs anything. The latency half, aarch64 only: `ipc_rtt`
  and `ipc_rtt_el0` move by 2 to 3% between the padded and un-padded builds, which is inside the
  run-to-run noise both builds show independently (repeated `--real` runs of the SAME binary vary
  by a similar amount). **No effect, on this machine**, which the block's own BUGS calls the weak
  direction: the dev Mac's L1i comfortably holds 11.2 KiB, so a negative result here proves little
  and wants the same small-cache board E1 does.
- **E4.** With a 5-repeat minimum (the first unrepeated run swung 2 to 3x between nominally
  identical conditions, a real methodological finding in its own right: this workload's batches run
  long enough to catch host preemption the way `smp_throughput`'s shorter ones do not), throughput
  lost to 8 concurrent IPC pairs (16 threads) is 0 to 5% across working sets from 4 to 128 KiB, over
  three repeated runs (widened from the first cut's "0 to 3%" by two more runs on the same shared,
  noisy machine; see this section's own bug about that). **No effect worth calling one, on this
  machine**, and it is not in tension with E1: 16 threads of background traffic sits inside E1's own
  flat region, below where E1 itself found any cost on this hardware.

  **The stronger follow-up this section originally deferred was taken 2026-08-23.** A second
  background-load condition, `SCALE_MAX_PAIRS` (48 pairs, 96 threads, the same pair count E1's own
  sweep tops out at), was added to `app_displacement` alongside the original 8-pair one. Over the
  same three repeated runs: throughput lost at 96 threads reads 2 to 9%, higher than the low-load
  figure at every one of the 5 working sets on every one of the 3 runs, no exception. That is a
  small, reproducible, direction-consistent effect, not a knee: the two ranges (0-5% and 2-9%)
  overlap, and E1's own equivalent point (94-96 threads) only shows an 8-11% cost, so a modest
  displacement number at the same load is the mutually consistent reading rather than a stronger
  independent finding. It wants the same small-cache board E1 and E3's latency half already want,
  because on this machine the two conditions are separated by a few percentage points riding on top
  of run-to-run noise of a similar size.

**E1, E3 and E4 can now reach a board, and have not, 2026-09-04.** The three rows above whose
2026-08-22 readings all end with "and it wants a small-cache board" now have one they can run on.
E1 and E4 were compiled for aarch64 only, and the reason recorded in `bench.rs` was that this tree
has no riscv64 *accelerator* with a real cache. That is true and was never the same statement as
"no riscv64 machine with a real cache": radon is a VisionFive 2 whose four U74s have 32 KB L1i and
32 KB L1d each, which is the number E1's prediction was computed against by name. What was missing
was a path to running there, and it is now three things: the two benchmarks build for riscv64, the
`single_hart` kernel feature parks the other three harts (both experiments need one hart, and a
card has no `-smp 1` to pass), and `script/board-image --bench` writes the card. The procedure and
its outcome table are **notes/footprint-perturbation.md**.

**The date column above says "never on a board" rather than being left to a reader's inference**,
which is this file's own complaint about the cross-OS row, applied to itself. Nothing here has been
measured on silicon; the board was powered off and there was no bench session on the day the
instrument was made runnable.

**And one thing E3 measures changed underneath it.** The padding was reachable only from
`sched::ipc_send`, so with milestone 188 phase 1's split (2026-09-04) it moved `ipc_send_recv` to
2.10x and `ipc_call_reply` to **1.00x** on riscv64: E3 was doubling the footprint of the shape
nothing in this tree runs, while leaving untouched the shape every service issues. `sched::ipc_call`
now pads too, and both shapes read roughly 1.85x on both ISAs. The 2026-08-22 E3 reading was correct
when taken and describes a quantity that no longer means what it said, which is the exact failure
this register exists to make visible.

## BUGS

- **E1, E3's latency half, and E4 need a real cache, and this tree has one accelerator that
  provides one.** "Tier A needs no silicon" is true of the experiments' *design*, but a Rust
  benchmark still needs somewhere with real caches to run on, and today that is HVF, aarch64-only.
  `cargo xtask bench --riscv` always runs under TCG (no riscv64 accelerator exists in this tree),
  so all three self-skip there rather than print a fiction, the same self-skip shape
  `real_single_hart_or_skip` in `kernel/src/bench.rs` already uses for the icount case. This is not
  the Tier B kind of gap (a counter or an authority question that does not exist yet); it is that
  this specific instrument needs hardware this tree already has, on one architecture. E3's static
  footprint measurement is unaffected: it is `objdump`-based and needs no accelerator on either ISA.
  Milestone 127's board, when it lands, is the natural second data point, not a blocker for a
  first one.

- **E2's naive reading was wrong, and finding that out is itself worth recording.**
  `sched::thread_count()` taken partway through the full test suite (279 `#[test_case]`s in one
  continuous boot) read 95 and 82 on the two ISAs, both dominated by threads earlier, unrelated
  tests left allocated. The fix (a baseline taken at the top of the same test, the census reported
  as a delta) is in `kernel/src/user/tests.rs` and `riscv_virtio_tests.rs`; any future instrumented
  count of "how many threads does X create" taken from inside the shared-boot suite should take the
  same delta rather than trust an absolute `thread_count()` reading.

- **Closed 2026-08-23.** E4's original background load (8 pairs, 16 threads) sat inside E1's own flat
  region, so a null result there was expected from E1's curve rather than independent evidence
  against displacement. `app_displacement` now also runs 48 pairs (96 threads, E1's own top pair
  count) as a second condition; see this file's E4 narrative above for the result, a small
  reproducible cost consistent with rather than stronger than E1's own finding at the same load. Not
  a knee, and the honest reading is that this machine's cache is still too large to see one; that
  wants the small-cache board, not another thread-count sweep here.
