# 176. The x86_64 discovery seam's wide half: COM1's IRQ and a CMOS RTC

**Status: NOT-STARTED.** Minted 2026-08-25, from milestone 161's own item 0, "the wide half is
still owed and should be its own milestone." Re-scoped fresh against the current tree rather than
carried over from 161's own text, which turned out to describe a partly stale state.

**Gate: NONE.** Both pieces are mechanical against already-decided ground: [DECISIONS
§121](../decisions/121-port-io-capability.md) settled x86 legacy port I/O as permanently
kernel-resident, and COM1's IRQ is already discovered, just unconnected.

## What 161's own text got right, and what it no longer does

`kernel/src/arch/x86_64/machine.rs`'s own `BUGS` section lists four device windows as "still read
from a tree by the front end and stay `None` here": the interrupt controller, the RTC, the UART's
interrupt line, and the PCIe ECAM range. **Two of those four are stale claims, checked directly
against the tree this milestone was minted from:**

- **The interrupt controller (IO APIC) is built**, milestone 161's own item 2, reached directly by
  `arch::x86_64::irq` rather than through `memory.rs`'s statics.
- **PCI is wired**, and has been since milestone 165's ACPI/MCFG work: `kernel/src/main.rs`
  calls `memory::record_pci_regions` from the ACPI `ecam` path, so `memory::pci_regions()` returns
  `Some` on x86_64 today. `machine.rs`'s own comment ("still report nothing on x86 even though the
  MCFG answered a few lines earlier") predates this and needs correcting as part of this
  milestone's own cleanup, not left standing as a live claim.

**Two are real**, and are this milestone's actual scope:

## Piece 1: wire COM1's already-discovered IRQ

`Acpi::isa_irqs` already resolves all sixteen legacy ISA IRQs at boot; `isa_irqs[4]` is the console
UART's line (`machine.rs`'s own doc comment: "could be routed the way the PIT's is; the x86 console
is polled, so nothing asks"). This is connecting an existing value to `memory::UART_IRQ` (or an
equivalent seam), the same shape `memory.rs`'s device-tree front end already does for the other two
architectures at line 135. Small and mechanical: the discovery already happened, nothing about ACPI
or interrupt routing needs designing.

## Piece 2: a CMOS RTC, under a shape of its own

**This is not "wire it," and sizing it precisely is this milestone's real work.** The other two
architectures' `RTC_REGION` is `Option<(u64, u64, u64)>`: a physical address, a size, and a kind,
because their RTCs (`pl031`, `goldfish-rtc`) are memory-mapped devices a driver reaches through a
page. The PC-compatible CMOS RTC is not memory-mapped at all: it is two fixed I/O ports (0x70
index, 0x71 data), reached the same way every other x86 legacy device is, which
[DECISIONS §121](../decisions/121-port-io-capability.md) already settled stays kernel-resident
rather than becoming a page-shaped capability. `RTC_REGION`'s existing shape has nowhere to put
"two fixed ports and a protocol," so this needs either a second, port-shaped seam beside
`RTC_REGION` or a kernel-resident RTC service exposed some other way, not a value poured into the
existing cell. Whoever picks this up should read `kernel/src/arch/x86_64/port.rs`'s own module doc
(it already names the CMOS clock as one of the legacy devices living behind that seam) before
designing the shape, and should re-measure the current skip count directly
(`kernel::user::date_tests` and any other clock-adjacent test) rather than trust a number from
before piece 1 or the PCI correction landed.

## What this does not decide

Whether CMOS RTC access becomes a new kernel-level API, a syscall-adjacent seam, or something else;
`port.rs`'s own existing shape for legacy port I/O is the precedent to extend, not a decision this
milestone reopens.
