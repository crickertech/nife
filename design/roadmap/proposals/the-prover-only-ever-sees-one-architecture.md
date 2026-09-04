# `cargo kani -p kernel` only ever compiles one architecture, and it is the runner's

**Status: PROPOSED 2026-09-04.** Written by milestone 255's lane, from what it found while looking
for the largest asm-free files in `kernel/src/arch/`.

**Gate: NONE.** The mechanism is a `script/verify` row and a runner label, both of which already
exist in other shapes. It is reversible: a row can be removed.

**In brief.** `kernel/src/arch/mod.rs` selects its subtree with `#[cfg(target_arch = ...)]`, and
under Kani the target is the **host**. Every job in `.github/workflows/verify.yml` runs on
`ubuntu-24.04-arm`, and the dev machine is Apple Silicon, so `cargo kani -p kernel` compiles
`arch/aarch64/` and **nothing at all** of `arch/riscv64/` or `arch/x86_64/`. Two thirds of the
architecture layer is out of the prover's reach for a reason that has nothing to do with `asm!`, and
nothing in the tree said so until now.

## Why this matters, and it is a measurement rather than a worry

Milestone 255's block counted the asm-free lines of `kernel/src/arch/` and listed the largest files
worth proving. **The two largest are `x86_64/irq.rs` (903 lines) and `x86_64/machine.rs` (762), and
neither can be reached at all from the runners this tree has.** `machine.rs` is the ACPI walk, which
takes length fields, checksums and counts from firmware; notes/x86-uefi-boot.md records that under
OVMF the kernel read a 118-descriptor memory map and a revision-2 RSDP where PVH gave 9 and revision
0, so both branches are live and one had never executed until milestone 195. That is untrusted-input
parsing, which is the strongest case in the tree for a bounded model checker, and it is unreachable
for a `cfg` rather than for a construct.

The same applies to `riscv64/iommu.rs` (314 lines). Milestone 255 proved the SMMUv3's entry-building
arithmetic in place and was careful to say that the property has no counterpart in the RISC-V driver,
which writes its device context in 64-bit stores with no split. Someone should write the RISC-V
driver's own property; nobody can run it.

**And this compounds the hazard already recorded.** notes/kernel-proofs.md's BUGS says the `kernel`
row *needs* an aarch64 host, because `kernel/Cargo.toml` depends on `aarch64-cpu` unconditionally and
`crate::arch::mmu::Format` resolves by the host's `target_arch`. So the row is simultaneously pinned
to one architecture and silently proving only that architecture's `arch/`. Those are the same fact
read from two sides, and only one side is written down.

## Shapes, and what each costs

- **A second `kernel` row on an x86_64 runner.** `verify.yml` already shards by crate, and GitHub's
  `ubuntu-24.04` is x86_64, so this is a runner label and a row. It reaches `arch/x86_64/` and
  nothing else new. The cost is that `aarch64-cpu` must go behind a target `cfg` first, which
  notes/kernel-proofs.md already names as the fix if the runner label ever changes; this proposal
  makes that a prerequisite rather than a contingency.
- **A riscv64 host, which does not exist as a runner.** GitHub offers no riscv64 image. `xenon` and
  `radon` are lab boards rather than CI, and Kani on a board is its own project. The honest answer
  is probably that `arch/riscv64/` stays unreachable and the block says so.
- **Refuse the whole thing and lift the arch-independent logic into crates.** This is milestone
  193's option B, which 193 chose against and 244 declined again on measurement. It would work, and
  it is the option this tree keeps refusing for a stated reason: if a property can be proved where
  the code lives, prove it there.

Choosing is the work. What is not optional is that the limit gets written down somewhere a reader
meets it, because "the kernel is Kani-reachable" is currently read as covering three architectures
and covers one.

## Where it came from

Milestone 255's `## Follow-on`. The lane went looking for `x86_64/machine.rs` on the strength of its
own block naming it as the second-largest asm-free file in `arch/`, and found it does not compile
under the prover at all on any machine this project has.
