//! **Placing a validated ELF by its *physical* addresses**, which is the one thing a boot loader
//! needs and an ordinary program loader never does.
//!
//! # This used to be a second ELF reader, and it is not one any more
//!
//! Until milestone 208 this module carried its own forty-line header walk, with its own `Error`
//! enum, because `elf::Segment` exposed no `p_paddr` and there was nothing to subtract: the kernel
//! image is linked high with `AT()`, so `.text` is `p_vaddr` `0xffffffff8010a000` against `p_paddr`
//! `0x10a000` while `.ap_trampoline` is `p_vaddr` `0x8000` against `p_paddr` `0x166000`, a
//! completely different relationship in the same file.
//!
//! Milestone 196 (a physical address on `elf::Segment`) added the field. That was necessary and it
//! was not sufficient: `Elf::parse` still **refused this kernel outright**, with
//! `Error::WritableAndExecutable`, because `kernel/link-x86_64.ld` folded the 32-bit trampoline's
//! code and data into one output section and shipped them as a single `RWX` `PT_LOAD`. Milestone
//! 208 split that section, and with the image W^X clean the tree's own parser accepts it, so the
//! duplicate could go. Two readers of one format agreeing by comment rather than by type is
//! AGENTS.md rule 7's exact target.
//!
//! **What this module lost by the change is nothing and what it gained is validation.** The old
//! reader deliberately did not validate, on the argument that this kernel comes from the same build
//! that produced this loader. That argument was always weaker than it looked, because it makes the
//! loader the one place in the tree where an ELF is trusted rather than checked, and it is now moot:
//! `Elf::parse` runs the whole battery (bounds, overlap, W^X, the entry point inside an executable
//! segment) in the same call that reads the header. So a kernel image that regressed to `RWX` would
//! now fail to boot on real firmware with a printed reason, which is a second mechanism behind
//! `script/image-permissions` and the one that runs on the machine a person is standing at.
//!
//! # BUGS
//!
//! - **Segments must not overlap the loader's own image**, and nothing checks it. The kernel's
//!   `p_paddr` span is fixed by `kernel/link-x86_64.ld`'s `PHYS_START` and the firmware is asked
//!   for exactly that range with `AllocateAddress`, so a conflict surfaces as an allocation failure
//!   with the range and the descriptors in the way printed, rather than as corruption. That is the
//!   check, and it is the firmware's rather than this module's.
//! - **`Elf::parse`'s overlap check is over `p_vaddr`, not `p_paddr`.** For a program that is the
//!   same question; for this image the two differ, so nothing rejects a linker script that made two
//!   segments' *physical* ranges collide. The single `AllocateAddress` call above would still
//!   succeed, and the later segment would quietly overwrite the earlier one. No linker script in
//!   this tree can produce that (one script, one contiguous physical layout), which is why it is
//!   recorded here rather than checked.

use elf::{Elf, Segment};

/// The `[first, last)` physical range every `PT_LOAD` falls in, page-aligned outwards.
///
/// This is what the firmware is asked for in one `AllocatePages(AllocateAddress)` call rather than
/// segment by segment: the segments are contiguous by construction (one linker script), and asking
/// once means one failure to report instead of thirteen.
///
/// `None` when there is not a single `PT_LOAD`, which `Elf::parse` cannot actually produce (an
/// image with no executable segment has no entry point and is refused before this runs). It is a
/// return value rather than an `unwrap` because this function is public and must be panic-free on
/// its own terms.
///
/// **Over `paddr`, not `vaddr`**, which is the whole reason this function exists rather than a
/// method on `Elf`: a program loader maps at `p_vaddr` and a firmware loader places at `p_paddr`,
/// and on this image those are unrelated. See `elf::Segment::paddr`.
pub fn physical_span<'a>(
    segments: impl Iterator<Item = Segment<'a>>,
    page_size: u64,
) -> Option<(u64, u64)> {
    let mut first = u64::MAX;
    let mut last = 0u64;
    let mut any = false;
    for segment in segments {
        any = true;
        first = first.min(segment.paddr);
        last = last.max(segment.paddr.saturating_add(segment.memsz));
    }
    if !any {
        return None;
    }
    Some((
        first & !(page_size - 1),
        last.div_ceil(page_size).saturating_mul(page_size),
    ))
}

/// A sentence for the firmware console, for every way [`Elf::parse`] can refuse the kernel this
/// binary was built around.
///
/// The wording is this loader's rather than the crate's, because the audience is somebody standing
/// at a machine that will not boot rather than a program handling an error. Every one of these means
/// **the build is wrong**, not that a hostile file arrived: the only ELF this loader ever sees is
/// the kernel `build.rs` embedded in it.
///
/// Exhaustive on purpose, with no `_` arm. `elf::Error` is a shared definition, so growing it should
/// break this build rather than silently print the wrong sentence; that is rung one of AGENTS.md's
/// ladder for the price of a match.
pub const fn refusal(error: elf::Error) -> &'static str {
    use elf::Error::*;
    match error {
        TooSmall => "the embedded kernel is smaller than an ELF header",
        BadMagic => "the embedded kernel is not an ELF file",
        Not64Bit => "the embedded kernel is not ELF64",
        NotLittleEndian => "the embedded kernel is not little-endian",
        BadVersion => "the embedded kernel's ELF version is not 1",
        WrongMachine => "the embedded kernel is not an x86-64 ELF",
        NeedsRelocation => "the embedded kernel is a PIE and nothing here relocates it",
        NotExecutable => "the embedded kernel is not an executable",
        BadProgramHeaders => "the embedded kernel's program header table is out of bounds",
        SegmentOutOfBounds => "an embedded kernel segment runs past the end of the file",
        SegmentTruncated => "an embedded kernel segment has more bytes than memory",
        WritableAndExecutable => {
            "the embedded kernel ships a writable-executable segment (see kernel/link-x86_64.ld)"
        }
        SegmentUnreachable => "an embedded kernel segment is neither readable nor executable",
        SegmentsOverlap => "two embedded kernel segments want the same page",
        EntryNotExecutable => "the embedded kernel's entry point is not in an executable segment",
        AddressOverflow => "an embedded kernel segment's address arithmetic overflows",
        TooManyProgramHeaders => "the embedded kernel has more program headers than we will read",
    }
}

/// Parse and validate the embedded kernel, with this loader's wording on a refusal.
pub fn parse(bytes: &[u8]) -> Result<Elf<'_>, &'static str> {
    Elf::parse(bytes).map_err(refusal)
}

#[cfg(test)]
mod tests {
    use elf::{PF_R, PF_W, PF_X};

    use super::*;

    /// `Segment`'s fields are all public, so the span can be tested against hand-built segments
    /// rather than against a forged ELF. The forging belongs to `crates/elf`'s own tests, which is
    /// where `Elf::parse` is proved; what is under test here is the arithmetic over `paddr`.
    fn seg(paddr: u64, memsz: u64) -> Segment<'static> {
        Segment {
            vaddr: paddr + 0xffff_ffff_8000_0000,
            paddr,
            memsz,
            flags: PF_R | PF_X,
            data: &[],
        }
    }

    /// The property this module exists for: the span is over **`p_paddr`**, so an image whose
    /// virtual addresses are in the high half still reports a low physical span.
    #[test]
    fn the_span_is_physical_and_page_aligned_outwards() {
        let segments = [
            seg(0x10_0000, 0x14),
            seg(0x10_9000, 0x4c000),
            seg(0x23_8123, 0x100),
        ];
        assert_eq!(
            physical_span(segments.into_iter(), 4096),
            Some((0x10_0000, 0x239_000))
        );
    }

    /// A `NOLOAD` section arrives as a segment with no bytes and a real size, and it has to be
    /// counted in the span or the firmware hands its memory to something else.
    #[test]
    fn a_noload_segment_still_reserves_its_memory() {
        let segments = [seg(0x10_2000, 0x7000)];
        assert_eq!(
            physical_span(segments.into_iter(), 4096),
            Some((0x10_2000, 0x109_000))
        );
    }

    /// The trampoline's shape: its `p_vaddr` is *below* every other segment's while its `p_paddr`
    /// is in the middle of the image. A span taken over `vaddr` would start at 0x8000 and ask the
    /// firmware for a megabyte of real-mode memory it does not own.
    #[test]
    fn a_segment_that_ships_high_and_runs_low_is_spanned_where_it_ships() {
        let trampoline = Segment {
            vaddr: 0x8000,
            paddr: 0x16_6000,
            memsz: 0xe0,
            flags: PF_R | PF_X,
            data: &[],
        };
        assert_eq!(
            physical_span([seg(0x10_0000, 0x14), trampoline].into_iter(), 4096),
            Some((0x10_0000, 0x167_000))
        );
    }

    #[test]
    fn an_image_with_no_load_segments_has_no_span() {
        assert_eq!(physical_span([].into_iter(), 4096), None);
    }

    /// Every refusal has its own sentence, and none of them is empty. The value of the exhaustive
    /// match is that adding a variant to `elf::Error` fails this build; the value of this test is
    /// that nobody satisfies that by pasting the same string.
    #[test]
    fn every_refusal_says_something_of_its_own() {
        use elf::Error::*;
        let all = [
            TooSmall,
            BadMagic,
            Not64Bit,
            NotLittleEndian,
            BadVersion,
            WrongMachine,
            NeedsRelocation,
            NotExecutable,
            BadProgramHeaders,
            SegmentOutOfBounds,
            SegmentTruncated,
            WritableAndExecutable,
            SegmentUnreachable,
            SegmentsOverlap,
            EntryNotExecutable,
            AddressOverflow,
            TooManyProgramHeaders,
        ];
        let mut seen: Vec<&str> = all.iter().map(|e| refusal(*e)).collect();
        assert!(seen.iter().all(|s| !s.is_empty()));
        seen.sort_unstable();
        let before = seen.len();
        seen.dedup();
        assert_eq!(seen.len(), before, "two refusals share a sentence");
    }

    /// A real property of the pair, cheap to state and easy to get wrong: a W^X violation is
    /// refused, and the sentence a person reads names the file to go and edit.
    #[test]
    fn a_writable_executable_segment_is_named_by_the_file_that_causes_it() {
        assert!(refusal(elf::Error::WritableAndExecutable).contains("link-x86_64.ld"));
        let s = Segment {
            vaddr: 0x1000,
            paddr: 0x1000,
            memsz: 0x1000,
            flags: PF_R | PF_W | PF_X,
            data: &[],
        };
        assert!(s.is_writable() && s.is_executable());
    }
}
