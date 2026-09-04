# Nothing in QEMU can hand the entropy service a bufferful of zeros

**Status: PROPOSED 2026-09-04.** Written by the `maintainer/ready-on-a-dead-device` lane, which
fixed the defect that needed exactly this test and could not write it.

**Gate: NONE.** QEMU already has the device (`-object rng-random,filename=/dev/zero`), the service
already has the check, and the only new thinking is how a wiring names *which* virtio-rng it wants.

**What the work is.** `entropy_proto::readiness` decides the readiness word from the first bufferful,
and refuses `READY` when every byte of it is zero (milestone 159's block, "Fixed 2026-09-04"). That
decision is host-tested in `crates/entropy_proto`, and the drivers' three calls to it are not tested
anywhere: no machine this repository boots can produce a device that answers with zeros. The one that
did was radon, whose TRNG has a gated clock, and it is not a machine CI can run.

**The shape.** QEMU can make one: `-object rng-random,filename=/dev/zero,id=zeros` behind a second
`virtio-rng-device`. What is missing is a way for a test to reach *that* device rather than the real
one, because `entropy_service::Bus` picks a transport and the scan takes the first virtio-rng it
finds on it. So this is a runner line plus a way to name which device a wiring should take, and the
end of it is a test asserting that the service reports `bringup_failure(STEP_FIRST_ALL_ZERO)` and
then answers `NO_ENTROPY` to every request.

**Why it was not done in the lane that wanted it.** It lands in the test-wiring hotspot AGENTS.md
names (`kernel/src/user/tests.rs`, the QEMU runners, `entropy_service.rs`), and a permanently-dead
entropy device sitting on a bus every other entropy test scans is a way to make those tests flaky
for a reason unrelated to what they assert. That is a design question about how a wiring names a
device, and it is larger than the defect it would have covered.

**What it would also unlock.** The same lever tests the *dry* device path (`filename=/dev/null`, or a
`rng-random` that never answers), which is likewise only ever exercised by a device nobody has.
