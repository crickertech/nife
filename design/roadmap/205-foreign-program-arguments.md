---
status: NOT-STARTED
raised: 2026-08-31
milestone_dependencies: none
decision_dependencies: 170
machine_requirements: none
specific_machine: none
needs_person: no
---
# 205. How a foreign program is told what to do

Minted 2026-08-31 from milestone 121's (`ripgrep`: enumeration as a
capability) lane. *(Number provisional until the merge queue lands it.)*

Every future program is written against the answer, which puts it in AGENTS.md's
irreversible category alongside the syscall surface, so this arrives as options rather than a
recommendation. It is §170 (how a
foreign program is told what to do), written up 2026-09-19 by milestone 435's lane, which also
corrected one premise: this ABI has no argument *vector*, but it does have arguments, one `u64` and
a flag bitmask bound into a program's declared manifest by `grant_plan::Endowment`.

**In brief.** Unmodified `ripgrep` runs on nife and stops at argument parsing, because
**`std::env::args()` compiles std's `unsupported` backend and yields nothing.** The nife ABI has no
argument vector. A stranger's program reaches its own usage error and can go no further.

Everything milestone 121 still owes is behind this: the confined `rg`, the loud `ENUMERATE` refusal,
and the walk benchmark.

## The options, and none is obviously right

- **An argv, as POSIX has it.** What every ported program expects, and it imports a convention this
  ABI deliberately does not have. DECISIONS §15 (the native ABI) chose out-of-band capability slots
  over a self-describing environment precisely to avoid inheriting Unix's shape by default.
- **A nife-shaped equivalent**, where arguments arrive the way capabilities do. Coherent with §15 and
  §47's conclusion that designation is authorization, and every foreign program needs a shim.
- **Decide that `grant_plan` is the only answer**, and foreign CLIs get a shim by design rather than
  by omission. The most honest about what this system is, and the least welcoming to the corpus
  milestone 123 (the demonstration: somebody else's software, running narrow) needs.

**No recommendation.** The choice determines whether the ecosystem risk stays retired or comes back
as a per-program tax, and it is the kind of decision AGENTS.md says reaches calef as options.

## BUGS

- **This block does not price any option**, and the shim option in particular is only cheap if
  somebody has costed one, which nobody has.
- **It says nothing about environment variables or exit codes**, which are the same family of
  question and will arrive right behind it.

## Index row

Minted from milestone 121's lane. Unmodified `ripgrep` runs and stops at argument parsing: `std::env::args()` compiles std's `unsupported` backend and yields nothing, because the nife ABI
has no argument vector. Everything 121 still owes is behind it. Gate: DECISION, and it arrives as
options rather than a recommendation, because every future program is written against the answer.
