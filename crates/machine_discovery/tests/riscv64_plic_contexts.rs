//! The PLIC context map, read from three machines' trees, on the host.
//!
//! The VisionFive 2 prep (notes/visionfive2.md): the kernel's `2*hart + 1` S-context formula is
//! QEMU `virt`'s layout, not a law, and the JH7110 breaks it because its disabled S7 contributes
//! only an M context. These tests hold both directions: the JH7110 fixture proves the different
//! answer is read, and the QEMU dumps prove the parsed answer equals what the formula assumed on
//! the machine every merge actually boots, at both hart counts the suite uses.

use machine_discovery::plic::PlicContexts;

const JH7110: &[u8] = include_bytes!("fixtures/jh7110.dtb");
/// The board's ACTUAL U-Boot control DTB PLIC node (bench, 2026-08-21), trimmed. Its
/// `compatible` is "riscv,plic0" only, not "sifive,plic-1.0.0": neither `JH7110` above nor
/// `jh7110-vendor.dtb` had this right, both having modeled the PLIC section on the mainline
/// dtsi rather than measuring the real firmware tree. See the fixture's own header for the full
/// story and notes/visionfive2.md's BUGS section.
const VISIONFIVE2_UBOOT: &[u8] = include_bytes!("fixtures/visionfive2-uboot-control.dtb");
/// The suite's own machine, single-hart, shared with the `dtb` crate's fixtures.
const QEMU_VIRT: &[u8] = include_bytes!("../../dtb/tests/fixtures/qemu-riscv64-virt.dtb");
/// The same machine at `-smp 4`, which is what `script/test` boots; dumped with
/// `qemu-system-riscv64 -machine virt,dumpdtb=... -smp 4`.
const QEMU_VIRT_SMP4: &[u8] = include_bytes!("fixtures/qemu-riscv64-virt-smp4.dtb");

fn tree(bytes: &[u8]) -> dtb::Dtb<'_> {
    dtb::Dtb::from_bytes(bytes).expect("fixture is a valid device tree")
}

/// **Hart h's S context is `2h` on this board.** The S7 contributes only an M context (context 0),
/// so every later context shifts down one from QEMU's layout. This is the number the old
/// `2*hart + 1` formula got wrong on every hart, and the reason the mapping now comes from
/// `interrupts-extended`.
#[test]
fn jh7110_s_contexts_are_2h_not_2h_plus_1() {
    let ctx = PlicContexts::from_device_tree(&tree(JH7110)).expect("the PLIC wiring parses");

    assert_eq!(
        ctx.s_context(0),
        None,
        "the S7 has no S-mode and no S context; a formula would have invented context 1"
    );
    for hart in 1..=4 {
        assert_eq!(
            ctx.s_context(hart),
            Some(2 * hart),
            "hart {hart}'s S context on the JH7110"
        );
    }
    assert_eq!(ctx.len(), 4);
}

/// **On QEMU `virt` the parsed layout equals the old formula**, at both hart counts the suite
/// uses. This is the regression proof for replacing the formula with the tree: same machine, same
/// numbers, only the provenance changes.
#[test]
fn qemu_virt_contexts_match_the_old_formula() {
    let one = PlicContexts::from_device_tree(&tree(QEMU_VIRT)).expect("parses");
    assert_eq!(one.s_context(0), Some(1), "smp1: hart 0's S context is 1");
    assert_eq!(one.len(), 1);

    let four = PlicContexts::from_device_tree(&tree(QEMU_VIRT_SMP4)).expect("parses");
    for hart in 0..4 {
        assert_eq!(
            four.s_context(hart),
            Some(2 * hart + 1),
            "smp4: hart {hart} follows the 2h+1 layout the formula assumed"
        );
    }
    assert_eq!(four.len(), 4);
}

/// A machine without a PLIC (the aarch64 fixture) is an empty map, not an error: the caller falls
/// back to its formula and the machine boots as before.
#[test]
fn a_machine_without_a_plic_is_an_empty_map() {
    const AARCH64: &[u8] = include_bytes!("../../dtb/tests/fixtures/qemu-aarch64-virt-smp4.dtb");
    let ctx = PlicContexts::from_device_tree(&tree(AARCH64)).expect("no PLIC is not an error");
    assert!(ctx.is_empty());
    assert_eq!(ctx.s_context(0), None);
}

/// **The board's real control DTB names its PLIC `riscv,plic0`, not `sifive,plic-1.0.0`.** Found
/// at the bench 2026-08-21: `PlicContexts::from_device_tree` returned an empty map
/// (`ctx.len() == 0`) against a live capture of the VisionFive 2's own U-Boot control DTB, even
/// though `jh7110_s_contexts_are_2h_not_2h_plus_1` above passes against a hand-written fixture
/// that models the same board. The difference was the compatible string the fixture assumed
/// versus the one the real firmware tree carries; this test holds the real one so it cannot
/// regress silently again. The S-context answer is unchanged: hart h's context is still 2h.
#[test]
fn visionfive2_uboot_control_dtb_is_read_despite_the_older_compatible_string() {
    let ctx =
        PlicContexts::from_device_tree(&tree(VISIONFIVE2_UBOOT)).expect("the PLIC wiring parses");

    assert_eq!(
        ctx.s_context(0),
        None,
        "the S7 has no S-mode and no S context; a formula would have invented context 1"
    );
    for hart in 1..=4 {
        assert_eq!(
            ctx.s_context(hart),
            Some(2 * hart),
            "hart {hart}'s S context on the board's real control DTB"
        );
    }
    assert_eq!(ctx.len(), 4);
}

const PLIC_SHAPES: &[u8] = include_bytes!("fixtures/plic-shapes.dtb");

/// **Three ways the two walks can fall out of step, in one tree.**
///
/// The context map is stitched from two lists that align only by tree order: the `riscv,cpu-intc`
/// nodes in one, the PLIC's `interrupts-extended` entries in the other. Every fixture above keeps
/// them aligned by construction, so nothing tested what keeps them aligned.
///
/// * The PLIC here is named `interrupt-controller@c000000`, the spelling the JH7110 uses, and it
///   is declared **before** `/cpus`. Both JH7110 fixtures declare theirs after, so a walk that
///   failed to filter the PLIC out by `compatible` still got the right answer there; here it would
///   take slot zero and shift every hart down one.
/// * `cpu@0`'s controller has no `phandle`, so no entry can name it. It still consumes its hart's
///   slot: a walk that did not count it would hand `cpu@1`'s context to `cpu@0`.
/// * `cpu@10`'s hardware id is 16, which is `MAX_CONTEXT_HARTS` exactly. The tree names an S
///   context for it and the record has no slot to put it in, so the answer is "the tree did not
///   say" rather than a write past the array.
#[test]
fn the_context_walk_stays_aligned_with_the_hart_walk() {
    let ctx = PlicContexts::from_device_tree(&tree(PLIC_SHAPES)).expect("the PLIC wiring parses");

    assert_eq!(
        ctx.s_context(0),
        None,
        "cpu@0's controller has no phandle, so no entry names it",
    );
    assert_eq!(ctx.s_context(1), Some(0), "the first entry is cpu@1's");
    assert_eq!(ctx.s_context(3), Some(2), "the third entry is cpu@3's");
    assert_eq!(
        ctx.s_context(16),
        None,
        "hart 16 is one past the sixteen this record holds",
    );
    assert_eq!(ctx.len(), 2);
    assert!(
        !ctx.is_empty(),
        "two contexts is not none, and every other test here asks only the other way",
    );
}
