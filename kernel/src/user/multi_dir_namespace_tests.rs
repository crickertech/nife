use filesystem_proto::dir;
use filesystem_proto::fixture::twodir as t;

use super::*;
use crate::sched;

/// The binary carrying the block server's role. One `cfg`, in `fs_service`, because the boot path
/// needs the same answer this module's test does.
pub(super) fn blk_server_image() -> &'static [u8] {
    fs_service::blk_server_image()
}

/// Wire two `fs_subtree_caretaker`s for **one** process (grant A over the fixture's `sub`, grant B
/// over its sibling `other`), run the two-directory witness against them, and return its verdict
/// bitmap.
///
/// `None` when no RedoxFS disk is attached (nothing to test; do not fail), the same convention
/// [`dir_capability_tests`] uses.
fn two_dir_witness() -> Option<u64> {
    let Some(report) = fs_service::start_granted_two_dirs(
        blk_server_image(),
        program("redoxfs_server").expect("no redoxfs_server program in the initrd archive"),
        program("fs_subtree_caretaker")
            .expect("no fs_subtree_caretaker program in the initrd archive"),
        program("fs_test_client").expect("no fs_test_client program in the initrd archive"),
        fs_service::TwoDirGrant {
            a: (filesystem_proto::fixture::tree::SUB, dir::READ),
            b: (filesystem_proto::fixture::tree::OTHER, dir::READ),
            role: 10, // ROLE_TWO_DIR
            arg: 0,
            stack_pages: 0,
        },
    ) else {
        crate::println!("    (no RedoxFS disk attached; skipping)");
        return None;
    };
    // Both caretakers' own handshakes happened inside `start_granted_two_dirs`, before this
    // witness existed: the same ordering fix `fs_service::wait_for_caretaker` records, run twice.
    let [tag, verdict, ..] = sched::ipc_recv(report);
    assert_eq!(
        tag,
        filesystem_proto::fixture::VERDICT,
        "the two-directory witness's report is not a verdict word",
    );
    Some(verdict)
}

/// **Milestone 154's whole deliverable, in one test**: one process holds two directory
/// capabilities, `/a/x` and `/b/y` (here `/a/inner` and `/b/secret`) both resolve to the grant
/// their own label names, `/a/../b` is refused before it ever reaches a caretaker, and neither
/// caretaker's tree is reachable through the other's endpoint.
///
/// The bitmap check is [`dir_capability_tests`]'s shape: an *exact* set rather than "something
/// happened", so a witness that reached nothing and one that reached everything both fail, and
/// [`t::OPENED_A`] / [`t::OPENED_B`] are the controls without which every refusal below would be
/// equally consistent with two caretakers that answer no to everything.
#[test_case]
fn a_process_holding_two_directory_capabilities_reaches_both_and_crosses_neither() {
    let Some(v) = two_dir_witness() else {
        return;
    };
    let want = t::OPENED_A | t::OPENED_B;
    let leaked = v & !want;
    assert_eq!(
        leaked, 0,
        "the two-directory witness reported an escape it must not have (verdict {v:#x}): a set \
         bit above OPENED_A|OPENED_B means it crossed from one grant into the other, or that \
         TwoRoots::resolve let `/a/../b` through",
    );
    assert_eq!(
        v, want,
        "the witness could not reach one of its own two grants (verdict {v:#x}), so its \
         refusals prove nothing: a capability that reaches nothing is trivially confined",
    );
}
