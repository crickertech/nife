//! Finding one ELF note by owner and type, through the program headers alone.
//!
//! **Research prototype** (lane `lane/elf-note-price`, 2026-09-26 UTC), written to price DECISIONS
//! §197 (a package is one archive file)'s option M2, a program's manifest carried inside its own
//! ELF. Every public name here is provisional and unratified; calef names public items.
//!
//! **Why program headers and not section headers.** A linker gathers allocated `SHT_NOTE` sections
//! into a `PT_NOTE` segment, and the loader already walks the program-header table to find
//! `PT_LOAD`. So a note is one more `p_type` in a table this crate already bounds and trusts, and
//! the crate keeps its "program headers only" property. Section headers are the other way in, and
//! they cost more than a second table: `strip --strip-section-headers` and `sstrip` delete them from
//! a working binary, so a reader that needed them would lose the manifest to a tool that leaves the
//! program runnable.
//!
//! **Nothing here runs inside [`Elf::parse`].** The loader's path is unchanged, byte for byte, and
//! the kernel never calls [`Elf::note`]: only a program that reads manifests (the shell, the
//! progenitor) does, and only on the bytes it was handed to spawn.
//!
//! # The format, and where the specification and practice disagree
//!
//! Each note is three little-endian `u32`s, `namesz`, `descsz`, `type`, then `namesz` bytes of owner
//! name (NUL included), padded, then `descsz` bytes of descriptor, padded. The gABI says an ELF64
//! note's words and padding are 8 bytes. **Nobody ships that.** Every toolchain writes 4-byte
//! words on ELF64 and 4-byte padding, except `.note.gnu.property`, which is 8-byte padded and sits
//! in its own `PT_NOTE` with `p_align` 8. So the padding is read from the segment's `p_align`: 0, 1
//! and 4 mean 4, 8 means 8, and anything else is refused. That is LLVM's `Elf_Note_Iterator` rule,
//! which is what `llvm-readelf` uses.
//!
//! # What a hostile file can try, and what happens
//!
//! - A `namesz` or `descsz` near `u32::MAX`: the extent arithmetic is checked, proved total by
//!   `verification::note_extent_never_panics`.
//! - A note whose padded end runs past its segment: refused ([`NoteError::Truncated`]), as LLVM
//!   refuses "ELF note overflows container". A lenient reader here would be a second opinion about
//!   where the last note ends.
//! - A `PT_NOTE` that points outside the file: refused ([`NoteError::SegmentOutOfBounds`]).
//! - **The same owner and type twice**: refused ([`NoteError::Duplicate`]). This is the one that is
//!   not about memory safety. The shell and the progenitor both read the manifest, and if one took
//!   the first copy and the other the last, the person at the prompt would approve one manifest and
//!   the progenitor would endow another. Refusing ambiguity makes "which one?" unaskable.
//! - An owner name with no terminating NUL where it must have one: the note is not a match, since
//!   the comparison includes the NUL.
//! - Up to 64 `PT_NOTE` headers (the crate's existing `MAX_PHNUM` bound), each walked once; each
//!   note advances the cursor by at least 12 bytes (proved), so the walk ends.

use super::{Elf, u32le, u64le};

/// `p_type` for a segment of notes.
const PT_NOTE: u32 = 4;

/// The three header words.
const NHDR_SIZE: usize = 12;

/// Why a note lookup refused the file. Provisional name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoteError {
    /// A `PT_NOTE` whose file range is not inside the file.
    SegmentOutOfBounds,
    /// A `PT_NOTE` whose `p_align` is not 0, 1, 4 or 8.
    BadAlignment,
    /// A note header, name or descriptor (with its padding) runs past the end of its segment.
    Truncated,
    /// The owner and type asked for appear more than once.
    Duplicate,
}

/// Where one note's pieces lie, as offsets into its segment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Extent {
    name_start: usize,
    name_end: usize,
    desc_start: usize,
    desc_end: usize,
    /// Where the next note's header starts: this note's padded end.
    next: usize,
}

/// The arithmetic of one note, loopless, so it can be proved for every value a file can hold.
///
/// `at` is where the note's header starts in a segment of `len` bytes; the caller has already
/// checked `at + 12 <= len` and read `namesz` and `descsz`, which is why that check is repeated
/// here: the proof must not lean on the caller.
fn note_extent(
    len: usize,
    at: usize,
    namesz: u32,
    descsz: u32,
    align: usize,
) -> Result<Extent, NoteError> {
    let name_start = at.checked_add(NHDR_SIZE).ok_or(NoteError::Truncated)?;
    if name_start > len {
        return Err(NoteError::Truncated);
    }
    let name_end = name_start
        .checked_add(namesz as usize)
        .ok_or(NoteError::Truncated)?;
    let desc_start = align_up(name_end, align).ok_or(NoteError::Truncated)?;
    let desc_end = desc_start
        .checked_add(descsz as usize)
        .ok_or(NoteError::Truncated)?;
    let next = align_up(desc_end, align).ok_or(NoteError::Truncated)?;
    if next > len {
        return Err(NoteError::Truncated);
    }
    Ok(Extent {
        name_start,
        name_end,
        desc_start,
        desc_end,
        next,
    })
}

/// `x` rounded up to a multiple of `align`, which is 4 or 8. `None` if that wraps.
fn align_up(x: usize, align: usize) -> Option<usize> {
    let mask = align - 1;
    Some(x.checked_add(mask)? & !mask)
}

/// `p_align` to note padding, by LLVM's rule.
fn padding(p_align: u64) -> Result<usize, NoteError> {
    match p_align {
        0 | 1 | 4 => Ok(4),
        8 => Ok(8),
        _ => Err(NoteError::BadAlignment),
    }
}

/// Walk one note segment and return the descriptor of the note owned by `owner` (compared with its
/// NUL) with type `kind`, refusing a malformed segment or a second match. `found` carries a match
/// from an earlier segment, so a duplicate across segments is caught too.
fn find_in_segment<'a>(
    seg: &'a [u8],
    align: usize,
    owner: &[u8],
    kind: u32,
    mut found: Option<&'a [u8]>,
) -> Result<Option<&'a [u8]>, NoteError> {
    let mut at = 0;
    while at < seg.len() {
        if seg.len() - at < NHDR_SIZE {
            return Err(NoteError::Truncated);
        }
        let namesz = u32le(seg, at);
        let descsz = u32le(seg, at + 4);
        let ntype = u32le(seg, at + 8);
        let e = note_extent(seg.len(), at, namesz, descsz, align)?;

        let name = &seg[e.name_start..e.name_end];
        if ntype == kind
            && name.len() == owner.len() + 1
            && name[name.len() - 1] == 0
            && &name[..owner.len()] == owner
        {
            if found.is_some() {
                return Err(NoteError::Duplicate);
            }
            found = Some(&seg[e.desc_start..e.desc_end]);
        }
        at = e.next;
    }
    Ok(found)
}

impl<'a> Elf<'a> {
    /// **The descriptor of the one note owned by `owner` with type `kind`**, or `None` if the file
    /// has no such note. Provisional name.
    ///
    /// `owner` is the name without its NUL (`b"nife"`). Every `PT_NOTE` segment is walked in full,
    /// so a malformed note anywhere refuses the lookup, and so does a second copy of the one asked
    /// for; see the module documentation for why both are refusals rather than tolerances.
    pub fn note(&self, owner: &[u8], kind: u32) -> Result<Option<&'a [u8]>, NoteError> {
        let mut found = None;
        for i in 0..self.phnum {
            let off = self.phoff + i * self.phentsize;
            let ph = &self.bytes[off..off + super::PHDR_SIZE];
            if u32le(ph, 0) != PT_NOTE {
                continue;
            }
            let p_offset = u64le(ph, 8) as usize;
            let filesz = u64le(ph, 32) as usize;
            let align = padding(u64le(ph, 48))?;
            let end = p_offset
                .checked_add(filesz)
                .ok_or(NoteError::SegmentOutOfBounds)?;
            if end > self.bytes.len() {
                return Err(NoteError::SegmentOutOfBounds);
            }
            found = find_in_segment(&self.bytes[p_offset..end], align, owner, kind, found)?;
        }
        Ok(found)
    }
}

#[cfg(kani)]
mod verification {
    use super::*;

    /// **The extent arithmetic never panics**, for any segment length, cursor, sizes and either
    /// padding. The fields come straight from the file.
    ///
    /// Falsification: replayable `crates/elf/falsifications/note.verification.note_extent_never_panics.patch`
    #[kani::proof]
    fn note_extent_never_panics() {
        let align: usize = if kani::any() { 4 } else { 8 };
        let _ = note_extent(kani::any(), kani::any(), kani::any(), kani::any(), align);
    }

    /// **A passing extent is ordered, inside the segment, and moves the cursor forward.** The
    /// ordering is what makes the two slices in `find_in_segment` in bounds; `at < next` is what
    /// makes the walk end.
    ///
    /// Falsification: replayable `crates/elf/falsifications/note.verification.a_passing_extent_is_inside_and_advances.patch`
    #[kani::proof]
    fn a_passing_extent_is_inside_and_advances() {
        let len: usize = kani::any();
        let at: usize = kani::any();
        let align: usize = if kani::any() { 4 } else { 8 };
        if let Ok(e) = note_extent(len, at, kani::any(), kani::any(), align) {
            assert!(at < e.name_start);
            assert!(e.name_start <= e.name_end);
            assert!(e.name_end <= e.desc_start);
            assert!(e.desc_start <= e.desc_end);
            assert!(e.desc_end <= e.next);
            assert!(e.next <= len);
            assert!(e.next - at >= NHDR_SIZE);
        }
    }

    /// **A note segment the solver chose is refused or read entirely inside itself**, walked by
    /// the real loop. 40 bytes holds at most three notes, so the loop runs at most three times and
    /// the owner comparison at most five bytes; the unwind bound covers both.
    ///
    /// Falsification: replayable `crates/elf/falsifications/note.verification.a_note_segment_is_read_inside_itself.patch`
    #[kani::proof]
    #[kani::unwind(7)]
    fn a_note_segment_is_read_inside_itself() {
        const LEN: usize = 40;
        let seg: [u8; LEN] = kani::any();
        let align: usize = if kani::any() { 4 } else { 8 };
        if let Ok(Some(desc)) = find_in_segment(&seg, align, b"nife", kani::any(), None) {
            let start = desc.as_ptr() as usize - seg.as_ptr() as usize;
            assert!(start + desc.len() <= LEN);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{NATIVE_MACHINE, PF_R, PF_X};
    extern crate std;
    use std::vec;
    use std::vec::Vec;

    /// One note, 4-byte padded.
    fn note(owner: &[u8], kind: u32, desc: &[u8]) -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(&((owner.len() + 1) as u32).to_le_bytes());
        v.extend_from_slice(&(desc.len() as u32).to_le_bytes());
        v.extend_from_slice(&kind.to_le_bytes());
        v.extend_from_slice(owner);
        v.push(0);
        while v.len() % 4 != 0 {
            v.push(0);
        }
        v.extend_from_slice(desc);
        while v.len() % 4 != 0 {
            v.push(0);
        }
        v
    }

    /// A one-`PT_LOAD` ELF plus one `PT_NOTE` per entry of `notes`, each `(p_align, bytes)`.
    fn image(notes: &[(u64, Vec<u8>)]) -> Vec<u8> {
        let phnum = 1 + notes.len();
        let mut body_off = 64 + phnum * 56;
        let mut v = vec![0u8; 64];
        v[0..4].copy_from_slice(&[0x7f, b'E', b'L', b'F']);
        v[4] = 2;
        v[5] = 1;
        v[6] = 1;
        v[16..18].copy_from_slice(&2u16.to_le_bytes());
        v[18..20].copy_from_slice(&NATIVE_MACHINE.to_le_bytes());
        v[24..32].copy_from_slice(&0x40_0000u64.to_le_bytes());
        v[32..40].copy_from_slice(&64u64.to_le_bytes());
        v[54..56].copy_from_slice(&56u16.to_le_bytes());
        v[56..58].copy_from_slice(&(phnum as u16).to_le_bytes());

        let code = [0xaau8; 16];
        let mut ph = vec![0u8; 56];
        ph[0..4].copy_from_slice(&1u32.to_le_bytes());
        ph[4..8].copy_from_slice(&(PF_R | PF_X).to_le_bytes());
        ph[8..16].copy_from_slice(&(body_off as u64).to_le_bytes());
        ph[16..24].copy_from_slice(&0x40_0000u64.to_le_bytes());
        ph[24..32].copy_from_slice(&0x40_0000u64.to_le_bytes());
        ph[32..40].copy_from_slice(&16u64.to_le_bytes());
        ph[40..48].copy_from_slice(&16u64.to_le_bytes());
        v.extend_from_slice(&ph);
        body_off += code.len();

        let mut body = code.to_vec();
        for (align, bytes) in notes {
            let mut ph = vec![0u8; 56];
            ph[0..4].copy_from_slice(&PT_NOTE.to_le_bytes());
            ph[4..8].copy_from_slice(&PF_R.to_le_bytes());
            ph[8..16].copy_from_slice(&(body_off as u64).to_le_bytes());
            ph[32..40].copy_from_slice(&(bytes.len() as u64).to_le_bytes());
            ph[40..48].copy_from_slice(&(bytes.len() as u64).to_le_bytes());
            ph[48..56].copy_from_slice(&align.to_le_bytes());
            v.extend_from_slice(&ph);
            body.extend_from_slice(bytes);
            body_off += bytes.len();
        }
        v.extend_from_slice(&body);
        v
    }

    #[test]
    fn a_note_is_found_among_others() {
        let mut seg = note(b"GNU", 3, &[1; 20]);
        seg.extend(note(b"nife", 1, b"manifest bytes"));
        seg.extend(note(b"nife", 2, b"other type"));
        let bytes = image(&[(4, seg)]);
        let elf = Elf::parse(&bytes).expect("a PT_NOTE does not disturb the loader");
        assert_eq!(elf.note(b"nife", 1), Ok(Some(&b"manifest bytes"[..])));
        assert_eq!(elf.note(b"nife", 9), Ok(None));
        assert_eq!(elf.segments().count(), 1, "PT_NOTE is not a load segment");
    }

    #[test]
    fn an_owner_that_is_a_prefix_is_not_a_match() {
        let bytes = image(&[(4, note(b"nifes", 1, b"x"))]);
        assert_eq!(Elf::parse(&bytes).unwrap().note(b"nife", 1), Ok(None));
    }

    #[test]
    fn the_same_note_twice_is_refused_within_and_across_segments() {
        let mut seg = note(b"nife", 1, b"a");
        seg.extend(note(b"nife", 1, b"b"));
        let bytes = image(&[(4, seg)]);
        assert_eq!(
            Elf::parse(&bytes).unwrap().note(b"nife", 1),
            Err(NoteError::Duplicate)
        );

        let bytes = image(&[(4, note(b"nife", 1, b"a")), (4, note(b"nife", 1, b"b"))]);
        assert_eq!(
            Elf::parse(&bytes).unwrap().note(b"nife", 1),
            Err(NoteError::Duplicate)
        );
    }

    #[test]
    fn a_descriptor_that_runs_past_its_segment_is_refused() {
        let mut seg = note(b"nife", 1, b"abcd");
        seg[4..8].copy_from_slice(&u32::MAX.to_le_bytes());
        let bytes = image(&[(4, seg)]);
        assert_eq!(
            Elf::parse(&bytes).unwrap().note(b"nife", 1),
            Err(NoteError::Truncated)
        );
    }

    #[test]
    fn a_missing_final_pad_is_refused() {
        let mut seg = note(b"nife", 1, b"abc"); // descriptor 3 bytes, padded to 4
        seg.pop();
        let bytes = image(&[(4, seg)]);
        assert_eq!(
            Elf::parse(&bytes).unwrap().note(b"nife", 1),
            Err(NoteError::Truncated)
        );
    }

    #[test]
    fn a_header_cut_short_is_refused() {
        let mut seg = note(b"nife", 1, b"abcd");
        seg.extend_from_slice(&[0; 8]);
        let bytes = image(&[(4, seg)]);
        assert_eq!(
            Elf::parse(&bytes).unwrap().note(b"nife", 1),
            Err(NoteError::Truncated)
        );
    }

    #[test]
    fn eight_byte_padding_follows_p_align() {
        // `.note.gnu.property`'s padding. With "GNU\0" (namesz 4) the name ends on 16, which is
        // 8-aligned already, so the rule is invisible; a 5-byte owner shows the difference.
        let mut seg = Vec::new();
        seg.extend_from_slice(&5u32.to_le_bytes());
        seg.extend_from_slice(&3u32.to_le_bytes());
        seg.extend_from_slice(&1u32.to_le_bytes());
        seg.extend_from_slice(b"nife\0\0\0\0\0\0\0\0"); // 12 + 5 pads to 24 under 8, to 20 under 4
        seg.extend_from_slice(b"abc\0\0\0\0\0");
        let bytes = image(&[(8, seg.clone())]);
        assert_eq!(
            Elf::parse(&bytes).unwrap().note(b"nife", 1),
            Ok(Some(&b"abc"[..]))
        );
        // Read with 4-byte padding, the same bytes put the descriptor at 20 and read the padding.
        let bytes = image(&[(4, seg)]);
        assert_ne!(
            Elf::parse(&bytes).unwrap().note(b"nife", 1),
            Ok(Some(&b"abc"[..]))
        );
    }

    #[test]
    fn an_alignment_that_is_not_four_or_eight_is_refused() {
        let bytes = image(&[(16, note(b"nife", 1, b"a"))]);
        assert_eq!(
            Elf::parse(&bytes).unwrap().note(b"nife", 1),
            Err(NoteError::BadAlignment)
        );
    }

    #[test]
    fn a_note_segment_outside_the_file_is_refused() {
        let mut bytes = image(&[(4, note(b"nife", 1, b"a"))]);
        let ph = 64 + 56;
        bytes[ph + 8..ph + 16].copy_from_slice(&(u64::MAX - 2).to_le_bytes());
        assert_eq!(
            Elf::parse(&bytes).unwrap().note(b"nife", 1),
            Err(NoteError::SegmentOutOfBounds)
        );
    }

    #[test]
    fn an_empty_note_segment_has_no_note() {
        let bytes = image(&[(4, Vec::new())]);
        assert_eq!(Elf::parse(&bytes).unwrap().note(b"nife", 1), Ok(None));
    }

    #[test]
    fn an_empty_descriptor_is_found_and_empty() {
        let bytes = image(&[(4, note(b"nife", 1, b""))]);
        assert_eq!(
            Elf::parse(&bytes).unwrap().note(b"nife", 1),
            Ok(Some(&b""[..]))
        );
    }
}
