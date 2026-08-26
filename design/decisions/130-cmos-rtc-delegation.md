# 130. How the kernel-resident CMOS RTC reaches the userspace clock service

**Status: PROPOSED.** [Milestone 176](../roadmap/176-x86-64-discovery-seam-wide-half.md)'s own
lane sized this while building the milestone's second piece and found a real design fork rather
than a shape to build, correctly declining to invent an answer. This entry lays out what it found,
verbatim from its own research, for that call.

## What §121 leaves open, and does not reopen here

[DECISIONS §121](121-port-io-capability.md) is **PROPOSED, not decided**, with a recommendation its
own text calls "deliberately weak": x86 legacy port I/O (the CMOS clock among them) stays
kernel-resident **by default**, not permanently, because nothing in ring 3 needs it yet and the one
mechanism with real per-port granularity (a port-range capability enforced by the TSS I/O permission
bitmap, §121's own "option 1") now has a measured cost: `notes/benchmarks.md`'s 2026-08-24 entry
found it adds **~1.5-2.7 us to every context switch on the machine** (release build, +423% over a
bare switch), for a device class where a raw `in`/`out` costs single digit cycles. §121 says outright
that this is revisited "the moment a userspace console on x86 becomes a thing calef wants
demonstrated." This entry takes §121's current recommendation as given and does not re-argue it; if
§121 moves, this entry's options change with it.

## What actually blocks this today

Every RTC consumer in this tree assumes the *userspace* clock service reads its device directly.
`kernel/src/user/clock_service.rs` maps the RTC's register page (`Flags::user_device()`) straight
into the service process, and `user/src/clock.rs` (`read_rtc`, dispatching on
`clock_proto::rtc::{PL031, GOLDFISH}`) is the driver, running unprivileged, that pokes those
registers itself. Under §121's current recommendation, that shape is not available for CMOS: no
capability names ports 0x70/0x71 today, so the userspace clock service cannot be the CMOS driver
under any grant this system currently issues. The kernel has to be the one that reads CMOS, and
nothing in this tree has an existing seam for "the kernel read a fact and a userspace *service
process* (not a device it maps) needs to receive it," a different shape than a plain fact-publishing
static like `memory::UART_IRQ`: this one has to cross into a process already running with
capabilities decided at spawn time.

## This blocks real functionality today, not a test count

`clock_service::start` already branches on `memory::rtc_region()`: `None` maps zero device pages
and spawns the service with `kind = clock_proto::rtc::NONE`
(`kernel/src/user/clock_service.rs:38-68`), so **the clock service has no working wall clock on
x86_64 at all**, which is why `date_tests.rs`, `time_tests.rs`, `clock_tests.rs`, and
`ntp_tests.rs` all skip via `clock_service::machine_has_no_rtc()`. This is the customer path (a
backup server has to know what time it is), not a cosmetic skip count, and it holds regardless of
which way §121 eventually goes: even under option 1 there, wiring CMOS behind a port capability is
a separate, larger piece of work than this milestone scoped, so an interim answer is worth having
either way.

## What was considered

1. **Repurpose `RTC_REGION`'s `(u64, u64, u64)` to carry `(0x70, 0x71, CMOS-kind)` instead of an
   address and size.** Rejected outright, not merely disfavored: `clock_service.rs` maps `rtc.0` as
   a physical page address. Pouring port numbers into that field would have the mapper build a
   device mapping of physical page `0x70`, which is real memory near the start of RAM. A
   correctness bug waiting to happen, not an implementation detail to smooth over.
2. **A port-range capability**, so the userspace service maps CMOS the way it maps a PL031. This is
   §121's own "option 1," and would make x86 structurally match the other two architectures: a
   capability the driver holds, hardware-enforced, no kernel mediation per access. Declined under
   §121's *current* recommendation, on the same cost grounds §121 measured for the console (the
   ~1.5-2.7 us per-switch bitmap-write cost lands on every thread on the machine, not just this
   one), and harder to justify here than for a UART: a UART is polled continuously by a driver that
   might want to stay resident in ring 3; a boot-time RTC read happens once. If §121 ever moves to
   option 1 for the console, revisit this option too.
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

**Option 3, given §121's current recommendation.** No new capability type, no new syscall, matches
real prior art, and the cost is genuinely just the protocol question, not an implementation cost
dressed up as one (this tree's own "would I still choose this if both options were the same amount
of work" test: yes, option 3 is also the least code of the four). But it changes a wire format two
programs already agree on and changes who is trusted to set the clock's first value on one
architecture, both calef's calls under this tree's own "move fast on what can be undone" tenet, not
a lane's.

If §121 is later decided toward option 1 (the port-range capability), for the console or for any
other reason, option 2 above becomes available on the same terms and should be re-priced against
option 3 at that point; nothing here forecloses that.

## What is blocked until this is answered

All of milestone 176's piece 2. Piece 1 (COM1's IRQ) is complete and independent of this.
