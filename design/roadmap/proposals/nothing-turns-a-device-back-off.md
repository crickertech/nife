# Nothing turns a device back off

**Status: PROPOSED 2026-09-04.** Named by milestone 220's lane, which added the first code in this
tree that turns a device *on* and deliberately did not add its inverse.

**Gate: DECISION.** The mechanism is small; who is allowed to hold it is calef's, because the
answer decides whether a capability variant appears on the syscall surface (§10, §16).

**What the work is.** `crates/jh7110_crg` can enable a clock and release a reset. It cannot gate a
clock or assert a reset, and `kernel/src/drivers/jh7110_crg.rs` offers no teardown at all. So a
driver process that exits, faults, or is revoked leaves its device clocked and running forever, and
nothing in the system can reclaim that. On a board with one TRNG this costs nothing measurable. On
the same `SoC`'s USB, PCIe and DMA blocks it is the whole of device power management.

**Why the lane refused to just add it.** Asserting a reset is not the mirror image of releasing
one. The JH7110's TRNG reset is documented as **shared** upstream
(`devm_reset_control_get_shared`): the same line resets the PL080 DMA engine at `0x1600_8000`. So
"turn my device off" is, in the hardware, "reset a block my neighbour is also using", and a
teardown written without that in view would interrupt a stranger mid-transaction. The refcounting
Linux gets for free from its clock framework does not exist here.

**The three questions it has to answer**, and they are the interesting part:

1. **Who may gate a clock?** Milestone 220 kept the whole controller in the kernel on the argument
   that granting it would *widen* a driver's authority rather than confine it (the STG window
   covers USB, both PCIe root ports and the DMA engine). A teardown that a driver can *ask* for is
   a different question from a controller a driver can *hold*, and the first may well be
   answerable without new syscall surface: the kernel already knows when a spawned service dies.
2. **What is the unit of reclamation?** Per-device, if something refcounts shared lines. Per-job,
   if it hangs off §40's subtree death, which is where §92's caretaker-lifetime question landed
   for an analogous reason. The second is more elegant and needs the refcount anyway.
3. **Is it worth anything before there is a power budget?** This tree measures rather than
   argues, and nobody has measured what an ungated STG domain costs radon. A milestone justified
   by tidiness rather than by a number would be the *implementation convenience* tenet running
   backwards.

**What is already recorded, so this proposal is not the only trace.**
`notes/jh7110-clock-and-reset.md`'s `BUGS` carries the limitation beside the feature, which is the
FreeBSD posture working as designed: a reader who meets the driver meets the fact that it has no
off switch. This proposal exists because the *decision* about authority has no home in a `BUGS`
entry.

**Related.** DECISIONS §86 (whether an NVMe driver can leave the kernel, and what capability would
let it) is the argument milestone 220 reused and the one this would extend. §40's subtree death is
the reclamation mechanism question 2 points at.
