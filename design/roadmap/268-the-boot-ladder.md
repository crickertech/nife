# 268. Every architecture boots the same way: describe the machine, test yourself, hand over

**Status: PARTIAL.** Minted 2026-09-09 by calef, from milestone 267's measurement and the parity
review that followed it. *(Number provisional until the merge queue lands it.)*

**Gate: DECISION, MILESTONE 182.** The ladder itself is decided (calef, 2026-09-09, in
conversation) and nothing in it is a design fork. What is gated is only its **last rung on
x86_64**: that architecture cannot reach a prompt until DECISIONS §149 says how `swish` gets a
console there, and until milestone 182 builds the entry point. **Every rung below the last one runs
before userspace exists and waits on neither**, so the bulk of this milestone can land first and
should.

## What calef decided, so a lane does not re-litigate it

- **The machine description prints on every architecture**, in the kernel, before userspace. It is
  what you need in order to bring up new hardware, which is exactly the moment there is no userspace
  to run a program from. Same display everywhere even though the backing differs.
- **Nothing halts.** The default boot ends at the `swish` prompt, and **the prompt is the signal that
  the boot finished**. This replaces the halt as the terminal state.
- **The boot self-tests come between the two**: describe the machine, prove the kernel works on it,
  then hand over.
- **The self-test reports; it does not gate.** A failure prints its verdict and the boot continues to
  the prompt anyway, because *"I could see building tools to diagnose what broke but we can only run
  them if there is a prompt"* (calef). A machine you cannot log into is a machine you cannot fix.
- **The verdict is not part of `swish`.** The console is how this system reports to its operator,
  whether that operator is a person, an agent or a harness, and unread lines on a terminal are not a
  defect of that channel.
- **`self_test` is ratified** (calef, 2026-09-09) after the prior art turned out to be in this tree
  already. See below.
- **Parity is reached by levelling up, never by levelling down** (calef, 2026-09-09):
  *"drive parity by not degrading functionality away from where we want to get but rather by driving
  everything towards where we want to go."* There is a specific trap in this milestone; see BUGS.

## The four findings this is built on, each measured

**1. `machine.rs` exists on x86_64 only.** aarch64 and riscv64 discover the same facts from the
device tree and print them scattered inline through `main.rs`. So this is not moving one
architecture's output to the others. The other two have no machine module at all.

**2. aarch64 has no `exceptions::self_test`.** riscv64 and x86_64 both have one, called from
`main.rs:210` and `main.rs:686`, right after `arch::init()`: it fires a breakpoint, proves it was
caught and stepped over, and returns the count. `notes/riscv-port.md` calls it "a boot self-test".
There is a second, unnamed one at `main.rs:873`, a dynamic kernel mapping check that maps a fresh
frame at an unused high-half address and reads it back. **The concept and the name are already in
this tree, on two architectures, uncollected.**

**3. `Stage::Tour` can never fire on two of three architectures.** `board_console` matches the single
string `"nife: the capability core runs on "`, printed at `kernel/src/main.rs:1401`, **inside the
RISC-V arm**. The aarch64 tour prints no marker at all. So `--until tour`, the checkpoint the bench
tooling relies on to mean "paging, traps, the timer, the frame allocator, SMP and the scheduler all
came up", is unreachable on aarch64 and x86_64, and a missing marker looks exactly like a slow board.

**4. The tour's checks report failure by printing a word and continuing.** It spawns two threads that
never yield and counts preemptions; it reads a filesystem through the real driver and compares
bytes. Both are real. Neither asserts: on failure they print `FAILED: a spinner did not run, or
nothing was preempted.` and `the pcie driver reported the wrong bytes`, and the boot reaches the same
end state either way. **That is rung four of AGENTS.md's ladder**, and it is why finding 3 hid for
months: nothing was gating on the rung everyone believed in.

## Why `self_test`, and the prior art is ours

The candidates were `self_test`, `power_on_self_test`, `checkup` and `diagnostic`. The tree decided
it before the question was asked: `arch::exceptions::self_test()` already exists on two
architectures, already runs at boot, already means "the kernel proving something about itself", and
`notes/riscv-port.md` already uses the words "boot self-test" in prose.

The collision the maintainer worried about (against `kernel/src/testing.rs`, `script/test`, and the
Kani harnesses) **is not real**: the tree already draws the line consistently, with `self_test`
meaning the kernel checking itself at boot and `test` meaning the suite.

`POST` is taken, and not by us. `notes/xenon-firmware.md` quotes Dell's own "Power On Self-Test
(POST)" from xenon's firmware, which an operator meets on the screen before ours runs.

External prior art was **recalled and not read**, and is recorded here as a claim to check rather
than as evidence: Linux's crypto subsystem runs boot-time self-tests with per-algorithm pass lines,
KUnit runs in-kernel tests at boot behind a config, FIPS 140 mandates power-up self-tests that do
gate, and hardware calls the concept BIST. The decision did not turn on any of them.

## What to build

1. **`machine` on all three architectures**, in the kernel, before userspace. The parity claim is
   **the same questions answered, not the same lines printed**, because one machine has ACPI and two
   have a device tree. The required set: processors, memory, console, interrupt controller, timer
   rate, initrd, PCIe window, IOMMU. Each architecture answers in its own vocabulary; each answers
   all of them or says plainly that it cannot.
2. **Collect the existing self-tests behind one `self_test`**, give aarch64 the `exceptions` one it
   lacks, and run the set on all three.
3. **One verdict line, and it is a contract.** Stable text, identical on all three architectures,
   because its audience includes `board_console` and CI. The tour's failure was never that a human
   missed a line; it was that the only matchable string lived in one architecture's arm, so on the
   other two there was no signal on the channel at all.
4. **Nothing halts by default.** The boot proceeds to userspace and to the `swish` prompt.
5. **`board_console` learns the new ladder**: `Handoff` -> `Banner` -> machine -> self-test verdict
   -> prompt, with a **failed** verdict as an outcome distinct from a clean boot, so a degraded board
   on the bench does not read as a good one.
6. **CI fails when a boot self-test reports a failure.** With the boot no longer gating, this is the
   mechanism that keeps the verdict honest, and without it this milestone recreates finding 4 with
   better prose.

## What must not change

**The measurement builds keep parking before userspace.** `bench`, `icount`, `soak` and `jobmix` all
deliberately stop short of it, and `kernel/Cargo.toml` gives the reason in the feature comments: a
board has no command line, so these are compile-time or they are nothing. "Nothing halts" is about
the **default** boot and only about it.

## The proof that this milestone worked

**`--until` reaches the self-test verdict on all three architectures**, and the same failure injected
into any one of them turns the verdict red on all three and fails CI. Not a survey of what each arm
prints, and not a marker that exists on one architecture, which is the defect being fixed.

## BUGS

- **The levelling-down trap is specific and a lane will meet it.** `arch::machine::attach_screen` is
  an x86-only kernel path with no counterpart on the other two, and a naive parity reading says
  delete it. It is **the only thing in this system that puts pixels on real hardware**, and xenon's
  monitor is the one working display we have. The other two grow a display path (milestone 157); x86
  does not lose one.
- **A verdict nobody reads is what we have now.** Item 6 is the whole mechanism and it is the piece
  most likely to be dropped as "follow-up", which would leave this milestone shipping finding 4 again
  under a ratified name.
- **The required question set in item 1 is asserted rather than derived.** It came from reading
  x86_64's existing output and asking what the other two would need; nobody has checked it against
  what a person actually wants when bringing up an unfamiliar board.
- **This block does not say what the verdict line reads.** That is a contract two programs agree on,
  so it is calef's under the *move fast on what can be undone* tenet, and a lane should ship a
  provisional wording and say so rather than wait.

## Follow-on

- **Milestone 182.** x86_64's boot has nowhere to land until it has a shell. This milestone's rungs
  all run before userspace, so it does not wait, but the top rung is unreachable there until 182.
- **Decision.** `design/decisions/149-kernel-served-console-endpoint.md`, how `swish` reaches a
  console on x86_64 where §121 leaves no userspace holder.
- **Milestone 269.** `machine` as a program that can be run from the prompt.
- **Recorded.** The `attach_screen` asymmetry is named in BUGS above rather than left for a lane to
  rediscover, because the wrong move is the obvious one.
- **Outstanding.** The ladder's **top rung on `x86_64`**: a default boot there still ends in
  `nife x86_64: boot complete, halting.` after a green verdict, because the architecture has no
  entry point that hands the machine to a shell. Still this block's scope and still gated on the two
  things named above it; checked 2026-09-14 by booting it.
- **Proposed.** `design/roadmap/proposals/retire-the-builder-program.md`. **Retire
  `components/src/builder.rs`.** calef ruled on 2026-09-14 that it should go
  away as a result of this milestone, because the claim it carries (*userspace, not the kernel,
  composes a process*) becomes one of item 2's collected self-tests on all three architectures. Its
  name is parked for that reason (*"Skip this one because it will go away with parity"*), so the
  provisional `process_builder` in its provenance block is not settled and was not touched. This lane
  did not perform the removal: the ladder subsumes the claim **on riscv64** and can show it (item 4
  made that boot hand over, so the same boot now watches the progenitor compose the whole system from
  its own budget), but a compose-a-process self-test on **all three** needs a compiled ELF to run at
  user level and `x86_64` has none (`user_rt` has no `x86_64` arms). So the retirement waits on
  either that leg or a one-sentence ruling that the progenitor carrying the claim wherever a
  progenitor runs is enough. The work when it happens: drop the tour's `init/build` step and
  `kernel::user::riscv_initrd_demo`, drop `builder` from `xtask`'s `boot_programs` and the archive
  table, delete the program, and give `crates/board_console` a live marker in place of
  `userspace_ran`'s `init/build` (the captured board transcript keeps needing the old one).
- **Proposed.** `design/roadmap/proposals/one-machine-description-not-two.md`. **Trim the
  duplicated bring-up narrative on riscv64 and `x86_64`**, now that the
  machine description answers the same questions in one block on every architecture. The constraint
  to respect is the one that put those lines there: a `test` or `bench` boot compiles the description
  out and still has to say what machine it ran on.
- **Proposed.**
  `design/roadmap/proposals/the-machine-description-should-say-the-screen-geometry.md`. **A
  machine-description line for the framebuffer's geometry.**
  `console::print_summary` reports a screen's address and length, which is what the kernel holds; the
  width, height and pixel order arrive in the handoff and are printed by the `x86_64` arm alone. A
  board with a monitor and no serial port is exactly the machine this block was written for.

---

## What was built (2026-09-14)

**PARTIAL and not BUILT, deliberately.** Every rung below the last one landed on all three
architectures on 2026-09-14. The last rung is reached on aarch64 and riscv64 and is still gated on
`x86_64`, which is exactly what this block's own gate line said would happen, so the gate is live
rather than stale and the status has to admit it. See "What `x86_64` could not reach".

### The proof condition, answered

> *`--until` reaches the self-test verdict on all three architectures, and the same failure injected
> into any one of them turns the verdict red on all three and fails CI.*

Both halves, measured on 2026-09-14:

```
$ script/boot-check
boot-check (aarch64): reached self-test verdict (2404 bytes in 2.4s)
boot-check (aarch64): nife machine: aarch64, 4 processor(s), 256 MiB, 100 Hz
boot-check (aarch64): nife self-test: 5 of 5 passed
boot-check (riscv64): reached boot tour complete (6862 bytes in 2.8s)
boot-check (riscv64): nife machine: riscv64, 4 processor(s), 256 MiB, 100 Hz
boot-check (riscv64): nife self-test: 5 of 5 passed
boot-check (x86_64): reached self-test verdict (6182 bytes in 2.9s)
boot-check (x86_64): nife machine: x86_64, 1 processor(s), 254 MiB, 100 Hz
boot-check (x86_64): nife self-test: 5 of 5 passed
boot-check: every architecture reached the self-test verdict and it was green
$ echo $?
0
```

And red, with `self_test_injection` made a *default* feature so that the ordinary gate met it
(rather than the `--inject` mode, which expects red and would have proved only itself):

```
boot-check (aarch64): nife self-test: 4 of 5 passed, 1 FAILED: exceptions
boot-check (aarch64): FAILED. The default boot did not reach a green self-test verdict. ...
boot-check (riscv64): nife self-test: 4 of 5 passed, 1 FAILED: exceptions
boot-check (riscv64): FAILED. ...
boot-check (x86_64): nife self-test: 4 of 5 passed, 1 FAILED: exceptions
boot-check (x86_64): FAILED. ...
$ echo $?
1
```

Exit 1 is what fails CI: `boot-check` is a `local` row in `script/ci-build`'s table (milestone 286's
one enumeration), and CI's test job names it beside `test` and `shell-check`. **The verdict has been
seen red on all three**, which is the half of a gate that usually never gets checked.

**And through the other reader too.** The proof condition names `--until`, which is the bench tool
rather than the gate, so it was checked there as well: `cargo xtask board-console --until selftest`
reaches the verdict on all three captures and exits 0, and on the three injected captures it reports

```
board-console: failed: the kernel's boot self-test failed on this machine: exceptions. The boot
continued to userspace anyway (it reports, it does not gate), so the board is up and degraded
rather than dead
board-console: self-test: nife self-test: 4 of 5 passed, 1 FAILED: exceptions
```

and exits 1. The two readers are the same recogniser, which is the point: a gate that used a second
reader would be gating something a bench run does not measure.

### The six items

1. **The machine description answers the eight questions on all three.** `print_machine_description`
   stays in `kernel/src/main.rs` (milestone 267's lint reads it there by name) and is now called
   from every architecture's arm rather than only from the shared aarch64 path. Processors, memory,
   console, interrupt controller, timer rate, initrd, PCIe window, IOMMU, each in its own
   vocabulary: a GICv2's two register blocks on aarch64, a PLIC and its S-mode context on riscv64,
   the local-APIC/IO-APIC pair on `x86_64`; SMMUv3, riscv-iommu and VT-d for the last question.
   Four new per-architecture summaries were written for it (`arch::irq::print_summary`,
   `arch::iommu::print_summary`) plus one portable one (`console::print_summary`).
2. **`self_test` collects the set and runs it on all three.** `kernel/src/self_test.rs`, five
   checks: `exceptions`, `mapping`, `frames`, `timer`, `scheduler`. Two of them were already here
   and uncollected, exactly as this block said: `arch::exceptions::self_test` (riscv64 and
   `x86_64` had it, aarch64 now does) and the RISC-V tour's unnamed `kmap test`. Both were removed
   from the tour and gained two architectures.
3. **One verdict line**, `nife self-test: 5 of 5 passed` / `..., 1 FAILED: <names>`. **Wording
   provisional**, as this block's `BUGS` instructed.
4. **Nothing halts by default on riscv64 either.** Its tour used to end in `arch::halt()`; it now
   ends in `riscv_hand_over`, the same call `--features shell` makes, so the default boot ends at a
   `swish` prompt. aarch64 already did this. Transcript of a riscv64 default boot with an archive:
   `nife: the capability core runs on RISC-V.` then `nife: handing the system to the userspace
   progenitor.` then `nife capability shell.` and a `$ ` prompt.
5. **`board_console` learned the ladder.** Three new stages, `Machine` -> `SelfTest` -> `Prompt`,
   and a **failed verdict is `Failure::SelfTestFailed`, not a stage**, so a degraded board does not
   read as a good one. `cargo xtask board-console --until machine|selftest|prompt`. Eight new host
   tests.
6. **CI fails on a red verdict**: `script/boot-check`, a `local` row in `script/ci-build`'s table
   (milestone 286's one enumeration), named by CI's test job beside `test` and `shell-check`.

### `crates/boot_ladder`, which was not in the plan

The rungs are strings two binaries agree on, so AGENTS.md rule 7 makes them a crate. That is not a
formality here: **the one marker that existed before was a literal inside the RISC-V arm of
`main.rs` and a second copy of it inside the recogniser**, which is finding 3's mechanism. Three
binaries now read one definition: the kernel, `swish`, and `board_console`. Name provisional.

### A fifth finding, the same shape as finding 3

**`Stage::Banner` was unreachable on aarch64.** It matches `nife on `, deliberately generic so that
"a recogniser that only knew the VisionFive 2's would report a healthy aarch64 or x86_64 board as
never having booted" (`progress.rs`, written in anticipation). aarch64 printed no such line at all:
its description opened with a bare `nife`. So the rung below the one this milestone was written
about had the same defect, found the same way, by looking for the marker rather than by anything
failing. aarch64 now prints `nife on aarch64 (EL1, MMU off: physical addresses until mmu::init)` as
the first line after `console::init`, which is where the other two have always printed theirs.

Two smaller ones, both fixed:

- **`arch::riscv64::mmu::print_summary` was `unimplemented!()`** and had never had a caller. Calling
  the same name on all three architectures found it on the first run, as a `[PANIC]` in the middle
  of the block it had just printed.
- **`arch::x86_64::exceptions::self_test` returned the cumulative breakpoint count** where the other
  two return a delta. Harmless while it had one caller; the second caller reported `2`.

### What `x86_64` could not reach, and why

**The prompt.** This block gated exactly that and nothing else, and the gate held: `x86_64` has no
entry point that hands the machine to a shell, so its boot still ends in `nife x86_64: boot
complete, halting.` after the self-test verdict. Both halves are needed and neither is this
milestone's: DECISIONS §149 for how `swish` reaches a console where §121 leaves no userspace holder,
and milestone 182 for the entry point. `crates/board_console`'s `Stage::Prompt` says so in its own
doc rather than leaving a reader to discover it from a watch that times out, and `script/boot-check`
says so in its `BUGS`.

Every rung below it landed on `x86_64` on the same terms as the other two.

### calef's ruling on `components/src/builder.rs` (2026-09-14), and what this lane did with it

calef, mid-lane: he expects `builder` to **go away** as a result of this milestone, because the claim
it carries (*userspace, not the kernel, composes a process*) becomes one of item 2's collected
self-tests, running on all three architectures instead of one. Its name is parked for that reason:
*"Skip this one because it will go away with parity"*, so the provisional `process_builder` in its
provenance block is not settled and was not touched.

**This lane did not remove it, and the reason is the ruling's own condition rather than scope.** The
ladder subsumes `builder`'s claim on riscv64 and can show it: item 4 made that architecture's
default boot hand over, so the same boot that runs `builder`'s `init/build` step now also watches
the progenitor load, measure and compose the console, the line discipline, the input driver and
`swish` out of its own budget, and offer a prompt. That is the same claim at a larger scale, on the
only architecture that ever ran `builder`.

**What is not true yet is "on all three".** A compose-a-process self-test needs a compiled ELF to run
at user level, and `x86_64` has none: `user_rt` has no `x86_64` arms, so that architecture runs
hand-assembled children and cannot load a program by name. Removing `builder` today would relocate
the claim on riscv64 and leave `x86_64` where it already is, which is *not* a loss, but it would also
close the door on the version calef actually described. And it is not free: the trust root
(`kernel/src/trust.rs`), the measured-boot manifest (`xtask`'s `boot_programs`), the archive table,
and `board_console`'s `userspace_ran` with its captured board transcript all name it.

So it is proposed rather than performed; see the follow-on below. The one-sentence decision calef
can make: whether the retirement waits for an `x86_64` leg that can run a compiled ELF, or goes
ahead now on the strength of the progenitor carrying the claim wherever a progenitor runs.

## BUGS (as built)

- **The machine description is printed twice-over on riscv64 and `x86_64`.** Both arms print their
  own bring-up narrative as each piece comes up (`isa`, `firmware`, `memory`, `pci`, `apic`), and
  the description then answers the same questions again in one block. The duplication is noise
  rather than a defect, and trimming the arms' copies is not free: the riscv64 arm's `isa` and
  `firmware` lines exist so that `test` and `bench` boots, which compile the description out, still
  report what machine they ran on. Proposed below.
- **Nothing proves an architecture ran the right five checks.** The verdict says five of five passed
  on a kernel that ran five; a check deleted from the list takes its own evidence with it. The count
  is the partial defence and review is the rest. `kernel/src/self_test.rs`'s own `BUGS` says this
  where a reader meets it.
- **The self-test can hang the boot.** `timer` and `scheduler` both wait, bounded by the
  free-running counter rather than by an iteration count, so a machine whose counter never advances
  sits in `timer` forever. Same exposure `arch::timer::spin_for` already has, recorded beside the
  code.
- **`script/boot-check` does not check the prompt**, for the reason in "What `x86_64` could not
  reach": asserting the top rung on two of three architectures would be the shape of defect this
  milestone exists to fix. `script/shell-check` reaches a prompt on the two that can.
- **The injected leg is not in CI.** `script/boot-check --inject` rebuilds three kernels for one
  boolean, and what it proves is a property of the gate rather than of the change under test. It is
  run by hand when the self-test or the recogniser changes; the transcript above is from the run
  that proved it. A gate for the gate, left at rung four deliberately and said out loud.
- **`Stage::Tour` is still one architecture's rung**, and is now documented as one rather than
  quietly left in the ladder. Levelling it up would mean giving aarch64 and `x86_64` a marker for a
  demonstration tour they do not have; levelling it down would delete riscv64's, which is the trap
  this block warns about. The portable rungs that replace it for every tool are `Machine` and
  `SelfTest`.
