# 219. The boot tour ends and the kernel halts, so there is nothing to soak

**Status: NOT-STARTED.** Minted 2026-09-01 by the maintainer, after radon booted under script
control and the gap became obvious: everything needed for a sustained run exists except a workload
that lasts. *(Number provisional until the merge queue lands it.)*

**Gate: NONE.** It needs no board to build and no board to test. QEMU can run it; a board is only
where the answer becomes interesting.

**In brief.** `design/fatal-risks.md` risk 5 (it cannot be made reliable on multicore, and the bugs
appear only on silicon) names its decisive experiment as *sustained multi-core stress on the boards
with the load-sensitive assertions live*. It is the risk that has already fired once, on radon, with
a receiver woken and nothing delivered on three harts, found in a bench session rather than by any
test.

**Nothing in this tree can sustain anything.** The kernel's boot tour runs its checks, prints
`nife: the capability core runs on RISC-V.`, and calls `halt()`. Captured on radon on 2026-09-01,
that is the last line, after which the board sits in `wfi` indefinitely. A soak needs a workload
that keeps running, and there is not one.

That is the whole milestone: **something that runs on every core, for hours, that fails loudly.**

## Why this is the binding constraint on risk 5 and the other pieces are not

The pieces that looked like blockers turn out not to be:

- **Booting radon under automation** was proved on 2026-09-01: a script interrupts U-Boot, types
  the five commands, and reads the console to completion.
- **Remote power** is unresolved (radon's Kasa plug is unreachable from the development machine)
  and **is not needed for a first soak**. A run that never crashes needs no power cycle; one that
  hangs needs a person, which is acceptable for the first attempt and is what milestone 218
  (every boot needs a human typing four commands into U-Boot) is about.
- **A console that logs and knows when to stop** is milestone 216.

So the missing piece is the workload, and it is the only one of the four that no other milestone
covers.

## What it needs

**A payload that runs indefinitely across every online core and whose failure is unmissable.**
Design questions this block does not answer, named so they are decided rather than discovered:

- **What it should stress.** The defect risk 5 already produced was in the wake path
  (`wake_load_aware`, a receiver made Ready without a delivery), so IPC and scheduling handoffs
  between harts are the obvious target. `crates/thread_wake_handshake` and `crates/work_steal_slot`
  exist and are where such a property would live.
- **How it fails loudly.** `notes/load-sensitive-assertions.md` is the existing thinking, and
  `script/repeat-under-load` and `script/interleaving-check` are the existing instruments. This
  should extend them rather than start beside them.
- **Whether it is a user program, a kernel mode, or a feature flag.** A user program is the most
  honest, since it exercises the real syscall path, but it cannot assert about kernel internals.
- **What "it passed" means.** A soak that ends with nothing printed proves less than it appears to.
  The run should produce a number that a later run can be compared against, which is what makes it
  evidence rather than a vigil.

## What would make it a real answer to risk 5

Running it on **radon**, **argon** and **xenon**, not just in QEMU, since the entire premise of the
risk is that emulation cannot show these defects. That is what the machines are for, and it is why
this block is worth minting even though the workload itself is ordinary software.

## BUGS

- **This block sets no duration**, and the honest reason is that nobody knows what duration would
  be persuasive. The risk's own text says this class "produces a confidence rather than a verdict".
- **A soak that finds nothing is weak evidence and must be reported as such.** The failure mode to
  guard against is a green run being quoted as though it proved the concurrency correct.
- **It says nothing about how a hang is distinguished from a slow run**, which is milestone 216's
  problem for the console and this milestone's problem for the workload; they must agree, and
  nothing here makes them.
