# nife

**A capability microkernel in Rust, built from the first instruction, where every driver and server
is an ordinary process and the trusted core is small enough to say its size out loud.**

Most operating systems ask you to trust millions of lines. Here the kernel allocates no memory of
its own, every filesystem, driver and network stack is a confined process at EL0, and the trusted
base is **694 `unsafe` blocks in 39,892 lines of kernel code**. It boots on **aarch64, riscv64 and
x86_64**, on real silicon as well as under emulation, and it runs software nobody wrote for it:
unmodified `ripgrep` 14.1.1 from crates.io, **zero patches**, with byte-identical transcripts from
three separately built binaries.

It is a **demonstrator**, which is what §14 (the project's direction) committed it to: built to
stand next to Linux, macOS and seL4 on the primitives that define an operating system, and to win
where a minimal kernel should.

**And it was built in about ten weeks by one person who does not write the lines.** Many machine
agents work in parallel lanes; one architect reviews architecture and outcomes. What makes that
safe rather than reckless is the gate discipline: every architecture builds and boots or the merge
fails, every proof must have a recorded way to go red, every citation must say what it cites, every
refusal must say what would change it, and every known limitation sits beside the feature it limits
rather than in a tracker. [notes/how-this-is-built.md](notes/how-this-is-built.md) has the numbers
and the caveats that make them mean something.

**Start with the part that could kill it.**
[design/fatal-risks.md](design/fatal-risks.md) lists the nine claims that, if false, mean this
project should stop, each with the experiment that would settle it and the honest verdict so far.
Two are amber. **One already fired**: the first customer went to Linux, because nife could not meet
a real deadline.

---

*The name is lowercase everywhere, sentence starts included, and is said like* knife: *Ni + Fe,
the Earth's nickel-iron core. The full story, refused spellings included, is
[design/naming.md](design/naming.md).*

<img src="art/cobble-realistic.jpg" alt="Cobble, the nife mascot: a stone golem with red eyes and mossy shoulders, holding a gear" width="300">

*Cobble, guardian of the machinery. Designed by Clay; realistic render. The first draft is
[art/cobble-first-draft.jpg](art/cobble-first-draft.jpg) and the full naming record is
[notes/mascot.md](notes/mascot.md).*

[![CI](https://github.com/crickertech/nife/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/crickertech/nife/actions/workflows/ci.yml)

The capability core carries machine-checked proofs, the same portable core boots on three ISAs, and
the kernel has run on real RISC-V silicon.

## Try it

```
script/setup               # one time: install the toolchain and QEMU, then build
script/console             # boot straight to an interactive shell at EL0
script/console --hvf       # ...on the real Apple Silicon core (instant boot)
script/server              # the full milestone tour, then the shell
script/test                # host tests, then the kernel under QEMU, all three ISAs
script/verify              # the machine-checked proofs (Kani)
script/bench               # icount microbenchmarks against the committed baseline
```

`script/*` is the normalized "Scripts to Rule Them All" front door; each is a thin wrapper over
`cargo xtask`, which still does the work (`cargo xtask shell` and friends work too).

**The list above is a deliberate subset**, the seven commands worth knowing on day one. The complete
reference is [notes/scripts.md](notes/scripts.md), and `script/lint` checks that every script has an
entry there and that nothing named here has since been renamed away. Two documents, one comprehensive
and one curated, is fine; two documents both claiming to be complete is not, which is the mistake the
Status section above made twice.

At the `$` prompt: `help`, `echo hello`, `least_authority_demo 7` (spawns a process that returns 49). Quit with
Ctrl-C, or `pkill qemu-system-aarch64` from another terminal.

## Start here

There are over 500 markdown files <!--count-at-least:markdown-files--> here, and more than
150 notes <!--count-at-least:notes-files-->; the problem a newcomer has is not missing documents,
it is that nothing says which one is first. Read these in this order and stop when you have what
you came for.

1. **Run it.** The block above, in about the time it takes to read the next item. A system you have
   watched boot is a different thing to read about than one you have not.
2. **[`CONTRIBUTING.md`](CONTRIBUTING.md)** if you might change something. What this project wants, what
   it will cost you, what is yours to decide, and what to bring up instead of building.
3. **[`AGENTS.md`](AGENTS.md)**, which is the project's constitution and is the file most of the
   rules live in and nowhere else. It addresses whoever is doing the work, human or agent.
   `CLAUDE.md` at the root is a symlink to it; the two are one file, and the section below says why
   the name misleads.
4. **[`notes/capabilities.md`](notes/capabilities.md)**, the one idea everything else is downstream
   of. If a design choice here looks strange, this is usually the reason.
5. **[`design/roadmap/`](design/roadmap/)**, the only status in the tree: one file per milestone,
   with a fixed vocabulary and a checker. `script/roadmap --ready` is the query most readers want.
   Anywhere else that claims status is stale by construction.
6. **[`design/decisions/`](design/decisions/README.md)**, one file per decision, cited elsewhere in
   the tree as `§N`, when you want the argument behind a specific choice, including the ones that
   were refused.
7. **[`notes/adding-a-program.md`](notes/adding-a-program.md)**, which is the first thing to do with
   your hands rather than your eyes. Doing it is how you find out whether you understood 4.
8. **[`notes/`](notes/README.md)**, by question, not in order. It is a glossary written while
   building, one file per question that turned out to be load-bearing.

**If you read only two**, make them 3 and 4: the rules, and the idea. Everything else you can look up
when it bites.

**Looking it up is one command**, and it is the one to reach for before you grep:

```
$ script/apropos syscall
     25  design/decisions/124-x86-64-syscall-abi.md        124. Ratify the x86_64 syscall ABI
      3  design/decisions/08-process-model-deferred.md     8. Process model / syscall ABI: DEFERRED ...
    ...
      5  crates/abi/src/lib.rs                             crate abi
```

It searches every note, decision and roadmap block plus every crate's and program's own module
header, and prints the path to open. It exists because five strangers in a row doing ordinary work
never reached a `design/decisions/` file, `notes/net.md`, or `crates/abi/src/lib.rs` (the syscall
numbers, on one screen), though none of them is hidden. One word per search; its own header lists
what else it cannot do.

## What the badge means

The CI badge above is green only when **every** gate passes:

| Gate | What it proves |
|---|---|
| `script/test` | The host-logic crates, then the kernel under QEMU on **all three ISAs**: aarch64, riscv64 and x86_64. Architectural parity is a gate, not an aspiration (DECISIONS §19 (architectural parity is a tenet)). |
| `script/verify` | over 100 Kani harnesses <!--count-at-least:kani-harnesses--> across more than 20 crates <!--count-at-least:harness-crates-->: the capability model, IPC, MMU isolation, the DMA validator, the IOMMU domain, the NTP era pivot. |
| `script/bench --check` | icount instruction counts against a committed baseline, on all three ISAs, so a performance regression surfaces next to the change that caused it. |
| `script/lint` | clippy at `-D warnings`, plus broken intra-doc links, stray conflict markers, the roadmap's status vocabulary, DECISIONS numbering and citations, and that every script is documented. |
| `script/supply-chain` | cargo-deny (advisories, licences, bans, sources) over every workspace, and proof that each vendored tree is the published tarball plus exactly its recorded patches. |
| `script/fuzz` | Coverage-guided fuzzing of the four parsers that read bytes we did not write (a device tree from firmware, an ELF the loader will map, a partition table off somebody else's disk, and the boot archive's round trip). The complement to the proofs, not a second opinion on them: they are exhaustive inside a bound, this is unbounded and random. It found two bugs on its first day. |
| `script/fmt --check`, coverage | Formatting, and an 80%-per-file line-coverage floor on the host crates. |

CI runs on an **aarch64** runner deliberately: this kernel targets a weakly-ordered machine, and a
missing `Acquire`/`Release` passes on an x86_64 host and fails only on real ARM. The Rust toolchain
is pinned to an exact nightly everywhere. QEMU is pinned to an exact version (`.qemu-version`) on CI
and on Linux, where `script/ci-qemu` builds it; **on macOS it is whatever Homebrew ships**, because
Homebrew cannot install an older release, and `script/qemu-check` warns rather than fails when the
two differ. So on a Mac "the tests passed" means the same thing as on a runner only up to that
emulator difference, which `script/qemu-check`'s header prices.

## What it does

This section is deliberately **not** status. Status lives in one place, with a gated status line and
a checker: **[design/roadmap/](design/roadmap/)**. What follows is what the system *is*, and each
claim points at the artifact that keeps it true rather than repeating a list that goes stale. The
previous version of this section did repeat them, and drifted twice inside three days.

- **The security-critical logic carries machine-checked proofs.** Kani, run by `script/verify` and
  gated in CI. Which crates and which properties, with the bounds and their justifications, is
  [notes/verification.md](notes/verification.md); the count is whatever the gate prints.
- **The kernel does not allocate.** There is no kernel heap. Page tables, TCBs, endpoints, and
  address spaces are all retyped out of untyped memory that userspace owns and pays for.
- **Processes come and go.** A userspace progenitor builds the whole system through granular
  capability verbs (retype, configure, insert, start), and object revocation tears a process
  back down: its TCBs, address spaces, endpoints, and the memory behind them, reclaimed safely.
- **It runs real workloads.** A CoreMark-derived compute program against the written native ABI
  ([notes/abi.md](notes/abi.md)), and ordinary Rust `std` programs on a custom target.
- **Two ISAs at parity.** Everything architecture-specific lives under `kernel/src/arch/`, and
  riscv64 proves it: SMP, the whole test suite, the interactive shell, and the benchmarks all run on
  both. Parity is a gate rather than an aspiration (DECISIONS §19 (architectural parity is a
  tenet)).
- **SMP.** Four cores via PSCI (aarch64), SBI (riscv64) and the APIC's startup IPI (x86_64), per-CPU run queues, cross-core
  placement by inbox plus a reschedule IPI. No shared run-queue lock.
- **Every driver and server is an EL0 process**, confined by the MMU and, for DMA, by a validator
  and an IOMMU. A driver that misbehaves faults; it does not take the kernel with it.
- **Benchmarked against Linux and macOS, honestly.** Same Apple Silicon core, same virtualization
  tier, release builds. Every number, and every caveat that makes a comparison not apples to apples,
  is in [notes/benchmarks.md](notes/benchmarks.md), which is the only place they are written down.

When something faults, you get this instead of a silent death:

```
[EXCEPTION]  Current EL, SP_ELx, Synchronous
             Data abort from the same EL (EC 0x25)

  ESR_EL1   0x0000000096000050   what happened
  FAR_EL1   0x00000000dead0000   the address that faulted
  ELR_EL1   0x0000000040081a40   the instruction that did it
  SPSR_EL1  0x00000000400003c5   the state it was in
```

## Quick start

```bash
git clone https://github.com/crickertech/nife
cd nife
script/setup               # installs the pinned Rust toolchain and QEMU, then builds

script/server              # boot it
script/test                # run the tests
script/console             # boot straight to the interactive shell
```

`script/server` boots the kernel on QEMU's `virt` machine and wires the emulated UART to your
terminal. Ctrl-A then X quits QEMU.

**On Linux, `script/setup` will stop at the QEMU check and tell you to run `script/ci-qemu` first.**
That is expected rather than broken: no Ubuntu release ships a QEMU with `riscv-iommu-pci`, and the
project refuses to drop the device, because a confinement test that quietly stops testing is worse
than a red build. Build the pinned QEMU (about twelve minutes), then run `script/setup` again.

```bash
script/catch-up            # what changed since you last looked
```

**`script/catch-up` is the one to run second**, and it is worth knowing about before you need it: it
recomputes what moved (milestone status, decisions landed, what is waiting on calef, what is ready to
start) from the roadmap, the decision files and git, rather than from a hand-written status page that
would rot. The second run of milestone 117 (the stranger test) called it the best onboarding command
here and noted that nothing pointed at it, which this paragraph is fixing.

The `script/*` commands are the normalized entry points (the [Scripts to Rule Them
All](https://github.com/github/scripts-to-rule-them-all) pattern, one interface across every
repo). They are thin wrappers over `cargo xtask`, which still does the work and exposes the rest:

```bash
cargo xtask objdump        # disassemble it
cargo xtask image          # build the flat arm64 Image and dump its header
cargo xtask gdb            # boot paused, waiting for a debugger on :1234
cargo xtask bench --riscv  # the benchmark suite on the second ISA
```

## What's here

```
kernel/
  src/arch/aarch64/    boot.s, vectors, MMU, GIC, timer, PSCI: everything ISA-specific
  src/arch/riscv64/    the same boundary, proved by a second ISA (SBI, Sv39, PLIC)
  src/arch/x86_64/     and by a third (PVH and UEFI boot, IA-32e paging, APIC, VT-d)
  src/drivers/         pl011, ns16550: a driver gets a base address and nothing else
  src/                 capabilities, scheduler, IPC, untyped, revocation, the syscall surface
user/                  EL0: the progenitor, the shell, the console/input/block drivers, servers
crates/                pure logic, host-tested in milliseconds: capability, paging, elf, pci,
                       inter_process_communication, device_tree_blob, page_frames, nifefs,
                       intrusive_fifo, address_space_identifier, ...
bench/                 the benchmark suite and committed baselines (all three ISAs)
script/                normalized entry points (setup, test, console, verify, bench, ...)
xtask/                 build orchestration (build, run, test, bench, gdb, objdump, image)
notes/                 a concept glossary, written as questions came up
design/                the roadmap and worked designs
design/decisions/      what we chose, what we rejected, and why
design/journeys/       end-to-end user stories, tracked as a bundle of the milestones they need
design/audit-reports/  every audit, its lens, and when the next one is due
design/fatal-risks.md  the nine claims that, if false, mean the project should stop
notes/project-metrics.md  what moved, week by week, computed from git history
```

## The notes are the point

[`notes/`](notes/) is a running glossary written *while* building, not afterward. Every
file in it exists because a specific question came up and the answer turned out to be
load-bearing for code we actually wrote.

If any of the code looks like noise, start with
[**Reading aarch64 assembly**](notes/reading-assembly.md) and
[**Registers**](notes/registers.md). The second one is the most fundamental thing in the
repo: the register file *is* the CPU's state, in about 248 bytes, which is why context
switches and interrupts work the way they do.

Also in there: [what an MMU is](notes/mmu.md), [why the stack
exists](notes/stack.md), [what `no_std` actually removes](notes/no-std.md), [what a linker
script is for](notes/linker-scripts.md), [what QEMU is](notes/qemu.md), and [how portable
kernels are structured](notes/portability.md).

## Milestones

**Not repeated here.** They live in **[design/roadmap/](design/roadmap/)**, one file per milestone,
each carrying a status line with a fixed vocabulary, and a checker (`script/roadmap`) that fails the
build if a milestone is cited in prose without a block, or carries a status outside the vocabulary.
There is no index: `script/roadmap --ready` answers what a newcomer usually came to ask, and the
vocabulary is in [notes/roadmap.md](notes/roadmap.md). This file used to hold a
second copy: fifty-two lines of tick-marks, a partial and out-of-order subset, and nothing checking
it. A duplicate of a gated artifact is the copy that goes stale, because only one of them has the
gate.

If you want the shape rather than the list: milestone 7 (user mode: EL0, capabilities, the ELF
loader, and IPC) is the dividing line between "a Rust program
that boots" and "an operating system": it is where EL0, address spaces, capabilities, the ELF loader
and IPC arrive together.

## The numbers, weekly

[![Milestones by status](notes/project-metrics/milestones.svg)](notes/project-metrics.md)

**[notes/project-metrics.md](notes/project-metrics.md)** is one row per ISO week: milestones and
decisions by status, code against comments, `BUGS` sections, Kani harnesses and what can falsify
them, `unsafe` density, how many of the nine fatal risks have been put to an experiment, and how far
the tree's prose runs over its own 3,000-word cap. The data is one CSV per measure in
[`notes/project-metrics/`](notes/project-metrics/), in the tree, and `script/metrics` regenerates all
of it from git history.

**Read the page's first section before you quote a bar.** Every row is a restatement: the series
applies today's definitions to old commits, which is what makes it comparable and is not what any of
those numbers were reported as at the time. Coverage is the one column that cannot be recovered from
history, and it is left visibly empty rather than dropped.

## Things this project has already gotten wrong

Kept on purpose, because the corrections were the most instructive part: a device tree pointer QEMU
never passed, a return address that does not go on the stack, and a kernel that executed its own
overwritten code. [notes/corrections.md](notes/corrections.md) has them, each pointing at the note
that carries the full account.

## Security

[SECURITY.md](SECURITY.md) says what is in scope (the confinement boundaries this kernel claims to
enforce), what is not (a demonstrator under QEMU is not a production system), and how to report
something privately. Two adversarial reviews are already on the record:
[notes/security.md](notes/security.md) and [notes/arch-audit.md](notes/arch-audit.md).

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
