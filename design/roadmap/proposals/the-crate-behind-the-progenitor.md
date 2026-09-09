# The crate behind the progenitor, and whether it keeps `system_initializer`

**Status: PROPOSED 2026-09-08.** Raised by milestone 266, which renamed the program and left the
crate alone on purpose.

**Gate: DECISION.** It is a crate name, and names are calef's.

## The relationship the names used to record

AGENTS.md: *"A crate and a program may share a name, and it says something when they do: the crate is
that program's logic, lifted out so it can be host-tested and Kani-reachable while the program keeps
the IO."* `coremark`, `line_editor` and `compositor` are all that pair.

`system_initializer` was that pair too, until milestone 266 renamed the program to `progenitor`. The
crate is still exactly what the pair describes: `user/src/progenitor.rs` is eighty lines of endowment
table and one call into `system_initializer::boot`, which is the whole interactive system. The
relationship did not change; only the evidence for it in the names did.

## Why it was not renamed in 266

`system_initializer` was ratified 2026-08-01 (milestone 63), replacing `sysinit`, with four refusals
recorded beside it. `progenitor` was ratified for the first process, not for its crate. A lane
renaming a ratified crate on the argument that a *different* ratified name implies it is exactly the
move milestone 115 exists to prevent.

## The argument for renaming it

calef's own case for `progenitor` applies to `initializer` unchanged: it is an agent noun formed from
a verb naming an action, and the thing is not an action. `system_initializer` is also the longest
crate name in the tree at nineteen characters, and it appears in every `use` in the boot path.

## The argument against

`initializer` is a legitimate agent noun and the crate genuinely *is* the initialisation, as
distinct from the process that runs it: the crate does not descend anything, so `progenitor` would be
a worse fit for it than for the program. A tree where `progenitor` is both the program and its crate
also loses the distinction that this file exists to point at, which is that the boot entry is data
and the system is code.

**A third option nobody has argued yet**: rename it to what it builds rather than what it does, on
the model of `compositor`. `interactive_system` and `boot_system` are both available and both
generic.

## What is blocked

Nothing. The mismatch is cosmetic and is recorded in `user/src/progenitor.rs`'s own name block.
