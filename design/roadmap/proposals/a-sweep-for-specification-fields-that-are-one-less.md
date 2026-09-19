# A sweep for specification fields that are stored one less than their value

**Status: PROPOSED 2026-09-17.** Found by milestone 319, which met this shape twice in one crate,
broken once and correct once, with nothing at either site saying which.

**Gate: NONE.** It is a `git grep` and a reading, not a mechanism.

## In brief

Hardware specifications routinely store a count or a width as one less than the number meant, so
that zero is a legal encoding. Undoing that needs one more value than the field's own type can hold,
and whether the widening happens before or after the `+ 1` decides whether the top of the range
panics.

Milestone 319 found both spellings within one crate:

- **Broken.** `machine_discovery::acpi::parse_dmar` read `body[0] + 1` into a `u8`, so a DMAR whose
  `HostAddressWidth` byte is `0xff` overflowed. `kernel/src/arch/x86_64/machine.rs` calls that
  directly on firmware bytes, so the panic was on the x86 boot path.
- **Correct.** `machine_discovery::riscv64::CounterInfo::bits` reads
  `Some(self.raw_width as u32 + 1)` for the SBI PMU counter width, widening first.

`device_tree_blob::be32`'s unchecked `at + 4` (milestone 18) is the same family one step removed.

## Why a sweep rather than a gate

**`cargo mutants` cannot find it**, which is worth saying because it is the tool this tree reaches
for. The mutation is in the *type*, not in the expression: `body[0] + 1` and `body[0] as u16 + 1`
are the same operator on the same operands, and a mutation testing tool that flips `+` to `-` finds
neither. Clippy has no lint for it either, because the arithmetic is correct at every value the
author had in mind.

What does find it is reading, guided by the specification's own wording. The tell is a doc comment
containing "one less than", "minus one", "N - 1", or "zero means one".

## What it would do

`git grep -in 'one less than\|minus one\|zero means one'` over `crates/` and `kernel/`, then for each
hit check that the type the `+ 1` lands in is wider than the field it came from. Where it is not,
widen the field rather than guarding the addition: making the wrong state unrepresentable is rung one
of AGENTS.md's ladder and guarding is rung two, and in both cases found so far the wider type is also
the honest one.

Add a harness where the value crosses a trust boundary, which is the rule milestone 319 already
applied: `the_dmar_fixed_part_decodes_without_arithmetic_overflow` and
`a_counter_width_is_widened_before_the_specifications_off_by_one_is_undone` are the two shapes to
copy, one for the broken case and one for the correct one.

## What is blocked until it is done

Nothing. It is cheap, and the reason to write it down rather than do it is that it spans crates no
single lane owns.
