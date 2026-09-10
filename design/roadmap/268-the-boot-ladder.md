# 268. Every architecture boots the same way: describe the machine, test yourself, hand over

**Status: NOT-STARTED.** Minted 2026-09-09 by calef, from milestone 267's measurement and the parity
review that followed it. *(Number provisional until the merge queue lands it.)*

**Gate: NONE for the work; DECISIONS §149 for the destination.** Nothing in this block is a design
fork. The boot ladder is decided (calef, 2026-09-09, in conversation) and what remains is building
it. Where the ladder *ends* on x86_64 is §149's question, and this milestone does not wait on it:
every rung below the last one runs before userspace exists.

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
- **Decision.** DECISIONS §149, how `swish` reaches a console on x86_64 where §121 leaves no
  userspace holder.
- **Milestone 269.** `machine` as a program that can be run from the prompt.
- **Recorded.** The `attach_screen` asymmetry is named in BUGS above rather than left for a lane to
  rediscover, because the wrong move is the obvious one.
