//! The `x86_64` four-level page-table format: 4 KiB pages, 48-bit virtual addresses.
//!
//! ```text
//!  47      39 38      30 29      21 20      12 11         0
//! ┌──────────┬──────────┬──────────┬──────────┬────────────┐
//! │  PML4    │  PDPT    │   PD     │   PT     │   offset   │
//! │  9 bits  │  9 bits  │  9 bits  │  9 bits  │  12 bits   │
//! └──────────┴──────────┴──────────┴──────────┴────────────┘
//! ```
//!
//! **The type is called `Ia32e` rather than `X86_64` on purpose.** "IA-32e paging" is Intel's own
//! name for this mode (SDM volume 3, chapter 4), so it is a term a reader already knows from
//! outside this tree, which is the naming tenet's protected class. The mechanical reason is smaller
//! and also real: `X86_64` is not camel case and `non_camel_case_types` is on. The module keeps the
//! architecture's name, matching [`aarch64`](crate::aarch64) beside it. Name provisional.
//!
//! # What this format does differently from the other two, and it is one thing
//!
//! **Permissions are inherited down the walk, not stated at the leaf.** On aarch64 and Sv39 an
//! intermediate entry carries no rights and the leaf is the single source of truth, which is why
//! [`PageFormat::table_entry`] takes no flags. x86 is the opposite: the effective permission for an
//! access is the **AND** of every level's `U/S`, `R/W` and (for NX, the OR of) `XD` bits. A leaf
//! that says "user, writable" under a PML4 entry that says "supervisor, read-only" is
//! supervisor-only and read-only.
//!
//! So this implementation makes intermediate entries **maximally permissive** (present, writable,
//! user, executable) and lets the leaf decide, which reproduces the other two formats' semantics
//! exactly and is what makes the shared [`Mapper`](crate::Mapper) walk correct here without knowing
//! any of this. That is a deliberate trade and it has a cost worth naming: the hierarchical bits are
//! a real x86 mechanism for revoking a whole subtree in one store, and this format gives it up in
//! exchange for one meaning of "what does this mapping grant" across three architectures. A future
//! subtree-revocation optimisation would have to reintroduce it, and would then have to explain
//! itself to `translate`, which reads the leaf alone.
//!
//! # NX depends on a bit outside the page tables
//!
//! Bit 63 means "no execute" only while `IA32_EFER.NXE` is set; with NXE clear it is a **reserved**
//! bit and setting it makes the entry faulting rather than non-executable. The kernel's boot
//! trampoline sets NXE before it builds any table (`arch/x86_64/boot.s`), which is the ordering
//! that makes this encoding safe. Nothing in this crate can check it, which is why it is said here.

use crate::{
    CAP_DEVICE, CAP_GLOBAL, CAP_KERNEL_EXEC, CAP_USER, CAP_USER_EXEC, CAP_WRITE, Flags, PageFormat,
};

const P: u64 = 1 << 0; // Present
const RW: u64 = 1 << 1; // Read/Write (clear = read-only)
const US: u64 = 1 << 2; // User/Supervisor (set = user may access)
const PWT: u64 = 1 << 3; // Page-level Write-Through
const PCD: u64 = 1 << 4; // Page-level Cache Disable
const A: u64 = 1 << 5; // Accessed
const D: u64 = 1 << 6; // Dirty
const G: u64 = 1 << 8; // Global (ignored unless CR4.PGE)
const XD: u64 = 1 << 63; // eXecute Disable, and only while IA32_EFER.NXE is set

/// Bit 9, the first of the three "available to software" bits (11:9) in every entry. Used to record
/// that a leaf was created as **kernel**-executable, which the hardware itself cannot express: x86
/// has one execute permission per entry (the absence of XD) and decides which ring it applies to
/// from `U/S`, exactly as Sv39 does with its `U` bit. That much round-trips without help. What does
/// not is the difference between "kernel data" and "kernel code", both of which are supervisor-only
/// and only one of which should be executable, so the encoder needs to remember which it was told.
const SW_KERNEL_EXEC: u64 = 1 << 9;

/// Bit 10, the second software bit: this leaf is device memory. x86 expresses uncacheable memory
/// through PWT/PCD (and the PAT), which [`Ia32e::attrs`] does set, but those bits are also what a
/// cacheability policy would use for ordinary memory, so they are not a reliable read-back of the
/// portable [`Flags::is_device`] intent. Same role as Sv39's `SW_DEVICE`.
const SW_DEVICE: u64 = 1 << 10;

/// The physical address occupies bits [51:12]: 40 bits. Bits 62:52 are reserved or software-use,
/// and bit 63 is XD, so the mask must not be widened without checking both ends.
const ADDR_MASK: u64 = 0x000f_ffff_ffff_f000;

/// The `x86_64` four-level (IA-32e) page-table format.
pub struct Ia32e;

impl Ia32e {
    /// Encode the permission/attribute bits (not the address or P) for a leaf with `flags`.
    ///
    /// A and D are set eagerly for the same reason Sv39 does it: hardware that has to set them
    /// itself takes a fault on the first touch of every page, and there is nothing here that wants
    /// to know about first touches. D is set only where a write is possible at all.
    const fn attrs(flags: Flags) -> u64 {
        let mut bits = A;
        if flags.is_writable() {
            bits |= RW | D;
        }
        if flags.is_user_accessible() {
            bits |= US;
        }
        if flags.is_global() {
            bits |= G;
        }
        if flags.is_device() {
            // Uncacheable, write-through: the conservative PAT-0 encoding that means "strong
            // uncacheable" under the reset-time PAT this kernel does not reprogram. A device
            // register that gets cached reads back a value the device never produced.
            bits |= PCD | PWT | SW_DEVICE;
        }
        // One execute permission, applying at whichever ring `U/S` names. A page executable by
        // nobody gets XD; anything else leaves it clear and relies on U/S, which is exactly Sv39's
        // arrangement of the same three facts.
        if flags.is_kernel_executable() {
            bits |= SW_KERNEL_EXEC;
        } else if !flags.is_user_executable() {
            bits |= XD;
        }
        bits
    }
}

impl PageFormat for Ia32e {
    const LEVELS: usize = 4;

    /// Bit 47. An `x86_64` virtual address is canonical when bits 63:47 are all equal, which is the
    /// same shape as Sv39's sign extension from bit 38, one bit position over.
    const SPLIT_SHIFT: u32 = 47;

    fn is_present(entry: u64) -> bool {
        entry & P != 0
    }

    fn entry_pa(entry: u64) -> u64 {
        entry & ADDR_MASK
    }

    fn table_entry(pa: u64) -> u64 {
        // Maximally permissive, because x86 ANDs the levels together and the leaf must be able to
        // grant anything. See this module's header for what that trades away.
        (pa & ADDR_MASK) | P | RW | US
    }

    fn leaf_entry(pa: u64, flags: Flags) -> u64 {
        (pa & ADDR_MASK) | P | Self::attrs(flags)
    }

    fn leaf_flags(entry: u64) -> Flags {
        let mut caps = 0;
        if entry & RW != 0 {
            caps |= CAP_WRITE;
        }
        if entry & US != 0 {
            caps |= CAP_USER;
        }
        if entry & XD == 0 {
            if entry & US != 0 {
                caps |= CAP_USER_EXEC;
            } else if entry & SW_KERNEL_EXEC != 0 {
                caps |= CAP_KERNEL_EXEC;
            }
        }
        if entry & G != 0 {
            caps |= CAP_GLOBAL;
        }
        if entry & SW_DEVICE != 0 {
            caps |= CAP_DEVICE;
        }
        Flags::from_caps(caps)
    }
}

/// **VT-d's second-level page-table format** (milestone 161, roadmap item 6): what an Intel IOMMU
/// walks to translate a device's DMA address, not what the CPU walks. Same level count, same
/// 9-bit-per-level, 4 KiB-leaf shape as [`Ia32e`] (VT-d's second-level tables were designed for
/// the same walking hardware), which is why this lives beside it rather than in its own module.
/// **The leaf encoding is not the same, and reusing `Ia32e` here would be wrong rather than
/// merely imprecise**: verified against QEMU's `hw/i386/intel_iommu_internal.h`, a second-level
/// leaf has exactly two meaningful bits, Read (0) and Write (1); bits 2 through 10 are
/// reserved-must-be-zero (`VTD_SPTE_PAGE_L1_RSVD_MASK`), which QEMU's model actually checks and
/// faults on. `Ia32e::leaf_entry` sets `US` (bit 2) for every user-accessible mapping and `XD`
/// (bit 63) for every non-executable one, both of which a DMA domain's `Flags::user_data()`
/// triggers on every leaf it builds; run through VT-d hardware, every one of those leaves would be
/// a reserved-bits-set fault rather than a working translation. A device has no privilege level to
/// gate (there is no `US` bit to set) and no separate execute permission in this mode (there is no
/// `XD` to clear), so this format does not attempt to carry either: [`leaf_flags`](Vtd::leaf_flags)
/// reports only what the two real bits mean.
pub struct Vtd;

/// Read: the transaction may read through this entry. Bit 0, same position as `Ia32e`'s `P`,
/// which is a coincidence worth naming: VT-d has no separate present bit, an entry with both `R`
/// and `W` clear is simply not present.
const VTD_R: u64 = 1 << 0;
/// Write: the transaction may write through this entry.
const VTD_W: u64 = 1 << 1;
/// Bits 51:12: the physical address of the next table or, at a leaf, the mapped frame. The same
/// width `Ia32e::ADDR_MASK` uses, because nothing this kernel runs names a wider physical address
/// yet; VT-d's actual width is `CAP_REG.MGAW`-defined and this driver has not needed to narrow to
/// it (`kernel/src/arch/x86_64/iommu.rs`'s BUGS says why).
const VTD_ADDR_MASK: u64 = 0x000f_ffff_ffff_f000;

impl PageFormat for Vtd {
    const LEVELS: usize = 4;

    /// Not a real VT-d concept (IOVAs are not split into a low and a high half the way CPU
    /// virtual addresses are); kept at the same value `Ia32e` uses so [`in_half`](PageFormat::in_half)
    /// admits exactly the addresses this driver ever asks it about, every physical address the
    /// frame allocator hands out on a machine with less than 128 TiB of RAM.
    const SPLIT_SHIFT: u32 = 47;

    fn is_present(entry: u64) -> bool {
        entry & (VTD_R | VTD_W) != 0
    }

    fn entry_pa(entry: u64) -> u64 {
        entry & VTD_ADDR_MASK
    }

    /// Both bits set: VT-d ANDs permissions down the walk exactly as the CPU's own tables do (an
    /// intermediate entry's `R`/`W` gate every leaf beneath it), so an intermediate must grant
    /// everything and let the leaf decide, the same reasoning `Ia32e::table_entry` documents.
    fn table_entry(pa: u64) -> u64 {
        (pa & VTD_ADDR_MASK) | VTD_R | VTD_W
    }

    /// Read is unconditional (every domain this seam builds is at least readable) and write
    /// follows `flags.is_writable()`. Nothing else: see this type's own doc for why `US` and `XD`
    /// would be reserved-bit violations here rather than the extra permissiveness they are on the
    /// CPU's own format.
    fn leaf_entry(pa: u64, flags: Flags) -> u64 {
        let mut bits = VTD_R;
        if flags.is_writable() {
            bits |= VTD_W;
        }
        (pa & VTD_ADDR_MASK) | bits
    }

    /// The only two facts a second-level leaf carries: read (always, once present) and write.
    fn leaf_flags(entry: u64) -> Flags {
        let mut caps = 0;
        if entry & VTD_W != 0 {
            caps |= CAP_WRITE;
        }
        Flags::from_caps(caps)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **Every leaf is present and accessed**, so no page faults on its own first touch.
    #[test]
    fn every_leaf_is_present_and_accessed() {
        for flags in [
            Flags::kernel_code(),
            Flags::kernel_data(),
            Flags::user_code(),
            Flags::user_data(),
        ] {
            let leaf = Ia32e::leaf_entry(0x10_0000, flags);
            assert_ne!(leaf & P, 0, "not present: {flags:?}");
            assert_ne!(leaf & A, 0, "accessed bit not set: {flags:?}");
        }
    }

    /// **A writable leaf is pre-dirtied, and a read-only one has nothing to dirty.** Neither bit
    /// reaches the portable `Flags`, so the round-trip test below cannot see D go missing.
    #[test]
    fn a_writable_leaf_is_pre_dirtied() {
        assert_ne!(Ia32e::leaf_entry(0x10_0000, Flags::kernel_data()) & D, 0);
        assert_eq!(Ia32e::leaf_entry(0x10_0000, Flags::kernel_code()) & D, 0);
    }

    /// **Nothing writable is also executable.** W^X is a property of the portable `Flags`
    /// constructors, and this asserts the encoder does not manufacture the combination on the way
    /// down: every entry that lacks XD must lack RW.
    #[test]
    fn no_encoded_leaf_is_both_writable_and_executable() {
        for flags in [
            Flags::kernel_code(),
            Flags::kernel_rodata(),
            Flags::kernel_data(),
            Flags::device(),
            Flags::user_code(),
            Flags::user_rodata(),
            Flags::user_data(),
            Flags::user_device(),
        ] {
            let leaf = Ia32e::leaf_entry(0x10_0000, flags);
            if leaf & XD == 0 {
                assert_eq!(leaf & RW, 0, "writable and executable: {flags:?}");
            }
        }
    }

    /// **A table-pointer entry grants everything**, because on x86 the levels are `AND`ed and a
    /// restrictive intermediate entry would silently override the leaf. This is the one place the
    /// three formats genuinely disagree, so it gets its own test rather than a comment.
    #[test]
    fn a_table_entry_grants_everything_and_leaves_the_leaf_to_decide() {
        let e = Ia32e::table_entry(0x20_0000);
        assert_ne!(e & P, 0);
        assert_ne!(
            e & RW,
            0,
            "a read-only intermediate would veto a writable leaf"
        );
        assert_ne!(
            e & US,
            0,
            "a supervisor intermediate would veto a user leaf"
        );
        assert_eq!(
            e & XD,
            0,
            "an XD intermediate would veto an executable leaf"
        );
        assert_eq!(Ia32e::entry_pa(e), 0x20_0000);
    }

    /// **Every constructor round-trips through encode/decode**, including the two software bits
    /// that exist precisely because the hardware encoding loses the distinction.
    #[test]
    fn flags_round_trip_through_a_leaf() {
        for flags in [
            Flags::kernel_code(),
            Flags::kernel_rodata(),
            Flags::kernel_data(),
            Flags::device(),
            Flags::user_code(),
            Flags::user_rodata(),
            Flags::user_data(),
            Flags::user_device(),
        ] {
            let leaf = Ia32e::leaf_entry(0x10_0000, flags);
            assert_eq!(Ia32e::entry_pa(leaf), 0x10_0000);
            assert_eq!(
                Ia32e::leaf_flags(leaf),
                flags,
                "round-trip failed for {flags:?}"
            );
        }
    }

    /// **Kernel code and kernel rodata differ only in the software bit**, which is the whole reason
    /// that bit exists: both are supervisor-only, and x86 has no per-ring execute bit to tell them
    /// apart the way it would need to.
    #[test]
    fn kernel_code_and_rodata_are_distinguished_by_the_software_bit() {
        let code = Ia32e::leaf_entry(0x10_0000, Flags::kernel_code());
        let rodata = Ia32e::leaf_entry(0x10_0000, Flags::kernel_rodata());
        assert_ne!(code & SW_KERNEL_EXEC, 0);
        assert_eq!(rodata & SW_KERNEL_EXEC, 0);
        assert_eq!(code & XD, 0, "kernel code must be executable");
        assert_ne!(rodata & XD, 0, "kernel rodata must not be executable");
    }

    /// **Device memory is uncacheable.** A cached device register reads back a value the device
    /// never produced, and that failure looks like a driver bug for a very long time.
    #[test]
    fn device_memory_is_uncacheable() {
        for flags in [Flags::device(), Flags::user_device()] {
            let leaf = Ia32e::leaf_entry(0xfee0_0000, flags);
            assert_ne!(leaf & PCD, 0, "cacheable device page: {flags:?}");
        }
    }

    /// **A leaf's only bits are R, W and the address.** This is the property that matters most for
    /// [`Vtd`]: verified against QEMU's `VTD_SPTE_PAGE_L1_RSVD_MASK`, any bit outside `R`/`W`/the
    /// address field is reserved-must-be-zero at a second-level leaf, and QEMU's model checks it.
    /// Every `Flags` constructor is tried, including the ones (`user_code`, `kernel_data`, ...)
    /// that would set `US` or `XD` on `Ia32e`'s encoding of the same flags.
    #[test]
    fn a_vtd_leaf_sets_no_bit_outside_read_write_and_address() {
        for flags in [
            Flags::kernel_code(),
            Flags::kernel_rodata(),
            Flags::kernel_data(),
            Flags::device(),
            Flags::user_code(),
            Flags::user_rodata(),
            Flags::user_data(),
            Flags::user_device(),
        ] {
            let leaf = Vtd::leaf_entry(0x10_0000, flags);
            assert_eq!(
                leaf & !(VTD_ADDR_MASK | VTD_R | VTD_W),
                0,
                "a reserved bit was set for {flags:?}: {leaf:#x}"
            );
        }
    }

    /// **The domain builder's only flags, `Flags::user_data`, round-trips on what VT-d actually
    /// has to say: writable.** `CAP_USER` has no VT-d encoding (see [`Vtd`]'s own doc) and is not
    /// expected back.
    #[test]
    fn a_vtd_leaf_is_present_and_writable_for_user_data() {
        let leaf = Vtd::leaf_entry(0x10_0000, Flags::user_data());
        assert!(Vtd::is_present(leaf));
        assert_eq!(Vtd::entry_pa(leaf), 0x10_0000);
        assert!(Vtd::leaf_flags(leaf).is_writable());
    }

    /// A read-only mapping is present (R alone means present) but not writable, and an entry with
    /// neither bit is not present at all: VT-d has no separate present bit, so this is the whole
    /// present/absent story.
    #[test]
    fn read_alone_is_present_but_not_writable_and_zero_is_absent() {
        let ro = Vtd::leaf_entry(0x10_0000, Flags::kernel_rodata());
        assert!(Vtd::is_present(ro));
        assert!(!Vtd::leaf_flags(ro).is_writable());
        assert!(!Vtd::is_present(0), "R and W both clear: not present");
    }

    /// **A VT-d intermediate entry grants everything**, the same reasoning as `Ia32e`'s table
    /// entries and for the same reason: VT-d ANDs `R`/`W` down the walk, so a restrictive
    /// intermediate would veto a writable leaf beneath it.
    #[test]
    fn a_vtd_table_entry_grants_everything() {
        let e = Vtd::table_entry(0x20_0000);
        assert_eq!(e & (VTD_R | VTD_W), VTD_R | VTD_W);
        assert_eq!(Vtd::entry_pa(e), 0x20_0000);
        assert_eq!(
            e & !(VTD_ADDR_MASK | VTD_R | VTD_W),
            0,
            "no reserved bit in a table entry either"
        );
    }
}

/// Machine-checked proofs of the `x86_64` format, mirroring the other two modules'. The shared
/// `Mapper` walk inherits all three formats' guarantees. See notes/verification.md.
#[cfg(kani)]
mod verification {
    use super::*;
    use crate::{Half, PAGE_SIZE};

    /// **The walk never indexes past a table** (four levels here).
    /// Falsification: unfalsified
    #[kani::proof]
    fn index_is_always_in_bounds() {
        let va: u64 = kani::any();
        let level: usize = kani::any();
        kani::assume(level < Ia32e::LEVELS);
        assert!(Ia32e::index(va, level) < crate::ENTRIES);
    }

    /// **The four indices and the offset tile the low 48 bits exactly.**
    /// Falsification: unfalsified
    #[kani::proof]
    fn the_indices_and_offset_tile_the_address() {
        let va: u64 = kani::any();
        let reconstructed = ((Ia32e::index(va, 0) as u64) << 39)
            | ((Ia32e::index(va, 1) as u64) << 30)
            | ((Ia32e::index(va, 2) as u64) << 21)
            | ((Ia32e::index(va, 3) as u64) << 12)
            | (va & (PAGE_SIZE - 1));
        assert_eq!(reconstructed, va & 0x0000_ffff_ffff_ffff);
    }

    /// **Distinct pages take distinct paths** within the 48-bit VA.
    /// Falsification: unfalsified
    #[kani::proof]
    fn distinct_pages_take_distinct_paths() {
        let a: u64 = kani::any::<u64>() & 0x0000_ffff_ffff_f000;
        let b: u64 = kani::any::<u64>() & 0x0000_ffff_ffff_f000;
        kani::assume(
            Ia32e::index(a, 0) == Ia32e::index(b, 0)
                && Ia32e::index(a, 1) == Ia32e::index(b, 1)
                && Ia32e::index(a, 2) == Ia32e::index(b, 2)
                && Ia32e::index(a, 3) == Ia32e::index(b, 3),
        );
        assert_eq!(a, b);
    }

    /// **The two halves are disjoint** at the canonical split (bit 47).
    /// Falsification: unfalsified
    #[kani::proof]
    fn the_two_halves_are_disjoint() {
        let va: u64 = kani::any();
        assert!(!(Ia32e::in_half(Half::Low, va) && Ia32e::in_half(Half::High, va)));
    }

    /// **The user-VA gate admits exactly the aligned low half**, never the high one.
    /// Falsification: replayable `crates/paging/falsifications/x86_64.verification.the_user_va_gate_admits_only_the_aligned_low_half.patch`
    #[kani::proof]
    fn the_user_va_gate_admits_only_the_aligned_low_half() {
        let va: u64 = kani::any();
        assert_eq!(
            crate::is_user_page_va::<Ia32e>(va),
            va & 0xfff == 0 && va >> 47 == 0
        );
        if crate::is_user_page_va::<Ia32e>(va) {
            assert!(Ia32e::in_half(Half::Low, va) && !Ia32e::in_half(Half::High, va));
        }
    }

    /// **A leaf keeps the address and the permissions apart, and the permissions round-trip.**
    /// Falsification: replayable `crates/paging/falsifications/x86_64.verification.the_leaf_keeps_address_and_permissions_apart.patch`
    #[kani::proof]
    fn the_leaf_keeps_address_and_permissions_apart() {
        let pa: u64 = kani::any();
        kani::assume(pa & !ADDR_MASK == 0);

        let all = [
            Flags::kernel_code(),
            Flags::kernel_rodata(),
            Flags::kernel_data(),
            Flags::device(),
            Flags::user_code(),
            Flags::user_rodata(),
            Flags::user_data(),
            Flags::user_device(),
        ];
        let i: usize = kani::any();
        kani::assume(i < all.len());
        let flags = all[i];

        let leaf = Ia32e::leaf_entry(pa, flags);
        // **The address field read out of the word, not through `entry_pa`** (milestone 211).
        // `leaf_entry` and `entry_pa` are an encoder and its own decoder, so a round trip
        // between them is satisfied by any pair that agree: a shift wrong in both would leave
        // this green while the hardware read the wrong page. The line below states where the
        // architecture puts the address, which is the one thing the implementation does not
        // get to choose, and it is spelled as a literal rather than through this crate's own
        // ADDR_MASK or PPN_SHIFT: a defect in one of those constants moves the implementation
        // and any harness that cited it together, which is the same trap one level down.
        // `no_vtd_entry_ever_sets_a_reserved_bit` in this crate already works this way; this
        // is the same move on the portable leaf.
        assert_eq!(
            leaf & 0x000f_ffff_ffff_f000,
            pa,
            "the address left bits 12..52 of the entry"
        );
        assert_eq!(leaf & 1, 1, "a leaf must have the present bit set");
        assert_eq!(Ia32e::entry_pa(leaf), pa);
        assert_eq!(Ia32e::leaf_flags(leaf), flags);
    }

    /// **No encoded leaf is both writable and executable**, over every constructor. The W^X
    /// property is enforced by the portable `Flags` constructors; this proves the x86 encoder does
    /// not reintroduce the combination while translating them.
    /// Falsification: replayable `crates/paging/falsifications/x86_64.verification.no_encoded_leaf_is_both_writable_and_executable.patch`
    #[kani::proof]
    fn no_encoded_leaf_is_both_writable_and_executable() {
        let pa: u64 = kani::any();
        kani::assume(pa & !ADDR_MASK == 0);

        let all = [
            Flags::kernel_code(),
            Flags::kernel_rodata(),
            Flags::kernel_data(),
            Flags::device(),
            Flags::user_code(),
            Flags::user_rodata(),
            Flags::user_data(),
            Flags::user_device(),
        ];
        let i: usize = kani::any();
        kani::assume(i < all.len());

        let leaf = Ia32e::leaf_entry(pa, all[i]);
        assert!(leaf & XD != 0 || leaf & RW == 0);
    }

    /// **No `Vtd` leaf or table entry ever sets a bit VT-d treats as reserved**, over every
    /// physical address and every portable `Flags` constructor. This is the property
    /// `a_vtd_leaf_sets_no_bit_outside_read_write_and_address` checks by example; Kani closes it
    /// for every address and every flag combination the type accepts, which matters here more than
    /// on `Ia32e` because QEMU's model (and real silicon) faults a transaction over a reserved bit
    /// rather than merely ignoring it.
    /// Falsification: replayable `crates/paging/falsifications/x86_64.verification.no_vtd_entry_ever_sets_a_reserved_bit.patch`
    #[kani::proof]
    fn no_vtd_entry_ever_sets_a_reserved_bit() {
        let pa: u64 = kani::any();
        kani::assume(pa & !VTD_ADDR_MASK == 0);

        let all = [
            Flags::kernel_code(),
            Flags::kernel_rodata(),
            Flags::kernel_data(),
            Flags::device(),
            Flags::user_code(),
            Flags::user_rodata(),
            Flags::user_data(),
            Flags::user_device(),
        ];
        let i: usize = kani::any();
        kani::assume(i < all.len());

        let leaf = Vtd::leaf_entry(pa, all[i]);
        assert_eq!(leaf & !(VTD_ADDR_MASK | VTD_R | VTD_W), 0);
        assert_eq!(Vtd::entry_pa(leaf), pa);

        let table = Vtd::table_entry(pa);
        assert_eq!(table & !(VTD_ADDR_MASK | VTD_R | VTD_W), 0);
    }
}
