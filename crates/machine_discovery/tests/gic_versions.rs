//! The GIC, found by its binding and cross-checked against the hardware's own words, on the host.
//!
//! Milestone 227. The three trees are QEMU 11.1.1's own `dumpdtb` output for the machines this
//! kernel is run on: `gic-version=2` under TCG (the default runner, reused from the
//! `device_tree_blob` fixtures rather than copied), `gic-version=3` under TCG, and
//! `gic-version=3` under HVF, which is the configuration this milestone exists to make work.
//! Regenerate the two GICv3 fixtures with:
//!
//!     qemu-system-aarch64 -machine virt,gic-version=3,iommu=smmuv3,dumpdtb=f.dtb -cpu cortex-a72 -smp 4
//!     qemu-system-aarch64 -machine virt,accel=hvf,gic-version=3,iommu=smmuv3,dumpdtb=f.dtb -cpu host -smp 4
//!     dtc -I dtb -O dtb -o crates/machine_discovery/tests/fixtures/<name>.dtb f.dtb

use device_tree_blob::{DeviceTreeBlob, Region};
use machine_discovery::gic::{self, Gic, Mismatch, Refusal};

const QEMU_GICV2: &[u8] =
    include_bytes!("../../device_tree_blob/tests/fixtures/qemu-aarch64-virt-smp4.dtb");
const QEMU_GICV3: &[u8] = include_bytes!("fixtures/qemu-aarch64-virt-gicv3.dtb");
const QEMU_GICV3_HVF: &[u8] = include_bytes!("fixtures/qemu-aarch64-virt-gicv3-hvf.dtb");
const QEMU_RISCV: &[u8] = include_bytes!("fixtures/qemu-riscv64-virt-smp4.dtb");

fn tree(bytes: &[u8]) -> DeviceTreeBlob<'_> {
    DeviceTreeBlob::from_bytes(bytes).expect("fixture is a valid device tree")
}

const GICD: Region = Region {
    start: 0x0800_0000,
    size: 0x1_0000,
};

/// **The default runner's machine is a GICv2**, and the second block is a CPU interface.
#[test]
fn qemu_gic_version_2_is_a_gicv2_with_a_cpu_interface() {
    let found = gic::discover(&tree(QEMU_GICV2)).unwrap();
    assert_eq!(
        found,
        Some(Gic::V2 {
            distributor: GICD,
            cpu_interface: Region {
                start: 0x0801_0000,
                size: 0x1_0000
            },
        })
    );
}

/// **The same address pair the old name-prefix read took, now in the role the binding gives it.**
/// This is the fixture milestone 222's failure came from: a second block of `0xf60000` bytes is a
/// redistributor array, not a 64 KiB CPU interface.
#[test]
fn qemu_gic_version_3_is_a_gicv3_with_redistributors() {
    let found = gic::discover(&tree(QEMU_GICV3)).unwrap();
    assert_eq!(
        found,
        Some(Gic::V3 {
            distributor: GICD,
            redistributors: Region {
                start: 0x080a_0000,
                size: 0xf6_0000
            },
        })
    );
}

/// HVF's tree names the same GICv3 at the same addresses; what differs is the MSI child (`v2m`
/// rather than `its`), which this module does not read. Pinned so that a QEMU which moved the
/// redistributors under HVF would be noticed here rather than on a boot.
#[test]
fn qemu_hvf_gic_version_3_matches_tcg() {
    assert_eq!(
        gic::discover(&tree(QEMU_GICV3_HVF)).unwrap(),
        gic::discover(&tree(QEMU_GICV3)).unwrap(),
    );
}

/// A tree with no GIC and no `intc@` node is an honest `None`, not a refusal: RISC-V's `virt` tree.
#[test]
fn a_machine_without_a_gic_says_so() {
    assert_eq!(gic::discover(&tree(QEMU_RISCV)).unwrap(), None);
}

// --- the cross-check: what the silicon says ---

/// The QEMU GICv3's `GICD_PIDR2`: `ArchRev` 3 in bits 7:4. Read off the HVF boot's refusal line on
/// 2026-09-19, where it printed as `0x30`.
const GICD_PIDR2_V3: u32 = 0x30;

/// A GICv3 whose distributor agrees is believed, and so is a GICv4 (`ArchRev` 4).
#[test]
fn a_gicv3_the_hardware_agrees_with_is_confirmed() {
    let gic = gic::discover(&tree(QEMU_GICV3)).unwrap().unwrap();
    assert_eq!(gic.confirm(GICD_PIDR2_V3), Ok(()));
    assert_eq!(gic.confirm(0x40), Ok(()));
}

/// **The milestone 222 shape, caught.** A GICv2 claim whose "CPU interface" is really a
/// redistributor frame reads `GICC_IIDR` from a reserved offset, which is zero, so the claim is
/// refused instead of believed. The record is built by hand, with the GICv3 fixture's
/// redistributor array in the CPU interface's place, and handed the zero that offset answers with.
#[test]
fn a_gicv2_claim_over_a_redistributor_is_refused() {
    let wrong = Gic::V2 {
        distributor: GICD,
        cpu_interface: Region {
            start: 0x080a_0000,
            size: 0xf6_0000,
        },
    };
    assert_eq!(
        wrong.confirm(0),
        Err(Mismatch {
            claimed: 2,
            field: 0
        })
    );
}

/// QEMU's GICv2 model and a GIC-400 both report `ArchitectureVersion` 2 in `GICC_IIDR`
/// (QEMU `hw/intc/arm_gic.c`: `(s->revision << 16) | 0x43b`; GIC-400 TRM: `0x0202143B`).
#[test]
fn a_gicv2_the_hardware_agrees_with_is_confirmed() {
    let gic = gic::discover(&tree(QEMU_GICV2)).unwrap().unwrap();
    for iidr in [0x0002_043b, 0x0202_143b] {
        assert_eq!(gic.confirm(iidr), Ok(()), "GICC_IIDR {iidr:#x}");
    }
}

/// A GICv3 claim whose distributor does not say GICv3 (zero: nothing there, or a GICv2
/// distributor, whose `0xffe8` is not an ID register) is refused.
#[test]
fn a_gicv3_claim_over_the_wrong_block_is_refused() {
    let gic = gic::discover(&tree(QEMU_GICV3)).unwrap().unwrap();
    assert_eq!(
        gic.confirm(0),
        Err(Mismatch {
            claimed: 3,
            field: 0
        })
    );
}

/// **An `intc@` node with a binding this kernel does not drive is a refusal that names it**, not
/// a guess. QEMU emits no such tree, so this one is written by hand ([`minimal_tree`]); the
/// binding is Apple's AIC, the controller an Apple core has natively and the one this kernel would
/// meet if it ever booted on one without a hypervisor in between.
#[test]
fn an_unknown_interrupt_controller_is_refused_by_name() {
    let blob = minimal_tree(b"intc@8000000", b"apple,aic\0");
    match gic::discover(&tree(&blob)) {
        Err(Refusal::UnknownController { compatible }) => {
            assert_eq!(compatible, b"apple,aic\0");
        }
        other => panic!("expected UnknownController, got {other:?}"),
    }
}

/// The smallest flattened device tree with one node carrying `compatible` and a two-block `reg`.
/// Written out by hand because the crate has no tree writer, and this is the only test that needs
/// a binding QEMU will not emit.
fn minimal_tree(node: &[u8], compatible: &[u8]) -> Vec<u8> {
    const BEGIN: u32 = 1;
    const END_NODE: u32 = 2;
    const PROP: u32 = 3;
    const END: u32 = 9;

    let strings: &[u8] = b"compatible\0reg\0#address-cells\0#size-cells\0";
    let off_compatible = 0u32;
    let off_reg = 11u32;
    let off_acells = 15u32;
    let off_scells = 30u32;

    let mut st = Vec::new();
    let push = |v: &mut Vec<u8>, w: u32| v.extend_from_slice(&w.to_be_bytes());
    let pad = |v: &mut Vec<u8>| {
        while !v.len().is_multiple_of(4) {
            v.push(0);
        }
    };

    push(&mut st, BEGIN);
    st.push(0); // root name ""
    pad(&mut st);
    for (off, val) in [(off_acells, 2u32), (off_scells, 2u32)] {
        push(&mut st, PROP);
        push(&mut st, 4);
        push(&mut st, off);
        push(&mut st, val);
    }
    push(&mut st, BEGIN);
    st.extend_from_slice(node);
    st.push(0);
    pad(&mut st);
    push(&mut st, PROP);
    push(&mut st, compatible.len() as u32);
    push(&mut st, off_compatible);
    st.extend_from_slice(compatible);
    pad(&mut st);
    push(&mut st, PROP);
    push(&mut st, 32);
    push(&mut st, off_reg);
    for w in [0u32, 0x0800_0000, 0, 0x1_0000, 0, 0x0801_0000, 0, 0x1_0000] {
        push(&mut st, w);
    }
    push(&mut st, END_NODE);
    push(&mut st, END_NODE);
    push(&mut st, END);

    let header_len = 40u32;
    let rsvmap_len = 16u32; // one terminating (0, 0) entry
    let off_rsvmap = header_len;
    let off_struct = off_rsvmap + rsvmap_len;
    let off_strings = off_struct + st.len() as u32;
    let total = off_strings + strings.len() as u32;

    let mut out = Vec::new();
    for w in [
        0xd00d_feed,
        total,
        off_struct,
        off_strings,
        off_rsvmap,
        17, // version
        16, // last compatible version
        0,  // boot cpu
        strings.len() as u32,
        st.len() as u32,
    ] {
        push(&mut out, w);
    }
    out.extend_from_slice(&[0u8; 16]);
    out.extend_from_slice(&st);
    out.extend_from_slice(strings);
    out
}
