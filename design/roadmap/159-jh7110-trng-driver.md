# 159. A real hardware entropy source: the JH7110's TRNG

**Status: NOT-STARTED.** Minted 2026-08-23, surfaced while investigating milestone 49's boot-wiring
fork (DECISIONS §120): the entropy service (milestone 56, `BUILT`) only has a virtio-rng backend,
which exists in QEMU and not on the VisionFive 2 the tree already boots (milestone 16a). Checked
before minting: the StarFive JH7110's TRNG is already named as the real-hardware candidate in two
places (`notes/entropy.md`, milestone 56's own doc) and tracked by no milestone in either.

**Gate: HARDWARE.** In milestone 53's sense: the board is on the desk and this needs hands on it.
The driver's own logic can be written and host-tested without silicon; whether it actually produces
usable entropy, and at what rate, can only be verified by reading real bits off a real TRNG.

**Status deliberately did not move to `PARTIAL` on 2026-08-24, and this paragraph is that lane's
report of why**, per `design/roadmap/README.md`'s own rule that a branch touching nothing in this
file must at least say so. A lane confirmed the TRNG (see below), wrote and host-tested its register
and DTB-discovery logic (`crates/jh7110_trng`), and wrote an unwired driver program
(`user/src/jh7110_trng.rs`) that builds and type-checks against `entropy_proto` on every target this
tree builds for. **None of that clears this tree's own bar for `PARTIAL`**: milestone 53's `PARTIAL`
names a phase that runs end to end, proven in QEMU; nothing here has run against anything, emulated
or real, because there is no JH7110 TRNG in QEMU to run it against. `crates/jh7110_trng`'s tests are
pure logic and a DTB-fixture check, on the host, with no device on the other end. `NOT-STARTED`
("specified, nothing built") is not quite right either, since a driver's worth of buildable, cited
code now exists; it is the closer of the two available tokens to the truth, and the honest gap is
recorded here rather than folded into a status word this vocabulary does not have. See
`crates/jh7110_trng/src/lib.rs`'s own module doc ("This has not run against real silicon") and
`user/src/jh7110_trng.rs`'s ("This is not wired to anything, and has never run") for the same
caveat, closer to the code.

## Why this is the same shape of gap as 53 and 157

Milestones 53 (network/storage on real silicon) and 157 (display via U-Boot's framebuffer handoff)
both exist because virtio only carries an emulator's paravirtual devices, and real hardware has
none. Entropy is the same story one subsystem over: `entropy_service` (milestone 56) speaks to a
`Virtio` capability today, proven end to end in QEMU, and has never run against real hardware
because nothing here has a driver for the JH7110's TRNG.

## What it needs

- **Confirm the TRNG actually exists and is reachable on this board**, **done from documentation,
  2026-08-24, not from hardware.** The JH7110 datasheet (v1.67) §2.8.2 documents a TRNG module
  ("Ring-oscillator based entropy source... LFSR based digital post process... self re-seeding...
  256-bit random number generation") and Linux carries a shipped, mainline driver
  (`drivers/char/hw_random/jh7110-trng.c`) and device-tree binding
  (`starfive,jh7110-trng`, `reg = <0x1600C000 0x4000>`, `interrupts = <30>`, clocks `hclk`/`ahb`,
  one reset line) for it. `crates/jh7110_trng` transcribes the register layout from the Linux driver
  and proves, on the host, that its DTB-discovery query finds a tree shaped like the binding's own
  example and correctly finds nothing on QEMU's riscv64 `virt` board (which has no such node). What
  remains unconfirmed is whether the VisionFive 2's own shipped device tree actually carries this
  node the way mainline's does; nobody has captured one from the board to check.
- **A driver, not a new protocol.** `entropy_service`'s own contract with its clients does not
  change; this is a new backend behind the existing service, the same relationship milestone 157's
  framebuffer driver has to rung one's existing `gfx_proto` contract. Rule 2 applies: it takes a
  base address and knows nothing else. `user/src/jh7110_trng.rs` is that backend, speaking
  `entropy_proto` unchanged, built and type-checked but **not wired to `entropy_service`'s `Bus`
  enum and not spawned by anything**; see its own module doc for why wiring it in was left for
  whoever has the board.
- **A rate and quality argument, measured rather than assumed.** Still open, and now sharper rather
  than answered. The datasheet documents exactly one hardware fault signal
  (`ISTAT.LFSR_LOCKUP`, an SEU in the post-processing stage), which is cheap to read and this
  driver reads it; **neither the datasheet nor the Linux driver document anything resembling a NIST
  SP 800-90B health test or a compliance claim (no FIPS, no AIS-31) over the raw bitstream.**
  Whether this tree needs one before trusting these bytes for anything security-shaped is a real
  design question the documentation does not resolve. It is **not decided by this lane** (a
  developer does not edit `design/decisions/`); see `crates/jh7110_trng/src/lib.rs`'s "Health
  testing" section for the full argument, and treat this as a candidate for a PROPOSED entry in
  `design/decisions/` if calef wants the question tracked formally rather than left in this
  paragraph and the crate's own doc.

## What this does not decide

**DECISIONS §120's stopgap question is separate and not reopened by this milestone.** Whether the
interactive boot should grant *any* entropy source (virtio-rng or this driver) before there is a
real customer for interactive login is §120's call, declined for now; building this driver does not
require revisiting that, and landing this driver does not by itself grant it to the boot path.

## What it unblocks

The real-hardware half of milestone 56's own claim: an entropy service this tree can trust on the
board it actually ships to, not only in QEMU. Downstream of that, whenever §120's stopgap question
is revisited with a real customer, the answer can be "real hardware entropy," not only "QEMU's
virtio-rng."
