# The entropy service reports READY while holding zeros

**Status: PROPOSED 2026-09-04.** Found on radon's first TRNG bring-up, by the line that was supposed
to be reporting a different failure.

**Gate: NONE.** The check is a few lines in `user/src/entropy.rs`, and the contract it has to meet is
already written in `crates/entropy_proto`.

**What the work is.** `entropy_proto::READY` is documented as the word the service sends *"once the
device is up **and its first bytes are in hand**"*. On radon it was sent with 32 bytes of zeros in
hand, from a TRNG whose clock is gated (milestone 220), and nothing refused.

Two paragraphs below that constant, the same file argues the opposite case for `NO_ENTROPY`:

> a caller that cannot be given randomness must find out, because the alternative is the exact
> silent-degradation failure

**A service that reports ready on a dead device is that failure.** The boot tour caught it only
because it prints the draws beside the report and computes `first-all-zero`; a client that trusted
`READY` would have consumed zeros believing them random, which is the worst failure mode a
randomness source has and the one that does not announce itself.

**What it is not.** It is not milestone 220's, and it must not be left for 220 to fix by accident.
Once the clock works the symptom disappears and the defect stays: the service would still send
`READY` on any future dead device, and the next one may be on a machine with no boot tour watching.

**The shape of the fix, and the part that needs judgement.** Refusing an all-zero first draw is the
obvious check and is not sufficient on its own: an all-zero buffer is legitimate output with
probability 2^-256, so a hard refusal is a correctness claim about a random variable. The honest
version is probably that the service treats an all-zero first bufferful as a bring-up failure and
says so in its report word (`0xDEAD_0000_0000_0000 | step` already exists for exactly this), while
recording in its own `BUGS` that a vanishingly improbable true zero would be misread as a dead
device. That trade is the right way round: a false "device is dead" costs a boot, a false "device is
alive" costs every secret derived from it.

**Related, and worth deciding together:** `design/decisions/137-trng-health-tests.md` is `PROPOSED`
and covers whether this device's output gets a health test at all. This is narrower than 137 (it is
about the readiness handshake rather than continuous testing) and could ship without it, but whoever
takes either should read the other.
