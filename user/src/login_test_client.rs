//! **The login service's test client** (milestone 49; see `user/src/login.rs`).
//!
//! One binary, several roles, on `credentialer_test_client`'s own pattern: a program that shares the
//! honest path with an attempted-wrong-secret run is a fairer test of a refusal than a different
//! program failing for its own reasons. Every role holds the identical endowment: the login
//! service's request endpoint, its result endpoint, a report endpoint, and the page it stages a
//! request in.
//!
//! - [`ROLE_CHRIS`] and [`ROLE_CORINNE`] present two different identities' correct credentials.
//!   Both must succeed, and the kernel test compares what each received: two distinct caretaker
//!   endpoints is the channel-shaped attribution DECISIONS §109 decided on.
//! - [`ROLE_WRONG_SECRET`] presents a real identity with the wrong secret. It must be refused, and
//!   nothing may follow: this role never calls `RECV_CAP`, because the protocol promises nothing
//!   does on a denial and a client that tried would block forever.
//! - [`ROLE_CHRIS_MARK`] and [`ROLE_CORINNE_MARK`] (DECISIONS §117) each log in and then, through
//!   the delegated directory, `CREATE` a one-shot marker file naming the identity that wrote it, and
//!   check that [`filesystem_proto::fixture::tree::INNER`] is *not* there (that name lives only in the old,
//!   shared fixture subtree every identity used to be attenuated to before §117; its absence is this
//!   client's own proof that the granted directory is not that one). [`ROLE_CHRIS_CHECK`] logs in as
//!   `chris` again, in a second, independent channel, and reads the marker back: the kernel test
//!   compares what it reads against what each role wrote, which is the whole property under test:
//!   two different identities land in two different, isolated subtrees, and the same identity's two
//!   sessions land in the *same* one.
//!
//! A role that succeeds does not stop at the verdict. It **uses** what it received: a directory
//! read through the caretaker's endpoint, the file service's shared frame mapped and used for that
//! read, and a page retyped from the delegated budget. A capability that arrived but does not work
//! would pass every earlier assertion and fail only this one.
//!
//! # Capability contract
//!
//! - slot 0: the login service's request endpoint, `WRITE`.
//! - slot 1: the login service's result endpoint, `READ` (both the verdict and, on success, the
//!   three delegated capabilities arrive here).
//! - slot 2: a report endpoint, `WRITE`.
//! - mapped [`PAGE_VA`]: the page shared with the login service, where this program stages its
//!   identity and secret.
//! - `a0`: the role.
//!
//! Name: unrecorded. Provisional, minted 2026-08-22 for milestone 49 and not yet put to calef, on
//! `credentialer_test_client`'s own pattern (`<service>_test_client`).

#![no_std]
// Program entry points, not the crates/ library surface milestone 68's ratchet tracks
// (DECISIONS §107): each `[[bin]]` is its own crate root with one `_start`, and 58 of them
// documenting an OS-facing ABI entry point is not what the lint is for.
#![allow(missing_docs)]
#![no_main]

use user_rt::{call, exit, invoke, map_frame, recv, recv_cap, send};

/// The login service's request endpoint (slot 0), `WRITE`.
const SERVICE: u64 = 0;
/// The login service's result endpoint (slot 1), `READ`.
const RESULT: u64 = 1;
/// The report endpoint (slot 2), `WRITE`.
const REPORT: u64 = 2;

/// The page shared with the login service.
const PAGE_VA: u64 = 0x0000_0000_00e2_0000;
/// Where this process maps the delegated file-service frame, once it has one. Distinct from
/// [`PAGE_VA`]: they are two different pages (this process's login request, and the filesystem
/// contract's shared page), and a client of both must not confuse them the way `credentialer.rs`'s
/// own two-frame rule exists to prevent for its own pair.
const FS_VA: u64 = 0x0000_0000_00f0_0000;

pub const ROLE_CHRIS: u64 = 0;
pub const ROLE_CORINNE: u64 = 1;
pub const ROLE_WRONG_SECRET: u64 = 2;
/// Log in as `chris`, write this identity's marker, and check the old fixed subtree's own name is
/// absent (DECISIONS §117). See the module docs.
pub const ROLE_CHRIS_MARK: u64 = 3;
/// `corinne`'s half of the pair above.
pub const ROLE_CORINNE_MARK: u64 = 4;
/// Log in as `chris` a second time, in an independent channel, and read the marker
/// [`ROLE_CHRIS_MARK`] wrote, packed into the report the same way `login`'s own audit trail packs
/// an identity ([`login_proto::identity_hint`]). See the module docs.
pub const ROLE_CHRIS_CHECK: u64 = 5;
/// A real, authenticated identity that no one ever ran `identity_provisioner` for: a credential
/// exists (`kernel/src/user/login_tests.rs` provisions it directly into the same store this
/// program's other roles authenticate against, without the matching `MKDIR`) but its home subtree
/// does not. Must be refused exactly like [`ROLE_WRONG_SECRET`] (DECISIONS §117's "no distinguishable
/// signal" answer; see `login.rs`'s own BUGS).
pub const ROLE_NO_SUBTREE: u64 = 6;

/// The one-shot marker file every `*_MARK`/`*_CHECK` role reads or writes, inside the identity's own
/// granted subtree. Chosen to collide with nothing else this tree's fixtures use.
const MARKER_NAME: &str = "whoami";

/// This role's identity and the secret it presents. `chris`/`corinne` are the same two identities
/// `credentialer_test_client.rs`'s own `PEOPLE` fixture already provisions, reused rather than
/// duplicated: `kernel/src/user/login_tests.rs` wires this service against
/// `credential_tests::provisioned`'s already-sealed store, so a second, login-only credential
/// service and a second fixture would be two copies of one fact. `graeme` is the same fixture's
/// third person, provisioned with a credential and (deliberately, for [`ROLE_NO_SUBTREE`]) no
/// subtree.
fn credentials(role: u64) -> (&'static [u8], &'static [u8]) {
    match role {
        ROLE_CHRIS | ROLE_CHRIS_MARK | ROLE_CHRIS_CHECK => {
            (b"chris", b"correct horse battery staple")
        }
        ROLE_CORINNE | ROLE_CORINNE_MARK => (b"corinne", b"a different secret entirely"),
        // The right identity, the wrong secret: a fairer refusal than an unknown identity would be,
        // for `credentialer_test_client`'s own reason (a boundary tested by a program sharing the
        // honest path rather than one failing for a different reason).
        ROLE_WRONG_SECRET => (b"chris", b"not-the-password"),
        ROLE_NO_SUBTREE => (b"graeme", b"and a third one"),
        _ => (b"", b""),
    }
}

/// The report's first word: what the service answered.
pub const RPT_OK: u64 = login_proto::OK;
pub const RPT_DENIED: u64 = login_proto::DENIED;
pub const RPT_MALFORMED: u64 = login_proto::MALFORMED;

/// Bits of the report's second word, set only when [`RPT_OK`] is the first: which of the delegated
/// capabilities this process proved actually work, rather than merely that they arrived.
pub const F_DIR_WORKS: u64 = 1 << 0;
pub const F_BUDGET_WORKS: u64 = 1 << 1;
/// **Set when [`filesystem_proto::fixture::tree::INNER`] is confirmed absent** from the granted directory:
/// that name exists only in the old, shared fixture subtree every identity used to be attenuated to.
/// Its absence is this client's own proof (not merely an assertion) that the identity in this role is
/// not looking at that subtree. Set only by [`ROLE_CHRIS_MARK`]/[`ROLE_CORINNE_MARK`].
pub const F_NOT_SHARED_SUBTREE: u64 = 1 << 2;
/// **Set when the marker file was created and written successfully.** Set only by
/// [`ROLE_CHRIS_MARK`]/[`ROLE_CORINNE_MARK`]; its absence there means the isolation proof below did
/// not get to run at all, which the kernel test must treat as its own failure rather than silence.
pub const F_MARKER_WRITTEN: u64 = 1 << 3;

#[unsafe(no_mangle)]
pub extern "C" fn _start(role: u64, _a1: u64, _a2: u64) -> ! {
    let (identity, secret) = credentials(role);
    // SAFETY: the wiring mapped one page read/write at PAGE_VA before this process ran.
    let page = unsafe { core::slice::from_raw_parts_mut(PAGE_VA as *mut u8, login_proto::PAGE) };
    let Some(w0) = login_proto::place(page, identity, secret, login_proto::LOGIN) else {
        done(RPT_MALFORMED, 0, 0);
    };
    send(SERVICE, w0, 0, 0);
    let (verdict, _, _) = recv(RESULT);

    if verdict != login_proto::OK {
        // `login_proto`'s own promise: nothing follows a refusal. Reporting here, rather than
        // attempting `RECV_CAP`, is the check that the promise holds; a service that sent a fourth
        // message anyway would leave the *next* login's first `RECV_CAP` reading this one's leftover
        // word instead of blocking as it should, which is exactly the kind of protocol desync a
        // client that blindly tried to receive here would hide rather than catch.
        done(verdict, 0, 0);
    }

    // Three capabilities, in login_proto's fixed order.
    let (_, dir_ep, _) = recv_cap(RESULT);
    let (_, fs_frame, _) = recv_cap(RESULT);
    let (_, budget, _) = recv_cap(RESULT);

    let mut flags = 0u64;
    let mut hint = 0u64;

    // Prove the directory capability works: map the delegated frame, then read the granted
    // subtree's listing through the delegated caretaker endpoint. A capability that merely arrived
    // would pass every check up to this line and fail only this one.
    if map_frame(fs_frame, FS_VA, true, budget) {
        let (r0, _) = call(
            dir_ep,
            filesystem_proto::fs::req(filesystem_proto::fs::READDIR, filesystem_proto::fs::ROOT, 0),
            0,
        );
        // `call` returns the reply word as a `u64`; a negative errno reads as a huge one
        // (`entropy_proto`'s convention, followed here and by `smb_server.rs`'s own `fs_readdir`).
        // A `READDIR` answers the byte count written (>= 0) or a negative errno. Any non-negative
        // answer is the capability working; this test does not pin the fixture's exact contents,
        // which is a fact about the image and not about this capability.
        if (r0 as i64) >= 0 {
            flags |= F_DIR_WORKS;

            // DECISIONS §117's own proof: which subtree did this login actually land in? Only the
            // `*_MARK`/`*_CHECK` roles do this; the original three roles above are unchanged.
            match role {
                ROLE_CHRIS_MARK => {
                    if write_marker(dir_ep, b"chris") {
                        flags |= F_MARKER_WRITTEN;
                    }
                    if absent(dir_ep, filesystem_proto::fixture::tree::INNER) {
                        flags |= F_NOT_SHARED_SUBTREE;
                    }
                }
                ROLE_CORINNE_MARK => {
                    if write_marker(dir_ep, b"corinne") {
                        flags |= F_MARKER_WRITTEN;
                    }
                    if absent(dir_ep, filesystem_proto::fixture::tree::INNER) {
                        flags |= F_NOT_SHARED_SUBTREE;
                    }
                }
                ROLE_CHRIS_CHECK => {
                    if let Some(h) = read_marker(dir_ep) {
                        hint = h;
                    }
                }
                _ => {}
            }
        }
    }

    // Prove the budget works: retype one page from it. `RETYPE`'s reply is the new frame's slot
    // (>= 0) or a negative error.
    // SAFETY: `svc`/`ecall`; the kernel validates the capability and the method.
    if unsafe { invoke(budget, abi::untyped::RETYPE, 0, 0, 0) } >= 0 {
        flags |= F_BUDGET_WORKS;
    }

    done(RPT_OK, flags, hint);
}

/// Copy `bytes` into the shared filesystem page (a name to open/create, or data to write).
fn put_page(bytes: &[u8]) {
    for (i, &b) in bytes.iter().enumerate() {
        // SAFETY: FS_VA is a mapped, writable page of at least `filesystem_proto::PAGE` bytes (this program
        // maps exactly one page of it); a name or this marker's short content is far shorter.
        unsafe { core::ptr::write_volatile((FS_VA + i as u64) as *mut u8, b) };
    }
}

/// Read `n` bytes out of the shared filesystem page into `out`.
fn get_page(n: usize, out: &mut [u8]) {
    for (i, b) in out.iter_mut().take(n).enumerate() {
        // SAFETY: as above; `n` is bounded by the page and by `out`.
        *b = unsafe { core::ptr::read_volatile((FS_VA + i as u64) as *const u8) };
    }
}

/// `CREATE` [`MARKER_NAME`] under `dir` and `WRITE` `content` into it, then `CLOSE` the handle.
/// `true` only if every step succeeded. Create is create, not create-or-open
/// (`filesystem_proto::fs::CREATE`'s own contract), so this fails loudly rather than overwriting a marker a
/// previous run left behind, which would silently defeat the isolation proof this role exists for.
fn write_marker(dir: u64, content: &[u8]) -> bool {
    put_page(MARKER_NAME.as_bytes());
    let (h, _) = call(
        dir,
        filesystem_proto::fs::req(filesystem_proto::fs::CREATE, 0, MARKER_NAME.len() as u64),
        0,
    );
    if (h as i64) < 0 {
        return false;
    }
    put_page(content);
    let (w, _) = call(
        dir,
        filesystem_proto::fs::req(filesystem_proto::fs::WRITE, h, content.len() as u64),
        0,
    );
    let ok = w as i64 == content.len() as i64;
    let _ = call(
        dir,
        filesystem_proto::fs::req(filesystem_proto::fs::CLOSE, h, 0),
        0,
    );
    ok
}

/// `true` if `OPEN`ing `name` under `dir` is refused. The expected answer for a name that lives only
/// in the old, shared fixture subtree ([`filesystem_proto::fixture::tree::INNER`]) when `dir` is a genuinely
/// different, identity-scoped one.
fn absent(dir: u64, name: &str) -> bool {
    put_page(name.as_bytes());
    let (r0, _) = call(
        dir,
        filesystem_proto::fs::req(filesystem_proto::fs::OPEN, 0, name.len() as u64),
        0,
    );
    (r0 as i64) < 0
}

/// `OPEN` and `READ` [`MARKER_NAME`] under `dir`, packed the same way `login`'s own audit trail
/// packs an identity ([`login_proto::identity_hint`]), so the kernel test can compare what this role
/// read against what [`write_marker`]'s caller wrote without a second encoding to keep in sync.
/// `None` if the marker could not be opened or read.
fn read_marker(dir: u64) -> Option<u64> {
    put_page(MARKER_NAME.as_bytes());
    let (h, _) = call(
        dir,
        filesystem_proto::fs::req(filesystem_proto::fs::OPEN, 0, MARKER_NAME.len() as u64),
        0,
    );
    if (h as i64) < 0 {
        return None;
    }
    let (n, _) = call(
        dir,
        filesystem_proto::fs::req(filesystem_proto::fs::READ, h, 16),
        0,
    );
    let _ = call(
        dir,
        filesystem_proto::fs::req(filesystem_proto::fs::CLOSE, h, 0),
        0,
    );
    if (n as i64) < 0 {
        return None;
    }
    let mut buf = [0u8; 16];
    get_page((n as usize).min(buf.len()), &mut buf);
    Some(login_proto::identity_hint(
        &buf[..(n as usize).min(buf.len())],
    ))
}

fn done(tag: u64, w1: u64, w2: u64) -> ! {
    send(REPORT, tag, w1, w2);
    exit()
}

user_rt::panic_handler!();
