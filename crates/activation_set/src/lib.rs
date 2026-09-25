//! **`activation_set`**: what is installed, as a table that is never edited, only succeeded.
//!
//! DECISIONS §208 (installing a package is granting it, and the activation set is versioned) ruled
//! that installing *records that a package exists*, and that **the table of entries is versioned**
//! so a set can be selected and rolled back as a whole. This crate is that table's logic, pure and
//! host-tested, for whichever process DECISIONS §215 (how the shell names an installed program to
//! the spawner) ends up giving it to. Nothing on a target reads it yet.
//!
//! # The shape, and why it is this one
//!
//! **A generation is a text file of entries and is never rewritten.** Installing, upgrading and
//! removing each produce the *next* generation from the current one ([`with_entry`],
//! [`without_entry`]); a one-line `current` file names which generation is live
//! ([`parse_current`], [`format_current`]). So a rollback is rewriting one line to name an older
//! generation, and every older set is still on disk, whole, to be named. That is the Nix profile's
//! arrangement (a generation is immutable, the profile is a pointer), recorded as recalled rather
//! than re-read.
//!
//! **An entry is `measured_boot`'s manifest line with one column added**: `<program> <package>
//! <digest>`, where `<package>` is the package's `name-version-architecture` stem. The progenitor
//! already parses the two-column form to decide what may run, so this adds a column to a parser it
//! trusts rather than a second format on its path. §215 records this as a recommendation, not a
//! ruling.
//!
//! **Strict, because this table decides what may be spawned.** A malformed line makes the whole
//! table unreadable ([`Error::Malformed`]) rather than skipped, for `measured_boot`'s reason: a
//! table that half-parses vouches for whatever survived.
//!
//! # EXAMPLES
//!
//! ```
//! use activation_set::{Entry, lookup, with_entry, without_entry};
//!
//! let digest = [7u8; 32];
//! let uptime = Entry { program: "uptime", package: "uptime-0.1.0-aarch64", digest };
//! let mut first = [0u8; 256];
//! let n = with_entry("", &uptime, &mut first).unwrap();
//! let generation_1 = core::str::from_utf8(&first[..n]).unwrap();
//! assert_eq!(lookup(generation_1, "uptime").unwrap().unwrap().package, "uptime-0.1.0-aarch64");
//!
//! let mut second = [0u8; 256];
//! let n = without_entry(generation_1, "uptime", &mut second).unwrap();
//! let generation_2 = core::str::from_utf8(&second[..n]).unwrap();
//! assert!(lookup(generation_2, "uptime").unwrap().is_none());
//! // Generation 1 is untouched, so rolling back is naming it again.
//! assert!(lookup(generation_1, "uptime").unwrap().is_some());
//! ```
//!
//! # BUGS
//!
//! - **Nothing collects old generations.** Every install leaves one file behind, which is what
//!   makes rollback free and is also unbounded. A retention rule (keep N, keep the last boot's) is
//!   owed before a system installs often.
//! - **A table is read whole into memory**, and its size is whatever the caller's buffer is. Forty
//!   entries fit a page.
//! - **One program per package entry.** A package with two programs is two entries naming the same
//!   package, which works and is not tested beyond that.
//! - **The `current` file is the one mutable thing**, so its write is the commit point, and nothing
//!   here makes that write atomic. On RedoxFS it is a one-line overwrite; whether a torn write of it
//!   is possible was not checked.
//!
//! Name: provisional 2026-09-24 (milestone 198 (a package manager)'s rung 3a consumer lane). §208's own phrase is
//! "activation set", which is why; `design/naming.md` is the rule and calef's the call.

#![no_std]

pub use measured_boot::Digest;

/// One installed program: its name at the prompt, the package it came from, and that package's
/// digest as it was verified at install.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Entry<'a> {
    /// The name a person types, and the name the spawner is asked for.
    pub program: &'a str,
    /// The package it came from, as its `name-version-architecture` stem.
    pub package: &'a str,
    /// The package file's SHA-256, as verified when it was installed.
    pub digest: Digest,
}

/// Why a table or an edit was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// A line is not `<program> <package> <64 hex>`, so the table vouches for nothing.
    Malformed,
    /// A name is empty, or holds a space, a newline or a `#`, so it would not read back as itself.
    BadName,
    /// [`without_entry`] was asked to remove a program the table does not have.
    NotInstalled,
    /// The output buffer is too small for the next generation.
    TooSmall,
}

/// Every entry in a generation, or [`Error::Malformed`] at the first line that is not one. Blank
/// lines and `#` comments are skipped, as in `measured_boot`'s manifest.
pub fn entries(table: &str) -> impl Iterator<Item = Result<Entry<'_>, Error>> {
    table
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| {
            let mut words = line.split(' ');
            let (Some(program), Some(package), Some(hex), None) =
                (words.next(), words.next(), words.next(), words.next())
            else {
                return Err(Error::Malformed);
            };
            let digest = measured_boot::parse_hex(hex).ok_or(Error::Malformed)?;
            if !good_name(program) || !good_name(package) {
                return Err(Error::Malformed);
            }
            Ok(Entry {
                program,
                package,
                digest,
            })
        })
}

/// The entry for `program`, if the generation has one. The whole table is checked first, so a
/// malformed line anywhere is a refusal even when the name asked for is on a good line.
pub fn lookup<'a>(table: &'a str, program: &str) -> Result<Option<Entry<'a>>, Error> {
    let mut found = None;
    for entry in entries(table) {
        let entry = entry?;
        if entry.program == program {
            found = Some(entry);
        }
    }
    Ok(found)
}

/// The next generation: `table` with `entry` installed. A program already present is **replaced in
/// place**, which is an upgrade; one that is not is appended. Returns the length written to `out`.
pub fn with_entry(table: &str, entry: &Entry<'_>, out: &mut [u8]) -> Result<usize, Error> {
    if !good_name(entry.program) || !good_name(entry.package) {
        return Err(Error::BadName);
    }
    let mut writer = Writer { out, at: 0 };
    let mut replaced = false;
    for existing in entries(table) {
        let existing = existing?;
        if existing.program == entry.program {
            writer.entry(entry)?;
            replaced = true;
        } else {
            writer.entry(&existing)?;
        }
    }
    if !replaced {
        writer.entry(entry)?;
    }
    Ok(writer.at)
}

/// The next generation: `table` without `program`. Removing what is not installed is
/// [`Error::NotInstalled`] rather than an identical generation, because an uninstall that silently
/// did nothing is a report the caller should see.
pub fn without_entry(table: &str, program: &str, out: &mut [u8]) -> Result<usize, Error> {
    let mut writer = Writer { out, at: 0 };
    let mut removed = false;
    for existing in entries(table) {
        let existing = existing?;
        if existing.program == program {
            removed = true;
        } else {
            writer.entry(&existing)?;
        }
    }
    if removed {
        Ok(writer.at)
    } else {
        Err(Error::NotInstalled)
    }
}

/// The live generation's number, from the `current` file: decimal digits and an optional newline.
pub fn parse_current(text: &str) -> Option<u32> {
    let digits = text.strip_suffix('\n').unwrap_or(text);
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    digits.parse().ok()
}

/// Write the `current` file naming generation `number`. Returns the length written.
pub fn format_current(number: u32, out: &mut [u8]) -> Result<usize, Error> {
    let mut digits = [0u8; 10];
    let mut n = number;
    let mut len = 0;
    loop {
        digits[len] = b'0' + (n % 10) as u8;
        len += 1;
        n /= 10;
        if n == 0 {
            break;
        }
    }
    let mut writer = Writer { out, at: 0 };
    for i in (0..len).rev() {
        writer.bytes(&[digits[i]])?;
    }
    writer.bytes(b"\n")?;
    Ok(writer.at)
}

fn good_name(name: &str) -> bool {
    !name.is_empty() && !name.starts_with('#') && name.bytes().all(|b| b > b' ' && b != 0x7f)
}

struct Writer<'o> {
    out: &'o mut [u8],
    at: usize,
}

impl Writer<'_> {
    fn bytes(&mut self, b: &[u8]) -> Result<(), Error> {
        let end = self.at + b.len();
        self.out
            .get_mut(self.at..end)
            .ok_or(Error::TooSmall)?
            .copy_from_slice(b);
        self.at = end;
        Ok(())
    }

    fn entry(&mut self, e: &Entry<'_>) -> Result<(), Error> {
        self.bytes(e.program.as_bytes())?;
        self.bytes(b" ")?;
        self.bytes(e.package.as_bytes())?;
        self.bytes(b" ")?;
        self.bytes(&measured_boot::hex(&e.digest))?;
        self.bytes(b"\n")
    }
}

#[cfg(test)]
mod tests {
    extern crate std;
    use std::collections::BTreeMap;
    use std::string::{String, ToString};
    use std::{format, vec};

    use super::*;

    /// The store as the protocol in the crate documentation uses it: generation files that are
    /// written once, and one `current` line. A `BTreeMap` stands in for the directory.
    struct Store {
        files: BTreeMap<String, String>,
    }

    impl Store {
        fn new() -> Self {
            let mut files = BTreeMap::new();
            files.insert("generation-0".to_string(), String::new());
            files.insert("current".to_string(), "0\n".to_string());
            Store { files }
        }
        fn current(&self) -> u32 {
            parse_current(&self.files["current"]).unwrap()
        }
        fn table(&self) -> &str {
            &self.files[&format!("generation-{}", self.current())]
        }
        fn newest(&self) -> u32 {
            self.files
                .keys()
                .filter_map(|k| k.strip_prefix("generation-"))
                .map(|n| n.parse::<u32>().unwrap())
                .max()
                .unwrap()
        }
        /// Write the next generation (never an existing file), then flip `current`: the commit.
        fn commit(&mut self, next: &[u8]) {
            let number = self.newest() + 1;
            let name = format!("generation-{number}");
            assert!(
                !self.files.contains_key(&name),
                "a generation was rewritten"
            );
            self.files
                .insert(name, String::from_utf8(next.to_vec()).unwrap());
            self.select(number);
        }
        fn select(&mut self, number: u32) {
            let mut out = [0u8; 16];
            let n = format_current(number, &mut out).unwrap();
            self.files.insert(
                "current".to_string(),
                String::from_utf8(out[..n].to_vec()).unwrap(),
            );
        }
        fn install(&mut self, entry: Entry<'_>) {
            let mut out = vec![0u8; 4096];
            let n = with_entry(self.table(), &entry, &mut out).unwrap();
            self.commit(&out[..n]);
        }
        fn remove(&mut self, program: &str) -> Result<(), Error> {
            let mut out = vec![0u8; 4096];
            let n = without_entry(self.table(), program, &mut out)?;
            self.commit(&out[..n]);
            Ok(())
        }
        fn package_of(&self, program: &str) -> Option<String> {
            lookup(self.table(), program)
                .unwrap()
                .map(|e| e.package.to_string())
        }
    }

    fn entry<'a>(program: &'a str, package: &'a str, seed: u8) -> Entry<'a> {
        Entry {
            program,
            package,
            digest: [seed; 32],
        }
    }

    /// **The property §208 asked for by name**: a rollback restores the whole set, not one package.
    /// Two programs installed, one upgraded, one removed; selecting the generation before both
    /// changes brings back the old version of the first and the presence of the second together.
    #[test]
    fn a_rollback_restores_the_whole_set() {
        let mut store = Store::new();
        store.install(entry("uptime", "uptime-0.1.0-aarch64", 1));
        store.install(entry("date", "date-1.0.0-aarch64", 2));
        let before = store.current();

        store.install(entry("uptime", "uptime-0.2.0-aarch64", 3));
        store.remove("date").unwrap();
        assert_eq!(store.package_of("uptime").unwrap(), "uptime-0.2.0-aarch64");
        assert_eq!(store.package_of("date"), None);

        store.select(before);
        assert_eq!(store.package_of("uptime").unwrap(), "uptime-0.1.0-aarch64");
        assert_eq!(store.package_of("date").unwrap(), "date-1.0.0-aarch64");
        // And rolling forward again is the same act.
        let newest = store.newest();
        store.select(newest);
        assert_eq!(store.package_of("date"), None);
    }

    #[test]
    fn an_upgrade_replaces_in_place_and_an_install_appends() {
        let mut store = Store::new();
        store.install(entry("a", "a-1-x", 1));
        store.install(entry("b", "b-1-x", 2));
        store.install(entry("a", "a-2-x", 3));
        let programs: std::vec::Vec<_> =
            entries(store.table()).map(|e| e.unwrap().program).collect();
        assert_eq!(programs, ["a", "b"]);
        assert_eq!(lookup(store.table(), "a").unwrap().unwrap().digest, [3; 32]);
    }

    #[test]
    fn removing_what_is_not_installed_is_reported_and_writes_nothing() {
        let mut store = Store::new();
        store.install(entry("a", "a-1-x", 1));
        let newest = store.newest();
        assert_eq!(store.remove("b"), Err(Error::NotInstalled));
        assert_eq!(store.newest(), newest);
    }

    #[test]
    fn a_malformed_line_anywhere_makes_the_table_vouch_for_nothing() {
        let good = "a a-1-x ".to_string() + &"11".repeat(32) + "\n";
        assert!(lookup(&good, "a").unwrap().is_some());
        for bad in [
            format!("{good}b b-1-x nothex\n"),
            format!("{good}b b-1-x\n"),
            format!("{good}b b-1-x {} extra\n", "22".repeat(32)),
            format!("{good}b b-1-x {}\n", "2".repeat(63)),
        ] {
            assert_eq!(lookup(&bad, "a"), Err(Error::Malformed), "{bad:?}");
        }
        // Comments and blank lines are not entries.
        let commented = format!("# installed\n\n{good}");
        assert!(lookup(&commented, "a").unwrap().is_some());
    }

    #[test]
    fn a_name_that_would_not_read_back_is_refused() {
        let mut out = [0u8; 256];
        for (program, package) in [("", "p"), ("a b", "p"), ("a", "p\nq"), ("#a", "p")] {
            let e = Entry {
                program,
                package,
                digest: [0; 32],
            };
            assert_eq!(
                with_entry("", &e, &mut out),
                Err(Error::BadName),
                "{program:?}"
            );
        }
    }

    #[test]
    fn a_buffer_too_small_is_refused_rather_than_truncated() {
        let mut out = [0u8; 40];
        let e = entry("uptime", "uptime-0.1.0-aarch64", 1);
        assert_eq!(with_entry("", &e, &mut out), Err(Error::TooSmall));
        assert_eq!(format_current(1234, &mut [0u8; 4]), Err(Error::TooSmall));
    }

    #[test]
    fn the_current_line_reads_back() {
        for n in [0, 7, 10, 4_294_967_295] {
            let mut out = [0u8; 16];
            let len = format_current(n, &mut out).unwrap();
            assert_eq!(
                parse_current(core::str::from_utf8(&out[..len]).unwrap()),
                Some(n)
            );
        }
        for bad in ["", "\n", "-1", "1 ", "4294967296", "0x1"] {
            assert_eq!(parse_current(bad), None, "{bad:?}");
        }
    }
}
