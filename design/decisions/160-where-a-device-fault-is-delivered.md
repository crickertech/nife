# 160. Where a confined device's IOMMU fault is delivered

**Status: PROPOSED.** Raised 2026-09-19 by milestone 435's lane, which found milestone 102 gated on
`DECISION` with no decision anywhere a reader can open. The block has stated the fork since it was
raised on 2026-08-04, and three separate documents defer to a fault-handling milestone that did not
exist when they were written. *(Section number provisional until the merge queue lands it.)*

## What is being decided

**What a production kernel does when a confined device reaches outside its grant.** The IOMMU
already stops it and already records it; the question is who is told.

**The small half is not part of this.** Clearing the event queue's overflow bit on drain needs no
decision, has a witness, and is a two-driver change under the architectural-parity rule. It is
named here only so the decision is not read as gating it.

## Whether the premise is true, measured 2026-09-19

**It is.** Every call site of `crate::iommu::take_fault` outside the driver definitions is a test:
two in `kernel/src/virtio.rs` (the DMA-escape test) and three in
`kernel/src/user/display_tests.rs` (milestone 29's framebuffer work). Both architecture drivers say
so in their own comments: `kernel/src/arch/aarch64/iommu.rs` and
`kernel/src/arch/riscv64/iommu.rs` each record *"take_fault (the confinement test); no production
fault handler yet."* So a confined device that faults during an ordinary boot reports to nobody,
and the kernel discards its own evidence that hardware confinement fired.

**Three documents already say it is owed**, which is the argument that this is a gap rather than a
possibility: `notes/iommu.md`'s honest limits, [§20](20-iommu-dma-isolation.md) (IOMMU-backed DMA
isolation)'s own limits list, and `notes/framebuffer-contract.md` mirrored into
[§29](29-framebuffer-grant.md) (the framebuffer is a bigger grant, not an exemption).

## What this tree already does in the analogous case

**A thread's death is already a message its supervisor holds.** [§26](26-fault-endpoint.md) (thread
death becomes a message a supervisor holds) built exactly this shape for CPU faults: the kernel
does not print, it delivers, and the party that holds the relationship is the party told.
[§32](32-reap-without-build.md) then made the supervision relationship the unit of authority rather
than a rights bit. A device escaping its grant is the same event one layer down, and the tree
already has the vocabulary for it.

**The counter-precedent is real and should be weighed**: the kernel prints and continues for
everything it cannot handle. That is the right default for a kernel with nobody to tell. It is the
wrong one here precisely because there *is* somebody to tell.

## The options

| | what happens | politics |
|---|---|---|
| **A** | **Print and continue.** | What the kernel does with everything else it cannot handle. It puts a security event on a console with no owner, so the evidence is preserved and nothing acts on it. |
| **B** | **Deliver to the holder**, as a message on the fault endpoint the driver's supervisor already holds. | The capability-shaped answer: the party granted the device is told the device misbehaved, and may restart it, drop it or ignore it. Not free: the IOMMU's event queue is an interrupt source **neither driver registers today**, so this adds an interrupt handler per architecture before it adds any policy. |
| **C** | **Disable the device.** | Safest, and it takes the decision away from the holder, which is the thing a capability system is usually trying not to do. It is also the only option that stops a fault storm without policy. |

**Recommendation: B**, and the reason is not that it is the most capability-flavoured. It is that A
and C both answer a question nobody asked: A decides that no action is correct, C decides that one
action is always correct, and only B leaves the decision with the party that has the context.
B also costs the most, which is stated here so it can be weighed as cost: two interrupt handlers
before a line of policy.

**A is a defensible interim** and should be called that if it is chosen, with the cost written
where a reader meets it, because AGENTS.md's ladder says an unmarked exception reads as a design and
the next person extends it.

## How reversible it is

**Where the fault is delivered is a message shape two parties agree on**, which is the expensive
category. The interrupt registration underneath it is ordinary kernel work and is not. Nobody
outside this tree has acted on either, so the cost today is only the writing.

## What is blocked until this is answered

**Milestone 102's large half.** Its small half (clearing the overflow bit on drain) is not blocked
and should not wait: today's correctness rests on no test ever overflowing the queue rather than on
a drain that clears the condition, and that already made one test misreport another.

**And the proof shape is settled**, so nothing else is waiting: point a confined device at a frame
outside its domain and assert the report arrives where the design says it should, keeping the
provocation to a single translation so the flood milestone 29 hit cannot recur.
