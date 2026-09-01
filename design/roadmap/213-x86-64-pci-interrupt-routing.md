# 213. A PCI function's interrupt reaches nothing on x86_64, so no userspace driver can run there

**Status: NOT-STARTED.** Minted 2026-09-01 by milestone 164's lane, which built the disk wiring,
ran it, watched it wedge, and reverted it. **Provisional number**: the integrator mints it at merge
like every other name global to the tree.

**Gate: NONE.** QEMU's `q35` reproduces all of it.

## What it needs

`kernel/src/arch/x86_64/irq.rs` has an IO APIC driver, a local APIC, and `enable(intid)` that
programs a redirection entry. What it does not have is the **mapping from a PCI function's INTx pin
to a global system interrupt**, and `arch::x86_64::mmu::PCI_IRQ_BASE` says so in as many words: it
is `0`, and its own comment calls that "the marker for that rather than a value to trust", because
on `q35` a function's legacy interrupt goes through the PIRQ router to an IO APIC input the ACPI
`_PRT` names, and MSI/MSI-X bypasses the routing entirely.

The consequence is not a missing feature, it is a **wrong answer that looks like a right one**.
`pci::intx_irq(base, dev, pin)` is `base + ((dev + pin - 1) % 4)`, so with `base = 0` and the
virtio-blk function at device 4 pin 1 it returns `0`, and `arch::x86_64::irq::enable(0)` resolves
that through `isa_routing` to the **PIT's** line. Measured in milestone 164's lane: the confined
userspace block server was armed on the timer, QEMU said `Interrupt Mask set, irq is not
generated`, and the suite wedged with no verdict.

A second symptom arrived with it and may or may not be the same bug:

```
qemu-system-x86_64: vtd_iommu_translate: detected translation failure (dev=00:04:00, iova=0xffde000)
```

VT-d confinement (milestone 161) is real and `kernel/src/arch/x86_64/iommu.rs` implements `attach`,
so `virtio::register`'s `iommu::confine` should have covered the device's DMA region. Whether the
fault is a genuine confinement gap or a consequence of a driver that never got its interrupt and so
never finished bring-up **was not determined**, and finding out is the first thing this milestone
should do, because the answer decides whether it is one piece of work or two.

## Why it matters

**It is the last thing between the x86_64 leg and a filesystem.** Milestone 164 put
`redoxfs_server` and `mkfs` in the x86_64 archive; 36 tests still take their "no RedoxFS disk
attached" arm because the disk cannot be driven. That is the same 21-of-67 debt PR #476 opened,
moved one wall over and now bounded.

**It generalises past the disk.** Every skipped x86_64 test whose reason names a bus (the
virtio-gpu ones, the virtio-rng ones, the virtio-input one) is waiting on the same thing: a device
this machine's runner could attach and this kernel could route an interrupt from. This is the piece
that turns the x86_64 port from a kernel into a system, and it is what milestone 87's OptiPlex boot
will want under it.

## Two routes, sized rather than guessed

Neither is chosen here, and picking between them wants a measurement this block did not take.

**Route A: legacy INTx.** Program the ICH9 LPC bridge's PIRQ routing registers and give
`PCI_IRQ_BASE` a real value, or read the routing out of ACPI. The cheap version hardcodes what
`q35` does; the honest version reads `_PRT`, which is AML, which this tree has no interpreter for
and should not grow one for this. **The risk is a hardcoded swizzle that happens to match QEMU and
does not match the OptiPlex**, which is precisely the class of thing milestone 87 exists to catch,
and catching it there rather than here would be expensive.

**Route B: MSI-X.** Parse the MSI-X capability, allocate a vector, write the message address and
data, and skip the routing question entirely: the device writes the vector straight to the local
APIC. More code than route A's cheap version, less than its honest version, and **no board-specific
table to be wrong about**, which is the property that matters for milestone 87. Every device this
port would attach (virtio-pci, NVMe) supports it. It is also the direction real x86 systems went,
so it is not novelty.

Route B looks better on the evidence available, and this block deliberately does not decide:
`DECISIONS.md` §19's parity rule means whatever is chosen here is what the aarch64 and riscv64 legs
will be compared against, and the two other architectures reach their PCI devices through INTx
today.

## What this does not cover

The fixtures. An x86_64 runner that can route a PCI interrupt still attaches only the images
somebody adds to `scripts/qemu-runner-x86_64.sh`; milestone 164's lane wrote and reverted the
two-disk version of that block, and it is a few lines once the interrupt works. The roster the
ordinals index is documented at `virtio::find_block_device_n` and in the other two runners.
