# 159. A real hardware entropy source: the JH7110's TRNG

**Status: NOT-STARTED.** Minted 2026-08-23, surfaced while investigating milestone 49's boot-wiring
fork (DECISIONS §120): the entropy service (milestone 56, `BUILT`) only has a virtio-rng backend,
which exists in QEMU and not on the VisionFive 2 the tree already boots (milestone 16a). Checked
before minting: the StarFive JH7110's TRNG is already named as the real-hardware candidate in two
places (`notes/entropy.md`, milestone 56's own doc) and tracked by no milestone in either.

**Gate: HARDWARE.** In milestone 53's sense: the board is on the desk and this needs hands on it.
The driver's own logic can be written and host-tested without silicon; whether it actually produces
usable entropy, and at what rate, can only be verified by reading real bits off a real TRNG.

## Why this is the same shape of gap as 53 and 157

Milestones 53 (network/storage on real silicon) and 157 (display via U-Boot's framebuffer handoff)
both exist because virtio only carries an emulator's paravirtual devices, and real hardware has
none. Entropy is the same story one subsystem over: `entropy_service` (milestone 56) speaks to a
`Virtio` capability today, proven end to end in QEMU, and has never run against real hardware
because nothing here has a driver for the JH7110's TRNG.

## What it needs

- **Confirm the TRNG actually exists and is reachable on this board**, rather than assume the
  candidate is real. `notes/entropy.md` names it as a candidate, not as verified; check the JH7110
  datasheet and device tree before writing a driver against an assumption.
- **A driver, not a new protocol.** `entropy_service`'s own contract with its clients does not
  change; this is a new backend behind the existing service, the same relationship milestone 157's
  framebuffer driver has to rung one's existing `gfx_proto` contract. Rule 2 applies: it takes a
  base address and knows nothing else.
- **A rate and quality argument, measured rather than assumed.** A TRNG's output rate and whether
  it needs conditioning (health tests, whitening) before it is fit to seed anything security-shaped
  is a real question `notes/entropy.md` already flags as open in general ("no hardware TRNG yet");
  this milestone is where that stops being hypothetical.

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
