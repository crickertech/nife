# `crates/elf`'s host tests assume an aarch64 host and fail on x86_64

**Status: PROPOSED 2026-09-12.** Found by milestone 277, which measured every host test binary's
peak address space and could not finish the sweep because this crate's suite goes red on an x86_64
machine.

**Gate: NONE.** It is a test-fixture fix in one crate, on the host side, with no syscall surface and
no wire format.

## In brief

`cargo test -p elf` fails **20 of 25 tests** on an `x86_64-unknown-linux-gnu` host, every one with
`WrongMachine`. The crate picks the ELF machine it accepts at compile time:

```
#[cfg(target_arch = "riscv64")]  const EXPECTED_MACHINE: u16 = EM_RISCV;
#[cfg(target_arch = "x86_64")]   const EXPECTED_MACHINE: u16 = EM_X86_64;
#[cfg(...)]                      const EXPECTED_MACHINE: u16 = EM_AARCH64;
```

which is right, and is documented as right: *"A kernel only ever loads binaries for its own
architecture, so the expected machine is a compile-time fact."* But the test `Builder` hardcodes the
machine it forges into the header:

```
e_machine: EM_AARCH64,
```

so on an aarch64 host the forgery matches `EXPECTED_MACHINE` and every test passes, and on an
x86_64 host it does not and `Elf::parse` correctly refuses it. The crate already exports
`EXPECTED_MACHINE` for this exact purpose, with a doc comment saying so: *"A test that forges an ELF
header has to write some machine number, and writing the native one is what lets the forgery get
past the machine check."* The `Builder` predates or ignores that, and the fix is probably one line.

One test needs more than the one line. `a_binary_for_the_other_supported_machine_is_refused`
iterates `[EM_RISCV, EM_X86_64]` under a comment reading *"with `EXPECTED_MACHINE == EM_AARCH64`"*,
so on an x86_64 host it asserts that an x86_64 binary is refused. It wants the two machines that are
not the native one, derived rather than listed.

## Why nobody has seen it

**CI runs on `ubuntu-24.04-arm` and the development machine is Apple Silicon.** Both are aarch64, so
the hardcoded `EM_AARCH64` is the native machine on every box that has ever run this suite.

This is milestone 117's stranger-test class precisely, and the tree has paid for it once already:
`xtask`'s host pass carries a comment about `swap_proto`, `virtio` and `supervision_proto` taking
`user_rt` dependencies, where *"cargo still had to build it for them, so the host pass stopped
compiling on an x86_64 host and nobody noticed, because CI moved to `ubuntu-24.04-arm` the same day
and on an aarch64 host it builds by accident. A stranger with a clean x86_64 checkout found it."*
Same shape, same cause, different crate: an aarch64 assumption that is invisible to every machine
this project owns.

**It matters more than an ordinary red suite**, because `x86_64-unknown-none` is a declared parity
target (DECISIONS §19) and `crates/elf` is the loader's validator. The host tests are the only place
its `x86_64` arm is exercised at all, and on the one host architecture where that arm is live they
do not run.

## What to do

1. Have the test `Builder` default `e_machine` to the crate's own `EXPECTED_MACHINE` rather than to
   `EM_AARCH64`, which is what the exported constant exists for.
2. Derive the "other supported machine" list from `EXPECTED_MACHINE` instead of listing two of the
   three by hand, so the test means the same sentence on all three hosts.
3. Consider whether anything should stop this returning. `script/stranger-test` is the mechanism
   already aimed at this class; whether it can reach an x86_64 host cheaply is the open question,
   and a `BUGS` note in the crate is the honest floor if it cannot.
