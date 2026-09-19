//! **Turning a position-independent ELF into the PE/COFF image UEFI firmware loads.**
//!
//! Name: provisional. Minted 2026-09-19 by the lane that built it
//! (`milestone/the-program-that-makes-the-stick`).
//! The format's own name, spelled out as design/naming.md asks of an acronym people say whole ("PE"
//! is what nobody expands, but "portable executable" is the phrase the specification's title uses,
//! which puts it on the spelled-out side of §154's test; calef's call either way). Considered and set
//! aside `elf_to_efi` (a verb phrase, and systemd's tool name) and `pe_image` (the acronym).
//!
//! UEFI loads applications in Microsoft's PE/COFF format. rustc builds PE directly for the three
//! UEFI targets it has (`aarch64-`, `i686-`, `x86_64-unknown-uefi`) and has **no riscv64 UEFI
//! target** (`rustc --print target-list` on the pinned nightly, 2026-09-19). So the riscv64 boot file
//! is built as an ordinary static-PIE ELF for `riscv64imac-unknown-none-elf` and converted here.
//! That is the approach systemd's `elf2efi` takes for every architecture it supports (recalled, not
//! read); U-Boot and gnu-efi instead hand-write a PE header in assembly at the front of the image.
//! This one was chosen because it is pure data transformation, so it is host-tested in
//! milliseconds, and the loader's own source stays free of a header written byte by byte.
//!
//! # What goes in and what comes out
//!
//! In: a 64-bit little-endian ELF of type `ET_DYN` (a PIE), linked with its segments page-aligned
//! (`-z separate-loadable-segments`) and its first segment at or above `0x1000`, whose only dynamic
//! relocations are the architecture's `RELATIVE` kind. That is what `cargo xtask`'s riscv64 build
//! produces, and anything else is refused by name rather than converted wrongly.
//!
//! Out: a PE32+ image whose section RVAs **are** the ELF's virtual addresses (image base 0), one
//! section per `PT_LOAD`, and a `.reloc` section holding one `IMAGE_REL_BASED_DIR64` fixup per ELF
//! relocation. The firmware loads the image at some base `B` and adds `B` to each fixed-up word;
//! this converter has already written each relocation's addend into its word, so the result is
//! `B + addend`, which is exactly what `R_*_RELATIVE` means.
//!
//! # Why the ELF is read here and not with `crates/elf`
//!
//! `crates/elf` is the tree's one ELF *loader* reader, and it is built around the machine it runs
//! on: `Elf::parse` refuses any machine but `NATIVE_MACHINE`, and refuses `ET_DYN` outright
//! (`NeedsRelocation`), both correctly for what it is for. This runs on the build host and reads a
//! different architecture's PIE, which is two refusals in, so it reads the four structures it needs
//! (the file header, program headers, the dynamic section and `Elf64_Rela`) itself. They are named
//! and bounded below, and every read is checked.
//!
//! # BUGS
//!
//! - **Only `RELATIVE` relocations are converted.** A static PIE with no dynamic symbols produces
//!   nothing else, and anything else is refused with its type number.
//! - **The image carries no debug directory and no symbols**, so a firmware debugger sees addresses
//!   only. The ELF beside it has them.
//! - **The PE timestamp is zero**, deliberately: the image is a function of the ELF, so two builds of
//!   one ELF are byte-identical.

#![cfg_attr(not(test), no_std)]

// `no_std` with `alloc`, like the tree's other pure crates: nothing here needs an operating system,
// so it builds and documents for the bare-metal targets alongside them rather than joining the
// host-only set `script/lint` has to name.
extern crate alloc;

use alloc::vec;
use alloc::vec::Vec;

/// `IMAGE_FILE_MACHINE_*` for the machines this converts, by ELF `e_machine`.
fn pe_machine(elf_machine: u16) -> Option<u16> {
    match elf_machine {
        62 => Some(0x8664),  // EM_X86_64 -> IMAGE_FILE_MACHINE_AMD64
        183 => Some(0xaa64), // EM_AARCH64 -> IMAGE_FILE_MACHINE_ARM64
        243 => Some(0x5064), // EM_RISCV -> IMAGE_FILE_MACHINE_RISCV64
        _ => None,
    }
}

/// The `R_*_RELATIVE` relocation type for each machine.
fn relative_type(elf_machine: u16) -> u32 {
    match elf_machine {
        62 => 8,     // R_X86_64_RELATIVE
        183 => 1027, // R_AARCH64_RELATIVE
        _ => 3,      // R_RISCV_RELATIVE
    }
}

/// Why an ELF could not be converted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// Not a 64-bit little-endian ELF.
    NotElf64,
    /// Not `ET_DYN`: a position-dependent image cannot be loaded where the firmware chooses.
    NotPositionIndependent,
    /// An `e_machine` this does not know a PE machine number for.
    UnknownMachine(u16),
    /// A header, table or segment runs past the end of the file.
    Truncated,
    /// A `PT_LOAD` whose address is not a multiple of 4 KiB, or that starts below `0x1000`, where
    /// the PE headers go.
    SegmentLayout,
    /// A dynamic relocation other than `RELATIVE`, by type number.
    UnsupportedRelocation(u32),
    /// `DT_REL`, `DT_RELR` or `DT_JMPREL` tables, which this does not convert.
    UnsupportedRelocationTable,
    /// A relocation outside every segment's file bytes.
    RelocationOutsideImage(u64),
}

const PAGE: u64 = 0x1000;
const FILE_ALIGNMENT: u64 = 0x200;

fn u16_at(b: &[u8], at: usize) -> Result<u16, Error> {
    let s = b.get(at..at + 2).ok_or(Error::Truncated)?;
    Ok(u16::from_le_bytes([s[0], s[1]]))
}
fn u32_at(b: &[u8], at: usize) -> Result<u32, Error> {
    let s = b.get(at..at + 4).ok_or(Error::Truncated)?;
    Ok(u32::from_le_bytes(s.try_into().unwrap_or([0; 4])))
}
fn u64_at(b: &[u8], at: usize) -> Result<u64, Error> {
    let s = b.get(at..at + 8).ok_or(Error::Truncated)?;
    Ok(u64::from_le_bytes(s.try_into().unwrap_or([0; 8])))
}
fn align(value: u64, to: u64) -> u64 {
    value.div_ceil(to) * to
}
fn usize_of(v: u64) -> Result<usize, Error> {
    usize::try_from(v).map_err(|_| Error::Truncated)
}

/// One `PT_LOAD`.
#[derive(Debug, Clone, Copy)]
struct Load {
    offset: u64,
    vaddr: u64,
    filesz: u64,
    memsz: u64,
    flags: u32,
}

/// **Convert** (see the module documentation).
pub fn from_elf(elf: &[u8]) -> Result<Vec<u8>, Error> {
    if elf.get(0..4) != Some(b"\x7fELF".as_slice())
        || elf.get(4) != Some(&2)
        || elf.get(5) != Some(&1)
    {
        return Err(Error::NotElf64);
    }
    if u16_at(elf, 16)? != 3 {
        return Err(Error::NotPositionIndependent);
    }
    let machine = u16_at(elf, 18)?;
    let pe_machine = pe_machine(machine).ok_or(Error::UnknownMachine(machine))?;
    let entry = u64_at(elf, 24)?;
    let phoff = usize_of(u64_at(elf, 32)?)?;
    let phentsize = usize::from(u16_at(elf, 54)?);
    let phnum = usize::from(u16_at(elf, 56)?);

    let mut loads = Vec::new();
    let mut dynamic: Option<(u64, u64)> = None;
    for i in 0..phnum {
        let at = phoff + i * phentsize;
        let kind = u32_at(elf, at)?;
        let flags = u32_at(elf, at + 4)?;
        let offset = u64_at(elf, at + 8)?;
        let vaddr = u64_at(elf, at + 16)?;
        let filesz = u64_at(elf, at + 32)?;
        let memsz = u64_at(elf, at + 40)?;
        match kind {
            1 => loads.push(Load {
                offset,
                vaddr,
                filesz,
                memsz,
                flags,
            }),
            2 => dynamic = Some((offset, filesz)),
            _ => {}
        }
    }
    loads.sort_by_key(|l| l.vaddr);
    for load in &loads {
        if load.vaddr % PAGE != 0 || load.vaddr < PAGE || load.filesz > load.memsz {
            return Err(Error::SegmentLayout);
        }
        let end = load
            .offset
            .checked_add(load.filesz)
            .ok_or(Error::Truncated)?;
        if usize_of(end)? > elf.len() {
            return Err(Error::Truncated);
        }
    }
    for pair in loads.windows(2) {
        if pair[0].vaddr + pair[0].memsz > pair[1].vaddr {
            return Err(Error::SegmentLayout);
        }
    }
    let image_end = loads
        .last()
        .map(|l| l.vaddr + l.memsz)
        .ok_or(Error::SegmentLayout)?;

    // The memory image, from address 0, holding every segment's file bytes where it will run.
    let mut image = vec![0u8; usize_of(image_end)?];
    for load in &loads {
        let (from, to) = (usize_of(load.offset)?, usize_of(load.vaddr)?);
        let len = usize_of(load.filesz)?;
        image[to..to + len].copy_from_slice(&elf[from..from + len]);
    }

    // The dynamic relocations.
    let mut fixups: Vec<u64> = Vec::new();
    if let Some((offset, size)) = dynamic {
        let (mut rela, mut relasz, mut relaent) = (None, 0u64, 24u64);
        let mut at = usize_of(offset)?;
        let end = at + usize_of(size)?;
        while at + 16 <= end {
            let tag = u64_at(elf, at)?;
            let value = u64_at(elf, at + 8)?;
            at += 16;
            match tag {
                0 => break,
                7 => rela = Some(value),
                8 => relasz = value,
                9 => relaent = value,
                17 | 36 => return Err(Error::UnsupportedRelocationTable),
                2 | 23 if value != 0 => return Err(Error::UnsupportedRelocationTable),
                _ => {}
            }
        }
        if let Some(rela) = rela {
            // `DT_RELA` is a virtual address; its bytes are in the image built above.
            let base = usize_of(rela)?;
            let count = usize_of(relasz / relaent.max(1))?;
            for i in 0..count {
                let at = base + i * usize_of(relaent)?;
                let r_offset = u64_at(&image, at)?;
                let r_info = u64_at(&image, at + 8)?;
                let r_addend = u64_at(&image, at + 16)?;
                let kind = (r_info & 0xffff_ffff) as u32;
                if kind == 0 {
                    continue; // R_*_NONE
                }
                if kind != relative_type(machine) || r_info >> 32 != 0 {
                    return Err(Error::UnsupportedRelocation(kind));
                }
                let inside = loads
                    .iter()
                    .any(|l| r_offset >= l.vaddr && r_offset + 8 <= l.vaddr + l.memsz);
                if !inside {
                    return Err(Error::RelocationOutsideImage(r_offset));
                }
                let word = usize_of(r_offset)?;
                image[word..word + 8].copy_from_slice(&r_addend.to_le_bytes());
                fixups.push(r_offset);
            }
        }
    }
    fixups.sort_unstable();
    fixups.dedup();
    let relocations = base_relocations(&fixups);

    // --- Lay out the PE file ---
    struct Section {
        name: [u8; 8],
        virtual_address: u64,
        virtual_size: u64,
        data: Vec<u8>,
        characteristics: u32,
    }
    let mut sections: Vec<Section> = loads
        .iter()
        .map(|l| {
            let (name, characteristics) = if l.flags & 1 != 0 {
                (*b".text\0\0\0", 0x6000_0020) // CODE | EXECUTE | READ
            } else if l.flags & 2 != 0 {
                (*b".data\0\0\0", 0xc000_0040) // INITIALIZED_DATA | READ | WRITE
            } else {
                (*b".rdata\0\0", 0x4000_0040) // INITIALIZED_DATA | READ
            };
            let from = l.vaddr as usize;
            Section {
                name,
                virtual_address: l.vaddr,
                virtual_size: l.memsz,
                data: image[from..from + l.filesz as usize].to_vec(),
                characteristics,
            }
        })
        .collect();
    let reloc_va = align(image_end, PAGE);
    if !relocations.is_empty() {
        sections.push(Section {
            name: *b".reloc\0\0",
            virtual_address: reloc_va,
            virtual_size: relocations.len() as u64,
            data: relocations.clone(),
            characteristics: 0x4200_0040, // INITIALIZED_DATA | DISCARDABLE | READ
        });
    }

    const DOS_LEN: usize = 0x40;
    const COFF_LEN: usize = 20;
    const OPTIONAL_LEN: usize = 240;
    let headers = DOS_LEN + 4 + COFF_LEN + OPTIONAL_LEN + 40 * sections.len();
    let size_of_headers = align(headers as u64, FILE_ALIGNMENT);
    if size_of_headers > PAGE {
        return Err(Error::SegmentLayout);
    }
    let size_of_image = align(
        sections
            .iter()
            .map(|s| s.virtual_address + s.virtual_size)
            .max()
            .unwrap_or(PAGE),
        PAGE,
    );

    let mut raw_at = size_of_headers;
    let mut placements = Vec::new();
    for s in &sections {
        let raw_size = align(s.data.len() as u64, FILE_ALIGNMENT);
        placements.push((raw_at, raw_size));
        raw_at += raw_size;
    }
    let mut out = vec![0u8; usize_of(raw_at)?];

    let put16 =
        |out: &mut Vec<u8>, at: usize, v: u16| out[at..at + 2].copy_from_slice(&v.to_le_bytes());
    let put32 =
        |out: &mut Vec<u8>, at: usize, v: u32| out[at..at + 4].copy_from_slice(&v.to_le_bytes());
    let put64 =
        |out: &mut Vec<u8>, at: usize, v: u64| out[at..at + 8].copy_from_slice(&v.to_le_bytes());
    let narrow = |v: u64| u32::try_from(v).map_err(|_| Error::SegmentLayout);

    // DOS header: the magic and the offset of the PE signature, nothing else.
    out[0..2].copy_from_slice(b"MZ");
    put32(&mut out, 0x3c, DOS_LEN as u32);
    out[DOS_LEN..DOS_LEN + 4].copy_from_slice(b"PE\0\0");

    let coff = DOS_LEN + 4;
    put16(&mut out, coff, pe_machine);
    put16(&mut out, coff + 2, sections.len() as u16);
    // TimeDateStamp, PointerToSymbolTable, NumberOfSymbols: zero.
    put16(&mut out, coff + 16, OPTIONAL_LEN as u16);
    // EXECUTABLE_IMAGE | LARGE_ADDRESS_AWARE | DEBUG_STRIPPED.
    put16(&mut out, coff + 18, 0x0002 | 0x0020 | 0x0200);

    let opt = coff + COFF_LEN;
    let code: u64 = sections
        .iter()
        .filter(|s| s.characteristics & 0x20 != 0)
        .map(|s| align(s.data.len() as u64, FILE_ALIGNMENT))
        .sum();
    let data: u64 = sections
        .iter()
        .filter(|s| s.characteristics & 0x20 == 0)
        .map(|s| align(s.data.len() as u64, FILE_ALIGNMENT))
        .sum();
    let base_of_code = sections
        .iter()
        .find(|s| s.characteristics & 0x20 != 0)
        .map_or(0, |s| s.virtual_address);
    put16(&mut out, opt, 0x20b); // PE32+
    put32(&mut out, opt + 4, narrow(code)?);
    put32(&mut out, opt + 8, narrow(data)?);
    put32(&mut out, opt + 16, narrow(entry)?);
    put32(&mut out, opt + 20, narrow(base_of_code)?);
    put64(&mut out, opt + 24, 0); // ImageBase: the RVAs are the ELF's addresses from zero
    put32(&mut out, opt + 32, PAGE as u32); // SectionAlignment
    put32(&mut out, opt + 36, FILE_ALIGNMENT as u32);
    put32(&mut out, opt + 56, narrow(size_of_image)?);
    put32(&mut out, opt + 60, narrow(size_of_headers)?);
    put16(&mut out, opt + 68, 10); // IMAGE_SUBSYSTEM_EFI_APPLICATION
    // DllCharacteristics: HIGH_ENTROPY_VA | DYNAMIC_BASE | NX_COMPAT, which is what the image is.
    put16(&mut out, opt + 70, 0x0020 | 0x0040 | 0x0100);
    put32(&mut out, opt + 108, 16); // NumberOfRvaAndSizes
    if !relocations.is_empty() {
        // Data directory 5: the base relocation table.
        put32(&mut out, opt + 112 + 5 * 8, narrow(reloc_va)?);
        put32(&mut out, opt + 112 + 5 * 8 + 4, relocations.len() as u32);
    }

    let table = opt + OPTIONAL_LEN;
    for (i, (s, (raw_at, raw_size))) in sections.iter().zip(&placements).enumerate() {
        let h = table + i * 40;
        out[h..h + 8].copy_from_slice(&s.name);
        put32(&mut out, h + 8, narrow(s.virtual_size)?);
        put32(&mut out, h + 12, narrow(s.virtual_address)?);
        put32(&mut out, h + 16, narrow(*raw_size)?);
        put32(&mut out, h + 20, narrow(*raw_at)?);
        put32(&mut out, h + 36, s.characteristics);
        let at = usize_of(*raw_at)?;
        out[at..at + s.data.len()].copy_from_slice(&s.data);
    }
    Ok(out)
}

/// **The `.reloc` section**: fixups grouped into one block per 4 KiB page, each block a page RVA,
/// a block size, and a 16-bit entry per fixup (type in the top four bits, offset in the low
/// twelve), padded to four bytes with an `IMAGE_REL_BASED_ABSOLUTE` entry, which is a no-op.
fn base_relocations(sorted: &[u64]) -> Vec<u8> {
    const DIR64: u16 = 10;
    let mut out = Vec::new();
    let mut i = 0;
    while i < sorted.len() {
        let page = sorted[i] & !(PAGE - 1);
        let mut entries: Vec<u16> = Vec::new();
        while i < sorted.len() && sorted[i] & !(PAGE - 1) == page {
            entries.push((DIR64 << 12) | (sorted[i] & (PAGE - 1)) as u16);
            i += 1;
        }
        if entries.len() % 2 == 1 {
            entries.push(0);
        }
        out.extend_from_slice(&(page as u32).to_le_bytes());
        out.extend_from_slice(&((8 + 2 * entries.len()) as u32).to_le_bytes());
        for e in entries {
            out.extend_from_slice(&e.to_le_bytes());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal riscv64 PIE: a code segment at 0x1000, a data segment at 0x2000 holding one pointer
    /// to 0x1010, a dynamic section and one `R_RISCV_RELATIVE` for that pointer.
    fn tiny_pie(reloc_type: u32) -> Vec<u8> {
        let mut e = vec![0u8; 0x3000];
        e[0..4].copy_from_slice(b"\x7fELF");
        e[4] = 2;
        e[5] = 1;
        e[6] = 1;
        e[16..18].copy_from_slice(&3u16.to_le_bytes()); // ET_DYN
        e[18..20].copy_from_slice(&243u16.to_le_bytes()); // EM_RISCV
        e[24..32].copy_from_slice(&0x1004u64.to_le_bytes()); // entry
        e[32..40].copy_from_slice(&64u64.to_le_bytes()); // phoff
        e[54..56].copy_from_slice(&56u16.to_le_bytes());
        e[56..58].copy_from_slice(&3u16.to_le_bytes());
        let ph = |e: &mut Vec<u8>,
                  i: usize,
                  kind: u32,
                  flags: u32,
                  off: u64,
                  va: u64,
                  fsz: u64,
                  msz: u64| {
            let at = 64 + i * 56;
            e[at..at + 4].copy_from_slice(&kind.to_le_bytes());
            e[at + 4..at + 8].copy_from_slice(&flags.to_le_bytes());
            e[at + 8..at + 16].copy_from_slice(&off.to_le_bytes());
            e[at + 16..at + 24].copy_from_slice(&va.to_le_bytes());
            e[at + 32..at + 40].copy_from_slice(&fsz.to_le_bytes());
            e[at + 40..at + 48].copy_from_slice(&msz.to_le_bytes());
        };
        ph(&mut e, 0, 1, 5, 0x1000, 0x1000, 0x20, 0x20); // R X
        ph(&mut e, 1, 1, 6, 0x2000, 0x2000, 0x100, 0x1100); // R W, with .bss
        ph(&mut e, 2, 2, 6, 0x2040, 0x2040, 0x40, 0x40); // DYNAMIC
        e[0x1000..0x1004].copy_from_slice(&0x0000_0013u32.to_le_bytes()); // nop
        // The pointer word at 0x2008 holds zero; the relocation's addend is 0x1010.
        let dyn_at = 0x2040;
        for (i, (tag, val)) in [(7u64, 0x2080u64), (8, 24), (9, 24), (0, 0)]
            .iter()
            .enumerate()
        {
            e[dyn_at + i * 16..dyn_at + i * 16 + 8].copy_from_slice(&tag.to_le_bytes());
            e[dyn_at + i * 16 + 8..dyn_at + i * 16 + 16].copy_from_slice(&val.to_le_bytes());
        }
        e[0x2080..0x2088].copy_from_slice(&0x2008u64.to_le_bytes());
        e[0x2088..0x2090].copy_from_slice(&u64::from(reloc_type).to_le_bytes());
        e[0x2090..0x2098].copy_from_slice(&0x1010u64.to_le_bytes());
        e
    }

    fn u16le(b: &[u8], at: usize) -> u16 {
        u16::from_le_bytes([b[at], b[at + 1]])
    }
    fn u32le(b: &[u8], at: usize) -> u32 {
        u32::from_le_bytes(b[at..at + 4].try_into().unwrap())
    }

    #[test]
    fn a_pie_becomes_a_pe_whose_rvas_are_its_addresses() {
        let pe = from_elf(&tiny_pie(3)).unwrap();
        assert_eq!(&pe[0..2], b"MZ");
        let pe_at = u32le(&pe, 0x3c) as usize;
        assert_eq!(&pe[pe_at..pe_at + 4], b"PE\0\0");
        let coff = pe_at + 4;
        assert_eq!(u16le(&pe, coff), 0x5064, "IMAGE_FILE_MACHINE_RISCV64");
        assert_eq!(u16le(&pe, coff + 2), 3, ".text, .data, .reloc");
        let opt = coff + 20;
        assert_eq!(u16le(&pe, opt), 0x20b);
        assert_eq!(u32le(&pe, opt + 16), 0x1004, "the entry point is the ELF's");
        assert_eq!(u16le(&pe, opt + 68), 10, "an EFI application");
        assert_eq!(
            u32le(&pe, opt + 56),
            0x5000,
            "SizeOfImage covers .bss and then .reloc on its own page"
        );

        let sections = opt + 240;
        let section = |i: usize| {
            let h = sections + i * 40;
            (
                &pe[h..h + 8],
                u32le(&pe, h + 8),
                u32le(&pe, h + 12),
                u32le(&pe, h + 20),
            )
        };
        let (name, vsize, va, raw) = section(0);
        assert_eq!((name, vsize, va), (b".text\0\0\0".as_slice(), 0x20, 0x1000));
        assert_eq!(&pe[raw as usize..raw as usize + 4], &0x13u32.to_le_bytes());
        let (name, vsize, va, raw) = section(1);
        assert_eq!(
            (name, vsize, va),
            (b".data\0\0\0".as_slice(), 0x1100, 0x2000)
        );
        // The pointer word now holds the addend, so firmware adding the load base makes it right.
        assert_eq!(
            u64::from_le_bytes(pe[raw as usize + 8..raw as usize + 16].try_into().unwrap()),
            0x1010
        );
        let (name, _, va, raw) = section(2);
        assert_eq!(name, b".reloc\0\0");
        assert_eq!(va, 0x4000);
        let block = &pe[raw as usize..];
        assert_eq!(u32le(block, 0), 0x2000, "the page");
        assert_eq!(
            u32le(block, 4),
            12,
            "header plus two entries (one real, one pad)"
        );
        assert_eq!(u16le(block, 8), (10 << 12) | 0x008, "DIR64 at offset 8");
        assert_eq!(u16le(block, 10), 0, "ABSOLUTE padding");
        assert_eq!(
            u32le(&pe, opt + 112 + 40),
            0x4000,
            "data directory 5 names .reloc"
        );
    }

    #[test]
    fn converting_twice_is_byte_identical() {
        assert_eq!(from_elf(&tiny_pie(3)), from_elf(&tiny_pie(3)));
    }

    #[test]
    fn refuses_what_it_cannot_convert_correctly() {
        assert_eq!(from_elf(&tiny_pie(2)), Err(Error::UnsupportedRelocation(2)));
        let mut exec = tiny_pie(3);
        exec[16] = 2;
        assert_eq!(from_elf(&exec), Err(Error::NotPositionIndependent));
        assert_eq!(from_elf(b"MZ not an elf"), Err(Error::NotElf64));
        let mut odd = tiny_pie(3);
        odd[64 + 16..64 + 24].copy_from_slice(&0x1234u64.to_le_bytes());
        assert_eq!(from_elf(&odd), Err(Error::SegmentLayout));
    }

    #[test]
    fn relocation_blocks_split_by_page_and_pad_to_four_bytes() {
        let r = base_relocations(&[0x2000, 0x2008, 0x3ff8]);
        assert_eq!(u32le(&r, 0), 0x2000);
        assert_eq!(u32le(&r, 4), 12);
        assert_eq!(u32le(&r, 12), 0x3000);
        assert_eq!(u32le(&r, 16), 12, "one entry padded to two");
        assert_eq!(r.len(), 24);
    }
}
