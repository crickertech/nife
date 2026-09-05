# The multi-tasking workload benchmark, and how to take its number

*(Milestone 168. Names in this page are **provisional**, per the naming tenet; calef names things.)*

**Nothing in this page has run on radon.** It was written on 2026-09-04 with the board powered off
and no bench session available, which is the same condition `notes/x86-uefi-boot.md` and
`notes/soak.md`'s rebooting-soak section were written in, and the same reason the procedure below is
as detailed as it is. Every claim here is either about code in this tree, which was built,
host-tested and rehearsed under QEMU on all three architectures, or is a question for the bench,
which is marked as one.

## What this instrument is for, in one paragraph

`design/decisions/96-process-kernel-or-event-kernel.md` asks whether this kernel should keep a kernel
stack per thread (a **process kernel**, what it is) or move to one stack per core with explicit
continuations (an **event kernel**, what seL4, OKL4 and NOVA all became). Three of its four inputs
are settled by measurement. The fourth is performance, and the retrospective it rests on says
precisely where the difference lives:

> Warton [2005] performed a thorough performance evaluation of the Pistachio process kernel vs an
> event-based (single-stack) kernel with continuations on an ARMv5 processor. He demonstrated
> comparable performance, generally within 1% on micro-benchmarks but a 20% performance advantage of
> the event kernel on a multitasking workload (AIM7).

Elphinstone and Heiser, *L4 Microkernels: The Lessons from 20 Years of Research and Deployment*, ACM
Transactions on Computer Systems 34(1), article 1, April 2016, section 4.1. Read on **2026-09-04**
from <https://trustworthy.systems/publications/nicta_full_text/8988.pdf>.

**Every instrument this project owns is on the left of that sentence.** `ipc_rtt`, `ipc_rtt_el0`, the
icount tripwire, milestone 132's footprint gate, and milestone 134's E1 through E4 are all
single-operation or single-shape measurements. This is the one that is not.

### Two things about the citation that are worth knowing before quoting it

**The 20% is the retrospective's summary of Warton, not Warton's own claim.** His thesis abstract
(read the same day from
<https://trustworthy.systems/publications/theses_public/05/Warton:be.abstract>) says only that
"significant memory savings can be achieved without degrading the kernels performance" and that
"preliminary results show improvement in the kernels performance due to the single stack
architecure, however more experiments are required to verify this result." The stronger, specific
figure appears in the 2016 retrospective. Both are real citations; they are not the same strength of
claim, and this tree has already carried a fabricated block quote for twelve days, so the provenance
is written down rather than remembered.

**The result is from 2005, on ARMv5, on Pistachio.** Nothing about it transfers to this kernel by
assumption. That is the whole reason this milestone exists rather than a rewrite.

## What the workload is, and what it is not

`crates/job_mix`'s header is canonical and this page does not repeat it. The short version:

AIM7 (read on 2026-09-04 from the benchmark's own README at
<https://github.com/davidlohr/areaim/blob/master/osdl-aim-7/_NOTICES/README.aim7>) forks many
**tasks**, each running **in random order** a set of subtests called **jobs**, over a sequence of
**subruns** with the task count incremented between them, reporting **jobs per minute** against task
count. Four methodological properties do the work, and this instrument keeps all four: heterogeneity,
per-task ordering, a task-count sweep, and a throughput metric.

**It keeps none of AIM7's 53 jobs**, which name Unix services this system does not have and should
not grow in order to be measured. Five jobs stand in for AIM7's categories: a compute grind, a
32 KiB working-set walk, a null syscall, a yield burst, and a `CALL`/`REPLY` round trip against a
shared server. `crates/job_mix`'s `BUGS` records the three categories that are still missing (file
operations, process creation, page mapping) and why each was refused.

**So this is not AIM7 and no number from it is comparable with an AIM7 number.** It is an
AIM7-*shaped* instrument for a capability microkernel.

## Rehearsing it, which needs no board

```sh
script/job-mix                          # aarch64
script/job-mix --arch riscv64 --smp 4   # radon's architecture and core count
script/job-mix --arch x86_64            # xenon's
```

Each takes a few minutes under TCG and prints the placement census, three repeats per subrun, and
one summary line per sweep point. **The magnitudes are fiction** (TCG models no cache) and the
rehearsal exists to prove the mechanism, not to produce a result.

Rehearsed on 2026-09-04, on patagonia, all three architectures, sweep complete on each.

## The bench procedure on radon, in order

The steps below are new only where they have to be. Everything about writing a card, attaching a
console and getting U-Boot to hand over is `notes/bench-runbook.md`'s and `notes/visionfive2.md`'s,
and repeating it here would give it somewhere to drift to.

### 1. Build the card

```sh
script/board-image --jobmix --card /Volumes/NIFE
```

**The archive is not optional** and a mismatched pair halts at `MEASURED BOOT REFUSED`, which cost a
boot on 2026-09-01; `--card` copies all three files as a set for that reason (milestone 217).

`--jobmix` and `--soak` are refused together: both replace the end of the boot tour, and a card built
from an ambiguous command is a card nobody can reproduce.

### 2. Attach the console before power

```sh
script/board-console --for 30m --until none --log target/radon-jobmix-$(date +%s).log
```

115200 8N1, a WCH CH343 at `/dev/cu.usbmodem*` on patagonia. `--until none` because this run ends by
halting rather than by reaching a stage the console recognises: read the log for `jobmix: done`.

**The console has no recogniser for this run**, which is a deliberate limitation and not an
oversight; see this page's `BUGS`.

### 3. Power on, and read the census before anything else

The first thing worth looking at is not a number, it is the arrangement:

```text
jobmix-census: core=0 threads=8 T2 T11 T13 ...
jobmix-census: core=1 threads=10 S1 T0 T7 ...
```

**This is the load-bearing step of the whole procedure.** `notes/soak.md` records four runs on this
board whose rates span **fifteenfold**, and milestone 240's census explains them: the rate tracks
how the boot-time placement lottery landed. A jobs-per-minute figure recorded without the census
that produced it is not a measurement, it is a draw.

### 4. Record the sweep

Six lines, one per subrun:

```text
jobmix: tasks=1 jobs=128 ticks=... jpm=...
...
jobmix: tasks=32 jobs=4096 ticks=... jpm=...
```

Record all six, plus the `jobmix-repeat:` lines behind them (the spread between repeats is
information on a board where nothing else is running, and the summary throws it away), plus the
census, plus the `cntfrq` from the started line.

### 5. Do it again, at least five times, power-cycling between

**One boot is one draw.** Five to ten boots is the minimum that says anything, and it is the same
argument milestone 249 makes for the rebooting soak. If the spread across boots is larger than the
shape of the curve within a boot, the curve is not the finding and the lottery is.

Nothing can power-cycle radon remotely (milestone 224): its Kasa KP303 answers the vendor app and is
invisible to ARP from both patagonia and cordoba, so this is a person at the bench.

## What each outcome means

| What the log shows | What it means | Where it routes |
|---|---|---|
| `jobmix: FAILED: no 'job_mix_task' program in the initrd archive` | the card carries a kernel and an archive from different builds, or an archive built before this milestone | rebuild with `script/board-image --jobmix --card ...`, which packs the archive before the kernel for exactly this reason |
| `jobmix: FAILED: could not spawn task N of 32` | the board ran out of memory or thread slots partway through building the pool | a real finding: `job_mix::MAX_TASKS` is 32 against `sched::MAX_THREADS`'s 256, so this is memory. Record N and reduce `MAX_TASKS` |
| the census, then nothing, ever | a task or a server wedged before the first subrun finished | the hang case. `crates/job_mix`'s `ROUND_TRIP` job is the only one that blocks on another process; a wedged echo server looks exactly like this |
| `jpm` roughly flat across the whole sweep | this machine's scheduling is not the bottleneck at 32 tasks | **the honest negative**, and it is a result: §96's performance argument does not bite at this scale on this silicon |
| `jpm` rising and then falling, with a knee | throughput collapsing under task count | the positive result. Record where the knee is and compare it against milestone 134's E1 knee (8 to 11% by 64 to 96 threads on the dev Mac) |
| `jpm` varying more between boots than across the sweep | the placement lottery dominates | not a result about §96 at all. More boots, and read `notes/soak.md`'s milestone 240 section |
| `jobmix: done` and six clean points | the sweep ran | record it in this page's own table, below, and in `notes/register-of-measures.md`'s dated row |

## Results

**None.** No run of this instrument on real silicon has happened. This section exists so that the
first one has somewhere to go that is not a pull request body, and so that its absence is visible
rather than inferred.

| Date | Machine | Boots | Census summary | jpm at 1 / 32 | Notes |
|---|---|---|---|---|---|
| | | | | | |

## What this instrument cannot settle, said plainly

**It does not compare a process kernel against an event kernel, and it cannot**, because there is no
event kernel here to compare against. It measures what *this* kernel does under multi-tasking load.
That is the input §96 says it is missing, and §96 stays open either way: a flat curve says the cost
is not visible at this scale on this silicon, which is a reason not to spend a pervasive rewrite; a
knee says there is something to price, and pricing it would then want the rewrite prototyped on one
path rather than the whole kernel.

## BUGS

- **`crates/board_console` has no recogniser for this run**, so `script/board-console` cannot tell a
  finished sweep from a wedged one and the operator reads the log. Adding a `Stage` for it was
  refused in this lane: the console's recogniser is a small piece of shared judgment that the soak
  and the boot sequence both depend on, and growing it for a run nobody has taken yet would be
  guessing at what the failure modes are. Proposed as follow-on work in this milestone's block.
- **There is no committed baseline and no `--check`.** `script/bench` gates because its icount counts
  are deterministic; a sweep whose entire subject is scheduling under contention is not, on any
  accelerator this tree has. A gate here would be asserting a tolerance nobody has measured.
- **The sweep tops out at 32 tasks**, where milestone 134's E1 reaches 96 threads. E1's were kernel
  threads; these are processes, each with an address space and a loaded image, and the memory is
  what binds. **If the knee is above 32 this instrument will not see it**, and the honest response is
  to record that rather than to raise the number blind: a `FAILED: could not spawn` line is the
  measurement that says how far it can go on a given board.
- **The pool is built once and released in slices**, where AIM7 forks a fresh set per subrun, so the
  parked tasks' kernel stacks exist during every subrun even though nothing touches them.
  `kernel/src/jobmix.rs`'s own header argues why that is the right choice for §96's question and why
  it is still a departure from AIM7.
- **The mix proportions are chosen, not derived.** AIM7 ships workfiles for four machine roles and
  nobody here has one for a capability microkernel. A different mix gives a different number, and no
  result from this instrument should be quoted without saying which mix produced it.
- **This page has not been followed end to end by its author**, the same caveat
  `notes/bench-runbook.md` carries about every procedure it points at.
