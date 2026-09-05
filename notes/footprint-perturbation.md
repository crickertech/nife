# Taking a benchmark on radon: E1, E3 and E4 on the small-cache board

*(Milestone 134. Named by calef on 2026-09-04, from three candidates. `board-bench.md` was refused
for a collision the file itself could not fix: `notes/bench-runbook.md` already exists and answers
"which machine should an evening be spent on", so two pages about benching on boards would have had
names that do not tell them apart. This one is named for the experiment, which is milestone 134's own
word for E3, so a reader looking for "does the fastpath's size cost anything" finds it. That it is a
board procedure is the first line's job rather than the filename's.)*

**Nothing in this page has been run on silicon.** radon was powered off and there was no bench
session on the day it was written. What exists is the instrument, built and compiled for the board,
plus the procedure and the reading. Every number quoted below is either a static footprint measured
on patagonia with `objdump` and no emulator at all, or a figure from the 2026-08-22 dev-Mac runs,
labelled as such. **The board columns are empty on purpose**, and filling them is the point of the
session this page is written for.

## Why this page exists: an instrument aimed at the wrong machine

Milestone 134's E3 pads the IPC fastpath with resident dead code and asks whether latency moves. Its
own block states the asymmetry that makes it worth re-taking:

> A *positive* result on the dev Mac is conclusive: if padding hurts on a machine with large caches,
> it certainly hurts on a 32 KB L1i. A *negative* result on that machine proves little, because an
> M-series core may simply absorb the whole path.

E3 ran on patagonia on 2026-08-22 and read 2 to 3% between the padded and un-padded builds, inside
run-to-run noise. That is the weak direction. E1 (IPC latency against thread count) and E4
(application displacement) are in the same position: both found small, direction-consistent effects
that their own notes attribute to the dev Mac's large L1d muting the knee.

**radon is the machine all three were designed against.** A StarFive VisionFive 2, four SiFive U74
cores, **32 KB L1i and 32 KB L1d each** (notes/benchmarks.md's machine table). E1's prediction was
computed against that number by name. Until 2026-09-04 none of the three could run there.

## What was in the way, and what closed it

Three separate things, and only the first was obvious.

| the obstacle | why it blocked | what closed it |
|---|---|---|
| E1 and E4 were `#[cfg(target_arch = "aarch64")]` | this tree has no riscv64 *accelerator* with a real cache, so they were gated on the accelerator rather than on the cache | the cfgs are `any(aarch64, riscv64)`; the TCG self-skip is now a per-arch counter frequency (10 MHz on QEMU `virt`, 4 MHz on the JH7110) rather than an arch-specific one |
| both require **one hart**, and a card has no `-smp 1` | radon seats four U74s, so a `board,bench` card would boot four and both benchmarks would print a skip line on the machine they were built for | the `single_hart` kernel feature: `smp::bring_up_secondaries` marks the boot core online and starts nobody |
| the padding was reachable only from `ipc_send` | milestone 188 phase 1 split the footprint gate into two closures and found the CALL/reply one is larger **and is the shape real services run**; the pad landed on the other one | `ipc_call` calls `maybe_pad` too |

**That third row is a finding, not a chore, and it is the reason this lane touched the kernel at
all.** Measured on riscv64 before the fix, `--features fastpath_pad` moved `ipc_send_recv` to
**2.10x** and `ipc_call_reply` to **1.00x**. E3 as built was padding a shape nothing in this tree
runs. It was correct when it was written: the split did not exist on 2026-08-22, and `ipc_fastpath`
was one number. Anyone who had taken E3 to the board in the four days after milestone 188 landed
would have measured the padding of a path their own benchmark barely uses.

## The static half, which needs no board and is already taken

`script/fastpath-footprint` is `objdump` over the built kernel. Run on patagonia, 2026-09-04, with
`ipc_call` padded:

| ISA | shape | un-padded | padded | ratio |
|---|---|---|---|---|
| riscv64 | `ipc_send_recv` | 4,632 | 9,726 | 2.10x |
| riscv64 | `ipc_call_reply` | 5,936 | 11,070 | 1.86x |
| aarch64 | `ipc_send_recv` | 5,356 | 11,192 | 2.09x |
| aarch64 | `ipc_call_reply` | 7,028 | 12,860 | 1.83x |

**Read the riscv64 `ipc_call_reply` row against 32 KB.** 5,936 bytes is 18% of radon's L1i; 11,070
is 34%. The padded build still fits, which is exactly the condition that makes the experiment
interesting rather than trivial: this is a footprint change large enough to matter under contention
and small enough that a naive "does it still fit" reading predicts no effect at all.

`ipc_fastpath` is the max of the two shapes, so it reads 1.86x on riscv64 rather than the 2.00x the
2026-08-22 run recorded against the old single-closure number. That is the same padding measured
against a larger denominator, not a weakening of the pad. "Roughly double", which is the block's own
wording, still holds.

**x86_64 is not padded on either shape**: `kernel/src/arch/x86_64/fastpath_pad.rs` does not exist.
Recorded in `script/fastpath-footprint`'s own BUGS and unchanged by this lane; xenon has no first
light, so nothing is waiting on it.

## What you need at the bench

Everything `notes/visionfive2.md`'s bench runbook and `notes/board-console.md` already list, and
nothing more:

- radon, DIP switches on QSPI, powered from its own Kasa outlet (**smart plug 2**; plug 3 is garcia
  and must never be switched off).
- The USB TTL adapter on the 40-pin header, TX/RX crossed, 3.3 V, `/dev/cu.usbmodem*` (`cu.`, never
  `tty.`).
- A microSD card already formatted and mounted, and its mount path.
- patagonia, this checkout, and about ten minutes of building per card.

**Two cards, or one card written six times, and this page said something looser until 2026-09-04.**
E3 is a comparison of two builds differing in exactly one Cargo feature. It said "writes the card
twice and boots twice", which describes a *blocked* order, and the section below requires an
**interleaved** one. With two cards those agree. **With one card they do not**, and calef has one.

So, with a single card, the order is:

```
unpadded -> padded -> unpadded -> padded -> unpadded -> padded
```

**six writes, not two.** A rewrite is about two minutes once the build is warm (`script/board-image`
rebuilds only what changed and the copy is 9 MB), so the session costs roughly 75 minutes rather
than 60.

**Do not take the blocked order to save the flashes.** Three unpadded boots followed by three padded
ones puts everything that drifts across the session, board temperature most obviously, entirely on
the second group, where it is indistinguishable from the effect being measured. The interleaving is
not tidiness; it is what makes a few-percent difference mean anything.

They produce the same three filenames, so nothing on the card says which is which;
`script/board-image` echoes its feature list for exactly this reason and that line belongs in the log
beside the numbers. **That the card cannot say what it is, is the reason six writes are risky rather
than merely slow**, and `design/roadmap/proposals/a-boot-banner-that-names-the-build.md` is the fix.

## The procedure, in order

### 1. Take the static numbers first, on patagonia, with no board

```sh
script/fastpath-footprint --arch riscv64
script/fastpath-footprint --arch riscv64 --features fastpath_pad
```

Two minutes, no hardware. It proves the padding still doubles what it claims to on the exact commit
about to be flashed, which is the one thing that would silently invalidate the whole session. Copy
both `ipc_call_reply` figures into the log.

### 2. Build and write the un-padded card

```sh
script/board-image --bench --card /Volumes/NIFE
```

Confirm the line it prints:

```
  features: board,bench,single_hart
```

`single_hart` is not optional and not a tuning knob. Without it the kernel boots four U74s, and E1
and E4 both print `skipped (needs a single hart ...)`. Its cost is stated where a reader meets it:
`smp_throughput`, `fs_read` and `fs_throughput` self-skip on this card. That is a fair trade for
this session and a bad one for any other, which is why it is a flag.

### 3. Boot it and capture everything

```sh
script/board-console --for 20m --until none --log target/radon-bench-unpadded-$(date +%s).log
```

Then power radon on. `--until none` because the bench boot prints a couple of dozen rows over
several minutes and then halts; there is no single banner worth stopping at, and a deadline that
fires mid-sweep loses the run. Twenty minutes is generous on purpose. The last line is
`bench: done`, and it is what says the suite finished rather than faulted.

**Do not power-cycle to "hurry it along".** E4 alone runs 5 working sets x 3 load conditions x 5
repeat batches, and E1 sweeps 7 pair counts with 4 repeats each.

### 4. Repeat the boot, unchanged, at least three times

Same card, same image, power-cycle between. This is the run-to-run distribution, and every
comparison below is against it rather than against a single pair of numbers. The 2026-08-22 dev-Mac
session found this the hard way: an unrepeated E4 swung 2 to 3x between nominally identical
conditions.

### 5. Build the padded card and repeat steps 3 and 4

```sh
script/board-image --bench --extra-features fastpath_pad --card /Volumes/NIFE
```

Confirm `features: board,bench,single_hart,fastpath_pad`. Log to a filename that says `padded`.

**Interleave the boots if the session has time**: unpadded, padded, unpadded, padded. Anything that
drifts over a session (ambient temperature, a card that is warming up) then lands on both conditions
instead of on whichever was measured second.

### 6. Read the rows

| row | experiment | what it is |
|---|---|---|
| `bench: ipc_rtt <ticks> 1000` | E3 | kernel-side round trip, the SEND/RECV shape |
| `bench: ipc_rtt_el0 <ticks> <iters>` | E3 | the same crossing EL0, which is what lmbench measures |
| `bench: call_reply <ticks> 1000` | E3 | **the CALL/reply shape, the one services run** |
| `bench: ipc_scale_<threads> <ticks> <iters>` | E1 | 7 rows, 2 to 96 threads |
| `bench: appdisp_<kib>k_solo/_ipc/_ipc96` | E4 | 15 rows, 5 working sets x 3 load conditions |
| `bench-probe: appdisp_<kib>k_*_lost_pct` | E4 | the derived percentages, printed so nobody has to divide |
| `bench: cntfrq 4000000` | all | **the proof this ran on the board.** 10,000,000 is QEMU `virt` |

That last row is the one to check before reading any other. A capture reading `cntfrq 10000000` is
an emulator, and E1 and E4 will have self-skipped in it.

**`call_reply` is the row E3's verdict should rest on**, not `ipc_rtt`. The padding now lands on both
shapes; the CALL one is what a service issues, and milestone 188's phase 4 decision is about that
path. `ipc_rtt` is kept because it is the row every previous E3 reading used and dropping it would
break the comparison with 2026-08-22.

## Controlling for the thing that has already burned this project

**Placement decides throughput on radon by up to fifteenfold** (milestone 240, notes/soak.md): four
soak runs on the same card spanned that range, and the census explains them by how many cores held
an IPC thread and no grinder. A latency number from a single boot is a draw from that distribution.

**A `single_hart` card removes the lottery rather than controlling for it, and that is the strongest
form available.** There is one core, so there is no spawn placement to draw, no work stealing, and
no migration; `pick_spawn_target` has one answer. The fifteenfold spread cannot be reproduced on this
card because the mechanism that produces it is not present.

**That is not the same as saying the boots are identical**, and the remaining variation is why step 4
repeats them anyway:

- DRAM training and the U74's own cold state differ boot to boot.
- The archive is measured at boot, the heap is laid out fresh, and where E1's 96 thread stacks land
  relative to each other is a property of one boot's allocator history.
- Nothing here is pinned to a hart *number*: OpenSBI's boot hart is not guaranteed to be 0, so a
  given boot runs on whichever hart it woke on. All four U74s are the same core, but they are not
  the same silicon.

So: **three boots minimum per condition, interleaved, and report the spread rather than a mean.**
An E3 effect smaller than the boot-to-boot spread is not an effect, and saying so is the finding.

## What each outcome means, and where it goes

This is the table the session exists to fill in. Milestone 188's phase 4 is a hand-written IPC
fastpath, held by calef pending evidence that the footprint costs anything real; its phases 1 to 3
measured `ipc_call_reply` at 48% to 103% over the 4 KiB target, so the bytes are settled and the
cycles are not.

| what the capture shows | what it means | where it routes |
|---|---|---|
| **`call_reply` and `ipc_rtt_el0` clearly slower padded**, beyond the boot-to-boot spread | Liedtke's claim is live on this machine at this footprint. §95's premise holds, measured rather than argued | **milestone 188 phase 4 is justified**; the magnitude is the expected payoff of a hand-written fastpath and the number to hold it to |
| **padded and un-padded within the spread**, as on patagonia | a doubling of footprint costs nothing measurable on a 32 KB L1i either. This is the strong direction of the negative: E3's own design says a null here is worth far more than a null on patagonia | **§95's premise is in serious doubt.** 188 phase 4 buys a standing verification obligation for an effect two machines cannot find. Route to `design/decisions/95-*` as evidence for closing it |
| **`ipc_rtt` moves and `call_reply` does not** (or the reverse) | the effect is real but shape-specific, which is a result about *which* path to optimise rather than whether to | 188 phase 4, with a narrower scope than currently sketched |
| **E1 `ipc_scale_*` bends sharply in the low tens** | Warton's effect reproduced on the machine the prediction was computed for. §96's performance input is live | `design/decisions/96-process-kernel-or-event-kernel.md`, read against E2's finding that the customer path runs 4 to 8 threads |
| **E1 flat to 96 threads** | the process kernel costs nothing on this axis on the smallest cache we target. Stronger than patagonia's 8-11% rise, in the opposite direction | §96 answered no, on data |
| **E4 `_ipc96` clearly above `_ipc`, and both above zero** | application displacement is real and load-dependent: the Liedtke measurement proper | the register, and 188 phase 4 as supporting rather than deciding evidence |
| **E1 or E4 print `skipped`** | the card was built wrong. `needs a single hart` means `single_hart` was missing; `QEMU virt detected` means this is not the board | rebuild, do not interpret |
| **`MEASURED BOOT REFUSED`** | kernel and archive came from different builds | `script/board-image --card` copies them as a set; copy all three files again |

**Two outcomes are worth naming as genuinely decisive and one is not.** Row 1 and row 2 both settle
188 phase 4, in opposite directions, and both are worth the session. A result that lands inside the
spread but "looks like" a trend is the outcome to resist: this project has a recorded habit of
reporting overlapping ranges as directional findings, and E4's own 2026-08-23 follow-up says so in
its own words.

## What milestone 74's riscv64 half adds, and what it still cannot see

`kernel/src/arch/riscv64/pmu.rs` landed 2026-09-04 and reads real cycles through the SBI PMU
extension. It was not available when E3 was designed, and E3 was designed to work without it: the
whole point of padding is that it tests Liedtke's claim **with no cache counter**, by making a
static footprint tool agree or disagree with a wall clock.

**What the PMU adds here is precision, not a new answer.** Every row above is a `rdtime` tick count
at 4 MHz, which is a 250 ns quantum; a 2% effect on a low-microsecond round trip is a handful of
ticks. Cycles at the core clock resolve that by three orders of magnitude, and they remove the one
methodological complaint nobody could answer on patagonia, which is whether a small percentage was
a real effect or a timer artifact.

**What it still cannot see is the mechanism.** A cycle counter says a round trip got slower; it does
not say the instruction cache is why. The direct measurement is M6, instruction-cache misses per
IPC, and **nothing in this tree reads a cache-miss counter on any architecture.** Milestone 134's
Tier B says so and its own BUGS warns that real PMUs do not implement every architected event, so
whether the U74 counts what M6 wants is unverified. Until then E3 remains what it was designed to
be: an inference from a perturbation, not an observation of a cache.

Wiring the PMU into these rows is a separate piece of work and it is not in this lane; see
`design/roadmap/proposals/cycles-per-ipc-on-the-bench-card.md`.

## EXAMPLES

### The whole session, as a shell transcript

```sh
# on patagonia, no board
script/fastpath-footprint --arch riscv64
script/fastpath-footprint --arch riscv64 --features fastpath_pad

# un-padded card
script/board-image --bench --card /Volumes/NIFE
diskutil unmount /Volumes/NIFE
# power radon on, then, for each of three boots:
script/board-console --for 20m --until none --log target/radon-bench-plain-$(date +%s).log

# padded card
script/board-image --bench --extra-features fastpath_pad --card /Volumes/NIFE
diskutil unmount /Volumes/NIFE
# three more boots:
script/board-console --for 20m --until none --log target/radon-bench-padded-$(date +%s).log

# read the comparison out of the captures
grep -h "^bench: \(call_reply\|ipc_rtt\|ipc_rtt_el0\|cntfrq\) " target/radon-bench-*.log
grep -h "^bench: ipc_scale_" target/radon-bench-plain-*.log
grep -h "^bench-probe: appdisp_" target/radon-bench-plain-*.log
```

### Checking the card before walking to the board

```sh
$ script/board-image --bench
...
built:
  features: board,bench,single_hart
  target/board/nife-vf2.img  (669792 bytes, Image header verified)
  target/board/nife-initrd.img  (... bytes, the userspace archive this kernel measures)
```

A `features:` line missing `single_hart` is a wasted boot, and it is cheaper to read it here than to
read `ipc_thread_scaling skipped` twenty minutes later.

## BUGS

- **No number in this page came off radon.** The procedure is unrun, and an unrun procedure is a
  hypothesis about a machine. The most likely place for it to be wrong is step 3's twenty-minute
  window: nobody has timed a `bench` boot on a 1.5 GHz in-order U74, and E1 and E4 between them do
  a great deal of arithmetic. If the capture ends without `bench: done`, raise the deadline before
  concluding anything about the kernel.
- **`single_hart` has never been booted on hardware either.** It compiles on all three
  architectures and its mechanism is one early return in `bring_up_secondaries` after the online
  mask is set, which is the ordering the x86 bring-up already proved matters. What is untested is
  a *board* boot with three U74s left parked by us rather than by firmware. They are left exactly
  as OpenSBI handed them over, which is the same state they are in before `bring_up_secondaries`
  runs on any boot, so the expectation is that nothing notices.
- **Nothing in CI compiles this card, or any card.** `board`, `soak`, `jobmix`, `reboot_soak`,
  `single_hart` and `fastpath_pad` are built when a person runs `script/board-image`, minutes before
  walking to the bench. A refactor that breaks a card build leaves the tree green until then, and
  the error arrives at the worst possible moment. Six release builds of one crate would close it:
  `design/roadmap/proposals/board-only-features-nothing-compiles.md`.
- **The bench card measures fewer things than an ordinary one.** `smp_throughput`, `fs_read` and
  `fs_throughput` self-skip under `single_hart`. A session that wants a multi-core number from
  radon builds a second card without the flag, and that card cannot produce E1 or E4.
- **The padded and un-padded images are indistinguishable on the card.** Same three filenames, no
  build stamp in the payload, and nothing on the board prints its feature set. The mitigation is
  the `features:` line in the build output and the operator's own log filename, which is rung four
  of AGENTS.md's ladder and is honest about it. A kernel that printed its own feature set at boot
  would be rung three and is proposed in
  `design/roadmap/proposals/a-boot-banner-that-names-the-build.md`.
- **E3's `black_box` guard is a confound on both shapes now, not one.** `ipc_send` and `ipc_call`
  each carry one untaken compare-and-branch when the feature is on. `kernel/src/fastpath_pad.rs`'s
  module doc prices it at around a nanosecond against a low-microsecond round trip; on radon the
  round trip is longer and the branch is not faster, so the ratio only improves. It is still a real
  asymmetry between the two builds and it is why an effect at the 1% level should not be believed.
- **The riscv64 TCG self-skip is a single-value test.** It says "not QEMU `virt`", not "has a
  32 KB L1". Another riscv64 machine with a different timebase would run E1 and E4 and be believed.
  Which machine a capture came from lives in this page and in the operator's log filename, not in
  the kernel.
- **Nothing here re-takes E2.** The thread census (4 new threads on the SMB/FS path) was taken on
  both ISAs under QEMU on 2026-08-22 and is a topology fact rather than a timing one, so it does
  not need the board. If the customer path changes, E2 changes, and E1's reading against it
  changes with it.
