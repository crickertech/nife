//! **`package_archive`**: the one file a nife package is, read and written by one definition.
//!
//! DECISIONS §197 (a package is one archive file), in
//! [its own file](../../../design/decisions/197-a-package-is-one-archive-file.md), ruled the container: **one archive file per package**, identified by name and
//! version, in the shape `.deb`, `.apk` and `.hpkg` all use, with
//! DECISIONS §195 (a reviewed recipe vouches for a package), in
//! [its own file](../../../design/decisions/195-a-recipe-vouches-and-the-owner-may-overrule.md),'s reviewed recipe carrying the digest that decides
//! whether the bytes may run. This crate is that container's bytes: a host tool writes one with
//! [`write_package`] and a target reads it with [`Package::parse`], so **one definition of the
//! format serves both**, which is the same arrangement `crates/nifefs` has and for the same reason.
//!
//! Pure logic, host-tested, `no_std`, no allocation. A reader borrows out of the buffer it is given
//! and builds nothing, because the first consumer of this format is a program on a system whose
//! loader has no allocator.
//!
//! **This is rung 3a's producer half and the parser it implies.** It deliberately does *not*
//! install anything: what installing a package *does* to a running system is the activation fork
//! (milestone 507 (installing a package: mutate, compose, or widen what can be spawned),
//! options with no winner), which is an architect's and is not decided. A package that
//! can be built, named, verified and read is what that ruling will act on, and it is buildable
//! without it.
//!
//! # The layout
//!
//! ```text
//!   header, HEADER_LEN = 112 bytes
//!     magic          "NIFEPKG1"   8 bytes, version in the last byte
//!     name           NAME_LEN = 32 bytes, NUL-padded
//!     version        NAME_LEN = 32 bytes, NUL-padded
//!     architecture   NAME_LEN = 32 bytes, NUL-padded
//!     member_count   u32 LE
//!     reserved       u32 LE, zero, so the table starts 8-byte aligned
//!
//!   the table of contents, member_count entries of MEMBER_LEN = 72 bytes:
//!     name           NAME_LEN = 32 bytes, NUL-padded
//!     offset         u32 LE, absolute, from the start of the file
//!     len            u32 LE
//!     digest         DIGEST_LEN = 32 bytes, SHA-256 of that member's bytes
//!
//!   member bytes, each starting at an 8-byte boundary
//! ```
//!
//! **Three choices in that table are worth their reasons.**
//!
//! `offset` is absolute rather than relative to the member area, so a reader never has to know
//! where the table ends to find data; only the writer computes that. `crates/nifefs` made the same
//! call about `start_block` and says so.
//!
//! Each member carries **its own digest**, where `.hpkg` carries none and keeps SHA-256 in the
//! repository index. §195 puts a digest over the whole file in the recipe, which answers "are these
//! the reviewed bytes"; a per-member digest answers "is *this* member the one that was reviewed",
//! which is the question a spawner asks when it is handed one program out of a package. The
//! measurement table the progenitor already enforces (`crates/measured_boot`) is exactly a
//! name-to-digest map, so a package's table of contents is that table travelling with its bytes.
//!
//! Member bytes are **8-byte aligned** because the largest member is an ELF and the cheapest way to
//! be wrong later is to hand a parser an odd address. It costs at most seven bytes per member.
//!
//! # Names, and the ceiling on them
//!
//! A name is at most [`NAME_LEN`] bytes and is **not** NUL-terminated when it uses all of them, so
//! a reader compares against `NAME_LEN` bytes and stops at the first NUL if there is one. That is
//! `nifefs`'s rule, kept deliberately: `NAME_LEN` there is the ceiling on every spawnable name in
//! this system, so a package member that cannot be an archive entry would be a member nothing could
//! install.
//!
//! # EXAMPLES
//!
//! Write a package, read it back, and check a member against its digest:
//!
//! ```
//! # use package_archive::{Attributes, Package, package_size, write_package};
//! let members: [(&str, &[u8]); 2] = [("uptime", b"\x7fELF..."), ("uptime.licence", b"MIT")];
//! let attributes = Attributes { name: "uptime", version: "0.1.0", architecture: "aarch64" };
//!
//! let mut file = vec![0u8; package_size(&members)];
//! write_package(&attributes, &members, &mut file).unwrap();
//!
//! let package = Package::parse(&file).unwrap();
//! assert_eq!(package.name(), "uptime");
//! assert_eq!(package.version(), "0.1.0");
//! assert_eq!(package.read("uptime"), Some(&b"\x7fELF..."[..]));
//! package.verify().unwrap();
//! ```
//!
//! Refuse a file that has been edited since it was written:
//!
//! ```
//! # use package_archive::{Attributes, Package, VerifyError, package_size, write_package};
//! # let members: [(&str, &[u8]); 1] = [("uptime", b"\x7fELF...")];
//! # let attributes = Attributes { name: "uptime", version: "0.1.0", architecture: "aarch64" };
//! # let mut file = vec![0u8; package_size(&members)];
//! # write_package(&attributes, &members, &mut file).unwrap();
//! let last = file.len() - 1;
//! file[last] ^= 0xff;
//! assert_eq!(Package::parse(&file).unwrap().verify(), Err(VerifyError { index: 0 }));
//! ```
//!
//! # BUGS
//!
//! - **Nothing installs a package.** This crate produces and reads one; the activation fork
//!   (milestone 507) decides what installing means, and until it is ruled a package is a file with
//!   no consumer on the target. That is the honest state of rung 3a, not an oversight.
//! - **The encoding is provisional**, and so is the crate's name. §197 ruled the *container*, not
//!   these offsets. Two of its own open questions are deliberately not answered here: where a
//!   program's manifest travels (a sibling member is merely *possible* under this layout, because
//!   a member is any named bytes; nothing in this crate requires or names one) and whether the
//!   digest is a Merkle root (this takes the plain SHA-256 §197 records as the default). Nobody
//!   outside this repository has a nife package, so the encoding is still free to move; §197 says
//!   the day one is fetched by somebody else is the day it is fixed.
//! - **No compression.** `.hpkg` chunks its heap with zlib and `.apk` is three gzip streams; this
//!   stores members whole. The first packages are ELFs that were about to be written to a disk
//!   anyway, and a compressor is a second hostile-input parser on the same path. It is a size cost,
//!   measured nowhere yet.
//! - **`a_short_file_is_refused` carries `#[kani::unwind(9)]`, and proves nothing less for it.**
//!   Unbounded, its falsification took **20 to 31 minutes** at **3.0-3.6 GB** of solver memory
//!   (patagonia, other lanes running, so an order of magnitude). The cause is the harness's shape
//!   and it is `nifefs`'s exactly: once the weakened guard admits the one concrete length, symbolic
//!   execution reaches the member loop, bounded by a `count` read from symbolic bytes, and CBMC has
//!   to unroll it. `nifefs`'s did not finish at all. On the correct tree the guard returns first and
//!   no loop is reachable, so the bound adds no unwinding assertion and cuts no path. Measured
//!   2026-09-24, three runs each, Kani 0.67.0: **before**, SUCCESSFUL, 0 of 343 checks failed (25
//!   unreachable), 0.19 to 0.22 s; **after**, the same 0 of 343 (25 unreachable), 0.18 to 0.23 s.
//!   The falsification now goes red in **1.3 s**, on the harness's own `assert_eq!` and nothing
//!   else. 9 is the least bound that lets the 8-byte magic compare finish, the reason `nifefs`
//!   chose it.
//! - **A package is bounded by `u32`.** A member longer than 4 GiB, or a file longer than 4 GiB,
//!   cannot be represented. The largest member anyone has packed is `rg` at 10.7 MB.
//! - **A duplicate member name is refused by the writer and first-wins in the reader**, which is
//!   `nifefs`'s split exactly. A reader cannot refuse it cheaply without comparing every pair, and
//!   the producer is the place that knows.
//! - **[`Package::parse`] does not check that members do not overlap**, nor that they lie after the
//!   table. Every accessor is bounds-checked against the buffer, so an overlapping file is readable
//!   rather than unsafe, and [`Package::verify`] is what makes an unreviewed arrangement of bytes
//!   fail: a member that overlaps another cannot match both digests unless the bytes really are
//!   shared.
//! - **Nothing here says where a package came from.** §195's per-source trust is the client's
//!   business, and there is no client yet.
//!
//! Name: provisional 2026-09-23 (milestone 198 (a package manager, and the trivial install)'s
//! rung 3a lane). `package_archive` because the
//! thing is an archive of a package's files and both words are the ones the field uses; a bare
//! `package` would be the generic-word failure `design/naming.md` names, and `pkg` the abbreviation
//! one. Not ratified by calef.

#![no_std]

pub use measured_boot::{DIGEST_LEN, Digest, sha256};

/// The magic, with the format version in the last byte, so a reader meeting a later package says
/// [`Error::BadMagic`] rather than striding a table whose entries have moved. `nifefs` records the
/// rule this follows: bump when a reader can tell.
pub const MAGIC: [u8; 8] = *b"NIFEPKG1";

/// The longest name, in bytes, for the package and for each member.
///
/// **32, which is `nifefs::NAME_LEN` and not a coincidence.** A member that cannot be an archive
/// entry is a member nothing in this system could install, so the two ceilings are one ceiling. It
/// is not a dependency on that crate: taking one to share a constant would put a filesystem in the
/// graph of every program that reads a package.
pub const NAME_LEN: usize = 32;

/// The header: magic, three names, the count, and four bytes of zero that keep the table 8-byte
/// aligned.
pub const HEADER_LEN: usize = 8 + NAME_LEN * 3 + 4 + 4;

/// One table-of-contents entry: the name, `offset` and `len` as `u32`s, then the member's digest.
pub const MEMBER_LEN: usize = NAME_LEN + 4 + 4 + DIGEST_LEN;

/// The alignment every member's bytes start on. See the crate docs for why it is not 1.
pub const MEMBER_ALIGN: usize = 8;

/// The most members a package may hold.
///
/// **A ceiling exists so that a header claiming four billion members is refused before anything is
/// multiplied by it**, which is the arithmetic a hostile file reaches for first. 64 is a judgement:
/// the largest thing anyone has proposed packaging is one program, its licence, its manifest and
/// its documentation bundle, which is four. Raising it costs nothing but the bound in this line.
pub const MAX_MEMBERS: usize = 64;

/// What a package says about itself: the identity §197 rules it is named by, plus the architecture,
/// because "per-architecture output is the normal case" (§197) and three triples means three files.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Attributes<'a> {
    /// The package's name, at most [`NAME_LEN`] bytes.
    pub name: &'a str,
    /// Its version, as the recipe spells it. Compared as bytes; nothing here orders versions.
    pub version: &'a str,
    /// The target triple's short name, as this tree spells it: `aarch64`, `riscv64`, `x86_64`.
    pub architecture: &'a str,
}

/// Why a package was refused, by [`Package::parse`] or by [`write_package`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// The first eight bytes are not [`MAGIC`], or the file is shorter than the header.
    BadMagic,
    /// The header claims more members than [`MAX_MEMBERS`].
    TooManyMembers,
    /// The file is shorter than the table of contents it claims, or a member's `offset` and `len`
    /// run past its end. One variant for both, because from a reader's side they are one fact: the
    /// file does not contain what it says it contains.
    Truncated,
    /// A name longer than [`NAME_LEN`] bytes. Refused rather than truncated, for `nifefs`'s reason:
    /// two names agreeing in their first `NAME_LEN` bytes would become one member, and whichever
    /// was packed first would answer for both.
    NameTooLong,
    /// A name containing a NUL, which the padding cannot distinguish from the end of the name.
    NameHasNul,
    /// Two members written under one name. The writer refuses it; see the crate's BUGS.
    DuplicateName,
    /// The buffer handed to [`write_package`] is not exactly [`package_size`] bytes.
    BufferWrongSize,
}

/// A member whose bytes do not hash to the digest the table of contents claims, by its index.
///
/// Its own type rather than an [`Error`] variant, because these are different questions asked at
/// different times: [`Package::parse`] asks whether a file can be read at all, and
/// [`Package::verify`] asks whether it is the file a recipe reviewed. A caller that has already
/// checked the whole file's digest against a recipe may skip the second.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VerifyError {
    /// Which member failed, as an index into the table of contents.
    pub index: usize,
}

/// A parsed package: the bytes, with the table of contents proven to be inside them.
///
/// Holds the buffer and nothing else. Every accessor re-reads the table, which costs a few
/// arithmetic operations and buys a type that is four words wide and can be a local on a small
/// stack. `nifefs::Fs` arrived at the same shape by having to.
///
/// Equality is over the whole buffer, which is the only meaning that would not surprise: two
/// packages sharing a name are not the same package. The tests that assert a refusal need it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Package<'a> {
    bytes: &'a [u8],
    count: usize,
}

impl<'a> Package<'a> {
    /// Read a package's structure, or say why it cannot be read.
    ///
    /// **This is the hostile-input entry point.** It proves, before any accessor runs, that the
    /// magic matches, that the count is within [`MAX_MEMBERS`], that the table of contents lies
    /// inside the buffer, and that every member's `offset .. offset + len` does too, with the
    /// addition done in `usize` on values widened from `u32` so it cannot wrap on any target this
    /// tree builds for. What it does not check is in the crate's BUGS.
    pub fn parse(bytes: &'a [u8]) -> Result<Self, Error> {
        if bytes.len() < HEADER_LEN || bytes[..8] != MAGIC {
            return Err(Error::BadMagic);
        }
        let count = u32::from_le_bytes([
            bytes[HEADER_LEN - 8],
            bytes[HEADER_LEN - 7],
            bytes[HEADER_LEN - 6],
            bytes[HEADER_LEN - 5],
        ]) as usize;
        if count > MAX_MEMBERS {
            return Err(Error::TooManyMembers);
        }
        let table_end = HEADER_LEN + count * MEMBER_LEN;
        if bytes.len() < table_end {
            return Err(Error::Truncated);
        }

        let package = Package { bytes, count };
        for index in 0..count {
            let (offset, len) = package.extent(index);
            // Widened to `usize` before adding: on a 64-bit target two `u32`s cannot overflow it,
            // and on a 32-bit one `checked_add` is what stops a file claiming a member that wraps
            // to a small, in-bounds slice.
            let end = offset.checked_add(len).ok_or(Error::Truncated)?;
            if end > bytes.len() {
                return Err(Error::Truncated);
            }
        }
        Ok(package)
    }

    /// The package's name, as the writer spelled it.
    pub fn name(&self) -> &'a str {
        field(self.bytes, 8)
    }

    /// Its version.
    pub fn version(&self) -> &'a str {
        field(self.bytes, 8 + NAME_LEN)
    }

    /// The architecture its members were built for.
    pub fn architecture(&self) -> &'a str {
        field(self.bytes, 8 + NAME_LEN * 2)
    }

    /// How many members it holds.
    pub fn len(&self) -> usize {
        self.count
    }

    /// Whether it holds none, which a package with only attributes legitimately may.
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// The name of the member at `index`, or `None` past the end.
    pub fn member_name(&self, index: usize) -> Option<&'a str> {
        (index < self.count).then(|| field(self.bytes, HEADER_LEN + index * MEMBER_LEN))
    }

    /// The bytes of the member at `index`, or `None` past the end.
    pub fn member(&self, index: usize) -> Option<&'a [u8]> {
        if index >= self.count {
            return None;
        }
        let (offset, len) = self.extent(index);
        // `parse` proved this range; the slice is what it proved it for.
        Some(&self.bytes[offset..offset + len])
    }

    /// The digest the table of contents claims for the member at `index`.
    ///
    /// Claims, not proves: [`Package::verify`] is what turns a claim into a check.
    pub fn member_digest(&self, index: usize) -> Option<Digest> {
        if index >= self.count {
            return None;
        }
        let at = HEADER_LEN + index * MEMBER_LEN + NAME_LEN + 8;
        let mut digest = [0u8; DIGEST_LEN];
        digest.copy_from_slice(&self.bytes[at..at + DIGEST_LEN]);
        Some(digest)
    }

    /// The bytes of the first member called `name`, if there is one. See BUGS on duplicates.
    pub fn read(&self, name: &str) -> Option<&'a [u8]> {
        self.index_of(name).and_then(|index| self.member(index))
    }

    /// Where `name` sits in the table of contents, if it is there at all.
    pub fn index_of(&self, name: &str) -> Option<usize> {
        (0..self.count).find(|&index| self.member_name(index) == Some(name))
    }

    /// Check every member's bytes against the digest beside it.
    ///
    /// The whole point of the per-member digest: a caller that trusts the file (a recipe vouched
    /// for its digest, §195) still learns *which* member it is holding, and a caller that does not
    /// learns whether anything in the file has moved.
    pub fn verify(&self) -> Result<(), VerifyError> {
        for index in 0..self.count {
            self.verify_member(index)?;
        }
        Ok(())
    }

    /// Check one member, by index. Returns `Ok(())` for an index past the end, because there is no
    /// member there to be wrong; callers that care use [`Package::len`].
    pub fn verify_member(&self, index: usize) -> Result<(), VerifyError> {
        let (Some(bytes), Some(claimed)) = (self.member(index), self.member_digest(index)) else {
            return Ok(());
        };
        if sha256(bytes) == claimed {
            Ok(())
        } else {
            Err(VerifyError { index })
        }
    }

    /// The `(offset, len)` of a member, read from a table `parse` has already bounds-checked.
    fn extent(&self, index: usize) -> (usize, usize) {
        let at = HEADER_LEN + index * MEMBER_LEN + NAME_LEN;
        let word = |o: usize| {
            u32::from_le_bytes([
                self.bytes[at + o],
                self.bytes[at + o + 1],
                self.bytes[at + o + 2],
                self.bytes[at + o + 3],
            ]) as usize
        };
        (word(0), word(4))
    }
}

/// Read a NUL-padded name of [`NAME_LEN`] bytes at `at`.
///
/// Invalid UTF-8 comes back as the empty string rather than as an error, which is deliberate and is
/// the one place this file is lenient: a name is compared, and a name that cannot be spelled matches
/// nothing, so a caller looking one up fails to find it. `parse`'s job is bounds, not spelling.
fn field(bytes: &[u8], at: usize) -> &str {
    let raw = &bytes[at..at + NAME_LEN];
    let end = raw.iter().position(|&b| b == 0).unwrap_or(NAME_LEN);
    core::str::from_utf8(&raw[..end]).unwrap_or("")
}

/// How many bytes [`write_package`] needs for these members, including the padding that aligns each
/// one. Deterministic: the same inputs give the same size and the same bytes, which is what lets a
/// recipe record a digest and a rebuild reproduce it.
pub fn package_size(members: &[(&str, &[u8])]) -> usize {
    let mut at = HEADER_LEN + members.len() * MEMBER_LEN;
    for (_, bytes) in members {
        at = at.next_multiple_of(MEMBER_ALIGN) + bytes.len();
    }
    at
}

/// Write a package into `buffer`, which must be exactly [`package_size`] bytes.
///
/// Refuses rather than truncates or mangles: a name too long, a name with a NUL, a duplicate name,
/// too many members, or a buffer of the wrong size. An archive is a mapping, and a writer that
/// silently changes a key has lost data.
///
/// **No timestamp, no ordering pass, no padding byte that is not zero.** The bytes are a function of
/// the inputs alone, so two hosts building the same recipe produce the same digest, which is the
/// property §195's reviewed recipe rests on.
pub fn write_package(
    attributes: &Attributes<'_>,
    members: &[(&str, &[u8])],
    buffer: &mut [u8],
) -> Result<(), Error> {
    if members.len() > MAX_MEMBERS {
        return Err(Error::TooManyMembers);
    }
    if buffer.len() != package_size(members) {
        return Err(Error::BufferWrongSize);
    }
    check_name(attributes.name)?;
    check_name(attributes.version)?;
    check_name(attributes.architecture)?;
    for (index, (name, _)) in members.iter().enumerate() {
        check_name(name)?;
        if members[..index].iter().any(|(earlier, _)| earlier == name) {
            return Err(Error::DuplicateName);
        }
    }

    buffer.fill(0);
    buffer[..8].copy_from_slice(&MAGIC);
    put_name(buffer, 8, attributes.name);
    put_name(buffer, 8 + NAME_LEN, attributes.version);
    put_name(buffer, 8 + NAME_LEN * 2, attributes.architecture);
    let count = members.len() as u32;
    buffer[HEADER_LEN - 8..HEADER_LEN - 4].copy_from_slice(&count.to_le_bytes());

    let mut at = HEADER_LEN + members.len() * MEMBER_LEN;
    for (index, (name, bytes)) in members.iter().enumerate() {
        at = at.next_multiple_of(MEMBER_ALIGN);
        let entry = HEADER_LEN + index * MEMBER_LEN;
        put_name(buffer, entry, name);
        buffer[entry + NAME_LEN..entry + NAME_LEN + 4].copy_from_slice(&(at as u32).to_le_bytes());
        buffer[entry + NAME_LEN + 4..entry + NAME_LEN + 8]
            .copy_from_slice(&(bytes.len() as u32).to_le_bytes());
        buffer[entry + NAME_LEN + 8..entry + MEMBER_LEN].copy_from_slice(&sha256(bytes));
        buffer[at..at + bytes.len()].copy_from_slice(bytes);
        at += bytes.len();
    }
    Ok(())
}

fn check_name(name: &str) -> Result<(), Error> {
    if name.len() > NAME_LEN {
        Err(Error::NameTooLong)
    } else if name.as_bytes().contains(&0) {
        Err(Error::NameHasNul)
    } else {
        Ok(())
    }
}

fn put_name(buffer: &mut [u8], at: usize, name: &str) {
    buffer[at..at + name.len()].copy_from_slice(name.as_bytes());
}

#[cfg(test)]
mod tests {
    extern crate std;
    use std::vec;

    use super::*;

    fn written(members: &[(&str, &[u8])]) -> std::vec::Vec<u8> {
        let attributes = Attributes {
            name: "uptime",
            version: "0.1.0",
            architecture: "aarch64",
        };
        let mut buffer = vec![0u8; package_size(members)];
        write_package(&attributes, members, &mut buffer).unwrap();
        buffer
    }

    #[test]
    fn a_package_reads_back_what_was_written() {
        let members: [(&str, &[u8]); 3] = [
            ("uptime", b"\x7fELF and then some"),
            ("uptime.licence", b"MIT OR Apache-2.0"),
            ("uptime.manual", b""),
        ];
        let file = written(&members);
        let package = Package::parse(&file).unwrap();

        assert_eq!(package.name(), "uptime");
        assert_eq!(package.version(), "0.1.0");
        assert_eq!(package.architecture(), "aarch64");
        assert_eq!(package.len(), 3);
        for (name, bytes) in members {
            assert_eq!(package.read(name), Some(bytes));
        }
        assert_eq!(package.read("absent"), None);
        package.verify().unwrap();
    }

    #[test]
    fn every_member_starts_aligned() {
        let members: [(&str, &[u8]); 3] = [("a", b"1"), ("b", b"22"), ("c", b"333")];
        let file = written(&members);
        let package = Package::parse(&file).unwrap();
        for index in 0..package.len() {
            let (offset, _) = package.extent(index);
            assert_eq!(offset % MEMBER_ALIGN, 0, "member {index} at {offset}");
        }
    }

    #[test]
    fn the_same_inputs_give_the_same_bytes() {
        // The property a recipe's recorded digest rests on. If this ever fails, §195's whole
        // arrangement fails with it: a reviewer would be vouching for bytes nobody can reproduce.
        let members: [(&str, &[u8]); 2] = [("uptime", b"\x7fELF"), ("uptime.licence", b"MIT")];
        assert_eq!(written(&members), written(&members));
    }

    #[test]
    fn a_name_that_fills_the_field_has_no_terminator() {
        let full = "n".repeat(NAME_LEN);
        let members: [(&str, &[u8]); 1] = [(full.as_str(), b"x")];
        let file = written(&members);
        let package = Package::parse(&file).unwrap();
        assert_eq!(package.member_name(0), Some(full.as_str()));
        assert_eq!(package.read(&full), Some(&b"x"[..]));
    }

    #[test]
    fn the_writer_refuses_what_it_cannot_represent() {
        let long = "n".repeat(NAME_LEN + 1);
        let attributes = Attributes {
            name: "uptime",
            version: "0.1.0",
            architecture: "aarch64",
        };
        let cases: [(&str, Error); 3] = [
            (long.as_str(), Error::NameTooLong),
            ("has\0nul", Error::NameHasNul),
            ("uptime", Error::DuplicateName),
        ];
        for (name, expected) in cases {
            let members: [(&str, &[u8]); 2] = [("uptime", b"x"), (name, b"y")];
            let mut buffer = vec![0u8; package_size(&members)];
            assert_eq!(
                write_package(&attributes, &members, &mut buffer),
                Err(expected)
            );
        }
    }

    #[test]
    fn a_buffer_of_the_wrong_size_is_refused() {
        let members: [(&str, &[u8]); 1] = [("uptime", b"x")];
        let attributes = Attributes {
            name: "uptime",
            version: "0.1.0",
            architecture: "aarch64",
        };
        let mut buffer = vec![0u8; package_size(&members) + 1];
        assert_eq!(
            write_package(&attributes, &members, &mut buffer),
            Err(Error::BufferWrongSize)
        );
    }

    #[test]
    fn a_truncated_file_is_refused_not_indexed() {
        let members: [(&str, &[u8]); 2] = [("uptime", b"\x7fELF"), ("uptime.licence", b"MIT")];
        let file = written(&members);
        for cut in [0, HEADER_LEN - 1, HEADER_LEN, file.len() - 1] {
            let expected = if cut < HEADER_LEN {
                Error::BadMagic
            } else {
                Error::Truncated
            };
            assert_eq!(Package::parse(&file[..cut]), Err(expected), "cut at {cut}");
        }
    }

    #[test]
    fn a_count_beyond_the_ceiling_is_refused_before_it_is_multiplied() {
        let mut file = written(&[("uptime", b"x")]);
        file[HEADER_LEN - 8..HEADER_LEN - 4].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(Package::parse(&file), Err(Error::TooManyMembers));
    }

    #[test]
    fn a_member_pointing_outside_the_file_is_refused() {
        let mut file = written(&[("uptime", b"\x7fELF")]);
        let at = HEADER_LEN + NAME_LEN;
        file[at..at + 4].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(Package::parse(&file), Err(Error::Truncated));
    }

    #[test]
    fn an_edited_member_fails_verification_and_names_itself() {
        let members: [(&str, &[u8]); 2] = [("uptime", b"\x7fELF"), ("uptime.licence", b"MIT")];
        let mut file = written(&members);
        let last = file.len() - 1;
        file[last] ^= 0xff;
        let package = Package::parse(&file).unwrap();
        assert_eq!(package.verify(), Err(VerifyError { index: 1 }));
        package.verify_member(0).unwrap();
    }

    #[test]
    fn wrong_magic_is_refused() {
        let mut file = written(&[("uptime", b"x")]);
        file[7] = b'2';
        assert_eq!(Package::parse(&file), Err(Error::BadMagic));
    }
}

#[cfg(kani)]
mod verification {
    use super::*;

    /// **A file the solver chose is either refused or entirely inside itself.**
    ///
    /// The property that matters for a parser on the install path: whatever bytes arrive, every
    /// accessor `parse` blesses stays in the buffer. Kani checks the slicing in `member` for out of
    /// bounds directly, so the harness's assertion is the arithmetic claim and the panic-freedom is
    /// the proof's own.
    ///
    /// Falsification: replayable `crates/package_archive/falsifications/verification.a_parsed_package_reads_only_inside_itself.patch`
    // **9, and the reason is `memcmp` rather than any loop in this file.** The magic comparison is
    // eight bytes, which the model checker unwinds as a loop; at 4 it reports an unwinding
    // assertion in `<builtin-library-memcmp>` and leaves 270 of 271 checks undetermined, which
    // looks like a failed proof and is a bound that is too small. The member loop is bounded by the
    // buffer: 272 bytes holds a table of at most two entries.
    #[kani::proof]
    #[kani::unwind(9)]
    fn a_parsed_package_reads_only_inside_itself() {
        const LEN: usize = HEADER_LEN + MEMBER_LEN * 2 + 16;
        let bytes: [u8; LEN] = kani::any();
        if let Ok(package) = Package::parse(&bytes) {
            assert!(package.len() <= MAX_MEMBERS);
            for index in 0..2 {
                if let Some(member) = package.member(index) {
                    assert!(member.len() <= LEN);
                }
            }
        }
    }

    /// **A file shorter than the header is refused rather than indexed.** `nifefs` proves the same
    /// boundary and for the same reason: the header read is the one place a short buffer would be
    /// indexed before anything had checked its length.
    ///
    /// Falsification: replayable `crates/package_archive/falsifications/verification.a_short_file_is_refused.patch`
    #[kani::proof]
    #[kani::unwind(9)]
    fn a_short_file_is_refused() {
        let bytes: [u8; HEADER_LEN - 1] = kani::any();
        assert_eq!(Package::parse(&bytes), Err(Error::BadMagic));
    }
}
