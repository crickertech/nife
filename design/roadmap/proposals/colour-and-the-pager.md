# Colour and the pager: the spawn protocol's other two thirds

**Status: PROPOSED 2026-09-03.** Written by the milestone 247 sweep, from milestone 40's block.

**Gate: DECISION.** Both halves widen a protocol two programs agree on, which is the same shape as
DECISIONS §106 itself and the same reason §106 was calef's. A spawn-protocol bit is on a wire, so it
cannot be un-shipped once a program is written against it.

**In brief.** DECISIONS §106 narrowed the spawn protocol for a tail stage's *primary output*, which
is what lets `doc <page>` render at the prompt with no `| wc` in front of it. It built the narrowest
slice that unblocked that one command. Two thirds of the original scope are untaken: a bit telling a
stage that it ends at a real screen, which is the honest replacement for `isatty` and is what colour
needs, and a way to grant one line of *input* without granting the keyboard, which is what a pager
needs. Both want the same wiring bit §106 built one third of.

## Why this matters

The first half is what stops this system growing a dishonest `isatty`. Unix programs decide about
colour by asking the kernel what their file descriptor is attached to, which is ambient authority
answering a question about presentation. A capability system can say it properly, as a bit the
spawner passes, and every program that wants to colour output will need that bit. Until it exists,
the choices are no colour anywhere or a program guessing, and a guess here becomes a convention
before anybody decides it is one.

The second half blocks the pager outright. A pager needs one line of input at a time and must not
hold the keyboard, because holding the keyboard is exactly the authority a confined viewer should
not have. There is no way to express that today, so `doc` renders and cannot page. Milestone 40's
own refusal of `ratatui` said the tree needs its terminal contract first; this is a piece of that
contract.

Left alone, this is a decision nobody meets until they try to write the sixth program that wants a
screen, and by then there will be a habit rather than a protocol.

## Where it came from

Milestone 40's block: *"The other two thirds of §106's spawn-protocol narrowing: a bit telling a
tail stage it ends at a real screen (colour, the honest `isatty` replacement), and a way to grant
one line of input without granting the keyboard (the pager). Both widen a protocol two programs
agree on, so both are calef's call."*

`notes/manual.md`'s "Where this goes next" is the only other record, and it heads its list with
these two: *"DECISIONS §106 took the narrowing for a tail stage's primary output; it did not extend
the same bit to 'tell this stage it ends at a real screen' ... or to granting one line of input
without granting the keyboard. Both still want the wiring bit this entry originally scoped for all
three."*
