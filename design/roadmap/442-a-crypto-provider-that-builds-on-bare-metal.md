# 442. A crypto provider `rustls` can use on all three bare-metal targets

**Status: NOT-STARTED.** Minted 2026-09-19 by the maintainer, from calef's ruling in [DECISIONS
§196](../decisions/196-nife-carries-tls-and-builds-the-provider.md) (take `rustls`, build the
provider). *(Number provisional until the merge queue lands it.)*

**Gate: NONE.** The dependency decision is made; what is left is making the code build and proving
it, which needs nobody.

## In brief

`rustls` itself is seven crates and builds. **No crypto provider under it builds for
`aarch64-unknown-none-softfloat`, `riscv64gc-unknown-none-elf` or `x86_64-unknown-none` today**, and
until one does, §196's ruling is a direction rather than a transport. The probe behind §196 found
two failure classes, and they want different work:

- **`getrandom` has no backend on a bare target** (`ring` 0.17.14, `rustls-rustcrypto` 0.0.2-alpha).
  This is the class `entropy_backend` exists for: nife has an entropy service and a randomness
  protocol, so the backend is ours to supply rather than something upstream owes us.
- **SIMD paths fail on soft-float x86_64** (`sha2`, `polyval`, met through `embedded-tls`). The
  probe ran against stock bare targets on the stable host toolchain, not against nife's own target
  specifications and pinned nightly, and says so; the first step here is to re-measure properly
  before choosing a provider.

## What this milestone owes

1. **A re-measurement that is ours**: every candidate provider built against nife's target
   specifications, on the pinned nightly, with `entropy_backend` wired in. The §196 table is a
   stable-host probe and is explicitly not a finding about our targets.
2. **One provider that builds and passes its own test vectors on all three architectures**, whether
   that is an upstream crate with a `getrandom` backend supplied, a narrowed fork, or code this tree
   writes. §46's rule decides the shape: write it if it is on the verification path, vendor it if
   correctness is won by exposure rather than by reading the spec, which is what it said about
   crypto in the first place.
3. **A client that speaks TLS 1.3 to one peer**, holding that peer's root or pinned key as a
   capability (§196 clause 4), not a system trust store.
4. **An honest `BUGS` section** about what is not covered: certificate expiry with no wall clock
   that a stranger's machine can trust, revocation, and what happens when the one pinned key
   rotates.

## What it unblocks

Rung 3c of the trivial install (§157): a package fetched by host name from a source somebody else
operates. Rung 3a does not wait for it, because §195 makes a recipe's digest decide what may run and
a digest is checkable over any transport.

## BUGS

- **Nothing here measures the cost.** A TLS handshake on a board with no hardware crypto may be slow
  enough to matter to the install story, and no number exists.
- **The pinned-key model has no rotation story**, which is the part every real deployment eventually
  needs and this block does not attempt.

## Index row

`rustls` is seven crates and builds; every crypto provider under it fails on our three bare-metal
targets, at `getrandom` for want of a backend nife can supply, or in SIMD paths on soft-float
x86_64. Minted from §196, which ruled that nife carries TLS and that the provider is work rather
than a purchase. Blocks rung 3c of the trivial install and nothing before it.
