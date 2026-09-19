# What "trivial install" means, measured against a stranger's first ten minutes today

**Status: PROPOSED 2026-09-19.** Written by milestone 198's scoping lane
(`milestone/198-package-manager-scoping`). Shaped as a `design/decisions/` section for the
integrator to mint, and it carries the proposed first slice of milestone 198.

**Gate: DECISION.** calef rules the definition below. It is a definition and a sequencing choice,
both reversible, so this proposal **recommends**. Blocked until answered: milestone 198's second
half, which its own `BUGS` says "cannot be completed or even scoped" without one.

## What is being decided

Milestone 198 names the constraint and not the definition: *"a person with hardware and no prior
knowledge of this project reaches a running system."* This proposal writes the ten minutes down as
they are today, from the tree and from commands run on 2026-09-19, and then proposes the smallest
definition that passes principle 3's test.

## A stranger's first ten minutes today

**Minute 0. There is nothing to download.** `gh release list` on `crickertech/nife` returns nothing:
no release has ever been published. The repository is public and 35,985 KB (`gh api
repos/crickertech/nife`). The only path is to build.

**Minutes 1 onward. The build path is a developer's path.** `README.md`'s "Try it" says
`script/setup`, which runs `script/bootstrap` and then `cargo xtask build`. Bootstrap installs:

- rustup, then the pinned `nightly-2026-09-17` with `rust-src`, `llvm-tools`, `rustfmt`, `clippy`,
  `miri` and four targets (`rust-toolchain.toml`). That toolchain is **1.9 GB** on disk on this
  machine (`du -sh ~/.rustup/toolchains/nightly-2026-09-17-*`).
- QEMU for three architectures, from Homebrew on macOS, or apt on Linux, where **no Ubuntu release
  ships a QEMU with `riscv-iommu-pci`**, so `script/qemu-check` fails and `script/ci-qemu` builds one;
  a stranger run measured that at twelve minutes (`notes/stranger-test.md`, run 2 and its
  2026-09-13 correction).
- Homebrew's `llvm` for a cross-capable clang, and three contributor tools a person who only wants
  to run the system does not need: `shellcheck`, `cargo-machete` (compiled from source by
  `cargo install`) and `typos`.

Then the build. The cold build time was **not measured** by this lane: two other lanes were gating
on this machine (QEMU for milestones 150 and 168 was running), and AGENTS.md records that timing
assertions fail under oversubscription. The nearest recorded figure is a stranger run's
`script/test` at "about 25 minutes wall clock" on a warm machine (`notes/stranger-test.md`).

**Ten minutes in, a stranger on a cold machine is still installing a toolchain.**

**The first prompt they can reach is in QEMU.** `script/console` boots aarch64 to `$`, and
`script/console --hvf` does it on the Apple Silicon core. That is the only interactive nife a
stranger can reach, on any hardware, today.

## Which hardware a stranger most plausibly has, and what it does today

A judgement, not a measurement: **an x86-64 PC with UEFI firmware, or a Mac.** A Raspberry Pi is
the most common hobby board and nife does not boot on one (`notes/target-hardware.md`'s recast,
2026-07-27, moved first silicon to RISC-V).

| Hardware | Install today | What it reaches | Blocked by |
|---|---|---|---|
| A Mac (Apple Silicon) | Build, then `script/console --hvf` | **The prompt**, interactive | Nothing; this works |
| A Linux PC, QEMU | Build, `script/ci-qemu` for riscv64 | The prompt | The apt QEMU gap above |
| An x86-64 UEFI PC | Build `cargo xtask uefi-image`, copy **one file**, `EFI/BOOT/BOOTX64.EFI` (**10,158,080 bytes** in the main checkout's `target/esp` today), to a FAT32 stick; Secure Boot off (`notes/x86-uefi-boot.md`) | The boot tour on the screen (xenon's first light, 2026-09-05). **Not a prompt** | x86-64 has no interactive boot (milestone 182, PARTIAL); no USB keyboard driver (milestone 242, NOT-STARTED); xenon has no PS/2 port (milestone 192) |
| VisionFive 2 | `script/board-image`: three files that must match (kernel `Image`, `nife-initrd.img` at **8,641,024 bytes**, `boot.scr.uimg`) on FAT32, U-Boot in flash, a serial adapter | The tour over serial; a keystroke at the prompt reached nothing on silicon on 2026-08-15, fixed in code since and not yet confirmed at the bench (`notes/visionfive2.md`) | A bench session |
| Raspberry Pi | None | Nothing | No port |

**The x86-64 row is the nearest thing to a trivial hardware install the tree has**, and it is
close: one file, the firmware's removable-media fallback path, no configuration file, because the
loader carries the kernel and the archive inside itself (`uefi_loader/build.rs`). What stops it is
input, not installation.

## Prior art for the first ten minutes (read)

- **Haiku**: "anyboot" images written straight to USB or run in QEMU, with SHA-256 checksums and
  minisign signatures (`haiku-os.org/get-haiku/`).
- **Redox**: prebuilt images per release and written QEMU recipes with a default login
  (`static.redox-os.org/releases/0.9.0/x86_64/`, `doc.redox-os.org/book/running-vm.html`).
- **SerenityOS**: no downloadable image; build from source and the build launches QEMU
  (`github.com/SerenityOS/serenity`). **This is where nife is today.**
- **Fuchsia**: `ffx product download`, then `ffx emu start` or `ffx target flash`
  (`fuchsia.dev/.../software_assembly/overview`).
- **Raspberry Pi Imager**: choose the device, the OS and the card, "write and verify"
  (`raspberrypi.com/documentation/computers/getting-started.html`).

Haiku, Redox and Fuchsia each ship something a stranger can run without a compiler. SerenityOS
does not, and that is the position nife is in.

## Options for the definition

| | Definition | Reachable in | Why it lost, or did not |
|---|---|---|---|
| **D1. Hardware, from a download** | A stranger writes one image to a stick for a PC and reaches the prompt | Milestones 182 and 242 first; 242 is "months" by milestone 192's own estimate | Right destination, wrong first rung: the blockers are input drivers, not installation, and 198 would wait on them |
| **D2. QEMU, from a download** | A stranger with a Mac or Linux PC and QEMU, and **no Rust toolchain**, downloads one bundle per architecture, runs one command, and reaches `$` | One milestone: every piece exists except the bundle and the launcher | **Recommended as the first rung.** It is Redox's and Fuchsia's shape, and it removes the 1.9 GB toolchain and the contributor tools from the path |
| **D3. From source, faster** | Keep the build path and make it quicker and quieter | Incremental | Loses: it is SerenityOS's position, and principle 3 is about "a person with hardware and no prior knowledge", not a contributor |

**Recommendation: D2 now, D1 as the named second rung**, with the hardware row chosen as the x86-64
UEFI PC because it is the most plausible stranger hardware and already has the one-file layout. The
test for D2 is the stranger harness pointed at running rather than building: a cold container with
QEMU and nothing else, the bundle, and a clock.

**§92 test.** Would D2 be chosen over D1 if both cost the same? No: D1 is the better definition, and
D2 is recommended because it is reachable within a milestone and D1 is not. **This recommendation
is about effort and sequencing, stated as such.** The non-effort argument for D2 is that it is also
the one an outside reviewer (fatal risk 7's adversarial exercise, gated on this milestone by calef's
no-third-parties position; `design/fatal-risks.md`) would use first.

**Reversibility.** A definition is text. **Publishing the bundle is not**: it is the first artifact
this project conveys, which is the moment DECISIONS §135's amendment 1 starts applying and the moment
calef's precondition is spent. So building the bundle is a lane's work, and publishing it is calef's
act, kept separate on purpose.

## The proposed first slice of milestone 198

Sized to be adequate within a milestone or two (principle 1's lesson), and needing **no ruling on
the format, activation or trust forks**, because it conveys no package and installs nothing at
runtime. Every piece is host-side and reversible.

1. **A package, as a host-side recipe.** A directory per package (provisional:
   `packages/<name>/`) declaring how to build it for each triple (an in-tree `[[bin]]` after
   milestone 150, or a fetch-and-build script, which `scripts/build-ripgrep.sh` already is), its
   licence, and its documentation bundle. One producer and one consumer, both in this tree, so the
   recipe shape is reversible. It generalises the two things already doing this by hand:
   `build-ripgrep.sh` and milestone 40's `DOC_BUNDLES` table.
2. **Image composition from a declared package set** (provisional command: `cargo xtask compose`),
   producing the existing sealed kernel-and-archive pair per architecture. This is option C1 of the
   format proposal and T1 of the trust proposal, used as a stopgap rather than chosen as an answer.
3. **A run bundle and a launcher** (provisional: `nife-<arch>.tar` and `run-nife`), holding the
   kernel, the archive, the disk image and a script that needs only QEMU. Built and tested by the
   stranger harness from a cold container; **not published** until calef says so.
4. **Removal and listing for free**: because the set is declared, removing a package is deleting a
   line, which answers milestone 150's "removal has no page" for packages as well as programs.

**What it unblocks.**

- **Milestone 40's shards** get their owner: a package's recipe carries its doc bundle and index
  shard, which is the phrase "installed by the package that owns it" made true at build time.
- **`git` and `nano` for us, built locally**: DECISIONS §135's amendment 1 ("what a build produces on
  a developer's own machine is unconstrained") and amendment 2 ("fetch GPL source, do not vendor it")
  are exactly a recipe that fetches upstream. **This does not run them**: milestone 205 (no argument
  vector) and milestone 169/170's raw-input primitive still stand in front of both, and a lane
  should not read this slice as clearing them.
- **The outside reviewer** fatal risk 7 (the confinement claim is false) wants, once calef chooses to publish the bundle.

**What it does not do**, recorded so nobody mistakes it for the whole milestone: install onto a
running system, let a third party author a package without cloning this repository, or reach a
prompt on hardware.
