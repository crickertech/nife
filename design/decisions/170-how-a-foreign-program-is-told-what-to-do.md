# 170. How a foreign program is told what to do

**Status: PROPOSED.** Raised 2026-09-19 by milestone 435's lane, which found milestone 205 gated on
`DECISION` with no decision anywhere a reader can open. The block was minted 2026-08-31 out of
milestone 121's lane and states the fork already; this is it in the place AGENTS.md says an open
decision lives. *(Section number provisional until the merge queue lands it.)*

## What is being decided

**How a program written by somebody else receives its arguments.** Every future program is written
against the answer, which is AGENTS.md's irreversible category alongside the syscall surface, so
this arrives as options rather than with a winner.

## Whether the premise is true, measured 2026-09-19

**Half of it.** The block says *"The nife ABI has no argument vector"*, and that is right. It is not
true that programs here take no arguments, and the difference decides the options.

`grant_plan::Endowment`, which is what the shell hands a child, carries:

- `arg: u64`, *"the integer argument to start it with (0 when the program takes none)"*;
- `flags: u64`, a bitmask where **bit `i` is set when the manifest's `flags[i]` was typed**,
  numbered by position rather than by letter, so nothing in the path has to know what any option
  means.

A program declares what it takes in a `Manifest` (`ArgSpec::Required`, `ArgSpec::Forbidden`, a flag
string), and the shell binds positional tokens into the slots the manifest declares, yielding either
an endowment or a typed refusal printed at the prompt.

**So the tree has an argument model. It is manifest-declared, capability-shaped, and it carries no
strings.** What unmodified `ripgrep` reaches for is `std::env::args()`, which compiles std's
`unsupported` backend and yields nothing, so a stranger's program stops at its own usage error.

## What this tree already does in the analogous case

**This exact fork was answered once, one category over.** Milestone 47 split what Unix puts in a
single string-to-string environment map into three parts, and each third got a different answer:

- **inert configuration** became a validated read-only page
  ([§111](111-inert-config-is-a-validated-page.md), inert configuration is a read-only page);
- **names** (`PATH`, `HOME`) were answered by the namespace work, because designation is
  authorization;
- **secrets** were answered by an endpoint ([§41](41-endpoint-as-broker.md), the endpoint is the
  broker).

**The lesson is the split, not any one of its answers.** "Arguments" is the same kind of word as
"environment": one Unix container holding several different things. A file path on a command line is
a designation, a `--jobs 8` is inert data, and a `--password` is a secret that should never have been
there. An answer that treats all three as one byte string imports the problem §111 was written to
refuse.

**And [§15](15-native-abi.md) (the native ABI) already chose** out-of-band capability slots over a
self-describing environment, precisely to avoid inheriting Unix's shape by default.

## The options

| | shape | cost |
|---|---|---|
| **A** | **An argv, as POSIX has it.** | What every ported program expects, so the corpus milestone 123 wants runs unmodified. It imports a convention §15 deliberately refused, and it puts designation, data and secrets back in one undifferentiated string vector. |
| **B** | **A nife-shaped equivalent**, where arguments arrive the way capabilities do: designations as capabilities, inert values as data, per §47 and §111's split. | Coherent with everything above and with milestone 47's conclusion that designation is authorization. **Every foreign program needs a shim**, and nobody has costed one. |
| **C** | **`grant_plan` is the only answer**, and foreign command-line programs get a shim by design rather than by omission. | The most honest about what this system is, and the least welcoming to the corpus 123 needs. It is B with the shim made explicit policy rather than a migration path. |

**No recommendation.** The choice determines whether the ecosystem risk stays retired or returns as a
per-program tax, and a syscall-surface-class decision arriving with a recommendation is already most
of the way made.

**What would make the choice cheap, and it is a lookup nobody has done**: cost one shim. B and C
differ only in whether the shim is temporary, and A wins outright if a shim turns out to be
per-program work rather than a library. That measurement is worth taking before the ruling, and it is
a lane's work rather than calef's.

## How reversible it is

**Not at all, in the way that matters.** Every future program is written against it, and so is every
port. Nothing outside this tree has acted on it yet, which is the only reason it is still cheap.

## What is blocked until this is answered

**Milestone 205, and everything milestone 121 still owes**: the confined `rg`, the loud `ENUMERATE`
refusal, and the walk benchmark. Milestone 123's corpus is behind it too.

**Not decided here, and arriving right behind it**: environment variables for a foreign program
(§111 answers the nife-shaped case and says nothing about `std::env::var` on a ported binary) and
exit codes. They are the same family and should be ruled together or explicitly deferred, not
discovered.
