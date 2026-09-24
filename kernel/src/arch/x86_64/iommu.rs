//! **The VT-d driver: Intel's IOMMU, in front of the PCIe bus.**
//!
//! Milestone 16b's role, x86's device (milestone 161, roadmap item 6). The register file, the
//! root/context tables, and register-based invalidation live here; the translation tables
//! themselves are the portable seam's job (`paging::domain` via `crate::iommu`), built in
//! [`paging::x86_64::Vtd`] rather than the CPU's own [`paging::x86_64::Ia32e`] (that module's own
//! doc says why the two must not be confused).
//!
//! # The shape of the hardware, and how it differs from the other two
//!
//! SMMUv3 and the RISC-V IOMMU are both driven almost entirely through memory: a stream/device
//! table the driver writes and queues the driver pushes commands into. **VT-d is driven through
//! registers**, and that is the real architectural difference, not a detail: there is no command
//! queue and no fault queue here, because the *legacy* (non-scalable, non-queued) interface this
//! driver speaks does invalidation with a register write-and-poll (`CCMD_REG`, the IOTLB invalidate
//! register) and reports faults through a small bank of Fault Recording Registers instead of a
//! ring in memory. That interface is what every VT-d unit supports unconditionally; the queued
//! (`ECAP.QI`) and scalable-mode interfaces are supersets this driver does not need yet.
//!
//! Two levels of memory-resident table stand between a register write and a working translation:
//!
//! - The **root table**: one page, 256 entries (one per PCI bus), each either absent or pointing
//!   at a context table.
//! - The **context table**: one page per bus that has an attached device, 256 entries (one per
//!   PCI device/function), each either absent or naming a domain id and a second-level page-table
//!   root, the tables [`crate::iommu::confine`] built.
//!
//! Both tables are allocated **lazily and default-absent**, which is the same default-deny posture
//! `init` gives the SMMUv3 and the RISC-V IOMMU: a bus with no context table, or a context entry
//! with its present bit clear, faults every transaction rather than routing it anywhere.
//!
//! # What is built, against what QEMU's `-device intel-iommu` on `q35` presents
//!
//! One DRHD (`crates/machine_discovery/src/acpi.rs`'s `first_drhd`), translation enable/disable
//! through `GCMD`/`GSTS`, the root and context tables, register-based context-cache and IOTLB
//! invalidation (global granularity only), and fault detection through `FSTS.PPF` and the first
//! Fault Recording Register. Interrupt remapping, queued invalidation, PASID/scalable mode, and
//! more than one DRHD are all real VT-d features this driver does not build; see the BUGS section
//! for what each costs and where the next piece would go.
//!
//! # Default deny
//!
//! `init` zeroes the root table and points `RTADDR_REG` at it before setting `GCMD.TE`, so
//! translation turns on over an all-absent root: every bus faults until its context table exists,
//! and every device on a bus that does have one faults until `attach` writes its entry. There is
//! no window where translation is on and a device is unconstrained by omission, the same property
//! the other two drivers' `init` establishes for their own table shapes.
//!
//! # BUGS
//!
//! - **Exactly one DRHD is brought up.** A machine with more than one VT-d unit (real multi-socket
//!   hardware, or a `q35` machine with more than one `-device intel-iommu`) has devices this driver
//!   never sees, because `machine_discovery::acpi::first_drhd` takes the first entry in the DMAR's
//!   remapping-structure list and stops. Milestone 87's `OptiPlex` 7050 is expected to report one,
//!   which is what makes this the honest first cut rather than a workaround; bringing up more than
//!   one is real future work (walking every DRHD, and routing a device to its owning unit by the
//!   device-scope lists `machine_discovery::acpi::DmarStructures` currently skips).
//! - **No interrupt remapping.** `ECAP.IR` is read and *reported* since milestone 317
//!   (the interrupt-remapping flags, and where MSI confinement actually lives) by
//!   [`interrupt_remapping_available`] and the bring-up line `print_summary` writes, and that
//!   is all: this driver never sets `GCMD.IRE`, never allocates an interrupt-remapping table, and
//!   never programs an entry in one. MSI/MSI-X delivery is unaffected either way (this kernel does
//!   not remap interrupts on any architecture yet), but a future PCI MSI driver on x86 would want
//!   to know this is missing before assuming a remapping table exists to program.
//!
//!   **The unit has been offering it all along, which is a correction and not a feature.**
//!   DECISIONS §86 recorded that interrupt remapping is off in every `x86_64` boot this tree runs,
//!   reasoning from the runner attaching `-device intel-iommu` with no `intremap=on`. Reading
//!   `ECAP` from inside the guest says otherwise: QEMU's `intremap` property defaults to `auto`,
//!   which resolves ON with no in-kernel irqchip, so `ECAP.IR` reads set on the default machine
//!   (`0xf00f4a`) and clear only under an explicit `intremap=off` (`0xf42`). Nothing read the bit,
//!   so nobody noticed. `NIFE_INTREMAP=off` (provisional name) is now the way to reach a machine
//!   without the capability.
//!
//!   **What remains unexercised is the whole of the rest.** Nothing here writes an `IRTE`, forges
//!   an MSI, or proves that a remapped interrupt lands where the table says it should. That is the
//!   confinement claim notes/confinement-claims.md carries as stated nowhere, and
//!   design/roadmap/317-interrupt-remapping-flags.md says what it would take.
//! - **Invalidation is global, never domain- or device-selective.** Every `attach` invalidates the
//!   *entire* context cache and the *entire* IOTLB rather than just the entry that changed, which
//!   is correct (nothing survives that should not) and expensive on a machine with many attached
//!   devices, exactly the same trade the RISC-V driver's `IOTINVAL.VMA` with no address makes.
//! - **`RWBF` (`CAP_REG` bit 4) is honoured but has never been exercised.** QEMU's model does not
//!   set it, so the write-buffer-flush branch in [`invalidate_all`] has run zero times under test;
//!   it is real spec behaviour a physical unit could require, kept rather than assumed away.
//! - **The fault path decodes and clears exactly one Fault Recording Register.** `CAP.NFR` is read
//!   to find where the bank starts, not to size it; QEMU's model reports `NFR = 0` (one register),
//!   so a real unit with more than one is read at the same fixed offset only, and a burst of faults
//!   past that one register overflows silently until `FSTS.PFO` is read (it never is).

use paging::PageFormat;
use paging::x86_64::Vtd;

use crate::arch::mmu::phys_to_virt;
use crate::sync::{IrqSafeMutex, rank};

// --- Register file (offsets from the DRHD's register base; Intel VT-d spec chapter 10, and
// QEMU's hw/i386/intel_iommu_internal.h, which is this driver's ground truth for what q35's
// emulation actually checks). ---
const CAP: u64 = 0x08; // 64-bit
const ECAP: u64 = 0x10; // 64-bit
const GCMD: u64 = 0x18;
const GSTS: u64 = 0x1c;
const RTADDR: u64 = 0x20; // 64-bit
const CCMD: u64 = 0x28; // 64-bit
const FSTS: u64 = 0x34;

// GCMD (write-only) / GSTS (read-only, same bit positions): translation and root-pointer control.
const GCMD_TE: u32 = 1 << 31; // Translation Enable
const GCMD_SRTP: u32 = 1 << 30; // Set Root Table Pointer
const GCMD_WBF: u32 = 1 << 27; // Write Buffer Flush
const GSTS_TES: u32 = 1 << 31;
const GSTS_RTPS: u32 = 1 << 30;
const GSTS_WBFS: u32 = 1 << 27;
// IRES: interrupt remapping is ENABLED. Read only by the test that asserts it is clear; this
// driver has no `GCMD_IRE` constant to pair it with, deliberately, because there is nothing here
// that should be one typo away from turning interrupt remapping on. See this module's BUGS.
#[cfg(test)]
const GSTS_IRES: u32 = 1 << 25;

// CAP fields this driver reads. SAGAW is a bitmap (bit N means "AGAW level N is supported"), not
// an index; bit 2 of the 5-bit field (so bit 10 of the register) is the 48-bit/4-level width
// `Vtd`'s four-level walk needs, the same width `AW_48BIT` below selects in a context entry.
const CAP_SAGAW_48BIT: u64 = 1 << 10;
const CAP_RWBF: u64 = 1 << 4;
const CAP_FRO_SHIFT: u64 = 24; // 10-bit field, in 16-byte units
const CAP_FRO_MASK: u64 = 0x3ff;

// ECAP fields: where the IOTLB invalidate register lives, since unlike the fault-recording bank
// it is not at a fixed offset by specification (QEMU happens to put it at a fixed offset; a real
// unit is not required to).
const ECAP_IRO_SHIFT: u64 = 8; // 10-bit field, in 16-byte units
const ECAP_IRO_MASK: u64 = 0x3ff;

// ECAP.IR (bit 3): the unit supports interrupt remapping. Read to REPORT, never to act on
// (this driver does not set `GCMD.IRE`; see BUGS). It is here because milestone 317 needed the
// machine's own answer to "is interrupt remapping present" to be visible from inside the guest:
// `-device intel-iommu,intremap=on` is a host-side string, and a boot that cannot tell the two
// machines apart cannot claim to have exercised either. `print_summary` is where it surfaces.
const ECAP_IR: u64 = 1 << 3;

// CCMD_REG: context-cache invalidation, register-based (the legacy, non-queued interface every
// VT-d unit supports). ICC is set to start, cleared by hardware on completion; CIRG selects
// global granularity, the only one this driver uses.
const CCMD_ICC: u64 = 1 << 63;
const CCMD_CIRG_GLOBAL: u64 = 1 << 61;

// The IOTLB invalidate register, same shape as CCMD: IVT starts it, hardware clears it, IIRG
// selects granularity.
const IOTLB_IVT: u64 = 1 << 63;
const IOTLB_IIRG_GLOBAL: u64 = 1 << 60;

// FSTS: fault status. PPF is the only bit read; PFO (overflow) is not (see this module's BUGS).
const FSTS_PPF: u32 = 1 << 1;

// Root entry (one page, 256 x 16 bytes, one per PCI bus): lower qword only, in legacy mode.
const ROOT_ENTRY_P: u64 = 1 << 0;
const ROOT_ENTRY_CTP_MASK: u64 = 0x000f_ffff_ffff_f000; // bits 63:12

// Context entry (one page, 256 x 16 bytes, one per device/function on a bus). Lower qword:
// present, translation type (00 = second-level-only, the shape every attach here writes), and the
// second-level page-table pointer. Upper qword: address width and domain id.
const CTX_ENTRY_P: u64 = 1 << 0;
const CTX_TT_MULTI_LEVEL: u64 = 0; // bits 3:2, value 0
const CTX_SLPTPTR_MASK: u64 = 0x000f_ffff_ffff_f000; // bits 63:12
const CTX_AW_48BIT: u64 = 2; // bits 2:0 of the upper qword: 010 = 48-bit AGAW, 4 levels
const CTX_DID_SHIFT: u64 = 8; // bits 23:8 of the upper qword

/// One recorded fault, in portable terms. `code` is the Fault Reason byte the spec defines (e.g.
/// `0x02` is a write past the second-level page table's write permission; `0x07` is no entry for
/// the address at all). Read today only by the confinement test; a production fault handler is
/// future work, the same posture the other two drivers take.
#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug, Clone, Copy)]
pub struct Fault {
    pub rid: u32,
    pub code: u32,
    pub addr: u64,
}

struct Iommu {
    base: u64,
    root: u64,
    /// One context-table root per PCI bus, allocated the first time a device on that bus is
    /// attached. `None` is the default-deny state: a bus with no table here has no root-table
    /// entry pointing at one either, so every device on it faults.
    ctx: [Option<u64>; 256],
    rwbf: bool,
    frcd: u64,
    /// `ECAP.IR`: does this unit offer interrupt remapping? Recorded, never acted on. See the
    /// constant's comment and this module's BUGS for why a read-only field earns its place.
    interrupt_remapping: bool,
}

static IOMMU: IrqSafeMutex<Option<Iommu>> = IrqSafeMutex::new(rank::IOMMU, None);

fn r32(base: u64, off: u64) -> u32 {
    // SAFETY: the DRHD's register file lies inside the direct map (it is ordinary MMIO below the
    // 4 GiB line on every machine this driver has run against), mapped device-typed by
    // `mmu::map_everything`; these reads are side-effect-free registers.
    unsafe { core::ptr::read_volatile(phys_to_virt(base + off) as *const u32) }
}
fn w32(base: u64, off: u64, v: u32) {
    // SAFETY: as above.
    unsafe { core::ptr::write_volatile(phys_to_virt(base + off) as *mut u32, v) }
}
fn r64(base: u64, off: u64) -> u64 {
    // SAFETY: as above; the 64-bit registers are 8-byte aligned.
    unsafe { core::ptr::read_volatile(phys_to_virt(base + off) as *const u64) }
}
fn w64(base: u64, off: u64, v: u64) {
    // SAFETY: as above.
    unsafe { core::ptr::write_volatile(phys_to_virt(base + off) as *mut u64, v) }
}

fn zeroed_page_frame(what: &str) -> u64 {
    let pa = crate::memory::alloc()
        .unwrap_or_else(|| panic!("no frame for the IOMMU {what}"))
        .addr();
    // SAFETY: a fresh frame, reachable through the direct map, owned by this module from here on.
    unsafe {
        core::ptr::write_bytes(
            phys_to_virt(pa) as *mut u8,
            0,
            page_frames::FRAME_SIZE as usize,
        );
    }
    pa
}

/// Wait for `cond(gsts)` to hold, polling `GSTS_REG`. Every global-command write this driver makes
/// is confirmed this way, the same write-then-poll shape `CCMD`/IOTLB invalidation use on their
/// own registers below.
fn wait_gsts(base: u64, what: &str, cond: impl Fn(u32) -> bool) {
    let mut spins = 0u32;
    loop {
        let v = r32(base, GSTS);
        if cond(v) {
            return;
        }
        spins += 1;
        assert!(spins < 1_000_000, "IOMMU {what} never completed ({v:#x})");
    }
}

/// **Bring the IOMMU up: an all-absent root table installed, translation enabled.** From here
/// every device on every bus faults until `attach` writes its context entry.
pub fn init(base: u64) {
    let mut g = IOMMU.lock();
    assert!(g.is_none(), "IOMMU initialized twice");

    let cap = r64(base, CAP);
    assert!(
        cap & CAP_SAGAW_48BIT != 0,
        "the IOMMU does not support 48-bit/4-level second-level translation (cap {cap:#x}); \
         Vtd's second-level format depends on it"
    );
    let rwbf = cap & CAP_RWBF != 0;
    let frcd = ((cap >> CAP_FRO_SHIFT) & CAP_FRO_MASK) << 4;
    let interrupt_remapping = r64(base, ECAP) & ECAP_IR != 0;

    let root = zeroed_page_frame("root table");

    w64(base, RTADDR, root);
    w32(base, GCMD, GCMD_SRTP);
    wait_gsts(base, "set root table pointer", |v| v & GSTS_RTPS != 0);

    w32(base, GCMD, GCMD_TE);
    wait_gsts(base, "translation enable", |v| v & GSTS_TES != 0);

    *g = Some(Iommu {
        base,
        root,
        ctx: [None; 256],
        rwbf,
        frcd,
        interrupt_remapping,
    });
}

/// **Does this machine's VT-d unit offer interrupt remapping (`ECAP.IR`)?** `None` when there is
/// no unit at all, which is a different answer from "a unit that says no".
///
/// Nothing in this kernel remaps an interrupt. This exists so the question is *askable* from
/// inside the guest, which is what milestone 317 is for: without it, turning
/// `-device intel-iommu,intremap=on` on and watching the suite stay green proves only that the
/// suite does not care. See design/roadmap/317-interrupt-remapping-flags.md.
// Two callers: `print_summary` (which a bench boot skips, the same treatment that function already
// carries) and this module's own test.
#[cfg_attr(feature = "bench", allow(dead_code))]
pub fn interrupt_remapping_available() -> Option<bool> {
    IOMMU.lock().as_ref().map(|s| s.interrupt_remapping)
}

/// Is the IOMMU up? The portable seam asks this to decide whether attaching is possible.
/// **This machine's IOMMU, for the machine description** (milestone 268).
///
/// One of the eight questions the description answers on every architecture. The vocabulary is this
/// architecture's, because that is what parity means here: the same question, answered in the terms
/// of the hardware that answers it. A machine with no IOMMU says so plainly rather than printing a
/// blank, because a blank is indistinguishable from a line nobody wrote.
// The machine description is the only caller, and it is
// `#[cfg(not(any(test, feature = "bench")))]`: a test boot exits through semihosting and a bench
// boot diverges into `bench::run`, so neither reads a bring-up transcript. Same treatment
// `memory::print_summary` already carries, and for the same reason.
#[cfg_attr(any(test, feature = "bench"), allow(dead_code))]
pub fn print_summary() {
    // Asked before the lock below, not through `s`, because `IOMMU` is not reentrant.
    let remapping = interrupt_remapping_available();
    match IOMMU.lock().as_ref() {
        Some(s) => crate::println!(
            "  iommu           : VT-d drhd at {:#018x}, root table default-deny, translating, \
             interrupt remapping {} (unused)",
            s.base,
            if remapping == Some(true) {
                "offered"
            } else {
                "absent"
            },
        ),
        None => crate::println!(
            "  iommu           : none (this machine's ACPI names no DMAR, or it names no DRHD)",
        ),
    }
}

pub fn is_active() -> bool {
    IOMMU.lock().is_some()
}

/// Register-based, global invalidation: the context cache first (a stale context entry would
/// still point translation at the previous domain), then the IOTLB, each a write-and-poll on its
/// own register. Both use global granularity; see this module's BUGS for the cost.
fn invalidate_all(s: &Iommu) {
    if s.rwbf {
        w32(s.base, GCMD, GCMD_WBF);
        wait_gsts(s.base, "write buffer flush", |v| v & GSTS_WBFS != 0);
    }

    w64(s.base, CCMD, CCMD_ICC | CCMD_CIRG_GLOBAL);
    let mut spins = 0u32;
    while r64(s.base, CCMD) & CCMD_ICC != 0 {
        spins += 1;
        assert!(spins < 1_000_000, "IOMMU context-cache invalidate hung");
    }

    let ecap = r64(s.base, ECAP);
    let iotlb_reg = (((ecap >> ECAP_IRO_SHIFT) & ECAP_IRO_MASK) << 4) + 8;
    w64(s.base, iotlb_reg, IOTLB_IVT | IOTLB_IIRG_GLOBAL);
    let mut spins = 0u32;
    while r64(s.base, iotlb_reg) & IOTLB_IVT != 0 {
        spins += 1;
        assert!(spins < 1_000_000, "IOMMU IOTLB invalidate hung");
    }
}

/// Point device `rid` (the PCIe requester id: bus in bits 15:8, device/function in bits 7:0) at
/// the domain rooted at `root` (a [`Vtd`]-format table the portable seam built), tagged `did`
/// (VT-d's domain id, the same role an ASID or a PSCID plays on the other two architectures), then
/// invalidate so the IOMMU drops anything cached for the previous state of this device.
pub fn attach(rid: u32, root: u64, did: u16) {
    let mut g = IOMMU.lock();
    let s = g.as_mut().expect("IOMMU attach before init");

    let bus = (rid >> 8) as usize & 0xff;
    let devfn = (rid & 0xff) as usize;

    let ctp = match s.ctx[bus] {
        Some(ctp) => ctp,
        None => {
            let ctp = zeroed_page_frame("context table");
            // Publish the (still all-absent) context table before the root entry that makes it
            // reachable, so the IOMMU can never walk to a root entry whose context table isn't
            // there yet.
            crate::arch::dma_wmb();
            let root_entry = phys_to_virt(s.root + bus as u64 * 16) as *mut u64;
            // SAFETY: `s.root` is a kernel-owned page-aligned frame; `bus` is masked to 0..256,
            // which is exactly the 256 entries a one-page root table holds.
            unsafe {
                core::ptr::write_volatile(root_entry, (ctp & ROOT_ENTRY_CTP_MASK) | ROOT_ENTRY_P);
                core::ptr::write_volatile(root_entry.add(1), 0); // upper qword: reserved, legacy mode
            }
            s.ctx[bus] = Some(ctp);
            ctp
        }
    };

    // The context entry, written back to front: the second-level root and the domain id first,
    // the present bit last, with a barrier between, so the IOMMU can never observe a present
    // entry with a stale second-level root. Same ordering discipline the RISC-V driver's
    // `attach` uses for its device context.
    let ctx_entry = phys_to_virt(ctp + devfn as u64 * 16) as *mut u64;
    let hi = CTX_AW_48BIT | ((did as u64) << CTX_DID_SHIFT);
    let lo = (root & CTX_SLPTPTR_MASK) | CTX_TT_MULTI_LEVEL;
    // SAFETY: `ctp` is a kernel-owned page-aligned frame; `devfn` is masked to 0..256, which is
    // exactly the 256 entries a one-page context table holds.
    unsafe {
        core::ptr::write_volatile(ctx_entry.add(1), hi);
        core::ptr::write_volatile(ctx_entry, lo);
    }
    crate::arch::dma_wmb();
    // SAFETY: as above.
    unsafe {
        core::ptr::write_volatile(ctx_entry, lo | CTX_ENTRY_P);
    }
    crate::arch::dma_wmb();

    invalidate_all(s);
}

/// Pop one fault, if any: `FSTS.PPF` says at least one Fault Recording Register holds an
/// unprocessed record, and (with `CAP.NFR` reporting one register on every machine this driver has
/// run against) the first is the only one there is. The confinement test drains this to prove a
/// DMA escape was stopped by the hardware, not merely absent.
#[cfg_attr(not(test), allow(dead_code))]
pub fn take_fault() -> Option<Fault> {
    let g = IOMMU.lock();
    let s = g.as_ref()?;

    if r32(s.base, FSTS) & FSTS_PPF == 0 {
        return None;
    }

    let lo = r64(s.base, s.frcd);
    let hi = r64(s.base, s.frcd + 8);
    if hi & (1 << 63) == 0 {
        // PPF was set but this record's own F bit is not: nothing to report from the one
        // register this driver reads. See this module's BUGS on why only one is ever read.
        return None;
    }

    let fault = Fault {
        rid: (hi & 0xffff) as u32,
        code: ((hi >> 32) & 0xff) as u32,
        addr: lo & !0xfff,
    };

    // Write-1-to-clear: writing back exactly what was read clears the F bit (bit 63 of `hi`) and
    // leaves every other field, which the hardware overwrites on the next fault anyway.
    w64(s.base, s.frcd + 8, hi);

    Some(fault)
}

// A compile-time check that this module and `Vtd` agree on the level count `CTX_AW_48BIT`
// promises the hardware: four levels, the same the context entry's AW field selects.
const _: () = assert!(Vtd::LEVELS == 4);

#[cfg(test)]
mod tests {
    //! Milestone 317's half of the interrupt-remapping question, which is the half a boot can
    //! answer. The other half (does a remapped interrupt land where an `IRTE` says it should)
    //! needs a driver that programs one, and this tree has none.

    use super::*;

    /// **The unit's interrupt-remapping capability is reported, and remapping is still off.**
    ///
    /// Two assertions, and the second is the one with teeth. The first says the machine answered
    /// at all: a `None` here on a runner that always attaches `-device intel-iommu` would mean the
    /// DRHD was never found, which every other VT-d test would also fail on, so it is a guard
    /// rather than a claim.
    ///
    /// The second states a limit, the shape notes/confinement-claims.md asks for: whatever
    /// `ECAP.IR` says, `GSTS.IRES` is clear, because this driver never sets `GCMD.IRE`. **That is
    /// what keeps `NIFE_INTREMAP` honest.** Booting a machine that offers remapping and watching
    /// the suite stay green would otherwise prove nothing; this fails the day somebody enables
    /// remapping without also retiring the claim in
    /// design/roadmap/317-interrupt-remapping-flags.md that nothing in this kernel remaps an
    /// interrupt.
    ///
    /// It runs identically with the flag and without it, on purpose. A test that only ran under
    /// the flag would be a test nobody runs.
    #[test_case]
    fn interrupt_remapping_is_reported_and_never_enabled() {
        let Some(offered) = interrupt_remapping_available() else {
            crate::testing::skip!("this machine's ACPI names no DMAR, so there is no unit to ask");
        };

        let base = IOMMU.lock().as_ref().expect("a unit answered above").base;
        let gsts = r32(base, GSTS);
        assert_eq!(
            gsts & GSTS_IRES,
            0,
            "GSTS.IRES is set: something enabled interrupt remapping (ECAP.IR was {offered}), \
             and the claim that this kernel never remaps an interrupt is now false",
        );
    }
}
