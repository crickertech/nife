# ASIDs ship with their payoff asserted rather than measured

**Status: PROPOSED 2026-09-03.** Written by the milestone 247 sweep, from milestone 15's block.

**Gate: HARDWARE.** Of the second kind: the machines exist. QEMU cannot show this, because it does
not model TLB refill cost, so a number from patagonia would be noise wearing a decimal point. It
needs argon, radon or xenon, and it needs somebody at one of them to flash an image and read a
serial console. Waiting does not discharge it.

**In brief.** Address space identifiers landed in July and the context switch stopped flushing the
TLB. No machine has measured what that changed. Put a number on it: the address-space-switch cost
with tagging on and with it off, on real silicon, on at least one of the three boards. Milestone
15's own block calls ASIDs the prerequisite for reasoning about switch cost, and that reasoning has
never had an input.

## Why this matters

The mechanism is in the kernel on every architecture and its justification is an argument. That is
the shape this project explicitly refuses: measure, do not argue. A benchmark this system publishes
about context switching is currently priced against a TLB behaviour nobody has characterised, and
the honest thing to say about ASIDs today is that they are standard practice, which is a reason to
implement them and not evidence that they helped here.

There is a specific reason the number could be surprising rather than confirmatory, and it is why
this is worth a board's time. This kernel bounds concurrent address spaces at 160 and milestone 15
refused the generation-and-rollover scheme because the exhaustion path is unreachable below 256
tags. A system that never recycles a tag has a different TLB pressure profile from Linux, which
recycles constantly, and the win could be larger than the textbook figure or smaller. Either
outcome is a result. Neither is available from QEMU.

It also feeds work that is already tracked. Address-space-switch cost is an input to scheduling and
to IPC pricing, and every comparison this project makes against Linux, macOS or seL4 on those
primitives currently omits the term.

## Where it came from

Milestone 15's Follow-on: *"Put a number on what ASIDs bought. The switch stopped flushing the TLB
in July and no machine has measured what that changed, nor the address-space-switch cost this block
calls ASIDs the prerequisite for reasoning about. QEMU cannot show it; argon, radon or xenon can.
The mechanism ships with its payoff asserted rather than measured."*

Two facts from the same block bound what a measurement means. The RISC-V half arrived separately in
milestone 58, because `sfence.vma` is local and discharging a tag machine-wide needs an IPI to
every hart through SBI RFENCE, so radon's number is about a different mechanism from argon's.
And `notes/asids.md` records that RISC-V permits `satp.ASID` to be zero bits wide, so a RISC-V
machine that cannot tell tags apart keeps flushing on every switch and would measure no change at
all. Whichever board is used, the report has to say which of these it was.
