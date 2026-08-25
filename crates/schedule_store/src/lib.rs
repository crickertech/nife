//! **The durable schedule store's shared names, and the one document `timetable` does not already
//! own** (milestone 152, DECISIONS §122 and §125).
//!
//! §122 decided the schedule itself: one file per identity, inside that identity's own subtree
//! (DECISIONS §117), in `timetable::parse`'s own document format, unchanged. That answers what one
//! identity's schedule looks like. It does not answer what tells boot-time re-derivation *which*
//! identities have one at all, and neither §122 nor §123 fully specified it: §123 assumes "the store
//! names" a set of sessions to re-derive, without saying how that set is discovered without falling
//! into milestone 126's refusal (enumeration is itself authority). This crate is that answer, worked
//! out as §125 and recorded there: a **manifest**, a second small document at a fixed, well-known
//! location, listing exactly which identities currently have a durable session with pending
//! scheduled work, one name per line.
//!
//! # Why a manifest rather than a directory listing
//!
//! The obvious-looking alternative is to `READDIR` the principal tree's root and treat every
//! subtree found there as "an identity with pending work". That is refused on this tree's own
//! terms: DECISIONS §123 says boot-time re-derivation must not be granted "anything that would let
//! it enumerate users rather than iterate a hard-wired set it was constructed to read", citing
//! milestone 126's enumeration-is-authority rule, and every identity provisioned
//! (`identity_provisioner`, milestone 155) gets a subtree whether or not it ever registers a
//! schedule, so a directory listing would answer a different question than the one boot-time
//! re-derivation needs to ask.
//!
//! A manifest answers the right question instead, and reading it is not enumeration in the sense
//! §123 refuses: `OPEN`ing one file at a name the reader already knows at compile time
//! ([`MANIFEST_FILE_NAME`]) is a targeted lookup, the same shape 152's own reattachment design uses
//! for the credentialer ("given a proven identity, return the one record for it, never a list").
//! What the manifest's *contents* name is data this crate interprets, not a directory the reader
//! walked to find out what exists.
//!
//! # What this crate is not
//!
//! It performs no IO and makes no syscalls (CLAUDE.md rule 7: two programs that must agree on a
//! format share a crate, not a wire convention re-derived twice). The write side
//! (`user/src/fs_test_client.rs`'s `ROLE_SCHEDULE_SEED`, this lane's own demonstration writer) and
//! the read side (`user/src/session_reviver.rs`, the boot-time re-deriver) both depend on it for
//! exactly the same reason `timetable` is shared by the process that writes the shipped
//! `timetable.conf` file and the process that reads it: the parser and the render logic must be one
//! function, not two that could drift.
//!
//! It does not touch `timetable::Schedule` or `timetable::parse` at all, and depends on nothing:
//! the schedule document itself is out of scope here (§122's own subject), and the manifest is
//! small enough that pulling in a whole other crate to write eight lines of text would be the
//! "more machinery" AGENTS.md's elegance tenet warns against.
//!
//! Name: provisional. `schedule_store` is the plain noun both §122 and §123 already use to describe
//! the thing this crate is a piece of ("the on-disk, per-user schedule store"), extended here to
//! cover the manifest as well as the location constant a reader of a schedule file needs
//! ([`SCHEDULE_FILE_NAME`]). calef's call to ratify per AGENTS.md.

#![no_std]

/// **The name of one identity's own schedule file, inside that identity's own subtree**
/// (DECISIONS §117, §122). `<principal tree root>/<identity>/schedule`. The exact filename is a
/// detail §122 left to whoever built the write path; this lane picked it and says so here rather
/// than leaving it as a magic string duplicated at both call sites.
pub const SCHEDULE_FILE_NAME: &str = "schedule";

/// **The manifest's own name, directly under the principal tree's root**, a sibling of every
/// identity's own subtree rather than something nested inside one (DECISIONS §125): the manifest is
/// not any one identity's own record, so it does not belong inside any one identity's own subtree.
pub const MANIFEST_FILE_NAME: &str = "durable-sessions";

/// The most identities one manifest may list.
///
/// A ceiling rather than a limit anybody meets, matching `timetable::MAX_ENTRIES`'s own reasoning:
/// this crate allocates nothing, so a parsed manifest borrows the document's own bytes and the
/// table backing it is a fixed array. Eight matches `timetable::MAX_ENTRIES` because there is no
/// reason for the two ceilings to differ in this demonstration; a real deployment sizes both against
/// how many principals it actually has.
pub const MAX_IDENTITIES: usize = 8;

/// The longest one identity name this crate will carry.
///
/// Matches `login_proto::MAX_IDENTITY` (64 bytes) by convention rather than by a shared type: the
/// manifest names the same identities a login authenticates, so a name too long to ever log in with
/// is not a name this store needs to carry either. Not enforced by a shared dependency (this crate
/// takes none), only by this constant and the comment naming why it was chosen.
pub const MAX_IDENTITY_LEN: usize = 64;

/// A parsed manifest: the identity names it lists, in document order, each borrowing the document's
/// own bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Manifest<'a> {
    names: [&'a [u8]; MAX_IDENTITIES],
    n: usize,
}

impl<'a> Manifest<'a> {
    /// The identity names this manifest lists, in the order the document wrote them.
    pub fn entries(&self) -> &[&'a [u8]] {
        &self.names[..self.n]
    }
}

/// Why a manifest document does not parse. Every variant carries the **1-based line number** it
/// went wrong on, matching `timetable::Error`'s own convention and for the same reason: a
/// configuration error nobody can find in the file is one nobody will fix.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Error {
    /// A name longer than [`MAX_IDENTITY_LEN`].
    NameTooLong(usize),
    /// More identities than [`MAX_IDENTITIES`], reported at the line that overflowed rather than
    /// dropped, matching `timetable::Error::TooManyEntries`'s own posture.
    TooManyIdentities(usize),
}

impl Error {
    /// The 1-based line this error is about.
    pub fn line(self) -> usize {
        match self {
            Error::NameTooLong(l) | Error::TooManyIdentities(l) => l,
        }
    }

    /// The fixed half of the sentence a reader gets. Host-tested, so the wording cannot drift.
    pub fn message(self) -> &'static str {
        match self {
            Error::NameTooLong(_) => "an identity name longer than this store carries",
            Error::TooManyIdentities(_) => "more identities than this manifest holds",
        }
    }
}

/// Parse a manifest document: one identity name per line, blank lines and `#` comments ignored,
/// matching `timetable::parse`'s own dialect for the same reason (a reader who already knows one of
/// this tree's document formats should not have to learn a second one for a document this small).
///
/// Fails on the **first** problem, with its line, `timetable::parse`'s own posture.
///
/// # Examples
///
/// ```
/// let doc = schedule_store::parse_manifest("# who has pending work\nchris\ncorinne\n")
///     .expect("that document is well formed");
/// assert_eq!(doc.entries(), &[b"chris".as_slice(), b"corinne".as_slice()]);
///
/// // Comments and blank lines are ignored, and an empty manifest is not an error: nobody has
/// // registered a schedule yet, which is a fact about the fleet, not a malformed document.
/// let empty = schedule_store::parse_manifest("# nobody yet\n\n").unwrap();
/// assert_eq!(empty.entries().len(), 0);
/// ```
pub fn parse_manifest(doc: &str) -> Result<Manifest<'_>, Error> {
    let mut names = [b"".as_slice(); MAX_IDENTITIES];
    let mut n = 0usize;

    for (i, raw) in doc.lines().enumerate() {
        let no = i + 1;
        let line = strip_comment(raw).trim();
        if line.is_empty() {
            continue;
        }
        let bytes = line.as_bytes();
        if bytes.len() > MAX_IDENTITY_LEN {
            return Err(Error::NameTooLong(no));
        }
        if n == MAX_IDENTITIES {
            return Err(Error::TooManyIdentities(no));
        }
        names[n] = bytes;
        n += 1;
    }
    Ok(Manifest { names, n })
}

/// Everything up to a `#`. No escape, matching `timetable::parse`'s own reasoning: an identity name
/// has no use for a literal `#`, and an escape nobody needs is a rule everybody has to know.
fn strip_comment(line: &str) -> &str {
    match line.split_once('#') {
        Some((before, _)) => before,
        None => line,
    }
}

/// **Render a manifest**, one identity name per line: the write-path half of this crate, used by
/// whoever records that an identity now has a durable session with pending work (this lane's own
/// demonstration writer, `user/src/fs_test_client.rs`'s `ROLE_SCHEDULE_SEED`; a real registrar,
/// #387, would call this every time a schedule changes).
///
/// A fixed buffer rather than a `String`, because this crate is `no_std` with no `alloc`
/// (`timetable::write_plan`'s own reasoning, matching it exactly). `None` if `names` will not fit
/// [`MAX_IDENTITIES`], any one name is longer than [`MAX_IDENTITY_LEN`], or the rendered document is
/// wider than `buf`; otherwise `Some(bytes written)`.
///
/// # Examples
///
/// ```
/// let mut buf = [0u8; 64];
/// let n = schedule_store::render_manifest(&[b"chris", b"corinne"], &mut buf)
///     .expect("two short names fit");
/// assert_eq!(&buf[..n], b"chris\ncorinne\n");
///
/// // Parsing what this just rendered gives back exactly what went in: the round trip the write
/// // path and the read path both depend on.
/// let doc = core::str::from_utf8(&buf[..n]).unwrap();
/// let parsed = schedule_store::parse_manifest(doc).unwrap();
/// assert_eq!(parsed.entries(), &[b"chris".as_slice(), b"corinne".as_slice()]);
/// ```
pub fn render_manifest(names: &[&[u8]], buf: &mut [u8]) -> Option<usize> {
    if names.len() > MAX_IDENTITIES {
        return None;
    }
    let mut n = 0usize;
    for name in names {
        if name.is_empty() || name.len() > MAX_IDENTITY_LEN {
            return None;
        }
        if n + name.len() + 1 > buf.len() {
            return None;
        }
        buf[n..n + name.len()].copy_from_slice(name);
        n += name.len();
        buf[n] = b'\n';
        n += 1;
    }
    Some(n)
}

/// Fixture data both this lane's own demonstration writer
/// (`user/src/fs_test_client.rs`'s `ROLE_SCHEDULE_SEED`) and the kernel test wiring them together
/// use, so the identity and the schedule document a reader meets in either place are the one the
/// other was written against, matching `filesystem_proto::fixture`'s own convention for
/// `SMB_SEED`/`SMB_SEED_NAME`.
pub mod fixture {
    /// The one identity this lane's demonstration seeds a durable schedule for. Deliberately not
    /// `chris` or `corinne` (used by other suites' own fixtures, `credentialer_test_client.rs`'s
    /// `PEOPLE` and `identity_provisioning_tests.rs`), so this suite's own subtree and manifest
    /// entry cannot collide with anything an earlier test in the same continuous boot already
    /// wrote.
    pub const DEMO_IDENTITY: &str = "durable_demo";

    /// One `at-boot` entry and one `every` entry, matching `timetable::parse`'s own document shape
    /// (`user/timetable.conf`'s own reference document is the model): enough to prove the format
    /// round-trips through a real read from the filesystem, not merely through `include_str!`.
    pub const DEMO_SCHEDULE_DOC: &str = "at-boot worker 3\nevery 30s worker 7\n";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_well_formed_manifest_lists_its_identities_in_order() {
        let doc = parse_manifest("# heading\nchris\n\ncorinne\n# trailing comment\n").unwrap();
        assert_eq!(doc.entries(), &[b"chris".as_slice(), b"corinne".as_slice()]);
    }

    #[test]
    fn an_empty_manifest_is_not_an_error() {
        let doc = parse_manifest("# nobody yet\n\n").unwrap();
        assert_eq!(doc.entries().len(), 0);
    }

    #[test]
    fn a_name_past_the_bound_is_refused_at_its_own_line() {
        let mut doc = repeat_byte_string(b'a', MAX_IDENTITY_LEN + 1);
        doc.push('\n');
        assert_eq!(parse_manifest(&doc), Err(Error::NameTooLong(1)));
    }

    #[test]
    fn more_identities_than_the_table_holds_is_refused_rather_than_dropped() {
        let doc = heapless_repeat("chris\n", MAX_IDENTITIES + 1);
        assert_eq!(
            parse_manifest(&doc),
            Err(Error::TooManyIdentities(MAX_IDENTITIES + 1))
        );
    }

    #[test]
    fn render_then_parse_round_trips() {
        let mut buf = [0u8; 256];
        let n = render_manifest(&[b"chris", b"corinne", b"durable_demo"], &mut buf).unwrap();
        let text = core::str::from_utf8(&buf[..n]).unwrap();
        let doc = parse_manifest(text).unwrap();
        assert_eq!(
            doc.entries(),
            &[
                b"chris".as_slice(),
                b"corinne".as_slice(),
                b"durable_demo".as_slice()
            ]
        );
    }

    #[test]
    fn rendering_refuses_what_it_cannot_carry() {
        let mut buf = [0u8; 256];
        // Too many names.
        let many: [&[u8]; MAX_IDENTITIES + 1] = [b"x".as_slice(); MAX_IDENTITIES + 1];
        assert_eq!(render_manifest(&many, &mut buf), None);

        // A name past the bound.
        let long = repeat_byte_string(b'a', MAX_IDENTITY_LEN + 1);
        assert_eq!(render_manifest(&[long.as_bytes()], &mut buf), None);

        // A buffer too small for what would otherwise fit.
        let mut tiny = [0u8; 3];
        assert_eq!(render_manifest(&[b"chris"], &mut tiny), None);

        // An empty name.
        assert_eq!(render_manifest(&[b""], &mut buf), None);
    }

    #[test]
    fn every_error_reads_differently() {
        assert_ne!(
            Error::NameTooLong(1).message(),
            Error::TooManyIdentities(1).message(),
        );
    }

    // This crate is `no_std` with no `alloc`; host tests link `std` (the usual `cfg(test)` shape
    // every crate in this tree uses), so a `String` here is fine even though the library code never
    // allocates one.
    extern crate std;
    use std::string::String;

    fn repeat_byte_string(byte: u8, len: usize) -> String {
        let mut s = String::with_capacity(len);
        for _ in 0..len {
            s.push(byte as char);
        }
        s
    }

    fn heapless_repeat(line: &str, times: usize) -> String {
        let mut s = String::new();
        for _ in 0..times {
            s.push_str(line);
        }
        s
    }
}
