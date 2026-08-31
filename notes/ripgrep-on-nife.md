# `ripgrep` on nife: what a stranger's program actually hits

*(Milestone 121, and `design/fatal-risks.md` risk 1's decisive experiment. Run 2026-08-31 on
aarch64, QEMU virt, against `ripgrep` 14.1.1 from crates.io.)*

Risk 1 says the platform *"can run hand-written Rust and nothing else, so every piece of software
anyone wants has to be rewritten."* It is the most dangerous of the nine because no amount of kernel
work fixes "nothing runs here". `ripgrep` was chosen as the falsifier because it is not a toy: forty
transitive crates, a filesystem walk, gitignore semantics, memory maps, and threads.

## The answer, in one paragraph

**Risk 1 is not realised, and the thing standing in the way is not what anybody expected.**
Unmodified `ripgrep` compiles for `aarch64-unknown-nife` with **zero source changes**, links, loads,
runs, resolves its own working directory through a granted directory capability, and exits cleanly
through `std::process::exit`. It never reaches `std::thread::spawn`, so DECISIONS §105 is not what
blocks it. What blocks it is that **the nife ABI has no argument vector**: `std::env::args()` yields
an empty iterator, `ripgrep` parses no arguments, and it prints its own diagnostic and stops.

```
test kernel::user::ripgrep_tests::unmodified_ripgrep_runs_and_has_no_arguments_to_run_on ...
    rg printed 62 bytes:
rg: ripgrep requires at least one pattern to execute a search
ok
```

That line is `ripgrep`'s, word for word, from `crates/core/main.rs`. Somebody else's application
reached its own error path on this kernel.

## The four questions, answered

### 1. Does it build?

**Yes, with no patch, no vendored copy and no fork.** `scripts/build-ripgrep.sh` downloads the
published crate and builds it. Everything that differs from a Linux build is on the command line:

| What | Why |
|---|---|
| `--target targets/aarch64-unknown-nife.json` | the custom target spec, `panic=abort`, `singlethread` |
| `-Zbuild-std=core,alloc,std,panic_abort` against `nife-dev` | the patched `std` PAL (notes/std.md) |
| `-Clink-arg=-T…link.ld -Clink-arg=-u_start -Clink-arg=--build-id=none` | what `std_exerciser/build.rs` supplies in-tree |
| `-Copt-level=s -Cstrip=debuginfo` | ripgrep's own release profile sets `debug = 1`, and a 25 MB ELF rides into RAM in the initrd |

Build time from cold: about 24 seconds. Every crate in the tree compiled, including the four that
milestone 64 flagged as interesting (`ignore`, `crossbeam-deque`, `memmap2`, `walkdir`). Two crates
resolve to non-Unix implementations without being asked to: `memmap2` compiles its `stub.rs`, and
`std::sys::args` compiles `unsupported.rs`. Neither is an error; both are the reason the link
succeeds.

### 2. Does it run, and on what subset?

**It runs.** `kernel/src/user/ripgrep_tests.rs` spawns it exactly as milestone 27's `std` demo is
spawned, with a heap untyped at slot 0, an output endpoint at slot 1, and the FS service's directory
capability at slot 4. What that proves, layer by layer, and none of it written for `ripgrep`:

- the ELF loader maps a **4.7 MB, three-segment** program;
- `std`'s allocator grows a heap one page at a time out of an untyped budget, under `regex`'s and
  `ignore`'s allocation patterns rather than a demo's;
- `std::env::current_dir()` answers `/`, the root of this process's own namespace, **because it holds
  a directory**. Without slot 4 the same binary prints
  `rg: failed to get current working directory: operation not supported on this platform`, which is
  the capability model felt from inside a stranger's program;
- output reaches the one endpoint it was granted;
- `main` ends in `std::process::exit` and the process leaves without a fault (the assertion on
  `USER_FAULTS` is milestone 64's fourth-pass reasoning, reused).

**The subset it cannot reach is everything after argument parsing**, which is all of the searching.
See §4.

### 3. Where does it hit DECISIONS §105?

**It does not, and this is the most useful negative result here.** The expectation in `fatal-risks.md`
was that `ripgrep` *"uses threads, so it runs straight into the one thing this project has decided not
to build"*, and that a red result would be §105 Option A arriving with evidence.

That is not what happened, for a reason worth generalising:

- `ripgrep` does not assume parallelism, it **asks for it**.
  `crates/core/flags/hiargs.rs:172` computes its default thread count as
  `std::thread::available_parallelism().map_or(1, |n| n.get()).min(12)`.
- nife's PAL answers **honestly**: `patches/std-nife/overlay/std/src/sys/thread/nife.rs` returns
  `Ok(1)`, with the comment *"the process model is one thread today, and that is an answer, not an
  error."*
- `ripgrep` therefore selects its own single-threaded paths (`hiargs.rs:632`, `search_serial` and
  `WalkBuilder::build` rather than `build_parallel`). `thread::spawn` is never called, and `-j1` is
  not needed because 1 is already the answer.

`crossbeam-deque`, `crossbeam-epoch` and `crossbeam-utils` all **compile and link**; they are simply
not entered. So §105's decline costs nothing here, and the PAL returning a truthful `1` rather than
an error is what buys that. **A platform that answered `Unsupported` to `available_parallelism`
would have failed this program**, which is an argument for the honest-answer posture generally.

The `--threads 1` caveat milestone 121's block requires for any published benchmark still stands, and
so does the `--no-mmap` one: `memmap2`'s stub means the memory map is unavailable rather than slow,
and `grep-searcher` falls back to reads on its own.

### 4. What does enumeration-as-a-capability mean here?

The milestone's own subject, and the honest answer is that **this experiment did not get far enough
to price the walk**, because it never got a directory to walk. What is now known:

- **The primitives a walker needs exist.** Milestone 122 landed `std::fs::Dir` and multi-component
  descent, and `std_exerciser`'s pinned transcript already asserts `read_dir descend ok`,
  `walk entry ok` (list a subdirectory, then open every file the listing named through the `path()`
  the listing handed back), `dir handle ok` and `remove_dir_all ok`. That last one is std's own
  generic recursion, written in terms of `read_dir` and paths it composes itself: a recursive walk
  by a stranger's code, working. So `walkdir` and `ignore` have what they are written against.
- **`ENUMERATE` is a right and its absence is `EPERM`, not an empty listing** (§47), which is the
  property that makes a confined `rg` meaningful rather than decorative. Untested here for the same
  reason.
- **The per-component IPC cost is still unmeasured**, which milestone 121's block names as the half
  worth the lane. Nothing in this run priced it.

## What actually stands between here and `rg pattern src/`

Three things, in the order they bite. All three are platform gaps rather than `ripgrep` gaps.

### A. There is no argument vector, and nothing to substitute for one

`std::env::args()` compiles `sys/args/unsupported.rs` and yields **nothing at all**, not even
`argv[0]`. notes/abi.md is explicit: *"There is no libc, no `argv`/`envp` array, no dynamic loader,
no `main` wrapper"*; a program is entered with three registers and a capability table.

The native answer is `grant_plan`: the shell parses the line, resolves it against the program's
`Manifest`, and sends init **a program id, one integer, and a page count**, plus capabilities. That
is a rich and deliberate design, and it is the reason `swish` can grant exactly the file a command
named. It is also, for a foreign program, no design at all: `ripgrep` wants a regex.

The env-var escape hatch is closed too, and it is worth naming because it looks open. `ripgrep`
reads `RIPGREP_CONFIG_PATH` and takes its arguments from the file it names, which would have been a
complete answer using only concepts nife already has. But `environment_proto` is a **closed
three-key page**: `TZ`, `LANG` and `TERM`, each validated against a curated domain list. There is no
way to hand a std program an arbitrary environment variable, by design (DECISIONS §111).

**This is the finding to act on**, and it is a wire-format decision rather than a lane's: what a
process may be told at startup, in bytes rather than capabilities. It is calef's under *move fast on
what can be undone*, because every future program is written against it.

### B. A program image has under 896 KiB before it hits its own stack

`user/link.ld` links every program at `0x40_0000`. `kernel/src/user.rs` puts every program's stack at
`USER_STACK_VA = 0x50_0000`, and a `std` program maps 32 more pages below that, so the first mapping
above a program's image is at `0x4E_0000`. **The ceiling is 0xE_0000, or 896 KiB.**

`ripgrep`'s `.text` alone is 1.37 MiB, and its whole image spans `0x40_0000..0x69C_000`. The loader
refuses it:

```
refused to load a user program: Unmappable(AlreadyMapped)
```

`scripts/build-ripgrep.sh` works around it by relinking at `0x100_0000`, derived from `user/link.ld`
by substitution so the two cannot drift. **That is a workaround and should not survive**: a stranger
compiling a program for this platform has no way to know the ceiling exists, the failure names an
overlap rather than a size, and 896 KiB is small for anything with a dependency tree.

The address is **not** a private kernel detail, which is why this lane did not simply change it. It
is written into `crates/supervision_proto` (`CHILD_STACK_VA`), `crates/timebase_proto`,
`crates/c_seam`, `user/src/builder.rs`, `user/src/os_primitives_benchmarker.rs`, and half a dozen
kernel tests. Moving `USER_STACK_VA` alone breaks `authority_tests` immediately (measured: the
supervision tree fails to build at stage 10, because the programs building it map their children's
stacks at the old address by hand). So it is an ABI-shaped change and belongs to calef.

### C. `mmap` is absent, and that one is already recorded

`memmap2` compiles its stub, so `grep-searcher` gets an error rather than a mapping and reads
instead. Milestone 99's block and milestone 121's own `BUGS` already name this; nothing new was
learned, and the cost is still unmeasured because no search ran.

## How to reproduce

```
scripts/build-ripgrep.sh      # fetches ripgrep 14.1.1 from crates.io, builds for aarch64-unknown-nife
script/test                   # kernel::user::ripgrep_tests now runs instead of skipping
```

**Nothing in the ordinary build does this, on purpose.** Making `script/test` fetch `ripgrep` and its
forty transitive crates would put a crates.io dependency tree in this repository's build, which
DECISIONS §46 makes calef's decision. So `xtask initrd-aarch64` packs `rg` only when the ELF is
already on disk, and the test skips with a reason when the archive has none, which is every ordinary
build and all of CI.

The test reclaims the program's 256-page heap region when it is done
(`fs_service::start_std_full`, `sched::reclaim_region`). That is not tidiness: a permanent charge
would make `kernel::testing`'s frame ledger fail for exactly the person running the experiment and
pass for everyone else.

## BUGS

- **Nothing here measured a search, so nothing here measured the walk.** Every performance claim
  milestone 121 wants (per-entry IPC cost separated from per-byte search cost, against
  `rg --threads 1 --no-mmap` on Linux) is still unmade. Gap A is what blocks it.
- **The confinement demonstration is unbuilt.** The milestone's load-bearing negative half, `rg`
  against a directory capability lacking `ENUMERATE` being refused loudly rather than returning
  nothing, needs the same argument vector.
- **The relink to `0x100_0000` is invisible to anyone who does not read the build script.** It is
  recorded here and in the script, and nowhere a stranger would meet it, which is rung four of
  AGENTS.md's ladder. Gap B is the fix.
- **One version, one architecture, one run.** `ripgrep` 14.1.1 on aarch64. The RISC-V leg was not
  built, and no other version was tried.
- **`ripgrep` never allocated much**, because it stopped before searching. The 256-page heap was
  sized from `std_exerciser` and is untested against a real workload; a search may want far more, and
  what a std program does when its untyped budget is exhausted is not exercised here.
- **The build script fetches from the network** and pins a version but checks no hash. It is an
  experiment's apparatus and is not on any trust path (the initrd's measurement table digests
  whatever it packs), but it is not a supply-chain-safe way to obtain software.
