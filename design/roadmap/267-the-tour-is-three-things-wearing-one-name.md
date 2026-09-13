# 267. The milestone tour is three things wearing one name, and only one of them belongs in the kernel

**Status: PARTIAL.** Minted 2026-09-09 by calef, from one question: *"Does the milestone tour need
to be part of the build at all? Can the milestone tour just be a userspace program that we run if we
want the milestone tour?"* Built on `milestone/267-tour-split`. The split and the move are done and
the measurement is below; the half that is not done is that the program cannot be typed at a prompt.

**Gate: NONE.** Everything here is code this project owns, and milestone 266 (one progenitor) landed
first, which is what makes `kernel/src/main.rs` safe to open.

## The measurement, first, because it changed what this milestone is about

`script/fastpath-footprint`'s method: release kernels, `llvm-nm --print-size`, `.text` symbols
summed. Measured at `8dd9dbf6`, before anything moved.

| arch | tour build | `--features shell` | what the tour costs |
|---|---|---|---|
| aarch64 | 193,332 | 179,312 | **14,020 (7.3%)** |
| riscv64 | 162,964 | 157,136 | 5,828, and it is not the tour |
| x86_64 | 140,842 | 140,842 | **0** |

**x86_64 is zero, and had always been.** Its boot arm is self-contained and ends in `arch::halt()`,
so the shared milestone tour is unreachable and LLVM had already deleted it. The boot-mode features
have never removed a byte on that architecture.

**riscv64's number is a different quantity.** That arm halts too, and its `shell` feature swaps its
own arch tour for `user::riscv_shell_boot`. 5,828 bytes is the difference between two RISC-V boot
paths, not the cost of the 230 lines this block was written about.

So the tour is **one architecture's 14 KB**. And the symbol-level attribution is the part that
matters, because it says the narrative is not where the bytes are:

| tour-only symbol group | bytes |
|---|---|
| `console_service::start` + `spawn_client` and their closures | ~3,040 |
| the two never-yielding spinner closures in `kernel_main` | ~3,336 |
| `virtio_service::wire` and its closures | ~1,692 |
| `memory_region_service::start` and its closures | ~1,276 |
| `kernel_main` itself (the driving code and the format strings) | +1,460 |
| `pci::find_block_device` | 436 |

Every one of those is a **demonstration**, and every demonstration is on the kernel-only list below.
The prose lived in `.rodata` as anonymous string constants, below the linker's 4 KiB section
granularity and, as it turned out, below the optimizer's noise floor: writing the new console-server
hoist two equivalent ways moved `.text` by 340 bytes, which is more than nine lines of text ever
weighed.

**After the move: 193,996 tour, 179,332 shell. The kernel is 664 bytes larger than it was.** That is
the finding and it is worth stating without dressing: the compile-time exclusion was not buying what
its name suggests, and moving the narrative out cannot make it buy less, because what costs bytes is
exactly what cannot leave.

The consequences for the feature pair are in
`design/roadmap/proposals/two-boot-mode-features-for-one-decision.md`. **Nothing was retired here.**

## The three things, which want opposite treatment

Boot output was one undifferentiated stream, compiled in or out as a unit. It is three audiences.

**The machine description** is diagnostics and belongs on **every** boot, never compiled out.
Paging, ISA, firmware, cores, ACPI tables, PCI ECAM, timers, memory map. **xenon proved this is not
decoration**: at first light there was no serial console this project could read, so the boot output
*was* the transcript, read off a photograph, and it is what diagnosed both the local-APIC collision
and the PCI BAR window landing in RAM. A port is verified by reading these lines.

**The milestone narrative** is a demonstration. *"milestone 1: we are running our own code on a CPU
with nothing underneath it."*

**And a genuine kernel remainder** that cannot move anywhere, because it demonstrates facts only
kernel code can establish.

## What was built

**1. The machine description is `print_machine_description`,** called unconditionally from
`kernel_main`, with byte-identical output. It was not gated on `shell` or `initboot` before either,
which is the one thing this block's original text got wrong: the `cfg` at what was line 1540 sat
*after* the description and governed only the tour. What was true is the rest of the diagnosis, that
the boundary between the two audiences was a reader's inference about where a line happened to sit,
and a line moved a few rows down would have changed audience with nothing saying so.

`script/lint` now fails if a `feature = "..."` cfg appears anywhere inside that function. The
signature keeps `#[cfg(not(any(test, feature = "bench")))]` and the check deliberately does not look
at it: a `bench` boot diverges into `bench::run` before this point and a `test` boot exits through
semihosting, and neither is a boot anybody reads to bring up a board.

**2. The narrative is `components/src/narrator.rs`,** a program at EL0, spawned as the console server's
client. The console server now comes up at the top of the tour instead of the middle, which keeps
the transcript in the order it always read, and the `hello` printing client it used to feed is gone:
running two programs to demonstrate one thing is one program too many.

This closed a gap nobody had named. The kernel used to print *"a userspace program printed to the
screen, and the kernel does not contain a line of code that puts a user's bytes on the wire"* on the
program's behalf, which was the only claim in the tour not demonstrated by the thing making it. The
narrator says it, through a driver at EL0 that holds the UART, from an address space that cannot
reach the device at all.

**3. The remainder is listed in one comment** at the top of the tour block, with the privilege each
entry needs:

| survivor | why it cannot leave |
|---|---|
| two threads that never yield | `sched::spawn`, `sched::preemptions()`, `timer::spin_for`. An EL0 spinner proves only that EL0 is preempted; these run *inside* the kernel and are still taken off the CPU, which is the stronger claim |
| the virtio and PCIe block demos | the kernel enumerates the bus and mints registers, a DMA page and an interrupt into a driver's world, then reads the report with `sched::ipc_recv`. The granter cannot be the grantee |
| the outlaw | `&raw const USER_FAULTS` is the address of a kernel static, handed to a program that faults reading it and increments the counter it reached for. A program cannot name that address |
| the memory-region demo | it prints `memory::stats().used` before and after a process spends its own budget, and the claim is that the number did not move. That number is the kernel's own frame accounting and is not exposed to EL0 |

**4. The measurement is above.**

## The `Reached(Tour)` consumers, since the block asked for them by name

All of them, and none of them observe the thing this milestone moved.

`Stage::Tour` is reached by exactly one line of text, `"nife: the capability core runs on "`, printed
at `kernel/src/main.rs:1401` in the **RISC-V** arm. The aarch64 tour prints no marker at all. The
consumers are `crates/board_console/src/progress.rs` (which reaches the stage and labels it),
`watch.rs` (whose `quiet_after` policy exempts exactly this stage, because a boot that ends in `wfi`
is quiet on purpose), `port.rs`, and `xtask/src/main.rs:9383`, which parses `--until tour` on the
command line. Six tests in `board_console` assert on it.

Nothing in that set was touched, and nothing in it could have been: it is the VisionFive 2's boot
that it watches.

## What must not break, and did not

- **The machine description prints on every boot, on all three architectures.** It is now harder to
  break than it was: it is a function with a name and a gate rather than the first half of a block.
- **`xtask`'s archive manifest.** `narrator` is packed into `initrd_aarch64`'s table. aarch64 only:
  the other two boots halt before the shared path, so packing it there would be archive bytes
  nothing can reach.
- **`script/shell-check` runs the real boot on both legs.** Green.

## The proof

`script/server` prints the machine description, then the narrative arrives from a program at EL0
through a driver at EL0, then the kernel-only demonstrations, then the progenitor builds the system
and the prompt appears. `--features shell` prints the machine description and goes straight to the
prompt.

## BUGS

- **The narrative is the project's front door and this milestone kept it, deliberately against this
  block's own "proof" paragraph.** That paragraph asked for *"a boot that prints the machine
  description and nothing else"*. This block's own BUGS section argued the opposite in the same
  breath, that `script/server` is what a stranger runs first and *"run this other program" is a
  worse default than "it prints"*. The second argument won. The default boot still prints the
  narrative; what changed is that a program prints it. A reader who wanted the first reading should
  know it was read, weighed, and refused, rather than missed.
- **You cannot type `narrator` at the prompt, which is the other half of "a program you can run".**
  It speaks the console server's raw protocol (two rendezvous endpoints and a shared page, the shape
  `fixtures/src/hello.rs`'s `printing_client` has), and `swish` starts a program with an output sink
  (`crates/byte_sink_proto`) instead. Three ways to fix it, all of them more than a line, in
  `design/roadmap/proposals/a-narrative-program-the-shell-cannot-start.md`.
- **`narrator` is a provisional name.** Every name in this tree is calef's. `tour` was refused
  because this tree already spends the word on the boot path itself (`Stage::Tour`,
  `script/board-console`), and reusing it for one program inside that boot would make the existing
  term ambiguous. The case for and against is in the program's own header.
- **The narrative names milestones 1 through 11 and nothing checks that against the roadmap.** There
  are 267 milestones. The text stops early on purpose, because it is the argument for the design
  rather than a changelog, but no gate says so and nothing would notice if it drifted. It was
  equally unchecked as a `println!`, so this is inherited rather than introduced.
- **The move made the kernel 664 bytes bigger.** Measured, above. It bought the split, the
  self-demonstrating claim, and a program; it did not buy bytes, and a reader should not infer that
  it did from the fact that a milestone about the cost of the tour exists.
- **The interleaving hazard grew slightly.** The kernel's own UART driver and the userspace console
  server write the same device with nothing arbitrating, which `script/shell-check`'s BUGS section
  documents at length and milestone 230 proposes fixing. There are fifteen more userspace-printed
  lines on the default boot than there were. It is confined to the tour boot: `script/test` compiles
  the tour out, `script/shell-check` runs `--features shell`, and neither sees these lines.
- **Preemption counts are still not proposed for exposure.** A supervisor or benchmark might want
  them one day, and the shape is already established by §139 and milestones 229/237: a per-thread
  grant, not an ambient fact. Nothing needs it today, so nothing is built. This is why the spinner
  demo is on the kernel-only list rather than being a program.

## Follow-on

- **Proposed.** The narrator speaks the console server's raw protocol, so `swish` cannot start it
  and the "a program you can run on purpose" half of this milestone is not delivered. Three options
  with their costs, including the one that needs a word on the kernel-to-progenitor handoff and is
  therefore calef's, in `design/roadmap/proposals/a-narrative-program-the-shell-cannot-start.md`.
- **Proposed.** Whether `initboot` and `shell` should collapse to one feature, or stop being a
  compile-time switch at all, now that the measurement exists: 7.3% on one architecture, zero on
  x86_64, and something else entirely on riscv64. Not acted on here, on purpose.
  `design/roadmap/proposals/two-boot-mode-features-for-one-decision.md`.
- **Outstanding.** `design/roadmap/proposals/what-the-boot-path-is-called.md` is **not** retired by
  this milestone, which is the opposite of what this block predicted. Checked by reading it: it
  chooses between `progenitor-boot` and `handoff` for the boot mode, and both features still exist
  and still mean the same thing, so the question it holds is untouched. What changed is that there
  is now a fourth candidate, because the feature is better described as naming the absence of the
  demonstrations than as naming a boot path.
- **Recorded.** The kernel's UART driver and the userspace console server write the same device
  unarbitrated, and this milestone put fifteen more userspace lines on the default boot. The
  limitation is `script/shell-check`'s and milestone 230's; the increment is recorded in the BUGS
  section above and in `kernel/src/main.rs`'s tour comment, where the `spin_for` window that keeps
  the two writers apart is named.
