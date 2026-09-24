# Cross-OS primitives: nife, Linux and macOS

*An appendix to [`notes/benchmarks.md`](../benchmarks.md), which carries the current numbers and is written so a reader can act without opening this file. This one holds the EL0 primitive suite, the first cross-OS table, and the map tie and spawn caveats, with the dates, tables and corrections behind them. Name provisional (`notes/benchmarks/` and this stem), minted 2026-09-24 by the lane that split the note; naming is calef's.*

## Compute vs. OS primitives: two benchmarks that measure different things (milestone 19e)

The microbenchmarks above are the *right* kind for a microkernel: IPC, context switch, the paths a
microkernel lives on. But "run a real workload" (19e) wanted a whole compute program, and thinking
through how to compare it across OSs turned up a distinction worth pinning down, because it decides
what any cross-OS comparison can and cannot show.

**Compute is OS-independent.** A tight compute loop, once it is running in userspace, does not touch
the OS: the CPU executes the same instructions no matter who scheduled it. So a compute benchmark
(CoreMark, Dhrystone) run on nife, macOS, and Linux on the same core comes out *nearly
identical*, and the small gaps are compiler codegen or allocator noise, not OS quality. That is a
real result ("we add no hidden compute overhead") but a null one by design. It cannot show OS
strengths or liabilities, because the OS is not in the loop.

**OS primitives are where an OS shows itself.** Syscall entry, context switch, IPC round-trip, page
map, page fault, thread spawn: these *are* the OS, and they are what distinguish Linux from macOS
from us. But the same source cannot measure them across three OSs, because "the same syscall" does
not exist on all three: you invoke each OS's own primitive (`getpid` on Linux, a Mach/BSD call on
macOS, our `svc` null-invoke). So the OS-revealing benchmark is a **matched harness per OS** (one
metric definition, three native implementations), which is exactly what lmbench is and how the
L4/seL4 papers compare to Linux. Our own microbenchmarks above are the nife side of it.

### The CoreMark workload (`crates/coremark`, `fixtures/src/coremark.rs`)

19e's real workload is CoreMark, the three work items of a CoreMark iteration (a linked-list sort, a
small-matrix multiply, a state machine over a byte buffer), each folded into a CRC so the compiler
cannot delete the work and a run self-validates. It runs as a spawned EL0 program against the native
ABI: the progenitor builds the `"coremark"` binary, grants it one endpoint, and it computes and SENDs the run's
CRC home. `coremark::PINNED_CRC_64` (`0x7954` for 64 iterations) is asserted by both the host crate
test and the kernel test, so the same computation gives the same answer on the host and on the
kernel's target, which is the property a cross-OS comparison rests on.

It is a **Rust reimplementation, not EEMBC-certified CoreMark**: a certified score needs the
unmodified reference C. The Rust choice buys the thing that matters for *our* comparison, that the
identical source compiles for nife, macOS, and Linux, so the compute run is one program on
three OSs. This binary reports correctness, not yet a score; timing a run needs a userspace clock
(enabling the EL0 virtual-counter read, as Linux does for its vDSO), which lands with the cross-OS
suite rather than here.

### The measurement plane: kernel-side (gating) vs EL0 (cross-OS)

A subtlety that decides comparability, found while starting the primitive suite. The microbenchmarks
at the top of this note run in **kernel context**: the bench threads are kernel threads calling
`sched::yield_now` and `sched::ipc_send/recv` directly, so they measure the kernel-internal path
length of each operation. That is exactly right for their job (regression gating: a code-path change
moves the count next to its commit). But it is **not** what lmbench measures. lmbench runs a
*userspace* program making real syscalls, so its numbers include the EL0→EL1 trap and return that a
kernel-side benchmark skips entirely.

So the cross-OS primitive numbers have to be measured **from EL0**, a userspace program that self-
times a loop of real `svc` syscalls, to be comparable to lmbench. That is why milestone 19e opened
EL0 access to the virtual counter (`CNTKCTL_EL1.EL0VCTEN`; `user_mode_runtime::now`/`cntfrq`; notes/abi.md):
userspace self-timing is the prerequisite for a fair comparison. The CoreMark workload is the first
program to use it, self-timing its run and reporting `[crc, ticks, freq]`; the EL0 primitive
benchmarks (null syscall, context switch, IPC round-trip, page map, all measured the lmbench way)
build on the same `user_mode_runtime::now`. The existing kernel-side suite stays, for gating; the EL0 suite is additive, for
cross-OS honesty. The two will differ by roughly the trap cost, and that difference is itself a
number worth having.

### The first EL0 numbers (nife, M-series host, HVF, debug build)

The `os_primitives_benchmarker` program (`fixtures/src/os_primitives_benchmarker.rs`), spawned by the bench boot, self-times each primitive
from EL0 and reports it as a normal bench line. So far:

| primitive | HVF ns/iter | what one iteration is |
|---|---|---|
| `null_syscall` | ~42 | one `svc` that the kernel rejects immediately: trap + dispatch + return |
| `ctx_switch` | ~692 | one `SYS_YIELD` to a peer *process* and back: two switches, address space included |
| `ipc_rtt_el0` | ~2272 | a `SEND` to a server process and a `RECV` of its reply: two rendezvous, four `svc`s |
| `map_el0` | ~909 | `invoke(aspace, MAP_INTO, va, frame, RO)`: trap + cap resolve + walk + PTE + record |

Two sanity checks pass. A context switch is ~16x a null syscall (two traps, the scheduler, two
register save/restores, and a TTBR0/ASID change, versus one bare trap). And the round trip lines up
against its parts: ~two context switches (2 × 692) plus four traps (4 × 42) plus dispatch ≈ 2272.

The EL0 round trip also has a kernel-side twin, the milestone-21 `ipc_rtt` (~951 ns in this same
2026-07-25 debug run; the ~705 ns from 2026-07-23 is a different debug binary, see the calibration
section), which measures the same rendezvous *without* the EL0↔EL1 crossings. The ~1.3 µs gap between
them is exactly the trap cost of the four `svc`s a real round trip pays, which is the reason the EL0
numbers, not the kernel-side ones, are what compare to lmbench. **All debug builds, and every figure
in this subsection is one**; the cross-OS comparison and the L4 calibration both want release builds
on all sides, and quoting a debug figure into either is the mistake the calibration section above
records. These line up against lmbench's `lat_syscall` / `lat_ctx` / `lat_pipe` and `sel4bench`.

**Map (lmbench's `lat_mmap`) behaves differently from the other three, and it is the primitive where
the honest answer is a tie, not a win.** It taught three things.

First, it *consumes resources per call*: every `MAP_INTO` writes a page-table entry and a revocation
record, paid from the target space's untyped region, so unlike a null syscall or a yield it cannot loop
forever. The loop is bounded (500 maps, one L3 table's worth); the kernel-side twin `map_new` maps 64.
And there is no unmap in the surface yet, so each VA is used once.

Second, the debug and release numbers diverged by ~10x, far more than any other primitive, and that
divergence is the whole lesson. `map_el0` **aliases one existing frame** at every VA, so it does no
page allocation and no zeroing: it is trap + capability resolve + walk + PTE write + a `record_mapping`
append. That append scans the head log page for a free slot, an ~85-entry linear walk on average, and
in a debug build that unoptimized scan *dominated* the number (~909 ns). Release compiles the scan down
to almost nothing, and the true cost of the mapping mechanism shows through: **~91 ns**. The kernel-side
`map_new`, by contrast, is ~524 ns in release and barely moved from debug, because its cost is the 4 KiB
**page zeroing** a fresh frame needs (`retype_page` hands back a zeroed page), which is memory-bandwidth
bound and the optimizer cannot speed it up.

That average walk length is a constant in `kernel/src/revoke.rs`, `LOG_ENTRIES`, and it was ~128 when
the ~909 ns above was measured: a log page held 255 records until §132 gave each one a third word
naming the capability it was made under, which took the page to 170. **So `LOG_ENTRIES` is a benchmark
input, and the icount tripwire is the thing that noticed.** §132's branch came in at `map_el0` -16.9%
aarch64 and -16.4% riscv64 against a baseline nothing else on the branch explained, and the
attribution is a one-number experiment rather than an argument: setting `LOG_ENTRIES` to 170 on `main`
and changing nothing else reproduces it to within 0.7%. The unoptimized suite is measuring the search
for a free slot roughly as much as it is measuring the mapping, which is exactly what makes the debug
and release numbers diverge by 10x, and it means a future change to that constant moves two benches on
two ISAs. Whoever makes it should expect to re-record them.

Third, and this is why map is a tie: **`map_el0` and the host `lat_mmap` do not measure the same thing.**
The host number is a first-touch page fault, which allocates and zeroes a fresh page; `map_el0` aliases
a frame and skips both. So `map_el0` ~91 ns is the *pure mapping mechanism*, and it is genuinely lean,
but it is not comparable to Linux's ~534 ns, most of which is the page zeroing our aliasing avoids. The
apples-to-apples comparison is our `map_new` (fresh page, allocate + zero + map), ~524 ns, plus one trap
(~28 ns) for the EL0 crossing the host's fault includes: ~552 ns, against Linux ~534 ns and macOS ~556
ns. That is a **three-way tie**, and it makes sense: page provisioning is dominated by zeroing 4 KiB,
which is the same silicon and the same bandwidth for all three. nife's lean mechanism is real (the
91 ns), but on the operation an application actually pays for, getting a usable page, it does not and
cannot win, because the win would have to come from zeroing memory faster than the other two, and nobody
can. A fair EL0 map that *does* provision a fresh page waits on retype-from-untyped reaching userspace
(a later milestone); until then the kernel-side `map_new` is the honest stand-in for the comparison.

### The first cross-OS numbers (nife vs Linux vs macOS)

`bench/host/` holds the host side of each metric: `null_syscall.rs` (a raw `getpid` through the
syscall gate, not libc's cached `getpid` which never traps), `ipc_rtt.rs` (a pipe round trip between
two forked processes, lmbench's `lat_pipe`), `ctx_switch.rs` (the derived context switch),
`mmap.rs` (first-touch fault-in, lmbench's `lat_mmap`), and `spawn.rs` (fork+exit, lmbench's
`lat_proc`). Two ways to run them: natively on macOS
(`rustc -O ... && ./bin`), and on **Linux at the same tier** as nife, `bench/host/run_linux.sh`
cross-compiles a static musl binary (`linux_all.rs`, the five metrics combined), packs it as `/init`
in a one-file initramfs, and boots it under QEMU-HVF, the exact machine nife boots on. So Linux
and nife sit on the **same M-series core at the same virtualization tier**; native macOS is the
bare-metal ceiling.

Run nife optimized (`cargo xtask bench --release`, which builds an opt-level-3 kernel and
userspace and implies `--real`), and compare on the same core:

| metric | nife **release** (HVF) | Linux (static musl, HVF) | macOS/XNU (native) |
|---|---|---|---|
| null syscall | **~27 ns** | ~139 ns | ~76 ns |
| context switch (per switch, derived) | **~28 ns** | ~415 ns | ~818 ns |
| IPC round trip | **~337 ns** | ~1723 ns | ~2620 ns |
| map a fresh page (provision + map) | ~552 ns (`map_new` + trap) | ~534 ns | ~556 ns |
| map mechanism only (aliased, no zeroing) | ~91 ns (`map_el0`) | n/a (fault always zeroes) | n/a |
| spawn (build + run + reap + reclaim) | **~7.7 µs** (`spawn_el0`) | ~19.7 µs (fork+exit) | ~291 µs (fork+exit) |

**nife wins four and ties one, and saying which is which is the point.** Same M-series core, same
HVF tier as Linux, both optimized. It is **~5x faster than Linux at the null syscall** (27 vs 139) and
**~5x faster at the IPC round trip** (337 vs 1723), it beats native macOS at both, and it builds a
process faster than either (spawn, below). These are seL4-class microkernel numbers, an IPC round trip
in the low hundreds of nanoseconds, next to the reference OS on the same silicon. **"seL4-class" is a
claim about magnitude and nothing more**; what it is worth measured against seL4's own published
cycles, and the four ways that comparison is not apples-to-apples, is the calibration section above.
Quoting this sentence without that one is how the last overstatement happened. **Map is a deliberate
non-win**: provisioning
a page is dominated by zeroing 4 KiB, which is bandwidth-bound and identical across the three, so all
land near ~550 ns. The lean mapping *mechanism* (91 ns, measured by aliasing to strip the zeroing) is
real and worth recording, but it is not a page an application can use, so it does not go in the win
column. The map row above compares like with like (`map_new` provisions a fresh page, as the host fault
does); the ~91 ns sits below it as the mechanism floor, not as a headline.

**Spawn is a real win, and an honest caveat.** (The ~7.7 µs below is a 2026-09-21 reading; the
current-CPU page has since made the path 6.3% longer, implying ~8.2 µs and ~2.4x. See the dated
entry at the end of this file, which has the measurement and says which number is implied rather
than measured.) `spawn_el0` builds a whole child from EL0 (`SPLIT` a
region, retype an address space and a TCB, map code and a stack, configure, start), runs it to exit,
reaps it, and `DESTROY`s its region, all in a self-timed loop that only repeats because object
revocation reclaims each child (notes/object-revocation.md). At ~7.7 µs it beats Linux `fork`+`exit`
(~19.7 µs) by ~2.6x and macOS by ~38x, on the same core, and it does so while paying **more** boundary
crossings than Unix: ~10 `svc`s per spawn against `fork`+`wait`'s two. That the heavier-trapping side
still wins is the honest part of the result. The caveat is the operations differ: `fork` **duplicates**
the parent (its address space copy-on-write, its descriptor table, its signal state), where nife
**builds a fresh minimal process from nothing**. A capability-microkernel process is a lighter object
than a Unix one, so the gap is mostly that structural difference, not a faster version of the same work.
We use `fork`+`exit`, not `fork`+`exec`, precisely to keep the Unix side as light as it gets (no binary
loaded); it still carries the weight of duplication that nife's from-scratch build does not. The
number stands, with its meaning stated: building a process is cheap when a process is a small thing.

The **context switch** is the softest of the three and its number the least load-bearing. No OS lets
you time a bare switch, so it is *derived*: on the host, `bench/host/ctx_switch.rs` measures a
two-process pipe round trip (two switches plus two pipe passes) and subtracts a self-pipe pass (a
`write`+`read` with no switch), leaving one switch = `round_trip/2 - self_pipe`. nife's
`ctx_switch` bench is a yield round trip (two switches plus two `SYS_YIELD`s); subtracting the trap
(`~2 x null_syscall`) leaves ~28 ns per switch. The subtraction is approximate and the *mechanisms
differ* (our lightweight yield versus a pipe pass), so read the ~15x gap to Linux as directional, not
exact. It points the same way the other two do, and that consistency, three metrics, three methods,
all favoring the minimal kernel, is the real signal.

The story the debug build told first was the *opposite* at IPC, and the gap between them is the whole
lesson. Debug nife: null syscall ~42 ns, ctx switch ~692 ns, IPC ~2272 ns. So `-O0` was a ~1.5x
tax on the bare syscall (which still won) but a **~6.7x tax on IPC** (which lost to Linux at 1723 ns
until this). The heavier a path, the more the optimizer matters, and the IPC path, two context
switches plus four traps plus the rendezvous, is heavy. The null-syscall win survived the debug
handicap; the IPC win was hidden by it. Measuring both builds is why we can say which.

Honest caveats remain. A semantic one for IPC: our endpoint is a synchronous three-word rendezvous, a
Unix pipe is a buffered byte stream through a kernel buffer, so this is our native IPC against Unix's
*standard* IPC (`lat_pipe`), not XNU's fastest (a Mach port would likely beat the pipe). And the host
context switch still wants lmbench's ring method to isolate cleanly. `sel4bench` (the one peer that
would tell us how close to the state of the art these numbers are) is the remaining comparison.
