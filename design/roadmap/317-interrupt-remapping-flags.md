# 317. The interrupt-remapping flags

**Status: BUILT.** Minted by the maintainer on 2026-09-17 out of
[DECISIONS §86](../decisions/86-el0-nvme-driver.md)'s research pass, which found the gap while
pricing an EL0 NVMe driver. *(Number provisional until the merge queue lands it.)*

## What this is, and what it deliberately is not

**This makes interrupt remapping exercisable. It does not remap an interrupt, and it does not
decide who may.**

An MSI or MSI-X message is a memory write to an architecturally special address, so an IOMMU doing
DMA remapping alone does not confine it: a component that can write a device's MSI-X table can aim
an interrupt at a vector it was never given. VFIO refuses to hand a device to an untrusted
userspace driver on a machine without interrupt remapping for this reason, and names its escape
hatch `allow_unsafe_interrupts`.

§86 found that **no boot this tree runs could exercise the question even if a claim existed**, and
`notes/confinement-claims.md` carries it as a claim stated nowhere. Two runner flags were named as
the cheap first move. This milestone is those two flags, plus the one thing that makes them worth
having: a way to tell from **inside the guest** which machine you are on.

**Who owns the page holding the MSI-X table is not decided here.** calef held that decision on
2026-09-17 for want of an experiment behind it. This is the experiment.

## x86_64: it works, and the suite does not notice

`scripts/qemu-runner-x86_64.sh` gains **`NIFE_INTREMAP`** (provisional name). Set it to anything
non-empty and the VT-d device gains `intremap=on`:

    -device intel-iommu                # default, unchanged
    -device intel-iommu,intremap=on    # NIFE_INTREMAP=1

Run the whole suite either way:

    script/test --arch x86_64                    # 214 passed, 70 skipped, exit 0
    NIFE_INTREMAP=1 script/test --arch x86_64    # 214 passed, 70 skipped, exit 0

**Identical, and that is the finding that needed a second half.** A green suite with the flag on
proves only that the suite does not care, which is milestone 202's hazard (every confinement test
is a ritual until somebody breaks the confinement) wearing a new coat. So the guest now reads
`ECAP.IR` and says what it saw, and the bring-up line differs between the two machines:

    iommu : VT-d drhd at 0x00000000fed90000, root table default-deny, translating, interrupt remapping absent
    iommu : VT-d drhd at 0x00000000fed90000, root table default-deny, translating, interrupt remapping offered

`arch::x86_64::iommu::interrupt_remapping_available` is the query; `print_summary` and one test are
its callers.

### `kernel-irqchip=split` is not needed here, and that is measured

The advice that pairs `intremap=on` with `-machine kernel-irqchip=split` is real and is a **KVM**
constraint: QEMU refuses interrupt remapping with an in-kernel irqchip. patagonia has no KVM, so
`q35` under TCG emulates the whole irqchip in the QEMU process and the check never fires. Against
QEMU 11.1.1, started with `-S` and quit from the monitor so machine init runs and nothing executes:

    -machine q35 -device intel-iommu,intremap=on                     # starts, no diagnostic
    -machine q35,kernel-irqchip=split -device intel-iommu,intremap=on # starts, no diagnostic
    -machine q35,kernel-irqchip=on -device intel-iommu,intremap=on    # starts, no diagnostic

The runner does not add the flag. A Linux host running this suite under KVM would need it, and
adding it here would assert a machine fact nobody on this machine can check.

### Why opt-in rather than default-on

**The flag breaks nothing**, so "it fails the default suite" is not the reason and would be a
better one if it were true. The reason is that nothing in this tree remaps an interrupt, so
default-on would give every boot a capability no code reads, and would retire the one thing the
flag is actually good for: running the same kernel against a machine that has the hardware and a
machine that does not, and being able to tell which. A default that erases the comparison spends
the flag to buy nothing.

It is also the reversible half of the choice (`move fast on what can be undone`). The day something
here programs an `IRTE`, default-on becomes the obvious call and costs one line.

## aarch64: it cannot be switched yet, and here is precisely why

**`gic-version=3` is one flag. Surviving it is milestone 227.** QEMU accepts the machine; this
kernel does not survive it, and the failure is silent rather than loud, which is the part worth
recording.

Measured with `dumpdtb` on QEMU 11.1.1, both versions of the same `virt` machine:

| | `gic-version=2` | `gic-version=3` |
|---|---|---|
| node | `intc@8000000` | `intc@8000000` |
| `compatible` | `arm,cortex-a15-gic` | `arm,gic-v3` |
| `reg[0]` | `0x8000000 + 0x10000` (GICD) | `0x8000000 + 0x10000` (GICD) |
| `reg[1]` | `0x8010000 + 0x10000` (GICC) | `0x80a0000 + 0xf60000` (GICR) |
| MSI child | `v2m@8020000` | **`its@8080000`** |

The ITS is right there for the taking, which is what makes this tempting and what makes the rest of
the row matter.

`memory::init` finds the controller by the **`intc@` name prefix** and never reads `compatible`, so
it matches a GICv3 node just as happily, takes `reg[1]`, and hands it to `arch::aarch64::irq::init`
as "the CPU interface". On a GICv3 that region is the redistributor frame array, whose register
layout has nothing to do with a GICv2 CPU interface. **Nothing anywhere refuses the mismatch.**

`NIFE_GIC=3 cargo xtask boot-check --arch aarch64` (the flag is this milestone's, and the runner
prints a warning saying the configuration is unsupported):

    interrupts      : GICv2, distributor 0x0000000008000000, cpu interface 0x00000000080a0000
    self-test       : timer      FAILED  the 62MHz counter advanced 1250000 and 0 tick(s) arrived in ~20ms
    self-test       : scheduler  FAILED  thread 5 did not run within 2s (saw 0x0)
    nife self-test: 3 of 5 passed, 2 FAILED: timer scheduler

Against `gic-version=2`, the same command is 5 of 5.

**Read the first line twice.** The kernel says "GICv2" and prints the redistributor base under the
heading "cpu interface". It is not confused; it never asked. The machine description is stating a
belief, and on this machine the belief is false. That is the shape of failure this tree fears most:
a record that is wrong and looks like a reading.

### What milestone 227 would owe

Four things, and only the second is the large one its block warns about:

1. **A machine that says which controller it has.** `memory::init`'s `intc@` prefix must read
   `compatible` and distinguish `arm,cortex-a15-gic` from `arm,gic-v3`, and
   `arch::aarch64::irq::print_summary` must report what it found rather than the literal `GICv2` it
   prints today. **This is worth doing even if 227 never starts**, because it converts a silent
   wrong answer into a refusal; it is not done here only because it is 227's file and this lane is
   not 227.
2. **The GICv3 driver.** Distributor programming moves to affinity routing, the CPU interface moves
   from MMIO to system registers (`ICC_*`, which is real `arch/` work), and the redistributor gets
   a per-core frame with its own wake protocol. This is the large piece.
3. **The ITS, which is the whole reason interrupt remapping wanted GICv3.** Command queue, device
   table, interrupt translation tables, and `MAPD`/`MAPTI`/`INV`. Without it `gic-version=3` buys a
   different interrupt controller and no MSI translation, which is not what this question is about.
4. **Parity's bill.** DECISIONS §19 makes a kernel capability ship on every architecture or carry a
   scope note. x86_64 can offer interrupt remapping today and aarch64 cannot, so any claim built on
   it starts life with a recorded gap and this block is where it points.

## riscv64

Untouched and unexamined by this lane. `-device riscv-iommu-pci` is on that runner and the RISC-V
IOMMU has MSI redirection in its own spec (`MSIPTP`, the MSI page table), so the question has an
answer there too and nobody has asked it. Named here rather than left implied; it is not this
milestone's work.

## EXAMPLES

Run the x86_64 suite against a unit that offers interrupt remapping:

    NIFE_INTREMAP=1 script/test --arch x86_64

See what the guest makes of the machine, both ways:

    cargo xtask boot-check --arch x86_64                  # ... interrupt remapping absent
    NIFE_INTREMAP=1 cargo xtask boot-check --arch x86_64  # ... interrupt remapping offered

Reproduce aarch64's position in one command:

    NIFE_GIC=3 cargo xtask boot-check --arch aarch64      # 3 of 5, timer and scheduler dead

Compare the device trees the two GIC versions produce, which is how the table above was made:

    qemu-system-aarch64 -machine virt,gic-version=3,iommu=smmuv3 -cpu cortex-a72 \
        -display none -dumpdtb=/tmp/gic3.dtb
    dtc -I dtb -O dts /tmp/gic3.dtb | grep -A 12 'intc@'

## BUGS

- **Nothing here remaps an interrupt, and the flag only proves the hardware is present.** No
  `IRTE` is allocated or written, `GCMD.IRE` is never set, and no MSI is forged and traced to a
  vector it was not given. `interrupt_remapping_is_reported_and_never_enabled` asserts `GSTS.IRES`
  is clear precisely so this sentence cannot quietly stop being true, but an assertion that
  remapping is *off* is not evidence about what remapping would *do*.
- **So the confinement claim is still stated nowhere**, and `notes/confinement-claims.md` still
  carries it as such, now pointing here for the flags. What would settle it is a test in milestone
  202's shape: give a component the MSI-X table page, have it aim an interrupt at a vector it was
  never granted, and assert the hardware refuses. That needs a driver outside the kernel that wants
  interrupts, which is §86's question and not this one.
- **`NIFE_GIC=3` produces a boot that is known-broken, on purpose.** It exists so milestone 227
  starts from a reproduction rather than a rediscovery. It is not a supported configuration, no
  gate runs it, and the runner warns on every use. If it ever stops failing, something in the GIC
  path changed and this block is stale.
- **`NIFE_INTREMAP` and `NIFE_GIC` are provisional names.** Names are calef's
  (`AGENTS.md`); these follow the existing `NIFE_*` runner-knob convention (`NIFE_SMP`, `NIFE_CPU`,
  `NIFE_EL2`, `NIFE_DISK`) and nothing outside this repository has acted on either.
- **The x86_64 measurement is one machine.** QEMU 11.1.1, TCG, macOS on Apple Silicon, no KVM. The
  `kernel-irqchip` finding above is specifically a statement about that machine, and the CI Linux
  runners have not been checked.
- **`ECAP.IR` is a capability bit, not a working unit.** It says QEMU's model advertises interrupt
  remapping. Whether that model's remapping behaves like silicon's is a question no boot here asks,
  and milestone 87's `OptiPlex` (xenon) is where it would get a second witness.
