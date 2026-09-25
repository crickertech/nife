# A soak kernel cannot reach xenon's stick, because the loader cannot vouch for it

**Status: PROPOSED 2026-09-25.** Raised by milestone 593 (a wedged kernel resets itself)'s lane,
which needed a `watchdog_soak_test` image for xenon's first watchdog boot and could not build one.

**Gate: DECISION.** The fix touches the measured-boot seal, which is a security property. The
options below differ in what that seal promises, and that is calef's call.

## What happens

`cargo xtask uefi-image` builds the kernel with no features, so it cannot produce a soak image at
all. Adding `--features` to it, which the lane tried, fails in `uefi_loader/build.rs`:

```
NOT SEALED: target/x86_64-unknown-none/debug/kernel does not vouch for target/initrd-x86_64.img.
  entry `progenitor` hashes to 1b2e40..., which is not in the kernel image's trust root
```

It fails the same way for plain `soak_test`, so this is not the watchdog's doing. Measured on
patagonia on 2026-09-25, with a default image built and sealed seconds earlier from the same
archive.

## Why, as far as the lane read it

`soak::run()` never returns, and every soak build calls it before the boot reaches the progenitor.
So `trust::require` is unreachable, the compiler drops `TRUST_ROOT`, and the digests the seal check
looks for are not in the image. The check is right that this kernel vouches for nothing. It is
also right that this kernel never runs anything it would have to vouch for. The inference is the
lane's and was not confirmed with a symbol dump.

## What the tree does in the analogous case

radon gets a soak through `script/board-image --soak`, which builds the archive first and the
kernel second, the order the seal wants. Its header records why: a soak kernel built against a stale
archive met `MEASURED BOOT REFUSED` at the bench. So on radon the seal is meant to hold for soak
builds too. Whether `script/card-check` passes on a soak card today was not checked by this lane. If
it fails, radon has the same gap and the fix here should cover both.

## Options

1. Keep `TRUST_ROOT` alive in soak builds, for example with `core::hint::black_box`. The pair then
   seals, and the seal still means "this kernel would accept exactly this archive". Smallest change.
2. Let `uefi_loader` accept a kernel that measures nothing when a soak feature built it. This
   weakens the seal's meaning for one class of image, so it needs a written reason.
3. Give xenon a soak path that does not go through the loader, the way radon has `script/board-image
   --soak`. The most work, and it duplicates what the loader already does.

Recommendation: option 1. It changes no promise and costs a line. This recommendation would stand
if all three cost the same.

## Cost and reversibility

Option 1 is a one-line change in `kernel/src/trust.rs` or `kernel/src/soak.rs` and one rebuilt
image to prove it. Options 1 and 3 are cheap to undo, because nothing outside the tree has acted on
a soak image yet. Option 2 is harder to take back once a card or stick built under it is on a desk.

## What is blocked

Milestone 225 (run the soak on radon, argon and xenon)'s xenon leg, and milestone 593's first bench
step on xenon. The lane's `--features` change to `uefi-image` is saved rather than landed, because
without a fix here it produces nothing that boots.
