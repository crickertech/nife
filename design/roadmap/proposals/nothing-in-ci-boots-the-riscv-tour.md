# Nothing on a pull request boots the riscv64 tour, so its userspace step is unasserted

**Status: PROPOSED 2026-09-14.** Written by milestone 289's lane, which was sent to find out whether
the tour was vestigial and found that it is live, unasserted, and that those two look identical from
a grep.

**Gate: DECISION.** Where the check goes is calef's: a CI job, `script/gates`, or a
`script/cadence-check` entry. It boots a kernel and it is not free, and milestone 232's own refusal
to assume CI is the answer for an expensive instrument applies here unchanged. The wiring and the
measurement are a lane's; the placement is not.

## The claim in one line

The **default** riscv64 kernel, which is the build `script/board-image` puts on a card, is booted by
nothing that runs on a pull request.

## What each riscv64 check actually boots, traced rather than assumed

| check | build | reaches the tour? |
|---|---|---|
| `script/test` (riscv64 leg) | `#[cfg(test)]` | No. `main.rs`'s test arm runs `test_main()` and exits via semihosting **before** the tour block. |
| `script/cpu-matrix` | the same test build, five CPU models (`rv64 sifive-u54 rva22s64 rva23s64 thead-c906`) | No, same arm. |
| `script/shell-check` (riscv64 leg) | `--features shell` | No. `riscv_shell_boot` runs and `arch::halt()`s before the tour. |
| `script/bench --riscv --check` | `--features bench` | No. `bench::run()` parks before the tour. |
| `script/icount` | `--features icount` | No, same shape. |
| `script/soak --arch riscv64` | `--features soak` | **Yes**, whole tour, then the workload. Not in CI. |
| `script/job-mix --arch riscv64` | `--features job_mix` | **Yes**, same shape. Not in CI. |
| `script/board-image` | `board` | **Yes**, on a board. Not in CI, and not in QEMU. |

Both `bench::run()` and `icount::run()` are `-> !` and end in `arch::halt()`, so those two are not
"skips the rest" by accident; they cannot reach the tour by construction. The `#[cfg(test)]` arm ends
`test_main(); arch::halt();` for the same reason.

So every step of that boot is asserted by nothing on a pull request: the userspace-builder step
(`init/build`), the preemption claim, the UART-driver step's device IRQ, the virtio and PCIe probes,
the hardware-entropy step's line, and the final banner. Some of what they demonstrate is covered
independently by the riscv64 kernel suite, which is why this is a gap rather than a hole; what is not
covered is the **boot as a sequence**, which is exactly what a board produces and what
`crates/board_console` was written to read. They are checked today by a person remembering to type
`script/soak`, which is AGENTS.md's rung four holding a claim `design/fatal-risks.md` cites.

## Why this is worth a milestone rather than a patch

**It is what made a live step look dead.** Milestone 289 was minted on the reasonable suspicion that
`components/src/builder.rs` was vestigial. Every signal that would have said otherwise was outside
CI: a crate's host tests over captured board logs, two `script/` tools nobody runs on a branch, and a
bench note. A step nothing asserts and a step nothing needs produce the same evidence, and this tree
has already spent one lane telling them apart.

**The recogniser already exists and is tested.** `crates/board_console` parses this exact transcript:
`Progress` ratchets `spl`/`opensbi`/`uboot`/`handoff`/`banner`/`tour`, recognises five failure
markers, and sets `userspace_ran` from the `init/build` line. `script/soak` already judges a QEMU run
with it. So the missing piece is a caller that boots the default kernel with `-initrd` under
`scripts/qemu-bounded.sh` and asks `Progress` whether it reached `Stage::Tour` with `userspace_ran`,
not a new instrument.

## What it would take, priced

The boot itself is one `cargo xtask` verb over machinery that is all present: `initrd_riscv()`,
`scripts/qemu-runner-riscv64.sh`, `crates/board_console`. A bounded tour boot is seconds, not
minutes, because the tour halts on its own; the cost is the riscv64 kernel build, which
`script/ci-build` already pays for other reasons.

The open questions are calef's and are the reason this is not just done:

- **Where it runs.** A CI job of its own, an arm of `script/ci-build` (which is where
  `script/shell-check` was put, for the same "reuse the build" reason), `script/gates`, or a cadence.
- **What it asserts.** Reaching `Stage::Tour` with `userspace_ran()` is the floor. The device-IRQ and
  preemption lines are the other two claims a reader would expect a tour check to make, and asserting
  them means recognising them, which is two more `Progress` fields.
- **Its name**, since it is a new `script/` entry point or a new xtask verb, and names are calef's.

## The aarch64 and x86_64 halves, named so they are not discovered later

aarch64's equivalent of the tour is the `initboot` path and it *is* exercised, by
`script/shell-check`. x86_64 has neither a shell boot nor a tour step that loads userspace from an
archive, and `shell_check()` says so in its own comment. So this proposal is riscv64-shaped on
purpose, and a general "boot every architecture's default kernel and read the transcript" is a larger
thing that should be argued for separately rather than assumed here.
