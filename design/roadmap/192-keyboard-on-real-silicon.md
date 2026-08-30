# 192. A keyboard on real silicon: the input half of every graphical story, which nothing owns

**Status: NOT-STARTED.** Minted 2026-08-30, discovered while tracing journey 3 (login to `kilo` on
real silicon) the way journey 1's trace discovered milestone 177 (wire the graphical terminal stack
into the real interactive boot). *(Number provisional until the merge queue lands it.)*

**Gate: DECISION.** The fork is in "what a keystroke arrives on", below, and it is calef's: one
option is months of driver work on three architectures and the other changes what the story means. A
recommendation is recorded below, because the fork is reversible; the call is still his.

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
machine", and it is large: xHCI alone is a substantial driver against a substantial spec, so plan in
months rather than weeks. It also lands in the most attacker-adjacent position in the system, a bus
that enumerates whatever a stranger plugs in, which is either a problem or the best confinement
demonstration available depending on how it is built.
Note the peer project Atom keeps xHCI **in the kernel**, which is exactly the contrast DECISIONS §31
(the foreign-language seam) already draws for FAT32.

**Option C: split the difference by architecture.** Serial input on the boards, a cheaper native
path where one exists. **This had exactly one candidate and it is now closed**: a PS/2 port on the
OptiPlex would have been a few dozen lines against xHCI's spec, and calef confirmed on 2026-08-30
that the 7050 Micro has no PS/2 port and that he owns neither a PS/2 keyboard nor an adapter. So
there is no cheap native shortcut on any of the three machines, and the fork is genuinely binary.
Recorded rather than deleted, because the next person will have the same idea.

## The recommendation: option A, serial input, first

**Recorded here rather than withheld**, correcting this block's own first draft. It said "no
recommendation is offered, deliberately", and that was wrong by AGENTS.md's rule: recommend on
reversible forks, give options only on irreversible ones. **This fork is reversible.** Choosing
serial input forecloses nothing, because option B can be built at any point afterward, against the
same `DECISIONS §21` line-discipline contract that both the UART console and `display_terminal`
already speak identically.

**The reasoning is about what journey 3 is for.** That journey exists to test two of the nine
entries in design/fatal-risks.md: risk 9 (the HAL is a fiction and an architecture costs a
restructure) and most of risk 6 (a confined driver cannot drive real hardware). **A framebuffer on
real silicon tests both of those. A keyboard adds almost nothing to the test.** It adds to the
story, and the story matters, but not enough to spend months on a bus driver before the other seven
risks have been answered at all.

**The honest cost, stated so it is a known trade rather than a hidden one:** the demonstration
becomes *a person typing into the machine across a null-modem cable* rather than *a person sitting
at the machine*. That is weaker, and anyone watching will notice. It is a real loss, taken
deliberately, and it is the kind of thing this tree's `BUGS` convention exists to say out loud.

**What would change the recommendation:** a customer, or an audience, for whom the difference
between those two sentences is the product. Option B is also the better answer the moment nife has
survived enough of the fatal-risk list that months of driver work is a reasonable thing to spend.

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
- **The recommendation is a sequencing call, not a claim that option B is wrong.** A demonstration
  operating system that cannot accept a keyboard is limited in a way no scope note repairs, and this
  block should not be read as saying otherwise.
- **Option A has never been run either.** "Zero new drivers" is a reading of the code, not a boot:
  nobody has yet displayed a terminal on a board's framebuffer while taking keystrokes over that
  board's UART, and the first attempt is where the unglamorous problems live.
