//! **`cargo xtask package`**: build a package from a reviewed recipe, and say what its digest is.
//!
//! Rung 3a of milestone 198 (a package manager, and the trivial install that makes a second
//! customer possible) has two halves, and this is the producer.
//! DECISIONS §195 (a reviewed recipe vouches for a package), in
//! [its own file](../../design/decisions/195-a-recipe-vouches-and-the-owner-may-overrule.md),
//! puts a package's digest in a version-controlled recipe changed by human review, which is
//! Homebrew's arrangement; this is the tool that turns such a recipe into the one archive file
//! DECISIONS §197 (a package is one archive file), in
//! [its own file](../../design/decisions/197-a-package-is-one-archive-file.md), rules a package
//! is.
//!
//! **nife cannot build software**, so a package is produced here, on a host with a cross-toolchain,
//! and consumed by a target with no compiler. That is why this lives in `xtask` and why a recipe
//! names an architecture: three triples, three packages, §197's "per-architecture output is the
//! normal case".
//!
//! # EXAMPLES
//!
//! ```text
//! $ cargo xtask package packages/uptime.recipe
//! uptime 0.1.0 aarch64, 2 members, 4600 bytes
//!   uptime          4528 bytes  7d8e...c1
//!   uptime.licence    68 bytes  a3f0...9e
//! wrote target/packages/uptime-0.1.0-aarch64.nifepkg
//! digest 9f2c...4b  (recorded in the recipe, and it matches)
//! package: PASS
//! ```
//!
//! A recipe with no `digest` line builds and prints the digest to paste in; one whose `digest`
//! disagrees with the bytes fails, which is the whole mechanism: the reviewed line is what decides
//! whether the bytes may run, so a rebuild that does not reproduce it is a fact somebody must see.
//!
//! # The recipe format
//!
//! One directive per line, `#` comments and blank lines skipped. Provisional, like everything else
//! this lane named.
//!
//! ```text
//! name uptime                 # required, at most package_archive::NAME_LEN bytes
//! version 0.1.0               # required
//! architecture aarch64        # required: aarch64, riscv64 or x86_64
//! program uptime              # a built ELF for that architecture, resolved from target/
//! member uptime.licence LICENSE-MIT   # any file, by a path relative to the repository root
//! digest 9f2c...              # optional: 64 hex characters, checked against what was built
//! ```
//!
//! `program` exists so a recipe does not have to spell a target triple and a cargo profile, which
//! are facts about this checkout rather than about the package.
//!
//! # BUGS
//!
//! - **It builds nothing.** A `program` whose ELF is not in `target/` is an error naming the file,
//!   not a cargo invocation. Packaging and building are separate acts here for the reason milestone
//!   150 gives about hand-maintained lists: a tool that quietly rebuilt would hide which binary it
//!   had packed.
//! - **Nothing installs the result.** The activation fork
//!   (milestone 507 (installing a package: mutate, compose, or widen what can be spawned)) is
//!   unruled, so a package
//!   is a file the target cannot yet do anything with. Rung 3a's consumer half waits on that.
//! - **The catalogue is one line printed and one file written**, not a repository index. §195's
//!   per-source trust needs a catalogue per source and a client that reads one; neither exists.
//! - **A recipe cannot say where its source came from.** Homebrew's formula carries a URL and a
//!   digest of the upstream tarball; this carries neither, because the only packages that exist are
//!   built from this repository.

use std::path::PathBuf;

use package_archive::{Attributes, Package, package_size, sha256, write_package};

use crate::host::workspace_root;
use crate::{RISCV_TARGET, TARGET, X86_TARGET, profile_dir};

/// Where built packages land. Under `target/` because a package is an artifact, and because
/// nothing in this tree is ready to publish one.
const OUTPUT: &str = "target/packages";

/// A recipe, parsed. Borrowed out of the text so the parser can be a pure function a test can call
/// with a string literal, which is the shape every parser in this tree that is worth testing has.
#[derive(Debug, PartialEq, Eq)]
struct Recipe<'a> {
    name: &'a str,
    version: &'a str,
    architecture: &'a str,
    /// `(member name, where its bytes come from)`, in the order the recipe lists them, because the
    /// package's bytes are a function of that order and a reviewed digest depends on it.
    members: Vec<(&'a str, Source<'a>)>,
    digest: Option<&'a str>,
}

#[derive(Debug, PartialEq, Eq)]
enum Source<'a> {
    /// A built ELF for the recipe's architecture, resolved from `target/`.
    Program(&'a str),
    /// A path relative to the repository root.
    File(&'a str),
}

/// `cargo xtask package <recipe>`: build it, check it against the recipe, and report.
pub(crate) fn package(recipe_path: Option<String>) -> bool {
    let Some(recipe_path) = recipe_path else {
        eprintln!("usage: cargo xtask package <recipe>");
        return false;
    };
    let root = workspace_root();
    let text = match std::fs::read_to_string(root.join(&recipe_path)) {
        Ok(text) => text,
        Err(error) => {
            eprintln!("package: could not read {recipe_path}: {error}");
            return false;
        }
    };
    let recipe = match parse_recipe(&text) {
        Ok(recipe) => recipe,
        Err(complaint) => {
            eprintln!("package: {recipe_path}: {complaint}");
            return false;
        }
    };

    // Read every member first, so a missing file is reported before anything is written.
    let mut bytes = Vec::new();
    for (name, source) in &recipe.members {
        let path = match resolve(&root, recipe.architecture, source) {
            Ok(path) => path,
            Err(complaint) => {
                eprintln!("package: {name}: {complaint}");
                return false;
            }
        };
        match std::fs::read(&path) {
            Ok(content) => bytes.push(content),
            Err(error) => {
                eprintln!(
                    "package: {name}: could not read {}: {error}",
                    path.display()
                );
                return false;
            }
        }
    }
    let members: Vec<(&str, &[u8])> = recipe
        .members
        .iter()
        .zip(&bytes)
        .map(|((name, _), content)| (*name, content.as_slice()))
        .collect();

    let attributes = Attributes {
        name: recipe.name,
        version: recipe.version,
        architecture: recipe.architecture,
    };
    let mut file = vec![0u8; package_size(&members)];
    if let Err(error) = write_package(&attributes, &members, &mut file) {
        eprintln!("package: refused: {error:?}");
        return false;
    }

    // Read the package back with the target's own parser before writing it out. The producer and
    // the consumer share one definition of the format, and this is where that stops being a claim:
    // a writer that could emit a file its reader refuses would ship one.
    let parsed = match Package::parse(&file) {
        Ok(parsed) => parsed,
        Err(error) => {
            eprintln!("package: wrote a file its own reader refuses: {error:?}");
            return false;
        }
    };
    if let Err(error) = parsed.verify() {
        eprintln!("package: member {} does not match its digest", error.index);
        return false;
    }

    println!(
        "{} {} {}, {} members, {} bytes",
        recipe.name,
        recipe.version,
        recipe.architecture,
        parsed.len(),
        file.len()
    );
    for index in 0..parsed.len() {
        let name = parsed.member_name(index).unwrap_or("");
        let len = parsed.member(index).map(<[u8]>::len).unwrap_or(0);
        let digest = parsed.member_digest(index).unwrap_or_default();
        println!("  {name:<24} {len:>9} bytes  {}", hex(&digest));
    }

    let digest = hex(&sha256(&file));
    let stem = format!("{}-{}-{}", recipe.name, recipe.version, recipe.architecture);

    // **The recorded digest is checked before anything is written**, which is the order §195 asks
    // for even though it costs a rebuild to find out. A package whose bytes do not reproduce the
    // reviewed line is one nothing accepts, so leaving it on disk beside a catalogue entry
    // vouching for it would be the tool disagreeing with itself.
    match recipe.digest {
        Some(recorded) if recorded == digest => {
            println!("digest {digest}  (recorded in the recipe, and it matches)");
        }
        Some(recorded) => {
            eprintln!("package: the recipe records {recorded}");
            eprintln!("package: these bytes are  {digest}");
            eprintln!(
                "package: a rebuild that does not reproduce the reviewed digest is the failure \
                 DECISIONS §195 exists to make visible; nothing was written."
            );
            return false;
        }
        None => println!("digest {digest}  (the recipe records none; review it and add it)"),
    }

    let output = root.join(OUTPUT);
    if let Err(error) = std::fs::create_dir_all(&output) {
        eprintln!("package: could not create {}: {error}", output.display());
        return false;
    }
    let written = output.join(format!("{stem}.nifepkg"));
    if let Err(error) = std::fs::write(&written, &file) {
        eprintln!("package: could not write {}: {error}", written.display());
        return false;
    }
    // The catalogue line is `measured_boot`'s manifest shape (a name, a space, 64 hex characters),
    // which is the format the progenitor already reads to decide whether a program may run. §195
    // makes the image's measurement table the first source of trust, so a package's entry looking
    // like an entry in that table is the point rather than a coincidence.
    let catalogue = output.join("catalogue");
    if let Err(error) = append(&catalogue, &format!("{stem} {digest}\n")) {
        eprintln!("package: could not write {}: {error}", catalogue.display());
        return false;
    }
    println!("wrote {}", relative(&root, &written));
    println!("package: PASS");
    true
}

/// Append a catalogue line, replacing any earlier line for the same name so a rebuild does not
/// leave two answers to one question.
fn append(path: &std::path::Path, line: &str) -> std::io::Result<()> {
    let name = line.split(' ').next().unwrap_or_default();
    let existing = std::fs::read_to_string(path).unwrap_or_default();
    let mut text: String = existing
        .lines()
        .filter(|kept| kept.split(' ').next() != Some(name))
        .map(|kept| format!("{kept}\n"))
        .collect();
    text.push_str(line);
    std::fs::write(path, text)
}

fn relative(root: &std::path::Path, path: &std::path::Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
}

fn hex(digest: &[u8]) -> String {
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Where a member's bytes live on this host.
fn resolve(
    root: &std::path::Path,
    architecture: &str,
    source: &Source<'_>,
) -> Result<PathBuf, String> {
    match source {
        Source::File(path) => Ok(root.join(path)),
        Source::Program(name) => {
            let triple = match architecture {
                "aarch64" => TARGET,
                "riscv64" => RISCV_TARGET,
                "x86_64" => X86_TARGET,
                other => return Err(format!("unknown architecture {other}")),
            };
            let path = root.join(format!("target/{triple}/{}/{name}", profile_dir()));
            if path.exists() {
                Ok(path)
            } else {
                Err(format!(
                    "{} is not built; build it first (cargo xtask initrd-{})",
                    relative(root, &path),
                    if architecture == "aarch64" {
                        "aarch64"
                    } else if architecture == "riscv64" {
                        "riscv"
                    } else {
                        "x86"
                    }
                ))
            }
        }
    }
}

/// Parse a recipe, or say which line could not be read and why.
///
/// A pure function over the text, so the interesting half is host-testable in milliseconds, which
/// is what `AGENTS.md` asks of every parser in this tree.
fn parse_recipe(text: &str) -> Result<Recipe<'_>, String> {
    let mut name = None;
    let mut version = None;
    let mut architecture = None;
    let mut digest = None;
    let mut members = Vec::new();

    for (number, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let number = number + 1;
        let (directive, rest) = line.split_once(' ').unwrap_or((line, ""));
        let rest = rest.trim();
        if rest.is_empty() {
            return Err(format!("line {number}: {directive} says nothing"));
        }
        match directive {
            "name" => name = Some(rest),
            "version" => version = Some(rest),
            "architecture" => architecture = Some(rest),
            "digest" => {
                if rest.len() != 64 || !rest.bytes().all(|b| b.is_ascii_hexdigit()) {
                    return Err(format!("line {number}: a digest is 64 hex characters"));
                }
                digest = Some(rest);
            }
            "program" => members.push((rest, Source::Program(rest))),
            "member" => {
                let Some((member, path)) = rest.split_once(' ') else {
                    return Err(format!("line {number}: member needs a name and a path"));
                };
                members.push((member, Source::File(path.trim())));
            }
            other => return Err(format!("line {number}: unknown directive {other}")),
        }
    }

    Ok(Recipe {
        name: name.ok_or("no name".to_string())?,
        version: version.ok_or("no version".to_string())?,
        architecture: architecture.ok_or("no architecture".to_string())?,
        members,
        digest,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const RECIPE: &str = "\
# a comment
name uptime
version 0.1.0
architecture aarch64

program uptime
member uptime.licence LICENSE-MIT
";

    #[test]
    fn a_recipe_reads_as_what_it_says() {
        let recipe = parse_recipe(RECIPE).unwrap();
        assert_eq!(recipe.name, "uptime");
        assert_eq!(recipe.version, "0.1.0");
        assert_eq!(recipe.architecture, "aarch64");
        assert_eq!(recipe.digest, None);
        assert_eq!(
            recipe.members,
            vec![
                ("uptime", Source::Program("uptime")),
                ("uptime.licence", Source::File("LICENSE-MIT")),
            ]
        );
    }

    #[test]
    fn a_missing_field_is_named() {
        for (line, missing) in [
            ("name uptime", "no version"),
            ("version 0.1.0", "no name"),
            ("name uptime\nversion 0.1.0", "no architecture"),
        ] {
            assert_eq!(parse_recipe(line).unwrap_err(), missing);
        }
    }

    #[test]
    fn a_digest_that_is_not_a_digest_is_refused() {
        // The one field where a typo would otherwise pass review and then fail a build on another
        // host, which is the failure §195's whole arrangement is trying not to have.
        let short = format!("{RECIPE}digest abc123\n");
        assert_eq!(
            parse_recipe(&short).unwrap_err(),
            "line 8: a digest is 64 hex characters"
        );
        let good = format!("{RECIPE}digest {}\n", "a".repeat(64));
        assert_eq!(
            parse_recipe(&good).unwrap().digest,
            Some("a".repeat(64)).as_deref()
        );
    }

    #[test]
    fn a_line_nobody_can_read_names_itself() {
        assert_eq!(
            parse_recipe("name uptime\nfetch https://example.invalid\n").unwrap_err(),
            "line 2: unknown directive fetch"
        );
        assert_eq!(
            parse_recipe("name uptime\nmember justaname\n").unwrap_err(),
            "line 2: member needs a name and a path"
        );
        assert_eq!(
            parse_recipe("name\n").unwrap_err(),
            "line 1: name says nothing"
        );
    }
}
