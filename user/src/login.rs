//! **The login service: authentication produces capabilities** (milestone 49,
//! design/roadmap/49-users-and-attribution.md, DECISIONS §109).
//!
//! Unix login authenticates and then mutates a global identity field, which is uid's whole trick
//! and the thing this system refuses to have. This process authenticates a presented identity
//! against the credential service milestone 56 already built, and on success hands back **a
//! capability set** instead: a fresh directory, a fresh budget, and (see this program's BUGS) not
//! yet a terminal. It is the powerbox pattern with the human at one end, answering the question
//! milestone 49's own doc named and left open: who gets which capabilities at startup, which used
//! to be a fact baked into `crates/system_initializer` at build time and is, for the one login path
//! this program serves, a fact decided here at run time instead.
//!
//! # What "produces capabilities" means, concretely
//!
//! A successful login does not narrow a capability this process already holds and hand a copy on
//! (that would make every principal a viewer of the same underlying object, which is the shared-
//! endpoint anti-pattern DECISIONS §109 names and rejects three times over: the compositor, the FS
//! server's handle table, the fault endpoint). It **builds a fresh `fs_subtree_caretaker`**, the
//! same construction `crates/system_initializer` performs for a directory-granted spawn, out of a
//! region split off this process's own construction budget. Two different successful logins are
//! therefore two different endpoint *objects*: distinguishable, independently revocable, and each
//! nameable only by the principal that established it, which is the channel-shaped attribution
//! DECISIONS §109 decided on.
//!
//! # What this slice does and does not scope (see BUGS for the rest)
//!
//! Every successful login is attenuated to the **same** subtree ([`SUBTREE_NAME`]) with the same
//! rights: this program does not decide *which* subtree a principal may see, only *that* it gets
//! one of its own. Per-principal subtree scoping is milestone 47's already-built mechanism
//! (`fs_subtree_caretaker`'s whole reason to exist); wiring identity to a specific subtree name is
//! named as follow-on rather than guessed at here.
//!
//! # Capability contract
//!
//! - slot [`REQUEST`]: `RECV`. A client sends one [`login_proto::LOGIN`] request here, identity and
//!   secret staged at [`LOGIN_VA`] the way [`login_proto::place`] writes them.
//! - slot [`RESULT`]: `WRITE | GRANT`. The verdict, and on [`login_proto::OK`] three delegated
//!   capabilities, in the order `login_proto`'s module docs give.
//! - slot [`VERIFY`]: `WRITE` on the credential service's verify endpoint (milestone 56). This
//!   process never provisions it and never could: the provision endpoint is deleted at both ends
//!   before any client of the credential service exists (`user/src/credentialer.rs`).
//! - slot [`FS_EP`]: `WRITE | GRANT` on the file service's root directory capability. What every
//!   minted caretaker attenuates.
//! - slot [`FS_FRAME`]: a `Frame`, `READ | WRITE`, the page the file service shares with its
//!   clients. Delegated on to each authenticated principal (see the module docs on why one frame
//!   serves every hop).
//! - slot [`CONSTRUCTION_UT`]: `WRITE | GRANT`. Everything a caretaker and a client budget are
//!   built from. Never given away, unlike `root_supervisor`'s: this process keeps serving logins
//!   for its whole life, so unlike an init that hands its authority away once, it must keep some.
//! - slot [`AUDIT`]: `WRITE`. One [`login_proto::ATTRIBUTED`] message per successful login, so the
//!   property DECISIONS §109 names (a server logging which channel it just established, and for
//!   whom) is checkable rather than merely claimed. See this program's BUGS on the scope of what
//!   this endpoint proves.
//! - mapped [`LOGIN_VA`]: the page shared with whichever client is calling right now.
//! - mapped [`CRED_VA`]: the page shared with the credential service, for the relayed `VERIFY`.
//! - mapped [`INITRD_VA`]: the archive, read-only, so this process can find `fs_subtree_caretaker`'s
//!   own bytes the way `crates/system_initializer` does.
//!
//! Name: unrecorded. Provisional, minted 2026-08-22 for milestone 49 and not yet put to calef.
//! `login` is the plain noun for what this program answers a request to do, on the pattern
//! `clock`/`entropy`/`credentialer` already set, which is the reasoning a ratification would test.
//!
//! # BUGS
//!
//! **One client at a time, structurally.** [`REQUEST`] and [`RESULT`] are each a single endpoint, so
//! two concurrent callers would interleave their words on one page, exactly the limit
//! `credentialer.rs` already documents for its own verify page and for the same reason: this
//! process has one thread and no wait-any primitive, so it cannot serve two rendezvous at once.
//! `fs_proto`'s answer (a channel per client) is the shape to copy when a second concurrent caller
//! exists; today this program's only callers are `login_test_client` roles the test suite runs one
//! at a time.
//!
//! **No terminal.** The roadmap's own text names three things a login hands back: a root directory,
//! a budget, a terminal. This program hands back the first two. A terminal in this system is a
//! singleton hardware-backed resource wired once at interactive boot
//! (`crates/system_initializer::boot`); minting a second one, or multiplexing the one that exists
//! across logins, is real work this slice does not attempt and does not want to guess the shape of.
//! It is unscoped follow-on, not an oversight.
//!
//! **Every successful login is attenuated to the same subtree, with the same rights.** See the
//! module docs above. Per-principal scoping needs a way to look an identity up against a subtree
//! name, which this program has no store for and milestone 49's own doc does not ask it to build
//! (the roadmap's "isolation between humans" row is already built as milestone 47's per-shell root;
//! what is missing is only the wiring between the two, which is follow-on).
//!
//! **Not wired into the interactive boot.** This process is spawned directly by the kernel's guest
//! test harness (`kernel/src/user/login_service.rs`), the same way `credentialer` is, and is not
//! reachable from `crates/system_initializer::boot`'s real prompt. Replacing the shell's own
//! build-time endowment with a real login prompt is the multi-lane remainder of this milestone.
//!
//! **This process does not consult `measured_boot::PROGRAM_MEASUREMENTS` before building a
//! caretaker.** `crates/system_initializer` refuses to load a program whose bytes do not match the
//! archive's measurement table (milestone 104); this process loads `fs_subtree_caretaker` by name
//! with no such check, because it is not wired into the trust chain the kernel's boot vouches for.
//! Inconsistent with milestone 104's discipline, and named here rather than fixed, because fixing it
//! well means deciding how a non-init loader joins that chain at all, which is a design question and
//! not a one-line patch.
//!
//! **A caretaker's construction memory is never reclaimed.** Every successful login spends
//! [`CARETAKER_REGION_PAGES`] and [`CLIENT_BUDGET_PAGES`] out of [`CONSTRUCTION_UT`] for the rest of
//! this process's life; there is no logout that gives the *memory* back. A real deployment needs a
//! teardown path (a principal's supervision endpoint reaching this process, or a caretaker that can
//! be `Untyped::DESTROY`ed by name, which needs the region kept rather than dropped, so it is a
//! design choice about who ends up holding it and not a smaller version of the fix below), which is
//! real work this slice does not build. [`CONSTRUCTION_UT`] is sized by whoever spawns this
//! process, and running out answers every further login with [`login_proto::DENIED`] rather than a
//! distinguishable error, folded into that code for `login_proto`'s own stated reason (a caller must
//! not learn "the service is out of resources" by comparing outcomes across two attempts with the
//! same identity).
//!
//! This used to also leak a **cspace slot** per login, a tighter and separate ceiling from the
//! memory one: this process's own capability table has sixteen slots (`kernel::cap::CSPACE_SLOTS`)
//! and eight are spent at rest, so keeping `region`'s capability past a successful mint left room
//! for exactly eight logins ever, after which the cspace itself (not `CONSTRUCTION_UT`) answered
//! every further attempt with `DENIED`. `mint` now drops its own copy of `region` once the
//! caretaker has confirmed descent (a `cap_delete`, not a `Untyped::DESTROY`: the caretaker's
//! address space, TCB and endpoints are untouched), which removes that ceiling; the memory ceiling
//! above is unaffected and still the real, documented bound.
//!
//! **The audit endpoint proves establishment, not per-request attribution.** [`login_proto::ATTRIBUTED`]
//! records which identity established which channel at the moment this process minted it. It does
//! not prove that a *downstream* server, later, can say which channel one of its own requests
//! arrived on; DECISIONS §109's own text describes both halves and this program is only the first.
//! No server in this tree today needs the second: `fs_subtree_caretaker` already serves exactly one
//! principal by construction (there is nothing to distinguish), and the credential service is
//! anonymous by design (DECISIONS §109 predates this and neither wants nor needs to know who is
//! asking). Wiring the second half into a real multi-tenant consumer is follow-on for whenever such
//! a consumer exists.

#![no_std]
// Program entry points, not the crates/ library surface milestone 68's ratchet tracks
// (DECISIONS §107): each `[[bin]]` is its own crate root with one `_start`, and 58 of them
// documenting an OS-facing ABI entry point is not what the lint is for.
#![allow(missing_docs)]
#![no_main]

use supervision_proto::{
    ChildEndowment, build_child, retype_obj_from as retype_obj, tcb_start, untyped_split,
};
use user_rt::{call, cap_delete, invoke, recv, send};

/// A client's login request, `RECV` (milestone 49).
const REQUEST: u64 = 0;
/// The verdict and, on success, three delegated capabilities, `WRITE | GRANT`.
const RESULT: u64 = 1;
/// The credential service's verify endpoint (milestone 56), `WRITE`.
const VERIFY: u64 = 2;
/// The file service's root directory capability, `WRITE | GRANT`.
const FS_EP: u64 = 3;
/// The file service's shared page, a `Frame`, `READ | WRITE`.
const FS_FRAME: u64 = 4;
/// Everything a caretaker and a client budget are built from, `WRITE | GRANT`.
const CONSTRUCTION_UT: u64 = 5;
/// One [`login_proto::ATTRIBUTED`] message per successful login, `WRITE`.
const AUDIT: u64 = 6;

/// The page shared with the current client. Distinct from `credentialer.rs`'s own request pages
/// (a different process, so no collision is possible), but numbered in the same family so a reader
/// who knows one contract's addresses recognises the shape of the other's.
const LOGIN_VA: u64 = 0x0000_0000_00e2_0000;
/// The page shared with the credential service, for the relayed `VERIFY`.
const CRED_VA: u64 = 0x0000_0000_00e3_0000;
/// Where the archive is mapped read-only. Must match `kernel::user::INITRD_VA`.
const INITRD_VA: u64 = 0x2000_0000;
/// Where a built caretaker and the file service's shared page meet. Must match
/// `user/src/fs_subtree_caretaker.rs`'s `PAGE_VA` (the same address every caretaker in this tree
/// uses, since the caretaker itself hardcodes it and this process copies its ELF, not its address).
const CARETAKER_FS_VA: u64 = 0x0000_0000_0060_0000;

/// **The one subtree every successful login is attenuated to, in this slice.** See this program's
/// module docs and BUGS: per-principal scoping is real follow-on work and not guessed at here.
const SUBTREE_NAME: &str = fs_proto::fixture::tree::SUB;

/// This process's own scratch mappings for `build_child` (its page tables, never a child's).
/// Small: one caretaker at a time is ever mid-construction, so this never needs to hold more than
/// one build's worth of intermediate page tables. Sized generously against `INIT_OWN_PAGES` (128
/// in `crates/system_initializer`, which builds six boot components against the same budget).
const OWN_UT_PAGES: u64 = 128;

/// One caretaker's whole construction: its address space, TCB, and stack.
/// `crates/system_initializer::DIR_JOB_REGION_PAGES` (96) covers a caretaker **and** the program
/// behind it; this process builds only the caretaker, so a smaller region should hold it, with
/// margin rather than a tight fit (a region too small fails as `Err(())` mid-login, which this
/// process can only answer with the one code `login_proto::DENIED` already carries for "could not
/// mint", see this program's BUGS).
const CARETAKER_REGION_PAGES: u64 = 64;

/// Stack pages beyond the one `build_child` maps, matching
/// `crates/system_initializer::CARETAKER_STACK_PAGES`: measured for that program rather than
/// guessed, and this is the same program.
const CARETAKER_STACK_PAGES: u64 = 4;

/// What this process hands each authenticated principal as its own budget. Arbitrary and modest,
/// for the demonstration this slice is; a real deployment sizes it against what a session actually
/// needs, which is not yet a question this program has enough callers to answer.
const CLIENT_BUDGET_PAGES: u64 = 64;

#[unsafe(no_mangle)]
pub extern "C" fn _start(_a0: u64, initrd_len: u64, _a2: u64) -> ! {
    // SAFETY: the wiring mapped `initrd_len` bytes of reserved RAM read-only at INITRD_VA.
    let archive =
        unsafe { core::slice::from_raw_parts(INITRD_VA as *const u8, initrd_len as usize) };
    let Ok(fs) = nifefs::Fs::parse(archive) else {
        fail(1)
    };
    let Some(care_bytes) = fs.read("fs_subtree_caretaker") else {
        fail(2)
    };
    let Ok(care_elf) = elf::Elf::parse(care_bytes) else {
        fail(3)
    };

    let Ok(own_ut) = untyped_split(CONSTRUCTION_UT, OWN_UT_PAGES) else {
        fail(4)
    };

    // How many channels this process has established, in order. The audit trail's sequence number,
    // not a capacity: `CONSTRUCTION_UT` is what actually bounds how many logins this process can
    // serve (see BUGS).
    let mut seq: u64 = 0;

    loop {
        let (w0, _w1, _w2) = recv(REQUEST);
        // SAFETY: the wiring mapped one page read/write at LOGIN_VA before this process ran.
        let page = unsafe { core::slice::from_raw_parts(LOGIN_VA as *const u8, login_proto::PAGE) };
        let Some((identity, secret)) = login_proto::read(page, w0) else {
            wipe_login_page();
            send(RESULT, login_proto::MALFORMED, 0, 0);
            continue;
        };
        // Computed before the page is wiped: `identity` borrows LOGIN_VA and must not be read after.
        let hint = login_proto::identity_hint(identity);

        // SAFETY: the wiring mapped one page read/write at CRED_VA before this process ran, shared
        // with the credential service and with nothing else.
        let cred_page =
            unsafe { core::slice::from_raw_parts_mut(CRED_VA as *mut u8, cred_proto::PAGE) };
        let placed = cred_proto::place(cred_page, identity, secret, cred_proto::verify::VERIFY);
        // The presented secret has now been copied to CRED_VA (or the placement failed and never
        // will be); either way LOGIN_VA's copy is done being read.
        wipe_login_page();
        let Some(cw0) = placed else {
            send(RESULT, login_proto::MALFORMED, 0, 0);
            continue;
        };
        let (cr0, _) = call(VERIFY, cw0, 0);
        cred_proto::wipe(cred_page);

        if !cred_proto::authenticated(cr0) {
            send(RESULT, login_proto::DENIED, 0, 0);
            continue;
        }

        match mint(own_ut, &care_elf) {
            Some((dir_ep, budget)) => {
                send(RESULT, login_proto::OK, 0, 0);
                delegate(dir_ep, abi::rights::WRITE);
                delegate(FS_FRAME, abi::rights::READ | abi::rights::WRITE);
                delegate(budget, abi::rights::WRITE | abi::rights::GRANT);
                cap_delete(dir_ep);
                cap_delete(budget);
                send(AUDIT, login_proto::ATTRIBUTED, seq, hint);
                seq += 1;
            }
            // Authenticated, and the service still could not serve it (the construction budget is
            // spent, or the caretaker's descent was refused). Answered identically to a wrong
            // secret; see login_proto::DENIED's own doc on why that fold is deliberate rather than
            // a missed distinction.
            None => {
                send(RESULT, login_proto::DENIED, 0, 0);
            }
        }
    }
}

/// **Mint one principal's capability set**: a fresh `fs_subtree_caretaker` and a fresh budget, both
/// held with full rights so [`delegate`] can narrow them on the way out. `None` on any failure,
/// which this process's caller answers with [`login_proto::DENIED`] (see this program's BUGS on why
/// that is the honest fold rather than a missing distinction).
fn mint(own_ut: u64, care: &elf::Elf) -> Option<(u64, u64)> {
    let region = untyped_split(CONSTRUCTION_UT, CARETAKER_REGION_PAGES).ok()?;
    let narrow_ep = retype_obj(region, abi::objtype::ENDPOINT).ok()?;
    let ready = retype_obj(region, abi::objtype::ENDPOINT).ok()?;

    let (lo, hi) = fs_proto::grant::pack_name(SUBTREE_NAME.as_bytes());
    let spec = fs_proto::grant::spec(SUBTREE_NAME.len(), fs_proto::dir::ALL);

    // Its whole authority: the file service to attenuate, the endpoint it will serve, one place to
    // say it is ready, and the frame it shares with the file service. No untyped of its own, no
    // clock, nothing that could name another process. See `crates/system_initializer::build_caretaker`,
    // which this mirrors.
    let built = build_child(
        own_ut,
        region,
        care,
        &ChildEndowment {
            caps: &[
                (FS_EP, abi::rights::WRITE),
                (narrow_ep, abi::rights::READ),
                (ready, abi::rights::WRITE),
            ],
            maps: &[(CARETAKER_FS_VA, FS_FRAME, abi::aspace::MAP_RW)],
            stack_pages: CARETAKER_STACK_PAGES,
            ..ChildEndowment::new()
        },
    );
    let tcb = built.ok()?;
    let started = tcb_start(tcb, lo, hi, spec);
    cap_delete(tcb);
    if !started {
        cap_delete(ready);
        cap_delete(narrow_ep);
        // `region` is not reclaimed here: a caretaker that failed to start left nothing running in
        // it, but this process has no `DESTROY` capability on its own construction budget's
        // children today. See BUGS.
        return None;
    }
    // The one bounded wait: the caretaker's descent against the file service, exactly the
    // handshake `crates/system_initializer::build_caretaker` performs.
    let (verdict, _, _) = recv(ready);
    cap_delete(ready);
    if verdict != fs_proto::fixture::READY {
        cap_delete(narrow_ep);
        return None;
    }

    // **Drop our own copy of `region` now.** The caretaker is up and holds its own narrowed
    // copies of everything it needs (`FS_EP`, its half of `narrow_ep`, the frame); this process
    // has no further use for the region it built it from. This is `cap_delete`, a local cspace
    // slot free, not `Untyped::DESTROY`: the caretaker's address space, TCB and endpoint are
    // untouched, exactly the pattern `root_supervisor` and `system_initializer::boot` use to drop
    // a builder's own authority once a child holds its own copies.
    //
    // Skipping this used to leak one of this process's own sixteen cspace slots per successful
    // login (`region` was never freed on the success path), which is a tighter and previously
    // undocumented ceiling than the memory bound this program's BUGS names:
    // `CSPACE_SLOTS` (16) minus this process's eight slots at rest (`REQUEST`..`AUDIT` plus
    // `own_ut`) left room for exactly eight successful logins ever, after which the ninth
    // correctly-authenticated one was silently answered `DENIED`, indistinguishable from a wrong
    // password. See `kernel::user::login_tests::the_login_service_serves_past_the_old_cspace_ceiling`.
    cap_delete(region);

    let budget = untyped_split(CONSTRUCTION_UT, CLIENT_BUDGET_PAGES).ok()?;
    Some((narrow_ep, budget))
}

/// Delegate our own copy of `slot`, narrowed to `rights`, over [`RESULT`]. `GRANT` must already be
/// on our own copy for the kernel to allow this at all (`abi::endpoint::SEND_CAP`'s contract);
/// every capability this process delegates was retyped or split by this process, so it always is.
fn delegate(slot: u64, rights: u64) {
    // SAFETY: `svc`/`ecall`; the kernel checks WRITE on RESULT and GRANT on the delegated capability.
    unsafe {
        invoke(RESULT, abi::endpoint::SEND_CAP, slot, rights, 0);
    }
}

/// Zero the identity/secret staged at [`LOGIN_VA`], on every path: a malformed request, a denial,
/// and a success all leave a presented secret sitting in a page two processes share until this
/// runs.
fn wipe_login_page() {
    // SAFETY: the wiring mapped one page read/write here, and this process is the only writer
    // between a request arriving and its reply going out.
    let page = unsafe { core::slice::from_raw_parts_mut(LOGIN_VA as *mut u8, login_proto::PAGE) };
    login_proto::wipe(page);
}

fn fail(step: u64) -> ! {
    send(AUDIT, 0xDEAD_0000_0000_0000 | step, 0, 0);
    supervision_proto::fail()
}

user_rt::panic_handler!();
