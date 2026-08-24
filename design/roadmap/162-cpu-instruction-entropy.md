# 162. Real hardware entropy on x86_64 and aarch64: RDSEED and RNDRRS

**Status: PARTIAL.** Minted 2026-08-24, from calef asking for architectural parity with milestone
159 (the JH7110's TRNG, riscv64/VisionFive 2 only) after "we now have a customer" reopened the
question DECISIONS §120 declined for want of one. Unlike 159, this is **not hardware-gated**: RDSEED
(x86_64) and RNDRRS (aarch64, ARMv8.5-RNG/FEAT_RNG) are CPU instructions, not board peripherals, and
QEMU's TCG emulation of both is real rather than a paravirtual stand-in (confirmed empirically
2026-08-24: `query-cpu-model-expansion` on `-cpu max` reports `rdrand`/`rdseed` as enabled props on
x86_64; QEMU has modeled ARMv8.5-RNG unconditionally under `-cpu max` since QEMU 7.0, though it is not
independently toggleable, so this milestone confirms the actual instruction executes rather than trusts
the absence of a prop either way). So this milestone can be fully built **and verified** without
physical hardware, unlike 159. See "What was built" below for why the title says `RDSEED`/`RNDRRS`
rather than the more familiar `RDRAND`/`RNDR`.

**Gate: NONE.** The design is a direct extension of milestone 56's existing shape (see below); no
open fork.

## Why this is not "the JH7110 driver on two more architectures"

The JH7110's TRNG is a discrete, memory-mapped SoC peripheral: reaching it needs a capability
(rule 2: a base address, nothing else) the same way virtio-rng needs a `Virtio` capability. RDRAND and
RNDR are neither: both are **unprivileged CPU instructions**, executable directly at any privilege
level with no MMIO, no capability, no device discovery. That is a materially different shape of work,
smaller than either the JH7110 driver or the original virtio-rng backend.

**It is also why this needs care rather than less of it.** `notes/entropy.md` and `user/src/entropy.rs`
both state the principle plainly: entropy access here is a *capability*, not ambient authority ("a
program's dependence on randomness is visible in what it holds"), and that is the whole reason
`entropy` is its own minimal process holding nothing else, rather than a library any program links.
Because RDRAND/RNDR need no capability to execute, **the risk is building this as a `getrandom`-style
direct call any program could make, which would silently reintroduce ambient authority into a system
that deliberately refused it.** The correct shape: these become new backends *inside* `entropy`
(replacing what "reads the device" means, for a process that already holds nothing else and already
speaks the same `entropy_proto` contract to its clients unchanged), never a path a client reaches
around the service.

## What it needs

- **Confirm the instructions actually execute under this project's QEMU invocation** (both runner
  scripts use `-cpu max`), not just that a feature flag is reported. A one-instruction probe that
  either returns bytes or is provably not `UD`/`SIGILL` settles this before anything else is built.
- **A new backend inside `entropy` (`user/src/entropy.rs`), replacing what "reach the device" means**,
  for each architecture, gated the way this tree already gates architecture-specific code (rule 1)
  wherever userspace's existing arch-specific `asm!` already lives; check the convention before
  inventing one, and do not put raw `asm!` directly in an architecture-neutral file.
- **Pass the bytes through unmodified, matching the existing backend's own discipline**: "No pool,
  no whitening, no mixing, no DRBG... these are the device's bytes" (`user/src/entropy.rs`). Whether
  RDRAND/RNDR need this at all is worth checking against the architecture manuals directly: both
  Intel's and Arm's specifications for these instructions describe on-die conditioning as part of the
  instruction's own contract (unlike a raw TRNG register), which may mean the JH7110 driver's software
  health-test question (repetition-count/adaptive-proportion) does not apply here the same way. Check
  the manuals, do not assume either answer.
- **Honor the retry/failure contract these instructions already define.** Both RDRAND and RNDR can
  report "no data this cycle" as part of their normal operation (a carry flag / Z flag check, not an
  exception), with documented bounded-retry guidance from both vendors. Follow it rather than
  retrying forever or treating one failure as `entropy_proto::NO_ENTROPY` immediately.

## What this does not decide

Same as 159: this does not reopen or grant DECISIONS §120's interactive-boot stopgap question on its
own. calef has separately noted the customer condition §120 named is now met; revisiting §120 is his
to do, not a consequence of this milestone landing.

## What it unblocks

A second, non-virtio-dependent entropy source on the two architectures that don't yet have real
hardware in front of them the way milestone 159 gives riscv64, and, unlike 159, one this tree can
actually prove works today, in CI, on every run, rather than only on a board on calef's desk.

## What was built (2026-08-24)

**`RDSEED`, not `RDRAND`; `RNDRRS`, not `RNDR`.** Checked against the actual specifications rather
than assumed: `RDRAND`/`RNDR` are DRBG-buffered (Intel's DRNG Software Implementation Guide
classifies `RDRAND` as SP800-90C RBG2(P), a hardware `CTR_DRBG` seeded from the entropy source and
serving up to 511 draws per reseed; ARM's `RNDR` is the same shape). `entropy`'s whole discipline is
"no pool, no whitening, no mixing, **no DRBG**"; taking the buffered instruction would silently
reintroduce, in hardware, the exact primitive this service already refuses in software. `RDSEED` and
`RNDRRS` instead draw straight from the conditioned entropy source with no DRBG in the path, one
sample per instruction, which is what keeps "these are the device's bytes" true. The cost is real: both
are rate-limited by the physical source and can run dry under load in a way the buffered instructions
do not; a caller that exhausts the retry budget gets `entropy_proto::NO_ENTROPY`, same as a dry
virtio device, rather than a silent fallback to the DRBG-backed sibling.

**On-die conditioning, checked rather than assumed.** Both Intel's and Arm's specifications describe
an SP800-90B-shaped noise-source-plus-conditioning-function model as part of the instruction's own
architectural contract, not something the OS adds. So the bytes are passed through unmodified, same
as the virtio backend; the JH7110 driver's software health-test question (milestone 159) does not
apply here.

**aarch64: fully proven, but not under the suite's default CPU.** `crates/machine_discovery::aarch64::Isa`
now decodes `ID_AA64ISAR0_EL1.RNDR` (host-tested: a real part without `FEAT_RNG`, the defined
"implemented" encoding, and every reserved encoding decoded conservatively as absent).
`kernel/src/arch/aarch64/isa.rs` reads it at boot. `kernel/src/user/entropy_service.rs` gained a third
`Bus` variant, `Instruction` (not really a bus; named alongside `Mmio`/`Pci` anyway because everything
the type already does, picking which source to wire, applies equally), spawning `entropy` in a new
mode that needs no `Virtio` capability, no DMA page, no `Irq`: two capability slots instead of four.
`user/src/entropy.rs` gained the `RNDRRS` backend itself (`MRS` on `S3_3_C2_C4_1`, checking
`PSTATE.NZCV` for the architected success/failure signal, the same idiom Linux's own
`arch/arm64/include/asm/archrandom.h` uses) and a real serve loop for it.

**Proven end to end under QEMU**: `kernel::user::entropy_tests::a_client_obtains_unpredictable_bytes_from_rndrrs_with_no_device_at_all`
spawns `entropy` in instruction mode and gets real, unpredictable `RNDRRS` bytes back over the request
endpoint, 64 words across the refill boundary, none repeated. **But only under `--cpu neoverse-n2`,
not the suite's default (`cortex-a72`, ARMv8.0-A, predates `FEAT_RNG`) and not `--cpu max` either**:
checked directly (2026-08-24, QEMU 11.0.2), `max` does carry `FEAT_RNG` (confirmed by reading
`ID_AA64ISAR0_EL1` at boot), but this kernel refuses to boot on it at all, for an unrelated reason
("no 4 KiB stage-1 granule (`ID_AA64MMFR0_EL1.TGran4`)"), a QEMU CPU-model quirk, not an entropy
question. `neoverse-n2` (Armv9.0-A) has both. The test itself skips cleanly under the default CPU,
the same way the virtio tests already skip when `NIFE_RNG` is unset; run it for real with
`script/test --arch aarch64 --cpu neoverse-n2`.

**x86_64: the instruction is proven; the service is not, because there is nowhere to run it yet.**
`kernel/src/arch/x86_64/isa.rs` checks `CPUID` leaf 7 `EBX` bit 18 and gained `draw_rdseed`, proven
in the boot tour (a kernel-side probe, ring 0, retrying per Intel's own DRNG guide). But
`kernel/src/user/entropy_service.rs::instruction_backend_available` returns `false` unconditionally
on x86_64: milestone 161's ring 3 (item 3) does not exist yet, so there is no way to schedule a
userspace `entropy` process onto this architecture at all. The userspace `RDSEED` backend
(`user/src/entropy.rs`) is written and builds, but is genuinely unreachable until ring 3 lands; this
is why the milestone's status is `PARTIAL` rather than `BUILT`, honestly, not `NOT-STARTED` (the
instruction-level proof is real and done). The next x86_64 step is free once ring 3 exists: flip
`instruction_backend_available`'s x86_64 arm to check `arch::isa::get().rdseed`, matching the aarch64
arm exactly.

**riscv64: correctly excluded.** Neither `RDSEED` nor `RNDR`/`RNDRRS` exists on this ISA; milestone
159's JH7110 TRNG is the real hardware source there, through its own driver, not through this file.
