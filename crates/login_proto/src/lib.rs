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
//! carry a delegated budget).
//!
//! **Two phases, not one, since milestone 49's channel-per-client update.** `REQUEST`/`RESULT` (the
//! endpoints every client is handed at spawn) are a **front door**, shared by every client this
//! service will ever see, for the width of exactly one message each: a bare [`CONNECT`], answered
//! with a fresh, private [`CONNECTED`] channel. The identity and secret an actual login needs never
//! touch the front door at all, which is what removes the hazard the front door used to carry (see
//! this service's BUGS, "One client at a time", for what that hazard was and why a shared front door
//! carrying only [`CONNECT`] does not reintroduce it):
//!
//! ```text
//!   client --send(REQUEST, connect_word(), 0, 0)------------------------> login
//!   client <----------------- recv(RESULT) -> CONNECTED ---------------- login
//!   client <---- RECV_CAP(RESULT) x 3: priv_request, priv_result, page - login
//!
//!   client --place(), send(priv_request, w0, 0, 0)-----------------------> login
//!   client <----------------- recv(priv_result) -> OK, DENIED or -------- login
//!                                                   NO_TERMINAL
//!   client <---- RECV_CAP(priv_result) x 5, only after OK ---------------  login
//!
//!   client --send(REQUEST, logout_word(), 0, 0)---------------------------> login
//!   client <----------------- recv(RESULT) -> LOGGED_OUT ----------------- login
//! ```
//!
//! [`CONNECT`] carries no page: there is nothing in it a client did not already know, so the front
//! door never maps or reads a shared staging page at all, and two clients racing to connect can only
//! ever contend for two harmless, empty, freshly-minted objects, never for each other's identity or
//! secret. Login answers [`CONNECT`]s one at a time (this process has one thread and no wait-any
//! primitive), but that is service order, not shared state: each answer is a private key handed to
//! exactly one holder before login goes on to serve the actual login it just enabled.
//!
//! **[`LOGOUT`] travels on the front door itself, not on a private channel** (milestone 49's
//! terminal update): unlike [`CONNECT`], it carries no secret and needs no private staging, so
//! there is nothing a shared front door would expose by handling it directly. See
//! [`logout_word`] and this contract's own BUGS for what it does and does not authenticate.
//!
//! On [`OK`], login sends exactly five capabilities over `priv_result`, in this order:
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
//! 5. **the terminal** (milestone 49's terminal update, DECISIONS-recommended "deny cleanly" shape,
//!    `design/roadmap/49-users-and-attribution.md`'s own BUGS): a `Rendezvous`, `WRITE` only, the
//!    same right the interactive boot's shell already holds on it. **Only ever present because it
//!    could be**: `login`'s own `serve_login` refuses with [`NO_TERMINAL`] before authentication is
//!    even attempted while another session already holds it, so every `OK` this contract answers
//!    carries this fifth capability too. There is exactly one physical terminal and exactly one
//!    holder at a time; see this contract's own BUGS and `user/src/login.rs`'s module docs for the
//!    single-session design this is deliberately not more than.
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
//! On [`DENIED`], [`MALFORMED`] or [`NO_TERMINAL`], nothing follows: the client holds exactly what
//! it held before it asked.
//!
//! # BUGS
//!
//! **[`LOGOUT`] authenticates nothing.** It is a bare word on the shared front door, deliberately:
//! unlike a login it carries no secret to protect and needs no private channel, but the flip side is
//! that any process holding the front-door request endpoint (every client this service will ever
//! see, by construction)
//! can free the terminal out from under whoever is using it. That is a real, unauthenticated
//! interruption a hostile co-tenant could perform, not merely a discourtesy; it is accepted here
//! because today's actual deployment is one interactive boot with one physical terminal and no
//! untrusted co-tenant reaching this endpoint at all, which is exactly the scope
//! `design/roadmap/49-users-and-attribution.md`'s own terminal BUGS entry names as this slice's
//! bound. A deployment that must defend against a hostile holder of `REQUEST` needs `LOGOUT` to
//! carry proof (the identity that is logging out, or better, a capability only that session holds),
//! which is real work this slice does not build.
//!
//! **A stale terminal handoff has no automatic recovery.** If the process holding the terminal
//! capability exits, crashes, or simply never calls [`logout_word`], `login`'s own `terminal_held`
//! flag stays set forever and every later login is refused `NO_TERMINAL`, indistinguishably from a
//! session that is genuinely still in use. There is no liveness check on the holder (this tree's
//! usual "no wait-any primitive" bound: `login` cannot watch its client and also keep serving new
//! connections), so recovering from an abandoned session today means restarting `login` itself. See
//! `user/src/login.rs`'s own BUGS for the same limitation stated at the component a reader meets
//! first.
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

/// **The front door's only legal request** (milestone 49's channel-per-client update): "give me my
/// own private channel." Carries no lengths and touches no page; build the word with
/// [`connect_word`] rather than [`place`] (there is nothing to stage).
pub const CONNECT: u64 = 2;

/// The one opcode a private, per-client channel accepts. There is only one verb (authenticate), so
/// unlike `credential_proto` (verify, provision) there is nothing to distinguish it from; it exists so a
/// reader of a captured request word can tell this contract's traffic from any other sharing the
/// encoding. Build a request word with `place(page, identity, secret, LOGIN)`, which returns it
/// already carrying this opcode and the two lengths. Sent on the private `priv_request` endpoint
/// [`CONNECTED`] delegates, never on the front door.
pub const LOGIN: u64 = 1;

/// **Free the terminal** (milestone 49's terminal update): "I am done; the next login may have it."
/// A bare word on the shared front door, like [`CONNECT`], and for the same reason [`CONNECT`]
/// carries no page: there is nothing here a caller did not already know. Build it with
/// [`logout_word`]. See this contract's own BUGS for what `LOGOUT` does not authenticate.
pub const LOGOUT: u64 = 3;

/// `send(REQUEST, connect_word(), 0, 0)`. The bare word [`CONNECT`] travels as; a client never calls
/// [`place`] for this step, because there is no identity or secret to stage.
pub fn connect_word() -> u64 {
    CONNECT << credential_proto::OP_SHIFT
}

/// `send(REQUEST, logout_word(), 0, 0)`. The bare word [`LOGOUT`] travels as, on the *front door*
/// (unlike [`connect_word`]'s answer, this needs no private channel): see [`LOGOUT`]'s own doc.
pub fn logout_word() -> u64 {
    LOGOUT << credential_proto::OP_SHIFT
}

/// **A private channel is ready.** Answered on the front door's `RESULT` endpoint, followed by
/// exactly three delegated capabilities, in this order: the private `priv_request` endpoint
/// (`WRITE`), the private `priv_result` endpoint (`READ`), and a page frame (`READ | WRITE`) to stage
/// the actual login on. See the module docs for the two-phase exchange.
pub const CONNECTED: u64 = 4;

/// **Authenticated.** Exactly five capabilities follow on the private result endpoint; see the
/// module docs for the order.
pub const OK: u64 = 1;

/// **Refused.** The identity is unknown, the secret is wrong, the service could not mint a
/// capability set for an otherwise-authenticated principal, or (on the front door) the service could
/// not mint a private channel at all (see this service's BUGS on the second and third cases: both
/// are folded into the same code on purpose, for [`credential_proto`]'s reason: a caller must not be able
/// to distinguish "wrong password" from "the service is out of memory" by trying the same identity
/// twice and comparing outcomes).
pub const DENIED: u64 = 2;

/// The request word's lengths are out of range, or the front door was sent something other than
/// [`CONNECT`] or [`LOGOUT`]. Not an authentication outcome, and a client that gets this has learned
/// nothing about whether the identity exists.
pub const MALFORMED: u64 = 3;

/// **The terminal is already held by another session** (milestone 49's terminal update, the
/// "deny cleanly" shape). Sent on the private `priv_result` endpoint, *before* the presented
/// identity and secret are even relayed to the credential service: this is global, caller-
/// independent state (there is exactly one physical terminal), not a fact about any identity, so
/// checking it first costs nothing an attacker could turn into a timing oracle and saves a round
/// trip to the credential service on every refusal. A client that gets this has learned nothing
/// about whether its identity or secret were correct.
pub const NO_TERMINAL: u64 = 5;

/// **The terminal is free again.** Sent on the front door's `RESULT` endpoint in answer to
/// [`LOGOUT`]. Idempotent: sent whether or not anything was actually held, since a logout that
/// arrives when nobody holds the terminal is harmless rather than an error.
pub const LOGGED_OUT: u64 = 6;

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

    #[test]
    fn connect_word_carries_no_lengths_and_reads_back_as_connect() {
        let w0 = connect_word();
        assert_eq!(op(w0), CONNECT);
        // Unlike a LOGIN word, there is nothing else packed into it: the low 32 bits (where `place`
        // packs the two lengths) are zero.
        assert_eq!(w0 & 0xffff_ffff, 0);
        // And it is distinguishable from LOGIN's own opcode, so a front door that only expects
        // CONNECT can refuse a stray LOGIN word rather than mistake it for one.
        assert_ne!(CONNECT, LOGIN);
    }

    #[test]
    fn logout_word_carries_no_lengths_and_reads_back_as_logout() {
        let w0 = logout_word();
        assert_eq!(op(w0), LOGOUT);
        assert_eq!(w0 & 0xffff_ffff, 0);
        // Distinguishable from both existing front-door/private-channel opcodes, so a front door
        // that dispatches on `op(w0)` can never confuse the three.
        assert_ne!(LOGOUT, CONNECT);
        assert_ne!(LOGOUT, LOGIN);
    }

    #[test]
    fn every_wire_code_this_contract_defines_is_distinct() {
        // The two namespaces (a request's opcode, and a private channel's OK/DENIED/... verdict)
        // are read from different fields by different code and are allowed to share numbers; this
        // checks each namespace is internally distinct, which is the property a dispatch `match`
        // actually relies on.
        let request_ops = [LOGIN, CONNECT, LOGOUT];
        for (i, a) in request_ops.iter().enumerate() {
            for b in &request_ops[i + 1..] {
                assert_ne!(a, b, "two request opcodes collide");
            }
        }
        let verdicts = [OK, DENIED, MALFORMED, CONNECTED, NO_TERMINAL, LOGGED_OUT];
        for (i, a) in verdicts.iter().enumerate() {
            for b in &verdicts[i + 1..] {
                assert_ne!(a, b, "two verdict codes collide");
            }
        }
    }
}

// ===========================================================================================
// **How the login service is started**, which is a contract between whoever spawns it and the
// program itself rather than between the program and its clients (milestone 233).
//
// It is in this crate because rule 7 leaves nowhere else: three binaries have to agree on these
// two addresses (`user/src/login.rs`, `crates/system_initializer`, and the kernel's own test
// harness in `kernel/src/user/login_service.rs`), and what two binaries agree on is a crate. The
// siting is the honest weak point: this crate's own first line calls itself "the wire contract
// between a client and the login service", and a spawn contract is neither wire nor client. The
// alternative was a crate holding two constants. Provisional, like every name a lane mints.
// ===========================================================================================

/// **Where `fs_subtree_caretaker`'s ELF bytes are mapped, read-only, before `login`'s `_start`
/// runs**, with the length in `x0`/`a0`/`rdi` (milestone 233).
///
/// **This replaced a mapping of the whole initrd archive, and the reason is not economy.** `login`
/// used to read the archive at `user_rt::initrd::INITRD_VA` and index it by name, which is what the
/// kernel's own test harness handed it and what nothing else ever did: `crates/system_initializer`
/// spawns this program through `supervision_proto::build_child`, which can map only pages the
/// spawner holds a `PageFrame` capability for, and the archive is reserved RAM the frame allocator
/// does not own and no capability names. So the real interactive boot started `login` with no
/// archive at all and it died at `_start` on every boot for an unknown length of time
/// (design/roadmap/233-login-never-runs.md).
///
/// The fix could have gone the other way, and giving `login` the archive is the option that was
/// refused: it needs one program's bytes and a manifest to check them against, and a service that
/// can read every file in the boot image to answer a password holds authority it never exercises.
/// A blob costs the spawner **no capability-table slots at all** (`supervision_proto`'s own
/// `fill_and_map` holds one frame at a time and deletes it), which is what let this land against
/// the 21-of-24 peak `kernel::cap::CAPABILITY_TABLE_PEAK_MEASURED` records.
///
/// Zero length means the spawner had no vouched-for caretaker to hand over. That is not a failure
/// to start: `login` comes up and answers every login [`DENIED`], which is exactly what
/// `crates/system_initializer` already does with a program it cannot measure.
pub const CARETAKER_ELF_VA: u64 = 0x0000_0000_0100_0000;

/// **Where `measured_boot::PROGRAM_MEASUREMENTS`' bytes are mapped, read-only, before `login`'s
/// `_start` runs**, with the length in `x1`/`a1`/`rsi` (milestone 233).
///
/// Four megabytes above [`CARETAKER_ELF_VA`] so a caretaker image would have to grow forty-fold
/// before the two could meet. Both sit well below `user_rt::initrd::INITRD_VA` and well above the
/// per-channel scratch VAs `login` bump-allocates, which is the only other thing in that address
/// space that grows.
pub const PROGRAM_MEASUREMENTS_VA: u64 = 0x0000_0000_0140_0000;
