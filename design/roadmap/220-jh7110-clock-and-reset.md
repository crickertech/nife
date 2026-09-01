# 220. This kernel drives no clock or reset controller, and the first real device will need one

**Status: NOT-STARTED.** Minted 2026-09-01 by the maintainer, from milestone 159's (a real hardware
entropy source: the JH7110's TRNG) lane, which named it as the most likely reason its own first
bench boot will fail. *(Number provisional until the merge queue lands it.)*

**Gate: HARDWARE.** In the sense that means the board is on the desk: the driver can be written
without radon, but whether it is needed at all can only be settled by watching a real device answer
or fail to answer.

**In brief.** nife has never programmed a clock or a reset line. Every device it has driven came up
already running: QEMU's virtio devices, the PL011 and NS16550 consoles, the PLIC. Real SoC
peripherals do not, and the JH7110 is the first case in reach.

Linux's own JH7110 TRNG driver takes **two clocks and a reset line** before it touches a register.
nife's driver takes neither, because nothing in this tree can supply them. So the most likely
outcome of milestone 159's first bench boot is not a driver bug: it is a register window that reads
as nothing because the device is held in reset or its clock is gated.

## Why it is minted before it is known to be needed

Milestone 159's lane deliberately did **not** build this on spec, which was right: a driver written
against a datasheet for a device nobody has seen answer is exactly the shape this project's
`HARDWARE` gate exists to defer. What it did instead is make the question answerable in one boot.
Its `hw entropy` line carries a raw `(STAT << 32) | ISTAT` bring-up diagnostic, and that number
separates the two failures a bench session otherwise cannot tell apart:

- **All zeros**: the register window read as nothing. Gated clock, undeasserted reset, or a wrong
  base address. **That is this milestone.**
- **Anything else**: the device answered and the sequence is wrong. That is milestone 159's.

So this block exists so that a single observation at the bench routes to a milestone rather than to
a person's memory of a conversation.

## What it would need

A clock and reset generator driver for the JH7110, and a decision this block does not make about
**where such a thing lives in a capability system**. That is the interesting question and it is why
this is not simply a small driver:

- A clock controller is a shared resource that many drivers need, so it is a service rather than a
  library, and every device driver becomes its client.
- It is also authority: whoever can gate a clock can stop a device, and whoever can assert a reset
  can interrupt one mid-transaction. That is a capability worth naming carefully.
- The alternative is that the kernel does it during bring-up and no userspace program ever holds
  that authority, which is simpler and less general, and may be right for a demonstrator.

## BUGS

- **Nobody has confirmed this is needed.** The entire premise is an inference from Linux's driver
  plus the absence of any clock code here. One boot settles it, and until that boot this block is a
  prediction.
- **It says nothing about argon or xenon**, which have their own bring-up assumptions that nothing
  has tested either.
- **Scope is unbounded as written.** "A clock and reset driver for the JH7110" could mean the one
  clock the TRNG needs or the whole controller, and those differ by an order of magnitude.
