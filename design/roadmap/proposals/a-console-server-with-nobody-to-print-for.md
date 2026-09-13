# The boot-time console server comes up with no client and nothing notices

**Status: PROPOSED 2026-09-13.** Left behind by `maintainer/delete-narrator`, which deleted the
narrator on calef's ruling and left the server that existed to print for it running.

**Gate: DECISION.** calef's, and specifically because the answer is not "delete it": this is
infrastructure rather than a demonstration, and the lane that found it was told to report rather
than decide. Nothing is blocked meanwhile; the boot is correct either way.

**In brief.** On a tour boot (no `shell`, no `initboot`), `kernel/src/main.rs:1676` still runs
`user::initrd().map(|_| user::console_service::start())`. That spawns `user/src/console.rs`, a real
UART driver at EL0 holding the PL011's registers, which blocks on `recv(REQUEST)` forever because
nothing in the boot holds a capability naming its endpoint. Its only client was the narrator.

**The compiler agrees, which is why this is not a reading.** With `spawn_client` gone, every field
of `console_service::Console` (`request`, `reply`, `shared_phys`) is written by `start` and read by
nobody; `rustc` reports *"fields `request`, `reply`, and `shared_phys` are never read"*. The handle
`start` returns is the wiring a client would need, and there is no client. The `#[expect(dead_code)]`
on that struct is this proposal made visible in the code, and it should be deleted when this is
answered rather than left as the answer.

## What it costs today

Not correctness, and saying so plainly matters because the cheap read is that this is a leak. It is
not. The server blocks in a rendezvous rather than spinning, so it burns no CPU; `no_leaked_threads`
does not police it because the tour boot is not a test boot.

What it does cost is real but small: one spawned process with its own address space, one zeroed
frame for a shared page nobody writes, two rendezvous objects, and **a second mapping of the UART's
registers handed to a program that will never use them**. That last one is the item worth a
decision. This tree's whole argument is that authority is granted deliberately; a device mapping
that exists because of a program deleted four days earlier is the opposite of deliberate, whatever
its runtime cost.

## The options, with what each one actually changes

**Leave it.** Zero work, and it keeps `console.rs` exercised on the default boot in the weak sense
that it is loaded, relocated and entered. The honest version of this option is that the exercise is
worth very little: nothing checks the server did anything, which is the same criticism that deleted
the narrator.

**Stop starting it on the tour boot.** Delete the `map` line and let `console_service::start` and
its module go with it. `user/src/console.rs` **stays**, because it has consumers that have nothing
to do with this path: `crates/system_initializer` loads it (`lib.rs:744`), `user/src/hello.rs:571`
builds it as init's print server, `measured_boot_tests.rs` measures it, and `swapper.rs` names it in
a dependency check. The interactive boot reaches it through `boot_via_progenitor`
(`kernel/src/main.rs:1904`), never through `console_service`. So this option deletes a kernel module
and one boot line, and deletes no program.

**Give it a client again.** Only if something is actually wanted on the tour boot that has to print
from EL0 through a driver at EL0. Nothing is today. Listed because it is the option that says the
server was infrastructure rather than scaffolding, and somebody should say that out loud if it is
true rather than letting the answer be decided by which is less work.

## Why the lane did not pick one

Two reasons, and the first is the binding one. **It was not authorised**: calef ruled the narrator
deleted, and `console_service::start` plus `user/src/console.rs` are not the narrator. Deleting
infrastructure on the momentum of a demonstration's deletion is exactly the sweep AGENTS.md's blind-
`sed` scar is about.

**And the second option touches a boot path.** `kernel/src/main.rs`'s tour block is the file
milestone 266 landed first to make safe to open, and the three boot arms diverge there. That is not
a fork a lane should take on its own initiative on the strength of a `dead_code` warning.
