//! **The login service's test client** (milestone 49; see `user/src/login.rs`).
//!
//! One binary, three roles, on `credentialer_test_client`'s own pattern: a program that shares the
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

/// This role's identity and the secret it presents. The same two identities
/// `credentialer_test_client.rs`'s own `PEOPLE` fixture already provisions (`chris`, `corinne`),
/// reused rather than duplicated: `kernel/src/user/login_tests.rs` wires this service against
/// `credential_tests::provisioned`'s already-sealed store, so a second, login-only credential
/// service and a second fixture would be two copies of one fact.
fn credentials(role: u64) -> (&'static [u8], &'static [u8]) {
    match role {
        ROLE_CHRIS => (b"chris", b"correct horse battery staple"),
        ROLE_CORINNE => (b"corinne", b"a different secret entirely"),
        // The right identity, the wrong secret: a fairer refusal than an unknown identity would be,
        // for `credentialer_test_client`'s own reason (a boundary tested by a program sharing the
        // honest path rather than one failing for a different reason).
        ROLE_WRONG_SECRET => (b"chris", b"not-the-password"),
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

    // Prove the directory capability works: map the delegated frame, then read the granted
    // subtree's listing through the delegated caretaker endpoint. A capability that merely arrived
    // would pass every check up to this line and fail only this one.
    if map_frame(fs_frame, FS_VA, true, budget) {
        let (r0, _) = call(
            dir_ep,
            fs_proto::fs::req(fs_proto::fs::READDIR, fs_proto::fs::ROOT, 0),
            0,
        );
        // `call` returns the reply word as a `u64`; a negative errno reads as a huge one
        // (`entropy_proto`'s convention, followed here and by `smb_server.rs`'s own `fs_readdir`).
        // A `READDIR` answers the byte count written (>= 0) or a negative errno. Any non-negative
        // answer is the capability working; this test does not pin the fixture's exact contents,
        // which is a fact about the image and not about this capability.
        if (r0 as i64) >= 0 {
            flags |= F_DIR_WORKS;
        }
    }

    // Prove the budget works: retype one page from it. `RETYPE`'s reply is the new frame's slot
    // (>= 0) or a negative error.
    // SAFETY: `svc`/`ecall`; the kernel validates the capability and the method.
    if unsafe { invoke(budget, abi::untyped::RETYPE, 0, 0, 0) } >= 0 {
        flags |= F_BUDGET_WORKS;
    }

    done(RPT_OK, flags, 0);
}

fn done(tag: u64, w1: u64, w2: u64) -> ! {
    send(REPORT, tag, w1, w2);
    exit()
}

user_rt::panic_handler!();
