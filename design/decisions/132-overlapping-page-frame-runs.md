# 132. What `PageFrame::REVOKE` owes an overlapping run

**Status: PROPOSED.** Raised 2026-08-27 by milestone 142's lane, out of the adversarial security
review of DECISIONS §102's build. **The section number is provisional**: a lane does not mint one,
and the integrator renumbers this at merge like every other global name. Cited from
`kernel/src/revoke.rs`'s `revoke_page_frame_run` BUGS section, `kernel/src/sched.rs`'s
`delete_page_frame_caps`, and notes/frames.md.

**Nothing is blocked by this today.** What is blocked is the first `Rights::GRANT` on a run
capability: `PageFrame::REVOKE` requires `GRANT` (`kernel/src/syscall.rs`), no run capability in
the tree carries it, and so no path below is reachable. That is the whole reason this is a written
decision rather than a fix: the fix is unreachable and the wrong one is expensive.

## What is being decided

§102 gave `Object::PageFrame` a page count, so one capability names a run. It said `REVOKE`
"unmaps it from every address space and deletes every capability to it", and it did not say what
"every capability to it" means when two capabilities name **overlapping but unequal** runs, because
when it was written no two capabilities could overlap.

Three questions, and they separate cleanly:

1. **Which capabilities does `REVOKE` delete?** Only the object invoked (today), or every
   capability whose run intersects it?
2. **Which mappings does `REVOKE` unmap?** Every address space that maps the physical page (today,
   `unmap_everywhere`), or only the mappings made under the capability being revoked?
3. **What does `REVOKE` owe a device?** Nothing today: revocation is a CPU-side operation and the
   driver's IOMMU/DMA window still covers the run.

## Is the premise true

Yes, checked rather than argued. Three capabilities over overlapping physical memory exist in the
tree's own display wiring today (`kernel/src/user/display_service.rs`, where `surface = dma +
FRAME_SIZE`):

| holder | slot | object |
|---|---|---|
| gpu driver | `DRIVER_SLOT_DMA` | `PageFrame(dma, 312)` |
| painting client | `CLIENT_SLOT_SURFACE` | `PageFrame(dma + 4096, 311)` |
| display terminal | `TERM_SLOT_SURFACE` | `PageFrame(dma + 4096, 311)` |

and a fourth in `compositor_service.rs`, where a capture client holds the same 311-page run
read-only as the compositor's `screen`. **This is deliberate, not a bug**: the driver's capability
has to cover the whole DMA window it registers with the IOMMU (`virtio::register(.., dma,
DMA_PAGE_FRAMES * FRAME_SIZE, ..)`), and the clients' has to start one page in, past the control
page they have no business touching. The overlap is what "the driver scans out the client's
surface" *means* on this path.

**None of the four carries `Rights::GRANT`** (three are `READ | WRITE`, one is `READ`), which is
what makes every option below unreachable today.

## What the tree already does in the analogous case

- **`revoke_device_from_others`** (DECISIONS §41) is the one revocation that is already selective,
  and it is selective by *holder*, not by capability: it spares the invoker and takes everyone
  else. Its doc says out loud that this is "one level of the capability-derivation tree §13
  deferred, and only one".
- **The mapping log carries no capability identity.** `revoke::LogEntry` is `{ phys, va }` and
  nothing else, and §102 explicitly kept it that way ("the revocation table is per-page, not
  per-capability, and that doesn't change"). So question 2 cannot be answered at all without
  changing that record's shape.
- **`delete_page_frame_caps_overlapping`** (added by this same review pass, `kernel/src/sched.rs`)
  already answers question 1 with "overlap" *for reclamation*, because reclamation had no choice:
  the pages go back to an allocator that hands them out again, so a surviving capability is §13's
  use-after-free. That is a different question from `REVOKE`'s, and it is answered differently on
  purpose.

## Prior art

seL4 keeps a full capability-derivation tree and a mapping database in which a frame mapping is
associated with the frame capability that made it, so revoking a capability revokes its own
mappings and its derivatives' and nothing else. This kernel deliberately kept neither (§13's
deferral), and §102 chose "fewer, fatter capabilities" for a fixed-size capability table where
seL4's radix tree would let it hold more of them. **Stated from recollection of seL4's design
rather than from a reading of its manual in this pass**, per this project's rule about claims from
memory; the shape of the argument does not depend on the details.

## The options, priced

### A. Leave it, record it (what the tree does now)

`REVOKE` deletes the exact object it was invoked on; the unmap stays space-blind.

**Cost: zero.** **Consequence:** revoking the terminal's 311-page run unmaps those physical pages
out of the *driver's* address space too, under a capability nobody revoked, and leaves the driver's
`PageFrame(dma, 312)` in place naming pages it can immediately re-map. So the revoke is
simultaneously too strong (it reached a space it was not aimed at) and too weak (it left an
overlapping capability holding authority). Neither is a use-after-free, because `REVOKE` reclaims
nothing: a region is spend-only, so the pages are still the region's.

### B. Overlap-scoped `REVOKE`

Make `REVOKE` use `delete_page_frame_caps_overlapping` too, so it deletes every capability whose
run intersects the invoked one.

**Cost: one line**, and it makes `REVOKE` and `DESTROY` agree. **Consequence:** it contradicts
§102's own text, which contemplates two capabilities coexisting over sub-ranges of one region ("if
a future consumer needs it, it can hold two capabilities: `Frame(phys, 401)` and `Frame(phys + 401
* 4096, 74)`"). Under B, revoking either destroys the other. It also lets a holder of a
one-page run delete a 312-page capability by naming any page inside it, which is an authority a
one-page capability should not have.

### C. Capability-scoped revocation

Record which capability made each mapping, and have `REVOKE` unmap only those. This is the answer
to both questions 1 and 2, and it is the seL4 shape.

**Cost, measured rather than asserted.** `LogEntry` grows from 16 bytes to 24 to carry the object
(`phys, va` plus the run base, which identifies the capability, derivatives included, since
`derive` never changes the object). `LOG_ENTRIES` falls from 255 to 170 per log page, so a space
pays about 1.5x the log pages it pays now; `revoke.rs`'s existing
`assert!(size_of::<LogPage>() == FRAME_SIZE)` keeps that honest at compile time. "Or its
derivatives" is free under this encoding and only under it: a narrowed derivative has the same
object, so matching on the object matches the whole family without §13's derivation tree.

**Consequence:** it does not close the device half (question 3), and it is a change to what
`REVOKE` means, which is the syscall surface.

### D. Forbid overlapping runs at mint time

Refuse to mint a run capability overlapping an existing one, so the question cannot arise.

**This loses on the premise, and that is worth recording rather than leaving as an intuition.** The
tree's display path *requires* the overlap: the driver's capability must span the IOMMU window it
registered and the clients' must start one page in. D would mean redesigning that wiring (a
separate control-page capability, a DMA registration decoupled from the frame capability) to buy a
property nothing currently needs. It is a real option only if the overlap is judged to be the
mistake, and the wiring's own reasoning says it is not.

## The device half, which none of the options above touch

`PageFrame::REVOKE` is a CPU-side operation. The gpu driver registers `[dma, dma + 312 * 4096)`
with `virtio::register`, and that window is what the DMA validator and the IOMMU domain check
against; it is not derived from any capability and revoking one does not narrow it. So a
capability-perfect revocation of the surface still leaves the device able to write those pages
until the driver's virtio registration is itself torn down. **Whether `REVOKE` should narrow a DMA
window is a separate question from all of the above**, and it is the one with the most surface: it
would couple two objects (`PageFrame` and `Virtio`) that are deliberately independent today.

## Recommendation

**A now, C when it becomes reachable, and the reachability is the trigger.** This is a
syscall-surface question, so per AGENTS.md it arrives with options rather than a made decision; the
lean is stated as a lean.

A is right today because every option costs something and no option buys anything: nothing can
invoke `REVOKE` on a run. B is cheap and wrong (it contradicts §102 and hands a one-page holder
authority over a 312-page capability). C is the honest answer and costs a log-format change plus a
syscall-semantics decision, which is a lot to spend on an unreachable path.

**The promotion trigger, in §71's shape:** the moment any `page_frame_run_cap` mint site gains
`Rights::GRANT`, this decision must be answered before that change lands. That is one grep
(`git grep -n page_frame_run_cap`) and it is the condition under which A stops being honest and
starts being a hole.

## What is blocked until this is answered

Nothing in flight. Specifically **not** blocked: milestone 142, the scanout, or any current
`PageFrame` work. **Blocked:** granting `GRANT` on any run capability, and therefore any future
design that wants a run to be delegable or revocable by its holder.
