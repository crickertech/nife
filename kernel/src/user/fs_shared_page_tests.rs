//! **The shared-frame witness** (milestone 599 (a frame per filesystem client channel), provisional).
//!
//! Finding 1 of `notes/shared-page-audit.md`: the FS server shared one read-write staging frame
//! with every client a boot wired, so what kept two clients apart was which of them was blocked, not
//! what each could map. This test builds two live clients on one file service, one rewriting the
//! frame the other staged its name in, and asserts the outcome. **Since milestone 599 gave each
//! client its own channel window** (keyed by the badge on its endpoint capability), the two map
//! different frames, so it now asserts isolation: the attacker's write cannot reach the victim's
//! window, and the victim's request resolves the name it staged. Before the fix this same witness
//! reproduced the substitution (`SHARED_SUBSTITUTED`); the assertion flipped when the fix landed,
//! which is what made this test the gate on it.
//!
//! One module for both ISAs, for `dir_capability_tests`'s reason: nothing here is
//! architecture-specific, so the parity gate (DECISIONS §19) is met by the same test running on
//! every architecture `script/test` boots. It wires two portable `fs_test_client` roles and one
//! portable `redoxfs_server`, and asserts on a verdict word.
//!
//! **Why a handshake and not a race.** The property under test is not "does a race fire" but "can a
//! second holder of the frame change the bytes the server resolves for another client's call". The
//! server reads the name from the page after the message arrives and has no way to tell whose bytes
//! it read. A two-message handshake over a sync endpoint forces the interleaving the audit
//! describes (victim stages its name, attacker overwrites it, victim calls) deterministically, so
//! the witness reproduces on every architecture and every run rather than only when a race lands.
//! The interleaving is one a genuinely runnable second writer reaches by chance on the four-core
//! boots; the handshake removes the flakiness, not the mechanism.

use filesystem_protocol::fixture;

use super::*;
use crate::sched;

/// The two `fs_test_client` roles, mirrored from `fixtures/src/fs_test_client.rs`. The numbers live
/// on both sides because one program is started by role, the same shape as every other role here.
const ROLE_SHARE_VICTIM: u64 = 13;
const ROLE_SHARE_ATTACKER: u64 = 14;

/// **Two live clients on one file service; one rewrites the other's name mid-request.**
///
/// The victim stages `fixture::SHARED_VICTIM_NAME` and, were it the only writer, would open exactly
/// that file. The attacker holds the same frame read-write and overwrites the staged name with
/// `fixture::SHARED_USURPER_NAME` before the victim's call reaches the server. The victim reports
/// which file it actually got.
///
/// `SHARED_SUBSTITUTED` is the live defect: the server resolved the attacker's name for the
/// victim's call, and the victim read a file it never named. When milestone 599 gives each client
/// its own channel, the attacker's write lands in its own frame, the victim's own name resolves,
/// and the verdict becomes `SHARED_ISOLATED`. **This assertion must invert then**, which is the
/// point: the day the fix lands, this test fails until someone flips it, the same way a decision
/// gate fails until someone reads the ruling.
#[test_case]
fn a_second_client_substitutes_the_name_the_file_server_resolves() {
    if fs_service::fs_server_image().is_none() {
        crate::testing::skip!(fs_service::NO_FS_SERVER);
    }
    let Some((_readiness, report)) = fs_service::start_shared_frame_witness(
        fs_service::blk_server_image(),
        program("redoxfs_server").expect("no redoxfs_server program in the initrd archive"),
        program("fs_test_client").expect("no fs_test_client program in the initrd archive"),
        ROLE_SHARE_VICTIM,
        ROLE_SHARE_ATTACKER,
    ) else {
        crate::testing::skip!("no RedoxFS disk attached");
    };

    let [verdict, w1, ..] = sched::ipc_recv(report);
    assert_ne!(
        verdict,
        fixture::SHARED_UNEXPECTED,
        "the witness is wired wrong: the victim's open failed or returned neither file's body \
         (errno/aux {w1:#x})",
    );
    assert_eq!(
        verdict,
        fixture::SHARED_ISOLATED,
        "the victim read the attacker's file ({verdict:#x}), so a second client on the service \
         substituted the name the server resolved for the victim's call. Milestone 599 gives each \
         client its own channel window keyed by the endpoint's badge, so this must be \
         SHARED_ISOLATED: the two clients map different frames and neither can reach the other's.",
    );
}
