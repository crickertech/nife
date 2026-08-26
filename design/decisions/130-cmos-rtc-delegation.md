# 130. How the kernel-resident CMOS RTC reaches the userspace clock service

**Status: PROPOSED.** [Milestone 176](../roadmap/176-x86-64-discovery-seam-wide-half.md)'s own
lane sized this while building the milestone's second piece and found a real design fork rather
than a shape to build, correctly declining to invent an answer. This entry lays out what it found,
verbatim from its own research, for that call.

## What's decided already, and does not reopen here

[DECISIONS §121](121-port-io-capability.md) settled, permanently, that x86 legacy port I/O
(including the CMOS clock) stays kernel-resident: no port-range capability will ever exist on this
architecture for any legacy device. That is not revisited.

## What §121 does not settle, and what actually blocks this

Every RTC consumer in this tree assumes the *userspace* clock service reads its device directly.
`kernel/src/user/clock_service.rs` maps the RTC's register page (`Flags::user_device()`) straight
into the service process, and `user/src/clock.rs` (`read_rtc`, dispatching on
`clock_proto::rtc::{PL031, GOLDFISH}`) is the driver, running unprivileged, that pokes those
registers itself. §121 forecloses that exact shape for CMOS: no capability can ever name ports
0x70/0x71, so the userspace clock service **cannot** be the CMOS driver under any grant this system
will ever issue. The kernel has to be the one that reads CMOS, and nothing in this tree has an
existing seam for "the kernel read a fact and a userspace *service process* (not a device it maps)
needs to receive it," a different shape than a plain fact-publishing static like `memory::UART_IRQ`:
this one has to cross into a process already running with capabilities decided at spawn time.

## This blocks real functionality today, not a test count

`clock_service::start` already branches on `memory::rtc_region()`: `None` maps zero device pages
and spawns the service with `kind = clock_proto::rtc::NONE`
(`kernel/src/user/clock_service.rs:38-68`), so **the clock service has no working wall clock on
x86_64 at all**, which is why `date_tests.rs`, `time_tests.rs`, `clock_tests.rs`, and
`ntp_tests.rs` all skip via `clock_service::machine_has_no_rtc()`. This is the customer path (a
backup server has to know what time it is), not a cosmetic skip count.

## What was considered

1. **Repurpose `RTC_REGION`'s `(u64, u64, u64)` to carry `(0x70, 0x71, CMOS-kind)` instead of an
   address and size.** Rejected outright, not merely disfavored: `clock_service.rs` maps `rtc.0` as
   a physical page address. Pouring port numbers into that field would have the mapper build a
   device mapping of physical page `0x70`, which is real memory near the start of RAM. A
   correctness bug waiting to happen, not an implementation detail to smooth over.
2. **A port-range capability**, so the userspace service maps CMOS the way it maps a PL031.
   Foreclosed permanently by §121 itself (its own "option 1," declined for the console on cost
   grounds that apply here too, if anything harder to justify for a boot-time RTC read than for a
   UART).
3. **The kernel reads CMOS once** (a dozen or so `in8`/`out8` pairs, the same shape as
   `arch::x86_64::timer`'s PIT calibration, sub-microsecond, no measurement needed to know it is
   cheap) **and hands the wall-clock seed to the clock service the way `kind` already crosses that
   boundary today**: as a plain `Spawn` argument, read by `user/src/clock.rs` as data instead of by
   polling a mapped register. The cheapest shape found, and the one real operating systems use
   (Linux reads the RTC once at boot in the kernel and runs off a monotonic clock after). Still a
   real decision, for two reasons: it makes the *kernel* a writer of the clock's initial value
   where today only the userspace service ever is (the mapping site's own comment: `// read/WRITE:
   the service is a setter`, singular), and it needs a new `clock_proto::rtc` kind (or an
   equivalent protocol addition) that `user/src/clock.rs` and `clock_service.rs`, two programs,
   have to agree on.
4. **A kernel-mediated IPC broker, queried on demand** (§121's own "option 3" for the console,
   priced there in general terms: ~337 ns per IPC round trip against a ~27 ns null syscall). Cheap
   enough for a boot-time query, but a second new mechanism next to option 3 for no shown benefit
   over it: a value that changes once a boot has no reason to be re-queried instead of handed over
   once.

## Recommendation

**Option 3.** No new capability type, no new syscall, matches real prior art, and the cost is
genuinely just the protocol question, not an implementation cost dressed up as one (this tree's own
"would I still choose this if both options were the same amount of work" test: yes, option 3 is
also the least code of the four). But it changes a wire format two programs already agree on and
changes who is trusted to set the clock's first value on one architecture, both calef's calls under
this tree's own "move fast on what can be undone" tenet, not a lane's.

## What is blocked until this is answered

All of milestone 176's piece 2. Piece 1 (COM1's IRQ) is complete and independent of this.
