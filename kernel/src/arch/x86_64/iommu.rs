//! **The IOMMU, x86_64.** Intel calls it VT-d; AMD calls it AMD-Vi. Neither is built.
//!
//! The role is the same one the aarch64 SMMU and the RISC-V IOMMU fill: a device emits addresses,
//! and something between it and memory decides whether those addresses mean anything. Without it a
//! device driver in userspace is not confined at all, because the device it drives can be told to
//! write anywhere.
//!
//! **What makes this the hardest of the three to bring up** is discovery. The other two find their
//! IOMMU in the device tree, at an address the tree states. VT-d is described by an ACPI table
//! (DMAR), which means the ACPI tables have to be parsed before the IOMMU can be found at all, and
//! this port does not parse them yet. That ordering is the reason this module is a stub rather than
//! a partial implementation: there is nothing useful to do before DMAR is readable.
//!
//! Milestone 87's OptiPlex 7050 has VT-d, which is one of the reasons it was chosen.

/// Is an IOMMU active? Always false: none is built, and saying so plainly is what keeps a
/// confinement claim honest.
pub fn active() -> bool {
    false
}

/// Bring up the IOMMU at `base`.
#[allow(dead_code)]
pub fn init(base: u64) {
    let _ = base;
    unimplemented!("x86_64 VT-d: not built, and blocked on ACPI DMAR parsing (milestone 161)")
}

/// Put the device with requester id `rid` behind the translation domain rooted at `root`.
#[allow(dead_code)]
pub fn attach(rid: u32, root: u64, pasid: u16) {
    let _ = (rid, root, pasid);
    unimplemented!("x86_64 VT-d: not built, and blocked on ACPI DMAR parsing (milestone 161)")
}

/// A translation fault the IOMMU recorded: which device, what went wrong, and at which address.
/// The shape is the arch contract's; the x86 fields it will be filled from are VT-d's fault-record
/// registers, which are a different layout with the same three facts in them.
#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug, Clone, Copy)]
pub struct Fault {
    pub rid: u32,
    pub code: u32,
    pub addr: u64,
}

/// Take the oldest unread fault, if any. Always `None`: with no IOMMU brought up there is nothing
/// recording faults, and an empty queue is the honest answer rather than a panic, because the
/// confinement test's shape is "was there a fault" and a panic would be a different failure.
pub fn take_fault() -> Option<Fault> {
    None
}
