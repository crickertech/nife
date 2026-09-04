# The watcher reads a board and never speaks to it, so stopping a reboot loop needs a person

**Status: PROPOSED 2026-09-03.** Written by the milestone 249 lane (the boot lottery is sampled by a
person walking to the board, so nine draws is a whole evening), which built the escape this would
automate.

**Gate: NONE.** It is a mode on a script that already holds the port.

**What the work is.** Milestone 249's self-rebooting soak is stopped by pressing a key: the kernel
polls the console UART's data-ready bit and disarms the reboot when any byte has arrived.
`script/board-console` holds that port for the whole series and **cannot send the byte**, because its
own header states that it reads and never writes.

So two things are rung four that need not be. **The escape is a keypress**, which means an unattended
series cannot be stopped by the thing watching it. And **the verification of the escape is a person
confirming `soak-reboot: DISARMED`**, which is the only way milestone 249 can prove its own safety
mechanism works at all, since a UART cannot receive a byte it sends.

A `--stop` mode, and a `--stop-after <n>` that ends the series at a chosen number of draws, would
make both a command.

**Why it was not done in 249.** It overturns an invariant stated in `script/board-console`'s own
header, which makes the shape of it calef's call rather than a lane's. A reader-only watcher is a
deliberate property: it is what makes it safe to attach one to a board another session is driving.
Whatever ships has to say what happens when two watchers hold the same port and one of them can
write.

**Recorded in the meantime** where a reader meets the tool, in `notes/board-console.md`'s `BUGS`.
