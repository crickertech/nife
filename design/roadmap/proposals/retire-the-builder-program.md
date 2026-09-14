# Retire `components/src/builder.rs`

**Status: PROPOSED 2026-09-14.** calef ruled during milestone 268 that this program should go away
as a result of that milestone. Milestone 268's lane did not perform the removal, and this file says
why and what the removal is when it happens.

**Gate: DECISION.** One sentence from calef, set out under "What calef has to decide" below. It is
not a design fork and it does not need a lane to research it: the options and their costs are here.

## What `builder` is, and the claim it carries

`components/src/builder.rs` is milestone 20's richer-initrd demonstration, and the RISC-V boot tour
enters it directly. The kernel loads it with **exactly two capabilities**, an untyped budget and a
report endpoint, and maps the archive in. It then parses the archive, reads `least_authority_demo`
by name, parses that ELF **in userspace**, builds a child out of its own budget, starts it with 9,
and the child sends back 81. The kernel never touches the child's bytes.

The claim is: **userspace, not the kernel, composes a process.** That is the thesis of a capability
microkernel in one boot step, which is why it exists and why retiring it needs an argument rather
than a deletion.

It runs on riscv64 only. aarch64's tour has no such step; `x86_64` packs the archive but never
enters it.

## calef's ruling, 2026-09-14

> he expects `components/src/builder.rs` to GO AWAY as a result of this milestone [...] The claim
> `builder` carries -- *userspace, not the kernel, composes a process* -- does not disappear. It
> becomes one of item 2's collected self-tests, running on all three architectures instead of one.
> The program goes away because the capability got LEVELLED UP into the ladder, not because a path
> was deleted to make three arms agree.

And, declining to rule on its name the day before: *"Skip this one because it will go away with
parity."* So the provisional `process_builder` in its provenance block is **not settled** and must
not be treated as one.

## Where milestone 268 left it

**The ladder subsumes the claim on riscv64, and that is demonstrable rather than argued.** Milestone
268 item 4 made riscv64's default boot hand over instead of halting, so one boot now shows both:

```
  init/build  : the userspace builder loaded 'least_authority_demo' from a 7584768-byte archive
                and built it as a child; the child sent 81 (expected 81)
nife: the capability core runs on RISC-V.

nife: handing the system to the userspace progenitor.
progenitor: construction budget dropped; retype answers NoSuchSlot
progenitor: every program measured against the archive table
nife capability shell. naming a resource in a command IS granting it.
$
```

The second half is the same claim at a larger scale: the progenitor loads, measures and composes the
console server, the line discipline, the input driver and `swish` out of **its own** budget, and the
kernel touches none of their bytes. aarch64's default boot has done this since milestone 28.

**What is not true is "on all three".** A compose-a-process self-test needs a compiled ELF to run at
user level, and `x86_64` has none: `crates/user_rt` has no `x86_64` arms, so that architecture runs
hand-assembled children and cannot load a program by name at all (`kernel/src/main.rs`'s x86 arm
says so itself: *"real ELF user programs (`user_rt` has no `x86_64` arms)"*). So the version calef
described, a self-test carrying this claim on every architecture, cannot be built today by any
amount of work inside milestone 268's scope.

Removing `builder` now would **relocate** the claim on riscv64 and **change nothing** on `x86_64`,
which already has neither the program nor the claim. So it would not lose anything. It would,
though, close the door on the shape calef asked for, and it is not free.

## What calef has to decide

**One sentence, either way:**

- **(a) Wait.** Retire `builder` once an `x86_64` leg exists that can load a compiled ELF at user
  level, and make the compose-a-process claim a boot self-test on all three at that point. This is
  the version of the ruling as written, and it is gated on work nobody has scoped (`user_rt`'s
  `x86_64` arms, beyond milestone 182's own entry point).
- **(b) Go ahead now.** Retire it on the strength of the progenitor carrying the claim wherever a
  progenitor runs, and accept that `x86_64` carries it nowhere until it can run a program at all,
  which is already true and already recorded.

**The recommendation is (b)**, and the reason is that (a) buys nothing it does not already have:
`builder` does not run on `x86_64` today, so waiting protects a claim that architecture does not
make either way. What (a) genuinely protects is the *minimality* half, which (b) does lose and which
is stated plainly in the risk below.

## The risk (b) takes, stated rather than buried

**`builder` proves composition from exactly two capabilities. The progenitor does not.** The
progenitor is granted a budget, endpoints, the UART and its interrupt line, because it is building a
system rather than demonstrating a minimum. So (b) keeps "userspace composes a process" and drops
"...from an authority you can count on one hand".

Whether that matters is calef's call. The tree's own answer may already be elsewhere:
`least_authority_demo` is named for the property and `crates/grant_plan` reasons about it, so the
minimality claim may be better served by a host-testable proof than by a boot step on one
architecture.

## What the removal is, concretely

Contained; surveyed 2026-09-14. Eight sites, no others:

1. `kernel/src/main.rs`: the RISC-V tour's `is_archive` branch and its `init/build` line.
2. `kernel/src/user.rs`: `riscv_initrd_demo`, which reads `"builder"` and calls
   `trust::require("builder", ...)`, plus the bench-diagnostic watcher inside it.
3. `components/src/builder.rs` and its `[[bin]]` entry in `components/Cargo.toml`.
4. `xtask/src/main.rs`: `boot_programs` (`"riscv64" | "x86_64" => &["progenitor", "builder",
   "hello"]`) and `portable_archive_entries`'s `("builder", "builder")` row. Both feed the
   measured-boot manifest, so both must move together or a card meets `MEASURED BOOT REFUSED`.
5. `kernel/src/trust.rs` and `crates/user_rt/src/initrd.rs`: doc comments naming it.
6. `crates/board_console/src/progress.rs`: `userspace_ran()` matches `init/build`, which nothing
   would print any more.

**Item 6 is the one with teeth**, and milestone 268's brief named it: *item 5 has to give
`board_console` something to match INSTEAD, or a healthy board reads as never having composed
anything.* Two constraints pull against each other. The live replacement is milestone 268's
`Stage::Prompt` (`boot_ladder::PROMPT`), which can only appear if userspace built the console, the
line discipline, the input driver and the shell. But the **captured board transcript**
`crates/board_console/tests/fixtures/captured/vf2-2026-09-01-userspace.log` contains `init/build` and
is asserted on, and that capture is evidence off real silicon that cannot be re-taken with a
different kernel. So the matcher has to stay for the fixture while the live claim moves, and the
accessor's doc has to say which it is answering. Recording that here so the next lane does not
discover it by deleting a test.

## Why this is a file rather than a paragraph in milestone 268

`script/roadmap`'s `Proposed.` disposition requires one, and the reason it requires one is this
tree's own scar: milestone 94 swept the tree for work that existed only in prose, then left its own
inventory in a pull request body for twelve days, by which point the item-level list was gone and
had to be re-derived. See notes/untracked-work-sweep.md.
