# Whether fetching a package needs TLS, and which one if it does

**Status: PROPOSED 2026-09-19.** Written by milestone 198's rungs lane
(`milestone/198-rungs-to-a-trivial-install`) for rung 3 of the trivial install DECISIONS §157
defines. §157 lists TLS as missing and as a dependency decision. The existing
`a-tls-stack-and-which-one.md` (2026-09-05) removed milestone 198 from its consumers and left
198's transport "198's to decide"; this is that decision, written so it can be ruled on.

**Gate: DECISION.** Taking a TLS stack is a dependency (DECISIONS §46, whose 2026-07-31 amendment
makes crypto "an ordinary dependency, pinned in `Cargo.lock`, gated by `deny.toml`"), and the
transport a package client speaks is something it and a repository agree on. **Options, no
winner.**

## What the transport has to provide, split into its parts

A package fetched over the internet needs **integrity** (the bytes are the ones meant), and may want
**confidentiality** (nobody on the path learns what was installed) and **freshness** (nobody replays
an old, vulnerable package list). TLS gives the first two for the connection; it does not give the
first for the *content*, since a compromised mirror serves bad bytes over perfect TLS.

**Integrity of content is the trust fork's job, whatever the transport**
(`what-vouches-for-a-package-the-image-did-not-carry.md`):

| Trust ruling | Where integrity comes from | Does it need TLS? |
|---|---|---|
| **T1** (the image, always) | The kernel's compiled-in table | No network install at all; the question does not arise |
| **T2** (a publisher's signature) | A signature over the package metadata, checked against a key in the measured image | **Not for integrity.** The Debian and Nix shape: any mirror, plain HTTP |
| **T3** (the owner) | A digest the owner records | **Not for integrity**, provided the owner got the digest somewhere trustworthy. On §157's path they did: **the web page**, read in their own browser over their browser's TLS |

So **under T2 or T3, TLS is not required for integrity.** What it would still buy is confidentiality
and a defence against a network that blocks or tampers with plain HTTP, and what plain HTTP needs
instead of TLS for freshness is an expiry in signed metadata (the Update Framework's timestamp role;
Debian's `Valid-Until`, recalled), which the format fork's metadata rows do not yet carry.

## A fact the choice depends on: where the packages are hosted

Measured on 2026-09-19 with `curl -sI`:

```
http://github.com/crickertech/nife/releases   -> 301 Location: https://github.com/crickertech/nife/releases
http://crates.io/                            -> 301
http://deb.debian.org/debian/dists/stable/Release -> 200 OK
```

**A repository on GitHub cannot be fetched over plain HTTP**: it redirects to HTTPS. Plain HTTP
means a host that serves it, which is a self-hosted server or a CDN configured for it. So this fork
and the hosting question (calef's, under §157's step 1) are one decision seen from two sides.

## The options

| | Shape | Measured on 2026-09-19 | Costs |
|---|---|---|---|
| **P1. Plain HTTP, integrity from T2 or T3** | An HTTP/1.1 client over the existing socket contract; every byte checked against the digest before use | **No new crypto for T3**: SHA-256 is already hand-written in `crates/measured_boot` and tested against FIPS 180-4 vectors. **T2 adds a signature verifier**: `ed25519-dalek` 2.x with default features off is **16 crates** and builds `no_std` for `aarch64-unknown-none-softfloat` and `riscv64imac-unknown-none-elf`; for `x86_64-unknown-none` it failed compiling `sha2` | A host that serves HTTP. No confidentiality. Freshness needs expiring metadata |
| **P2. HTTPS with `rustls`** | `rustls` 0.23.45 for the protocol, plus a crypto provider | **`rustls` itself**, default features off plus `tls12`: **7 crates**, all Rust (`once_cell`, `rustls-pki-types`, `rustls-webpki`, `subtle`, `untrusted`, `zeroize`), and it **builds `no_std` for all three bare-metal targets**. It does nothing without a provider: | A provider, and a trust store (below) |
| &nbsp;&nbsp;P2a. provider `ring` 0.17.14 | C and assembly, built by `cc` | Failed on all three bare targets at `getrandom` 0.2 (no backend), and `notes/crates-io-on-nife.md` records it **failing class C (C sources)** on nife's own targets even with a custom `getrandom` | C in the shipping graph |
| &nbsp;&nbsp;P2b. provider `aws-lc-rs` 1.18.1 | C, default feature `aws-lc-sys` | **Not probed** by this lane | C and a CMake-class build |
| &nbsp;&nbsp;P2c. provider `rustls-rustcrypto` | Pure Rust | Version **0.0.2-alpha**; a **97-crate** graph with `alloc` and `tls12`; failed on all three bare targets at `getrandom` 0.2 (via `rand_core` 0.6) | An alpha provider; this tree's `entropy_backend` answers `getrandom` 0.3 and 0.4 only |
| **P3. HTTPS with `embedded-tls`** 0.19.0 | A `no_std` TLS 1.3 client, RustCrypto underneath | **58 crates** with default features off; builds for aarch64 and riscv64 bare targets, failed on `x86_64-unknown-none` in `sha2` and `polyval`. Certificate verification is optional (`webpki`, `rustpki` features) | TLS 1.3 only; a smaller project with less exposure, which is what §46 says crypto is bought with |
| **P4. Write TLS** | | | **Refused by §46** in so many words ("take it, do not write it"); listed so the refusal is visible |
| **P5. OpenSSL, confined** | The existing TLS proposal's third answer | Not probed here | Priced nowhere yet; that proposal's `BUGS` says so |

**The probe's limits, stated so the table is not over-read.** It ran on the stable host toolchain
against the stock bare-metal targets, not against nife's own target specifications and the pinned
nightly. The `x86_64-unknown-none` failures in `sha2` and `polyval` are the kind a soft-float target
produces for crates with SIMD paths, which is a reason to expect a different answer on nife's x86_64
target rather than a finding about it; that was not checked. The `getrandom` failures are the class
`entropy_backend` exists to answer, for the versions it answers.

## The trust store, which is the part of HTTPS nobody prices

HTTPS needs roots to verify against. The existing TLS proposal already found the circularity: a
trust store has to be updated independently of the system, and the update mechanism **is** the
package manager. Shipping roots inside the image freezes them until the next image. It also proposed
the capability-shaped answer (a client granted exactly the roots its one peer chains to). Under P2
or P3 for packages, that is the shape to use: **the package client holds one root, or one pinned
key, for its one repository**, not a system store.

## What the tree does in the analogous case

Nothing crosses the internet today. The one analogous transfer, the kernel and archive reaching the
kernel, is integrity by digest with no transport security at all (`measured_boot`, milestone 104),
which is P1's shape at boot.

## Reversibility, and who acts on it

**The day a stranger's installed system fetches from a repository, the transport is fixed for that
system** until it is reimaged or updates its client, because the client is what reads the answer.
Before that day every option is reversible. A dependency taken under P2 or P3 is §46's expensive
kind.

## The §92 test

P1 is cheapest under T3 by a wide margin (no new crypto at all). If P1 is chosen *because* it is
cheaper, that is effort and should be said. The non-effort case for P1 is Debian's: integrity of
content rather than of the channel, one key instead of every certificate authority, and any mirror
can serve it. The non-effort case for P2 or P3 is confidentiality and working behind networks that
interfere with plain HTTP.

## What each ruling blocks

Rung 3's internet half (`design/roadmap/198-package-manager.md`, "Rescoped 2026-09-19") waits on
this ruling and on the trust ruling. **Rung 3's LAN half does not**: a package fetched from a host on
the same network, over plain HTTP, verified by digest, needs neither, and that is the first thing to
build.

## BUGS

- **No HTTP client exists in the tree** (`git grep` for an HTTP request line in `components/` and
  `crates/` finds none). Whichever option is chosen, a small client is part of rung 3; `std::net`'s
  `TcpStream` is bound (milestones 27 and 64), so it can be a `std` program.
- **DNS is not here**: `a-name-resolver-and-who-holds-it.md` owns it, and `smoltcp`'s `socket-dns`
  feature is still off in `components/Cargo.toml`.
- **The crate counts are graph sizes, not audit burdens**, and were taken with `cargo tree` on one
  day; they move with every release.
