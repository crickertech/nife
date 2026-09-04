# `script/board-console` cannot type at a board, and nobody has decided whether it should

**Status: PROPOSED 2026-09-03.** Written by the milestone 247 sweep, from milestone 216's block.

**Gate: DECISION.** The question is calef's and it is the whole item: whether the console tool
should ever write to the serial port, and if so whether that is this tool with an explicit mode or a
second tool. Milestone 218 may remove the need entirely by fixing autoboot, so the answer may turn
out to be no.

**In brief.** `script/board-console` reads a board and never writes, to the port or to the outlet.
That was a deliberate refusal rather than an omission: milestone 216 declined to settle the question
by implementing it. A board sitting at a U-Boot prompt, or one that has wedged before autoboot,
cannot be typed at by anything in this tree.

## Why this matters

A bench session facing a board that will not boot has no sanctioned way to interact with it. The
fallback is a terminal emulator opened by hand, outside the tool that tees every byte to a log,
which means the one session where the record matters most is the one with no record. That is the
gap milestone 216 was written to close, one step further in.

The reason it is a decision and not a lane's is that reading and writing are different objects. A
tool that only reads cannot damage a board or a bench; one that can write can send anything to
whatever is listening, and the bench has an outlet on the same strip feeding an external drive that
must never be switched off. Milestone 216 refused power control for exactly that reason and left
this beside it.

## What would settle it

Milestone 218 is the reason to wait rather than decide now. It is fixing hands-free boot on radon,
and if the board boots unattended the case for typing at it shrinks to debugging a board that has
already failed. If 218 lands and the need persists, the shape of the answer is the second question:
one tool with a mode you have to ask for, or two tools where the reading one stays incapable of
writing by construction. The second is the higher rung on AGENTS.md's ladder and costs a second
binary.

## Where it came from

Milestone 216 (nothing in this tree can read a board) left it open on purpose: *"Whether
`script/board-console` should ever be able to write to the serial port at all, and if so whether
that is this tool with an explicit mode or a second tool. It is calef's call. Milestone 218 may
remove the need by fixing autoboot, in which case the answer is no; while it is open, a bench
session facing a board that will not boot has no sanctioned way to type at it."*
