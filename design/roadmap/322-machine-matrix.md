# 322. One machine matrix for three architectures, because a board is a memory map and not a CPU

**Status: NOT-STARTED.** Minted 2026-09-18 by calef, from asking whether breadth is bought in QEMU
or only on silicon. *(Number provisional until the merge queue lands it.)*

**Gate: NONE.** Every machine named below is in the pinned QEMU already. Nothing has to be bought,
ported or decided first; `script/cpu-matrix` is the shape to copy and milestone 59 already argued
the general case for narrowing an emulator rather than forking one.

**In brief.** Milestone 59 built a matrix over `-cpu` and left `-machine` at `virt` on every
architecture. **Every RISC-V, aarch64 and x86_64 result this project has is from one board per
architecture**, which is the same blind spot 59 found in `-cpu rv64` one axis over: a single machine
that has never told the kernel anything it did not want to hear.

## The evidence that this is not hypothetical

**A machine option already boots the whole tour and silently loses every interrupt.** Milestone 222
measured `-machine virt,gic-version=3` on 2026-09-02, and the result is the most dangerous shape a
failure has: the kernel boots, brings four cores online, prints `timer: 100 Hz tick, interrupts ON`,
and then takes **zero interrupts and zero preemptions, with nothing faulting and nothing said**.
`memory::gic_regions()` matches `intc@` by name and takes the first two `reg` blocks, so a GICv3 node
hands it the distributor and the **redistributor**, and `gic::init_this_cpu` writes `GICC_PMR` and
`GICC_CTLR` into registers that are not there.

That was found by a person running a command by hand, not by a gate, and it is why the runner's
`gic-version=2` pin is load-bearing rather than tidy. **A machine matrix is the gate that would have
found it**, and it is the same failure this project keeps meeting from other directions: a run that
is indistinguishable from a passing one.

**And a machine's firmware has already hidden a missing boot path.** `notes/x86-port.md`: PVH is a
hypervisor protocol and no real firmware speaks it, so the kernel booted under QEMU for weeks by a
route the OptiPlex could not offer. QEMU's convenience concealed the gap until real hardware refused
it. A second machine profile is the cheap half of noticing that class earlier.

## What it is not

**It is not "run every machine".** The pinned QEMU offers about 118 aarch64 and 137 x86_64 machines,
and a matrix over them would be a very slow way to test one memory map many times. The axis worth
covering is **what a machine makes the kernel negotiate**: its memory map, its interrupt controller,
whether it describes itself by device tree or ACPI, and how its PCI configuration space is reached.
Each row earns its place by answering one of those differently from `virt`.

**It is not a substitute for silicon, and nothing here should be read as one.** notes/cpu-models.md's
`BUGS` is the standard: *"a narrower QEMU model is still QEMU"* and *"a green matrix is not a portable
kernel. It is the absence of one specific class of failure."* Cache behaviour, real memory maps,
errata and real firmware stay silicon's, and `design/fatal-risks.md`'s risk 5 already fired there:
the VisionFive 2 produced a receiver woken with nothing delivered, on three harts, that no emulator
run had ever shown.

## The rows, and why each one differs from `virt`

Provisional, and the first deliverable is confirming each against the pinned QEMU rather than this
list being taken on faith.

**aarch64**

- `virt,gic-version=3`, and then `4`. The interrupt controller, which is the case above. This row is
  red until milestone 227 lands a GICv3 driver, and **that is the point**: it turns 227 from a
  judgement into a failing gate.
- `sbsa-ref`. Describes itself by **ACPI with no device tree**, which is the discovery seam x86_64
  already exercises and aarch64 never has.
- `raspi3b`. A real SoC memory map with a different UART, which is the shape milestone 20's portable
  HAL claims to make cheap.

**riscv64**

- `sifive_u`. A real SoC map rather than `virt`'s, closest in kind to the U74 on radon.
- `spike`, `microchip-icicle-kit`. Different again, and cheap; keep whichever says something new.

**x86_64**

- `pc` (i440fx) beside `q35`. Legacy PCI configuration against PCIe ECAM, which is exactly what
  milestones 165, 215 and 314 reason about, tested today on one of the two.

## Whether more CPU models belong here too

**Only where a model negotiates differently, and the criterion is the deliverable.** Milestone 59's
five models earn their keep by saying no to distinct things: `svadu` (hardware A/D bits, absent on
`sifive-u54` and `thead-c906`, inverted on the RVA profiles), Sv39 against `rv64`'s Sv57, and `sstc`
(timer compare in a CSR against an SBI call). A sixth model that refuses nothing the five already
refuse buys runtime and no coverage.

So this milestone should **write that criterion down and apply it once per architecture**, which is
work neither 59 nor this block has done: aarch64 and x86_64 have no CPU matrix at all, and whether
they want one is the same question answered with their own feature lists rather than by analogy.

## Index row

Every result on every architecture comes from one QEMU machine, `virt`, so the kernel has never been
asked to cope with a second interrupt controller, memory map, discovery mechanism or PCI
configuration route. That is milestone 59's blind spot one axis over, and it is not hypothetical:
`virt,gic-version=3` already boots the whole tour and then takes zero interrupts with nothing said,
found by hand rather than by a gate. This adds the `-machine` axis on all three architectures, with
each row earning its place by negotiating something `virt` does not, and records the criterion for
when another CPU model is worth its runtime.
