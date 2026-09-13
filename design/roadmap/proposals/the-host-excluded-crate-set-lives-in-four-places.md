# Four files each record which crates cannot compile for the host, and one of them derives it

**Status: PROPOSED 2026-09-12.** Written from milestone 278's block, which touches all four and
would have made it five.

**Gate: NONE.** `script/lint` already derives the set for its own check, so the hard half exists;
what is missing is the other three consuming that derivation instead of restating it.

**In brief.** A crate that reaches `crates/user_rt` cannot compile for the host, because `svc` and
`ecall` from EL0 on a machine with no nife kernel under it are a fault. Four separate places record
which crates those are: `script/lint`'s two clippy invocations, `xtask`'s `test`, `script/coverage`'s
exclusions, and `.cargo/mutants.toml`'s. **`script/lint` asks cargo.** The other three carry a list.

## Why this matters

This has already broken once, and the way it broke is the argument. The exclusion set went wrong on
**2026-08-03 and stayed wrong until 2026-08-14**, invisible for eleven days, because CI had moved to
an aarch64 runner the same day, where the EL0 assembly compiles by accident. `script/lint`'s own
header records that, and the derived check it now runs is the repair.

Milestone 244 then found the repair had covered **two consumers, not four**, and that the two it had
missed are the two that publish a number: coverage and mutation. `.cargo/mutants.toml` says in its
own head comment that its list "deliberately mirrors script/coverage's exclusions" and asks the next
person to keep the two in step. That is rung four of the ladder, in a file whose output is quoted in
a fatal risk's verdict.

Milestone 278 moves two modules out of `user_rt` into a crate that *can* compile for the host, which
means editing all four lists in one change. A fifth list would be added by the next milestone that
splits anything.

## The shape

`script/lint` already runs the derivation: ask cargo which workspace members reach `user_rt`, and
check every consumer excludes every one. Two ways to close the class, and choosing is the work:

- **Emit the set.** One command prints the exclusion list, and `script/coverage`, `xtask test` and
  the mutants config consume it rather than restate it. `.cargo/mutants.toml` is static TOML, which
  is the awkward one and may need generating or a `--exclude` passed at the call site instead.
- **Keep the lists and keep the check.** Weaker, and it is roughly today's state with the gate
  widened to four consumers, which milestone 244 already did. The remaining defect is that four
  files still have to agree; the gate only tells you after they stop.

The first is the higher rung. The second is honest about cost and should say so out loud if chosen.

## What this is not

It is not a change to the exclusion itself. `user_rt` is correctly excluded and stays excluded;
milestone 278's argument is that the *boundary* is in the wrong place, not that the rule is wrong.

## BUGS

- **Nobody has priced the `.cargo/mutants.toml` half**, which is the one that decides between the two
  shapes. Static TOML cannot read a command's output, so either the file is generated (and then a
  generated file is checked in, which this tree has opinions about) or the exclusions move to the
  `script/mutation` call site and the file stops being the record. Both are defensible and neither
  has been measured.
