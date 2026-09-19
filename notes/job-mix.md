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

**Since 2026-09-13 the thesis body has been read too, and it is harder on the number than the
abstract is**: Warton expected the opposite result, called his own 20% something to treat "with
scepticism until it can be satisfactorily explained", and never ran the cache simulation that would
have explained it. The next section has the quotations.

**The result is from 2005, on ARMv5, on Pistachio.** Nothing about it transfers to this kernel by
assumption. That is the whole reason this milestone exists rather than a rewrite.

## What Warton ran AIM7 on, and why the 20% is not a target

Checked on **2026-09-13**, because AGENTS.md's fourth question is *is the premise true?* and nobody
had asked what the 20% was measured **on**. It took two documents, and the second one settles it.

### The retrospective does not say, and that is worth knowing before quoting it

Read from <https://trustworthy.systems/publications/nicta_full_text/8988.pdf> (629,888 bytes, 30
pages). "AIM7" occurs **exactly once** in the paper, in the sentence this page quotes above. The
whole setup it gives is section 4.1, page 1:16: *"the Pistachio process kernel vs an event-based
(single-stack) kernel with continuations on an ARMv5 processor."* Two arms and an ISA. **The
software above the kernel is not named**, and "Wombat" appears in the paper only as a bibliography
entry cited from section 5.1 (Virtualisation), unconnected to Warton's measurement.

### Warton's own thesis says, and the answer is Wombat

<https://trustworthy.systems/publications/theses_public/05/Warton%3Abe.pdf>, read the same day
(361,145 bytes, 43 pages). Matthew Warton, *Single Kernel Stack L4*, BE thesis, UNSW, submitted
2 November 2005, supervisor Gernot Heiser. Section 5.4:

> The AIM7 benchmark is a measures system performance by simulating workload on a multiuser system.
> The AIM7 benchmark was modified slightly so that it could run on Wombat. The modifications included
> disabling the network operation simulations because Wombat does not support the GetHost function,
> and disabling the file system operation simulations, because Wombat runs from a ram disk, and the
> ram disk is not large enough to support the benchmarks.

(`pypdf` drops this PDF's `fi` ligature, so "modified", "file", "benefits" and "conflict" arrive
from the extractor with the `fi` missing. They are restored in every quotation on this page, since
the ligature is an artifact of reading the document rather than of writing it. Nothing else in any
quotation here is altered.)

**Wombat is the paravirtualised ARM Linux**, the same one the retrospective cites at Leslie, van
Schaik and Heiser 2005. So the 20% is a delta between two microkernels **measured through a hosted
Linux's syscall path**. Four things follow, and each one costs this instrument something different.

**1. The premise fails.** `crates/job_mix` runs native tasks on nife's own primitives. Warton
measured Linux processes whose every system call became an exception IPC to a user-level Linux
server. Those are different experiments, and no fidelity work on the mix turns one into the other.

**2. The 20% is a ratio, and this tree has one kernel.** Section 6's table, five runs:

| | single stack | multi stack | variable stack |
|---|---|---|---|
| average time | 157.69 | 197.22 | 157.78 |
| standard deviation | 0.015 | 0.344 | 0.019 |

(197.22 − 157.69) / 197.22 is 20.0%, which is where the retrospective's figure comes from. It is a
**paired** measurement: neither column means anything alone. nife has no event kernel to be the
other column, so the mix produces one arm and no ratio, whatever jobs it contains.

**3. Warton's AIM7 run had already disabled two of the three categories `crates/job_mix`'s `BUGS`
apologises for.** Section 5.4 turned off the filesystem jobs (the ramdisk was too small) and the
network jobs (Wombat had no `GetHost`). The crate records a missing disk-file category as a fidelity
gap against AIM7; **the AIM7 run being cited did not have one either.**

**4. It was two tasks, and there was no sweep.** Section 5.4: *"The precise benchmark used was 2
clients with the normal workload file, with the disk and network tests removed"*, and section 6:
*"This workload was run in two user tasks."* This page lists a task-count sweep as one of AIM7's
four methodological properties and keeps it out to 32 tasks. **The run the 20% comes from swept
nothing.** The sweep is a good idea on its own merits and it is not a reproduction of Warton.

### What Warton thought of his own number

This is the part the retrospective compresses away entirely, and it is the reason the figure should
never be quoted flat. He expected the opposite result (section 5.2: *"the single stack kernel is
expected to perform similarly to or worse than the multi stack kernel"*), and the micro-benchmarks
delivered it. Section 7.3, on the AIM7 result:

> As with any experimental result, this needs to be treated with scepticism until it can be
> satisfactorily explained. Because the benchmark is not very stable and crashes on some runs, I
> initially doubted the results.

He ruled out the timer by re-running against a wall clock. Then:

> If the result is accurate, it must be due to reductions in the cache and TLB footprint of the
> single stack kernel. I did not expect that this reduction would make such a massive difference in
> performance [...] To determine the validity of this result a simulation of the cache impact of
> the kernels must be performed. There was not enough time to complete this simulation in the
> course of this thesis, due to external events.

**The validating simulation was never run**, and section 7.4 asks for another macro-benchmark for
the same reason. So the chain this project has been reasoning from is: a BE thesis reports an
unexplained 20% its author flagged as needing scepticism, on a modified AIM7, on two tasks, on a
hosted Linux; a retrospective eleven years later summarises it as a flat *"20% performance
advantage of the event kernel on a multitasking workload (AIM7)"*; and this tree built an instrument
to chase it. Every step is a real citation. The compression happened at the second one.

### The hardware, for completeness

Section 5.2: a littlechips LN2410SBC single-board computer with *"a Samsung S3C2410 arm processor
clocked at 200 MHz, a 32 kilobyte, 64 way associative cache, and 64 megabytes of ram."* The thesis
never says "ARMv5"; that is the retrospective's own gloss on the SoC. Note the cache: Warton
expected *"not many caching benefits [...] due to the single kernel stacks reduction of conflict
misses"* precisely because 64-way associativity makes conflict misses rare, which is what made the
result surprising to him.

### What survives, and it is the reason to keep the instrument

**The mechanism, not the number.** The only explanation Warton offered for his 20% is kernel cache
and TLB footprint, which is exactly the quantity `kernel/src/bench.rs`'s `app_displacement` and
milestone 134's E1 measure. Whether per-thread kernel stacks displace enough cache to cost
throughput **on this kernel** is a real, open, local question, and a knee in jobs-per-minute against
task count answers it. That is a genuine input to
`design/decisions/96-process-kernel-or-event-kernel.md`. A reproduction of 20% was never available
and is not what this instrument was ever going to deliver.

### One gloss in §96 that the paper contradicts in the same sentence

§96 twice discounts the memory argument with the clause *"Warton's result came from resource-starved
embedded systems"*. The paper's own sentence, section 4.1, page 1:17, is:

> While this decision was driven initially by the realities of resource-starved embedded systems and
> later the needs of verification, the approach's benefits are not restricted to those contexts, and
> we believe it is generally the best approach on modern hardware.

The first half is where §96's clause comes from and it is fair: the **move** was driven by
resource-starved embedded realities. The second half rejects the inference §96 draws from it, in the
same sentence, and §96 does not record that it exists. The authors are not neutral on this and the
tree should quote them rather than paraphrase them into agreement.

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
script/board-image --job-mix --card /Volumes/NIFE
```

**The archive is not optional** and a mismatched pair halts at `MEASURED BOOT REFUSED`, which cost a
boot on 2026-09-01; `--card` copies all three files as a set for that reason (milestone 217).

`--job-mix` and `--soak` are refused together: both replace the end of the boot tour, and a card built
from an ambiguous command is a card nobody can reproduce.

### 2. Attach the console before power

```sh
script/board-console --for 30m --until none --log target/radon-job-mix-$(date +%s).log
```

115200 8N1, a WCH CH343 at `/dev/cu.usbmodem*` on patagonia. `--until none` because this run ends by
halting rather than by reaching a stage the console recognises: read the log for `job-mix: done`.

**The console has no recogniser for this run**, which is a deliberate limitation and not an
oversight; see this page's `BUGS`.

### 3. Power on, and read the census before anything else

The first thing worth looking at is not a number, it is the arrangement:

```text
job-mix-census: core=0 threads=8 T2 T11 T13 ...
job-mix-census: core=1 threads=10 S1 T0 T7 ...
```

**This is the load-bearing step of the whole procedure.** `notes/soak.md` records four runs on this
board whose rates span **fifteenfold**, and milestone 240's census explains them: the rate tracks
how the boot-time placement lottery landed. A jobs-per-minute figure recorded without the census
that produced it is not a measurement, it is a draw.

### 4. Record the sweep

Six lines, one per subrun:

```text
job-mix: tasks=1 jobs=128 ticks=... jpm=...
...
job-mix: tasks=32 jobs=4096 ticks=... jpm=...
```

Record all six, plus the `job-mix-repeat:` lines behind them (the spread between repeats is
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
| `job-mix: FAILED: no 'job_mix_task' program in the initrd archive` | the card carries a kernel and an archive from different builds, or an archive built before this milestone | rebuild with `script/board-image --job-mix --card ...`, which packs the archive before the kernel for exactly this reason |
| `job-mix: FAILED: could not spawn task N of 32` | the board ran out of memory or thread slots partway through building the pool | a real finding: `job_mix::MAX_TASKS` is 32 against `sched::MAX_THREADS`'s 256, so this is memory. Record N and reduce `MAX_TASKS` |
| the census, then nothing, ever | a task or a server wedged before the first subrun finished | the hang case, and since milestone 324 the tool says so rather than leaving it to the operator: `script/job-mix` and `script/board-console --until sweep-done` both exit **2**. `crates/job_mix`'s `ROUND_TRIP` job is the only one that blocks on another process; a wedged echo server looks exactly like this |
| `jpm` roughly flat across the whole sweep | this machine's scheduling is not the bottleneck at 32 tasks | **the honest negative**, and it is a result: §96's performance argument does not bite at this scale on this silicon |
| `jpm` rising and then falling, with a knee | throughput collapsing under task count | the positive result. Record where the knee is and compare it against milestone 134's E1 knee (8 to 11% by 64 to 96 threads on the dev Mac) |
| `jpm` varying more between boots than across the sweep | the placement lottery dominates | not a result about §96 at all. More boots, and read `notes/soak.md`'s milestone 240 section |
| `job-mix: done` and six clean points | the sweep ran | exit **0**. Record it in this page's own table, below, and in `notes/register-of-measures.md`'s dated row |

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

- **The sweep has no wall-clock heartbeat, so a watcher's wedge timer is a guess with headroom.**
  This entry replaces *"`crates/board_console` has no recogniser for this run"*, which was true when
  this page was written and stopped being true on **2026-09-19** (milestone 324 part 2): the
  recogniser has `Stage::Sweep` and `Stage::SweepDone`, reading `crates/job_mix`'s own marker
  constants, and `script/job-mix` judges with it and returns `script/board-console`'s five exit
  statuses. What that milestone could not fix is the thing that makes a sweep harder to watch than a
  soak. `kernel/src/soak.rs` prints every five seconds whatever the workload is doing, so a missed
  beat is a missed deadline; `kernel/src/job_mix.rs` prints only when a subrun ends, so the longest
  legitimate silence is the slowest subrun and a watcher has to allow for it. The default is sixty
  seconds against a 2.6-second subrun measured under TCG, twenty to one, and a board slower than
  that reads as wedged when it is merely slow. `--quiet-after 0` is the escape and it gives up the
  detection. A heartbeat in the supervisor is the real fix and is a kernel change; milestone 324's
  block records it as follow-on.
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
  `kernel/src/job_mix.rs`'s own header argues why that is the right choice for §96's question and why
  it is still a departure from AIM7.
- **The mix proportions are chosen, not derived.** AIM7 ships workfiles for four machine roles and
  nobody here has one for a capability microkernel. A different mix gives a different number, and no
  result from this instrument should be quoted without saying which mix produced it.
- **This page has not been followed end to end by its author**, the same caveat
  `notes/bench-runbook.md` carries about every procedure it points at.
