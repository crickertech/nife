# 168. A multi-tasking workload benchmark: the number that would decide the event-kernel question

**Status: PARTIAL.** The instrument is built, gated and rehearsed on all three architectures.
**Five boots of radon on 2026-09-16 produced numbers, and this block still does not turn `BUILT`**,
because one of the six sweep points did not produce a number so much as a distribution. The measured
curve and the reason are in "What five boots measured" below; the honest summary is that the
**shape** is solid and `tasks=4` is not yet a figure anyone should quote. Started and left partial
2026-09-04 by a lane with no board, which is what the gate below predicts rather than a shortfall
against it. Minted 2026-08-25, from [DECISIONS §96](../decisions/96-process-kernel-or-event-kernel.md)'s own recommendation: *"Build the instrument that could decide it. The blocker is that a multi-tasking workload is the only place the difference appears, and we have none."*

**Gate: HARDWARE.** Real silicon, and that is all. **Corrected 2026-09-04 by calef**; it read
`HARDWARE, MILESTONE 127` from 2026-08-25 until then, and the paragraph below explains why that was
right when written and is not now.

**The old gate was doing two jobs and only one of them still needs argon.** Its stated reasons are
that a multi-tasking difference *"needs real hardware scheduling behavior, not TCG or HVF timing"*
and that it wants *"a place to run it that has real PMU access"*. **radon satisfies both.** It is
real silicon, it boots, and milestone 74's riscv64 half landed on 2026-09-04, so
`kernel/src/arch/riscv64/pmu.rs` reads cycles through the SBI PMU extension. When this block was
written radon had no cycle counter at all, which is why milestone 127 was named.

**What milestone 127 is actually for is a different question**, and its own title says so: *the seL4
machine: a Jetson TX1, so identical silicon referees the comparison.* That matters for **milestone
25**'s cross-OS numbers, which are compared against seL4's published runs and therefore want the
machine those runs were made on. It does not matter for DECISIONS §96's question, which asks how much
of *this kernel's* time goes into process-kernel overhead under multi-tasking load. That is a number
nife produces about itself, on any real silicon.

**So the two uses split, and this block serves both without needing the same machine for each:**

- **For §96** (process kernel or event kernel), radon is sufficient and available now.
- **For milestone 25**'s comparison against seL4, argon is still required, and nothing here loosens
  that.

**And building the instrument is gated by neither.** The workload is architecture-neutral and can be
developed and tested under QEMU; what needs silicon is the *number*, not the code. A lane may build
this today and leave the measurement to a bench evening, which is how this milestone was launched.

## What this is for

DECISIONS §96 asks whether nife should stay a process kernel (what it is today: every thread gets its own kernel stack) or move to an event kernel (one stack per core, explicit continuations), the model seL4, OKL4 and NOVA all eventually adopted. §96 found three of the four inputs to that question already settled by measurement (memory savings are negligible at this project's scale; stack-shrinking is closed off, no slack remains; the verification argument doesn't transfer, since Kani never reaches `kernel/src` here). The fourth input, performance, is the one live, unmeasurable argument: the paper §96 cites (Warton, on Pistachio) found event kernels roughly tied with process kernels on micro-benchmarks but **20% better on a real multi-tasking workload (AIM7)**. Every instrument this project currently owns (`ipc_rtt`, `ipc_rtt_el0`, the icount tripwire, milestone 132's footprint gate) is a micro-benchmark, and would show approximately nothing for this question.

**This milestone is that missing instrument, and nothing else.** It does not decide §96; it produces the number §96 needs to decide itself.

## What was built, 2026-09-04

Four pieces, and the shape is `kernel/src/soak.rs`'s rather than `kernel/src/bench.rs`'s, for a
reason the gate makes concrete: **this number has to be taken on a board, and the bench boot has
never run on one.** What a board takes is `script/board-image` plus `script/board-console`, which is
a kernel feature printing to the serial console.

- **`crates/job_mix`**, the workload's definition, host-tested. What AIM7 actually is, read on
  2026-09-04 from the benchmark's own README and from the ACM TOCS 2016 retrospective rather than
  guessed at; which four of its methodological properties are kept (heterogeneity, per-task random
  order, a task-count sweep, a throughput metric) and which of its 53 Unix-shaped jobs are
  deliberately not. Five jobs stand in for its categories.
- **`fixtures/src/job_mix_task.rs`**, one EL0 task. Processes rather than kernel threads, which is the one
  place this differs from milestone 134's E1 on purpose: §96 asks what a process kernel costs a
  *workload*, and a workload is processes.
- **`kernel/src/jobmix.rs`** (`--features jobmix`), the supervisor. It builds the pool once, releases
  a subrun's worth of tasks, owns the wall clock, and **prints the placement census before the first
  number**, because milestone 240 found placement decides throughput on radon by up to fifteenfold
  and a figure without its arrangement is a draw rather than a result.
- **`script/job-mix`** (QEMU rehearsal) and **`script/board-image --jobmix`** (the card).
  `notes/job-mix.md` is the bench procedure, with a table mapping every observable outcome to what it
  means and where it routes.

Rehearsed on patagonia on 2026-09-04: aarch64, riscv64 and x86_64, sweep complete on each, six
points per run. **Those magnitudes are not a result and are not recorded as one**; TCG models no
cache.

**On the citation, because this tree has carried a fabricated block quote before.** The 20% figure is
the *retrospective's* summary of Warton: Elphinstone and Heiser, ACM TOCS 34(1), April 2016, §4.1,
*"generally within 1% on micro-benchmarks but a 20% performance advantage of the event kernel on a
multitasking workload (AIM7)"*. Warton's own abstract claims something weaker (memory savings
"without degrading the kernels performance", with a performance improvement called preliminary and
needing more experiments). Both are real; they are not the same strength of claim.
`notes/job-mix.md` carries the provenance and the URLs.

## What it needs

- **A real multi-tasking workload**, not a single-operation timing. The cited paper's own instrument is AIM7 (see `notes/l4-lessons.md`'s citation for the exact numbers this project has already quoted from it: *"20% performance advantage on a multi-tasking workload (AIM7)"*); whoever builds this should read the paper's own methodology rather than guess at AIM7's shape from the name, and decide whether to port something AIM7-equivalent or design a workload that exercises the same property (many threads, contended scheduling, real context-switch pressure) in a way that fits this project's own capability model.
- **Real hardware**, per the gate above: this specific difference does not show up under QEMU TCG or Apple's HVF, the same reason `sel4bench` (milestone 25) is deferred to real silicon.
- **A place to run it that has real PMU access**, matching what milestone 25's own `sel4bench` piece already needs from milestone 127's machine, so the two pieces of hardware-gated work should likely be sequenced together rather than treated as unrelated.

## Why it matters, beyond §96

**Milestone 25's cross-OS comparison has the same hole**, and this milestone closes it too rather than duplicating it. Checked directly: milestone 25 is explicitly a set of EL0-measured *primitive* benchmarks (single syscall, single context switch, single IPC round trip, single map, single spawn) compared against lmbench and `sel4bench`; every one of them a micro-benchmark in the same sense §96 means the word. Milestone 25's own remaining piece (`sel4bench`) is also single-operation PMU timing, not a multi-tasking workload. So neither milestone currently has an instrument that could show what a real multi-tasking difference looks like, and building one here serves both.

## What this does not decide

Whether nife should actually switch kernel models. That is DECISIONS §96's own question, and it stays open until this milestone's number exists (or until a real customer-path workload starts creating threads in the hundreds, the other condition §96 names for reopening early).

## What five boots measured, radon, 2026-09-16

Transcripts `bench/radon-2026-09-16/jobmix-boot1.log` through `boot5.log`, all five complete
(six sweep rows and `job-mix: done` each), booted over TFTP from an identical image.

Each `job-mix:` line is the **best of three repeats**, per `job_mix::REPEATS` and the reason stated
there: the minimum is the least-contended sample.

| tasks | boot 1 | boot 2 | boot 3 | boot 4 | boot 5 | spread |
|---|---|---|---|---|---|---|
| 1 | 319,052 | 319,013 | 319,039 | 319,013 | 319,072 | **0.0%** |
| 2 | 626,242 | 622,410 | 626,452 | 628,362 | 629,044 | 1.1% |
| 4 | 849,610 | 793,137 | 929,866 | 766,361 | 991,671 | **29.4%** |
| 8 | 935,893 | 901,011 | 923,732 | 911,129 | 997,548 | 10.7% |
| 16 | 987,572 | 1,003,653 | 940,648 | 946,568 | 987,017 | 6.7% |
| 32 | 1,032,586 | 1,060,264 | 1,054,227 | 1,057,537 | 1,050,944 | 2.7% |

**The shape is the result, and it held on every boot.** 1 to 2 tasks is 1.96x on a machine with four
usable harts, so nearly free; the knee is between 2 and 4; past 8 it is flat; and at 32 tasks, which
is 8x oversubscription, throughput still creeps up rather than falling. A kernel losing significant
time to process-kernel overhead under load would bend earlier **and keep declining**. This plateaus.

**`tasks=4` is not a number, and five boots are what established that rather than five boots being
what it needed.** The variance is *within* a boot, not between boots. Pooling all fifteen repeats at
each sweep point:

| tasks | min ticks | max ticks | worst within-boot spread |
|---|---|---|---|
| 1 | 96,279 | 96,402 | 0.1% |
| 2 | 97,672 | 112,124 | 14.2% |
| 4 | **123,912** | **181,408** | **37.3%** |
| 8 | 246,364 | 299,644 | 18.5% |
| 16 | 489,731 | 544,301 | 6.1% |
| 32 | 927,165 | 1,016,672 | 6.8% |

Boot 3's three repeats at `tasks=4` span 132,148 to 181,408 on their own, which **contains the
entire boot-to-boot range of the best-of-three values**. So a best-of-three drawn from a
distribution this wide is itself a coin flip, which is why boot 4 read 766,361 and boot 5 read
991,671 from the same image. **More power cycles sample the wrong axis.**

**Why `tasks=4` and not elsewhere, from the code rather than from the shape of the output.**
`job_mix::ECHO_SERVERS` is 2, deliberately fewer than the task count so the endpoint is genuinely
contended. At 1 and 2 tasks there is no contention (1:1 or better); at 4 there is 2:1 contention for
the first time, with too few samples to average it; by 8 and above the contention is deeper but the
averaging is better. **The knee and the instability are the same phenomenon**, which is worth
stating because reading them as two facts would suggest the knee is a scheduling cost when it is a
contention point the instrument was built to create.

**A correction, recorded because it was nearly written into this block as a finding.** The
`job-mix-census:` lines were read across three boots as evidence that thread placement is
deterministic (6/9/9/10 every time) and therefore that `notes/soak.md`'s fifteenfold placement
hazard had not materialised. That inference was wrong: the census prints **once, before the sweep**,
and describes the 32-thread pool. It says nothing about where four tasks land during the `tasks=4`
subrun, which is exactly the configuration whose variance was being explained.

**What is solid.** `tasks=1` at 0.0% across five cold boots is a determinism check on the instrument
itself, and it passes. The curve shape is stable. The `tasks=32` figure, about 1.05M jobs per minute,
is the most repeatable point on the sweep at 2.7%.

## Follow-on

- **Outstanding.** **The number itself, on radon.** Checked against the tree on 2026-09-04:
  `notes/register-of-measures.md`'s milestone 168 row reads "no run on silicon", and
  `notes/job-mix.md`'s Results table is empty and says so in words. The instrument, the card build
  and the procedure all exist; what is missing is a bench evening and five to ten power cycles.
  This block stays `PARTIAL` until that table has rows.
- **Outstanding.** **What the sweep cannot see above 32 tasks.** `job_mix::MAX_TASKS` is 32 because
  every task is a process with an address space, where milestone 134's E1 reached 96 kernel threads.
  Checked by reading `crates/job_mix` and `kernel/src/jobmix.rs`: nothing has measured where a board
  actually runs out, and the supervisor's `FAILED: could not spawn task N` line is the measurement
  that would say. It is part of the first bench evening rather than separate work.
- **Proposed.** `crates/board_console` has no recogniser for this run, so an operator tells a
  finished sweep from a wedged one by reading the log.
  `design/roadmap/proposals/a-board-console-recogniser-for-the-job-mix-sweep.md`, with the reason it
  was deliberately not written blind: the outcome table in `notes/job-mix.md` becomes evidence after
  the first bench evening and a guess before it.
- **Proposed.** Three of AIM7's categories have no representative in the mix (file operations,
  process creation, page mapping), and two of them are exactly the jobs that block deep in a kernel
  path, which is where §96's cost lives.
  `design/roadmap/proposals/three-aim7-job-categories-the-job-mix-does-not-have.md`.
- **Recorded.** The mix proportions are chosen rather than derived from an AIM7 workfile, so a
  different mix gives a different number and no result is quotable without saying which mix produced
  it. It stays a limitation: nobody has an AIM7 workfile for a capability microkernel, and inventing
  one would be a stronger claim than the evidence supports. Recorded beside the feature in
  `crates/job_mix`'s own `BUGS` and again in `notes/job-mix.md`.
- **Refused.** A committed baseline and a `--check` for this sweep, the way `script/bench` gates
  against `bench/baseline-*.txt`. Refused because the icount instrument's determinism is what makes
  that gate meaningful, and a workload whose entire subject is scheduling under contention is not
  deterministic on any accelerator this tree has. A gate here would be asserting a tolerance nobody
  has measured, which is how `script/lint` has already lost three checks.

## Index row

Minted from DECISIONS §96's own recommendation: every benchmark this project owns is a
micro-benchmark, and the one number that could decide process-kernel-vs-event-kernel only shows up
on a real multi-tasking workload, on real hardware. The instrument is built, gated and rehearsed
on all three architectures (`crates/job_mix`, `fixtures/src/job_mix_task.rs`, `--features jobmix`, `script/job-mix`); **the number is outstanding and needs radon**. Closes the same hole in
milestone 25's cross-OS comparison too.
