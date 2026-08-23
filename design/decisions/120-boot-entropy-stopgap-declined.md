# 120. A QEMU-only virtio-rng stopgap for the interactive boot is declined for now

**Status: DECIDED.** calef, 2026-08-23, on milestone 49's boot-wiring fork: *"Mint the TRNG
milestone, defer the stopgap for now."*

## The question

Milestone 49's boot-wiring lane traced "not wired into the interactive boot" down to a real
blocker: `login` needs `credentialer`, which refuses to start without the entropy service
(DECISIONS §42, no silent degradation), which needs a `Virtio` capability onto a virtio-rng
device. Neither aarch64's `spawn_init` nor riscv64's `riscv_shell_boot` grants one, and the
interactive boot's QEMU invocation does not attach the device either -- a deliberate, existing
minimal-device-surface choice (`NIFE_RNG`, like `NIFE_GPU`/`NIFE_KBD`/`NIFE_NVME`, is test-leg
only). The fork the lane surfaced rather than decided: add virtio-rng to the interactive boot now,
as a QEMU-only stopgap, or wait.

## The decision

**Declined for now.** No customer needs interactive login working before real hardware entropy is
sorted. Same shape as `std::thread::spawn` (§105), hard links (§110), state handoff (§116), and
`OutOfMemory`'s cause collapse (§119): revisit when a customer needs it, not before.

## Why, beyond just deferring

**This was never really "add entropy or not," it was two separable questions wearing one fork.**
Checked before accepting the lane's framing: the JH7110's TRNG is already named as the real
hardware answer in two places (`notes/entropy.md`, milestone 56's own doc), verified nowhere,
tracked by no milestone -- the same shape of gap milestones 53 (board network/storage) and 157
(board display) already carry: virtio only exists in QEMU, real silicon needs its own driver.
Milestone 159, a real hardware entropy source (the JH7110's TRNG, minted alongside this decision),
tracks the real answer. This decision covers only
the second, separable question: whether to grant a QEMU-only stand-in *before* that real answer
lands.

**The lane's own cost accounting holds up and is the real reason to wait, not just "no customer."**
Granting virtio-rng in the interactive QEMU boot is cheap by itself (one flag, matching the three
that already exist for test legs), but it is coupled to two expensive things: a new permanent
kernel-to-init capability grant on a `BootEndowment` cspace already documented as one slot from the
wall at peak (restructuring which capabilities init holds simultaneously, not adding a field), and
a claim about what hardware the interactive boot represents that milestone 55's real Pi-class
target contradicts if answered the QEMU-only way. Per the *move fast on what can be undone* tenet,
a new capability two programs (kernel and init) must agree on is exactly the expensive category,
and there is no customer forcing the question yet.

## What this does not decide

Milestone 159's real-hardware TRNG driver is not blocked by this decision either way, and building
it does not require revisiting this fork -- the two were only coupled by both routing through
`entropy_service`, not by any shared mechanism.

## What it unblocks

Nothing was gated on this; it closes milestone 49's open fork so its BUGS section can record
DECLINED with a reason rather than sitting as an unanswered "what I need from you."
