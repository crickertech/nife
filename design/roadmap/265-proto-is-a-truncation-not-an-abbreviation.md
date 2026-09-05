# 265. `_proto` is a truncation, and it collides with the other word it could be short for

**Status: NOT-STARTED.** Minted 2026-09-05 by calef, on being shown `timebase_proto` for
ratification: *"I think `_proto` was lazy on my part. It should have been `_protocol` globally to
differentiate from prototype."* *(Number provisional until the merge queue lands it.)*

**Gate: NONE.** The sequencing below is not optional even so: this is a 349-file rename and it wants
a quiet tree.

## The measurement

**14 crates, referenced across 349 files.**

```
byte_sink_proto   clock_proto      credential_proto  entropy_proto    environment_proto
filesystem_proto  graphics_proto   login_proto       mdns_proto       ntp_proto
socket_proto      supervision_proto swap_proto       timebase_proto
```

## Why, and the rule it fails is the tree's own

**`proto` is not an abbreviation, it is a truncation.** `notes/naming.md` already refuses the shape:

> Truncating a word you happen to be tired of typing is not abbreviation, it is shorthand, and
> shorthand is what the third principle ("a newcomer must be able to succeed without asking anyone")
> exists to refuse.

And the test it gives: *"would a competent stranger who has never read this tree recognise it?"*
`pci` passes that. `proto` does not, because **it is equally short for `prototype`**, and this tree
uses that word for a real thing. Milestone 263's spike built a prototype and deleted it on purpose on
2026-09-05, and wrote about doing so in a tree carrying fourteen `_proto` crates. **The ambiguity is
live rather than theoretical.**

**It also fails the acronym rule set the same day**, one category over. That rule asks whether an
expansion teaches: `pci` expands to peripheral component interconnect and the reader is no wiser, so
it stays. `proto` expands to **protocol**, which is exactly what these crates are and is the fact a
reader most needs, so it goes. The rule was written for acronyms and the principle is the same: keep
the short form when it teaches nothing, spell it when it teaches.

**calef named it as his own laziness**, which is worth recording because the convention was his and
because §75's own line is that the refusals are the valuable half. This one was never refused, only
never examined.

## What this is

**A mechanical rename, tree-wide.** Directory, package name, every `use`, every `Cargo.toml`
dependency, and the prose that cites them. `_proto` becomes `_protocol`; nothing else about these
crates changes.

**Milestone 63 is the precedent** and did about twenty names in one pass, including three directories
whose package names matched neither the directory nor the rule.

## Sequencing, which is the whole risk

**It collides with almost everything, so it goes when the tree is quiet.** Two lanes are already in
the naming area: milestone 264 is writing provenance blocks for sixty unrecorded names, and milestone
91 will touch nearly every documentation file. **This should follow both**, and 91's own block already
carries the same constraint for the same reason.

**And `timebase_proto` should not be ratified before this lands.** It is on
`script/names --unratified` as provisional today, and ratifying it would settle a name into a form
this milestone is about to change. calef held it back on 2026-09-05 for exactly that reason.

## BUGS

- **This is a rename with no functional change**, which makes it the kind of diff nobody reads
  carefully. A mechanical sweep across 349 files can quietly take a line it should not, which is
  what the blind `sed` in this tree's own history did when it rewrote the row recording that a name
  had been *refused*. Whoever does it should say what pattern they used and what they checked it
  against.
- **It does not touch `_rt`, `_cli` or any other suffix**, and nobody has checked whether the tree
  carries other truncations of the same kind. That sweep is a different milestone and this block does
  not claim it.
- **Fourteen crate names get five characters longer**, and `nifefs` caps archive names at 32 bytes.
  Crates are not in the archive, so nothing here is bounded by it, but a program taking one of these
  names later would be.
