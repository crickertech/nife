# An acronym sweep the tree can do at once

**Status: PROPOSED 2026-09-05.** Named by milestone 264, which asked the acronym question of sixty
names and deliberately answered none of them, because answering one at a time is the failure mode.

**Gate: DECISION.** calef names things, and this is a list of names.

## Why it cannot be done a name at a time

**An acronym is spelled out unless its expansion teaches nothing** (calef, 2026-09-05). The rule
deratified `dma`, `dtb`, `gpt`, `ipc` and `asid` in the sentence that set it, and notes/naming.md
already says the sweep is its own milestone because `ipc` is load-bearing across the tree.

Milestone 264 found the second reason, which is smaller and sharper. **Several of these names exist
in matched pairs that a partial sweep would break.** The program `ntp` and the crate `ntp_proto` are
one word twice, and calef ratified the crate on 2026-08-23. Spelling out the program alone leaves the
pair disagreeing; spelling out the crate alone overturns a ratification as a side effect of tidying a
program. The same holds for `jh7110_trng`, which is a crate and the program built from it, a pairing
AGENTS.md describes as deliberate and worth seeing.

## The names 264 surfaced, each with the question already asked

Every one of these carries the question in its own provenance block, recorded as open rather than
guessed at.

| Name | Expansion | The tension |
|---|---|---|
| `ntp`, `ntp_proto` | network time protocol | Expands into something more informative than itself, which is the deratified class. Against: it is the protocol's registered name and `ntp_proto` is ratified. |
| `mdns_proto`, `mdns_config`, `mdns_responder` | multicast DNS | The expansion contains a second acronym. A full spelling runs to `multicast_domain_name_system_proto` and has stopped teaching before it ends. |
| `jh7110_trng` (crate and program) | true random number generator | Expansion teaches, and the acronym is not one a reader outside hardware carries. |
| `jh7110_crg` | clock and reset generator | Expansion teaches. Against: both device trees for this chip spell the blocks `syscrg`, `stgcrg` and `aoncrg`, so the acronym is the hardware documentation's own. |

`cpu` and `icount` were asked and answered inside 264: `cpu` expands to something a reader already
has, and `icount` is a contraction rather than an acronym and is the exact string QEMU prints and
accepts.

## What settling it needs

A ruling per name, and one decision on the nested case (`mdns`), where the rule's own asymmetry
argument runs out: spelling out the outer acronym exposes an inner one, so the expansion does not
reach a word the newcomer knows either way. That is the interesting question in this list and it is
not answerable by applying the rule harder.

## BUGS

- **The five names the rule deratified by name are not in the table above**, because 264's scope was
  the sixty unrecorded ones and all five were ratified. They are the larger half of the work and
  `ipc` is the reason this is a milestone rather than an afternoon.
