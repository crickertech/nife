//! The x86_64 four-level page-table format: 4 KiB pages, 48-bit virtual addresses.
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

/// The x86_64 four-level (IA-32e) page-table format.
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

    /// Bit 47. An x86_64 virtual address is canonical when bits 63:47 are all equal, which is the
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

    /// **A table-pointer entry grants everything**, because on x86 the levels are ANDed and a
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
}

/// Machine-checked proofs of the x86_64 format, mirroring the other two modules'. The shared
/// `Mapper` walk inherits all three formats' guarantees. See notes/verification.md.
#[cfg(kani)]
mod verification {
    use super::*;
    use crate::{Half, PAGE_SIZE};

    /// **The walk never indexes past a table** (four levels here).
    #[kani::proof]
    fn index_is_always_in_bounds() {
        let va: u64 = kani::any();
        let level: usize = kani::any();
        kani::assume(level < Ia32e::LEVELS);
        assert!(Ia32e::index(va, level) < crate::ENTRIES);
    }

    /// **The four indices and the offset tile the low 48 bits exactly.**
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
    #[kani::proof]
    fn the_two_halves_are_disjoint() {
        let va: u64 = kani::any();
        assert!(!(Ia32e::in_half(Half::Low, va) && Ia32e::in_half(Half::High, va)));
    }

    /// **The user-VA gate admits exactly the aligned low half**, never the high one.
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
        assert_eq!(Ia32e::entry_pa(leaf), pa);
        assert_eq!(Ia32e::leaf_flags(leaf), flags);
    }

    /// **No encoded leaf is both writable and executable**, over every constructor. The W^X
    /// property is enforced by the portable `Flags` constructors; this proves the x86 encoder does
    /// not reintroduce the combination while translating them.
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
}
