# 176. The x86_64 discovery seam's wide half: COM1's IRQ and a CMOS RTC

**Status: PARTIAL.** Minted 2026-08-25, from milestone 161's own item 0, "the wide half is
still owed and should be its own milestone." Re-scoped fresh against the current tree rather than
carried over from 161's own text, which turned out to describe a partly stale state. Piece 1 built
the same day (below); piece 2 was sized and turned out to need a decision this lane could not make,
so it is unbuilt and the finding is written up in full under "Piece 2" below.

**Gate: DECISION.** Piece 1 needed none, and is done. All that is left is piece 2, and it is
blocked on the design fork "Piece 2" describes: how a kernel-resident-only device (DECISIONS §121)
hands its reading to the userspace clock service, whose whole existing shape assumes the service
reads its device itself.

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

## Piece 2: a CMOS RTC, sized and found to need a decision, not built

**Sizing this found a real design fork, not a shape to build.** The other two architectures'
`RTC_REGION` is `Option<(u64, u64, u64)>` because their RTCs (`pl031`, `goldfish-rtc`) are
memory-mapped: a driver reaches them through a page. The PC-compatible CMOS RTC is two fixed I/O
ports (0x70 index, 0x71 data), which [DECISIONS §121](../decisions/121-port-io-capability.md)
already settled, permanently, stays kernel-resident: no port-range capability will ever exist, on
this architecture, for any legacy device including this one. That much is decided and this milestone
does not reopen it.

**What §121 does not settle, and what actually blocks this piece:** every consumer of an RTC in this
tree assumes the *userspace* clock service reads the device itself. `kernel/src/user/clock_service.rs`
maps the RTC's register page (`Flags::user_device()`) straight into the service process, and
`user/src/clock.rs` (`read_rtc`, dispatching on `clock_proto::rtc::{PL031, GOLDFISH}`) is the driver,
running unprivileged, that pokes those registers directly. §121 forecloses that exact shape for CMOS:
no capability can ever name ports 0x70/0x71, so the userspace clock service **cannot** be the CMOS
driver under any grant this system will ever issue. The kernel has to be the one that reads CMOS, and
nothing in this tree has an existing seam for "the kernel read a fact and now a userspace *service*
process (not a device it maps) needs to receive it," which is a different shape than `record_uart_irq`
above: that one publishes a fact other kernel code reads, this one has to cross into a process that
is already running with capabilities decided at spawn time.

**This blocks real functionality today, not just a test count.** `clock_service::start` already
branches on `memory::rtc_region()`: `None` maps zero device pages and spawns the service with
`kind = clock_proto::rtc::NONE`, so **the clock service has no working wall clock on `x86_64` at
all**, which is why `date_tests.rs`, `time_tests.rs`, `clock_tests.rs` and `ntp_tests.rs` all skip
via `clock_service::machine_has_no_rtc()`. This is the customer path (a backup server has to know
what time it is), not a skip-count cosmetic; re-measuring confirmed the four files and did not
change the shape of the finding.

### What was considered, and why each option needs calef rather than a lane

1. **Repurpose `RTC_REGION`'s `(u64, u64, u64)` to carry `(0x70, 0x71, CMOS-kind)` instead of an
   address and size.** Rejected outright, not merely disfavored: `clock_service.rs` maps `rtc.0` as
   a **physical page address** (`Mapping { va: RTC_VA, phys, flags: user_device() }`).
   Pouring port numbers into that field would have the mapper build a device mapping of physical
   page `0x70`, which is real memory near the start of RAM. That is a correctness bug waiting to
   happen, not an implementation detail to smooth over.
2. **A port-range capability**, so the userspace service maps CMOS the way it maps a PL031.
   Foreclosed permanently by §121 itself (its own "option 1", declined for the console on
   cost grounds that apply here too, if anything harder to justify for a boot-time RTC read than for
   a UART).
3. **The kernel reads CMOS once (a dozen or so `in8`/`out8` pairs, same shape as
   `arch::x86_64::timer`'s PIT calibration, sub-microsecond, no measurement needed to know it is
   cheap) and hands the wall-clock seed to the clock service the same way `kind` already crosses
   that boundary today**: as a plain `Spawn` argument, read by `user/src/clock.rs` as data instead
   of by polling a mapped register. This is the cheapest shape found and the one real operating
   systems use (Linux reads the RTC once at boot in the kernel and runs off a monotonic clock
   after). **It is still a decision, not a mechanical fill-in**, for two reasons: it makes the
   *kernel* a writer of the clock's initial value where today only the userspace service ever is
   (the page is documented at the mapping site as `// read/WRITE: the service is a setter`, singular),
   and it needs a new `clock_proto::rtc` kind (or an equivalent protocol addition) that
   `user/src/clock.rs` and `clock_service.rs` (two programs) have to agree on. `clock_proto` is
   exactly "anything two programs agree on" (this file's own tenets, and CLAUDE.md's): cheap to
   prototype, expensive to un-ship once a real `date` and NTP path depend on what the new kind means.
4. **A kernel-mediated IPC broker, queried on demand** (§121's own "option 3" for the console,
   priced there in general terms: ~337 ns per IPC round trip against a ~27 ns null syscall). Cheap
   enough for a boot-time query, but it is a second new mechanism next to option 3 above for no
   shown benefit over it: a value that changes once a boot has no reason to be re-queried instead of
   handed over once.

**Recommendation, not a decision: option 3.** No new capability type, no new syscall, matches real
prior art, and the cost is genuinely just the protocol question, not an implementation cost being
dressed up as one (CLAUDE.md's "would I still choose this if both options were the same amount of
work" test: yes, option 3 is also the least code). But it changes a wire format two programs already
agree on and changes who is trusted to set the clock's first value on one architecture, which are
both calef's calls under this tree's own "move fast on what can be undone" tenet, not a lane's.

**What is blocked until it is answered:** all of piece 2. Piece 1 is complete and independent of
this; nothing about it depends on how piece 2 resolves.
