# Calibration against L4 and seL4

*An appendix to [`notes/benchmarks.md`](../benchmarks.md), which carries the current numbers and is written so a reader can act without opening this file. This one holds what the IPC numbers mean next to seL4's published cycles, and why sel4bench cannot run on this host, with the dates, tables and corrections behind them. Name provisional (`notes/benchmarks/` and this stem), minted 2026-09-24 by the lane that split the note; naming is calef's.*

## The first real numbers, for the record (2026-07-23, M-series host, HVF, **debug build**)

IPC round trip ~705 ns; call/reply ~886 ns; yield round trip ~437 ns; spawn-to-reap ~2.8 µs;
fresh-page map ~634 ns. **These are debug-build numbers**, which is easy to miss and cost this note
a wrong comparison for twelve days (see the calibration section below). Statistical, single run,
shared machine: shapes, not gospel. The 24 MHz
counter grain (~42 ns) means per-iteration ticks are coarse; totals over 1000+ iterations are
what to read. Cycle-exact PMU numbers arrive with milestone 16's real silicon, which inherits
this harness and swaps the clock.

## Calibration: what these numbers mean next to L4's (corrected 2026-08-04)

IPC cost is *the* microkernel number because IPC multiplies through the whole architecture:
Mach's ~100 us IPC discredited microkernels in the 1980s, and Liedtke's L4 rehabilitated them
with ~250-cycle IPC on a 486, the "sub-microsecond" banner seL4's few-hundred-cycle fastpath
still carries. So this is the paragraph a reader quotes, and from 2026-07-23 to 2026-08-04 it was
wrong.

### The three errors in the old paragraph, and why they hid each other

The first version of this section converted `ipc_rtt`'s ~705 ns at an assumed 3.2 GHz to ~2,200
cycles and reported us "4 to 7 times heavier" than an L4-lineage fastpath's 300 to 600. Three
independent defects, and the reason nobody caught them is that they do not point the same way:

1. **Wrong plane.** `ipc_rtt` is kernel-side. Two kernel threads call `sched::ipc_send/recv`
   directly, in one address space, taking no trap. L4's published numbers are user-to-user across
   address spaces with the trap included. The benchmark that pays what theirs pays is
   `ipc_rtt_el0`, and it has existed since the EL0 primitive suite landed; this note reports it
   further down the page, and the calibration never followed. **Fixing this alone makes us look
   worse.**
2. **Wrong build.** ~705 ns is a **debug** figure. L4's are optimized builds, and the debug-to-
   release tax on the IPC path is ~6.7x (measured, in the cross-OS section below). **Fixing
   this alone makes us look better**, by more than the plane correction costs.
3. **Wrong convention, which is the one nobody had noticed at all.** seL4 publishes **one-way**
   costs, the call and the reply timed as separate operations. Ours is a **round trip**. Comparing
   our round trip against their one-way figure doubles the ratio for free.

Errors 1 and 3 inflate the ratio and error 2 deflates it, so "4 to 7 times heavier" landed in a
plausible-looking place by cancellation rather than by being right. That is the failure mode worth
remembering: an arithmetic chain over figures taken from different runs can be wrong in three ways
and still read as sober.

### One clean run, both planes (2026-08-04)

`cargo xtask bench --release --real`, Apple M3 (Mac15,3, 4 P-cores + 4 E-cores), HVF, `-smp 1`,
five boots back to back off one build. Host load average was ~3.2 of 8 cores throughout (other
agent lanes were active on this machine), which is not a silent machine; the run-to-run spread
below is the evidence that it did not matter much here. Both figures come from **the same boot** on
every boot, which is the property the old comparison lacked.

| bench | plane | ns/iter, median of 5 | the five | what one iteration includes |
|---|---|---|---|---|
| `ipc_rtt` | kernel-side | **46** | 46, 46, 46, 46, 49 | two rendezvous, one address space, no trap |
| `ipc_rtt_el0` | EL0 to EL0 | **350** | 347, 348, 350, 357, 369 | two rendezvous, two address spaces, four `svc`s |
| `null_syscall` | EL0 | 27 | 27, 27, 27, 27, 28 | one trap and return, for scale |

These agree with the 2026-07-29 refresh (~50 and ~361) to within run-to-run noise, so nothing has
moved; what was missing was never a measurement, it was the paragraph.

### The comparison, done on the right number

**Cycles here are arithmetic, not a reading.** HVF passes through no PMU (notes/pmu.md), so a cycle
count is nanoseconds times an *assumed* clock. The old paragraph assumed 3.2 GHz, which is not this
machine: the host is an Apple M3, 4.05 GHz on a P-core and 2.75 GHz on an E-core, and nothing pins
the QEMU vCPU thread to either. So the range, not a point:

| | ns | cycles at 2.75 GHz | cycles at 4.05 GHz |
|---|---|---|---|
| `ipc_rtt_el0` round trip | 350 | ~960 | ~1,420 |

seL4 publishes, for the same-core different-address-space path, **413 cycles for the IPC call and
426 for the IPC reply**, one-way each (sel4.systems/performance.html). Which machine that is matters
and the page does not put it up front: it is the **Jetson TX1, a Cortex-A57 at 1.9 GHz**, in seL4's
default configuration, and it is the *only* aarch64 platform the performance page carries
(notes/aarch64-board-survey.md, read 2026-08-13). A round trip in our sense is their call plus their
reply, **~839 cycles**.

**So the corrected figure is roughly 1.1x to 1.7x an L4-lineage round trip, not 4 to 7 times.** And
converting the *debug* EL0 number instead (~2272 ns) gives ~6,200 to ~9,200 cycles and a ratio of
~7x to ~11x, which is the same class of mistake in the other direction: a debug number in a release
comparison.

### Why that is not a win, and what is still not apples-to-apples

Recording a better ratio than the page used to claim is worth less than recording what it does not
mean, so the caveats are the substantive half of this section.

- **Cortex-A57 is 2015 silicon and the M3 is 2023.** A cycle is not a neutral unit across a decade
  of microarchitecture: the M3 is far wider and deeper, and an IPC path is mostly serially
  dependent loads, stores and unpredictable branches, exactly the workload a bigger out-of-order
  window helps. **A large part of the gap closing is the machine, not the kernel.** The only fix is
  the same kernel measured on comparable silicon, which is what milestone 24's board and milestone
  74's PMU are for.
- **Their fastpath is on and we have none.** seL4's published figures are its best case, with the
  `Call`/`ReplyRecv` fastpath enabled. Ours is the fully general path every time: scheduler lock,
  proved rendezvous, generational Tid checks. Read the ratio as "the general path is within a small
  factor of a tuned fastpath on this silicon", which is a statement about headroom, not about
  having matched them.
- **Their round trip is two syscalls and ours is four.** seL4 fuses send-and-wait into `Call` and
  reply-and-wait into `ReplyRecv`; our EL0 path issues `SEND`, `RECV`, `SEND`, `RECV` (see milestone 188's correction below: that is the `ipc_rtt_el0` benchmark, and a real service issues `CALL`, `RECV_CAP`, `REPLY`, which is three). At ~27 ns
  (~110 cycles) per trap, the two extra crossings are ~220 cycles, a sixth to a quarter of our
  round trip, and they are self-inflicted rather than structural. The kernel-side `call_reply` bench
  measures the fused shape, but **there is no EL0 twin of it**, so the structurally matched
  comparison to seL4's published pair is not currently measured at all. That gap is named here
  rather than papered over.
- **Different measurement methods.** seL4 times a single operation through the PMU with the caches
  hot and a measured overhead subtracted; we average a 5,000-iteration loop against a 41.67 ns
  counter tick. Both are legitimate (notes/pmu.md, the two-clocks section) and they do not fail the
  same way.
- **We run under a hypervisor and they do not.** The tax on this particular path is small (no
  devices touched, so essentially no VM exits; the cost is indirect, via stage-2 TLB pressure and
  host cache pollution), which is precisely why the bench loops keep devices out. Small is not zero.
- **The clock is assumed.** Every cycle figure above inherits a ~1.5x uncertainty from not knowing
  which core type the vCPU thread ran on. Milestone 74 (cycle counters) exists to retire this, and
  until it lands no cycle ratio from this note should be quoted tighter than "same order".
- **Both sides will be measured in a benchmarking build, and ours is the slower one** (milestone
  237, the cycle-counter grant as a measurement build). Reading `PMCCNTR_EL0` at EL0 needs milestone
  229's per-thread grant, which the context switch enforces, and that grant is behind
  `--features cycle_counter_grant` rather than in the shipping kernel: measured, it costs
  `sched::schedule` 136 bytes, taking `script/fastpath-footprint`'s aarch64 `ipc_fastpath` closure
  from 5852 to 5988. So **any cycle figure taken with the instrument on is slightly pessimistic
  about nife**, and the direction is the right one to err in for a comparison we intend to publish.

  This is the same rule milestone 221 (a soak that crosses cores) records for the soak, which is
  that a soak number is comparable only with another soak number. It does not cut against us here,
  because **seL4's published 413 and 426 come from a benchmarking build too**. `sel4bench` reads
  `PMCCNTR_EL0` from user level, which on Arm needs `KernelArmExportPMUUser`, and seL4's own
  configuration reference describes that option as *"Grant user access to the performance monitoring
  unit. While useful for benchmarking, this option opens the possibility of timing channels"*, with
  a default value of `OFF` (docs.sel4.systems/projects/sel4/configurations.html, read 2026-09-03).
  So comparing our benchmark kernel against theirs is like for like; comparing our production kernel
  against their benchmark one would not be.

**The kernel-side number stays, and it is a real thing to know.** `ipc_rtt` at 46 ns (~130 to ~190
cycles) is the honest cost of the kernel's own rendezvous, and it is the right instrument for the
job it has: the icount tripwire gates on it, and a change to the kernel's IPC code moves it next to
its commit. It is not a comparison number. Putting it beside an 839-cycle round trip would compare a
path with no trap and no address-space switch against one that has both, which is the original error
of this section; deleting it would be a second error, because the gate needs it and the ~300 ns gap
between the two planes *is* the trap cost, which is itself a number worth having.

What the comparison legitimately supports, in one sentence: the Mach failure mode is nowhere in
sight, the general path is viable at this price, and whether a fastpath is ever worth its complexity
is a question for these measurements rather than for L4 envy.

### Where the two nanosecond figures for `ipc_rtt` came from

This note has carried **~705 ns** (2026-07-23) and **~951 ns** (2026-07-25) for the same kernel-side
benchmark, which reads as a contradiction and is not one. Both are debug, both are single runs, and
they are different binaries on different days: the 19f object-capability refactor landed between
them and genuinely grew the scheduler and thread hot paths, on top of the whole-crate codegen drift
this note documents below. Neither was wrong when it was written. **The defect was leaving both on
the page with no dates and no build profile attached**, which is what let a comparison quote the
older one for eleven days. Every figure in this note now carries its build; that is the fix.

### seL4: built and booting, but stopped by the PMU wall (deferred to real hardware)

`sel4bench` was built (seL4 kernel + the benchmark app suite, for `qemu-arm-virt` aarch64, `RELEASE`
and `FASTPATH` on, i.e. seL4 at its best) and it boots on this Mac under both QEMU-TCG and QEMU-HVF.
It **cannot produce valid numbers here**, and the reason is worth recording because it is the same
constraint the roadmap called out for our own silicon-cycle plans.

sel4bench times a **single operation** per sample (one `seL4_Call`, `RUNS` samples) and reads the
**PMU cycle counter**, `PMCCNTR_EL0`, before and after (notes/pmu.md explains the PMU and why it is the
counter that does not survive virtualization). That needs a real, high-resolution cycle counter
(~0.25 ns per tick at ~4 GHz). Neither virtualization mode on this host provides one:

- **QEMU-TCG** does not model a cycle counter; `PMCCNTR` returns quantized junk (we saw 0 and 1000),
  and sel4bench's own stability check refuses to continue ("*Benchmarking overhead of a call is not
  stable*"). Milestone 74's aarch64 half measured the same thing from this kernel's side on
  2026-09-19, once it started the counter: without `-icount` it advances in steps of 1000 (about 32
  per `CNTVCT_EL0` tick), and under `-icount` it is the instruction count (exactly 16 per tick, which
  is `script/icount`'s `instructions_per_counter_tick`). Neither is a cycle; see notes/pmu.md.
- **QEMU-HVF** on Apple Silicon does not virtualize the guest PMU, so `PMCCNTR` is unstable there too,
  and the same check stops the run.

The only counter HVF passes through is the architected virtual counter, `CNTVCT_EL0`, at the host's
24 MHz `CNTFRQ`, which is **41 ns per tick**, far too coarse to resolve one ~50 ns IPC in a single
shot. (`CONFIG_ALLOW_UNSTABLE_OVERHEAD` forces sel4bench past the check, but then the numbers are the
same junk, so it buys nothing.)

**This validates our own measurement design rather than undermining it.** Our bench works under HVF for
exactly the reason sel4bench does not: we read `CNTVCT` (which HVF passes through) and we time a **loop
of thousands** of operations per sample, so the coarse 41 ns tick is averaged away. sel4bench's
single-shot-PMU method is precisely what cannot survive this virtualization tier. Getting a same-machine
seL4 number would mean either rewriting sel4bench to our method (CNTVCT plus batched loops, real surgery
on its measurement core) or giving it a real PMU.

**So the seL4 comparison is deferred to real hardware**, which also aligns with the planned second-board
port (design/roadmap/24-second-aarch64-board.md): a Raspberry Pi has a real PMU, sel4bench runs on it natively, and
it is the board nife is heading toward anyway. The build recipe, reproducible when a Pi is on hand
(rebuild with the Pi `PLATFORM` instead of `qemu-arm-virt`), via the official seL4 Podman image:

```
podman pull docker.io/trustworthysystems/sel4        # ~3.6 GB, bundles repo/cmake/ninja/aarch64-gcc
mkdir sel4bench && cd sel4bench
podman run --rm -v "$PWD":/sel4bench:Z docker.io/trustworthysystems/sel4 bash -lc '
  cd /sel4bench
  repo init -u https://github.com/seL4/sel4bench-manifest.git && repo sync -j4
  mkdir build && cd build
  ../init-build.sh -DPLATFORM=qemu-arm-virt -DAARCH64=TRUE -DSIMULATION=TRUE   # -DPLATFORM=rpi4 for a Pi
  ninja'
# image at build/images/sel4benchapp-image-arm-qemu-arm-virt; run with build/simulate (qemu) or on the Pi
```

### The cross-OS comparison, when we build it

- **Reuse an existing primitive suite** where one exists: **lmbench** on Linux and macOS (it builds
  on both), **`sel4bench`** for seL4. We write the nife side (the microbenchmarks above,
  extended to match the metric set), not the whole thing.
- **The peers.** seL4 is the direct one: a capability microkernel that targets the *same* QEMU
  `aarch64 virt` machine we do, so it runs on the identical instrument (QEMU-HVF) and publishes
  comparable cycle counts. L4Re/Fiasco and Genode are more effort for less marginal insight.
- **Match the virtualization tier.** QEMU with `-accel hvf` *is* virtualization (Hypervisor.framework
  on the real core), not emulation, so nife and Linux run virtualized under QEMU-HVF; macOS runs
  as a guest under Apple's Virtualization.framework (same underlying hypervisor, different VMM shell);
  native macOS is the bare-metal ceiling reference. For guest-internal microbenchmarks the VMM layer
  is off the hot path (no VM exit on a null syscall or context switch), so the QEMU-vs-VZ difference
  is a footnote, not a confound.
- **XNU is a hybrid, name it.** macOS's kernel has a Mach microkernel core but runs BSD and drivers
  *in* the kernel, so most macOS syscalls are in-kernel BSD calls and Mach IPC is not on the hot path
  the way our endpoints are. Comparing "our IPC" to "macOS syscall latency" measures two different
  things; saying so is part of the honesty.
