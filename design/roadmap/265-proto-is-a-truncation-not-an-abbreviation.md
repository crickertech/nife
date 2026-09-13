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

## Four stems calef ruled on 2026-09-13, which this milestone now carries

Working the unratified worklist, calef ruled the `mdns` family and `ntp_proto`'s stem. **The rulings
are recorded and the rename is not performed**, deliberately: doing it now means renaming the same
files twice, once here for the stem and again for the suffix. So this milestone grew by four names
and the tree grew by none.

| Today | After this milestone | Ruled |
|---|---|---|
| `mdns_proto` | `multicast_dns_protocol` | stem 2026-09-13, suffix by this block |
| `ntp_proto` | `network_time_protocol` | stem 2026-09-13, suffix by this block |
| `mdns_config` | `multicast_dns_config` | 2026-09-13 |
| `mdns_responder` | `multicast_dns_responder` | 2026-09-13 |

The last two carry no `_proto` suffix and are here because they carry the same **stem**: renaming the
protocol crate and leaving its config and its responder spelled the short way would split one
protocol across two spellings, which is the state this milestone exists to end.

**`network_time_protocol` is also the answer to a stutter.** Expanding the stem alone gives
`network_time_protocol_proto`, which says protocol twice. The suffix change removes the duplication
rather than adding to it, which is an argument for this milestone that its own block did not have.

**The external-standard exemption is narrowed by this ruling, and that has to be said out loud.**
Both crates' own provenance argued against expanding, and the argument was not weak.
`ntp_proto`'s said NTP is RFC 5905's own name for the protocol, *"the same external-standard
exemption `elf`/`pci`/`dtb`/`gpt` already carry"*. `mdns_proto`'s said the expansion does not stop
cleanly, since DNS is itself an acronym and a consistent spelling runs to
`multicast_domain_name_system_proto`. calef ruled against both on 2026-09-13, twice, having been
shown them.

So the exemption now reads: **a standard's own name stays whole where it names a format or a piece
of hardware (`elf`, `pci`, `dtb`, `gpt`), and expands where it names a network protocol.** That is a
line drawn rather than derived, and a reader is owed the reason: `elf` and `pci` are what the thing
*is* and have no useful longer form in a reader's head, where a protocol's expansion says what it
*does* (network time, multicast DNS) to someone who has not met the acronym. **DNS stops because it
is the `pci` case one level down**: domain name system teaches nothing a reader did not already
have.

**The cost is honest and is this block's to carry**: the exemption used to be one rule and is now a
rule with a boundary, and nothing mechanical can tell a format from a protocol. The next name that
tests it comes to calef.

**The `ntp` program stays `ntp`, and that is an exception that must say so.**
`design/roadmap/proposals/an-acronym-sweep-the-tree-can-do-at-once.md` names this exact pair as a
reason not to work one name at a time: *"Spelling out the program alone leaves the pair disagreeing;
spelling out the crate alone overturns a ratification as a side effect of tidying a program."* Here
the crate is expanded and the program is not, so the pair does disagree. It is deliberate rather
than a side effect: `AGENTS.md` leaves the length of a typed command to its author, which is the
same latitude that produced `mdr` on the same day, and `ntp` is what a person types. The cost is
that a reader meets `network_time_protocol` and `ntp` and must be told they are one thing. This
block is where they are told.

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
