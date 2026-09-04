# Source comments that lead with a machine name instead of the hardware

**Status: PROPOSED 2026-09-03.** From DECISIONS §143 (a machine's name is not a hardware fact, and
source comments should say the hardware), decided the same day.

**Gate: NONE.** Twelve comments and one index line.

**What the work is.** §143 decided that a source comment states the hardware and may name a machine
only as a trailing gloss where the instance is genuinely the point. The tree mostly does this
already; twelve uses need bringing into line, and they are of three kinds.

**Two carry the fact with no hardware at all**, and both are really claims about the SoC or about
silicon-versus-emulation, which is what the reader needs:

- `kernel/src/soak.rs`: *"on radon, in `wake_load_aware`"*
- `kernel/src/arch/riscv64/timer.rs`: *"recorded that as unknown on **radon** and it is still
  unknown"*

**Nine gloss the name but lead with it** (`"on radon (the StarFive VisionFive 2)"`), where the
relevant half belongs first.

**One is example data**, `crates/mdns_proto`'s `host: "patagonia"`, which §143 explicitly puts
outside the rule. Changing it is a preference (a household name in a public crate's documentation),
not a consequence of the decision, and whoever does this should say which they acted on.

**And `notes/README.md` indexes `target-hardware.md` as "Where nife could actually run"**, which does
not advertise that it is where the machine names are defined. A stranger who meets `radon` and wants
to resolve it has no obvious route. One clause fixes it, and it is the smallest and most valuable
part of this.

**What it must not do.** `notes/` and `design/` keep their machine names: about a hundred uses, and
they are load-bearing there because a measurement series has to assert that the same physical machine
was held constant. A sweep that treated this as a rename would destroy the thing that makes
notes/soak.md's nine-boot spread evidence at all. §143's `BUGS` records that the boundary is a
judgement, so this wants a reader rather than a `sed`, which is the lesson of the blind rename
already on AGENTS.md's record.
