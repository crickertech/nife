# What a nife package is, and what still cannot be done with one

Milestone 198 (a package manager, and the trivial install that makes a second customer possible)'s
rung 3a has two halves. This note is the producer half and the format both halves share, built
2026-09-23, and the first part of the consumer half, built 2026-09-24: a target fetches a package
over plain HTTP and accepts it only by a digest its own image vouches for. Since 2026-09-26 an
installed program runs by its bytes (DECISIONS §219 option D). Nothing on the target installs one
yet: see "Where this stops" below.

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

Those two together say what a package *is* and what decides whether its bytes may run. What
installing one does was the third fork, and DECISIONS §208 (installing a package is granting it, and
the activation set is versioned) ruled it on 2026-09-23.

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
these offsets. Its manifest question was ruled 2026-09-26: an ELF note inside the executable, not
a member. Whether the digest is a Merkle root is still open; this takes the plain SHA-256 §197
records as the default. §197 says the day somebody outside this repository fetches a package
is the day the format is fixed; nobody has.

## The producer

`cargo xtask package <recipe>` turns a reviewed recipe into one package file, a digest, and a
catalogue line. `packages/uptime.recipe` is the worked example and a real recipe rather than a
fixture.

```
$ cargo build -p components --bin uptime --target aarch64-unknown-none-softfloat
$ cargo xtask package packages/uptime.recipe
uptime 0.1.0 aarch64, 2 members, 90491 bytes
  uptime                       89168 bytes  d801cd2b65dbfbd4982226394c8ad5de471d4f782f39eb16c5ee06cd71bcef26
  uptime.licence                1067 bytes  dba2f854c33606c0a4f028f88baf9d9bcef8d30714e29c8f3314dd5eeebf1c14
digest d74f8eb3ecc14b43b9f55b2113c830ee856394eee4398b52da2da1614d426cd9  (the recipe records none; review it and add it)
wrote target/packages/uptime-0.1.0-aarch64.nifepkg
package: PASS
```

A program is packed stripped since 2026-09-24, the same bytes the image packs. The first run
above read 881,152 bytes for `uptime`, nearly all of it debug sections, against a job region of 40
pages (160 KiB): an installed program is bytes something on the target must read into memory to
build a process from, and nothing there reads DWARF. The digests in this note's history changed
with it; nobody had fetched one.

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

## The consumer's first half: fetch, and verify against the image

Built 2026-09-24 by the rung 3a consumer lane. Three pieces, each doing one thing:

- The image carries its own package source. Every archive build (`cargo xtask initrd-aarch64`
  and `initrd-riscv`) runs every recipe under `packages/` for its architecture, writes the package
  to `target/packages/`, and packs the catalogue lines as the archive entry
  `package_archive::CATALOGUE` (provisional name), above the measurement table. So the kernel's
  trust root vouches for the catalogue, and the catalogue vouches for the package. That is §195's
  "the image's measured table becomes the first source" taken literally, and it is why plain HTTP
  is enough on this rung: the digest the client checks against never crossed the network. It also
  means the producer runs end to end on every build, which this note's BUGS said nothing did.
- A host on the network serves it. `helpers/package-http-peer` is a `guestfwd` peer at
  10.0.2.9:8080 in both QEMU runners, started by slirp once per connection with the connection on
  its standard input and output, exactly as the TCP echo peer at 10.0.2.9:7777 is a `/bin/cat`. A
  real HTTP/1.0 exchange with a real host process, and nothing binds a port on the machine or
  outlives QEMU. `GET /tampered/<name>` serves the same file with one byte flipped halfway through.
- The client hashes as it reads. `crates/http_response` (provisional name) writes the `GET` and
  reads the response a socket read at a time, keeping only the head, so the body goes straight
  into `measured_boot`'s streaming SHA-256 and a package costs the client one page of socket
  frame. The client is a mode of `net_stack`'s socket-contract client (`TEST_HTTP_PACKAGE`), because
  that is the only thing in the tree that holds a `Stack` capability.

### EXAMPLES

```
$ script/test --arch aarch64 --test package
running 1 of 359 tests (filter: package)
test kernel::user::tests::a_package_fetched_over_http_is_accepted_only_by_the_image_digest ... ok
test result: ok. 1 passed
$ cat target/packages/catalogue
uptime-0.1.0-aarch64 d74f8eb3ecc14b43b9f55b2113c830ee856394eee4398b52da2da1614d426cd9
```

The test fetches twice through one `net_stack`: the genuine package, which must be accepted, then
the tampered copy, which must be refused. The second fetch is what gives the first its meaning. The
tampered response is a complete, correct HTTP exchange of the right length; only the digest can
tell, and a client that accepted whatever arrived would pass the first half too. They share one
spawn because every `net_stack` a test starts holds a virtio slot for the rest of the boot; this
lane took the table's tenth bump (`MAX_DEVICES`, to 34) and filed the unregister it keeps deferring
as `design/roadmap/proposals/a-virtio-slot-comes-back-when-its-driver-dies.md`. riscv64 runs the
same test as a twin in `kernel/src/user/riscv_virtio_tests.rs`.

### The versioned table, as logic

`crates/activation_set` (provisional name) is §208's second clause as a pure, host-tested crate: a
generation is a text file of `<program> <package> <digest>` lines that is never rewritten, a
one-line `current` names the live one, and install, upgrade and remove each produce the next
generation. Its test `a_rollback_restores_the_whole_set` is the property calef asked for by name.
Nothing on a target reads it yet, for the reason below.

## Running what was installed, by its bytes

DECISIONS §219 (how the shell names an installed program to the spawner) was ruled on 2026-09-26:
option D, the executable's bytes as frames the caller owns, with gate D2. It is built.

```
$ installed/uptime
  up 00:00:05
$ installed/unvouched
    refused: those bytes are not in the activation set, and running unvouched bytes needs a capability this session does not hold
```

A command word with a `/` in it is a file. The shell binds the line against
`grant_plan::INSTALLED_MANIFEST_OF`, which is `uptime`'s manifest and the ceiling every installed
program gets until its manifest is read from its ELF note (§197, ruled 2026-09-26). It opens the
file and sends `spawnproto::request(len, ..)` with `IMAGE_BIT` set: word 0 is the byte length, then
one `SEND_CAP` per page narrowed to `READ`, then the grants as today, holding one frame at a time.

The progenitor maps each frame through the loader's never-reused scratch window, copies it into a
page of its own, and deletes the capability before taking the next. It copies because the caller
keeps a mapping of its frames and could change them between a hash and a build. It hashes the copy,
reads `activation/current` and then that generation through the file service it already held, and
looks the digest up (`activation_set::lookup_digest`). A hit is built from the copy. A miss gets
`SPAWN_UNVOUCHED`, whose sentence names the missing capability: D2, which no session holds yet.

The digest is the member's, not the package's: the spawner is handed the executable, and the
package's table of contents already carries each member's digest. The recipe's digest over the whole file (§195 (a reviewed recipe vouches for a package))
is still what installing checks first; the activation table records the member's.

### What proves it

`script/swish-check` seeds the RedoxFS image on the host the way the installer will
(`seed_installed` in `xtask/src/disk.rs`: the package built by the producer, the member and its
table-of-contents digest, one generation) and types the two lines above. Green on aarch64, riscv64
and x86_64 (under OVMF) on 2026-09-26. Both lines were falsified on aarch64. A seeded digest with one
bit flipped refuses the first line; a progenitor that vouches for everything runs the witness in
the second.

### What it costs, measured 2026-09-26

| | aarch64 | riscv64 | x86_64 |
|---|---|---|---|
| `uptime`, stripped | 89,168 bytes, 22 frames | 49,168 bytes, 13 frames | 28,352 bytes, 7 frames |
| Messages on the spawn endpoint | 1 `SEND` + 22 `SEND_CAP`s | 1 + 13 | 1 + 7 |
| File-service calls by the progenitor | 8 (3 opens, 2 reads, 3 closes) | 8 | 8 |
| Pages staged on each side, returned after | 22 | 13 | 7 |
| Progenitor capability peak | 23 of 24, unchanged | 23 of 24, unchanged | the hand-over mark only |

At most one of the caller's frames is in the progenitor's table at a time, beside the staging
region. The gauge reports the boot's high-water mark, so it shows the image path stays under the
peak without measuring the path itself.

One-time costs are the shell's primer page with its page tables, and the progenitor's mapping of the
file page. Per image spawn, the progenitor's scratch window advances a page per frame, which costs a
page table from its own budget about every 23 `uptime` runs on aarch64. Every child already pays
that debt (§162 (whether a holder can give up a mapping)); this pays it about half again as fast.

Two orderings are load-bearing, because a region returns its pages to its parent only if it is the
parent's most recent carve (`memory_regions`' `return_to_parent`). The progenitor splits the child's
region before the staging region, and the shell maps one primer page once so the window's page
tables exist before any staging region does. Either one wrong silently strands every staging page.

## Where this stops

Rung 3a's exit criterion is a package fetched, verified, installed, run, still there after a reboot,
rolled back and removed. Fetch, verify and run are built. Installing on the target is next: writing
a generation and `current`, placing the member, and deciding who may write `activation/`. Reboot,
rollback and removal follow, since each is a file the installer writes. After that, §219's gate D2
lets a miss run with only what the caller delegated, and milestone 202 (every confinement test is a
ritual until somebody breaks the confinement)'s unvouched-child probe becomes testable;
`installed/unvouched` is already its fixture.

## BUGS

- The activation set lives where the shell can write it: `activation/` at the root of the file
  service, which the shell holds with its boot rights. A session can add a digest to the live
  generation and have its bytes vouched. That grants nothing extra today, because every installed
  program is endowed as `uptime` is, and it stops being harmless the day a manifest travels. The
  installer is what must put the table where no session can write.
- An installed program's manifest is a ceiling, not its own (`grant_plan::INSTALLED_MANIFEST_OF`).
  A program that needs more than `uptime` finds its slot empty rather than being refused by name.
- Only a plain line runs an image. A path in a pipe or behind a redirection reaches the planner as
  a program name and is refused as "no such program", and `caps <path>` prints no `provenance:`
  row. `crates/grant_plan/src/spawnproto.rs`'s `BUGS` has the full list.
- The installed file and its generation are seeded on the host, standing in for an installer.

- **No compression.** `.hpkg` chunks its heap with zlib and `.apk` is three gzip streams; this
  stores members whole. The first packages are ELFs that were about to be written to a disk anyway,
  and a compressor is a second hostile-input parser on the same path. The size cost is measured
  nowhere.
- **A package is bounded by `u32`** in both member length and file length.
- The catalogue is one file in `target/` and one archive entry, not a repository index. The
  image's own source is the only source; §195's per-source trust needs a catalogue per source the
  owner opted into, and a way to add one.
- The fetch runs only in the kernel's test harness, which plays the progenitor's part and maps
  the catalogue into the client the way the progenitor hands `login` its blobs. The booted system
  has no network (the proposal above).
- x86_64 has no fetch test: its QEMU runner attaches no `-netdev`, and no x86 network test
  exists. The archive build does not pack a catalogue for it either, because nothing there could
  read one. Milestone 494 (a driver for the network card a PC actually has) is where x86 networking
  starts.
- `uptime` is also in the image, so the package the tests fetch is not a program the image
  lacks. The tests prove the bytes, not an install; "absent from the image" is the install tests'
  criterion. The run-by-bytes line in `script/swish-check` has the same limit: it runs a copy of
  the image's own `uptime`, vouched by a generation, which proves the path and not novelty. The
  installer's tests are where a program the image lacks first runs.
- The package peer is a `guestfwd` process, not a server on a LAN. It speaks HTTP to the guest
  over slirp's forwarding, which is enough to prove the client and not enough to prove a real
  network card or a host elsewhere on a network (rung 3b).
- Plain HTTP carries the package, and that is safe only because of the image's catalogue. A
  source whose digests arrive over the same connection would be worth nothing against a machine in
  the middle; that is what §196 (nife carries TLS)'s TLS is for on rung 3c.
- **A recipe cannot say where its source came from.** Homebrew's formula carries an upstream URL and
  a digest of the tarball; this carries neither, because every package that exists is built from
  this repository. The first out-of-tree package is what forces it.
- **`cargo xtask package` builds nothing.** A `program` whose ELF is not in `target/` is an error
  naming the file. Packaging and building are separate acts here for milestone 150 (adding a program should not need eight hand-maintained lists)'s reason: a tool
  that quietly rebuilt would hide which binary it had packed.
