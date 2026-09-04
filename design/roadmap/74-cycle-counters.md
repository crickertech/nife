# 74. Cycle counters: SBI PMU on RISC-V, `PMCCNTR_EL0` on aarch64

**Status: PARTIAL.** Raised 2026-08-03, from an audit of what milestone 16a actually needs. Its
deliverable includes "the benches on real cycles via the SBI PMU extension", and until 2026-09-03
**nothing in the tree implemented it**: `PMU` appeared only in device-tree test fixtures and in this
file.

**Gate: MILESTONE 75, HARDWARE.** The aarch64 half must not land until 75 answers whether EL0 may
read the counter at all; `design/decisions/139-cycle-counter-authority.md` is that question's
evidence and calef's answer, and 75 itself is still `NOT-STARTED`. The `HARDWARE` half is now the
*second* sense of that gate rather than the first: the riscv64 code is written and gated, and what
remains is a person at radon following notes/riscv-cycle-counters.md's procedure, because QEMU-TCG
models an instruction counter that has nothing to do with cycles.

**The riscv64 half is built** (2026-09-03): the SBI PMU extension is probed as an optional fifth
row of `SBI_TABLE`, `kernel/src/arch/riscv64/pmu.rs` asks firmware to find and start a counter for
`SBI_PMU_HW_CPU_CYCLES` and remembers which CSR reads it, the boot prints both facts, and
`cargo xtask bench --riscv` prints one `cycles_per_tick` probe. **The aarch64 half is not**, by this
file's own gate.


## What we read today, and why it is not cycles

Both ISAs read a **fixed-rate reference counter**, not a cycle counter:

| | aarch64 | riscv64 |
|---|---|---|
| today | `CNTVCT_EL0` + `CNTFRQ_EL0` | the `time` CSR (`rdtime`) |
| counts | a fixed tick, 62.5 MHz under QEMU | a fixed tick |
| resolution | ~41 ns on real silicon | comparable |
| the cycle counter we lack | `PMCCNTR_EL0` | SBI PMU, or the `cycle` CSR when `mcounteren` permits |

notes/pmu.md already sets this out for aarch64 and calls confusing the two a category error. The
generic timer is the OS's clock; the PMU counts CPU cycles at ~0.25 ns resolution and its rate moves
with frequency scaling. **Two ways to measure a fast operation: one shot at high resolution (PMU,
which is what sel4bench does) or a long loop at low resolution (the generic timer, which is what we
do).** Both are valid and they fail under different conditions.

## Why it matters more than "another counter"

The thesis claim is a cross-OS comparison, and **the literature it is compared against is denominated
in cycles**, not nanoseconds. notes/benchmarks.md does the conversion by hand and draws the honest
conclusion:

> seL4 publishes, for the same-core different-address-space path, 413 cycles for the IPC call and
> 426 for the IPC reply, one-way each ... So the corrected figure is roughly 1.1x to 1.7x an
> L4-lineage round trip, not 4 to 7 times.
>
> -- notes/benchmarks.md

**This file has now quoted that paragraph wrongly twice, which is why the quote above carries an
attribution line a gate can check.** The first version quoted the retracted arithmetic (*"At ~3.2
GHz, 705 ns is ~2,200 cycles round trip... we are 4 to 7 times heavier"*) as the current record,
after milestone 101 had re-measured it and found three errors. The replacement written on 2026-08-04
was a **paraphrase presented as a quotation**: it read "At ~3.2 GHz, 350 ns is roughly 960 to 1,420
cycles round trip", and those words appear in no note.

**It also put back the one assumption the correction had removed.** The note says in as many words
that the old paragraph's 3.2 GHz "is not this machine"; the 960-to-1,420 range is 350 ns against the
**M3's two clocks**, 2.75 GHz on an E-core and 4.05 GHz on a P-core, and nothing pins the vCPU
thread to either. At 3.2 GHz, 350 ns is ~1,120 cycles, a single number rather than a range. So the
sentence attached a correct range to the clock that had just been rejected, and read as sober while
doing it.

The mechanism rather than the apology: a prose block quote of another document is a citation no gate
resolves, and `script/citations` (milestone 97) checks one **only when it carries a `-- path`
attribution line**. Neither wrong version had one, so both passed every gate in the tree, twice, in
the file notes/citations.md already uses as its worked example. The binding form costs one line and
is rung two of CLAUDE.md's ladder instead of rung four.

The figure above is still arithmetic performed on a nanosecond measurement using an assumed clock
rate, which is the whole point of this milestone. Measuring cycles directly turns the project's
most-cited number from a derived figure into a read one, and it is the number a reader from the L4
world will look for first.

## Two things block on it

- **16a** cannot deliver "benches on real cycles" without it.
- **Milestone 25's `sel4bench`** is built and booting but was deferred to real hardware precisely
  because it times single operations through `PMCCNTR_EL0`, which neither QEMU-TCG nor Apple HVF
  provides. notes/pmu.md's last section explains why virtualization keeps the PMU out of reach: the
  generic timer is architected state a hypervisor must present, and the PMU is not.

## Parity makes this two ISAs, not one

§19 is a gate, and it bites here in an unobvious direction. The milestone reads as RISC-V work because
16a is the RISC-V board, but **`PMCCNTR_EL0` is equally unimplemented**, so a RISC-V-only cycle
counter would create a parity gap in the one subsystem whose entire purpose is cross-machine
comparison. Both sides are small and they are not symmetrical in shape:

- **aarch64**: enable the counter (`PMCR_EL0`), open it to EL0 (`PMUSERENR_EL0`), read `PMCCNTR_EL0`.
  Register writes, no firmware call. **Whether that EL0 opening happens at all is milestone 75's
  decision, not this one's**, and the aarch64 half should not land until it is answered: the counter
  is ~160x finer than the one §10 already excepted, so it does not inherit that exception.
- **riscv64**: the SBI PMU extension (EID `0x504D55`), which discovers counters, configures an event,
  and starts and stops it. The tree already makes SBI calls (`SBI_HSM_EID`, `SBI_IPI_EID`,
  `SBI_RFENCE_EID`, SBI TIME), so the plumbing exists and this is a fifth extension rather than new
  machinery. **Built 2026-09-03**, and the one thing it forced was not the `ecall`: `SBI_REQUIRED`
  was every row of `SBI_TABLE` by construction, because every extension the kernel had ever called
  was one it could not boot without. PMU is the first that is an **instrument**, and a kernel that
  refused to boot without an instrument would have confused the measurement with the thing measured.
  So `SbiRow` grew the `required` field `Row` already carried one source over, and the existing
  accumulation test failed on the first run, which is what it was for.

## What can be done before the board, and what cannot

**Buildable now:** both drivers, the `Isa`-style capability probe, the benchmark harness change, and
the aarch64 path end to end (Apple Silicon has a real PMU; whether macOS lets a guest reach it is the
open question notes/pmu.md raises).

**Not verifiable until silicon:** the RISC-V numbers. QEMU-TCG models an instruction counter that has
nothing to do with cycles, so a green test under emulation proves the plumbing and says nothing about
the measurement. Say so in the note rather than publishing an emulated cycle count.

**And the emulator was more misleading than that prediction allowed for**, which is the one finding
worth carrying out of the riscv64 half. Under `-icount` the `cycle` CSR and the `time` CSR are driven
off the same virtual clock, so the probe reads **`cycles_per_tick 100.00`**, exactly the ratio
between the two declared rates, with a rounding wobble and nothing else. That is not a number that
looks wrong. It looks like a clean measurement of a 1 GHz core, and a reader who did not know what
TCG does to these two registers would have every reason to write it down. The defence is that the
probe line prints its inputs (`10000029 cycles over 100000 ticks at cntfrq 10000000`) rather than
only the ratio, so the arithmetic is visible, and that notes/riscv-cycle-counters.md's outcome table
names an exact round ratio as the tell rather than as the answer.

## Scope note

**Do not turn this into a profiling framework.** One counter, read before and after, on two ISAs. The
PMU can count dozens of events and the temptation to expose them generically should wait for a second
consumer, which is CLAUDE.md's rule against speculative trait-ification. `sel4bench` comparability is
the requirement; anything beyond it is scope.

## What the riscv64 half built, and what it deliberately did not

**Built 2026-09-03**, lane `milestone/74-cycle-counters-riscv`.

- `SBI_PMU` / `EID_PMU` as an **optional** row of `machine_discovery::riscv64::SBI_TABLE`, with
  `SBI_REQUIRED` narrowed to the rows that ask for it. Probed by the existing `probe_sbi` loop with
  no new code, and printed on the existing `firmware    :` boot line for the same reason.
- `CounterInfo`, the host-tested decode of `sbi_pmu_counter_get_info`'s packed word, including the
  `+ 1` on a width field the specification writes as one less than the width, and `None` for both
  CSR and width on a firmware counter (the specification says they "should be ignored", so they are
  not returned rather than returned as numbers a caller could use by accident).
- `kernel/src/arch/riscv64/pmu.rs`: the four calls, the CSR-number dispatch, and one boot line.
  Four kernel tests, each of which asserts plumbing and says in its own doc comment that it is not
  asserting a measurement.
- One `bench-probe: cycles_per_tick` line from `cargo xtask bench --riscv`. **A probe, not a row**:
  it is a rate rather than a duration, so it never enters `bench/baseline-riscv64.txt` and `--check`
  never polices it, which is the existing convention `map_new`'s shootdown probe established. One
  measured ratio converts every existing tick-denominated row at once, which is why it is one line
  at the top rather than a second number on every line.
- notes/riscv-cycle-counters.md, the bench procedure, written in notes/x86-uefi-boot.md's shape:
  in order, with real commands, and a table mapping each observable line to what it means. Step one
  is whether radon's OpenSBI implements the extension at all, because that is a fact about somebody
  else's firmware and every later step is conditional on it.

**Deliberately not built**, and each is the scope note above rather than an oversight: no second
event, no per-hart counter record (SBI PMU counters are per-hart and this configures the boot
hart's; the one consumer is a single-hart probe), no user-facing read (the U-mode `rdcycle` path is
milestone 229's grant and 237's measurement build, which already exist), and no firmware-counter
support (reading one costs an `ecall` per read, and an `ecall` inside a cycle measurement measures
the `ecall`).
