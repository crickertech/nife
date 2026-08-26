# 176. The x86_64 discovery seam's wide half: COM1's IRQ and a CMOS RTC

**Status: PARTIAL.** Minted 2026-08-25, from milestone 161's own item 0, "the wide half is
still owed and should be its own milestone." Re-scoped fresh against the current tree rather than
carried over from 161's own text, which turned out to describe a partly stale state. Piece 1 built
the same day (below); piece 2 was sized and turned out to need a decision this lane could not make,
so it is unbuilt and the finding is written up in full under "Piece 2" below.

**Gate: NONE.** Piece 1 is done. Piece 2's design fork is resolved:
[DECISIONS §130](../decisions/130-cmos-rtc-delegation.md), decided 2026-08-26 ("Ratify option 3").
Piece 2 is unbuilt but unblocked; whoever picks it up builds against §130 directly.

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

## Built: piece 1, COM1's IRQ wired, 2026-08-25

`Acpi::isa_irqs` already resolved all sixteen legacy ISA IRQs at boot; `isa_irqs[4]` is the console
UART's line. `kernel/src/main.rs`'s boot tour now reads it (`acpi.isa_irqs[4].gsi`) and hands it to
a new `memory::record_uart_irq`, the same-shaped setter `memory::record_pci_regions` already was for
PCI: it fills `memory::UART_IRQ`, the exact static the other two architectures fill from their
device tree at `memory.rs` line 135. `user::uart_irq_and_source()` (already generic across all three
architectures) now reports a real answer on `x86_64` instead of always falling back to the
`UART_RX_INTID` constant; its "device tree" source string, and `memory::uart_irq()`'s matching doc
comment, were generalized to "machine description" since ACPI is now a second source feeding the
same seam. Nothing calls it yet (the x86 console is polled, as the milestone that raised this item
already said), so this is exactly what it was scoped as: filling a seam for a future reader to find
real, rather than routing an interrupt.

`kernel/src/arch/x86_64/machine.rs`'s stale `BUGS` comment was also corrected: PCI has been wired
since milestone 165 and the interrupt controller since milestone 161 item 2, neither is `None`
today, and the `BUGS` section now names only the CMOS RTC (piece 2, below) as the device window
with no seam at all.

## Piece 2: a CMOS RTC, decided, not yet built

Sizing this found a real design fork, not a shape to build: the other two architectures' RTCs are
memory-mapped, so their `RTC_REGION` seam is a physical page a driver maps and pokes directly. The
PC-compatible CMOS RTC is two fixed I/O ports, and [DECISIONS §121](../decisions/121-port-io-capability.md)'s
current (still-PROPOSED) recommendation keeps all x86 legacy port I/O kernel-resident, so the
existing "map the device, let userspace drive it" pattern has no CMOS equivalent today. Full writeup,
four options considered, and the decision itself now live in
[DECISIONS §130](../decisions/130-cmos-rtc-delegation.md): **option 3**, the kernel reads CMOS once
at boot and hands the seed to the clock service as a `Spawn` argument, the same way `kind` already
crosses that boundary.

This blocks real functionality today, not just a test count: `clock_service::start` already spawns
with `kind = clock_proto::rtc::NONE` on `x86_64`, so `date_tests.rs`, `time_tests.rs`,
`clock_tests.rs` and `ntp_tests.rs` all skip via `clock_service::machine_has_no_rtc()`. §130 details
why.

**What is left:** building against §130. A new `clock_proto::rtc` kind (or equivalent `Spawn`
argument), a boot-time CMOS read in the kernel, and wiring `user/src/clock.rs` to use the handed-in
seed instead of polling a mapped register on this one architecture. Piece 1 is complete and
independent of this.
