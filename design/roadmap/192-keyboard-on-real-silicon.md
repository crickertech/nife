# 192. A keyboard on real silicon: the input half of every graphical story, which nothing owns

**Status: NOT-STARTED.** Minted 2026-08-30, discovered while tracing journey 3 (login to `kilo` on
real silicon) the way journey 1's trace discovered milestone 177 (wire the graphical terminal stack
into the real interactive boot). *(Number provisional until the merge queue lands it.)*

**Gate: DECISION.** The fork is in "what a keystroke arrives on", below, and it is calef's: one
option is weeks of driver work on three architectures and the other changes what the story means.

**In brief.** Every graphical story this project tells assumes a keyboard, and on real hardware there
is no way to press a key. Milestone 29 (the framebuffer contract, bitmap font, VT engine and virtio
keyboard driver) gets keystrokes from **virtio-input**, which is a QEMU device. None of the three
boards has one. **A search of the entire roadmap for `usb`, `xhci`, `hid` and `ps/2` returns
nothing**, so this is not a milestone that exists and is unscheduled; it is work nobody has written
down at all.

## Why it went unnoticed for so long

Because every consumer of input so far has been either a test harness or a serial console, and both
work. The plain UART `console`/`input` pair the real interactive boot spawns today reads keystrokes
over the UART, which is real on all three boards. Nothing broke, so nothing asked.

It only becomes visible when the story is *a person sitting at the machine*, which is what journey 3
is and what journey 1 is a QEMU-shaped rehearsal for.

## The fork: what a keystroke arrives on

**Option A: serial input, framebuffer output.** Display on the board's own framebuffer (milestone
157, U-Boot's `simple-framebuffer` handoff), keystrokes over the existing UART. On the OptiPlex this
is already wired: milestone 87 (the x86_64 bare-metal machine) bought a Dell C4PDJ serial module and
an RS-232 chain precisely so a host can drive the box. Zero new drivers. The cost is that the story
becomes "a person at a laptop typing into a machine across a null-modem cable", which is a real
demonstration of a real system and is **not** the story journey 3's title claims.

**Option B: a USB HID stack.** An xHCI driver, enough USB core to enumerate and configure a device,
and a HID keyboard driver, on three architectures. This is the honest version of "sit down at the
machine", and it is large: xHCI alone is a substantial driver, and it lands in the most
attacker-adjacent position in the system (a bus that enumerates whatever a stranger plugs in), which
is either a problem or the best confinement demonstration available depending on how it is built.
Note the peer project Atom keeps xHCI **in the kernel**, which is exactly the contrast DECISIONS §31
(the foreign-language seam) already draws for FAT32.

**Option C: split the difference by architecture.** Serial input on the boards, USB only where it is
cheapest first. Rejected on sight rather than costed, because DECISIONS §19 (architectural parity is
a gate) exists to stop precisely this, and an input path that differs per ISA would make "it works on
aarch64" stop predicting anything. Recorded so the option is refused rather than forgotten.

**No recommendation is offered**, deliberately. AGENTS.md's rule is to recommend on reversible forks
and give options on irreversible ones, and this one sets the bar for what every graphical milestone
in the project is claiming. It is also a scoping decision about how much of the project's remaining
capacity goes into a bus driver.

## What is true either way

- **The framebuffer half is milestone 157** and is unaffected by this fork.
- **Option A is a strictly smaller superset of nothing**: it needs no new milestone at all, which is
  why it is worth pricing honestly against B rather than dismissing.
- **If B is chosen, it is not one milestone.** xHCI, USB core, and HID are three, and the scoping
  pass is the first piece of work.

## BUGS

- **This block prices neither option.** The xHCI estimate is an adjective, not a measurement, and
  nobody has looked at what the JH7110, the Jetson TX1 and the OptiPlex each expose.
- **It says nothing about a mouse**, which milestone 33's compositor will eventually want and which
  the same stack would carry.
- **The OptiPlex may or may not have a PS/2 port.** A 7050 Micro's front and rear IO has not been
  checked against this question, and if it has one, the x86_64 leg of option B gets dramatically
  cheaper while the other two do not. That is a two-minute check nobody has done.
