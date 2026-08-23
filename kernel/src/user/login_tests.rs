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
/// `CARETAKER_REGION_PAGES` 64 + `CLIENT_BUDGET_PAGES` 64) for the nine this suite performs in total
/// (one in the headline test, two in the two-identity test, six in
/// `the_login_service_serves_past_the_old_cspace_ceiling`, which relies on those three having
/// already run against this same memoized instance to reach nine, one past the old eight-login
/// cspace ceiling): 128 + 9*128 = 1280. 1408 leaves the same headroom the smaller number did, for
/// `build_child`'s own bookkeeping, without costing the ledger a number nobody can explain. Every
/// one of those nine pages is permanent (see `user/src/login.rs`'s BUGS: the caretaker's
/// construction *memory* is still never reclaimed, only the cspace slot is now), so raising this
/// past 640 raised `kernel::testing::SUITE_FRAME_BUDGET` too; that comment carries the account.
const CONSTRUCTION_PAGES: u64 = 1408;

/// **Wire the whole system once**: entropy, the credential service (`credential_tests::provisioned`'s
/// own fixture, which already provisions `chris` and `corinne` among the three family logins
/// design/roadmap/56-secrets-and-entropy.md names), the file service, and the login service itself.
/// Reusing that fixture rather than provisioning a second store is deliberate: a login-only store
/// would duplicate `credentialer_test_client.rs`'s `PEOPLE` fixture for no reason a reader could
/// point at.
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
