//! **Reading the kernel ELF by its *physical* addresses**, which is the one thing
//! `crates/elf` cannot do.
//!
//! # Why this is not `crates/elf`
//!
//! That crate is the kernel's user-program loader, and a program is loaded at its `p_vaddr`. This
//! loader is standing in for a firmware boot loader, and a boot loader places an image at its
//! `p_paddr`. On this kernel the two are not related by any offset a caller could apply: the image
//! is linked high with `AT()`, so `.text` has `p_vaddr` `0xffffffff80109000` and `p_paddr`
//! `0x109000`, while `.ap_trampoline` has `p_vaddr` `0x8000` and `p_paddr` `0x165000`, a completely
//! different relationship. `elf::Segment` does not carry `p_paddr` at all, so there is nothing to
//! subtract.
//!
//! Adding `paddr` to `elf::Segment` is the tidier answer and is left for its own lane: that struct
//! is public, the kernel's loader is its consumer, and widening it is a change to a shared
//! definition rather than to this milestone.
//!
//! # What this deliberately does not do
//!
//! It does not validate. `crates/elf` validates exhaustively because it is handed programs from an
//! archive; this module is handed **the kernel that the same build produced**, embedded in this
//! binary by `build.rs`, so the only failure it can have is a build that went wrong. The checks
//! below are therefore assertions about our own build (is it ELF64, is it x86-64, does it have
//! program headers), reported as an error a person reads at the machine rather than trusted.
//!
//! # BUGS
//!
//! - **Segments must not overlap the loader's own image**, and nothing checks it. The kernel's
//!   `p_paddr` span is fixed by `kernel/link-x86_64.ld` at 1 MiB and the firmware is asked for
//!   exactly that range with `AllocateAddress`, so a conflict surfaces as an allocation failure
//!   with a printed address rather than as corruption. That is the check, and it is the firmware's
//!   rather than this module's.

/// What the loader needs out of the ELF header, once.
#[derive(Debug)]
pub struct Image<'a> {
    bytes: &'a [u8],
    /// `e_entry`. On this kernel it is `_start`, the 32-bit trampoline, at its physical address.
    pub entry: u64,
    phoff: usize,
    phentsize: usize,
    phnum: usize,
}

/// One `PT_LOAD`, as a boot loader sees it.
#[derive(Debug)]
pub struct LoadSegment<'a> {
    /// Where the bytes go: **`p_paddr`, not `p_vaddr`.**
    pub paddr: u64,
    /// How much memory the segment occupies. The tail past `data.len()` is `.bss` and must be
    /// zeroed; this loader zeroes the whole span before copying anything, so it is covered.
    pub memsz: u64,
    /// The `p_filesz` bytes from the file, which may be empty for a `NOLOAD` section.
    pub data: &'a [u8],
}

/// Why an embedded kernel could not be read. Every one of these means the build is wrong, so they
/// are printed with their own text rather than a number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// Not an ELF at all.
    NotElf,
    /// Not ELF64, or not little-endian, or not `EM_X86_64`.
    NotNativeElf64,
    /// The program header table runs past the end of the file.
    BadProgramHeaders,
    /// A segment's file range runs past the end of the file.
    BadSegment,
}

impl Error {
    /// A sentence for the firmware console.
    pub const fn text(self) -> &'static str {
        match self {
            Error::NotElf => "the embedded kernel is not an ELF file",
            Error::NotNativeElf64 => "the embedded kernel is not a little-endian x86-64 ELF64",
            Error::BadProgramHeaders => {
                "the embedded kernel's program header table is out of bounds"
            }
            Error::BadSegment => "an embedded kernel segment runs past the end of the file",
        }
    }
}

const ELF_MAGIC: [u8; 4] = [0x7f, b'E', b'L', b'F'];
const ELFCLASS64: u8 = 2;
const ELFDATA2LSB: u8 = 1;
const EM_X86_64: u16 = 62;
const PT_LOAD: u32 = 1;
const EHDR_LEN: usize = 64;

impl<'a> Image<'a> {
    /// Read the header. See the module's note on why this is not a validating parser.
    pub fn parse(bytes: &'a [u8]) -> Result<Self, Error> {
        if bytes.len() < EHDR_LEN || bytes[0..4] != ELF_MAGIC {
            return Err(Error::NotElf);
        }
        if bytes[4] != ELFCLASS64 || bytes[5] != ELFDATA2LSB || u16(bytes, 18) != EM_X86_64 {
            return Err(Error::NotNativeElf64);
        }

        let entry = u64(bytes, 24);
        let phoff = u64(bytes, 32) as usize;
        let phentsize = u16(bytes, 54) as usize;
        let phnum = u16(bytes, 56) as usize;

        let table = phentsize
            .checked_mul(phnum)
            .and_then(|n| phoff.checked_add(n))
            .ok_or(Error::BadProgramHeaders)?;
        if phentsize < 56 || table > bytes.len() {
            return Err(Error::BadProgramHeaders);
        }

        Ok(Image {
            bytes,
            entry,
            phoff,
            phentsize,
            phnum,
        })
    }

    /// Every `PT_LOAD`, in file order, with the physical placement a boot loader needs.
    ///
    /// **`NOLOAD` sections show up here with `data.len() == 0` and a non-zero `memsz`**, which is
    /// how `.boot_scratch` (the boot page tables) and the two per-CPU stack areas arrive. They are
    /// address space to reserve rather than bytes to copy, and getting that wrong is not subtle:
    /// the kernel's trampoline zeroes its own page tables in `.boot_scratch` on the assumption that
    /// nothing else owns them.
    pub fn load_segments(&self) -> impl Iterator<Item = Result<LoadSegment<'a>, Error>> + '_ {
        (0..self.phnum).filter_map(move |i| {
            let at = self.phoff + i * self.phentsize;
            if u32(self.bytes, at) != PT_LOAD {
                return None;
            }
            let offset = u64(self.bytes, at + 8) as usize;
            let paddr = u64(self.bytes, at + 24);
            let filesz = u64(self.bytes, at + 32) as usize;
            let memsz = u64(self.bytes, at + 40);

            let Some(end) = offset
                .checked_add(filesz)
                .filter(|e| *e <= self.bytes.len())
            else {
                return Some(Err(Error::BadSegment));
            };
            Some(Ok(LoadSegment {
                paddr,
                memsz,
                data: &self.bytes[offset..end],
            }))
        })
    }

    /// The `[first, last)` physical range every `PT_LOAD` falls in, page-aligned outwards.
    ///
    /// This is what the firmware is asked for in one `AllocatePages(AllocateAddress)` call rather
    /// than segment by segment: the segments are contiguous by construction (one linker script),
    /// and asking once means one failure to report instead of thirteen.
    pub fn physical_span(&self, page_size: u64) -> Result<(u64, u64), Error> {
        let mut first = u64::MAX;
        let mut last = 0u64;
        for segment in self.load_segments() {
            let segment = segment?;
            first = first.min(segment.paddr);
            last = last.max(segment.paddr.saturating_add(segment.memsz));
        }
        if first == u64::MAX {
            return Err(Error::BadProgramHeaders);
        }
        Ok((
            first & !(page_size - 1),
            last.div_ceil(page_size).saturating_mul(page_size),
        ))
    }
}

fn u16(bytes: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([bytes[at], bytes[at + 1]])
}

fn u32(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap())
}

fn u64(bytes: &[u8], at: usize) -> u64 {
    u64::from_le_bytes(bytes[at..at + 8].try_into().unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal ELF64 header plus `n` program headers, built by hand so the tests do not need a
    /// real kernel on disk.
    fn forge(segments: &[(u64, u64, usize, usize)]) -> Vec<u8> {
        let phentsize = 56usize;
        let phoff = EHDR_LEN;
        let mut bytes = std::vec![0u8; EHDR_LEN + phentsize * segments.len()];
        bytes[0..4].copy_from_slice(&ELF_MAGIC);
        bytes[4] = ELFCLASS64;
        bytes[5] = ELFDATA2LSB;
        bytes[18..20].copy_from_slice(&EM_X86_64.to_le_bytes());
        bytes[24..32].copy_from_slice(&0x10_1000u64.to_le_bytes());
        bytes[32..40].copy_from_slice(&(phoff as u64).to_le_bytes());
        bytes[54..56].copy_from_slice(&(phentsize as u16).to_le_bytes());
        bytes[56..58].copy_from_slice(&(segments.len() as u16).to_le_bytes());

        for (i, (paddr, memsz, offset, filesz)) in segments.iter().enumerate() {
            let at = phoff + i * phentsize;
            bytes[at..at + 4].copy_from_slice(&PT_LOAD.to_le_bytes());
            bytes[at + 8..at + 16].copy_from_slice(&(*offset as u64).to_le_bytes());
            bytes[at + 24..at + 32].copy_from_slice(&paddr.to_le_bytes());
            bytes[at + 32..at + 40].copy_from_slice(&(*filesz as u64).to_le_bytes());
            bytes[at + 40..at + 48].copy_from_slice(&memsz.to_le_bytes());
        }
        let needed = segments.iter().map(|s| s.2 + s.3).max().unwrap_or(0);
        if bytes.len() < needed {
            bytes.resize(needed, 0xab);
        }
        bytes
    }

    #[test]
    fn the_entry_and_the_segments_come_back() {
        let bytes = forge(&[(0x10_0000, 0x20, EHDR_LEN + 56, 0x20)]);
        let image = Image::parse(&bytes).unwrap_or_else(|e| panic!("{}", e.text()));
        assert_eq!(image.entry, 0x10_1000);

        let segments: Vec<_> = image
            .load_segments()
            .map(|s| s.unwrap_or_else(|e| panic!("{}", e.text())))
            .collect();
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].paddr, 0x10_0000);
        assert_eq!(segments[0].data.len(), 0x20);
    }

    /// The property the whole module exists for: the span is over **`p_paddr`**, so an image whose
    /// virtual addresses are in the high half still reports a low physical span.
    #[test]
    fn the_span_is_physical_and_page_aligned_outwards() {
        let bytes = forge(&[
            (0x10_0000, 0x14, EHDR_LEN + 112, 0x14),
            (0x10_9000, 0x4c000, EHDR_LEN + 112, 0),
            (0x23_8123, 0x100, EHDR_LEN + 112, 0),
        ]);
        let image = Image::parse(&bytes).unwrap();
        assert_eq!(image.physical_span(4096).unwrap(), (0x10_0000, 0x239_000));
    }

    /// A `NOLOAD` section arrives as a segment with no bytes and a real size, and it has to be
    /// counted in the span or the firmware hands its memory to something else.
    #[test]
    fn a_noload_segment_still_reserves_its_memory() {
        let bytes = forge(&[(0x10_2000, 0x7000, EHDR_LEN + 56, 0)]);
        let image = Image::parse(&bytes).unwrap();
        let segment = image.load_segments().next().unwrap().unwrap();
        assert!(segment.data.is_empty(), "NOLOAD contributes no file bytes");
        assert_eq!(segment.memsz, 0x7000);
        assert_eq!(image.physical_span(4096).unwrap(), (0x10_2000, 0x109_000));
    }

    #[test]
    fn a_file_that_is_not_our_elf_is_refused_rather_than_read() {
        assert_eq!(
            Image::parse(b"not an elf at all, really").unwrap_err(),
            Error::NotElf
        );

        let mut bytes = forge(&[(0x10_0000, 0x20, EHDR_LEN + 56, 0x20)]);
        bytes[4] = 1; // ELFCLASS32
        assert_eq!(Image::parse(&bytes).unwrap_err(), Error::NotNativeElf64);

        let mut bytes = forge(&[(0x10_0000, 0x20, EHDR_LEN + 56, 0x20)]);
        bytes[18..20].copy_from_slice(&183u16.to_le_bytes()); // EM_AARCH64
        assert_eq!(Image::parse(&bytes).unwrap_err(), Error::NotNativeElf64);
    }

    #[test]
    fn a_program_header_table_past_the_end_is_refused() {
        let mut bytes = forge(&[(0x10_0000, 0x20, EHDR_LEN + 56, 0x20)]);
        bytes[56..58].copy_from_slice(&500u16.to_le_bytes());
        assert_eq!(Image::parse(&bytes).unwrap_err(), Error::BadProgramHeaders);
    }

    #[test]
    fn a_segment_whose_file_range_runs_off_the_end_is_refused() {
        let mut bytes = forge(&[(0x10_0000, 0x20, EHDR_LEN + 56, 0x20)]);
        // Patched after forging rather than declared: `forge` grows the file to hold whatever it
        // was asked for, so a segment can only be made to overrun by widening it afterwards.
        let filesz_at = EHDR_LEN + 32;
        bytes[filesz_at..filesz_at + 8].copy_from_slice(&(1u64 << 20).to_le_bytes());
        let image = Image::parse(&bytes).unwrap();
        assert_eq!(
            image.load_segments().next().unwrap().unwrap_err(),
            Error::BadSegment
        );
    }
}
