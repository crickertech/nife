//! **Telling a device-tree kernel where its archive is**: a copy of the firmware's device tree with
//! `/chosen/linux,initrd-start` and `linux,initrd-end` set.
//!
//! On aarch64 and riscv64 the kernel learns about the machine from a flattened device tree, and it
//! finds the userspace archive the way Linux finds an initrd: two properties under `/chosen`
//! (`memory::init`, via `device_tree_blob::DeviceTreeBlob::initrd`). QEMU's `-initrd` writes them
//! and U-Boot's `booti` writes them. **UEFI firmware does not**, because a UEFI loader places the
//! archive itself and is expected to say so; this is that saying. It is the same job the Linux EFI
//! stub does in `drivers/firmware/efi/libstub/fdt.c` (recalled, not read), done here in the
//! smallest form that works: rebuild the blob with the two properties, and nothing else changed.
//!
//! **Pure and host-tested**, like the rest of this library: every test decodes the output with the
//! crate the kernel decodes with, so the writer and the reader cannot drift apart without a test
//! failing in milliseconds. That is the rule `handoff.rs` already follows for x86's
//! `hvm_start_info`.
//!
//! The format is the Devicetree Specification v0.4, chapter 5 (read against the tree's own reader,
//! `crates/device_tree_blob`, which is what actually has to agree).
//!
//! # BUGS
//!
//! - **The firmware's memory map is not carried into the tree.** Linux's stub replaces the
//!   `/memory` nodes with the UEFI map; this does not, so memory the firmware keeps for its runtime
//!   services is, to the kernel, ordinary RAM. That is harmless for exactly one reason, which is
//!   that nife never calls a UEFI runtime service. The day it does, this has to reserve those ranges
//!   (`/memreserve/` entries or `/reserved-memory` children) first.
//! - **Nothing is removed**: a stale `linux,initrd-*` pair the firmware left is replaced, and every
//!   other property is copied as it was.

/// The device-tree magic, big-endian at offset 0.
pub(crate) const MAGIC: u32 = 0xd00d_feed;
pub(crate) const HEADER_LEN: usize = 40;
pub(crate) const BEGIN_NODE: u32 = 1;
pub(crate) const END_NODE: u32 = 2;
pub(crate) const PROP: u32 = 3;
const NOP: u32 = 4;
pub(crate) const END: u32 = 9;

const INITRD_START: &[u8] = b"linux,initrd-start";
const INITRD_END: &[u8] = b"linux,initrd-end";

/// Why a tree could not be rewritten.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// The magic is wrong: this is not a flattened device tree.
    NotADeviceTree,
    /// A header offset or a token runs past the blob.
    Truncated,
    /// A token this format does not define, or nodes that do not nest.
    Malformed,
    /// The output buffer is smaller than [`output_len`] said it had to be.
    OutputTooSmall,
}

impl Error {
    /// The sentence for the firmware console.
    pub const fn reason(self) -> &'static str {
        match self {
            Error::NotADeviceTree => "the firmware's device tree has the wrong magic",
            Error::Truncated => "the firmware's device tree is truncated",
            Error::Malformed => "the firmware's device tree is malformed",
            Error::OutputTooSmall => "no room to rewrite the device tree",
        }
    }
}

fn be32(bytes: &[u8], at: usize) -> Result<u32, Error> {
    let b = bytes.get(at..at + 4).ok_or(Error::Truncated)?;
    Ok(u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
}

const fn align4(n: usize) -> usize {
    (n + 3) & !3
}

/// The parts of the header this needs.
struct Header {
    total: usize,
    off_struct: usize,
    off_strings: usize,
    off_rsvmap: usize,
    size_strings: usize,
    size_struct: usize,
    boot_cpuid: u32,
}

fn header(src: &[u8]) -> Result<Header, Error> {
    if be32(src, 0)? != MAGIC {
        return Err(Error::NotADeviceTree);
    }
    let h = Header {
        total: be32(src, 4)? as usize,
        off_struct: be32(src, 8)? as usize,
        off_strings: be32(src, 12)? as usize,
        off_rsvmap: be32(src, 16)? as usize,
        boot_cpuid: be32(src, 28)?,
        size_strings: be32(src, 32)? as usize,
        size_struct: be32(src, 36)? as usize,
    };
    if h.total > src.len()
        || h.off_struct
            .checked_add(h.size_struct)
            .is_none_or(|e| e > h.total)
        || h.off_strings
            .checked_add(h.size_strings)
            .is_none_or(|e| e > h.total)
        || h.off_rsvmap >= h.total
    {
        return Err(Error::Truncated);
    }
    Ok(h)
}

/// The size of the reservation block: 16-byte entries up to and including the all-zero one.
fn rsvmap_len(src: &[u8], h: &Header) -> Result<usize, Error> {
    let mut at = h.off_rsvmap;
    loop {
        let entry = src.get(at..at + 16).ok_or(Error::Truncated)?;
        at += 16;
        if entry.iter().all(|&b| b == 0) {
            return Ok(at - h.off_rsvmap);
        }
    }
}

/// **An upper bound on the rewritten tree's size**: the original plus a new `/chosen` node, two
/// eight-byte properties and their two names. The caller allocates this much.
pub fn output_len(src: &[u8]) -> Result<usize, Error> {
    let h = header(src)?;
    let rsv = rsvmap_len(src, &h)?;
    // BEGIN_NODE "chosen" (4 + 8) + END_NODE (4) + two props (12 + 8 each) + two names.
    let extra = 12 + 4 + 2 * 20 + INITRD_START.len() + 1 + INITRD_END.len() + 1;
    Ok(HEADER_LEN + 8 + rsv + h.size_struct + h.size_strings + extra + 8)
}

/// Offset of `name` in the strings block, if it is already there as a whole string.
fn find_string(strings: &[u8], name: &[u8]) -> Option<usize> {
    let mut at = 0;
    while at < strings.len() {
        let end = strings[at..].iter().position(|&b| b == 0)? + at;
        if &strings[at..end] == name {
            return Some(at);
        }
        at = end + 1;
    }
    None
}

pub(crate) struct Out<'a> {
    pub(crate) buf: &'a mut [u8],
    pub(crate) at: usize,
}

impl Out<'_> {
    pub(crate) fn bytes(&mut self, b: &[u8]) -> Result<(), Error> {
        let end = self.at + b.len();
        self.buf
            .get_mut(self.at..end)
            .ok_or(Error::OutputTooSmall)?
            .copy_from_slice(b);
        self.at = end;
        Ok(())
    }
    pub(crate) fn u32(&mut self, v: u32) -> Result<(), Error> {
        self.bytes(&v.to_be_bytes())
    }
    pub(crate) fn pad_to(&mut self, align: usize) -> Result<(), Error> {
        while !self.at.is_multiple_of(align) {
            self.bytes(&[0])?;
        }
        Ok(())
    }
    fn prop_u64(&mut self, name_off: usize, value: u64) -> Result<(), Error> {
        self.u32(PROP)?;
        self.u32(8)?;
        self.u32(name_off as u32)?;
        self.bytes(&value.to_be_bytes())
    }
}

/// **Rewrite `src` into `out` with `/chosen/linux,initrd-{start,end}` set to `[start, end)`**, and
/// return the new tree's length. `out` must be at least [`output_len`] bytes.
///
/// `/chosen` is created as the root's last child if the firmware's tree has none.
pub fn with_initrd(src: &[u8], start: u64, end: u64, out: &mut [u8]) -> Result<usize, Error> {
    let h = header(src)?;
    let rsv = rsvmap_len(src, &h)?;
    let strings = &src[h.off_strings..h.off_strings + h.size_strings];

    // Name offsets: reuse the firmware's where the names already exist, append otherwise.
    let mut appended: [&[u8]; 2] = [&[], &[]];
    let mut appended_count = 0;
    let mut next_off = h.size_strings;
    let mut name_off = |name: &'static [u8]| -> usize {
        if let Some(off) = find_string(strings, name) {
            return off;
        }
        let off = next_off;
        next_off += name.len() + 1;
        appended[appended_count] = name;
        appended_count += 1;
        off
    };
    let start_off = name_off(INITRD_START);
    let end_off = name_off(INITRD_END);

    let mut o = Out { buf: out, at: 0 };
    o.bytes(&[0; HEADER_LEN])?; // filled in last
    o.pad_to(8)?;
    let new_rsvmap = o.at;
    o.bytes(&src[h.off_rsvmap..h.off_rsvmap + rsv])?;
    let new_struct = o.at;

    let mut at = h.off_struct;
    let struct_end = h.off_struct + h.size_struct;
    let mut depth = 0usize;
    let mut in_chosen = false;
    let mut wrote_chosen = false;
    loop {
        if at >= struct_end {
            return Err(Error::Truncated);
        }
        let token = be32(src, at)?;
        match token {
            BEGIN_NODE => {
                let name_at = at + 4;
                let name_len = src
                    .get(name_at..struct_end)
                    .and_then(|s| s.iter().position(|&b| b == 0))
                    .ok_or(Error::Truncated)?;
                let next = name_at + align4(name_len + 1);
                o.bytes(&src[at..next])?;
                depth += 1;
                if depth == 2 && &src[name_at..name_at + name_len] == b"chosen" {
                    in_chosen = true;
                    wrote_chosen = true;
                    o.prop_u64(start_off, start)?;
                    o.prop_u64(end_off, end)?;
                }
                at = next;
            }
            END_NODE => {
                if depth == 0 {
                    return Err(Error::Malformed);
                }
                if depth == 1 && !wrote_chosen {
                    // The root is closing and there was no /chosen: add one as its last child.
                    o.u32(BEGIN_NODE)?;
                    o.bytes(b"chosen\0\0")?;
                    o.prop_u64(start_off, start)?;
                    o.prop_u64(end_off, end)?;
                    o.u32(END_NODE)?;
                    wrote_chosen = true;
                }
                if depth == 2 {
                    in_chosen = false;
                }
                o.u32(END_NODE)?;
                depth -= 1;
                at += 4;
            }
            PROP => {
                let len = be32(src, at + 4)? as usize;
                let nameoff = be32(src, at + 8)? as usize;
                let next = at + 12 + align4(len);
                if next > struct_end {
                    return Err(Error::Truncated);
                }
                let name = strings
                    .get(nameoff..)
                    .and_then(|s| s.iter().position(|&b| b == 0).map(|n| &s[..n]))
                    .ok_or(Error::Truncated)?;
                // Replace, never duplicate: a stale pair from the firmware is dropped.
                let ours = in_chosen && depth == 2 && (name == INITRD_START || name == INITRD_END);
                if !ours {
                    o.bytes(&src[at..next])?;
                }
                at = next;
            }
            NOP => at += 4,
            END => {
                if depth != 0 {
                    return Err(Error::Malformed);
                }
                o.u32(END)?;
                break;
            }
            _ => return Err(Error::Malformed),
        }
    }
    let size_struct = o.at - new_struct;
    let new_strings = o.at;
    o.bytes(strings)?;
    for name in &appended[..appended_count] {
        o.bytes(name)?;
        o.bytes(&[0])?;
    }
    let size_strings = o.at - new_strings;
    let total = o.at;

    let fields: [u32; 10] = [
        MAGIC,
        total as u32,
        new_struct as u32,
        new_strings as u32,
        new_rsvmap as u32,
        17, // version
        16, // last compatible version
        h.boot_cpuid,
        size_strings as u32,
        size_struct as u32,
    ];
    for (i, field) in fields.iter().enumerate() {
        o.buf[i * 4..i * 4 + 4].copy_from_slice(&field.to_be_bytes());
    }
    Ok(total)
}

#[cfg(test)]
mod tests {
    use device_tree_blob::{DeviceTreeBlob, Region};

    use super::*;

    const PLAIN: &[u8] =
        include_bytes!("../../crates/device_tree_blob/tests/fixtures/qemu-aarch64-virt.dtb");
    const WITH_INITRD: &[u8] =
        include_bytes!("../../crates/device_tree_blob/tests/fixtures/qemu-aarch64-virt-initrd.dtb");
    const RISCV: &[u8] =
        include_bytes!("../../crates/device_tree_blob/tests/fixtures/qemu-riscv64-virt.dtb");

    fn rewrite(src: &[u8], start: u64, end: u64) -> Vec<u8> {
        let mut out = vec![0; output_len(src).unwrap()];
        let n = with_initrd(src, start, end, &mut out).unwrap();
        out.truncate(n);
        out
    }

    /// Everything the kernel reads from the tree, before and after, except the archive.
    fn same_machine(a: &[u8], b: &[u8]) {
        let (a, b) = (
            DeviceTreeBlob::from_bytes(a).unwrap(),
            DeviceTreeBlob::from_bytes(b).unwrap(),
        );
        let mut ra = [Region { start: 0, size: 0 }; 8];
        let mut rb = [Region { start: 0, size: 0 }; 8];
        let na = a.memory_regions(&mut ra).unwrap();
        let nb = b.memory_regions(&mut rb).unwrap();
        assert_eq!(ra[..na], rb[..nb], "the RAM the kernel sees");
        let na = a.reserved_memory_regions(&mut ra).unwrap();
        let nb = b.reserved_memory_regions(&mut rb).unwrap();
        assert_eq!(
            ra[..na],
            rb[..nb],
            "the /reserved-memory the kernel honours"
        );
        assert_eq!(
            a.node_prop(b"chosen", b"stdout-path").unwrap(),
            b.node_prop(b"chosen", b"stdout-path").unwrap()
        );
    }

    #[test]
    fn the_kernel_reads_back_the_archive_it_is_told_about() {
        for src in [PLAIN, WITH_INITRD, RISCV] {
            let out = rewrite(src, 0x4800_0000, 0x4849_0000);
            let tree = DeviceTreeBlob::from_bytes(&out).unwrap();
            assert_eq!(
                tree.initrd().unwrap(),
                Some(Region {
                    start: 0x4800_0000,
                    size: 0x49_0000
                })
            );
            same_machine(src, &out);
            assert!(out.len() <= output_len(src).unwrap());
        }
    }

    #[test]
    fn a_stale_pair_from_the_firmware_is_replaced_not_duplicated() {
        let before = DeviceTreeBlob::from_bytes(WITH_INITRD)
            .unwrap()
            .initrd()
            .unwrap();
        assert!(
            before.is_some(),
            "the fixture carries QEMU's own -initrd pair"
        );
        let out = rewrite(WITH_INITRD, 0x9000_0000, 0x9000_1000);
        let tree = DeviceTreeBlob::from_bytes(&out).unwrap();
        assert_eq!(
            tree.initrd().unwrap(),
            Some(Region {
                start: 0x9000_0000,
                size: 0x1000
            })
        );
        let count = out
            .windows(INITRD_START.len())
            .filter(|w| *w == INITRD_START)
            .count();
        assert_eq!(count, 1, "the name is in the strings block once");
    }

    #[test]
    fn a_tree_without_chosen_gets_one() {
        // Build a minimal tree by hand: root with one property and no children.
        let mut src = vec![0u8; 40];
        let rsv = src.len();
        src.extend_from_slice(&[0; 16]);
        let st = src.len();
        for w in [BEGIN_NODE, 0, PROP, 4, 0, 0x2a, END_NODE, END] {
            src.extend_from_slice(&w.to_be_bytes());
        }
        let size_struct = src.len() - st;
        let strings = src.len();
        src.extend_from_slice(b"answer\0");
        let fields = [
            MAGIC,
            src.len() as u32,
            st as u32,
            strings as u32,
            rsv as u32,
            17,
            16,
            0,
            7,
            size_struct as u32,
        ];
        for (i, f) in fields.iter().enumerate() {
            src[i * 4..i * 4 + 4].copy_from_slice(&f.to_be_bytes());
        }
        let out = rewrite(&src, 0x1000, 0x2000);
        let tree = DeviceTreeBlob::from_bytes(&out).unwrap();
        assert_eq!(
            tree.initrd().unwrap(),
            Some(Region {
                start: 0x1000,
                size: 0x1000
            })
        );
    }

    #[test]
    fn refuses_what_is_not_a_tree() {
        let mut out = [0u8; 256];
        assert_eq!(
            with_initrd(&[0; 64], 0, 1, &mut out),
            Err(Error::NotADeviceTree)
        );
        assert_eq!(
            with_initrd(&PLAIN[..20], 0, 1, &mut out),
            Err(Error::Truncated)
        );
        assert_eq!(
            with_initrd(PLAIN, 0, 1, &mut out),
            Err(Error::OutputTooSmall),
            "256 bytes is not a whole QEMU tree"
        );
    }
}
