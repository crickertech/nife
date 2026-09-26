# 595. The shell runs a `std` program, and `rg pattern` works at the prompt

**Status: NOT-STARTED.** Minted 2026-09-25 by the maintainer's lane `maintainer/shell-runs-std`, from
the gap #1314 recorded in `notes/foreign-program-arguments.md`'s `BUGS` section. *(Number and title
provisional: the integrator mints the number at merge, and the title is a draft until an architect
names it.)*

**Gate: DECISION §170, DECISION §171.** Two open forks each stop a different step between a typed
line and a running `rg`, and both are calef's. A third, §219, was ruled on 2026-09-26. The sections
below say which step each one stops.

## The gap, checked 2026-09-25

The shell cannot launch a `std` program at all. `swish` resolves a typed name through
`grant_plan::Prog` and sends its wire id to the progenitor, which indexes a table it filled at boot.
That enum has 16 variants, ids 0 to 15, and every one is a native program. No `std` binary has a row.

`ripgrep` does run on nife, on all three architectures, but only from the kernel test harness.
`kernel/src/user/ripgrep_tests.rs` spawns it through `fs_service::start_std_full`, which builds the
address space `std`'s runtime expects. The progenitor has no equivalent. Its loader,
`supervision_protocol::build_child`, has never produced a child in that layout. The layout is the
eight slots `patches/std-nife/overlay/std/src/sys/pal/nife/rt.rs` fixes, plus the file-service page
at `0x1100_0000`. The harness also maps 32 stack pages where a native child gets 12.

Nothing in `design/roadmap/` owns this. Milestone 121 (`ripgrep` on nife) owes a confined
demonstration, but its outstanding items are written against the harness. Milestone 198 (a package
manager) owes running an installed program, which is a different program arriving by a different
route.

## The goal

From the prompt, `rg needle docs` runs confined to the directories the line granted and prints its
matches. A path outside the grant is not found, because nothing names it. This holds on aarch64,
riscv64 and x86_64 per DECISIONS §19 (architectural parity is a tenet), or a scope note says which
architecture is missing and why.

## What it waits on

- §219 (how the shell names an installed program to the spawner), decided: option D with gate D2
  (calef, 2026-09-26, recorded by #1317). The shell sends a binary's bytes as frames it owns, which
  fits a program built by `helpers/build-ripgrep.sh` and run by path. That is also why `rg` cannot be
  an enum row: it is never in an ordinary archive, because fetching its crates in CI is a §46 (thin
  primitives or whole subsystems) decision nobody has made. Two consequences follow. Such an `rg`
  is unvouched, so the session needs D2's capability to run it. And an unvouched child gets only
  what the line delegates, plus the clock and configuration pages, so its directory must come from
  the line. That is the confinement this milestone wants. Option D is not built yet; milestone 198
  (a package manager) owes it.
- §170 (how a foreign program is told what to do), and milestone 205 (the nife ABI has no argument
  vector), which builds the answer.
  `std::env::args()` yields nothing on nife, so `rg` prints its own usage text and stops. #1314
  measured the answer in two halves. Getting bytes into `args_os()` is one library of about 50
  lines, and every option needs it. Deciding which bytes become capabilities is per program:
  `ripgrep` 14.1.1 has 104 flags. This milestone needs the first half and the decision on the
  second.
- Milestone 206 (a program image has under 896 KiB), and §171 (where a program image starts).
  The image meets its own stack there, and `rg`'s `.text` is 1.37 MiB. The harness relinks `rg` at
  `0x100_0000` to get past it. A spawn from the shell cannot rely on that trick.

The std address-space layout in the progenitor is this milestone's own work and waits on nothing.
It could start today, proven with `std_exerciser`, which is in every archive.

## What it unblocks

- Milestone 121's three outstanding items: the confined demonstration, the loud refusal of a
  directory lacking `ENUMERATE`, and the benchmark that prices the walk. Each becomes a typed line
  rather than a harness call.
- Milestone 123 (the demonstration: somebody else's software, running narrow). Its first element is
  a ported tool run here and on Linux over one corpus. A tool a person cannot type is a harness
  result, not a demonstration.

## Which fatal risks it serves

Risk 1 (only software written for nife runs on nife) is green in the harness and not at the prompt.
A stranger does not run the kernel test suite, so for them the risk is still open until this lands.
Risk 7 (the confinement claim is false) gets its most legible test: a search that cannot see outside
its grant, run by the person making the claim.

## The gate that proves it

A boot test, `shell_runs_std_tests.rs` (provisional name), drives a scripted shell the way
`pipeline_tests.rs` does. It has two halves, because `rg` is absent from CI.

- In every build, on all three architectures: the shell spawns an in-tree `std` program with a
  string argument and a directory grant. The program prints both back, and the transcript is
  asserted byte for byte.
- When `rg` is in the archive: `rg needle docs` prints the expected matches. `rg needle ..` with only
  `docs` granted finds nothing outside it. A grant without `ENUMERATE` is refused with `EPERM`,
  never an empty listing. It skips with the same reason `ripgrep_tests.rs` gives when `rg` is
  absent.

## BUGS

- The CI half proves the mechanism with a program this project wrote. Only the second half answers
  risk 1, and it runs only on a machine that built `rg`.
- Designation stays open. Until §170 rules, nothing says whether `docs` on the line becomes a
  capability because the shell guessed it is a path or because `rg`'s manifest said so.

## Index row

The shell cannot launch a `std` program: all 16 programs `swish` can name are native, and `ripgrep`
runs only from the kernel test harness. This makes `rg needle docs` work at the prompt, confined to
the granted directories, on all three architectures. It builds on §219's option D for naming and
waits on §170 for arguments and §171 for image size, and it unblocks milestone 121's remaining items and milestone
123's demonstration.
