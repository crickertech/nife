# 159. Whether a holder can give up a mapping, and what gives it up

**Status: PROPOSED.** Raised 2026-09-19 by milestone 435's lane, which found milestone 95 gated on
`DECISION` with no decision anywhere a reader can open. The block has called it *"a design fork for
calef before it is a task"* since it was raised on 2026-08-04 out of milestone 22's closing lane.
*(Section number provisional until the merge queue lands it.)*

## What is being decided

**Whether a holder can give up a mapping it made, and if so by what method.** Three parts, and the
third can make the first two moot:

1. Does unmap live on the address space or on the frame.
2. What it does to a mapping some other holder also has.
3. Whether a loader that never holds more than one page at a time removes the need for it.

## The finding, re-measured 2026-09-19 and unchanged

There is no unmap in the ABI. `crates/abi` gives an address space `MAP_INTO` (with `MAP_RO`,
`MAP_RW`, `MAP_CODE`) and a listing method, and a frame `MAP`; nothing removes a mapping. So the
progenitor maps each page it lays down for a child in order to write it and never lets go, which
`notes/trusted-init.md`'s honest-limits section states in its own words:

> is never unmapped (it cannot be: nothing in the ABI unmaps a page), so the progenitor can read and
> write any page it laid down for a child.
>
> -- notes/trusted-init.md

Reaping a job hides this for jobs, because reclaiming a region revokes every mapping of its pages
first ([§13](13-frame-revocation.md)), but the boot servers are never reclaimed. The note names the
consequence precisely: the console's, the line editor's, the input driver's, the shell's and the
terminal sink adapter's memory is still reachable from the progenitor, and **giving the
construction budget away does not touch that.**

There is a second, smaller instance in the same note: the shell's output frame stays mapped in the
progenitor for life, because `Frame::REVOKE` would take the page from the shell too.

## What this tree already does in the analogous case

**Take-back already exists, and it is not this.** [§41](41-endpoint-as-broker.md) (a device is
revoked by taking it back) made `Frame::REVOKE` on an `Object::DeviceFrame` delete every capability
naming the page from every cspace *except the invoker's*. That is the right shape for handing a
device between two servers and the wrong shape here: the progenitor wants to drop its own window
and leave the child's alone, which is the exact inverse.

**And [§13](13-frame-revocation.md) already decides what happens to another holder's mapping** when
a region is reclaimed. Whatever unmap does must not contradict it, which is a constraint on the
options rather than an answer.

## The options

| | shape | cost |
|---|---|---|
| **A** | **`AddressSpace::UNMAP(va)`**, symmetric with the `MAP_INTO` that created the window. | A new method on an existing object, which AGENTS.md allows within the model provided its semantics are recorded here. Says what it means: *this* space gives up *this* window, and no other holder is touched. Needs a rule for unmapping a va that was never mapped, and a decision on whether the frame capability survives. |
| **B** | **`Frame::UNMAP`**, the holder naming the frame rather than the address. | Reads as a second revoke and invites confusion with §41's take-back, which already dispatches on the object kind. A frame mapped at two addresses in one space has no unambiguous answer. |
| **C** | **No new method: a one-page loader.** The builder maps a page, writes it, releases it before the next, so the window is never wider than one page and closes when construction ends. | No syscall surface at all, which is the strongest thing that can be said about an option here. It does not remove the window, it bounds it: the last page stays mapped unless something gives it up, so C is likely A plus a loader change rather than an alternative to it. Its price is boot time, and it is measurable. |

**Recommendation: measure C first, then A.** Not because C is less work (it is a restructure of the
loader against a one-line method, so it is more), but because the measurement decides whether the
syscall surface has to grow at all, and this project's rule is that the surface stays small and
every method is deliberate. If a one-page loader costs nothing observable at boot, the question
becomes whether A is still worth a permanent boundary change for the residual last page; if it
costs real time, A is the answer and the measurement said so.

**The one question C cannot answer** is what a *program* does when it wants to give up a window it
made for its own reasons. Nothing in this tree asks for that today, which is why C is worth
measuring rather than dismissing.

## Would we still choose this if both options cost the same

**Yes, and this is worth saying because the recommendation looks like the cheap one.** If a loader
restructure and a new syscall were the same amount of work, the ordering would be unchanged: the
surface is a boundary every future program is written against, and taking the measurement first is
what makes the addition deliberate rather than reflexive. The effort argument would run the other
way, since C is the larger change.

## How reversible it is

**A is the irreversible half.** A new method on the address-space object is part of the surface
forever. C is a refactor of one program and can be undone in an afternoon. Nothing outside this
tree has acted on either.

## What is blocked until this is answered

**Milestone 95.** Its proof shape is already settled and needs nothing from here: the progenitor
writes to a boot server's page and faults, as a negative control, which is the shape milestone 22
used.

**Not blocked:** the record. The residual is written where a reader meets it, in
`notes/trusted-init.md`'s honest limits, and stays there whatever is decided.
