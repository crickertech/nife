# The narrator prints on every boot and cannot be typed at the prompt

**Status: PROPOSED 2026-09-09.** Left behind by milestone 267's lane, which moved the milestone
narrative out of `kernel_main` into `user/src/narrator.rs` and got half of what that milestone asked
for.

**Gate: NONE.** Nothing is owed and nothing is blocked. It wants a lane because it is a second
protocol's worth of work rather than a line, and because the shape of the answer is a small design
question about what a program's output slot means when the kernel is the one starting it.

**In brief.** Milestone 267's stated proof was *"a boot that prints the machine description and
nothing else, and a program you can run that prints the milestone narrative"*. The program exists.
You cannot run it.

`narrator` speaks the console server's raw protocol: two rendezvous endpoints in capability slots 0
and 1, and one page at `0x60_0000` that it writes bytes into while the server reads them. That is
the shape `user/src/hello.rs`'s `printing_client` has had since 19f.3 and it is what
`kernel/src/user/console_service.rs`'s `spawn_client` hands out, so the kernel's tour can start it.

`swish` starts a program with an output **sink** (`crates/byte_sink_proto`), which is a different
contract with a different opcode set, reached through a different slot. A program written against
one cannot be started by the other, so `narrator` at the prompt does nothing but fault or exit.

## Why the lane did not just fix it

Three options, none of them a line of code, which is why this is a proposal rather than a follow-up
commit.

**Write the narrator against the sink protocol and have the kernel adapt.** The kernel would then
need something that presents the console server as a sink, which is `terminal_sink_caretaker`'s job
one level up and is a program the tour does not currently start. It is the right shape and it is a
new wiring in the boot path, which is exactly the code milestone 267 was trying to remove.

**Give the narrator both protocols and pick at `_start`.** Cheap, and it makes a demonstration
program carry a branch whose only purpose is to hide that this tree has two ways for a program to
write bytes. That is the kind of convenience argument AGENTS.md asks a proposal to name out loud, so
here it is named: it is less work and it is worse.

**Have the progenitor start the narrator instead of the kernel.** The progenitor already builds the
console, the line discipline and the shell, so it has a real terminal to hand over, and the narrator
would be an ordinary sink program that the shell can also start. The obstacle is that the progenitor
cannot see the kernel's features: a `--features shell` boot runs the same progenitor and must not
print the narrative, so the kernel would have to tell it, and *"the kernel tells the first process
what kind of boot this is"* is a word on the handoff that two programs agree on. That is the
expensive category in AGENTS.md's own terms and it is calef's call, not a lane's.

## What is not in question

The narrative must keep printing on the default boot. Milestone 267's block is explicit that
*"run this other program" is a worse default than "it prints"*, because `script/server` is what a
stranger runs first. Any of the three options above has to keep that true.

## What it costs to leave alone

A `BUGS` entry in `user/src/narrator.rs` and one in milestone 267's block, both written. The
demonstration still reaches everyone who boots the system, which is the audience that matters most,
and the missing half is the ability to see it again without rebooting.
