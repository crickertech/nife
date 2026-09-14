# One machine description, not a description and a narrative saying the same thing

**Status: PROPOSED 2026-09-14.** Found by milestone 268's lane, which caused it: the machine
description now prints on all three architectures, and on two of them the arm above it had already
said most of the same things in its own words.

**Gate: NONE.** It reads the tree and moves lines within one function.

## In brief

`kernel/src/main.rs`'s riscv64 and `x86_64` arms narrate their own bring-up, one line per step as
each piece comes up: `isa`, `firmware`, `memory`, `paging`, `pci`, `apic`, `clocks`, `io apic`. Then
`print_machine_description` answers the eight questions in one block, and several of the answers are
the same facts in a different layout. A riscv64 boot prints its `isa` and `firmware` lines twice.

The duplication is noise rather than a defect, and it is the honest cost of getting parity first:
the description had to land on all three before anything could be trimmed from either.

## What makes this less trivial than it looks

**The arms' lines serve an audience the description does not reach.** A `test` boot exits through
semihosting and a `bench` boot diverges into `bench::run`, and `print_machine_description` is
`#[cfg(not(any(test, feature = "bench")))]` precisely because neither is a boot anybody reads to
bring up a board. The riscv64 arm's `isa::print_summary` call carries a comment saying it is placed
where it is *"so that the test, shell and bench boots report it too"*. Delete it and those two
configurations stop saying what machine they ran on, which is a real loss on a bench.

**And the timing differs, which is sometimes the point.** The arms print each fact *at the moment
that piece comes up*, so a boot that dies halfway leaves a transcript that says how far it got. The
description prints everything at once, after everything is up, so a boot that dies before it prints
nothing at all. On a board those are different instruments, and milestone 267's lint exists because
somebody once moved a line between them.

## Roughly what to do

Decide, per duplicated line, which instrument it belongs to, and say so where it sits:

- facts a dying boot needs *early* stay in the arm;
- facts a healthy boot needs *collected* move to the description;
- facts a `test` or `bench` boot needs get a third home, or the description's `cfg` gets revisited
  (milestone 267's lint guards its **body**, not its signature, so that is a decision rather than a
  prohibition).

Measured, not guessed: a boot transcript per architecture before and after, in the block.

## Not a levelling-down

Nothing here removes a capability. The question is only whether one fact is printed once or twice,
and the default answer where it is unclear should be twice: a duplicated line costs a reader a
second, and a missing one costs a bench session.
