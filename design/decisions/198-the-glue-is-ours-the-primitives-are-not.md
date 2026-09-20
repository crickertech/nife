# 198. The glue is ours, the primitives are not

**Status: DECIDED.** calef, 2026-09-20, on the lane report for milestone 442 (a crypto provider
`rustls` can use on all three bare-metal targets): *"rustls-rustcrypto doesn't seem like a high
quality dependency."* *(Section number provisional until the merge queue lands it.)*

**The ruling.** nife assembles its own `rustls` `CryptoProvider` over published RustCrypto
primitives. The primitives are taken; the glue that wires them into `rustls` is written here.
`rustls-rustcrypto`, the crate that would have supplied that glue, is refused.

DECISIONS §196 (nife carries TLS: `rustls` for the protocol) said the provider was work rather
than a dependency and left which crate open. This is that answer.

## Why the crate was refused, with the numbers rather than an impression

Read from crates.io on 2026-09-20, not recalled:

- **`0.0.2-alpha`, published 2024-04-24**, seventeen months before the refusal. Three versions have
  ever existed and two are yanked, so the live release is both the newest and an alpha.
- **It pins superseded majors**: `sha2` 0.10, `aes-gcm` 0.10, `p256` 0.13, against current releases
  of 0.11.0, 0.11.1 and 0.14.0, all shipped during 2026. Taking it fixes this tree to the dependency
  choices of a crate nobody is updating.
- **73 crates** in its graph with `rustls`, against **67** for the same coverage written here and
  **53** for the provider as actually built. The full glue graph is a strict *subset* of the alpha's.

**73 against 67 is not an argument, and this section will not pretend it is one.** The case is
maintenance, and it is the one place DECISIONS §46 (thin primitives or whole subsystems; we write
everything in between) points hardest. §46 says to vendor what is won by *exposure* rather than by
reading a specification, and an alpha abandoned for seventeen months has had the least exposure of
anything in the graph it assembles. The glue was the weakest link in a chain of otherwise well-worn primitives.

**What it is not refused for.** Its licence is Apache-2.0 OR MIT, the pair this tree publishes under,
and it built and ran correctly on all three architectures, which the lane measured rather than
assumed. The refusal is about who maintains the glue, not about whether it works today.

## Where the line falls, stated piece by piece

§46's line is not self-applying, so the lane drew it out loud:

- **Ciphers and curve arithmetic are taken.** `aes-gcm` and `chacha20poly1305` handle every byte on
  the wire and carry NCC Group's 2020 audit in their own READMEs. That is exposure, and it is not
  something this tree can manufacture by reading RFCs carefully.
- **The provider is written.** Suite tables, key provider, entropy source, the `rustls` traits: this
  is plumbing whose correctness is won by reading `rustls`'s own contract, which is §46's first case.
- **RSA PKCS#1 v1.5 verification is refused despite being pure public-key arithmetic**, because
  Bleichenbacher and BERserk are both spec-reading failures in exactly that code. A thing that looks
  like arithmetic is not therefore on the write-it side of the line.

**The honest caveat**: `p256`'s own README says its curve arithmetic *"has never been independently
audited"*, so the audit argument covers the AEADs and not the whole graph. Eight crates in the graph
runtime-detect SIMD; five need an explicit force-soft flag on nife's targets, `curve25519-dalek`
needs a backend selector, and on `x86_64-unknown-nife` a crate that detects AVX2 executes it in ring
3 and dies with `vector 6 (invalid opcode)` before printing anything. That hazard is a property of
the targets rather than of this ruling, and `notes/cryptography-provider.md` carries it.

## Still calef's

**Whether to take `rsa`.** Without it the provider verifies `github.com` and not the hosts that serve
the bytes: `objects.githubusercontent.com` presents RSA 2048 and `ghcr.io` RSA 4096, both measured.
With it, `deny.toml` needs its first-ever `ignore` entry, for RUSTSEC-2023-0071. The argument and
what is not known are in
`design/roadmap/proposals/whether-to-take-rsa-and-what-it-costs-either-way.md`. Nothing waits on it,
because under DECISIONS §195 (a reviewed recipe vouches for a package) a recipe's digest decides what
may run and TLS is not load-bearing for integrity.
