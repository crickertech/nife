# Proving things about `kernel/src`

Milestone 193. The companion to [verification.md](verification.md), which is about proving the pure
crates; this one is about the 64,818 lines the prover could not see until 2026-08-30, and about the
stubs you take on when you point it at them.

*Name provisional: notes are an interface and their names are calef's call (AGENTS.md). `kernel-proofs`
says what the file is about and matches `verification.md`'s neighbourhood; expect it to change.*

## Why this note exists at all

Milestone 191 asked whether any Kani harness in this tree had ever caught a defect after the day it
was written. None had. The cause was not the harnesses and not the technique: it was one line in
`script/verify`'s own header, which has always been honest about it.

> `cargo kani -p <crate>` never compiles the kernel, the user programs, or xtask.

So every proof in the tree was aimed at pure logic, and every concurrency, hardware-contract and
resource-accounting defect the corpus recorded lived somewhere else. DECISIONS §14 promises a
verified core and the core was the part not being verified.

The distance turned out to be four changes.

## What it took, exactly

`cargo check -p kernel --target aarch64-apple-darwin` failed with three errors before this
milestone, and `cargo kani -p kernel` with two more once those were gone. All five, and the fix for
each:

| what stopped it | why | the fix |
|---|---|---|
| `unwinding panics are not supported without std` | the workspace deliberately does not set `panic = "abort"` (DECISIONS §7) | nothing: Kani passes `-C panic=abort` itself, so this is a `cargo check` problem and not a `cargo kani` one |
| `invalid Mach-O section specifier`, twice | `#[unsafe(link_section = ".interrupt_stacks")]` and `".secondary_stacks"` are ELF section names, and the dev machine's host target is Mach-O | `#[cfg_attr(target_os = "none", ...)]`, because the linker script that gives those sections meaning exists only for the bare-metal build |
| `found duplicate lang item panic_impl` | Kani links `std` into the crate it proves, and `std` already defines the panic handler | `#[cfg(not(kani))]` on `kernel/src/panic.rs`'s handler |
| `Crate kernel contains global ASM, which is not supported by Kani` | eleven `global_asm!` blocks: the boot entry, the vector table and the context switch, on all three architectures | `-Z unstable-options --ignore-global-asm`, in `script/verify`'s `kernel` case |

That is the whole list. **DECISIONS §4 rule 1 is why it is this short**: architecture-specific code
lives under `kernel/src/arch/`, so `asm!` appears outside it at only three sites in two files
(`cpu.rs` once, `user.rs` twice) and the other ~50,000 lines are ordinary Rust that a host compiler
was always going to accept. A tree that had let `asm!` spread would not have had a front door here at
all.

## The stub boundary, enumerated

**A proof with an unexamined stub is worse than no proof, because it reads as coverage.** So this is
the exhaustive list of what a harness in `kernel/src` cannot see, and it is short enough to keep
exhaustive. The same list is in `syscall.rs`'s `mod proofs`, where somebody writing the next harness
will actually meet it.

1. **Global assembly is skipped.** Boot entry, vectors, context switch, three architectures. Nothing
   in a harness reaches it, and nothing could.
2. **`asm!` is an unsupported construct.** All of `kernel/src/arch/` plus the three sites above.
   This is a *hard* boundary rather than a soft one, which is the good direction: if a harness ever
   calls into that code, Kani reports the unsupported construct instead of proving past it. It also
   means **the architecture layer stays unverified**, which is where the VisionFive 2's
   undelivered-wake defect actually lived. This milestone does not touch that.
3. **The panic handler is absent.** Nothing proved here says anything about what the kernel does
   after a panic.
4. **The link sections are absent.** Under `cfg(kani)` the host target is not `target_os = "none"`,
   so `STACKS` and `SECONDARY_STACKS` are ordinary statics rather than statics in their own regions.
   A harness reasoning about guard-page layout would therefore be reasoning about a fiction. Do not
   write one; the region-size claim is held by
   `the_secondary_stack_region_is_the_size_the_linker_reserved` in a real build instead.
5. **MMIO and fixed physical addresses** are raw pointers to nothing under a model checker. The same
   argument applies: stub the boundary, do not pretend.
6. **`script/lint`'s harness-clippy pass excludes `kernel`**, so clippy lints do not fire inside
   these harnesses. That pass's own comment carries the two tooling reasons and what still covers
   them. Practical consequence: **keep kernel harnesses free of `unsafe`.**

## What is proved today

Two properties, both in `kernel/src/syscall.rs`, both about `run_end_va` and the map loops that
guard on it. This is the code milestone 193's block predicted would never become a crate, and it is
right: syscall dispatch is a boundary, not a library.

`run_end_va` computes the last virtual address a `count`-page run starting at `va` covers.

- **`the_run_end_is_exact_and_refuses_exactly_what_does_not_fit`.** For every `va` and every
  `count >= 1`: a `Some(last)` is exactly `va + (count - 1) * PAGE_SIZE` computed without wrapping,
  and a `None` is exactly a run that does not fit in a `u64`. The claim is stated in `u128`, so the
  harness does not repeat the implementation's expression back to itself.
- **`every_page_between_the_checked_ends_is_itself_a_user_page`.** Both mapping paths check `va` and
  the run's last address and then walk `va + k * PAGE_SIZE` for `k` in `0..count` with nothing
  re-checking the pages between. Given the two endpoints pass `is_user_page_va`, every page in the
  run does too, and no page is computed past the checked end. `k` is symbolic rather than iterated,
  so this covers every page of every run with no unwind bound.

### Why these two and not the timer

Milestone 193's block nominates the milestone 6 timer re-arm drift as the first property worth
trying, on the grounds that its property is already proved in `crates/timetable`'s `next_after` and
the timer does not call it. That is still a good observation and a good future lane, but **the
re-arm is in `kernel/src/arch/aarch64/timer.rs` and `kernel/src/arch/riscv64/timer.rs`**, which item
2 above puts out of reach: `rearm` reads `CNTVCT_EL0` and writes `CNTV_CVAL_EL0`, both `asm!` through
`aarch64-cpu`. Proving it means first lifting the arithmetic out of the register accesses, and where
that seam goes is a design question rather than a mechanical one.

`run_end_va` was chosen instead on the criterion that matters most given milestone 191's finding:
**it has a real defect on record.** Milestone 142's review, MAJOR 4, found that the old
implementation checked only the addition in `va.checked_add((count - 1) * PAGE_SIZE)`, leaving the
multiply free to wrap, so a large `count` produced a `last_va` a few pages above `va` and the guard
admitted a run spanning the address space.

## EXAMPLES

Prove just the kernel's harnesses, with a counterexample trace on failure:

```console
$ cargo kani -p kernel -Z unstable-options --ignore-global-asm
...
Complete - 2 successfully verified harnesses, 0 failures, 2 total.
```

One harness on its own:

```console
$ cargo kani -p kernel -Z unstable-options --ignore-global-asm \
      --harness the_run_end_is_exact_and_refuses_exactly_what_does_not_fit
```

The whole suite, kernel included, the way CI runs it:

```console
$ script/verify
```

**Falsify a new harness before you believe it.** This is the tree's standing discipline for proofs
(verification.md says the same) and milestone 191 is the reason it is not optional. Re-introduce the
historical defect and check that both harnesses go red:

```console
$ # in run_end_va, replace the checked arithmetic with milestone 142's bug:
$ #   va.checked_add((count - 1).wrapping_mul(paging::PAGE_SIZE))
$ cargo kani -p kernel -Z unstable-options --ignore-global-asm --output-format=terse
Verification failed for - syscall::proofs::every_page_between_the_checked_ends_is_itself_a_user_page
Verification failed for - syscall::proofs::the_run_end_is_exact_and_refuses_exactly_what_does_not_fit
Complete - 0 successfully verified harnesses, 2 failures, 2 total.
```

Both catch it. That was run on 2026-08-30 and is the reason these two harnesses are worth their
place rather than an assertion that they are.

## Adding a harness to `kernel/src`

1. Put it beside the code it proves, in a `#[cfg(kani)] mod proofs`, not in a separate file. The
   stub list above is the reason: a reader has to meet the caveats where they meet the harness.
2. Check the call graph against the stub list. If it reaches `crate::arch`, stop; you are proving
   nothing, and Kani will say so rather than lie.
3. Falsify it. Break the code under it and watch the harness fail, then put the code back.
4. `kernel` is already in `script/verify`'s crate table, so nothing needs adding there. If you add a
   harness to a crate that is *not* in that table, `script/lint`'s "every crate with proof harnesses
   is in the verify table" check will fail; that gate exists because two crates
   (`mdns_proto`, `jh7110_trng`) each spent months carrying harnesses nothing ran.

## BUGS

- **`kernel/src/arch/` is unreachable and will stay unreachable**, so nothing here should be read as
  covering the architecture layer. Two of the corpus's own defects (the timer re-arm drift, the
  VisionFive 2 undelivered wake) are inside it.
- **Two harnesses is not coverage of a 64,818-line crate**, and the number to watch is not the count
  but whether the properties are ones a defect would violate. The two here were chosen because a
  defect *did* violate them.
- **`--ignore-global-asm` is a global switch, not a per-item one.** A future `global_asm!` block that
  a harness genuinely needs would be skipped silently rather than refused. Nothing detects that; the
  reason it is acceptable today is that Kani's `asm!` restriction makes any such harness fail for a
  second, louder reason.
- **`user/` and `xtask` are still out of reach**, for exactly the reason the kernel was. `user/`
  holds real parsers over untrusted input and has at least as good a claim on the prover as the
  kernel does. Not attempted here.
- **The `kernel` row's 5 seconds in `script/verify`'s table is a dev-Mac number**, like
  `mdns_proto`'s and `jh7110_trng`'s, and the wrong machine for that column. Replace it from the
  first CI log that carries it. Almost all of it is the crate's own compile rather than solver time,
  so it will grow with the harnesses and not with the kernel.
- **The `kernel` row needs an aarch64 host, and nothing enforces that.** `kernel/Cargo.toml` depends
  on `aarch64-cpu` unconditionally, and `crate::arch::mmu::Format` resolves by the host's
  `target_arch` under Kani, so `cargo kani -p kernel` on an x86_64 box would fail to build and would
  be proving a different `Format` if it did. It works today because **every job in
  `.github/workflows/verify.yml` runs on `ubuntu-24.04-arm`**, which is aarch64 Linux, and because
  the dev machine is Apple Silicon. Milestone 193's block predicted the opposite ("CI is x86_64
  Linux and it very likely will not [compile]"); the premise was checked and is false, and the two
  ELF-style `link_section` names it worried about are valid ELF anyway, so only the Mach-O host ever
  rejected them. If that runner label ever changes, this row breaks, and the fix is putting
  `aarch64-cpu` behind a target `cfg`.
- **This note does not price the rest of the work.** What was measured is that the front door opens.
