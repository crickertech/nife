# `script/test`'s UEFI leg asserts two cores online, and the second one does not always start

**Status: PROPOSED 2026-09-14.**

**Gate: NONE.** Nothing is owed and nothing is missing. It is a measurement and a decision about a
gate's assertion, both of which a lane can do today.

**Found by milestone 294's lane, which had no business being anywhere near it.** That lane changed
markdown, `script/roadmap` and one comment in `script/lint`, and nothing `cargo xtask` builds. Its
first `script/test` failed:

```
  smp: init-sipi-sipi via the local apic, trampoline at 0x8000
  smp: cpu 1 did not start (firmware returned -1)
  smp: 1 core(s) online
  ...
uefi-boot: the boot transcript is missing "smp: 2 core(s) online"
```

The next two runs passed, on the same tree, same machine, same commit: **one failure in three.**

## Why this is not already the recorded bug

`kernel/src/arch/x86_64/ap_boot.rs`'s module BUGS section records exactly this class and bounds it
away from here: *"A third or later secondary, brought up while an earlier one is already online and
running, fails intermittently and non-deterministically ... At `-smp 3` and above, exactly one
secondary typically fails."* `xtask/src/main.rs` sets `NIFE_SMP=2` for the UEFI tour, and the
comment beside it reasons from the same bound: *"The suite below stays at one core, because the
two-core AP defect this tour does not touch."*

So the record says the flake starts at three cores and the gate was written believing it. **It
reaches two.** Either the bound in `ap_boot.rs` is wrong, or this is a second failure mode that
happens to print the same line, and the difference matters: the recorded one is a secondary
brought up *beside an already-running one*, which cannot be what happened at `-smp 2`.

## What this costs today

`uefi-boot` asserts `smp: 2 core(s) online` unconditionally, so the gate fails on a defect the tree
has already recorded as intermittent and not root-caused. A lane that hits it reads a red gate it
did not cause, and the cheapest available conclusion is that the flake is somebody else's problem
and the run should be repeated, which is how a gate stops being read.

Note that **no pull request check catches this**: `.github/workflows/ci.yml` runs every job on
`ubuntu-24.04-arm`, and nothing there boots the x86_64 UEFI image. It is only ever seen by a lane
running `script/test` on an x86_64 host.

## What a lane should do

1. **Measure it.** Boot `cargo xtask uefi-boot` enough times to put a rate on it at `-smp 2`, which
   one lane's three runs cannot. One in three is a sample, not a number.
2. **Decide which record is wrong**, and correct that one: either `ap_boot.rs`'s "third or later"
   bound, or `xtask/src/main.rs`'s comment that the tour does not touch the two-core defect.
3. **Then decide what the gate should assert**, which is the part worth arguing rather than
   assuming. Asserting two cores is what makes the assertion worth having (it proves the
   trampoline page the loader asked for was usable, which is what its own comment says). Retrying
   the boot, or asserting one core, both weaken it. A gate that fails one time in three is worse
   than either, and a gate that is *known* to fail one time in three and stays that way is worst.
