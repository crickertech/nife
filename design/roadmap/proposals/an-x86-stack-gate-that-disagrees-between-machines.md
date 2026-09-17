# An `x86_64` stack gate that disagrees between machines

**Status: PROPOSED 2026-09-17.** Raised by milestone 313's security audit while gating (finding 7),
outside its lens.

**Gate: NONE.** It is a measurement to reconcile and then either a limit to move with a reason or a
chain to shorten.

## What was measured

On patagonia, `cargo xtask test --arch x86_64 --test a_refusal_and_a_success` (one test,
`language_tests::a_refusal_and_a_success_report_different_numbers`) ends with

```
stack high-water: boot   62456/65504 bytes (95%), paint floor 4864
[PANIC] panicked at kernel/src/stack.rs:621:5:
boot stack high-water 62456 exceeded 61440
```

four times out of four, byte-identical, including on the base commit `52da4ae4`'s own `kernel/` and
`crates/` with nothing of the audit's applied. CI's `build + test` job at that same commit is green,
and `cargo xtask test` boots all three legs by default. The job's log as `gh run view --job --log`
returns it carries only the setup and cleanup steps, so the lane could not read CI's own
`stack high-water: boot` line for the `x86_64` leg to say whether CI measures the chain lower, runs
under a configuration that skips the test, or something else.

## Why it matters

A gate that is red on one machine and green on another for the same commit is worse than a red gate:
it teaches whichever side sees green that the other is noise. The limit was set at +7 KiB over a
measured 54216 (notes/stack-high-water.md) for aarch64 and riscv64, and no `x86_64` number appears
in that note's tables at all, so it is possible the `x86_64` leg has simply never been measured
against the limit on a machine that reaches this chain.

## What would close it

Read CI's transcript for the `x86_64` leg (the harness captures QEMU output and prints it only on
failure, so a green run may need `--no-capture` or a printed summary to show the number), reconcile
the two measurements, and then either record an `x86_64` column in the stack note with a limit that
has a reason, or find out what that language test does 8 KiB deeper on `x86_64` than the deepest
aarch64 chain does.
