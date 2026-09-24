# The register of measures: every number this kernel owes itself

*(Milestone 134. The name `register-of-measures.md` is **provisional**; naming is calef's, and a
lane ships a provisional one and says so.)*

This tree measures a great deal and remembers almost none of it. A number gets taken once, written
into a note beside the reasoning that needed it, and then sits there being true on the day it was
written. `notes/counted-claims.md` found three such numbers on 2026-08-14 and **all three were
wrong**; every one had been right when somebody typed it.

That convention fixed the class of number a `grep` can re-derive. This register is the other half:
the numbers that need an instrument, a boot, or a walk over the source. It says which ones this
kernel is holding itself to, which ones it merely knows, and which ones it has defined and cannot
yet take.

**And `notes/project-metrics.md` is the half that moves.** This register says which numbers are owed
and what each one is held to; that page plots the ones that change every week, one row per ISO week,
recomputed from git history by `script/metrics` so nothing here has to be remembered. Several
measures below appear there as a series: the `unsafe` density and its ceiling, the Kani harness
count, the markdown corpus. A reader who finds either file should find the other.

**Since 2026-09-24 that page is a deck and this file holds its prose.** Each chart there carries one
line; the definitions, the arguments, the capture deadlines and the reconciliations are in *The
weekly series* below.

## What belongs here, and the test

**A number belongs if something depends on its value and it can move without anybody editing it.**

Both halves are load-bearing and the second is the one that cuts.

- *Something depends on its value.* Not "somebody would find it interesting". A decision rests on
  it, a constant is sized against it, a claim in the documentation quotes it, or a customer notices
  when it moves. `documentation::render::LINE_MAX` is 2048 because the longest markdown line was 1841, so
  that measurement has a **consumer**; the kernel's image size, which
  `notes/benchmarks.md` itself calls "the number that does not matter", has only a reader.
- *It moves on its own.* A constant somebody chose is not a measure, it is a decision, and it
  belongs in `design/decisions/`. The stack guard page is 4,096 bytes because a page is 4,096 bytes. The
  deepest chain that can reach that guard is a measure, because the compiler moves it every week
  and nobody is asked.

**A register that lists every number in the tree is worthless**, so the exclusions are as much of
this document as the rows are, and each one names the test it failed. They are in their own section
below rather than implied by absence.

## The three states, and the middle one is the finding

Every row is in exactly one of these. The state is a property of the **instrument**, not of the
measure's importance.

| state | means | what happens when the number moves |
|---|---|---|
| **gated** | an instrument re-takes it, and something fails on a bad move | a red build, at the commit that moved it |
| **dated** | a named command re-takes it; nothing fails | the recorded value goes stale, silently |
| **owed** | defined, with its instrument named; no instrument exists yet | nothing, because nothing is measured |

**The `dated` rows are the answer to the question this milestone was raised to ask.** They are the
numbers something depends on where a regression arrives as somebody's data being slow rather than
as a red check. Promoting one to `gated` is the work; recording that it is `dated` is what makes the
work visible.

`dated` is not a defect by itself. `notes/counted-claims.md` puts it plainly: *"A wall clock is not
a count... dating a measurement is the honest alternative to gating it, and the two should not be
confused."* A number that costs a forty-minute boot to re-take does not belong in a gate that runs
on every push. The defect is a `dated` row with no date, or with no command.

## Gated

Nine instruments, and it is worth seeing them in one table because **six of them are the same
shape**: a ceiling that fires when a number grows and stays silent when it falls. Four of those six
were here before milestone 134, unnamed and unconnected, which is why `count-at-most` is a name for
a pattern rather than a new idea. Row 1 is the odd one out, a two-sided drift band, and row 9 is a
floor.

| measure | instrument | what fails |
|---|---|---|
| icount ticks, 14 benchmarks, both ISAs | `script/bench --check` | drift over 10% from `bench/baseline-*.txt` |
| IPC fastpath instruction footprint | `script/fastpath-footprint --check` | growth over 5% from `bench/fastpath-*.txt` |
| the largest kernel stack frame | `script/stack-frame-check` | any frame over the 4,096-byte guard page |
| the deepest reachable kernel-thread chain | `script/stack-depth-check` | a chain over the 24,576-byte stack |
| kernel stack high-water, at runtime | `script/test`, `report_high_water` | boot 61,440, secondary 16,384, thread 18,432 |
| eleven counted claims (harnesses, syscalls, rights bits, ...) | `script/lint` | a marked number disagreeing with the tree |
| unsafe density outside `kernel/src/arch/` | `script/lint` | over 94 blocks per 10,000 lines of code |
| `unsafe impl Send`/`Sync` claims | `script/lint` | over 17, which is today's tree exactly |
| per-file line coverage | `script/coverage` | any file under the 80% floor |

The ceilings are rows 2 through 5 and 7 and 8, and reading their thresholds together is the
useful part, because they are six different kinds of number. 5% is a tolerance. 4,096 is a hardware
fact. 24,576 is a configuration constant. The high-water limits are margins over an observed
maximum. 17 is today's tree exactly. And 94 per 10,000 (lowered from 100, then 97, then 96, then
95, then 94, by milestone 139; round 5 reduced the block count further but left the truncated
density and therefore the ceiling unchanged, see notes/unsafe-obligations.md) is a **claim about
the tree that was false until shortly before it was written**, which makes it the
only one that expresses a direction rather than a limit. That distinction is what `count-at-most`
exists for; see notes/unsafe-obligations.md for the measurement behind it.

Two of these are the register doing its job on itself: the unsafe rows did not exist when milestone
134 opened, and the `unsafe fn` count that would have been a third turned out to be **already
derived** by `script/lint`'s `==> unsafe fn contracts` check. Finding a number already tracked is as
much a result as finding one that is not.

**Row 7's density mixes kernel and userspace unsafe into one population, and nothing here gates the
split.** `script/metrics` (the unsafe-census-by-trust-boundary milestone, provisional) now tracks
`unsafe_trust_kernel`/`unsafe_trust_userspace` alongside it, weekly, with no ceiling of their own:
see notes/project-metrics.md's "The same unsafe blocks, by trust boundary" for the numbers, why row
7's 94 answers a different question than either half, and the recommendation on which one a future
ceiling belongs on.

## Dated

The command is the point of each row. A dated measurement whose re-taking is folklore is a `dated`
row pretending to be one.

| measure | last taken | the command that re-takes it |
|---|---|---|
| IPC round trip in nanoseconds, both planes | 2026-08-04 | `script/bench --real` |
| filesystem throughput, milestone 38's four phases | 2026-08-18 | `script/bench --real --smp`, with a RedoxFS disk attached |
| primitives against Linux and macOS on the same host | **no date recorded** | `bench/host/run_linux.sh`, then `script/bench --real` |
| `unsafe {}` blocks inside `kernel/src/arch/` | every run | `script/lint`, which prints it and asserts nothing |
| E1: IPC round trip against thread count | **2026-09-04 (radon, 6 boots)**; 2026-08-22 (dev Mac) | `cargo xtask bench --real` (`ipc_scale_*` rows); on radon, `script/board-image --bench` and notes/footprint-perturbation.md |
| E2: thread census on the customer path | 2026-08-22 | `cargo xtask test`, the "E2 thread census" line in `a_host_process_connects_to_the_guest_and_is_answered` (both ISAs) |
| E3: IPC fastpath footprint doubled, and the latency it costs | **2026-09-04 (radon, 6 boots); confounded, see below**; 2026-08-22 (dev Mac) | `script/fastpath-footprint --features fastpath_pad [--layout]` (both ISAs); `cargo xtask bench --real --extra-features fastpath_pad` against `cargo xtask bench --real`; on radon, eight images over `NIFE_FASTPATH_PAD`/`NIFE_FASTPATH_SHIFT` (the layout control, 2026-09-19) built by `script/board-image --bench --extra-features fastpath_pad`, procedure in notes/footprint-perturbation.md |
| E4: application working-set displacement under IPC traffic, at typical (8-pair) and high (48-pair, E1's-knee) background load | **2026-09-04 (radon, 6 boots)**; 2026-08-23 (dev Mac) | `cargo xtask bench --real` (`appdisp_*_ipc`/`appdisp_*_ipc96` rows); on radon, `script/board-image --bench` and notes/footprint-perturbation.md |
| per-IPC kernel stack depth, per shape and role (milestone 134) | **2026-09-19 (QEMU, all three ISAs, debug and release)** | debug: `script/test`, the `ipc-stack-depth:` lines of `one_ipc_reaches_a_measured_depth_into_its_kernel_stack`; release: a `bench,ipc_stack_depth` kernel booted on one hart (notes/stack-high-water.md, "Per-IPC depth") |
| multi-tasking throughput, jobs per minute against task count (milestone 168) | **2026-09-16 (radon, 5 boots; old instrument, `tasks=4` not a number)** | `script/board-image --job-mix --tftp`, then `script/board-console`, by `notes/job-mix.md`'s bench-evening procedure; the rehearsal is `script/job-mix` |

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

**The milestone 168 row was the register's first `dated` row that had never been taken**, until
five boots of radon on 2026-09-16. Those boots measured the curve's shape (a knee near four tasks,
then a plateau with no decline through 32) with an instrument that kept the best of three repeats,
and showed that `tasks=4` under that rule was not a number: 29.4% across boots of one image. On
2026-09-19 the instrument changed to the median of 21 repeats and gained a page-mapping job and a
process-creation job, so **the 2026-09-16 date is a date for a different instrument**, and the row
says so rather than letting the date imply the current one has been taken. The next radon evening
re-dates it, and its result is what `design/fatal-risks.md`'s risk 4 and
`design/decisions/96-process-kernel-or-event-kernel.md` are waiting for.

**The filesystem row is the one on the customer path**, and it is the clearest case in the register
for why `dated` is a finding rather than a filing. Milestone 55 is a Time Machine target the
family's Macs back up to. A three-times regression in sequential write would show up as a backup
that used to finish overnight and now does not, reported by a person rather than by CI, and nothing
in this tree would have said a word. It is `dated` because taking it needs a boot with a disk
attached, which is not a thing to put on every push; the honest promotion is a scheduled run rather
than a gate, and it wants a lane.

**The cross-OS row is the register earning its keep on its first pass.** Its section in
notes/benchmarks.md, "The first cross-OS numbers (nife vs Linux vs macOS)", **carries no date at
all**, and the numbers in it are the ones a stranger is most likely to quote back at us: they are
the comparison against Linux and macOS. A dated measurement with no date is a `gated` row's opposite
and a `dated` row's failure mode at once, and nothing in this tree would have said so. Dating it
means re-taking it, because nobody now knows which run it was; that is a small lane and it is named
in this milestone's handoff.

**The arch row is the odd one and it is deliberate.** There is no ceiling on unsafe inside
`kernel/src/arch/`, because driving that number down means either writing assembly wrong or moving
it out of `arch/`, and rule 1 says arch code belongs there. A target would be a gate pushing against
the architecture. But an unmarked number in a note is exactly the snapshot this whole register is
against, so `script/lint` prints it on every run: on screen every build, asserted never. **A number
with a consumer gets a relation; a number with only a reader gets printed.**

## Owed

Eight measures (M5 through M12, "Tier B") are defined here, and as of 2026-09-19 the blocker is no
longer the same one for all of them. E1 through E4 ("Tier A") no longer belong in this section: all
four ran 2026-08-22 and are `dated` rows above.

| measure | its instrument, checked against the tree 2026-09-19 | what is still missing |
|---|---|---|
| M5, cycles per IPC round trip | **exists on all three ISAs**: every tick row times `bench::cycles_per_tick` (milestone 74's two halves, milestone 309 for x86_64) | on riscv64, nothing: radon read `250.00` on 2026-09-16, so `call_reply` is about 1,256 cycles. On aarch64, argon's session and calef's `PMCCFILTR_EL0` ruling (the-aarch64-half-of-74, decision A) before any figure is published |
| M6, I-cache misses per IPC | none | an event-counter driver: nothing programs `PMEVTYPER<n>_EL0` or an SBI PMU cache event on any ISA (aarch64's boot line now reports six event counters visible, and none is used) |
| M7, D-cache misses per IPC in the stack region | half: the per-IPC stack depth (row above) bounds the bytes, not the misses | the same event-counter driver, and for the attribution half a data-address sampler that neither the A57 nor the U74 has; expect M7 to become "misses rise with thread count" plus the depth, rather than attribution |
| M8, TLB misses per IPC | none | the same event-counter driver |
| M9, per-phase cycles across one IPC | the counter: `arch::pmu::cycles()` is readable in-kernel on all three ISAs (riscv64 configures the boot hart only) | phase stamps at trap entry, dispatch, rendezvous, switch and exit, in a build that measures nothing else; not built |
| M10 to M12 | as milestone 134's block says | unchanged |

**Which of these calef's `PMCCFILTR_EL0` ruling touches:** M5 and M9 on aarch64, and M12 (seL4's
413 and 426 are TX1 cycle counts, so the comparison is exactly the number the filter decides). M6
to M8 count events rather than cycles, and `PMCCFILTR_EL0` filters only the cycle counter; each event
counter has its own filter in `PMEVTYPER<n>_EL0`, which will raise the same question when a driver
first writes one.

They are **not duplicated into this table**, because they already have a home that carries each
one's instrument, its prediction, and what its outcome settles:
design/roadmap/134-the-measurements-that-decide.md. Two open kernel decisions were waiting on the
Tier A half of them, and the block's own correction is worth knowing before anyone reaches for
hardware: §95 and §96 both recommend waiting for the TX1, and **both over-gated**, because the
experiments that produce a verdict need no silicon. Tier A's results are summarized above; Tier B
remains genuinely gated on the counters and the board.

## Deliberately not in this register

Each of these was considered and each names the half of the test it failed. The list is here so the
next person does not add them back.

| number | why it is out |
|---|---|
| the kernel's image size (290,816 bytes on aarch64) | **no consumer.** notes/benchmarks.md derives it and then says in its own heading that it is "the number that does not matter": `.text` that never runs during an IPC costs nothing in cache |
| `script/verify`'s wall clock (~47 minutes) | **no consumer.** It is a reader's patience, not a constraint anything is sized against, and notes/verification.md dates it honestly |
| lines of Rust, crates, user programs, commits | **no consumer.** AGENTS.md's method figures are rhetoric about scale, and that file says so; a gate on them would be measuring a paragraph |
| `nifefs`'s `NAME_LEN = 32` | **does not move on its own.** It is a decision with a cost per directory block, not a measurement |
| the number of `#[cfg(kani)]` unsafe blocks (14) | **already gated**, by milestone 113's fourteenth clippy configuration, per block rather than in aggregate |
| `unsafe {}` against `// SAFETY:` parity | **measured and refused.** `clippy::undocumented_unsafe_blocks` already enforces it per block as a hard error, and a count comparison disagrees with it in 65 places (38 after the regex is loosened), every one read a document that is right. notes/unsafe-obligations.md carries the reading |
| the CoreMark score | **already gated**, as a row in `bench/baseline-*.txt` |

The parity row is the one worth reading before proposing a new gate. A count check that fails
correct documents is not a weak gate, it is a gate that will be deleted, and `script/lint` has
already lost three checks with that signature.

## The weekly series: the arguments behind each column

**`notes/project-metrics.md` became a deck on 2026-09-24** (calef: *"we've packed a lot of text into
notes/project-metrics.md, but I really envision more of a deck... I think we should start clean and
build"*). A chart there is a heading, an image, and at most a line or two. That leaves the caveats
that will not fit on one line without a home, and this is the home: every argument below was moved
here verbatim from that page rather than deleted, because most of it is the honesty that keeps an
honest chart from becoming a misleading one.

**The division of labour, so the next person keeps it.** A caveat that prevents misreading the chart
stays on the page, compressed to one line under that chart. Definitions, the arguments behind a
measure, the capture deadlines, the reconciliations and the dated analyses come here. The page links
here once, not once per section.

**In the passages below, "this page" means the weekly series and its charts on
`notes/project-metrics.md`**, because that is where they were written. They were moved verbatim
rather than reworded, so that anyone comparing the two files against git history reads the same
sentences rather than having to trust a paraphrase.

### Read this before you read a number, in full

**Every row is a restatement, not a report.** The series is regenerated by applying *today's*
definitions to old commits. That is the right choice for a trend, because a series measured seven
different ways is not a series. It is also **not what was reported at the time**, and the gap is
not hypothetical here. On 2026-09-02 alone, three of this project's own instruments turned out to
have been wrong:

- Milestone 214 (a test that prints "skipping" and returns is counted as passed) found 80 sites
  doing exactly that. Twenty-five tests left the pass column on `x86_64` once they told the truth.
- Milestone 212 (`script/falsifications` walks `crates/` only, so the ratio it prints is not the
  tree's) found the falsification denominator excluding the kernel and `user/`.
- DECISIONS §139 (who may read the cycle counter, and by what authority) corrected a figure that
  had conflated a counter's tick resolution with the cost of reading it.

So a bar here is best read as *what the tree would say today about how it looked then*, and the
distance between that and what anybody believed at the time is real.

**And every figure in this project is derived by an agent, including these.** There has been no
external audit of any of it. This is a project measuring itself with instruments it wrote, which is
an argument *for* watching the numbers move rather than against it, but the page should not be read
as an independent account. Where a number here has been checked two different ways, this page says
so explicitly and names the method; everywhere else, it has been recorded twice by the same method,
which is consistency and not validation.

**A missing bar means the record did not exist, not that the number was zero.** The roadmap, the
decision statuses, the falsification records, `design/fatal-risks.md`, the naming provenance blocks
and `design/roadmap/proposals/` each arrived on a date, and before it there is nothing to restate.
The two newest series are the sharpest instances: names have three empty weeks because the
convention that records a ratification was invented on 2026-08-04, and proposals have seven because
the directory was created on 2026-09-04. Neither is backfilled, and each section says so where its
bars are.

**Weeks are ISO weeks in UTC**, which is this tree's date convention. The first commits are stamped
2026-07-12 in the architect's local time and fall on the Monday in UTC, so the series starts at
2026W29 and there is no 2026W28.

**The charts show the ten most recent weeks; the CSV keeps every one** (calef, 2026-09-19). So
2026W29 leaves the charts when 2026W39 arrives, and stays in its measure's CSV, the table view
the charts rely on for the three colours that sit under 3:1 on a white page. A week is never
deleted, only no longer drawn.

**A week is spelled `2026W36`, everywhere.** The chart axis, the CSV's `week` column,
`script/metrics`' own output and the prose on this page all use it, and calef ratified that on
2026-09-02 after the page carried three spellings at once (`2026-W30` in the narrative, `202636` on
an axis, `2026-W29` in the data), which is two more than a reader should have to hold.

It is ISO 8601's designation with the hyphen dropped. It still sorts lexically, and it still cannot
be misread as an ordinary number the way a bare `202636` can. It is also the dictionary key that
makes rerunning `script/metrics` replace a row rather than add one, and the axis label is that key
rather than something derived beside it, so the two cannot drift apart.

### The nine things that would kill nife

From `design/fatal-risks.md`, by **Experiment status**: the field calef ratified on 2026-09-23, with
three values and no fourth. `RUN` means the experiment has been performed, `NOT-RUN` that it has not
and could be, `CANNOT-RUN` that it cannot be performed at all. `script/fatal-risks` fails on any
other word, which is why this can be charted as an enumeration rather than read out of a sentence.

**It says whether an experiment happened. It never says what it found.** That is the half of the
2026-09-23 proposal calef did not take, and leaving it out is deliberate rather than pending:
`GREEN`, `AMBER`, `MEASURED` and `AUDITED` are in the file, in prose, beside the argument that earns
them, and a colour band on a chart would be a worse version of a paragraph. `CANNOT-RUN` is the one
value that carries a judgement anyway, and it is the file's own: risk 8 cannot be observed until
milestone 198 (a package manager, and the trivial install that makes a second customer possible)
lands, and **a fatal risk that cannot be tested is the most dangerous state a fatal risk can be in**.

**This replaced a tested/untested pair on 2026-09-23, and the pair's refusal of a verdict column was
right when it was written.** It said a colour series would be "a script reading a sentence and
guessing", which it would have been: four statuses, three of them ending in GREEN or AMBER and risk
7's in neither. What changed is not the reading but the thing read. The pair had to go for a simpler
reason as well: all nine entries carried a status line by 2026-09-23, so it sat at nine and zero and
told a reader nothing.

**Weeks before the field existed are read through the words the file used then**, so the early bars
are shorter than nine: a risk with no status line at all counts in none of the three, because an
entry that said nothing said nothing. `MEASURED` and `AUDITED` are read as `RUN` for those weeks,
and `NOT YET`, `UNRUN` and `UNTESTED` as `NOT-RUN`, `NOT-RUN` and `CANNOT-RUN`. None of those five
words may be written today.

### Kani proof harnesses, and what can falsify them

The harness count is the whole series; the split into "falsification on record" and "unfalsified"
has exactly one point, because the record itself is four days old. Milestone 194 (build §134: the
falsification record, its lint, and the sweep that replays it) built it, and DECISIONS §134 (a
harness carries a machine-replayable falsification record, or it is not evidence) is the reason.

**36 of 146.** This is fatal risk 2's number, and the risk is that the proofs prove trivia. A proof
nobody can turn red is one level of indirection away from evidence, which is the same shape as a
roadmap status that was wrong in both records. A harness with no record at all is counted as
unfalsified, because that is what it is.

The harness count itself is taken after blanking comments, which matters more than it sounds: a raw
grep for `#[kani::proof]` finds 151, because `kernel/src/syscall.rs` explains in prose what one is
and `vendor/` and a lint shim carry four more.

### unsafe blocks outside `kernel/src/arch/`

Blocks per 10,000 code lines, outside `kernel/src/arch/`, which is `script/lint`'s census and the
one number in this tree that is actually gated on a direction. The dashed line is the ceiling.

227, 243, 138, 121, 111, 93, 77, 77. **The absolute count of unsafe blocks outside `arch/` went from
171 to 704 over the same period while the density fell by two thirds.** Both are true and only the
second one is about soundness: the first is a system being built. That is why the gate holds a
ratio.

The one rise is 2026W29 to 2026W30, and it is the honest shape of an early kernel: the tree was
7,500 lines and one driver moved the number.

#### How much this particular series has actually been checked

This is the one place on the page where a number has been checked two different ways, so it is worth
separating what that bought.

**A consistency check, which is what comparing against `notes/unsafe-obligations.md` is.** That note
carries a seven-point table, 227.8 down to 78.8, taken on seven different dates. It is *not* an
independent record: it is the output of this same derivation, written down seven times by lanes.
Reproducing it proves the walk agrees with what was recorded, and nothing more. It did agree. On
2026-08-18 the reconstruction hits **747 blocks over 80,359 lines exactly**, at commit `93607fa4`,
which is both figures to the digit. `unsafe impl Send`/`Sync` and the inside-`arch/` count match the
recorded value at five of the seven dates, and are within three at the other two. The residual is
which commit inside the day the column was taken at, not a difference in definition. The 2026-08-23
column is the worked example: the table prints 777 and this reconstruction prints 799, and the note's
own prose already says the count immediately before that lane's reduction "was 799, not 777", over
85,476 lines, which is this reconstruction to the digit.

**An independent check, which is a different thing.** A second counter was written from scratch for
this: a character scanner that tracks Rust's real lexical states rather than blanking comments and
literals with one regular expression. It differs deliberately in two places the regex is loose:
block comments **nest** in Rust and the regex's do not, and a lifetime `'a` is not the start of a
character literal. Run over every in-scope file at `705e3919`, the two methods agree on **701 blocks outside `arch/`
and 253 inside, with zero files disagreeing** (that commit's own row; the number moved with the tree
afterwards). That is the check worth having, and it says the shortcuts in `script/lint`'s regex do
not bite in this tree.

### The same unsafe blocks, by trust boundary

**The chart above this one mixes two populations that mean opposite things, and this is the same
census with that mixed once and for good.** `unsafe_outside_arch` (824 today) adds every `unsafe`
block that is not in `kernel/src/arch/`, kernel and userspace both, into one number. In a capability
microkernel that is not a detail: an `unsafe` block inside `kernel/src` runs with nothing confining
it, and an `unsafe` block in a userspace program (`components/`, `fixtures/`, or a crate that ships
only into one of them) is confined by the same MMU-plus-capability-table mechanism that confines
every other program. A single density cannot answer "how much of the code that matters for isolation
is unsafe", because it never separates the two populations that question is about. Raised as exactly
this problem by a research lane on 2026-09-20 (`notes/trusted-base.md`, landing alongside this
section, written for the RedLeaf comparison; see `notes/redleaf.md`), which hand-computed a first cut
and flagged in its own `BUGS` that nothing kept it computed. This split is that mechanism.

**Today, 2026-09-20: 694 kernel, 382 userspace, 19 shared, 37 boot chain, 0 unclassified**, which
sums to 1,132, not 1,138 (824 + 314). The 6-block difference is not the split disagreeing with the
old census; it is a separate, smaller correction this pass found while classifying `crates/`: three
crates (`board_console`, `portable_executable`, `stick_maker`) are host tooling that runs on the
developer's Mac and never on nife, exactly like `bench/host/`, `xtask/` and the rest of
`unsafe_census`'s own `HOST_ONLY` exclusion, just not under one of `HOST_ONLY`'s path prefixes; each
says so in its own header. `unsafe_outside_arch`/`unsafe_density` above are left untouched by this
finding (they still mean exactly what they meant, and `script/lint`'s ceiling still gates the same
number it always has); this split simply does not count those three crates' 6 blocks toward either
side, because the question this split answers, kernel privilege or userspace confinement, has no
answer for code that runs on neither. Whether `unsafe_census`'s own `HOST_ONLY` should widen to match
is calef's call, recorded rather than made here.

**What each bucket is**, and the boundary decisions behind it are in `scripts/rust_source.py`'s own
comment on `trust_boundary_census`:

- **kernel** (694 blocks, 48,724 code lines, density **142** per 10,000): `kernel/src/**` (arch and
  not) plus the sixteen `crates/` members reachable, over a real `cargo metadata` dependency edge,
  **only** from the `kernel` package: `paging` and `dma_validator` among them, both lifted out of
  `kernel/src` on purpose so Kani could reach them. Counting only `kernel/src/**`, as the hand
  computation in `notes/trusted-base.md` does, misses these sixteen crates entirely: its 577 is
  *undercounting the trusted base by the 117 blocks those crates carry*, which is worth flagging to
  that note's own lane before it lands, since its `BUGS` section already names this exact risk
  ("a tree can shrink [the kernel line count] by moving code out of `kernel/src` without reducing
  what anyone has to trust") without checking whether it had happened to its own number.
- **userspace** (382 blocks, 31,639 code lines, density **120** per 10,000): `components/`,
  `fixtures/`, the sixteen `crates/` members reachable only from them, and five packages that are
  each their own cargo workspace and never appear in the main one (`redoxfs_server`'s `el0` build,
  `std_exerciser`, `entropy_backend`, `cryptography_exerciser`, `cryptography_provider`); every one
  of them is, by its own header, a program or library that runs on nife at EL0 and never as kernel
  code.
- **shared** (19 blocks, in 5 of 36 crates reachable from both sides): code that genuinely executes
  with kernel privilege in the kernel binary and, separately, under confinement in a userspace
  program. Not a hedge: `environment_protocol`'s `ConfigPage` and `clock_protocol`'s `ClockPage` are
  built by the kernel's `unsafe fn new`/`from_raw_parts` and read back through the identical
  accessor by a userspace `std` program, so the same unsafe source is real in both roles. Folding it
  into either side would overcount one and undercount the other; telling apart, per call site,
  whether a specific block also runs from the kernel's own `#[cfg(test)]` oracles (several of these
  crates' `Cargo.toml` comments say their kernel-side use is exactly that, a test predicting what a
  client sent, and would never ship) is a source-level read this pass did not do and is recorded
  as future work.
- **boot chain** (37 blocks): `uefi_loader` and the one crate only it reaches (`sealed_pair`). It
  runs once, before the kernel starts, with the full privilege of the pre-OS environment, to decide
  which kernel image gets control, and its memory is gone by the time the kernel's isolation
  boundary exists to enforce anything. That is a chain-of-trust question, not a runtime-isolation
  one, so it is reported on its own rather than folded into either of the other two; which claim it
  backs is calef's to decide.

**Which number the ceiling should be held against.** `script/lint`'s `<!--count-at-most:unsafe-
density-outside-arch-->` (`notes/unsafe-obligations.md`) holds the mixed density (currently 88
against 77) and this pass does not change that marker's value; a gate's threshold is calef's. What
this pass can say is what the ceiling would mean under each candidate: **142** if held against the
kernel-only density, which is the number that answers "how much of the code nothing confines is
unsafe"; **120** against the userspace density, the confined population, where a ceiling matters
less because a bug there is a bug in one program, not in the base; or **77**, the status quo, which
answers neither question precisely because it is built from both. Recommendation: the kernel density
is the one worth a ceiling of its own, because it is the population where the mixed number's blind
spot actually lives, but a ceiling set on it starts cold (no history of it moving deliberately) and
that is a decision for calef to make with these numbers in hand, not one this pass makes for him.

**The history, backfilled, and where it is honest about a gap rather than papering over one.**
`script/metrics --backfill` restated the whole series with this split, the same way milestone 448
restated `SUPERSEDED`/`REFUSED` into weeks already written. Of the ten weeks recorded before
`--backfill` ran, **eight (2026W29 through 2026W36, then W37 too) carry a nonzero
`unsafe_trust_unclassified`** (57 up to 188 blocks), because this tree spells its crate names out and
most of them did not always have their current spelling: `crates/ipc`, `crates/dtb`, `crates/asid`
and around forty more were renamed to `inter_process_communication`, `device_tree_blob`,
`address_space_identifier` and so on over the weeks this series covers, and the classification table
in `scripts/rust_source.py` is built from today's names (the same restatement trade
`MILESTONE_STATUSES`/`NAME_STATUSES` already make, stated in that file's own header). **This was not
reconstructed for the same reason a shortcut was refused rather than taken**: `git log
--diff-filter=R --summary` finds candidate renames, but at least one pairing it offers is wrong
(`crates/canary_gate` paired with `crates/work_steal_slot` by a `Cargo.toml`-only content match,
while `canary_gate`'s own `src/lib.rs` correctly pairs with `memory_corruption_canary_gate`), and a
hand-verified alias table for around forty old names, several of them (`mdns_proto`, `smb_proto`,
`ntlm`) naming protocols since removed outright (SMB and Time Machine are out of this project's
customer path entirely, per `AGENTS.md`) with no current bucket to map to at all, is real work this
pass chose not to rush. **From 2026W38 (2026-09-20) the split is exact: `unsafe_trust_unclassified`
reads 0.** A future lane rebuilding that alias table, verified file-by-file rather than trusted from
`--summary` alone, is the way to close the gap; until then, read the early weeks' kernel/userspace
bars as undercounts of both sides by whatever their `unclassified` band carries, exactly the caution
this page already asks for `names_no_block` in the weeks before naming provenance existed.

**That backfill was lost for two days and restored 2026-09-23.** The `unsafe_trust_*` columns
above landed on `main` at 2026-09-20 23:30 (`3764a78d3`) with the ten weeks above already backfilled,
matching this section's own numbers. `weekly.csv` was then the single series file; milestone 581
(one metrics file per measure) has since retired it. The next commit to touch `weekly.csv`
(`6e8934474`, "Rebuild the weekly series after the merge", 2026-09-21) was resolving a conflict
between two branches that had each added columns to the same file, and its message says the plan
was to restore its own branch's cost columns and "let `script/metrics` add the trust-boundary
columns from the merged script." What ran next was `--update`, not `--backfill`: `--update` only
recomputes the current week and any week missing outright, so it added the ten new column headers
to every row but left `unsafe_trust_kernel`, `unsafe_trust_kernel_code_lines`,
`unsafe_trust_kernel_density`, the userspace triple, `unsafe_trust_shared` and
`unsafe_trust_boot_chain` **empty, not zero**, for 2026W29 through 2026W38, which is why
`unsafe-trust.svg` drew one point instead of ten from 2026-09-21 until this was found and
`script/metrics --backfill` was rerun on 2026-09-23. The nine restored values match the
2026-09-20 backfill exactly (the `unclassified` counts for 2026W29 through 2026W37 are the same
57-to-188 figures this section already names, and 2026W38 is still exactly 0), so this is the same
known rename-alias gap reappearing rather than a new one. **The lesson is about the merge, not the
census**: any column added by one branch while another branch is independently adding columns to
the same CSV needs a `--backfill` after the conflict is resolved, because `--update` treats every
already-present row as already correct and will not notice that a resolved merge just introduced
blank cells into it. Checked for the same shape elsewhere on 2026-09-23 and found nowhere else:
every other column that could go this route (`milestones_superseded`, `milestones_refused`, and
the fatal-risk columns, then `fatal_risks_tested` and `fatal_risks_untested`) was in fact backfilled
at the commit that added it,
and the flow columns (`milestones_built_this_week`, `merged_pull_requests`, the four cost columns)
are recomputed for every row on every run regardless of mode, so they cannot hold a stale blank this
way.

### Milestones built each week

**The only flow on this page.** Every other milestone number here is a stock: what the tree holds at
a revision. This is what happened in those seven days, and it is the one that answers "how fast".
Added 2026-09-20 at calef's request, who asked for it per day first and then for per week, which is
the same number over seven and fits the cadence every other series on this page already has.

**What it is counted from**: the index's `Built` column, one row per milestone, with `REMOVED` rows
counted too because removal does not unbuild anything (`script/roadmap`'s own rule is that a
milestone built on a date turned BUILT on that date, and nothing later makes that untrue).

**It is derived from today's tree for every week, which breaks this file's usual rule, and the
reason is worth keeping.** Computing it from each week's own revision was tried first and is wrong.
The index was generated by milestone 294 (`design/roadmap/README.md`'s index is generated) and
regenerated at an integrator's convenience, so a week whose revision predates its own regeneration
reports zero and a later week reports both. Backfilled that way on 2026-09-20 it gave **0** for
2026W29, 2026W30, 2026W31 and 2026W33, against **10, 8, 20 and 9** actually built in those weeks. A
stock read late is merely stale; **a flow read late lands in the wrong bucket**, which is worse than
stale because it still looks like history. A BUILT date is a stable fact, so reading them all from
one tree is not a revision of history but the only way to get it right. The cost, stated because it
is real: this column alone changes for a past week if somebody corrects that week's date, and
correcting it is what we would want.

**A milestone is not a fixed unit, and this chart cannot fix that.** They range from one build flag
that turned exit 101 into exit 0, in milestone 164 (x86_64 userspace can't build `aes`), to a
three-architecture bring-up. Read the shape, not the height. `AGENTS.md` says the same of its own
count: take it as scale, never as a claim about correctness.

**And some of the height is bookkeeping rather than building.** A week in which an integrator
promotes proposals to milestones, or backfills refusals as their own blocks, moves rows into the
roadmap that nobody built. Those land as `NOT-STARTED` and `REFUSED` rather than `BUILT`, so they do
not enter this chart, which is the reason it counts dated rows instead of differencing last week's
`BUILT` total against this one. A difference would be wrong in both directions and quietly: a
`PARTIAL` block that turns `BUILT` was already counted as a milestone, and a block that turns
`SUPERSEDED` leaves the `BUILT` column without anything having been unbuilt.

#### 2026-09-23: this chart does not reconcile with the `Built` stock, and that is the design

calef read the effort chart above and asked which milestones 2026W35's denominator of **8** covered.
Diffing the `Built` total between that week's row and the previous one gives **6**, and so does
diffing `**Status:**` across every roadmap block between those two rows' own commits. Three numbers,
two answers, and the larger one is the one that flatters the project: 8 in the denominator prices
2026W35 at about 1,021 million tokens per milestone, 6 prices it at about 1,362.

**All three are right, and the column is the one to trust.** The two sixes are the same
measurement wearing different clothes. Both ask what the roadmap *said* at two moments a week apart;
the column asks what was *built*, from the dates the roadmap carries today. Those differ by
whatever lag sat between a milestone being finished and its row being flipped, and that lag is not
small here:

| week | built that week | `Built` stock | stock change | counted here, recorded later |
| --- | --- | --- | --- | --- |
| 2026W29 | 10 | 0 | | 2, 3, 4, 5, 6, 7, 8, 9, 10, 11 |
| 2026W30 | 8 | 0 | 0 | 12, 13, 14, 15, 18, 19, 21, 26 |
| 2026W31 | 21 | 26 | +26 | 50, 69, 70 |
| 2026W32 | 31 | 63 | +37 | 58, 65, 67, 104, 107, 112, 115 |
| 2026W33 | 9 | 73 | +10 | 80, 93, 125 |
| 2026W34 | 18 | 97 | +24 | 344 |
| 2026W35 | 8 | 103 | +6 | 191, 193 |
| 2026W36 | 49 | 150 | +47 | 375, 380 |
| 2026W37 | 13 | 160 | +10 | 281, 285, 286, 287 |
| 2026W38 | 64 | 228 | +68 | 434, 447, 518 |
| 2026W39 | 16 | 246 | +18 | 512 |

**Eleven weeks, eleven disagreements, and not one of them is a milestone this chart invented.**
Every number in the last column turns up in a later week's stock change, which is the whole of the
gap. 2026W35's two are the shape in miniature. Milestone 193 (put `kernel/src` within reach of the
prover) was flipped on 2026-08-30 and the week's index regeneration had already happened, so the
stock records it in 2026W36. Milestone 191 (did the proofs catch the bugs? a retrospective of
every real defect) was flipped on 2026-09-11 under a commit that says so in its subject, *"has
been BUILT since 2026-08-30 and nothing noticed"*, so the stock records it in 2026W37, twelve days
after the work. 2026W29's ten are the extreme case: milestones 1 to 11 were backfilled as blocks
weeks after the kernel they describe booted, so a stock difference puts the project's entire first
fortnight in 2026W31 and 2026W32.

**The totals do agree, which is the check that matters.** The tree holds 247 `BUILT` plus 3
`REMOVED` blocks, and one of the three carries no `**Built:**` date because it was removed before
it was built: milestone 55 (Time Machine: SMB3 with Apple's extensions, and mDNS). That is 249
dated blocks, and bucketing them by week accounts for all 249: the 247 in the table, plus the one
that falls before the series starts, plus one dated into 2026W39 after that row was last written,
since the current week is still open and is rewritten on every run. The weekly buckets differ from
the stock; the population does not.

**One milestone falls off the left edge, and `script/metrics` now says so.** Milestone 1 (boot to
Rust on QEMU `virt`, and print to the PL011 UART) is dated 2026-07-12, a Sunday, which is 2026W28,
and the series starts at 2026W29 because that is where the first commit is. It is dropped rather
than absorbed into the neighbouring week, for the same reason the $200 of subscription in the same
week is: a week gets a row only if a commit fell in it, and inventing one would leave every other
column in it empty or wrong. That drop was silent until 2026-09-23; the cash column had carried a
stderr warning for its identical case since it was written, and this one now does too.

**Nothing published changes.** The column was checked against the blocks and reproduces exactly, the
stock column reproduces exactly at all eleven of its own commits, and every other column was checked
for the gap that left `unsafe_trust_*` blank for ten weeks (a `--update` where a `--backfill` was
needed). Only the cost columns and coverage are blank anywhere, and those are blank on purpose,
which is the section below. `milestones_built_this_week` cannot take that damage: it is written for
every week in the file on both `--update` and `--backfill`, never only for the week being added.

### Pull requests merged each week

**The second flow on this page, and it is here because the first one undercounts.** Milestones count
*declared* units of work. A large share of what lands on `main` is not one: a decision, a correction,
a record fix, a capture, a gate that was wrong about the tree. On 2026-09-21 the maintainer opened
about fifteen pull requests of which three were milestones, and every column on this page was blind
to the other twelve. Added at calef's request the same day, alongside milestone 519's cost columns,
because that invisible work is most of what the cost columns are measuring.

**Counted from git, never from the GitHub API**, which is the reason it backfills to the first commit
like everything else here. A merge commit carries its own committer date and `main` holds all 996 of
them; Actions retains run history for ninety days, so an API-derived series would have started in
June and could never have been extended backwards. The shape matched is GitHub's own default merge
subject, `Merge pull request #N from ...`, which is what the merge queue writes; a merge made any
other way is not counted and cannot be told apart from an ordinary merge commit after the fact.

**Read it beside the velocity chart, because the pairing is what is diagnostic.** Many pull requests
against few milestones means the week went into record-keeping or into churn (reverts, fix-ups,
corrections). Those are very different weeks and this page cannot tell them apart; it can only make
the question visible, which is more than it could do before.

**2026W29 and 2026W30 are genuine zeros.** The first two weeks were commits straight to a branch
with no pull request; the practice starts in 2026W31 with 8.

#### Commits are deliberately not a series here

There are 5,053 of them as of 2026-09-21 and they will not be charted, and this paragraph exists so that the next
person does not add the obvious omission. The count measures **lane hygiene at least as much as
output**. `AGENTS.md` requires a lane to commit whenever a piece works, because uncommitted work in a
worktree is the one thing no part of this system protects, and then to squash the checkpoints into
purposes before reporting. So the number reflects how faithfully lanes did both, and a week where
lanes squashed well would look *less* productive than one where they did not. It stays a headline
figure in prose, where it can carry that sentence, rather than a line on a chart where it cannot.

#### And a pull request is not a unit of value

Merging four small corrections is four pull requests, and it might be the best hour of the day or the
worst. This series answers **how much landed**. The cost columns below answer **what it took**.
Neither of them answers whether it was worth doing, and a rising line here should not be read as
though one of them did.

### What this project costs

**Three units, and deliberately no fourth that adds them up.** Milestone 519 (what this project
costs, tracked where it cannot rot) refuses a single headline number, because three audiences want
three different ones: person-weeks for the comparison against the verification literature, tokens
with a dated price for the argument that the method is getting cheaper, and cash for calef's own
budgeting. The columns are `human_person_weeks`, `lane_tokens` with `lane_wall_clock_hours` and
`price_per_mtok_at_date`, and `cash_spend`.

**The shape those numbers make is the finding.** Cash to date is under a thousand dollars all in,
against eleven person-weeks of one experienced engineer's full attention (milestone 519's block
says "about ten", written two days before this column existed; the column counts eleven calendar
weeks, 2026W29 through the current partial 2026W39, and the difference is arithmetic rather than a
correction). At any plausible rate
for that person's time, **the human cost is on the order of 99% of the economic cost and the machines
are a rounding error.** The claim this project can honestly make is not that software became cheap.
It is that the scarce input is still a person, and what changed is how much one person's attention
can be made to carry.

#### The chart is a ratio, and it is the one that answers the claim

A total would say the project was busy. Millions of tokens per milestone built says whether the
method got **cheaper**, which is the claim principle 2 of `AGENTS.md` actually makes. It fell from
307 in 2026W34 to 73 and 68 in 2026W36 and 2026W38. Four things before anyone quotes that:

- **A milestone is not a fixed unit**, which the velocity section says at length and which this
  inherits with interest, because here it is the denominator. A week that built one build-flag fix
  and a week that built a three-architecture bring-up divide by the same 1.
- **2026W35's 1,021 is the highest and its week built 8 milestones**, against 2026W36's 49. Both
  numbers are real. Whether that week was expensive or merely undeclared is not answerable from here,
  and the pull-request series above is the first place to look.
- **The current week is always understated.** Its tokens accumulate all week and its milestones land
  in a burst near the end.
- **Four weeks are marked as not captured rather than drawn as zero.** See the deadline below.

#### The deadline, and what was already gone when the capture started

Every other column on this page is computed from a git revision, so history backfills by reading the
tree at a past commit: that is how the mutation census, the velocity column and the coverage cells
were all filled in retroactively. **Token and wall-clock records are not in git.** They live in the
agent harness's session records under `~/.claude/projects/`, on one laptop, outside this repository,
and nothing promises to keep them.

The first capture ran on **2026-09-21** and read 477 record streams. It found **2026W34 through
2026W39**, and found **nothing at all for 2026W29 through 2026W33**, which is the first five weeks of
the project against a first commit on 2026-07-12. Those weeks are recorded as **absent, not zero**,
in the CSV and on the chart, because an absent week averaged in as a zero is a lie that ends up in a
published figure.

**2026W37 is captured and should still be treated as suspect.** It reads 12.2 machine-hours against
110 to 133 on either side, in a week whose roadmap says 13 milestones were built. Either that week
really was quiet, or its records were rotated away before anyone looked. Nothing can tell those apart
after the fact, and that is the argument for the snapshot described under *How it stays current*.

#### What `lane_wall_clock_hours` is, and what it conflates

The sum, over every record stream, of the gap between consecutive responses, dropping any gap longer
than thirty minutes. A stream is one session or one subagent lane, so **parallel lanes add**: four
lanes working for an hour is four hours here, and a week can exceed 168. That is the point, since it
is an effort figure rather than an elapsed one.

It **conflates queueing with work**. A lane waiting on the merge queue, on a gate, or on a rate limit
is indistinguishable here from one thinking. And the thirty-minute cut is a judgement, not a
measurement: a longer gap is a session left open overnight, and counting it would put a sleeping
laptop in the total. `IDLE_GAP_S=600 script/effort` shows how much the choice moves.

#### Two dollars, and which one is which

This is the refusal milestone 519 is sharpest about. **This project pays a fixed monthly
subscription, so the marginal cost of one more lane is zero dollars.** `cash_spend` is what was
actually paid: $200 a month since 2026-07-12, plus $271.81 of hardware on 2026-08-15.

`price_per_mtok_at_date` is a **shadow price**, and it is a rate rather than a total on purpose. It
is that week's own token mix priced at vendor list rates, so a reader who wants the shadow figure
multiplies it by `lane_tokens` and knows exactly what they have multiplied. Quoting one of these two
numbers while implying the other is the dishonest version, so neither ever appears here without its
label. Both rates and purchases live in
[`notes/project-metrics/ledger.md`](project-metrics/ledger.md), which is appended to by hand and
never regenerated.

**The blended rate moves with the mix, not only with prices.** It runs from $0.30 to $0.78 per
million tokens across the captured weeks while no published price changed, because this tree's work
is split roughly half and half between `claude-opus-5` and `claude-sonnet-5` by token, and because a
cache read lists at a tenth of an input token. A week that cached well reads cheap.

**$200 of real spend is outside this series and that is not rounding.** calef dates the subscription
2026-07-12, a Sunday, which is 2026W28; the first commit is 2026-07-13 UTC and the series starts at
2026W29. A week gets a row only when a commit fell in it, so `cash_spend` sums to $671.81 against
$871.81 actually paid. `script/metrics` prints the discrepancy on every run rather than folding it
into a neighbouring week.

#### `human_person_weeks` is a statement, and will stay one

1.0 per calendar week, because calef works on nife full time. It is not a measurement and nothing
will make it one without time tracking, which milestone 519 refuses on the grounds that a person
required to log hours stops volunteering the honest ones, and the metric is then worth less than what
it displaced. The only thing ever asked of him is a **correction** when the datum stops being true,
and the column is carried through every rewrite so that a hand-edited week survives the next
backfill. It is accurate to the bucket the comparisons need (seL4 at about eleven person-years plus
nine more for the proof; Atmosphere at 1.5 person-years on verification) and no better.

### Architecture decisions by status

From `design/decisions/README.md`. The grey band in the first three weeks is the honest bucket: a
decision was a `## N.` heading in one 5,320-line `DECISIONS.md` and **nothing said whether it still
held**. Milestone 114 (split `DECISIONS.md`, and give a decision a status) is where a status exists
at all, so counting those early decisions as `DECIDED` would invent a claim the record never carried.
They are counted, and counted as having no status.

139 decisions in eight weeks, of which 17 are `AMENDED` and 3 `SUPERSEDED` at 2026W36. Twenty
decisions revised or replaced out of 139 is the number that says the vocabulary is doing work: a tree
where nothing was ever amended would mean either that every first answer was right or that nobody
went back.

`PROPOSED` is the queue waiting on calef and it stays small (10, 4, 8, 2, 3). It is a queue depth
rather than a backlog, which is the shape it should have.

### Names by what the tree records about them

From the provenance block in each named thing's own header, which is where milestone 115 (the names that
were ratified, and the ones that were refused) put it: a crate's `src/lib.rs`, a program's, a `script/` entry
point's comment, a Cargo package's manifest. `script/names` derives the same four counts by walking
the working tree; this derives them from git history. They share the parse
(`scripts/name_provenance.py`) and not the file walk, so the two agree by construction rather than by
luck, and at 2026W36 they do: 204 names, 104 `ratified`, 37 `recorded`, 63 `provisional`, 0
`unrecorded`.

The four words are what a block *says*. **`Ratified`** is calef ruling, with a date and what was
refused. **`Recorded`** is the tree arguing the name somewhere and nobody ever putting it to him.
**`Provisional`** is whoever coined it saying out loud that they expect it to change. **`Unrecorded`**
is nothing outside the block saying why the name is what it is.

**The total is every named thing, and the four statuses do not have to add up to it.** The pale band
is the difference: named things carrying no block at all. That is a different claim from
`Unrecorded`, which is a block saying the history is silent, and the two are kept apart for the same
reason the decisions chart above will not read a statusless decision as `DECIDED`.

**The first three weeks are the sharpest restatement artifact on this page, and there the band is the
whole bar.** There were 16 named things at 2026W29 and 119 at 2026W31, not one of them carrying
provenance, because the convention that records it did not exist until 2026-08-04. So read those
bars as "nobody was writing this down yet", and read the first blue bar as a convention arriving
rather than as 72 names being ratified in a week. Backfilling them was considered and refused: a
ratification invented to fill a cell would put a false claim in the one record whose entire job is
saying who claimed what.

**The band that survives into 2026W32 and 2026W33 is not an artifact, and it is the most useful thing
this series found.** It is seven names, and all seven are Cargo packages: `kernel`, `user`, `xtask`,
`redoxfs_server`, `redoxfs_host`, `std_exerciser` and the fuzz package. Milestone 115 covered three
surfaces and a package was not one of them, so for two weeks `script/names std_exerciser` answered
"neither a name in the tree nor a recorded refusal" while looking exactly as authoritative as a true
answer. calef found it on 2026-08-18, the `package` kind closed it, and the band goes to zero in
2026W34. **Nothing told this series about that hole**; it walks four kinds today and finds the fourth
missing from the weeks before it existed. A registry with a hole answering confidently is the failure
`script/names`' own header records, and this is what it looks like from outside.

**2026W36 is one milestone doing one thing.** `Unrecorded` goes from 60 to zero, `Recorded` from 9 to
37 and `Provisional` from 18 to 63. That is milestone 264 (sixty names the history cannot justify,
and the research that would let calef rule on them), which converted every name nobody had written a
reason for into one that says something: the reasoning where the history supplies it, an argued
proposal where it does not. The bar is the same height it would have been without it. What changed is
what the tree can say about the names in it.

#### A rising `Provisional` band is not debt

This is the number here most likely to be misread, and the misreading would cost something real, so
it is worth saying flatly.

**Nothing in this tree fails because a name is unratified.** `AGENTS.md`:

> `script/names --unratified` is a worklist rather than a wall precisely so that an unratified name
> never blocks anyone's build.

`script/names --check` gates on a block being *present*, never on it saying `ratified`, and that is
deliberate: a gate that demanded the queue be drained would block every unrelated merge behind a
review nobody can hurry.

**A provisional name is the mechanism working, not the mechanism failing.** It is what `AGENTS.md`
tells a lane to ship when it needs a name and the decision is calef's, and it exists to convert an
expensive decision into a cheap one by refusing to pretend it is settled. A lane that coins a name,
argues it, records what it refused and marks the result provisional has done the thing the convention
asks for. A lane that quietly ships a name without saying it is unsigned has not, and it will not
show up on this chart at all, which is the limit of what a count of signatures can see.

So the green band going up means lanes are naming things and being honest that nobody ruled. It is a
queue depth against one person's attention, and this project's scarcest resource is exactly that
attention, so a growing queue says the tree is growing faster than one reviewer rules on it and says
nothing at all about the names being wrong. **The band worth an alarm is `Unrecorded`**, because that
is a name nobody anywhere argued for, and it is the one this chart has at zero.

### Unnumbered proposals

**74 at 2026W36, and zero in every week before it**, which is the `proposals_unnumbered` column in
the CSV. `design/roadmap/proposals/` was created on 2026-09-04 by milestone 247 (follow-on work named
by a finished milestone goes nowhere, and this is the third time).

**They are drawn on top of the milestones chart, since 2026-09-19**, as the eighth series. This page
used to say there was no chart because there was one bar, which was true at 2026W36 and stopped
being true two weeks later without anybody revisiting it: the column had been collected every week
and drawn nowhere. **The bar totals on that chart now include them**, so 2026W38 reads 431, which is
324 numbered milestones and 107 proposals, and the jump at 2026W36 is the pile appearing when the
directory did rather than a burst of milestones. They sit on top because they are the work that has
not entered the roadmap yet, and because a new slot is appended so that no existing series changes
colour.

**Nothing else on this page could count these, and that is the reason for the column.** The
milestones chart keys on a milestone number, reading the blocks in `design/roadmap/` for a revision
that has no index and the index rows for one that does (a week counted before a stale index was
regenerated undercounts by however many milestones landed in between; it self-corrects on the next
regeneration, and past weeks are read from their own revisions).
A proposal is *defined* by not having one: a lane that finds work it is not doing writes
`design/roadmap/proposals/<slug>.md`, because the thing concurrent lanes collide over is the number
and not the authority, and an integrator assigns the number at promotion. So the pile was invisible
to every column here by construction, not by oversight.

#### What a rising line means here, which is not what it means for names

The naming section above says a rising `Provisional` band is not debt. **Do not carry that reading
across.** A provisional name costs nothing while it sits, because nothing is waiting on it. An
identified piece of work that nobody has scheduled is a different object: something in this tree was
found to be wrong or missing, and the finding is parked. That is closer to debt, and it would be
dishonest to file it under the same reassurance.

But the count alone cannot tell you whether the pile is stalling, for two reasons that are worth
stating rather than leaving to a reader's optimism.

**It is a net count, and the flow is gross.** Five proposals have left the directory since it
existed, so 86 have been written and 81 remain. They left in two different ways, which is the more
interesting half: one was promoted to a number the ordinary way (milestone 256 (x86_64 places PCI
BARs in a hardcoded window, and on xenon that window is RAM)), and the others were **done**, by a
lane that picked the file up and fixed the thing, sometimes filing a narrower proposal in its place (`the-tcb-capability-that-outlives-start`
became a fix plus `the-region-half-of-the-retention-declaration`). A flat line on this column would
be consistent with a stalled pile and equally consistent with one draining exactly as fast as it
fills, and nothing here distinguishes them.

**Age is the tell, and age is not in this column.** `script/roadmap`'s own header says so: a gate on
age ("no proposal older than N days") would be routed around by not writing proposals, which is
worse, so what it does instead is print the count and the date of the oldest on every `script/lint`
run. Today the oldest is 2026-09-03 and the directory is a week old, so nothing has had time to go
stale and the count says nothing yet. **The number to watch is not this one going up; it is this one
going up while the oldest date stops moving.** `script/roadmap --proposed` lists them oldest first
and is the view that answers it.

**And a rising line is still better than the alternative it replaced.** The work in this pile used
to live in lane reports, which are read once, by one person, on the day they are written.
`AGENTS.md` records what that cost: milestone 90 (a guard page under the per-CPU secondary stacks)
exists only because calef happened to be at his desk the day a report named it.
Milestone 94 (the untracked-work sweep, and the convention that ends the category) swept the tree
for exactly this category and then left its own inventory in a pull request body for twelve days. 74 visible proposals is a worse
number than 74 scheduled milestones and a far better one than 74 findings nobody can enumerate.

### Milestones by status

From the milestone blocks in `design/roadmap/`, and from `design/roadmap/README.md`'s index table
for the weeks before calef retired it on 2026-09-21. The two zero weeks are a restatement artifact and
they are the sharpest one on this page. There really was a roadmap in 2026W30: `design/roadmap.md`
landed 2026-07-22 with ten rows in it. It had no status column. A milestone's state was prose inside
a cell, phrased a dozen different ways (`Built`, then `Built (frame scope)`, then `Built:` followed
by a paragraph on which half), which is exactly the defect the status vocabulary was later minted to
fix, and it is unparseable now for the same reason it was unreadable then. The first bar this chart
can draw is the week the column exists.

Rows 1 to 11 were backfilled into the roadmap later by milestone 76 (split the roadmap:
`design/roadmap/README.md` as index, one file per milestone). Its index half was retired
2026-09-21 and the one-file-per-milestone half is the roadmap today. Eleven milestones had been built before
the first bar; none of them is in it.

**The interesting line is not `BUILT`, it is `NOT-STARTED`.** Reading the rows as they stand at
2026W36: built milestones went 26, 63, 73, 97, 103, 125 across the six weeks the roadmap has
existed, which is a steady rate. Not-started went 16, 35, 32, 43, 62, 79. **The roadmap is growing
faster than the lanes drain it**, and the gap widened most in the last two weeks. That is what a
project generating its own work looks like, and it is the number to watch if the ranking function
ever needs defending: a backlog that grows faster than it is consumed is only healthy while
something is choosing the order.

`REMOVED` appears for the first time in 2026W36, with two rows. The token itself was minted
2026-08-30, when milestone 54 (a network file service a Mac can actually mount) was deleted and the
six words then available could only lie about it.

**Two statuses were missing from this chart's own vocabulary until 2026-09-20, and one of them had
been missing for five days without anyone noticing.** `script/metrics` keys the count on a fixed
list, and a token it does not hold is counted as nothing rather than as an error, so
`milestones_total` went short by exactly the blocks it could not see. `SUPERSEDED` was minted
2026-09-15 and never added: seven blocks were invisible, and 2026W38's total read 435 where the
tree had 442. Milestone 448 (a refusal gets a number, a status, and a condition that would change
it) added `SUPERSEDED` and `REFUSED` together and restated the history with `--backfill`, so the
correction reaches the weeks already written rather than only the next one. The failure is worth
keeping in view: a chart that undercounts silently looks exactly like a chart that is right.

**`REFUSED` will move every denominator on this page, and it is not work appearing.** Milestone 448
backfilled 42 blocks for refusals that name executable work, taking the roadmap from 444 milestones
to 487. None of them is a backlog item: they are excluded from `script/roadmap --ready`, from the
gate classification and from every count that reads as outstanding, and the ready count was 120
before and 120 after. The numbers that move are the totals a stranger quotes, so they are recorded
here rather than left to be rediscovered as a cliff in a bar chart. The column arrives on this chart
only when the index table is regenerated at merge, because this chart reads index rows and a lane
never edits that table.

### Rust in the tree

Every tracked `.rs` file outside `vendor/`, with `kernel/src` split from the rest. The split is the
point rather than a courtesy: the kernel is commented far more heavily than production code would be,
deliberately, so a single tree-wide ratio hides the thing that makes the number interesting.

A line counts as code if anything survives blanking its comments and string literals, and as a
comment otherwise. A line with code and a trailing comment is code.

**Two things worth reading.**

**The kernel's comment share is not flat, it is climbing.** `kernel/src` was 39.3% comment lines at
2026W31 and is 45.3% at 2026W36, rising every week in between. `AGENTS.md` says 40%, which was
accurate when it was written and is now three weeks stale. The rest of the tree is doing the same
thing more slowly, 31.5% to 41.4%. Whether that is the kernel getting better documented or the
comment-to-code ratio drifting past what a reader wants is not a question this instrument can
answer; it can only say the number moved. Flagged rather than corrected in `AGENTS.md`, because that
is calef's file.

**Total Rust fell for the first time between 2026W35 and 2026W36**, from 192.0k lines to 189.0k,
while the week was still adding code. That is milestone 54's deletion, which is also the `REMOVED`
bar in the Milestones by status chart above. A line count that only ever rises is measuring typing;
one that falls when code is deleted is measuring the tree.

### BUGS sections

**Rising is good here, and it is worth saying plainly because the word says the opposite.** A `BUGS`
section is the FreeBSD convention this tree copies hardest: an honest limitation written next to the
feature it limits, in the manual, rather than hidden in a tracker. `AGENTS.md` calls them "not
modesty, they are the mechanism", and the reason is a newcomer's: someone who hits a limitation the
docs named will trust the docs, and someone who hits one the docs hid will not trust anything again.

Zero in the first two weeks, 359 at 2026W36 (238 markdown headings and 121 in Rust doc comments).
A falling line here would be the alarming one.

### Coverage

**Every week but the first is measured, and each was measured by its own tree.** This section
used to say coverage could not be recovered from history. That was a rule about `script/metrics`,
which may not build or check anything out, mistaken for a fact about the measurement. On 2026-09-19
a lane checked out each week's representative commit in a throwaway worktree, ran **that commit's**
`script/coverage` on **that commit's** pinned nightly, and handed the lcov to
`script/metrics --coverage-for <WEEK> --coverage-from <lcov>`. That is the same instrument the
weekly workflow runs, so the backfilled cells mean what the live ones mean.

**This is the one series on the page that is not a restatement.** Every other column applies
today's definitions to an old tree. Coverage applies each week's own: its own crate exclusions and
its own feature flags, so the scope moves from week to week exactly as it moved at the time.

| week | commit | toolchain | lines hit / found | per cent |
|---|---|---|---|---|
| 2026W29 | `a80e5182d54c` | none | | **empty** |
| 2026W30 | `aef018cdaa3a` | nightly-2026-07-26, **reconstructed** | 1954 / 2114 | 92.4 |
| 2026W31 | `190268d086f0` | nightly-2026-08-02 | 11019 / 12106 | 91.0 |
| 2026W32 | `f6fd097488b1` | nightly-2026-08-04 | 15712 / 16746 | 93.8 |
| 2026W33 | `60698aa1a594` | nightly-2026-08-16 | 20573 / 21976 | 93.6 |
| 2026W34 | `132f6ad08ade` | nightly-2026-08-23 | 25084 / 26624 | 94.2 |
| 2026W35 | `685900ec6bf5` | nightly-2026-08-30 | 27330 / 28959 | 94.4 |
| 2026W36 | `d0b254c5a5e6` | nightly-2026-09-06 | 26066 / 27604 | 94.4 |
| 2026W37 | `5cd67cd3f193` | nightly-2026-09-13 | 26472 / 27952 | 94.7 |
| 2026W38 | `5d9d5e4c1f6a` | nightly-2026-09-17 | 26063 / 27525 | 94.7 |
| 2026W39 | `3dd86628a` | nightly-2026-09-23 | 32365 / 33994 | 95.2 |

Four things a reader should hold against those numbers:

- **2026W39 was measured by CI, not on the dev Mac, and at a different commit from the one in its
  row.** The dev machine was under four lanes' builds that evening, and this page's own guidance is
  that a heavy job taken under contention is worth less than an honest gap. The lcov is the one
  `ci.yml`'s coverage job uploaded for `3dd86628a` (run 35917383053, green), which is the same
  instrument on the same week; the row's other columns are read from `35390e595f89`, later in that
  week. That is the "nothing ties an lcov file to a commit" caveat above, firing on purpose rather
  than by accident, and it is the same size of gap this page already accepts between a live cell and
  a backfilled one.

- **2026W29 is empty because no instrument existed.** `script/coverage` arrived on 2026-07-22.
  Running a later script on that tree would measure something the week never measured, which is a
  restatement of a different kind from the rest of this page, and the bar is left missing rather
  than invented.
- **2026W30's toolchain is a reconstruction.** Its `rust-toolchain.toml` said `nightly`, unpinned,
  so the compiler that week actually used is not recorded anywhere. The backfill used the nightly
  dated on the commit's UTC day, which is what the floating channel resolved to when it was made.
  Every later week is pinned and was measured on exactly its pin.
- **2026W36 was the control, and it reproduced.** Its 94.4 was measured by the weekly workflow on
  2026-09-07 on Linux (cargo-llvm-cov 0.9.0, 26029 of 27571 lines); the backfill on the dev Mac
  (cargo-llvm-cov 0.8.7) got 26066 of 27604. Both round to 94.4. The 33-line gap is the platform
  and tool version, and it is the size of error to expect between a live cell and a backfilled one.
  The tool is today's `cargo-llvm-cov` for every backfilled week, not the version each week had.

2026W38's earlier 94.6 was taken at that week's first Monday commit; it was re-measured at the
row's current commit, so its cell and the rest of its row now describe the same tree. 2026W31's dip
is a change of scope rather than of testing: the measured set went from 15 files to 49 that week,
which is also the week rule 7 turned `#[path]` modules into crates.

The floor `script/coverage` gates on is per file, not this aggregate; the dashed line is that floor
drawn for scale, and the panel below plots the number it is actually drawn against.

### The lowest-covered file

**The chart above is the trend; this one is the thing that can fail a build.** `script/coverage`
gates at 80% **per file**, never on the aggregate, and the two can move in opposite directions
without contradicting each other: the workspace can sit at 94.7% while one file slides from 85% to
81% and the aggregate does not visibly twitch, because that file is a few hundred lines out of
twenty-eight thousand. Reading the aggregate as "the coverage the floor is set against" is the
natural reading and it is wrong; this panel exists so the page stops inviting it. The argument is
`script/coverage`'s own, applied to the dashboard: a per-total number hides a hole, because a big
well-tested crate subsidizes an untested one.

**The series starts at 2026W39, and a short line here is a missing record rather than a new
problem.** A per-file minimum needs that week's lcov, and only the aggregate was ever kept from
each run, so there is nothing to recompute the earlier weeks from. Re-measuring them the way
coverage itself was backfilled would not help either: the exemption list in `script/coverage` has
changed several times (build scripts in 2026-08-30, the host half of `stick_maker` in 2026-09-19),
so a minimum taken today over an old lcov would be a minimum over a population that week's gate did
not have. Empty weeks are marked on the chart rather than drawn as zero, the same treatment
`unsafe_trust_*` gets for the ten weeks before its census existed.

**Where the number comes from, and why not from the lcov.** `script/coverage` writes
`target/llvm-cov/floor.txt` beside its lcov, in the same awk pass that applies the floor, and
`script/metrics --coverage-min-from <floor.txt>` reads it. Nothing re-parses the lcov to find a
minimum, and that is deliberate rather than tidy: the lowest file in an unfiltered lcov is one of
the files that cannot execute a line on the host (`virtio`, the protocol wrappers, a `build.rs`),
so an independently derived minimum would be a number no build can ever fail on. The exemptions
that decide the population live in `script/coverage` and nowhere else, so the minimum has to be
taken there too.

`floor.txt` carries more than the one plotted cell: which file is lowest, how many files are
gated, and how many a floor of 85 or of 90 would newly fail. `script/coverage` prints the same
three lines at the end of every run. That last pair is the number a proposal to raise the floor
needs, and it is reported rather than left to be re-derived from the HTML report.

**What the distribution actually says, measured 2026-09-23.** Over the 116 files the floor acts on
at `3dd86628a`: none under 80%, **one** in 80-84.9%, **one** in 85-89.9%, 16 in 90-94.9%, 66 in
95-99.9%, and 32 at 100%. The minimum is **84.0%**, `crates/machine_discovery/src/interrupt_id.rs`
(21 of 25 lines), and the next one up is
`crates/globally_unique_identifier_partition_table/src/lib.rs` at 89.7%. So **a floor of 85 newly
fails one file, and a floor of 90 newly fails two.** Raising the floor is not a backlog here; it is
two files. That is the number the question needs, and it is calef's to act on. `script/coverage`
prints it on every run, so it never has to be re-derived.

**The count at 90 is platform-sensitive, and the platform that gates is CI.** The same run on the
dev Mac later that evening (cargo-llvm-cov 0.8.7, nightly-2026-09-23) agrees exactly on the minimum,
84.0% and the same file, and on the aggregate to two decimals, 95.21 against CI's 95.2. It disagrees
on one file: `crates/globally_unique_identifier_partition_table/src/lib.rs` is **315 of 351 lines
(89.7%) on CI and 318 of 351 (90.6%) here**. Three lines, straddling 90, which is enough to move the
85-89.9% band from one file to none and the answer at a floor of 90 from two files to one. So a
floor of 90 would fail a build on CI while passing for the person asked to fix it, which is the
worse of the two directions. This is the same class of platform gap the 2026W36 control already
prices at 33 lines; it only becomes visible here because a single file happens to sit on the
boundary. A floor of **85** has no such ambiguity on either platform.

**A higher floor is not automatically a better one, and this panel must not be read as a target.**
`script/coverage` says it plainly: the floor is a floor, a file at 81% is not "done", and 100% is
not the goal because tests have to prove something. `AGENTS.md` is blunter still: do not add filler
tests. A floor raised above what the tree has earned pushes a lane toward writing whatever reaches
the number, and this tree has already caught three variants of a test that passes while proving
nothing. The number to watch on this chart is the direction, and a drop is worth more attention
than the level.

**BUGS.** The series cannot be backfilled, for the reason above. Nothing ties `floor.txt` to a
commit any more than it ties the lcov to one, so `--coverage-min-from` believes the caller about
which week it measured. The cell is carried across a `--backfill` like the aggregate, so after a
row is repointed at a later commit of the same week the minimum describes the earlier one until
someone re-measures. And the minimum is one file: two files at 81% and one at 81% draw the same
bar, which is what `floor.txt`'s band counts are for and the chart is not.

### The prose budget

calef ratified a prose budget on 2026-09-23 (UTC): 3,000 words of main body per document, enforced
as a ratchet, so a document already over may not grow and one under may not cross. The decision
section is proposed in pull request #1187 and has no number yet. A ratchet is invisible without a graph, which is why
this panel exists and why calef asked for it the same day he ratified the cap.

The first chart is the debt: how many words would have to move into appendices for the tree to meet
its own rule. The second is how many places that work sits in. Two panels rather than one, because
the series stack with nothing. They also move independently: splitting one long document cuts the
debt and leaves the count where it was.

Both are measured over every `.md` directly under `design/`, `design/decisions/`,
`design/roadmap/`, `notes/` and `briefs/`, plus `AGENTS.md`. Directly under, not recursively, which
is the scope the ruling's own evidence paragraph used. Words are whitespace-separated over the whole
file. Main body and whole file are the same number until a document here has an appendix.

Every document is counted, one carrying a marked exception included. The exception mechanism belongs
to the gate the ruling asks for, which does not exist yet. A chart that subtracted exempted
documents would hide the debt rather than measure it, and the debt is what the chart is for.

**BUGS.** A word cap rewards moving prose rather than cutting it, which the ruling says in its own
`BUGS` section. Every file can pass while the tree's total grows, and these two series will show
that as a falling debt against a rising document count. Neither chart measures whether anybody reads
the words. And the numbers here include `AGENTS.md`, while the ratified headline figures (569,775
words, 174 documents, 2026-09-23) were measured without it. `AGENTS.md` alone is 10,967 words, so
the two reconcile exactly at 7,967 words and one document.

### How the series stays current

`script/metrics --update` recomputes the current week's row and redraws the charts.
`.github/workflows/metrics.yml` runs it daily (calef asked for this on 2026-09-23) and opens or
refreshes a pull request if anything changed, following `toolchain-bump.yml`, which is the closest
existing shape. Coverage stays weekly, taken on Monday only: it is the one column that needs a
build, `script/metrics` carries the cell on every other day rather than recomputing it, and a daily
build would be seven `script/coverage` runs for a column nobody reads more than once a week. The
row is keyed by ISO week regardless, so the daily runs update the current week's row in place; they
do not add rows between Mondays.

**The data is one CSV per measure**, in the tree and versioned with the code it describes, so the
page renders the repository's own numbers rather than holding a copy of them that can rot. A
measure's file carries the weeks it has something to say about and no others, so a series that
began in 2026W39 starts there rather than trailing ten blank cells behind it.

#### One column the scheduled run cannot produce, and what stands in for it

**The workflow runs on a GitHub runner with a fresh checkout, daily. It cannot see the session
records the cost columns are measured from, and no workflow ever will.** Those records live under
`~/.claude/projects/` on the machine the lanes ran on. So machine effort is captured by a person
running `script/effort --update` on patagonia and committing the result, which is rung four of
`AGENTS.md`'s ladder, and rung four is where this tree's failures live.

Two mechanisms stand behind it rather than one, because the two ways this dies are different:

- **`script/effort --snapshot` makes forgetting survivable.** It reads the same records and writes
  the week's aggregate to a machine-local cache outside the repository, touching no file in the tree,
  so it is safe to run unattended while lanes are working. A week's numbers then outlive the
  transcripts they came from, and a month of nobody committing costs nothing. Under `launchd` beside
  the merge drain and the trunk watcher (`notes/merge-queue.md` has the plist shape), with
  `ProgramArguments` of `/bin/sh -c 'cd <checkout> && script/effort --snapshot'` and a
  `StartInterval` of 21600, four times a day is ample for a weekly figure.
- **`script/cadence-check` says when the capture has not happened.** It asks whether the committed
  `notes/project-metrics/effort.csv` holds the current week, and `scripts/trunk-health.sh` already
  calls it every five minutes on patagonia, which is the same machine that holds the records. It is
  the only row in that report that is not a workflow, for the reason above: there is no run history
  to interrogate, so the output is checked instead of the run.

**Neither of them invents a number.** A week that nobody captured stays absent, and the chart draws
it as absent. That is the whole design: this tree has twelve recorded instances of a missing failure
signal being read as a pass, and a cost series that quietly stopped updating would be the thirteenth.

`--update` refuses to lower a figure a past week already carries, and says which week and column it
refused. That is the shape of the failure this measurement actually has: transcripts get pruned, a
later run sees less, and the smaller number goes over the larger one without comment.

**It is idempotent, and that is a property of how the file is written rather than a check on top of
it.** The CSV is never appended to. It is read into a dictionary keyed by ISO week, the week being
written **replaces its own entry**, and the whole file is rewritten from that dictionary in sorted
week order with a fixed column order. So running it twice in one week produces the same bytes, and so
does a backfill against a history that has not moved. `script/metrics --check` is exactly "run it
into memory and compare", which is why it can be trusted to say whether the tree is current.

The representative commit for a week is the first-parent commit with the greatest committer date
inside it. First-parent because that is the trunk a reader means by "the tree on that date", and
committer date because a merge's author date can be days old and would file the merge under the week
its branch was cut. For a past week that answer never changes, which is the other half of the
idempotence; for the current week it is `HEAD`, and the row moves as work lands.

### A dated reading kept for the record: two days of movement in 2026W39

The `2026W39` row was last computed 2026-09-21, the morning the scheduled workflow's Monday run
merged it (`.github/workflows/metrics.yml`, `0 6 * * 1`). That is the automation working as
designed, not a fault: the row is refreshed weekly, on purpose, so it does not open the same
near-empty pull request six times in a row for a quiet week. The two days since were not quiet, so
`script/metrics --update` was run by hand on 2026-09-23 to catch the row up mid-week, which is the
mechanism this page already documents for exactly this case rather than anything new.

**The biggest mover is `merged_pull_requests`: 17 to 108, all verified against `git log
--first-parent` for the same range.** `milestones_built_this_week` went 2 to 16, `milestones_total`
521 to 566 (`milestones_built` 232 to 246, `milestones_not_started` 198 to 226; the backlog again
grew faster than it drained). `fatal_risks_tested` went 5 to 9 and `fatal_risks_untested` 4 to 0: all
nine risks in `design/fatal-risks.md` had a status line on record. Those two columns were replaced
later the same day by `fatal_risks_run`, `fatal_risks_not_run` and `fatal_risks_cannot_run`, for the
reason the chart's own section gives: a pair that reads nine and zero is a flat line. `names_total`
moved 234 to
244 (`names_provisional` 53 to 61) and `decisions_total` 203 to 207. `proposals_unnumbered` fell 17
to 2, most of that pile promoted or resolved rather than abandoned.

**`unsafe_trust_unclassified` reappeared: 0 to 16, and this is a data gap, not an instrument
change.** `scripts/rust_source.py` was not touched in this window; its hand-maintained crate tables
were. Four `crates/` directories landed in the last two days that none of `KERNEL_ONLY_CRATES`,
`USERSPACE_ONLY_CRATES`, `SHARED_CRATES` or `BOOT_CHAIN_CRATES` know about yet: `boot_slot`,
`current_cpu_protocol`, `file_allocation_table` and `top`. `current_cpu_protocol` carries the
unsafe blocks; the other three carry none. This is exactly the gap this page's unsafe-by-trust
section already names as a limitation of a table read once and baked in, showing up again the first
time new crates land after it was written. `unsafe_trust_boot_chain` also moved, 37 to 62, and that
one is real growth in `uefi_loader`/`sealed_pair`, not a classification artifact.

**The token and cost columns did not move, and that is deliberate, not an oversight.**
`lane_tokens`, `lane_wall_clock_hours`, `price_per_mtok_at_date` and `cash_spend` for 2026W39 are
byte-identical to the 2026-09-21 row: 506,140,600 tokens, 12.0 hours, $0.439/Mtok, $0.00 cash.
`notes/project-metrics/effort.csv` was not recaptured for this update, because the only way to
recapture it is to run `script/effort` from this very session, which would fold this session's own
token consumption into the week it is trying to report. So the effort chart's "million tokens per
milestone" line for 2026W39 fell from 253.1 to 31.6 in this redraw, and every bit of that fall is
`milestones_built_this_week` growing from 2 to 16 under an unmoved numerator, not tokens getting
cheaper. Read it as what it is: still the most understated cell on the page, now more so, because
two of its busiest days are not in the capture at all. `coverage_lines_pct` for 2026W39 is still
empty for the same reason it was empty on 2026-09-21: nothing here re-derives coverage, and this
update did not run `script/coverage`.

**One earlier week moved too, by the mechanism this page already documents rather than by
anything new.** `2026W31`'s `milestones_built_this_week` went from 20 to 21. That column is read
from today's tree for every week, on purpose (see "Milestones built each week" above), so a BUILT
date discovered since the last run can still move a week that closed seven weeks ago. `2026W38`'s
effort ratio moved from 67.8 to 65.7 for the same reason: its `milestones_built_this_week` corrected
from 62 to 64.

## EXAMPLES

### Adding a measure to the register

Take the unsafe census, from calef's question to a gated row, because every step of it went
differently than expected.

**1. Apply the test out loud.** Does anything depend on the amount of unsafe in this tree? Yes: the
whole demonstrator claim is a verified-Rust capability microkernel, and unsafe is where verification
stops. Does it move without anybody editing it? Yes, 42 non-merge commits changed it in fourteen
days. Both halves pass.

**2. Take the number, and take it more than once.** A single measurement cannot tell a direction
from a level, and here it inverted the answer:

```sh
# blocks outside kernel/src/arch/, at four points in the tree's history
2026-07-15   171 blocks in   7,508 lines   227.8 per 10,000
2026-08-04   728 blocks in  58,351 lines   124.8 per 10,000
2026-08-16   817 blocks in  73,129 lines   111.7 per 10,000
2026-08-18   747 blocks in  80,359 lines    93.0 per 10,000
```

The count more than quadrupled and the density more than halved. A ceiling on the count would have
fired on nearly every lane; a ceiling on the density holds a trend that is already going the right
way.

**3. Choose the relation from the shape of the quantity, not from taste.** Equality for a census
somebody maintains, `count-at-least` where more is better and a deletion is the bad event,
`count-at-most` where less is better and a drift up is. See notes/counted-claims.md.

**4. Watch it fail.** This is not optional and it is where the two real bugs were:

```sh
# add one `unsafe impl Send` anywhere, then:
$ script/lint
lint: a counted claim disagrees with the tree:
  notes/unsafe-obligations.md:461: claims at most 17, the tree has 18
  (unsafe-thread-safety-claims: how many `unsafe impl Send`/`Sync` claims the tree makes, each one
  a hand-written assertion that the compiler is wrong about a type). A ceiling is only wrong when it
  stops being true, so this means the count went UP by 1 past the headroom. Take the addition back
  out, or raise the ceiling in this commit and say beside it why the addition was worth it
```

The density ceiling's first marker **did not fire when it should have**, and the reason is the sort
of thing only a deliberate failure finds. It was written as `at most 91 blocks per 10,000 lines`,
and the convention binds a marker to the **last** number on the line, so the gate was comparing
10,000 against 92 and passing every time. The marker now sits immediately after its own number.

### Re-taking a dated measure

There is no wrapper and there should not be one: each dated row's command is in its table cell
because the commands are genuinely different animals, and a `script/measures` that ran all of them
would take an hour and be run by nobody. Copy the cell.

```sh
# the filesystem row, which needs a disk attached
script/bench --real --smp

# then edit the date in this file's table, in the same commit as the numbers
```

If the number moved, **the finding is the movement**, not the new value. Say what moved and against
what, in notes/benchmarks.md where the series lives, and leave this register holding only the date.

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

- **A `dated` row goes stale silently, which is the whole point and is also the limitation.** This
  register makes the staleness visible to a reader who opens the file; it makes it visible to
  nobody else. Nothing checks that a date is recent, and a check that did would be asserting a
  policy nobody has set. If a row's staleness starts to matter, the fix is to promote it to
  `gated`, not to add a freshness gate.

- **The register is a ratchet, like the convention it extends.** A measure nobody adds is not
  tracked, and "the register is complete" is never a thing anybody can say. It grows as people
  notice numbers, which is the same honest boundary `notes/counted-claims.md` records.

- **`patches/std-nife/overlay/` is outside the unsafe census, and it is our code.** Thirty-seven
  `unsafe {}` blocks in the `std` platform layer are counted by nothing here. Two separate reasons,
  and only the first is a decision: a ceiling asserts a direction, and that code implements `std`'s
  internal interfaces, so it cannot be restructured to hold fewer unsafe blocks without diverging
  further from the crate we track. The second is worse and is not a decision at all: **that code is
  compiled into `std` by the farm and never by a clippy configuration here**, so
  `undocumented_unsafe_blocks` and `unsafe_op_in_unsafe_fn` do not reach it either. Fifteen of its
  blocks have no `SAFETY:` comment in the form the lint wants, and nothing has ever said so. That
  is a coverage hole in the lint policy rather than a gap in this register, and it wants a lane.

- **Unsafe density can be diluted by writing more safe code, and nothing stops that.** The
  denominator is non-blank lines after comments and string literals are stripped, so prose cannot
  move it, but a verbose safe refactor can. The counter-argument is that the effect is small at
  80,000 lines and that the alternative, a raw count, was measured and is worse. Watch the printed
  numerator, which `script/lint` prints beside the ratio for exactly this reason.

- **The unsafe derivation is a text scanner, not a parser.** It blanks comments and literals with a
  regex before matching keywords, which is what keeps fourteen `unsafe {}` written inside doc
  examples out of the count. Block comments are matched non-greedily and Rust's nest; the tree has
  no nested ones, and a nested one could only make the count too high, which fails loud. Same caveat
  as `script/lint`'s `# Safety`, dead-code and `#[path]` checks, which are built the same way.

- **Nothing here measures the verification argument, and nothing can.** Unsafe density says how much
  code is outside the compiler's guarantees; it says nothing about whether the invariants written in
  the `SAFETY:` comments are true. §61 already records that a lint checks a comment exists and never
  that it is right. A register of numbers is not a substitute for reading them.

- **The gated and dated rows are maintained by hand.** Nothing checks that
  `script/fastpath-footprint` still exists or that `script/bench --real --smp` is still the command,
  which makes this document exactly the class of artifact it was written to complain about, one
  level up. The mitigating fact is that `script/lint` already fails when a script in `script/` has
  no entry in notes/scripts.md, so a renamed instrument cannot vanish quietly from the tree, only
  from this table.
