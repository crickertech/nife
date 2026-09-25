# What a nife package is, and what still cannot be done with one

Milestone 198 (a package manager, and the trivial install that makes a second customer possible)'s
rung 3a has two halves. This note is the producer half and the format both halves share, built
2026-09-23. **Nothing installs a package yet**, and the reason is a ruling rather than a gap: see
"Where this stops" below.

## The two decisions this is downstream of

DECISIONS §197 (a package is one archive file), in
[its own file](../design/decisions/197-a-package-is-one-archive-file.md), ruled the container: one archive file per package, identified by name and version, the shape
`.deb`, `.apk` and `.hpkg` all use. It accepted a cost out loud while doing it, because a container
is bytes a target must parse: *"the reader owes a fuzz target and the Kani treatment `crates/nifefs`
and `crates/elf` already carry."*

DECISIONS §195 (a reviewed recipe vouches for a package), in
[its own file](../design/decisions/195-a-recipe-vouches-and-the-owner-may-overrule.md), ruled trust: a package's digest lives in a version-controlled recipe
changed by human review, trust is scoped per source the owner opted into, and the owner may
overrule. That is Homebrew's arrangement, and Homebrew is also §197's worked example of the pairing.

Those two together say what a package *is* and what decides whether its bytes may run. They say
nothing about what installing one does, which is the third fork and is unruled.

## The format

`crates/package_archive` holds it, and holds it once: the host tool writes a package with the same
code a target reads one with, which is the arrangement `crates/nifefs` has for the boot archive.
The crate's own documentation carries the byte layout and the reasons; three choices are worth
repeating here because a reader comparing this against `.hpkg` will ask about them.

- **Offsets are absolute**, so a reader never has to know where the table of contents ends to find
  data. `nifefs` made the same call about `start_block`.
- **Every member carries its own SHA-256**, where `.hpkg` carries none and keeps digests in the
  repository index. §195 puts a digest over the whole file in the recipe, which answers *are these
  the reviewed bytes*; the per-member digest answers *is this member the one that was reviewed*,
  which is what a spawner handed one program out of a package needs to ask. It is also the shape
  `crates/measured_boot`'s table already has, so a package's table of contents is that table
  travelling with its bytes.
- **Member bytes are 8-byte aligned.** The largest member is an ELF, and the cheapest way to be
  wrong later is to hand a parser an odd address.

**The encoding is provisional and the crate's name is provisional.** §197 ruled the container, not
these offsets, and two of its own open questions are deliberately left open here: where a program's
manifest travels (this layout makes a sibling member *possible*, because a member is any named
bytes, and requires nothing) and whether the digest is a Merkle root (this takes the plain SHA-256
§197 records as the default). §197 says the day somebody outside this repository fetches a package
is the day the format is fixed; nobody has.

## The producer

`cargo xtask package <recipe>` turns a reviewed recipe into one package file, a digest, and a
catalogue line. `packages/uptime.recipe` is the worked example and a real recipe rather than a
fixture.

```
$ cargo build -p components --bin uptime --target aarch64-unknown-none-softfloat
$ cargo xtask package packages/uptime.recipe
uptime 0.1.0 aarch64, 2 members, 882475 bytes
  uptime                      881152 bytes  9bdebc907fb3c7eff678a0f1d345098e1d21d509232b745b336b8a9fc731b4a8
  uptime.licence                1067 bytes  dba2f854c33606c0a4f028f88baf9d9bcef8d30714e29c8f3314dd5eeebf1c14
digest f303e834bfcde7f676389f61ca2abce25bba81a897a3cf00a09c464784e62a6f  (the recipe records none; review it and add it)
wrote target/packages/uptime-0.1.0-aarch64.nifepkg
package: PASS
```

Three things in that run are the mechanism rather than decoration.

**It reads its own output back with the target's parser before writing anything to disk.** A
producer that could emit a file its consumer refuses would ship one, and the gate that would catch
it does not exist yet on the target side.

**A recipe's recorded digest is checked before anything is written.** A rebuild that does not
reproduce the reviewed line is exactly the failure §195's arrangement exists to make visible, so the
tool prints both digests and writes nothing. That ordering costs a rebuild to discover and is worth
it: a package nothing accepts, sitting on disk beside a catalogue entry vouching for it, would be
the tool disagreeing with itself.

**The catalogue line is `measured_boot`'s manifest shape**, a name, a space, 64 hex characters,
which is what the progenitor already reads to decide whether a program may run. §195 makes the
image's measurement table the first source of trust, so a package's entry looking like an entry in
that table is the point.

**The bytes are a function of the inputs alone**: no timestamp, no ordering pass, no non-zero
padding. A reviewed digest is worth nothing if two hosts building the same recipe disagree, and
`the_same_inputs_give_the_same_bytes` is the host test that says so.

`packages/uptime.recipe` deliberately records **no** digest, and the comment in it says why: the
program it names is rebuilt by this checkout whenever anything it links changes, so a recorded
digest would be a number that fails for the next reader. A recorded number that is wrong is worse
than an absent one. The line goes in when there is a release to pin it to, which is rung 4.

## What proves it

- **11 host tests** in `crates/package_archive`, over the round trip, the refusals, alignment,
  determinism, and four ways a hostile file is refused rather than indexed.
- **4 host tests** over the recipe parser in `xtask/src/package.rs`.
- **`fuzz/fuzz_targets/package_archive_roundtrip`**, the first half of §197's accepted debt. 1.39
  million runs in 46 seconds on the development Mac on 2026-09-23, no crashes.
- **Two Kani harnesses** in the crate, the second half of that debt: a file the solver chose is
  either refused or reads only inside itself, and a file shorter than the header is refused rather
  than indexed. Both discharge, in **4 seconds** together, which is the row `script/verify`'s table
  now carries. **The first run failed and the failure was the bound, not the code**: at
  `#[kani::unwind(4)]` the eight-byte magic comparison reports an unwinding assertion inside
  `<builtin-library-memcmp>` and leaves 270 of 271 checks undetermined, which reads exactly like a
  refuted proof. 9 is the bound that covers it, and the reason is recorded at the attribute rather
  than here, because that is where the next person raising it will be looking.
- **The end-to-end run above**, which is the first package this project has produced.

## Where this stops, and it is a ruling rather than a gap

Rung 3a's exit criterion is a package fetched over a network, verified, installed, run, still there
after a reboot, and removable. Everything after "verified" waits on the **activation** fork
(milestone 507 (installing a package: mutate, compose, or widen what can be spawned)),
which has options and no winner and is an architect's. That proposal's own
finding is why it cannot be worked around: the program namespace is sealed at boot and the spawner
gives the file service away, so **nothing that builds processes can read an installed program
today**, whatever a package looks like.

The transport half stops in a different place and for a friendlier reason: §196 (nife carries TLS) rules HTTPS, and
under §195 a recipe's digest is what decides whether bytes may run, so rung 3a over a LAN needs no
TLS. What it needs is the client, and the client needs somewhere to put what it fetches.

## BUGS

- **No compression.** `.hpkg` chunks its heap with zlib and `.apk` is three gzip streams; this
  stores members whole. The first packages are ELFs that were about to be written to a disk anyway,
  and a compressor is a second hostile-input parser on the same path. The size cost is measured
  nowhere.
- **A package is bounded by `u32`** in both member length and file length.
- **The catalogue is one file in `target/`**, not a repository index. §195's per-source trust needs
  a catalogue per source and a client that reads one, and neither exists.
- **A recipe cannot say where its source came from.** Homebrew's formula carries an upstream URL and
  a digest of the tarball; this carries neither, because every package that exists is built from
  this repository. The first out-of-tree package is what forces it.
- **`cargo xtask package` builds nothing.** A `program` whose ELF is not in `target/` is an error
  naming the file. Packaging and building are separate acts here for milestone 150 (adding a program should not need eight hand-maintained lists)'s reason: a tool
  that quietly rebuilt would hide which binary it had packed.
- **Nothing gates the producer in CI.** The host tests and the fuzz target run; the end-to-end
  `cargo xtask package` run does not, because it needs a built user program and the gate that builds
  one is the archive gate. Wiring it is cheap and was left to the lane that has a consumer to gate
  with it.
