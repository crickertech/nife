# 242. USB host and HID, because on commodity hardware the keyboard is not a UART

**Status: NOT-STARTED.** Minted 2026-09-03 by calef, from asking how nife reaches hardware that has
no serial port. *(Number provisional until the merge queue lands it.)*

**Gate: NONE.** Nothing external blocks it. It is large, and it is currently hiding inside another
milestone's word.

**In brief.** Milestone 192 (a keyboard on real silicon: the input half of every graphical story,
which nothing owns) is `PARTIAL`. Its option A landed on 2026-09-02 and did the structural half well:
the keystroke **source** is now one `match` in the kernel, the board's UART and virtio-input are
interchangeable, and `crates/system_initializer` cannot tell which it got. **Option B is a third arm
of that match and nothing else in the guest is downstream of the choice.**

**But that third arm is USB, and nothing owns it.** The string `xhci` appears in exactly one roadmap
block, 192's, and only as a word. There is no host-controller milestone, no HID stack, no hub
enumeration, no transfer-ring code. **On commodity hardware the keyboard is USB**, so option B is a
large unbuilt subsystem wearing the word "option", and this block exists to stop it being priced as a
line item in someone else's plan.

calef's own bar for 192, set the hour it was minted: *"192 isn't done until we can sit down at the
keyboard connected to the computer and display the OS on a monitor plugged into the machine."* That
sentence is this milestone.

## Why it is on the customer path rather than the risk path

AGENTS.md records that **the customer path is vacant** as of 2026-08-30, and its own guidance is that
a first customer should be something nife can plausibly be adequate at within a milestone or two.
**"Runs on a machine someone already owns" is the most obvious thing that would fill that path**, and
a machine someone already owns has USB and no serial header.

It is also the precondition AGENTS.md attaches to the ranking function: no third party sees this
until there is a package manager and a trivial install process, and an install process assumes a
keyboard.

## What it needs, and what makes it big

- **A host controller.** xHCI is the one commodity hardware has, and it is a ring-based DMA interface
  rather than a register poke: command ring, event ring, transfer rings per endpoint, a device
  context base array. **This is the first driver in the tree whose data structures the device walks
  on its own.**
- **Enumeration.** Reset, address assignment, descriptor reads, configuration selection, and hubs,
  because a keyboard is often behind one.
- **HID.** Boot protocol is the small mercy here: a keyboard in boot protocol sends an eight-byte
  report with a fixed layout, which is far less than a full HID report-descriptor parser.
- **And the confinement question, which is the interesting part.** Milestone 159's (a real hardware
  entropy source: the JH7110's TRNG) driver needed one device page and two endpoints. **A DMA-driven
  controller needs memory the device writes into**, so this is the first real test of what
  `crates/dma_validator` and the IOMMU work are for, on a device a person can unplug.

## BUGS

- **This block does not price it in hours**, and should not: xHCI is the largest single driver this
  project would have written, and the honest first step is a survey rather than an estimate.
- **Boot protocol may not survive contact with real keyboards.** Some report protocol only, some
  need a `SET_PROTOCOL` they then ignore, and the ones that misbehave are exactly the cheap ones a
  stranger owns.
- **It says nothing about USB storage, hubs beyond the first, or anything but a keyboard**, and
  should not: the deliverable is a keystroke, not a USB stack.
- **Nothing here helps a headless machine**, which is the other half of the same question and is
  milestone 243 (a machine with no serial port has no way to say anything).
