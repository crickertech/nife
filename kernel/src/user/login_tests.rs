use login_service as ls;

use super::*;
use crate::sched;

/// The login service's construction budget (`user/src/login.rs`'s `CONSTRUCTION_UT`), sized for
/// this suite's exact needs rather than guessed generously: `crate::untyped::create` reserves this
/// many frames from the boot's free pool the moment `ls::start` runs, and nothing here ever gives
/// them back (see `user/src/login.rs`'s BUGS), so this number is a direct, permanent charge against
/// `kernel::testing::SUITE_FRAME_BUDGET`. The first version of this constant (2048) blew that budget
/// by itself; this is the account the ledger's own message asks for.
///
/// 128 for `login`'s own scratch (`OWN_UT_PAGES`), plus 128 per successful login (that program's
/// `CARETAKER_REGION_PAGES` 64 + `CLIENT_BUDGET_PAGES` 64) for the twelve this suite performs in
/// total: one in the headline test, two in the two-identity test, six in
/// `the_login_service_serves_past_the_old_cspace_ceiling`, and three in
/// `login_scopes_each_identity_to_its_own_provisioned_subtree` (DECISIONS §117's per-identity
/// scoping: `chris`'s mark, `corinne`'s mark, and `chris`'s second, independent session). 128 +
/// 12*128 = 1664. One more login in this suite is *meant* to fail to mint
/// (`login_denies_an_authenticated_identity_with_no_provisioned_subtree`: `graeme` has a real
/// credential and no home subtree) and still permanently spends `mint`'s `CARETAKER_REGION_PAGES`
/// (64, not the full 128: `CLIENT_BUDGET_PAGES` is never split on this failing path) before the
/// descent refuses it (`user/src/login.rs`'s own BUGS: `region` is not reclaimed on this path
/// either): 1664 + 64 = 1728. 1856 leaves the same 128-page headroom the smaller number did, for
/// `build_child`'s own bookkeeping, without costing the ledger a number nobody can explain. Every
/// one of the twelve successful logins' pages is permanent (see `user/src/login.rs`'s BUGS: the
/// caretaker's construction *memory* is still never reclaimed, only the cspace slot is now), so
/// raising this past 640 raised `kernel::testing::SUITE_FRAME_BUDGET` too; that comment carries the
/// account.
const CONSTRUCTION_PAGES: u64 = 1856;

/// `EEXIST`, matching `identity_provisioner.rs`'s own local constant: `fs_proto` does not re-export
/// it under a name (that file's own comment), so every direct caller of `fs::MKDIR` names it again.
const EEXIST: i32 = 17;

/// **Ensure `name` exists as a subtree under `fs_ep`'s root, tolerating `EEXIST`** exactly as
/// `identity_provisioner.rs`'s own `mkdir_home` does: the same opcode
/// ([`fs_proto::fs::MKDIR`]), the same rights ([`fs_proto::dir::ALL`]), and the identity string
/// itself as the name (DECISIONS §117). Issued directly from this test rather than through a
/// spawned `identity_provisioner`, because this suite's fs root is the tree-wide shared fixture
/// (`fs_service`'s own memoized `ensure`, one FS server for the whole kernel test binary): another
/// suite may have already created this name (`identity_provisioning_tests.rs` provisions `chris`
/// too, against its own, separate credential store, sharing only the filesystem), and `EEXIST` is
/// the correct, expected answer then, not a failure.
///
/// This is what makes `wired`'s login instance below able to attenuate `chris` and `corinne` to a
/// subtree of their own at all: `login.rs` no longer creates one (DECISIONS §117: provision-time
/// creation, never auto-vivified at login), so something upstream of it must, exactly as a real
/// deployment's `identity_provisioner` would before anyone logs in.
fn ensure_home_subtree(fs_ep: sched::EpId, fs_frame: u64, name: &[u8]) {
    // SAFETY: `fs_frame` is the file service's own shared page, already wired and idle (no client
    // exists yet at the point `wired` calls this, before the login service or any test client is
    // spawned), so writing into it directly and then calling through `fs_ep` is the same shape
    // `identity_provisioner_service.rs`'s own `provision` uses for its request pages.
    let page = unsafe {
        core::slice::from_raw_parts_mut(mmu::phys_to_virt(fs_frame) as *mut u8, fs_proto::PAGE)
    };
    page[..name.len()].copy_from_slice(name);
    let r = sched::ipc_call(
        fs_ep,
        [
            fs_proto::fs::req(fs_proto::fs::MKDIR, fs_proto::fs::ROOT, name.len() as u64),
            fs_proto::dir::ALL,
        ],
    );
    match fs_proto::reply_errno(r[0] as i64) {
        Some(errno) => assert_eq!(
            errno,
            EEXIST,
            "MKDIR of {:?} failed for a reason other than already existing",
            core::str::from_utf8(name),
        ),
        // A fresh directory handle this call minted; close it, best-effort, the same choice
        // `identity_provisioner.rs`'s own `mkdir_home` makes for the same reason (this test has no
        // further use for it, and a leaked handle-table slot in the FS server is not a correctness
        // bug a caller of this helper could observe).
        None => {
            let _ = sched::ipc_call(fs_ep, [fs_proto::fs::req(fs_proto::fs::CLOSE, r[0], 0), 0]);
        }
    }
}

/// **Wire the whole system once**: entropy, the credential service (`credential_tests::provisioned`'s
/// own fixture, which already provisions `chris`, `corinne` and `graeme` among the three family
/// logins design/roadmap/56-secrets-and-entropy.md names), the file service, and the login service
/// itself. Reusing that fixture rather than provisioning a second store is deliberate: a login-only
/// store would duplicate `credentialer_test_client.rs`'s `PEOPLE` fixture for no reason a reader
/// could point at.
///
/// **`chris` and `corinne` are given a home subtree** ([`ensure_home_subtree`]) before the login
/// service starts; `graeme` deliberately is not, so a credential that authenticates but has nowhere
/// to attenuate to is a fixture this suite already has on hand rather than one built to order. See
/// DECISIONS §117 and `login.rs`'s own BUGS on why an authenticated identity with no subtree is
/// refused rather than served, and `login_denies_an_authenticated_identity_with_no_provisioned_subtree`
/// below for the test that checks it.
///
/// Memoized on `credential_tests::provisioned`'s own terms and for the same reason: the login
/// service's construction budget is spent and never reclaimed by design in this slice (see
/// `user/src/login.rs`'s BUGS), so a re-wiring test-to-test would exhaust it for nothing.
///
/// `None` when a dependency this depends on is not attached to this run: no virtio-rng
/// (`credential_tests::provisioned`'s own condition) or no RedoxFS disk (`fs_service`'s). Both are
/// legitimate boot configurations this tree supports, so a caller skips rather than fails.
fn wired() -> Option<ls::Wiring> {
    use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};
    static DONE: AtomicBool = AtomicBool::new(false);
    static OK: AtomicBool = AtomicBool::new(false);
    static SAVED: [AtomicU64; 3] = [AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0)];

    if !DONE.load(Ordering::Acquire) {
        let result = (|| {
            let (cred_wiring, _, _) = credential_tests::provisioned()?;
            let (fs_ep, fs_frame) =
                fs_service::root_directory(fs_service::blk_server_image(), fs_server_image())?;
            ensure_home_subtree(fs_ep, fs_frame, b"chris");
            ensure_home_subtree(fs_ep, fs_frame, b"corinne");
            let login_image = program("login").expect("no login program in the initrd archive");
            let w = ls::start(
                login_image,
                cred_wiring.verify,
                cred_wiring.verify_frame,
                fs_ep,
                fs_frame,
                CONSTRUCTION_PAGES,
            );
            Some(w)
        })();

        if let Some(w) = &result {
            SAVED[0].store(w.request, Ordering::Relaxed);
            SAVED[1].store(w.result, Ordering::Relaxed);
            SAVED[2].store(w.audit, Ordering::Relaxed);
            OK.store(true, Ordering::Release);
        }
        DONE.store(true, Ordering::Release);
    }
    if !OK.load(Ordering::Acquire) {
        return None;
    }
    Some(ls::Wiring {
        request: SAVED[0].load(Ordering::Relaxed),
        result: SAVED[1].load(Ordering::Relaxed),
        audit: SAVED[2].load(Ordering::Relaxed),
    })
}

/// The name `fs_server`'s image travels under, on the same terms `dir_capability_tests` reads it:
/// present only when `test` built the FS server, absent for a plain interactive run.
fn fs_server_image() -> &'static [u8] {
    program("fs_server").expect("no fs_server program in the initrd archive")
}

/// **The headline.** A correct identity and secret produce a directory capability and a budget
/// that actually work: a real `READDIR` through the minted caretaker, and a real page retyped from
/// the minted budget. Neither is asked to match the fixture's exact contents, which is a fact about
/// the image and not about this capability (see `login_test_client.rs`).
#[test_case]
fn login_grants_a_working_capability_set_to_the_identity_it_verified() {
    let Some(w) = wired() else {
        crate::testing::skip!("no virtio-rng device or no RedoxFS disk attached");
    };
    let cli =
        program("login_test_client").expect("no login_test_client program in the initrd archive");
    let r = ls::client(cli, &w, ls::ROLE_CHRIS);
    assert_eq!(
        r[0],
        ls::RPT_OK,
        "a correct identity and secret were not authenticated",
    );
    assert_eq!(
        r[1] & ls::F_DIR_WORKS,
        ls::F_DIR_WORKS,
        "the delegated directory capability did not answer a real READDIR",
    );
    assert_eq!(
        r[1] & ls::F_BUDGET_WORKS,
        ls::F_BUDGET_WORKS,
        "the delegated budget could not retype a page",
    );
    // `login`'s `send(AUDIT, ...)` is a blocking rendezvous: it does not return, and the service
    // cannot serve the *next* login, until something receives it. `wired()` memoizes one service
    // instance across this whole suite, so every test that causes a successful login must drain
    // this endpoint or leave the service permanently stuck here for every later test.
    let a = sched::ipc_recv(w.audit);
    assert_eq!(
        a[0],
        login_proto::ATTRIBUTED,
        "no attribution record followed a successful login",
    );
}

/// **The refusal, and the promise that nothing follows it.** A real identity with the wrong secret
/// is denied. `login_test_client`'s `ROLE_WRONG_SECRET` never calls `RECV_CAP`; if the service ever
/// sent a capability after a denial anyway, that role would simply never reach its report and this
/// test would hang rather than fail cleanly, which is the honest failure mode for a protocol
/// promise broken at the sender.
#[test_case]
fn login_denies_a_wrong_secret_and_sends_nothing_further() {
    let Some(w) = wired() else {
        crate::testing::skip!("no virtio-rng device or no RedoxFS disk attached");
    };
    let cli =
        program("login_test_client").expect("no login_test_client program in the initrd archive");
    let r = ls::client(cli, &w, ls::ROLE_WRONG_SECRET);
    assert_eq!(r[0], ls::RPT_DENIED, "a wrong secret was not refused",);
}

/// **Two principals, two channels, correctly attributed.** `chris` and `corinne` both log in and
/// both get a working capability set (this suite's headline test already proves one identity's
/// set works; this proves a second, different identity's does too, independently). The login
/// service's own audit trail then names, in order, which sequence number belongs to which
/// identity, which is DECISIONS §109's property made checkable: a server that establishes a
/// channel and can say who it belongs to.
///
/// **What this does not prove**, and it is named rather than assumed: that the two channels are
/// not secretly the same underlying object. That claim rests on the kernel's own guarantee that
/// retyping from a freshly split region always yields a new object, which is a capability-system
/// primitive this tree relies on everywhere and does not re-derive at this integration test's
/// level. What this test *does* prove is that both channels work independently and that the audit
/// record is honest about which principal established which.
#[test_case]
fn two_different_identities_get_independently_working_channels_and_correct_attribution() {
    let Some(w) = wired() else {
        crate::testing::skip!("no virtio-rng device or no RedoxFS disk attached");
    };
    let cli =
        program("login_test_client").expect("no login_test_client program in the initrd archive");

    let r_chris = ls::client(cli, &w, ls::ROLE_CHRIS);
    assert_eq!(r_chris[0], ls::RPT_OK, "chris was not authenticated");
    let a_chris = sched::ipc_recv(w.audit);
    assert_eq!(
        a_chris[0],
        login_proto::ATTRIBUTED,
        "no attribution record followed chris's login",
    );
    assert_eq!(
        a_chris[2],
        login_proto::identity_hint(b"chris"),
        "the attribution record named the wrong identity for the first channel",
    );

    let r_corinne = ls::client(cli, &w, ls::ROLE_CORINNE);
    assert_eq!(r_corinne[0], ls::RPT_OK, "corinne was not authenticated",);
    let a_corinne = sched::ipc_recv(w.audit);
    assert_eq!(
        a_corinne[0],
        login_proto::ATTRIBUTED,
        "no attribution record followed corinne's login",
    );
    assert_eq!(
        a_corinne[2],
        login_proto::identity_hint(b"corinne"),
        "the attribution record named the wrong identity for the second channel",
    );

    assert!(
        a_corinne[1] > a_chris[1],
        "the second channel's sequence number ({}) did not follow the first's ({})",
        a_corinne[1],
        a_chris[1],
    );

    for (label, r) in [("chris", r_chris), ("corinne", r_corinne)] {
        assert_eq!(
            r[1] & ls::F_DIR_WORKS,
            ls::F_DIR_WORKS,
            "{label}'s directory capability did not work",
        );
        assert_eq!(
            r[1] & ls::F_BUDGET_WORKS,
            ls::F_BUDGET_WORKS,
            "{label}'s budget did not work",
        );
    }
}

/// **Nothing else would have caught this**: `login`'s own capability table has sixteen slots
/// (`kernel::cap::CSPACE_SLOTS`), eight are spent at rest (`REQUEST`..`AUDIT` plus its own scratch
/// untyped), and before `user/src/login.rs`'s `mint` learned to drop its own copy of the
/// caretaker's construction region, that region's capability was never freed on a successful
/// login. That left room for exactly eight successful logins ever; a ninth, correctly
/// authenticated, was silently answered [`login_proto::DENIED`], indistinguishable from a wrong
/// password, at a far tighter ceiling than the memory bound `user/src/login.rs`'s BUGS documents.
///
/// This performs **six** successful logins against one service instance, on top of the three the
/// headline test and the two-identity test above already performed against the same memoized
/// instance (`wired()`): nine in total, one past that old eight-login ceiling. It asserts every one
/// of its own six actually worked (not merely that the verdict was `OK`): a real `READDIR` through
/// the minted directory and a real page retyped from the minted budget, the same proof the headline
/// test above asks for. Alternates `chris` and `corinne` rather than repeating one identity, so a
/// regression that only leaked on one particular identity's path would not hide from this test by
/// accident.
///
/// Six rather than nine performed here directly: a real login's `CARETAKER_REGION_PAGES` and
/// `CLIENT_BUDGET_PAGES` are permanently unreclaimable memory (see `user/src/login.rs`'s BUGS), so
/// every attempt this test adds is a permanent charge against `kernel::testing::SUITE_FRAME_BUDGET`
/// and not merely against `CONSTRUCTION_PAGES`. Reusing the other two tests' three logins rather
/// than repeating them here is what keeps that charge to what actually proves the fix.
#[test_case]
fn the_login_service_serves_past_the_old_cspace_ceiling() {
    let Some(w) = wired() else {
        crate::testing::skip!("no virtio-rng device or no RedoxFS disk attached");
    };
    let cli =
        program("login_test_client").expect("no login_test_client program in the initrd archive");

    const ATTEMPTS: usize = 6;
    for i in 0..ATTEMPTS {
        let role = if i % 2 == 0 {
            ls::ROLE_CHRIS
        } else {
            ls::ROLE_CORINNE
        };
        let r = ls::client(cli, &w, role);
        assert_eq!(
            r[0],
            ls::RPT_OK,
            "login {i} of {ATTEMPTS} in this test (the 4th through 9th against this shared \
             instance, past the old eight-login cspace ceiling) was not authenticated; a real \
             password was answered as though it were wrong",
        );
        assert_eq!(
            r[1] & ls::F_DIR_WORKS,
            ls::F_DIR_WORKS,
            "login {i}'s directory capability did not work",
        );
        assert_eq!(
            r[1] & ls::F_BUDGET_WORKS,
            ls::F_BUDGET_WORKS,
            "login {i}'s budget did not work",
        );
        // Drain the attribution record so the service is free to serve the next login (the
        // headline test's own note: `send(AUDIT, ...)` is a blocking rendezvous).
        let a = sched::ipc_recv(w.audit);
        assert_eq!(
            a[0],
            login_proto::ATTRIBUTED,
            "no attribution record followed login {i}",
        );
    }
}

/// **DECISIONS §117: each identity is attenuated to its own provisioned subtree, not the old
/// shared fixture and not another identity's.**
///
/// `chris` and `corinne` each log in and, through the directory capability `login` delegated,
/// `CREATE` a marker file naming themselves ([`ls::ROLE_CHRIS_MARK`],
/// [`ls::ROLE_CORINNE_MARK`]); both also confirm
/// [`fs_proto::fixture::tree::INNER`] is absent, which is this suite's own proof (not merely a
/// stated intent) that neither landed in the old shared subtree every identity used to be
/// attenuated to before this milestone. `chris` then logs in a **second, independent** time
/// ([`ls::ROLE_CHRIS_CHECK`]) and reads the marker back: it must read `chris`'s own,
/// which is the property under test stated positively: the *same* identity's two sessions land in
/// the *same* subtree, and it is not `corinne`'s (had the old bug still been present, `corinne`'s
/// later write would have overwritten `chris`'s marker in the one subtree they would have shared,
/// and this final read would come back `corinne`'s instead).
///
/// `wired`'s own `ensure_home_subtree` issues the identical `fs_proto::fs::MKDIR` request
/// `identity_provisioner.rs`'s own `mkdir_home` does (same opcode, same
/// [`fs_proto::dir::ALL`] rights, the identity string as the name); that milestone's own suite
/// (`identity_provisioning_tests.rs`, `provisioning_creates_a_working_credential_and_a_real_subtree`)
/// already proves the *tool* produces a real, descendable subtree end to end, so this test's job is
/// the other half: that `login`, given one, finds and attenuates to the *right* one.
#[test_case]
fn login_scopes_each_identity_to_its_own_provisioned_subtree() {
    let Some(w) = wired() else {
        crate::testing::skip!("no virtio-rng device or no RedoxFS disk attached");
    };
    let cli =
        program("login_test_client").expect("no login_test_client program in the initrd archive");

    let r_chris = ls::client(cli, &w, ls::ROLE_CHRIS_MARK);
    assert_eq!(r_chris[0], ls::RPT_OK, "chris was not authenticated");
    assert_eq!(
        r_chris[1] & ls::F_MARKER_WRITTEN,
        ls::F_MARKER_WRITTEN,
        "chris's marker was not created and written",
    );
    assert_eq!(
        r_chris[1] & ls::F_NOT_SHARED_SUBTREE,
        ls::F_NOT_SHARED_SUBTREE,
        "chris's granted directory carries the old shared fixture's own file \
         (fs_proto::fixture::tree::INNER), so it is not a subtree of chris's own",
    );
    let a_chris = sched::ipc_recv(w.audit);
    assert_eq!(
        a_chris[0],
        login_proto::ATTRIBUTED,
        "no attribution record followed chris's marking login",
    );

    let r_corinne = ls::client(cli, &w, ls::ROLE_CORINNE_MARK);
    assert_eq!(r_corinne[0], ls::RPT_OK, "corinne was not authenticated");
    assert_eq!(
        r_corinne[1] & ls::F_MARKER_WRITTEN,
        ls::F_MARKER_WRITTEN,
        "corinne's marker was not created and written",
    );
    assert_eq!(
        r_corinne[1] & ls::F_NOT_SHARED_SUBTREE,
        ls::F_NOT_SHARED_SUBTREE,
        "corinne's granted directory carries the old shared fixture's own file \
         (fs_proto::fixture::tree::INNER), so it is not a subtree of corinne's own",
    );
    let a_corinne = sched::ipc_recv(w.audit);
    assert_eq!(
        a_corinne[0],
        login_proto::ATTRIBUTED,
        "no attribution record followed corinne's marking login",
    );

    let r_check = ls::client(cli, &w, ls::ROLE_CHRIS_CHECK);
    assert_eq!(
        r_check[0],
        ls::RPT_OK,
        "chris's second, independent login was not authenticated",
    );
    assert_eq!(
        r_check[1] & ls::F_DIR_WORKS,
        ls::F_DIR_WORKS,
        "chris's second login's directory capability did not work",
    );
    assert_eq!(
        r_check[2],
        login_proto::identity_hint(b"chris"),
        "chris's second, independent login did not read back chris's own marker: either it did \
         not land in chris's subtree, or corinne's write clobbered it, which is exactly the \
         everyone-shares-one-subtree bug this milestone fixes",
    );
    let a_check = sched::ipc_recv(w.audit);
    assert_eq!(
        a_check[0],
        login_proto::ATTRIBUTED,
        "no attribution record followed chris's second login",
    );
}

/// **The considered fold this milestone adds: a real, authenticated identity with no provisioned
/// subtree is refused, indistinguishably from a wrong password.**
///
/// `graeme` has a real credential in `wired`'s store (`credential_tests::provisioned`'s own family
/// fixture) and, deliberately, no home subtree (`wired` only calls `ensure_home_subtree` for `chris`
/// and `corinne`): the case `identity_provisioner` never having run for a real identity, or its
/// `MKDIR` never reaching this file service's disk. `mint`'s caretaker construction reaches the
/// same `OPENDIR`-against-a-missing-name refusal, and `login`'s existing fold answers it with
/// [`login_proto::DENIED`], the same code [`login_denies_a_wrong_secret_and_sends_nothing_further`]
/// above already proves a wrong password gets. See `user/src/login.rs`'s own BUGS for the reasoning
/// (a caller must not be able to tell "your identity has no home" from "your password is wrong" by
/// comparing outcomes across attempts).
///
/// No audit record follows this login, by the same rule a wrong secret gets none: `login`'s `AUDIT`
/// send lives only on the success path.
#[test_case]
fn login_denies_an_authenticated_identity_with_no_provisioned_subtree() {
    let Some(w) = wired() else {
        crate::testing::skip!("no virtio-rng device or no RedoxFS disk attached");
    };
    let cli =
        program("login_test_client").expect("no login_test_client program in the initrd archive");
    let r = ls::client(cli, &w, ls::ROLE_NO_SUBTREE);
    assert_eq!(
        r[0],
        ls::RPT_DENIED,
        "graeme's real credential, with no provisioned subtree, was not refused",
    );
}
