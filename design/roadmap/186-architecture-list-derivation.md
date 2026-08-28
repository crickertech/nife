# 186. Derive the architecture list, and close what it does not reach

**Status: NOT-STARTED.** Minted 2026-08-28, on calef's question against the sweep in pull request
#568: is there a milestone for actually fixing these and driving parity, and there was not. The
sweep is `notes/architecture-list-sweep.md` and **it is this milestone's worklist**; this block
says what "done" means and in what order, and does not restate the table.

**Gate: NONE.** No design fork. The recommendation is argued and priced in the note, the tree
already holds every mechanism it proposes, and every part of it is reversible.

## The finding, in one paragraph

`script/stack-frame-check` gated `arches="aarch64 riscv64"` ten days after x86_64 became a
`script/test` target, and its `BUGS` section, which honestly recorded four other limitations, did
not record that one. The sweep asked how many siblings that line has: **eleven silent gaps in nine
files**, beside nine recorded gaps that are working as designed and about fifteen things that are
legitimately one architecture's. Rule 5 and §19 (architectural parity is a tenet) make those
bugs; the other twenty-four are not, and the sweep's value is in that split.

**The pattern is the actual result.** Everything in the tree that stayed complete at three
architectures is either a Rust `match` the compiler pushed on (`kernel/build.rs`'s `panic!` default
arm, `crates/elf`'s `EXPECTED_MACHINE`, `xtask`'s `ArchLegs`) or a per-architecture file whose
absence a build notices. Everything that went stale is a space-separated string in a shell script, a
YAML step, a TOML array, or a sentence in a note. Ten of the eleven silent gaps are in that second
group and the eleventh is a `#[cfg]` pair with no `else`.

## What "done" means

1. Every one of the sweep's eleven silent gaps is either closed or converted into a recorded gap with
   a reason and a trigger, in the shape `script/lint`'s x86_64 block already uses. **A recorded gap
   is a legitimate outcome**, per rule 5, and forcing all eleven to close would be the wrong reading of
   this milestone.
2. The arch-to-triple table has **one** authority that the shell scripts read, rather than the
   four copies it has today.
3. A fourth architecture cannot be added without the tree saying where the lists are. That is what
   items A and B below buy, and it is the half that makes this a milestone rather than a cleanup.

## The order, and why the order is load-bearing

**Each widening is its own commit and its own green run.** This is not tidiness. Newly gating an
architecture runs a check that has never run there, and a check that has never run may have real
offenders. It already does: `script/stack-frame-check --arch x86_64` surfaced
`kernel::arch::x86_64::iommu::init` at **12,504 bytes** against a 4,096-byte guard page, found by
the lane on pull request #567 the same night. A milestone that lands eleven widenings in one commit
turns `main` red for a reason unrelated to whoever pushed, and leaves the next reader unable to tell
which widening did it. Milestone 96's loader commit is the precedent: separateness is the argument.

So the worklist is ordered by **how likely a widening is to surface work**, cheapest first.

### Phase 1: the widenings that cannot surface an offender

These add a target to an install list or a verdict to a linter. Nothing gates code that has not been
gated before, so each is a commit and a green run with no follow-on work possible.

- `script/bootstrap`'s `rustup target add` list.
- `script/toolchain-bump`'s, which is the one that actually bites: it installs against the **new**
  toolchain, where `rust-toolchain.toml`'s `targets` array does not cover for the omission.
- `script/drift`'s, for the same reason.
- `deny.toml`'s `targets` array. Note this one is a **correction**, not an addition: it already
  carried a scope note saying x86_64 bare metal "is declared in §19 but not started; it joins this
  list when it does", and it started. cargo-deny may report new advisories on that graph, which is
  the check working rather than the milestone failing.

### Phase 2: the derivation, which is what stops phase 1 recurring

- **Option A.** One authority for the arch-to-triple table, read by every shell script and every
  `xtask` path. `rust-toolchain.toml` already carries the list, complete at three, and rustup
  already reads it; `script/bootstrap` already parses that same file with a one-line `sed` for
  `channel`. The gap is that nobody extended the parse by one field. About fifteen lines of POSIX
  sh, replacing four hand-written `case "$arch" in` blocks and four bare triple lists.
- **Option B's `compile_error!` arm**, for the `#[cfg]` pairs with no fallback. Three lines at four
  sites (`crates/virtio`, `user/src/gpu_driver.rs`, `user/src/keyboard_driver.rs`,
  `user/src/net_transport.rs`),
  with `entropy_backend`'s backend ladder as the in-tree precedent for ending in `compile_error!`
  rather than a fallback.
- **Option C is refused**, and the refusal is the valuable half. A gate comparing each
  per-architecture file set against the one list would ship with **six exceptions against sixteen
  file-set families**: three legitimately incomplete (the `dtb` fixtures, the port notes, the
  shootdown notes) and three recorded gaps awaiting their own milestones (`targets/*.json`,
  `bench/fastpath-*.txt`, `fastpath_pad.rs`). An exception table more than a third the size of the
  gate it guards is the limitation `script/stack-frame-check`'s own `BUGS` already records about
  itself, in the entry saying its table is maintained by hand and a function that moves keeps its
  exemption with nothing noticing. It would also not have caught the finding that prompted the
  sweep, which was a string inside a script rather than a missing file.

Phase 2 is where the milestone earns its name. Phase 1 fixes 2026-08-28; phase 2 is what fires when
a fourth architecture arrives.

### Phase 3: the widenings that will surface work, each separately scoped

- `script/stack-depth-check`. Not one word: its `TRAP_FRAME`, `DISPATCH` and `BODY` tables each need
  an x86_64 row, measured against that arch's own `size_of::<TrapFrame>()` and symbol names. **Then
  it may fail**, and what it finds is not this milestone's to fix (see below).
- CI's bench job, which today runs two of three legs while `bench/baseline-x86_64.txt` is committed
  and `cargo xtask bench --x86 --check` exists. The tripwire is built and recorded and nothing pulls
  it, so the first run may find real drift.
- `script/bench`'s `EXAMPLES`, which names one architecture of three and does not mention `--riscv`,
  which CI runs on every pull request. Documentation, so it cannot fail a gate, but it is the
  FreeBSD standard's own test and it belongs with the leg it documents.
- `script/fastpath-footprint`, whose `arches` is still `aarch64 riscv64` and which has no x86
  mention at all. Not a one-word widening either: it needs `bench/fastpath-x86_64.txt` and
  `kernel/src/arch/x86_64/fastpath_pad.rs`, neither of which exists, plus an x86_64 row in its
  `ROOTS` table.

**`script/stack-frame-check` is already done and is the model for this phase.** Pull request #567
widened it, ran it, found `iommu::init`'s 12,504-byte frame, and then held x86_64 out of the
*default* set while making it reachable through `--arch`, with a `BUGS` entry recording what was
found and saying that dispositioning the offender is not that gate's call. `main` stayed green by
naming the finding rather than by not looking. Every phase-3 item should end that way, and the
offender it names becomes somebody else's lane.

## What this milestone does not cover

Per this tree's `BUGS` convention, stated where a reader meets the plan rather than discovered
later.

**The offenders that widening surfaces are not this milestone's to fix.** `iommu::init`'s 12,504-byte
frame is a real finding about x86_64 code, not about an architecture list. Closing it is either a
shrink of that function or a deliberate exception with a reason, and either way it is a lane of its
own. This milestone's job ends at "the gate now asks the question on every architecture"; answering
the question is separate work, and conflating the two is how a mechanical milestone acquires an
unbounded tail.

**`notes/arch-audit.md`'s scope gap is not a line item here.** That audit read
`kernel/src/arch/aarch64/` and `kernel/src/arch/riscv64/` in full, about 6,200 lines, and has never
read `kernel/src/arch/x86_64/`, which is 18 files and **6,797 lines**, larger than the two it did
read combined. It is finding 8 of the sweep and it is genuinely an architecture-list gap, but the
work is not widening a list: it is reading 6,797 lines of hand-written assembly and low-level Rust
looking for the bug class that audit defines, in the least-verified code in the trusted computing
base, on an ISA whose secondary bring-up and VT-d driver have no analogue in what was audited. That
does not fit beside a `rustup target add` line.

**Proposed, provisionally, as its own milestone: audit the x86_64 arch tree, on
`notes/arch-audit.md`'s terms.** The number is the integrator's to mint; this block does not claim
one. Two things argue for doing it soon rather than eventually: the audit cadence
(`script/audits`, `.github/workflows/audit-cadence.yml`) counts audits against elapsed time and
shipped components and has **no notion of an architecture**, so a whole unaudited ISA reads to it as
a tree in good standing and nothing will ever raise it; and the note's own argument is that with no
prover for assembly, an audit by reading is the compensating control, which means a third of the
arch tree currently has neither.

**No prior art outside the tree was consulted.** `AGENTS.md` asks for it read rather than recalled,
and the sweep did not leave the repository, so nothing is claimed. If it is worth an hour, the
question is how a multi-target project keeps one target list authoritative across a build system, a
CI matrix, and a package manifest, which is the general form of option A.

## What was already fixed rather than scheduled

`user/src/pgrep.rs`'s panic handler is **not** in this milestone, because it was a live defect
rather than a coverage gap and it landed with the sweep. Neither of its two `cfg` arms matched on
x86_64, so control reached a spin loop and a panic burned a thread forever instead of trapping,
which is the opposite of the signal its own comment promised. It was the last hand-rolled panic
handler left outside `user_rt` after milestone 130 swept forty-eight of them, and the fix was to use
`user_rt::panic_handler!()` like every other program in `user/`.

The distinction is worth keeping: a list that is incomplete is this milestone's, and shipped code
that does the wrong thing on an architecture is not something to schedule.

## Effort

Phase 1 is an hour. Phase 2 is a day, most of it in converting the four `case` blocks and checking
that each caller still behaves. Phase 3 is unbounded on its own terms and bounded by this
milestone's scope note: the widenings are small, and what they find is somebody else's lane.
