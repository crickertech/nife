# A stack gate that fires only on a filtered run

**Status: PROPOSED 2026-09-17.** Raised by milestone 313's security audit while gating (finding 7),
outside its lens.

**Gate: NONE.** It is a measurement to explain and then either a limit to move with a reason or a
chain to shorten.

## What was measured

On patagonia, on the base commit `52da4ae4`'s own `kernel/` and `crates/` with nothing of the
audit's applied, `cargo xtask test --arch x86_64 --test a_refusal_and_a_success` (one test,
`language_tests::a_refusal_and_a_success_report_different_numbers`) ends with

```
stack high-water: boot   62456/65504 bytes (95%), paint floor 4864
[PANIC] panicked at kernel/src/stack.rs:621:5:
boot stack high-water 62456 exceeded 61440
```

four times out of four, byte-identical. The **full** `x86_64` suite on the same tree, both boot
modes, ends green with the boot stack at **53144** (PVH, 243 passed) and **50272** (UEFI, 214
passed). So the same test drives the boot stack about 9 KiB deeper when it is the only test selected
than when it runs inside the suite, and the gate that would catch real growth trips on the filtered
run and not on the one CI runs.

## Why it matters

Two reasons, and the second is the one that touches something this tree relies on.

A filtered run is how a person reproduces one failure, and it is how milestone 210's `--test
<substring>` was built to be used. A gate that fires only there teaches whoever hits it that the gate
is noise, which is the wrong lesson about a stack gate.

And `script/falsifications --sweep` replays every kernel record as a filtered single-test run. A
record whose test happens to sit on a deep chain would report **error** ("the kernel never reached
its test runner" is not the shape; it would be a red at `stack.rs:621` rather than at the assertion
the patch names), and the sweep's whole discipline is that a red for the wrong reason is not
evidence. No record today names that test, so nothing is wrong yet.

## What would close it

Find what a filtered run does 9 KiB deeper than the suite does on the same test. The likely shape is
something that runs once per boot before the first test and is on the boot stack when the first
selected test begins, so that in a full run it has been and gone by the time this test is reached and
in a filtered run it is still there. Then either an `x86_64` row in `notes/stack-high-water.md` with
a limit that has a reason, or a shorter chain.
