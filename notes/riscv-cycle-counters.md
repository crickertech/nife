# Cycle counters on RISC-V, and why nothing here has measured one

Milestone 74's riscv64 half. What the kernel now asks firmware for, what it does with the answer,
and the bench procedure that would turn any of it into a number. **The procedure has not been run.**
radon (the VisionFive 2) was powered off and there was no bench session while this was built, so
everything below that is not marked as read off a machine is either read off QEMU (which cannot
answer the question) or read out of a specification.

## Three counters, and only one of them counts cycles

| | what it is | how U-mode reads it | rate |
|---|---|---|---|
| `time` (`0xc01`) | a fixed-rate reference tick | `rdtime`, gated by `scounteren.TM` | 10 MHz on QEMU `virt`, 4 MHz on the JH7110 |
| `cycle` (`0xc00`) | this hart's CPU cycles | `rdcycle`, gated by `scounteren.CY` **and** `mcounteren.CY` | the core clock, and it moves |
| `instret` (`0xc02`) | retired instructions | `rdinstret`, same two gates | one per instruction |

`crates/user_rt`'s `now()` and `crate::arch::timer::now()` both read the **first** row. Every
number in `bench/baseline-riscv64.txt` is denominated in it. That is the right instrument for a
long loop and it is not cycles: notes/pmu.md sets out the same distinction for aarch64 and calls
confusing the two a category error.

## Why a supervisor kernel has to ask firmware

The counters themselves are M-mode state. `mcycle` and the `mhpmcounterX` CSRs are started,
stopped and configured through `mcountinhibit` and `mhpmeventX`, and this kernel never runs in
M-mode. The `cycle` CSR an S-mode kernel can read is a read-only shadow of `mcycle` that firmware
may or may not have enabled.

So there is no `csrw` that starts a cycle counter here, and that is what the SBI **PMU** extension
(EID `0x504D55`, `"PMU"`) is for. Its own opening paragraph says it, and this is the RISC-V SBI
Specification **v3.0** (ratified 2025-07-16), `src/ext-pmu.adoc`, read 2026-09-03 from
`github.com/riscv-non-isa/riscv-sbi-doc` at tag `v3.0`:

> These hardware performance counters can only be started, stopped, or configured from machine-mode
> using `mcountinhibit` and `mhpmeventX` CSRs.

## What `kernel/src/arch/riscv64/pmu.rs` does

Four calls, in this order, once, on the boot hart, right after `arch::isa::init` has probed which
SBI extensions firmware implements.

1. **`sbi_pmu_num_counters`** (FID #0). How many logical counters this hart has, hardware and
   firmware together.
2. **`sbi_pmu_counter_config_matching`** (FID #2) over that whole set, for event
   `SBI_PMU_HW_CPU_CYCLES` (type 0, code 1), with `CLEAR_VALUE` and `AUTO_START`. This is the call
   that matters: it means *find me a counter that can count this and start it*, so the kernel never
   has to know a platform's event-to-counter mapping. `SKIP_MATCH` is deliberately **not** passed,
   because it would tell firmware to hand back the first counter in the set without checking it can
   count the event, which is the one thing being asked for.
3. **`sbi_pmu_counter_get_info`** (FID #1) on whichever counter came back. Its packed word says
   hardware or firmware, and for hardware **which CSR reads it**.
4. **`sbi_pmu_counter_stop`** (FID #4), which is never called on the happy path.

The read is then one instruction. That is the whole point of step 3 and the reason the interface is
worth this much ceremony: **an `ecall` inside a cycle measurement measures the `ecall`.** A
*firmware* counter would have to be read through FID #5, one `ecall` per read, so this module
refuses those rather than reporting a number whose cost dominates whatever it was timing.

`counter_info`'s layout has one trap in it and the decoder is host-tested against it
(`crates/machine_discovery/tests/riscv64_isa_strings.rs`): the width field is **one less** than the
counter's width, so a 64-bit `mcycle` reports 63. A decoder that skipped the `+ 1` would report a
counter that wraps at 2^63 and nothing would ever notice.

## What QEMU says, and why it settles nothing

Every merge boots this, and it passes:

```text
  firmware    : OpenSBI 0x10006, SBI 3.0, TIME IPI RFENCE HSM PMU
  cycles      : SBI PMU counter 0, CSR 0xc00, 64 bits
```

and `cargo xtask bench --riscv` prints

```text
  probe: cycles_per_tick 100.00 (10000029 cycles over 100000 ticks at cntfrq 10000000)
```

**That ratio is an artifact and the exactness is the tell.** QEMU-TCG has no cycles to count; under
`-icount` the `cycle` CSR and the `time` CSR are both driven off the same virtual clock, so the
answer is the ratio between two views of one number and comes out to 100.00 with a rounding
wobble. A green test under emulation proves the plumbing and says nothing whatever about the
measurement.

So what the tree can honestly claim today is: **the calls are made, the answer is decoded, the CSR
named is readable and advances.** Four kernel tests assert exactly that much and no more
(`kernel/src/arch/riscv64/pmu.rs`, `mod tests`).

## The bench procedure

**Not run.** Written to be followed rather than interpreted, in the shape notes/x86-uefi-boot.md's
own bench section uses, and it assumes you have already done the setup in notes/visionfive2.md's
runbook: card, DIP switches to QSPI, serial on pins 6/8/10 at 115200 8N1 with the terminal attached
before power, USB-C in last.

### Step 0: the whole of it is three lines of output

Nothing here needs a debugger or a special build. The boot prints two of the three answers and
`cargo xtask bench` prints the third, so the session is: boot once, read two lines; boot the bench
image, read one more.

### Step 1: does radon's OpenSBI implement the PMU extension at all?

This is the first thing to check because it is a fact about somebody else's firmware and every
later step is conditional on it. The kernel already probes it and prints it.

```console
$ cd /path/to/nife
$ script/board-image --card /Volumes/NIFE          # then eject, insert, power on
$ script/board-console --until tour --log /tmp/radon-pmu.log
```

Read the `firmware    :` line. The extension list is printed in `SBI_TABLE` order, so PMU is last:

| What the line says | What it means | What to do |
|---|---|---|
| `... TIME IPI RFENCE HSM PMU` | firmware implements SBI PMU | go to step 2 |
| `... TIME IPI RFENCE HSM` (no PMU) | **this firmware has no PMU extension.** Not a defect in this tree and not a build problem; the vendor's OpenSBI predates it or was configured without it | record the OpenSBI version from its own banner earlier in the log, and stop. The cycle half of milestone 74 cannot proceed on this firmware, and the next question is whether a newer OpenSBI can be flashed |
| `SBI base extension did not answer` | firmware is older than SBI 0.2 and cannot be asked anything | stop; this contradicts every previous boot of this board and something else is wrong |

**`sbi probe` at the U-Boot prompt is not a substitute** and should not be used to answer this. The
kernel's own probe is what the kernel will act on, and a second tool agreeing or disagreeing with it
is a fact about the tool.

### Step 2: did firmware hand back a hardware counter, and which CSR?

The next line, printed by `arch::pmu::print_summary`.

| What the line says | What it means | What to do |
|---|---|---|
| `cycles      : SBI PMU counter 0, CSR 0xc00, 64 bits` | the expected answer: `mcycle`, read through `cycle`, full width | go to step 3 |
| `cycles      : SBI PMU counter N, CSR 0xc03..0xc1f, W bits` | firmware gave a programmable `hpmcounter` instead of the fixed one. Legitimate, and **the width matters**: the U74's `hpmcounter`s are not necessarily 64 bits, and a narrow counter wraps inside a long benchmark | record N and W; go to step 3, and treat any bench window longer than `2^W` cycles as unusable |
| `cycles      : SBI PMU present but no hardware cycle counter` | one of three things: firmware answered `SBI_ERR_NOT_SUPPORTED` to the match, it offered a *firmware* counter (which this kernel refuses, see above), or it named a CSR outside `0xc00..0xc1f` | this is the case worth a bisect. Add a temporary print of the `counter_config_matching` error and the raw `counter_info` word in `pmu::init`; the three cases are distinguishable and they need different fixes |
| no `cycles` line at all | the boot did not reach `arch::pmu::print_summary`, so this is a boot problem and not a PMU one | notes/visionfive2.md's failure-triage ladder |

### Step 3: is the number plausible?

```console
$ cargo xtask bench --riscv                        # for the card image, per notes/visionfive2.md
```

and read the one probe line:

```text
  probe: cycles_per_tick R.RR (C cycles over T ticks at cntfrq 4000000)
```

`cntfrq` should read **4000000** on this board (the JH7110's `/cpus/timebase-frequency`; QEMU
`virt` says 10 MHz, and reading 10 MHz here would mean the DTB was not the board's).

| `R.RR` | Reading |
|---|---|
| in the hundreds | **This is the answer the milestone wanted.** `R * 4 MHz` is the core clock the counter ran at, measured rather than assumed, and every row in `bench/baseline-riscv64.txt` can now be restated in cycles by multiplying by `R` |
| `1.00`, or any exact round number with no wobble across runs | the two counters are the same counter wearing two names. That is what QEMU does and it should not happen on silicon; suspect firmware mapped the event to `REF_CPU_CYCLES` or to a fixed-frequency counter rather than to `mcycle` |
| `0.00`, or the same `C` at both ends | the counter is not running. Most likely `mcounteren.CY` is clear in this firmware, so the S-mode `rdcycle` is trapping or reading zero, which is the exact assumption §139's BUGS section flagged as untested |
| wildly different between two runs of the same command | frequency scaling, which is real and is the counter working. Record several and say so; a cycle counter whose rate moves is what the SBI specification's own note warns about |

**Record the number in `notes/benchmarks.md`, not here.** The cited round-trip figure there is
currently arithmetic performed on a nanosecond measurement using an assumed clock, and this ratio is
what turns it into a read one. That is the whole reason milestone 74 exists.

## What this changes about milestone 16a

16a's deliverable includes "the benches on real cycles via the SBI PMU extension". The extension is
now spoken and the harness prints the ratio, so what is left is a person at the bench following the
three steps above. That is the second sense of the `HARDWARE` gate, not the first.

## BUGS

- **Nothing here has been measured on silicon.** The procedure is written and untested, the same
  way notes/x86-uefi-boot.md's bench section was when it was written, and the first person to
  follow it should expect at least one line of output to be worded differently from the table.
- **Whether radon's OpenSBI implements SBI PMU is unknown.** Step 1 exists because it is a fact
  about the vendor's firmware build and nobody here has read it. What *is* read is upstream: OpenSBI
  master's `lib/sbi/sbi_hart.c` writes `CSR_MCOUNTEREN, -1` ("Supervisor mode usage for all counters
  are enabled by default") and `CSR_MCOUNTINHIBIT, 0xFFFFFFF8`, leaving `CY` and `IR` clear so
  `mcycle` and `minstret` run (read 2026-09-03). The shipped flash is a different build and a
  possibly much older one.
- **The boot hart only.** SBI PMU counters are per-hart: `counter_config_matching` and
  `counter_start` act on the calling hart, and the counter index is that hart's. `pmu::init` runs
  once, on the hart that runs `kernel_main`, so `pmu::cycles()` is only meaningful to a reader who
  knows which hart it ran on. The one consumer today is a single-hart bench probe. A per-hart record
  is real work and belongs with a caller that needs it, which is milestone 147's shape rather than
  this one's.
- **The counter is never stopped.** It runs for the life of the boot, which is correct for a
  free-running counter read as a difference, and it means `pmu::stop` is exercised only by the test
  that proves the call works.
- **`cycles_per_tick` is a probe and never gates.** It is a rate, not a duration, so it does not
  enter `bench/baseline-riscv64.txt` and `--check` never polices it. On a machine that scales
  frequency it could not be policed: that is the counter working, not a regression.
- **The aarch64 half is not built and is not this note's.** `PMCCNTR_EL0` is equally unimplemented,
  which is a real §19 parity gap in the one subsystem whose entire purpose is cross-machine
  comparison. It waits on milestone 75, by milestone 74's own gate. notes/pmu.md is the aarch64 side.
- **A firmware counter is refused rather than read.** If a platform's only way to count cycles is a
  counter the SBI implementation maintains itself, this kernel reports no cycle counter at all
  rather than reading it at one `ecall` per read. That is deliberate and it is a real limit: such a
  platform gets nothing here.
