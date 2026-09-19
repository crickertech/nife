# 168. Where a program image starts, and where the stack goes

**Status: PROPOSED.** Raised 2026-09-19 by milestone 435's lane, which found milestone 206 gated on
`DECISION` with no decision anywhere a reader can open. The block was minted 2026-08-31 from
milestone 121's lane, which hit the ceiling the hard way. *(Section number provisional until the
merge queue lands it.)*

## What is being decided

**The user address-space layout**: where a program's image starts, where its stack lives, and
therefore how large an image may be. `USER_STACK_VA` is a constant more than one program agrees on,
which AGENTS.md puts in the expensive category.

## The measurement, re-checked 2026-09-19

- `crates/user_mode_runtime/link.ld` sets `. = 0x400000;`, so every program's ELF loads at
  `0x40_0000`.
- `kernel/src/user.rs` defines `USER_STACK_VA = 0x50_0000` and `USER_STACK_TOP = USER_STACK_VA +
  FRAME_SIZE`.
- 32 std stack pages sit below the stack base.

**So a program image has under 896 KiB.** `ripgrep`'s `.text` alone is 1.37 MiB.

**The failure is `Unmappable(AlreadyMapped)`**, which names an overlap and not a size, so nobody
hitting it learns what is wrong. Milestone 121's lane worked around it by relinking at `0x100_0000`,
derived from the linker script by substitution so the two cannot drift.

**One correction to the block, from re-measuring**: it says `USER_STACK_VA` is *"a protocol constant
in two `_proto` crates"* and names `supervision_protocol` and `c_seam`. Today it appears in **23
files**, and among the crates the protocol-side reference is
`crates/counter_frequency_protocol`, whose own documentation places its page *"deliberately far above
every other low-half address this tree hands out"* and reasons explicitly against `0x40_0000` and
`0x50_0000`. `crates/line_editor` and four files under `components/` refer to it in documentation
when explaining why their buffers live in `.bss`. The conclusion is unchanged and the shape is
sharper: **it is a layout several programs reason about, not a private kernel detail**, and the lane
measured that moving it alone breaks `authority_tests` at stage 10.

**And the low few megabytes are already crowded.** `counter_frequency_protocol`'s note records that a
first attempt to place its page at `0x60_0000` collided with `fixtures/src/window.rs`'s own `CTL_VA`,
caught by a full-suite x86_64 run as `AlreadyMapped`. So any new layout is placing things among
fixtures that also picked addresses, not into empty space.

## What this tree already does in the analogous case

**Addresses two programs agree on are already treated as wire values here.** The
`counter_frequency_protocol` page is sited with a written argument rather than a convenient number,
which is the precedent to follow: a layout decision carries its reasoning at the constant.

**And the std heap is already far away**, based at `0x4000_0000`, with shared pages above it. So the
tree already has a high-half-ish region for large things; what it does not have is a rule saying
which band is for what.

## The options

| | layout | cost |
|---|---|---|
| **A** | **Leave it, fix the error message.** `AlreadyMapped` says the image overlaps the stack and names both addresses. | Cheapest by a wide margin, and it is the best part of this whatever else is chosen: it converts a mystery into a sentence. It does not raise the ceiling, so the corpus milestone 123 wants still does not fit. |
| **B** | **Move the stack up**, leaving the image at `0x40_0000` with a much larger gap below the stack. | One constant moves. It touches every file that reasons about `USER_STACK_VA` and it has to clear the fixtures that already picked addresses in the low megabytes. |
| **C** | **Move the image**, as milestone 121's lane did for `ripgrep`, and site the stack relative to it. | Keeps the stack where things expect it. Changes the linker script, which is the value the loader and every ELF agree on. |
| **D** | **Write the address-space map down**: bands for image, stack, heap, protocol pages and fixtures, with the constants derived from it. | The only option that stops this recurring, and it is what the `0x60_0000` collision says is already needed. Largest, and it wants the fixtures brought under the same rule rather than left picking addresses. |

**Recommendation: A now, D as the decision.** A is not a compromise on D; it is independent of every
layout question and should ship whatever is ruled, because a reader hitting a size limit should be
told it is a size limit. D is the answer to the actual question, and the reason it beats B and C is
that both of those pick a new number without writing down what the numbers mean, which is how
`0x60_0000` happened.

**Would we still choose D if all four cost the same?** Yes, and this is the case where it matters:
D is the most work by a large margin and is still the right answer, because the thing being bought
is what a reader can know rather than what a program can do.

## How reversible it is

**The constants are the expensive part and the ceiling is the visible symptom.** A layout several
programs reason about cannot be changed quietly, and the 23 files are only the ones that name the
symbol. Nothing outside this tree has acted on it, so the cost is entirely internal today and grows
with each program added.

## What is blocked until this is answered

**Milestone 206**, and behind it **milestone 123**'s corpus, since 896 KiB is small for anything with
a dependency tree. This ceiling is the second thing a foreign program meets, right after
[§167](167-how-a-foreign-program-is-told-what-to-do.md) (how a foreign program is told what to do).

**Not blocked:** the error message, which is option A and needs nobody.
