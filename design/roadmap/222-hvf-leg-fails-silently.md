# 222. The one command a person runs before pushing has a leg that fails instead of skipping

**Status: NOT-STARTED.** Minted 2026-09-02 by the maintainer, from milestone 208's (the x86_64
kernel image ships an RWX segment) lane, which hit it while proving an unrelated boot path and
reproduced it outside this project. *(Number provisional until the merge queue lands it.)*

**Gate: NONE.** The diagnosis is done and nothing here needs hardware this project does not have.

**In brief.** `script/test --hvf` fails:

```
qemu-system-aarch64: HVF does not support GICv2 emulation
```

Reproduced on 2026-09-02 with a bare `qemu-system-aarch64 -machine virt,gic-version=2 -accel hvf`
and **no nife kernel involved**, on QEMU 11.1.1. The runner asks for GICv2; HVF requires GICv3.

**Why it is worth a block rather than a shrug.** `script/gates` calls this *the one command a person
runs before pushing*, and the `--hvf` leg is where it reaches a real core rather than an emulated
one. Today that leg is **skipped by failing**, which is worse than either alternative: a leg that
runs tells you something, a leg that skips loudly tells you it did not, and a leg that fails tells
you something is broken without saying whether it is yours. A contributor meeting this has no way to
know the failure is the accelerator's constraint rather than their change.

That is AGENTS.md's third principle, the newcomer who must succeed without asking anyone, failing at
the exact command the tree tells them to run.

## What it needs

**A decision this block does not make**, because both answers are defensible and the choice should
be recorded rather than assumed:

- **Ask for GICv3 when the accelerator is HVF.** The runner already knows which accelerator it is
  configuring. `gic.rs` is GICv2-only (milestone 127's scope note records this and says a GICv3
  driver should be minted deliberately someday), so this may not be a one-line change at all, and
  finding that out is the first thing to do.
- **Skip the leg loudly** when the combination is unavailable, which is honest and cheap and leaves
  the tree with no accelerated coverage on this machine.

**Whichever is chosen, the failure must stop being ambiguous.** That is the part that is not
optional.

## BUGS

- **This is not a fatal risk and does not pretend to be.** It is a gate telling a contributor
  something false, which this tree treats as a defect in the tree rather than in the contributor.
- **The GICv3 route may be a much larger milestone wearing a small one's clothes**, since the driver
  is GICv2-only by design and the last GICv2 silicon anyone will buy is argon's generation.
- **Nothing here checks the other accelerated paths.** Whether the same shape exists for KVM on
  cordoba, or for WHPX anywhere, is unexamined.
