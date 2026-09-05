# 111. A shell that can endow a child with entropy

**Status: NOT-STARTED.** *(Gate answered 2026-09-05; see below.)* Raised 2026-08-04 from `notes/entropy.md:194`, which calls it "future work
with no design problem in it". The smallest entry in this sweep, and the block should say so rather
than dress it up.

**Gate: NONE.** Answered by calef on 2026-09-05: **give it a lane now**, against this block's own
recommendation, which was to fold it into milestone 65 or milestone 31's phase two.

**The recommendation was not wrong when it was written; its premise acquired an expiry date.** The
gate asked whether anything typed at the prompt needs entropy before either of those runs, and said
nothing does. That is still literally true: `disk_partitioner` is spawned only by the test harness
(`kernel/src/user/disk_tests.rs`), never by `init` and never as a shell command. But it is the
program a person types to partition a disk, and on the same day a milestone was minted to wipe
xenon's NVMe so nife can drive it (the EL0 NVMe driver §86 decided, whose number the integrator
mints at merge). A GPT gives every partition a random globally unique id and
`crates/gpt` refuses to invent one, so the moment that disk is partitioned from a prompt this is on
the path.

**The ranking argument against a lane was that this has no consumer today**, and calef's answer is
sequencing: the work is an afternoon whichever way it is done, and doing it now means it lands
before 261 needs it rather than after. Folding it would have made it arrive late for the one
consumer that can be named.

**The finding.** Milestone 56 built the entropy service and the grant that reaches it. Nothing at
the prompt can pass that grant on. The note, under BUGS:

> **`init` does not endow the shell with entropy.** The std wiring and the milestone-56 tests do.
> Ambient entropy would be ambient authority, and the point of the grant is that a program's
> dependence on randomness is visible in what it holds. A shell that needs to hand entropy to a
> child is future work with no design problem in it.

So a program that needs randomness works when the system spawns it and cannot be run by a person.

**Why the design is already settled.** The manifest already expresses per-program endowments and the
shell already plans grants against what it holds (`crates/grant_plan`). Entropy is one more
endowment of the same kind: a program declares it needs the service, the shell hands over the
endpoint it holds, and a program that did not declare it gets nothing. There is no new mechanism,
no new right, and no question about what "ambient" would mean, because the answer is the same answer
the rest of the shell already gives.

**What it costs.** Init endows the shell, the manifest grows an entropy endowment, and the shell's
planner learns one more capability to place. The interesting part is not the wiring: it is that
`caps <program>` then prints "entropy" for a program that draws random numbers, which is the visible
form of the property milestone 56 built the service to have.

## Scope note

**This may not deserve its own lane, and the honest options are two.**

- **Fold into milestone 65 (a secrets service).** 65 holds keys and exposes operations, so it will
  need the shell to endow a child with a *service* endpoint under exactly this pattern. If 65 is
  scheduled, this is one endowment inside it and should not be a separate lane.
- **Fold into milestone 31 (a capability shell), phase two.** 31 owns per-file grants pointing at FS
  server directory capabilities, which is the same planner change with a different capability in it.

**What would decide it:** whether anything at the prompt needs entropy before 65 or 31 runs. Today
nothing does, which is why this has sat unbuilt without hurting. The moment a typed command needs
randomness (a `uuid`, a key generator, anything in 65's family), it stops being foldable and becomes
a prerequisite. Until then, folding is the better answer and this block exists so the work is not
lost in a BUGS list while the fold is being decided.

**Not a health test, a rate limit, or a hardware TRNG.** `notes/entropy.md` records all three as open
and they are the service's business, not the shell's. In particular "nife has a cryptographic
random source" is still a claim about QEMU until the JH7110's TRNG is verified on the VisionFive 2,
and endowing a shell with a grant does not change what is behind it.
