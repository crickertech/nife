# One grant order for the progenitor, on every board

**Status: PROPOSED 2026-09-08.** Raised by milestone 266, which unified the first process into one
program and could not unify the one thing left under a `cfg` inside it.

**Gate: NONE.** Nothing outside this repository has acted on the slot numbers.

## In brief

`components/src/progenitor.rs` holds two `BootEndowment` tables. They differ because the two kernels grant
the boot capabilities in a different order:

| | aarch64 (`kernel::user::spawn_progenitor`) | riscv64 / x86_64 (`kernel::user::riscv_shell_boot`) |
|---|---|---|
| untyped | 0 | 0 |
| the kernel's report endpoint | 1 | absent |
| UART registers | 2 | 1 |
| the 19d.2b test interrupt | 3 | absent |
| UART receive interrupt | 4 | 2 |
| clock page | 5 | 3 |
| configuration page | 6 | 4 |
| file service, shared page | 7, 8 | 5, 6 |
| virtio-rng trio | 9, 10, 11 | 7, 8, 9 |
| graphical terminal trio | 12, 13, 14 | 10, 11, 12 |

Every row after the first is displaced by exactly the two capabilities aarch64 grants and the
interactive system never uses: a report endpoint nothing receives on, and a test interrupt no
component waits for. They are there because **aarch64's boot path is shared with milestone 19d's
test roles**, which enter `hello` through the same function with the same endowment, and whose slot
numbering is written into `fixtures/src/hello.rs`'s role catalogue and into six `spawn_progenitor` tests.

## What it would take

Grant the two test capabilities *after* the interactive set on aarch64, or grant them only for the
test roles, so both boards number the interactive capabilities identically from slot 0. Then one
`BootEndowment` serves every architecture and the `cfg` goes.

The work is not the kernel side. It is that 19d's roles read fixed slot numbers (`REPORT = 1`,
`TEST_IRQ = 3`, `UART_DEV = 2`) and each is named in `hello.rs` and asserted from the kernel's test
module. Renumbering them is mechanical and the blast radius is a whole milestone's worth of reading,
which is exactly why 266 did not fold it in: a boot failure and a renumbered test suite arriving in
one change is the ambiguity that milestone forbade itself.

## Why it is worth doing rather than living with

The `cfg` is data rather than code and it sits in one file beside the table it disagrees with, which
is the cheapest place to notice it. But it is still one architectural difference inside the first
process, in a milestone whose argument was that there should not be one, and DECISIONS §19's failure
mode is precisely a fact that is true on one ISA and quietly not on another.

## What it is not

It is not a request to make the two kernels one function. `spawn_progenitor` and `riscv_shell_boot`
differ in what they *have* to do (a GIC and a PL011 device capability against a PLIC and an NS16550),
and that difference is rule 1 working. Only the order is arbitrary.
