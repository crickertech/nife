# 134. The register of measures: every number this kernel owes itself

**Status: PARTIAL.** The register exists and holds the tier A and tier B measures below.
**Tier A (E1 through E4) ran 2026-08-22, and E4's own follow-up (a background load near E1's knee)
ran 2026-08-23**; **E1, E3 and E4 were re-taken on radon on 2026-09-04**, which is the small-cache
board all three were designed against and where the dev Mac's large L1 had been muting them.
Tier B remains gated on milestone 74's counters and
milestone 127's silicon (see "What is built, and what is not" below for both). What landed
2026-08-18 is the register itself
(notes/register-of-measures.md), with the test for what belongs in it and the three states a measure
can be in, plus the unsafe census calef folded into this block the same day and the `count-at-most`
ceiling relation that census needed. See "What is built, and what is not" below. Raised 2026-08-18
by calef, in one question: *"So what data would enable us to make these decisions?"* Both open kernel decisions ended in the same place, that the deciding
number does not exist, and §96 said the instrument "wants a roadmap block of its own". This is it.

**Gate: NONE.** Tier A is startable today. That is also a correction to the two decisions that raised
this block: §95 and §96 both say to wait for milestone 74's counters on milestone 127's TX1, and both
over-gated, because the experiments that produce a *verdict* need no silicon. The PMU produces
*mechanism*. See the correction section below.

**The board session's three results, because two of them are stronger than the dev Mac's and one is
a defect in the experiment.** Six interleaved boots on a `single_hart` card; the reading is
notes/footprint-perturbation.md and the capture is `bench/radon-2026-09-04/`. **E1 found the knee**,
at 16 threads, 68% up from 2 and then flat to 96, where patagonia showed 8 to 11% with no bend.
**E4 found displacement**, 5 to 8% under 96-thread IPC load against 0 to 1% at ordinary load,
peaking at a 32 KiB working set which is radon's L1d exactly. **E3 found that it cannot separate
footprint from code layout**: the padded build is 1.49% slower on `call_reply` and 3.01% *faster* on
`ipc_rtt_el0`, and dead code that is never executed has no mechanism for the second. That is a flaw
in E3's design rather than in the session, it was equally present on 2026-08-22, and the fix is
`design/roadmap/proposals/a-layout-control-for-the-perturbation-experiments.md`.


**Extended the same day, at calef's direction**, and the extension changes what this block is. The
first draft listed only the experiments runnable today, on the reasoning that a measure we cannot take
is not yet useful. calef's correction: *"I'm also fine defining measures that we cannot capture until
the hardware. I just want to capture the measures as a milestone."* He is right, and the reason is
this tree's own ladder. **A measure that exists only as an intention is at rung zero.** Written down
with its instrument and its blocker, it is a thing the next person can find, argue with, and take when
the hardware arrives. Defining it costs an hour; rediscovering the need for it costs the milestone
that goes without it.

So this is a **register**, not a work queue. Two tiers, defined to the same standard, and the tier is
a property of the instrument rather than of the measure's importance. Several of the most valuable
numbers here are in tier B.

**What this block is not.** It does not build the counters, which is milestone 74, nor decide who may
read them, which is milestone 75, nor run the cross-OS comparison, which is milestone 25 on milestone
127's silicon. It says **what to measure and why**, so those three have a customer.

## What the two decisions are waiting on

| Decision | The claim it cannot check | Why our instruments miss it |
|---|---|---|
| §95, an IPC fastpath | 5.6 KiB of hot path costs something | icount models no cache; nothing measures I-cache |
| §96, process against event kernel | per-thread kernel stacks cost something | every bench in the tree is two threads |

The two share one root: **every instrument here is a micro-benchmark, and both claims are about cache
behaviour under load.** Warton's event-kernel result is the warning written out, at "generally within
1% on micro-benchmarks" and 20% on a multi-tasking workload.

## Tier A: available today, on the dev machine under Hypervisor.framework

### E1. IPC latency against thread count. Decides §96.

**The mechanism, stated so the experiment can falsify it.** Thread kernel stacks are
`STACK_SLOT_SPAN` apart, which is 28 KiB. Each IPC that switches threads touches a **different**
stack, so the lines are cold; an event kernel touches the same per-core stack every time and they are
hot. The penalty therefore **scales with the number of distinct threads cycling through the kernel**,
and it is invisible at two.

**The prediction, with the arithmetic shown so it can be wrong.** Against the smallest L1d among
machines we run on (32 KB, the SiFive U74), and taking one IPC to touch roughly 1 to 2 KiB of its own
kernel stack, the stacks alone fill L1d somewhere between **16 and 32 threads**. So the curve should
be flat and then bend, with a knee in the low tens.

**The measurement.** `ipc_rtt_el0` with N client-server pairs in the rotation, N from 2 to 128, wall
clock, reported per iteration. Nothing about it needs a cycle counter.

**What each outcome settles.** A flat curve to 128 threads says the process kernel costs us nothing
on this axis and **§96 is answered no, on data**. A knee near the prediction reproduces Warton's
effect on our own kernel and gives its magnitude, which is the number §96 says it lacks.

**It must not run under icount**, which models no caches and would report a flat line by
construction, and that flat line would look like an answer.

### E2. The thread census on the customer path. Decides whether §96 matters at all.

Cheapest of the four and possibly decisive on its own. **How many threads does the actual workload
create?** If the SMB and filesystem path runs a dozen, the rotation never approaches the knee, the
stacks stay hot, and §96 is moot for this system whatever E1's curve looks like. §96's memory input is
already dead (3.00 MiB, 0.069% of a 4 GiB board); this asks whether its performance input is dead too.

Instrumented run of the existing service path, counting live threads at peak. No new benchmark.

### E3. Footprint perturbation. Decides §95's premise, with no PMU.

**Pad the IPC fastpath with resident dead code** until `script/fastpath-footprint` reports roughly
double, and measure the latency change. If footprint is the binding constraint the padding hurts; if
latency does not move, 5.6 KiB is not costing anything on that machine.

This is how to test Liedtke's claim **without a cache counter**, which is what makes it available
today.

**Its power is asymmetric, and that is the limitation to state loudly.** A *positive* result on the
dev Mac is conclusive: if padding hurts on a machine with large caches, it certainly hurts on a
32 KB L1i. A *negative* result on that machine proves little, because an M-series core may simply
absorb the whole path. So E3 answers §95 cheaply in one direction and needs a small-cache board in
the other.

### E4. Application displacement. The Liedtke measurement proper.

Everything above measures the kernel. **Liedtke's actual claim is about the cost the kernel imposes
on the application after the syscall returns**, which no kernel-side benchmark can see by
construction, and which is the reason Mach's IPC looked acceptable in isolation.

Shape: a userspace program with a known, tunable working set, measured with and without concurrent
IPC traffic. The number is the throughput the application loses, which is the AIM7 shape and the one
figure that speaks to the thesis rather than to a subsystem.

This is the most valuable of the four and the least defined. It is listed last for that reason, not
because it matters least.

## Tier B: gated on the counters, the silicon, or both

Each of these names its instrument and what it settles. **None is blocked on design work here**; they
are blocked on milestone 74 (the counters), milestone 75 (who may read the cycle counter, and by what authority) and milestone 127
or a VisionFive 2 (silicon with a real PMU and a small cache).

The recurring reason they need hardware: **QEMU-TCG models no cache, no TLB and no cycle**, and HVF
passes through no PMU, so on the dev machine every one of these reads as either zero or as
nanoseconds times an assumed clock.

### M5. Cycles per IPC round trip

Instrument: `PMCCNTR_EL0`, and the SBI PMU or the `cycle` CSR on RISC-V. **What it settles:** the
±1.5x uncertainty milestone 101 records, which comes from not knowing whether the vCPU thread ran on
a 2.75 GHz E-core or a 4.05 GHz P-core, and which is why no ratio in that file may be quoted tighter
than "same order". Until this exists, **every cycle figure in this project is arithmetic**, and the
seL4 comparison rests on it. Reference point: seL4's published 413 and 426 cycles one-way on the same
class of silicon.

### M6. Instruction-cache misses per IPC

**The Liedtke quantity for §95**, and the thing `script/fastpath-footprint` is a proxy for rather than
a measurement of. Milestone 132 says so in its own BUGS. A footprint of 5.6 KiB against a 32 KB L1i
predicts near-zero steady-state misses; if the measurement disagrees with the prediction, the proxy is
wrong and the gate's threshold should move. Good: flat in steady state. Bad: misses tracking footprint.

### M7. Data-cache misses per IPC, attributed to the kernel stack region

**The direct form of §96.** E1's curve is the symptom; this is the cause, and the two together are
what make the process-kernel question answerable rather than arguable. The specific test is narrow:
do D-misses rise with thread count, and do the missing lines fall inside the stack area? A rise with
no stack attribution refutes the mechanism while leaving the symptom, which would be the most
informative outcome of the whole register.

### M8. TLB misses per IPC, instruction and data

**Settles two claims at once.** Whether the 28 KiB stack stride costs TLB coverage as threads
multiply, which is the other half of M7. And whether the ASID work does what notes/asids.md claims,
that the context-switch flush disappeared on aarch64 while RISC-V keeps flushing when `satp.ASID` is
zero bits wide. That asymmetry is currently an argument from the code, and this is what would turn it
into a number.

### M9. Per-phase cycle attribution across one IPC

Trap entry, decode and dispatch, rendezvous, switch, trap exit. **What it settles:** where fastpath
effort should go, which nothing currently knows. Milestone 132 found `syscall::dispatch` is the
largest single **symbol** at 2,024 bytes, and §95 reasons from that to it being the obvious thing to
skip. **Large is not the same as costly**, and this measure is the only thing that can tell the
difference. It could easily refute the premise of the fastpath as currently sketched.

### M10. Application working-set displacement per IPC

E4 measures this as lost throughput, which is the honest indirect form. This is the direct one: **how
many lines of the application's working set does one IPC evict.** It is Liedtke's actual claim,
stated as a number, and it is the figure that speaks to the thesis rather than to a subsystem.
notes/benchmarks.md already sets a target of under 1 KiB of data touched per IPC, roughly 16 lines,
and **nothing measures it**; this is what would.

### M11. Interrupt latency, worst observed and worst bounded

**The counterweight to §95's fastpath.** Direct process switch "generally ignore[s] priorities",
which is why seL4 made it subject to them and Fiasco.OC made it optional. A kernel that gets faster
at IPC while getting less predictable under interrupt has made a trade nobody priced. Needs real
hardware because interrupt delivery under emulation is not the thing being measured.

### M12. The same measures on seL4, on the same board

Milestone 25's deliverable on milestone 127's TX1, cited rather than duplicated. What belongs **here**
is the constraint that makes it honest: **M5 through M10 must be captured on both kernels, on the same
silicon, in the same build configuration**, or the comparison inherits exactly the class of error
milestone 101 recorded, where three separate mistakes cancelled into a plausible-looking ratio.

## The correction this milestone makes to the decisions that raised it

§95 recommends holding the fastpath for "milestone 74's cycle counters on milestone 127's TX1", and
§96 recommends re-opening "when that number exists", both reading as though silicon were the blocker.
**Neither is true as stated.** E1, E2 and E3 run on the existing dev machine, and E2 needs no new
benchmark at all. The honest gate on both decisions is **these experiments**, not the hardware, and
the hardware improves the answer rather than enabling it.

Recorded here rather than by editing those sections, because a recommendation that was wrong is worth
more standing next to its correction than quietly fixed.

## What is built, and what is not (2026-08-18)

**Built.** notes/register-of-measures.md, which is the block's own subject: the numbers this kernel
holds itself to (nine gated), the numbers it merely knows (four dated, each with the command that
re-takes it), and a deliberate-exclusions table where every rejected number names the half of the
test it failed. The test it settled on is sharper than this block's first draft: **a number belongs
if something depends on its value and it can move without anybody editing it.** The second half is
what cuts, because it separates a measure from a decision.

**Also built, folded in at calef's direction the same day**, from a separate question about unsafe:
the unsafe census, in notes/unsafe-obligations.md, and the `count-at-most` relation in `script/lint`
that it needed. The census's own result is the argument for having taken it: outside
`kernel/src/arch/` the raw count went 171 to 747 since 2026-07-15 while the **density fell from 228
to 93 per 10,000 lines, at every sample**, so the tree has been getting proportionally safer and
nothing was measuring it. A single commit two days earlier, `d5a969a2`, moved that count by 94 in
one change and nothing recorded it.

**Built 2026-08-22: E1, E2, E3 and E4**, as registered instruments rather than one-off scripts
(`script/fastpath-footprint --features <name>`, `cargo xtask bench --extra-features <name>`,
`kernel/src/bench.rs`'s `ipc_thread_scaling`/`app_displacement`). Results, each a real number rather
than a placeholder: **E2** (thread census) found 4 new threads at the SMB/FS customer-path topology.
**E1** (IPC latency vs. thread count) is flat at ~1,270-1,310 ns/iter through 16 threads, rising
8-11% by 64-96 threads, a small but real knee consistent with the predicted low-tens location, muted
by the dev machine's larger L1d against the 32 KB SiFive U74 target. **E3** (fastpath padding)
roughly doubles instruction footprint on both ISAs but shows only a 2-3% latency effect on this
machine, within run-to-run noise, no measurable cost here. **E4** (application displacement) is
0-3% across 4-128 KiB working sets under concurrent IPC load, consistent with E1's own flat region.
Full detail in `notes/register-of-measures.md`, updated the same day (Owed count 12 to 8, three
honest `BUGS` entries: the sweeps were run at shipped values only, and both were measured on a
shared, noisy machine rather than the quiet single-tenant conditions earlier steps had).

**One correction this lane makes to the block above.** The block treats "register" and "experiments"
as one deliverable. They are not the same size, and separating them is what let the register land in
a day: the register is a document plus a gate, and E1 through E4 are four benchmark harnesses. A
future lane should take E2 alone, then E1, rather than reading this block as one piece of work.

**Built 2026-08-23: E4's own follow-up, closed.** E4's original 8-pair background load sits inside
E1's flat region, so the 0-3% it found was expected from E1's own curve rather than independent
evidence against displacement (`notes/register-of-measures.md`'s own BUGS said so). `app_displacement`
now also runs a second condition at `SCALE_MAX_PAIRS` (48 pairs, 96 threads), the same pair count E1's
own sweep tops out at. Over three repeated runs, on the same dev Mac under HVF: the low-load condition
still reads 0-5% and the high-load condition reads 2-9%, consistently higher than the low-load figure
at every working set on every run, though the two ranges overlap and neither is a clean step function.
It is a small, reproducible effect in the predicted direction rather than a decisive one, consistent
with E1's own finding that this machine's larger L1d mutes the knee it would show on the 32 KB SiFive
U74 target, and it wants the same small-cache board re-run E1 and E3 already want before either range
is treated as more than a first data point. Detail and raw numbers in `notes/register-of-measures.md`.

## Scope note

Instrumentation and measurement. No syscall surface, no wire format, no dependency. It builds
benchmarks and produces numbers; it decides nothing by itself, which is the point.

**Sequencing:** within tier A, E2, then E1, then E3, then E4. E2 first because it is nearly free and
can retire §96 on its own; E4 last because it needs a design rather than a harness. Tier B is ordered
by its blockers rather than by us: M5 the moment milestones 74 and 75 land, M6 through M9 as soon as
there is silicon, M10 and M11 after, and M12 last because it needs the other kernel built and booted.

**A tier B measure is not a promise to take it.** Some may be answered by a tier A result, and a
measure this register carries and nobody ever needs is a cheap thing to have been wrong about.

## BUGS

- **The dev Mac is the wrong machine for a null result.** Its caches are large enough to hide both
  effects, so E1 and E3 are conclusive when they show something and weak when they do not. Both want
  a re-run on a 32 KB L1 board before a negative is believed. notes/portability.md already calls HVF
  a rehearsal rather than a verdict, and that applies exactly here.
- **E1 measures the process kernel's penalty, not the event kernel's benefit.** It shows whether the
  cost exists and how it scales; it cannot show what a continuation-based kernel would actually
  recover, because that requires building one. A knee makes the case worth taking seriously rather
  than proving the alternative wins.
- **The 1 to 2 KiB per-IPC stack figure in E1's prediction is an estimate**, not a measurement.
  Milestone 84's high-water number (11.4 KiB) covers the deepest chain in the whole suite, which is
  spawn and teardown rather than IPC. The per-IPC depth should be measured before the prediction is
  quoted as anything but a sighting shot.
- **None of this measures the verification argument**, and nothing can. §96's claim that explicit
  continuation state suits a model checker better than an implicit stack is a design argument, and it
  will still be a design argument after every number here exists.
- **Tier B is specified from the outside, against instruments nobody here has used yet.** Every
  measure in it names a counter this project has never read: `PMCCNTR_EL0`, the SBI PMU, and whatever
  cache and TLB events the board's PMU actually exposes. **Real PMUs do not implement every
  architected event**, and some implement them wrongly, so a measure may turn out to be unavailable or
  untrustworthy on the specific silicon. Expect M6 through M8 to need adjusting to what the TX1 and
  the U74 really count, and treat the event names as intent rather than as a plan that will survive
  contact.
- **M9 could refute the fastpath as sketched**, and that is a feature of listing it rather than a risk
  of taking it. §95 reasons from `syscall::dispatch` being the largest symbol to it being the thing
  worth skipping. If per-phase attribution says dispatch is cheap and the trap entry dominates, the
  fastpath work should change shape. Better to find that with a counter than after building it.
## Follow-on

- **Milestone 229.** Tier B's authority blocker is gone.
  `design/decisions/139-cycle-counter-authority.md` is DECIDED (calef, 2026-09-02, a per-thread
  grant in the spawn manifest) and `design/roadmap/229-the-counter-grant.md` is BUILT the same day;
  `kernel/src/arch/aarch64/timer.rs` opens and closes the counter per thread at the switch.
- **Outstanding.** Milestone 74's counter driver is still NOT-STARTED, so M5 through M9 have no
  instrument. The grant opens the register and nothing yet reads it as a benchmark. Checked
  2026-09-03.
- **Done.** The silicon this register waits on arrived: `notes/target-hardware.md` lists argon as
  in hand and radon as booting nife and wired as a bench target, which is the small-cache board
  this block names as milestone 127's alternative.
- **Outstanding.** The small-cache re-run of E1, E3's latency half and E4 is now
  *runnable* and has still not been *taken*. E1 and E4 were `#[cfg(target_arch = "aarch64")]` and
  are now built for riscv64 as well; the `single_hart` kernel feature parks radon's other three
  U74s, because both experiments need one hart and a card has no `-smp 1`; and
  `script/board-image --bench [--extra-features fastpath_pad]` writes the pair of cards E3
  compares. The procedure, with a table mapping each observable outcome to what it settles and
  which milestone it routes to, is **notes/footprint-perturbation.md**. **No number came off radon**: the
  board was powered off and there was no bench session, so the register's E1/E3/E4 rows still say
  dev Mac only, correctly. What is outstanding is the session.
- **Recorded.** A correction found on the way, 2026-09-04, which would otherwise have wasted the
  session. E3's padding
  was reachable only from `sched::ipc_send`. Milestone 188 phase 1 (2026-09-04) split the footprint
  gate into `ipc_send_recv` and `ipc_call_reply` and established the second as the shape services
  actually run; measured on riscv64 the day after, `--features fastpath_pad` moved `ipc_send_recv`
  to **2.10x** and `ipc_call_reply` to **1.00x**. E3 was padding a path essentially nothing in this
  tree uses. `sched::ipc_call` now calls `maybe_pad` as well, and both shapes pad to roughly 1.85x
  on both ISAs. The experiment was correct when it was built; the thing it measures moved
  underneath it, which is what a dated instrument does.
- **Outstanding.** The per-IPC kernel stack depth is still an estimate.
  `notes/stack-high-water.md` reports a deepest standing path across the whole suite rather than a
  per-IPC figure, so E1's prediction remains a sighting shot. Checked 2026-09-03.
- **Milestone 25.** M12, the same measures on seL4 on the same board, is that block, itself PARTIAL
  and gated on hardware and milestone 74, which this one already cites.
- **Recorded.** E1 measures the process kernel's penalty rather than the event kernel's benefit.
  Showing the cost and its scaling is all a benchmark can do; what a continuation-based kernel
  would recover needs one built.
- **Recorded.** Nothing here measures the verification argument. §96's claim that explicit
  continuation state suits a model checker better than an implicit stack stays a design argument
  after every number exists.
- **Recorded.** Tier B's event names are intent rather than a plan that survives contact. Real PMUs
  do not implement every architected event and some implement them wrongly, so M6 through M8 should
  be expected to change shape against what the TX1 and the U74 actually count.
- **Recorded.** M9 could refute the fastpath as sketched, and listing it is the point. If per-phase
  attribution says dispatch is cheap and trap entry dominates, §95's premise moves.
