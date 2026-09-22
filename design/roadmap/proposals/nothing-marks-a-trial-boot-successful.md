# Nothing marks a trial boot successful, so a good upgrade rolls back too

**Status: PROPOSED 2026-09-21.** Raised by the rung 2b lane of milestone 198 (a package manager, and
the trivial install that makes a second customer possible), which built the two-slot rollback and
left exactly this half of it unbuilt.

**Gate: DECISION.** Not the mechanism, which is small and reversible, but the two things it carries
across a boundary: **how a chain-loaded image is told which slot it came from**, which on x86_64
means a token on the kernel's command line, and **what counts as having come up**, which is a
promise the whole feature is measured against. Both are calef's by `AGENTS.md`'s own list.

## The gap, in one sentence

`crates/boot_slot` has a `successful` bit and a `State::confirmed`, the installer sets the bit on
the slot it writes, and **nothing on a running machine ever sets it again**. So an upgrade written
into the other slot, which boots perfectly and runs for a week, is abandoned the moment its tries
run out and the machine quietly returns to the older image.

It fails safe, which is why the rung shipped without it. It also means **upgrades do not stick**,
which is not a system anybody would run.

## Why it was not built with the rest

Honestly and for a reason that is about cost rather than design: the rollback half is self-contained
in the loader and the installer, and this half crosses four boundaries. The lane chose to land a
proven rollback rather than an unproven whole, and `AGENTS.md`'s §92 (a caretaker is supervised by the client it serves) test applies and is answered
out loud: **at equal cost both halves would have been built together**, so this deferral is about
effort and is stated as such.

## What it needs, in the order the information travels

| | |
|---|---|
| 1 | `uefi_loader`'s chooser already writes `nife-slot=N` into the chain-loaded image's `LoadOptions`, and the started image already reads it. Nothing more is needed here |
| 2 | That image has to put the slot number where the kernel will see it. On x86_64 the `hvm_start_info` command line already carries one thing (where the screen is), so this is a second token on a line that exists |
| 3 | The kernel has to keep it and hand it to whatever confirms. `kernel/src/user/install_service.rs` is the neighbour that already spawns disk-holding programs |
| 4 | A program holding the disk, an entropy endpoint it does not need, and the slot number, which reads the table, sets `State::confirmed` on that entry and writes both copies back |

Step 4 is the smallest piece and it is very nearly `installer`'s `ROLE_SURVEY` with a write: one
more role on a program that already reads a GPT off a disk it was handed and already refuses to do
anything it was not granted.

**Step 2 is the decision.** A token on the kernel command line is a value two programs agree on,
which `AGENTS.md` puts in the irreversible category, and the alternative shapes (a field in
`machine_discovery`'s handoff, a module, a fixed physical page) each cost differently and none is
obviously right. A lane should not pick it.

## The other decision: what "came up" means

This is the more interesting half and it is a promise rather than a mechanism. Candidates, weakest
first:

| criterion | what it would catch | what it would miss |
|---|---|---|
| the kernel reached its self-test | a kernel that does not boot at all | every driver, the filesystem, the shell: almost everything an upgrade changes |
| the progenitor started userspace | the above, plus a broken archive | a machine with no working disk or console |
| **the filesystem server mounted the installed disk and the shell is reachable** | everything between power-on and a person being able to type | a regression in something nobody exercises at boot |
| a person, or a service, says so afterwards | anything, eventually | it is not automatic, and an unattended appliance has nobody to ask |

The third is the lane's recommendation and it is what the Boot Loader Specification's own counting
leaves to the operating system for the same reason: the bootloader cannot know, and the system can.
It is also the point at which a machine stops needing anybody: past it, somebody can log in and fix
whatever else is wrong, which is the honest definition of "this upgrade did not brick the machine".

**What it costs to be wrong in each direction.** Confirming too early marks a broken upgrade good
and the machine stays broken, which is the failure the whole feature exists to prevent. Confirming
too late, or never, is where the tree is today: safe, and upgrades do not stick. So the bias should
be late, and the third row is about as late as an automatic criterion can be.

## What is blocked until this is answered

Nothing that is built. An upgrader is blocked in the sense that shipping one without this would ship
a machine that reverts every upgrade a few boots after it succeeds, which is worse than having no
upgrader at all, because it fails days later and looks like something else.

## Prior art, read rather than recalled

ChromeOS's update engine sets the successful bit from userspace after the update is deemed good,
which is the same division of labour: the firmware counts down, the system decides. The Boot Loader
Specification's `+tries-left-tries-done` file-name counting is the same shape again with the state
in a filename, and it also leaves the "it worked" call to the operating system.

## BUGS

- **This proposal has not measured step 2.** The four shapes are named and none is priced, because
  pricing them is most of the work of doing it and `AGENTS.md` asks a fork to arrive with its
  questions answered. A lane taking this should price them before bringing the fork, and should
  expect the command-line token to win on being a line that already exists.
