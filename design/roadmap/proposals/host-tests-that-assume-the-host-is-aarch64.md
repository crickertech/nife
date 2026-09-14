# Host tests that assume the host is aarch64

**Status: PROPOSED 2026-09-13.** Found by milestone 287 while gating on an **x86_64 Linux** box, the
first machine in this project's history to run `script/test` from a host that is not aarch64. It is
pre-existing on `main` and no part of milestone 287 touches Rust.

**Gate: NONE.** A lane can close this. The diagnosis below is complete; what it needs is a judgement
about what one test should assert on a host that is not aarch64, which is a testing question rather
than calef's.

## What happens

```
$ cargo test -p elf --lib
test result: FAILED. 5 passed; 20 failed
---- tests::a_good_binary_parses stdout ----
panicked at crates/elf/src/lib.rs:708:38: should parse: WrongMachine
```

Twenty of twenty-five tests in `crates/elf` fail, and `script/test` therefore cannot reach the kernel
legs at all: the host pass runs first. `script/setup` now completes on a stock Linux box (milestone
287) and `script/test` still does not go green there, which is the same principle one step further
in: *a newcomer must be able to succeed without asking anyone.*

## Why, exactly

`crates/elf` picks the machine it accepts at compile time, correctly:

```rust
#[cfg(target_arch = "riscv64")]  const EXPECTED_MACHINE: u16 = EM_RISCV;
#[cfg(target_arch = "x86_64")]   const EXPECTED_MACHINE: u16 = EM_X86_64;
                                 const EXPECTED_MACHINE: u16 = EM_AARCH64;   // otherwise
```

and exports `NATIVE_MACHINE = EXPECTED_MACHINE` **for this exact purpose**, in a doc comment that
states the whole problem before it happens:

> *"A test that forges an ELF header has to write some machine number, and writing the native one is
> what lets the forgery get past the machine check and reach the property actually under test."*

**The crate's own test `Builder` does not use it.** `Builder::new()` hardcodes
`e_machine: EM_AARCH64`, so on an x86_64 host every forgery is foreign and every happy-path test dies
at the machine check before reaching the property it exists to test. The module doc-test one screen
above uses `NATIVE_MACHINE` and passes; the unit tests underneath do not and fail.

A second test states the assumption out loud and is wrong in a different direction:

```rust
/// **A binary for one of the *other* nife architectures is refused too.** These host tests build
/// with `EXPECTED_MACHINE == EM_AARCH64`, so a riscv ELF (243) and an `x86_64` one (62) are both
/// foreign here
fn a_binary_for_the_other_supported_machine_is_refused() {
    for machine in [EM_RISCV, EM_X86_64] { ... }
}
```

On an x86_64 host `EM_X86_64` is the native machine, so asserting it is refused asserts the opposite
of the truth. **That comment is the bug's own confession**: it says "these host tests build with
`EXPECTED_MACHINE == EM_AARCH64`", which is a statement about the architect's laptop rather than
about the crate.

## Why nobody met it

Every machine that has ever run this suite is aarch64: macOS on Apple Silicon, and CI on arm64
runners. `EXPECTED_MACHINE` then happens to equal the hardcoded `EM_AARCH64` and the whole class is
invisible. Milestone 161 already trod on the edge of it: the `a_binary_for_another_machine_is_refused`
test had to move from `x86_64`'s number to SPARC's "the day `x86_64` became a target", and its comment
records that. The same reasoning applied one test further down would have found this.

## What would close it

1. `Builder::new()` uses `NATIVE_MACHINE` rather than `EM_AARCH64`. That is the one-word half and it
   fixes nineteen of the twenty.
2. `a_binary_for_the_other_supported_machine_is_refused` iterates *the two that are not native* rather
   than a hardcoded pair, and its doc comment stops describing the architect's machine. This is the
   half that wants a moment's thought rather than a `sed`: the test's point is that the check is
   symmetric and not aarch64-privileged, and it should be written so that it proves that on whichever
   host runs it.
3. Sweep the other host-test crates for the same shape. `crates/elf` is where it bites first because
   it is the one crate whose logic is *about* machine identity, but a `#[cfg(target_arch)]` constant
   paired with a hardcoded literal in a test is a pattern, not an incident.
4. Decide whether an x86_64 host belongs in CI. Today the answer is implicitly no and nothing says so;
   if it stays no, that is a scope note somebody should be able to find, because the failure a
   stranger meets is twenty red tests with no explanation.

## What it is worth

**An x86_64 Linux box is the commonest developer machine there is**, and this is what one sees after
following `script/setup` to completion. Milestone 287 removed the first wall a Linux newcomer hits;
this is the second, and it is louder, because twenty failing tests in a crate named `elf` read as
"this project is broken" rather than as "your host is unusual".
