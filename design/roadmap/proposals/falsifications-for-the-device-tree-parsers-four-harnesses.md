# Falsifications for the device-tree parser's four harnesses

**Status: PROPOSED 2026-09-17.** Found by milestone 319, whose whole argument was built on `dtb`
being the sibling that had proofs.

**Gate: NONE.** Four patches against code that already exists.

## In brief

`crates/dtb` carries four Kani harnesses (`be32_is_total`, `be64_is_total`,
`be32_reads_big_endian_when_in_bounds`, `align4_rounds_up_to_a_multiple_of_four`) and **all four are
recorded `unfalsified`**. DECISIONS §134's three states make that an honest answer rather than a
failure, and `script/falsifications --unfalsified` is the worklist it exists to feed.

Milestone 319 argued from `dtb`: it is the crate that proved the same class of code and whose
`be32_is_total` records a real defect caught (an unchecked `at + 4`, reachable from a corrupt device
tree on the boot path). That argument cuts both ways. A harness whose record says nobody has
falsified it is, by the project's own standard, a claim one level of indirection from evidence.

## Why these four specifically

They are unusually good candidates, because the defect each one forbids is already written down.
`be32`'s doc says the checked add is "the hardening proved", so the patch is the unchecked `at + 4`
that shipped, exactly as milestone 319 restored its own two shipped defects rather than inventing
substitutes. `align4`'s harness asserts three properties over one `div_ceil`, and `div_ceil` against
`(n + 3) / 4` is the mistake that was available.

`be32_reads_big_endian_when_in_bounds` is the one to think about rather than pattern-match: swapping
two shifts is the obvious patch and would also fail the crate's host tests, which means it is not
evidence the *harness* is load-bearing. A patch that only the harness catches is what the record is
for.

## Cost

Four patches, each a few lines, each swept by `script/falsifications --sweep dtb`. An afternoon. The
ratio it moves is small (four of 168) and the crate it moves it in is the one the verification thesis
is most often argued from.

## What is blocked until it is done

Nothing.
