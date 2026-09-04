# `script/fastpath-footprint`'s entry set is flat, so an inlining flip can move 12% into it

**Status: PROPOSED 2026-09-04.** Found by the milestone 133 lane, whose change to region teardown
failed this gate by 12.1% without touching a timer, a trap path, or a syscall.

**Gate: NONE.** Nothing is owed. It wants a lane because the fix is a judgement about what the
number is *for*, and getting that wrong in either direction costs something real: a looser gate
stops catching regressions, and a tighter one fails changes that are not regressions.

**In brief.** The `ipc_fastpath` half of this gate is a **closure** with a `COLD` exclusion, so
panic, formatting and teardown bytes are kept out of it deliberately and the script explains why at
length. The `syscall_entry` half is **flat**: it sums a fixed list of symbols, and it has no way to
exclude anything, because there is nothing inside a symbol to exclude. That is fine while the
symbols contain only what a syscall fetches. It stops being fine the moment LLVM folds something
else into one of them.

## What happened, since the abstract version of this is easy to wave away

`riscv_trap_body` is in the riscv64 entry set for a correct reason the script argues carefully: on
this ISA an `ecall` arrives through the same handler as a page fault and a timer interrupt, so those
bytes really are on a syscall's path. But that handler has arms, and **the timer arm is not one a
syscall fetches.**

Milestone 133 changed `sched::reap_region_objects` and `crates/ipc`. It touches no timer, no trap
path, and no syscall. LLVM nonetheless decided to inline `arch::riscv64::timer::tick` into
`riscv_trap_body` in the new build and not the old one, which put **226 bytes** (1870 to 2096, 12.1%
against a 5% bound) onto a number that measures the syscall path. Confirmed by diffing the two
disassemblies symbol by symbol: `syscall::dispatch`, `trap_entry`, `trap_return` and
`riscv_trap_dispatch` were byte-identical, `riscv_trap_body` grew by exactly that, and the call to
`timer::tick` disappeared from it.

The lane closed it with `#[inline(never)]` on `timer::tick`, which restores 1870 exactly and is
defensible on its own (a `jal` once per 10 ms tick costs nothing). **That is a patch on the symptom
and the next one will look different**, because nothing stops LLVM folding a different arm of a
different handler in next month, and the person who meets it will have to re-derive all of the
above from a percentage.

## It happened again the next day, so "the next one will look different" is now observed

**2026-09-04, PR #716, the daily toolchain bump.** That branch changes **one line**,
`rust-toolchain.toml` from `nightly-2026-09-03` to `nightly-2026-09-04`, and no kernel source at
all. It failed this gate at **10.4%** (1870 to 2064) on the same symbol, `riscv_trap_body`, which
grew 550 to 744 while `trap_entry`, `trap_return`, `riscv_trap_dispatch` and `syscall::dispatch`
were byte-identical. The inlined callee this time was **`drivers::plic::disable`**, in the
`S_EXTERNAL` device-interrupt arm, which vanished from the disassembly entirely; inlining it also
dragged in `set_enable_bit`'s locked read-modify-write. Closed the same way, with `#[inline(never)]`
on `plic::disable` and the reasoning beside it.

Three things this instance settles that the milestone 133 one could only assert:

- **The prediction was right about the shape.** A different arm, a different callee, the same
  symbol, one day later. Neither arm is one a syscall fetches.
- **The trigger need not be a code change.** Milestone 133's was at least a change to `sched.rs`,
  which made "unrelated change" arguable. This one is a compiler upgrade, so there is no source
  edit anywhere to attribute the 194 bytes to. Any lane can be handed this failure by a toolchain
  bump it did not make.
- **The cost of the current answer is now measurable as a rate.** Two instances in two days, each
  costing a lane a build-and-diff of two disassemblies to name one symbol. The last bullet below,
  *report the delta per symbol when the gate fails*, would have turned both of those into reading
  the failure message. It is the cheapest item on the list and it is the one that pays every time.

## What the options look like

- **Do nothing, and treat the attribute as the pattern.** Cheapest, and it is not absurd: the tree
  already carries `#[inline(never)]` for exactly this reason on `sched::grant_cycle_counter` and
  milestone 156's spawn-path bodies. The cost is that the pattern is invisible until you trip it,
  and each instance is discovered by a red gate on an unrelated change.
- **Measure the arms rather than the symbols.** Follow the branch structure inside a root and count
  only the blocks a syscall can reach. This is what the closure half already does one level up, and
  it is the honest answer. It is also the expensive one, and it needs the disassembly parser to
  understand conditional branches, which the script deliberately does not today (its `CALL` regex
  excludes them, and says why).
- **Give the flat set the same exclusion the closure has**, by attributing bytes inside a symbol to
  an inlined callee where the debug info can say so. Middle cost; depends on what the release build
  keeps.
- **Report the delta per symbol when the gate fails**, whatever else is decided. The diff above took
  a disassembly of two builds and a short script; the gate could have printed it. This is the piece
  worth doing first regardless, because it turns a percentage into a cause.

## Where it came from

Milestone 133's Follow-on. `design/roadmap/132-the-fastpath-footprint.md` owns the gate.
