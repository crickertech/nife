use filesystem_protocol::dir;
use filesystem_protocol::fixture::twodir as t;

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
            a: (filesystem_protocol::fixture::tree::SUB, dir::READ),
            b: (filesystem_protocol::fixture::tree::OTHER, dir::READ),
            role: 10, // ROLE_TWO_DIR
            arg: 0,
            stack_pages: 0,
        },
    ) else {
        // No print, no skip!() here: this is a helper, and `skip!()` returns from the function it
        // is written in, which would leave the test running. `None` is the fixture's absence
        // travelling to the `#[test_case]`, which is the only place that can honestly skip.
        return None;
    };
    // Both caretakers' own handshakes happened inside `start_granted_two_dirs`, before this
    // witness existed: the same ordering fix `fs_service::wait_for_caretaker` records, run twice.
    let [tag, verdict, ..] = sched::ipc_recv(report);
    assert_eq!(
        tag,
        filesystem_protocol::fixture::VERDICT,
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
    if fs_service::fs_server_image().is_none() {
        crate::testing::skip!(fs_service::NO_FS_SERVER);
    }
    let Some(v) = two_dir_witness() else {
        crate::testing::skip!("no RedoxFS disk attached");
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

/// The `swish` binary's two-tree role (`components/src/swish.rs`'s `ROLE_TWO_TREES`).
const ROLE_TWO_TREES: u64 = 6;

/// Wire the **real shell** into the same two caretakers [`two_dir_witness`] uses, and run its
/// two-tree script. `None` when no RedoxFS disk is attached.
///
/// The rights are read-only on purpose: the script moves, lists, opens and plans, and writes
/// nothing, so it can run against an image other tests have already written to and leave it as it
/// found it.
fn two_tree_shell() -> Option<u64> {
    let rights = dir::ENUMERATE | dir::READ | dir::DESCEND;
    let report = fs_service::start_granted_two_dirs(
        blk_server_image(),
        program("redoxfs_server").expect("no redoxfs_server program in the initrd archive"),
        program("fs_subtree_caretaker")
            .expect("no fs_subtree_caretaker program in the initrd archive"),
        program("swish").expect("no swish program in the initrd archive"),
        fs_service::TwoDirGrant {
            a: (filesystem_protocol::fixture::tree::SUB, rights),
            b: (filesystem_protocol::fixture::tree::OTHER, rights),
            role: ROLE_TWO_TREES,
            arg: filesystem_protocol::grant::spec(0, rights),
            // **Seven, measured.** This script plans an `rm`, and the planner's own frames in a
            // debug build (`plan_against_with` alone is over 11 KiB) are the deepest chain it
            // reaches, about 26 KiB with the witness above them. Four overflowed there, presenting
            // as a data abort on the shell's own `sp` and then as the lost-wakeup watchdog, and six
            // would leave under 2 KiB; the seventh is headroom, one short of this wiring's ceiling.
            stack_pages: 7,
        },
    )?;
    let [tag, verdict, ..] = sched::ipc_recv(report);
    assert_eq!(
        tag,
        filesystem_protocol::fixture::VERDICT,
        "the two-tree shell's report is not a verdict word",
    );
    Some(verdict)
}

/// **The live half of milestone 154 (a process that holds two directory capabilities): the shell
/// itself holds two trees**, moving as §126 (a real, single, moving cwd) decided. The real
/// `swish` builtins start at `/a`, move to `/b` by label, list and open in each, refuse `..` at
/// `b`'s root, `/a/../b` and an unlabeled `/secret` without moving, carry a `<` planned into `b`
/// through to an open in `b`, refuse an `rm` into `b` at delivery, bind a name into `b`, and come
/// home to `/a`.
///
/// An exact set, [`a_process_holding_two_directory_capabilities_reaches_both_and_crosses_neither`]'s
/// shape: the reaching bits are the controls without which every refusal would be equally
/// consistent with a shell that reaches nothing.
#[test_case]
fn a_shell_holding_two_trees_moves_between_them_and_crosses_neither() {
    use filesystem_protocol::fixture::twotrees as tt;
    if fs_service::fs_server_image().is_none() {
        crate::testing::skip!(fs_service::NO_FS_SERVER);
    }
    let Some(v) = two_tree_shell() else {
        crate::testing::skip!("no RedoxFS disk attached");
    };
    let want = tt::PWD_STARTS_AT_A
        | tt::LISTED_A
        | tt::MOVED_TO_B
        | tt::LISTED_B
        | tt::OPENED_RELATIVE_IN_B
        | tt::OPENED_A_FROM_B
        | tt::CLAMPED_AT_B
        | tt::DOT_DOT_REFUSED
        | tt::UNLABELED_REFUSED
        | tt::REDIRECTED_FROM_B
        | tt::HOME_IS_A
        | tt::BOUND_INTO_B
        | tt::RM_IN_B_REFUSED;
    assert_eq!(
        v & !want,
        0,
        "the two-tree shell reported something it must not have (verdict {v:#x}): bit 16 is a \
         refused move that moved, 17 a crossing that was not refused, 18 a name reached through \
         the wrong tree's label, 19 nothing reached at all",
    );
    assert_eq!(
        v,
        want,
        "the two-tree shell could not do something it holds the authority for (verdict {v:#x}, \
         missing {:#x}), so its refusals prove less than they claim",
        want & !v,
    );
}
