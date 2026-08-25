#![no_std]
//! **The wire contract between a client and the login service** (milestone 49's login half).
//!
//! Unix login authenticates and then mutates a global identity field. This contract is the other
//! shape: a client presents an identity and a secret, and on success the service delegates a fresh
//! **capability set** back over the same channel rather than changing anything ambient. See
//! `user/src/login.rs` for the service and notes/login.md for the design.
//!
//! # The exchange
//!
//! Not a `CALL`: a one-shot reply capability carries two words and nothing else (`abi::reply`), and
//! a successful login has to hand back capabilities, which only `abi::rendezvous::SEND_CAP` can do.
//! So this is two persistent endpoints and a fixed message order, the same shape
//! `grant_plan::spawnproto` already uses for exactly this reason (a shell's spawn request that may
//! carry a delegated budget):
//!
//! ```text
//!   client --place(), send(REQUEST, w0, 0, 0)-------------------------> login
//!   client <--------------------------- recv(RESULT) -> OK or DENIED-- login
//!   client <---- RECV_CAP(RESULT) x 4, only after OK -----------------  login
//! ```
//!
//! On [`OK`], login sends exactly four capabilities over the result endpoint, in this order, and a
//! client that does not read all four leaves its own protocol out of step for the *next* login
//! (there is no other client on this endpoint in the slice this contract ships with; see this
//! service's BUGS):
//!
//! 1. the **directory** capability: a freshly built `fs_subtree_caretaker`'s endpoint, `WRITE`;
//! 2. the **filesystem's shared page**, a `PageFrame`, `READ | WRITE`: the client maps it itself
//!    (`user_rt::map_page_frame`) at whatever address it chooses, and uses it for both the request it
//!    stages to the directory endpoint and the caretaker's own hop to the file service, which is
//!    sound for the reason `crates/system_initializer` gives (`fs_subtree_caretaker` and its client
//!    share one frame because every request on both hops is a blocking `CALL`);
//! 3. the **budget**: a `MemoryRegion`, `WRITE | GRANT`, freshly split so the client's memory is
//!    genuinely its own and it may in turn split, spend, or hand pieces of it on. `WRITE` is also
//!    what `MemoryRegion::DESTROY` needs, so this capability doubles as its own reclaim: a client that
//!    calls `DESTROY` on it, alongside the fourth capability below, gives back everything a session
//!    spent rather than only the caretaker's half;
//! 4. the **logout ticket**: a `MemoryRegion`, `WRITE` only, the exact region the directory capability's
//!    caretaker was built from (`user/src/login.rs`'s `mint`, see that program's module docs,
//!    "Reclaiming a session"). It has nothing left to `SPLIT` or `RETYPE` (its whole budget went
//!    into building the caretaker), so its only remaining use is `invoke(cap, abi::memory_region::DESTROY,
//!    0, 0, 0)`, which reclaims the caretaker's TCB, address space and endpoint and returns the
//!    pages to `login`'s own construction budget. **A client should retry `DESTROY` a bounded few
//!    times on refusal rather than treat one attempt as final**: the caretaker can be transiently
//!    mid-request to the file service when a logout arrives, which refuses the very first `DESTROY`
//!    (the same shape `crates/system_initializer::reclaim` already retries for a directory grant's
//!    own caretaker); it never refuses permanently, because the caretaker's own client-facing
//!    endpoint is retyped from this same region, so its steady state (parked in `recv` between
//!    requests) is always reclaimable, never the permanently-blocked case
//!    `notes/hung-component.md` documents as unfixable. Logging out is optional: a client that never
//!    calls `DESTROY` costs `login` exactly what it always cost (see that program's BUGS on
//!    `CONSTRUCTION_UT` exhaustion), this ticket just makes not costing it possible.
//!
//! **A full logout destroys capability 3 before capability 4, and the order is load-bearing.**
//! `mint()` splits the fourth capability's region from `login`'s own `CONSTRUCTION_UT` first and the
//! third capability's budget second, so the budget sits above the region in `CONSTRUCTION_UT`'s
//! watermark. `crates/regions`' own reclaim only returns a freed child's pages to a reusable state
//! when it is the *top* of its parent's watermark (LIFO, the same rule §16's object revocation and
//! `job_undertaker`'s pool already live under, and DECISIONS §92 already named for a caretaker's own
//! region); destroying the region first, while the budget above it is still alive, still reclaims the
//! caretaker's TCB, address space and endpoint (`DESTROY` still returns success), but leaves its
//! pages a stranded hole that does not come back to `login`'s reusable capacity until
//! `CONSTRUCTION_UT` itself is destroyed. Destroy the budget first, then the region, and both spans
//! return cleanly. **This is a property of this specific pair, not a general promise**: it holds
//! regardless of what any other client does, because nothing else is ever split from
//! `CONSTRUCTION_UT` *between* one login's own two capabilities (`mint()` builds both, back to back,
//! before either is delegated); it does not extend to reclaiming *two different logins'* memory out
//! of the order they were minted in, which needs the same LIFO discipline this tree already accepts
//! elsewhere.
//!
//! On [`DENIED`] or [`MALFORMED`], nothing follows: the client holds exactly what it held before it
//! asked.
//!
//! # The request page
//!
//! Identical in shape to [`credential_proto`]'s: an identity and a secret, because this service's whole
//! first act is relaying the presented pair to the credential service's own `VERIFY` unchanged. Its
//! `place`/`read`/`req`/`op` do not encode anything specific to that service's own semantics (`op`
//! is a caller-supplied word), so this contract reuses them rather than defining a second copy of
//! the same layout and risking the two drifting the way `credentialer.rs`'s own compile-time
//! assertions exist to catch for `cred`/`credential_proto`.
//!
//! Name: ratified 2026-08-23 (calef, a kernel-dependency crate naming review). Minted 2026-08-22
//! for milestone 49, following the tree's existing `<subject>_proto` pattern (`credential_proto`,
//! `clock_proto`, `entropy_proto`).

pub use credential_proto::{MAX_IDENTITY, MAX_SECRET, PAGE, op, place, read, wipe};

/// The one request opcode this contract defines. There is only one verb (authenticate), so unlike
/// `credential_proto` (verify, provision) there is nothing to distinguish it from; it exists so a reader
/// of a captured request word can tell this contract's traffic from any other sharing the encoding.
/// Build a request word with `place(page, identity, secret, LOGIN)`, which returns it already
/// carrying this opcode and the two lengths.
pub const LOGIN: u64 = 1;

/// **Authenticated.** Exactly three capabilities follow on the result endpoint; see the module docs
/// for the order.
pub const OK: u64 = 1;

/// **Refused.** The identity is unknown, the secret is wrong, or the service could not mint a
/// capability set for an otherwise-authenticated principal (see this service's BUGS on the second
/// case: it is folded into the same code on purpose, for [`credential_proto`]'s reason: a caller must not
/// be able to distinguish "wrong password" from "the service is out of memory" by trying the same
/// identity twice and comparing outcomes).
pub const DENIED: u64 = 2;

/// The request word's lengths are out of range. Not an authentication outcome, and a client that
/// gets this has learned nothing about whether the identity exists.
pub const MALFORMED: u64 = 3;

/// **One attribution record**, sent once per successful login on the service's own audit endpoint
/// (`user/src/login.rs`'s `AUDIT` slot), so the property DECISIONS §109 names ("a server ... logs
/// which channel a request arrived on") is checkable rather than merely claimed. `w0` is
/// [`ATTRIBUTED`], `w1` is the channel's sequence number (the order this service established
/// channels in, starting at 0), `w2` is [`identity_hint`] of the identity that established it.
///
/// This is **login's own record of what it just established**, not a downstream server logging a
/// later request against that channel; see this service's BUGS for the natural place the second
/// half would live and why nothing in this tree needs it yet.
pub const ATTRIBUTED: u64 = 1;

/// Pack up to the first 8 bytes of `identity` into one `u64`, big-endian, zero-padded. A debugging
/// aid for the audit record, not a general identity encoding: two identities that share an 8-byte
/// prefix are indistinguishable in it, which is fine for this contract (the audit record exists to
/// let a test, or an operator, confirm *which* login produced *which* channel among the identities
/// actually in use, not to serve as a second store of who exists).
pub fn identity_hint(identity: &[u8]) -> u64 {
    let mut buf = [0u8; 8];
    let n = identity.len().min(8);
    buf[..n].copy_from_slice(&identity[..n]);
    u64::from_be_bytes(buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_hint_round_trips_short_names() {
        assert_eq!(identity_hint(b"chris"), identity_hint(b"chris"));
        assert_ne!(identity_hint(b"chris"), identity_hint(b"corinne"));
    }

    #[test]
    fn req_and_place_agree_with_cred_proto_on_the_shape() {
        let mut page = [0u8; PAGE];
        let w0 = place(&mut page, b"chris", b"secret", LOGIN).expect("fits");
        assert_eq!(op(w0), LOGIN);
        let (id, secret) = read(&page, w0).expect("well-formed");
        assert_eq!(id, b"chris");
        assert_eq!(secret, b"secret");
    }
}
