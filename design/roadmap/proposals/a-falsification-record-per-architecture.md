# A falsification record per architecture

**Status: PROPOSED 2026-09-17.** Raised by milestone 313's security audit (finding 5).

**Gate: NONE.** `script/falsifications` and its record format are the only things touched.

## What is being proposed

Let a kernel test carry one falsification record **per architecture it runs on**, so a portable
confinement test is evidenced on every leg rather than on the one its patch happens to name. Two
shapes fit the existing convention and either would do: `<module.path>.<fn>.<arch>.patch` beside the
bare name, or an `Architecture:` line that lists several with one patch that applies to all of them.
The sweep then boots each named leg and requires red on each.

## Why

Row 21 of `notes/confinement-claims.md` (a user program cannot read a kernel address, on every ISA)
is evidenced on aarch64 twice and riscv64 once, and on `x86_64` not at all:
`a_user_program_cannot_read_a_kernel_address` runs there and its record declares
`Architecture: aarch64`. The mechanism admits one architecture per record because the record's
filename is the test's name. So on `x86_64` the row is a green test that has never been shown able
to go red, which is precisely the state milestone 305 found the RISC-V twin in and spent a milestone
on. The aarch64 defect (map the kernel's `.rodata` `U/S`-accessible) would work on `x86_64` as it
stands, since SMAP is off; nothing can record it.

DECISIONS §19 makes parity a gate for the capability. This is parity for the evidence, which the
same note already says is the weaker of the two and the one that drifts.

## What it costs

A record per leg costs a boot per leg on `--sweep`, which is already the sweep's posture (a report,
not a per-commit gate). The `--check` half changes shape slightly: a test that names two records must
have both, and a record whose test does not run on its architecture is rot.
