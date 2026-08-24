# 139. Drive the unsafe count down, and cinch the ratchet behind it

**Status: PARTIAL.** Minted 2026-08-18 by calef, immediately after folding the unsafe census into
milestone 134: *"Can we also create a milestone to drive down the unsafe metrics and cinch up the
ratchet?"*

**Gate: NONE.** Milestone 134's instrument (the census and the ceiling relation in `script/lint`)
has existed and been live all along; this milestone spent it for its first real reduction below.

## What was built

**The `MappedWindow` cluster, the milestone's first real reduction.** Seven userspace programs
(`entropy`, `kbd`, `net_transport`, `mdns_responder`, `socket_test_client`, `smb_server`, `ntp`)
each hand-rolled the same `r8`/`w8`/`r16`/`w16`/`r32` volatile-access functions over a DMA page or
a shared IPC frame, one hand-written `// SAFETY:` comment per function asserting the same
invariant ("this offset is inside the page the kernel mapped here") by hand at every call site --
the exact §94 shape this milestone's own text names as the best available reduction.
`user_rt::mapped_window::MappedWindow` (new) holds that invariant once, at construction, and turns
every access into a bounds-checked call: a wrong offset used to be a silent out-of-bounds volatile
access and is now a panic naming the access, which is a real soundness improvement the hand-written
copies never had, not just a relocation.

**Measured precisely from the diff** (a before/after tree census is contaminated by unrelated
concurrent growth -- five days between this milestone's baseline and this reduction added 30
unrelated `unsafe` blocks elsewhere in the tree at roughly the tree's own rate): **32 `unsafe {`
blocks removed across the seven programs, 11 added** (9 window constructions -- one per program
except `smb_server`, which needs two: one for its boot-wired FS channel sized to
`fs::TRANSFER_MAX`, one for its runtime-mapped socket frame -- plus the 2 generic `read`/`write`
methods inside `MappedWindow` itself). **Net -21.** `smb_server.rs` alone is flat (11 blocks before
and after), still a real reduction by this milestone's own criterion 2 (raw pointer arithmetic
replaced by a typed, bounds-checked abstraction) even though it does not move that file's own
count.

**Density**: 93.4 (799 blocks over 85,476 lines) immediately before this reduction, 90.8 (778 over
85,526) after -- essentially the density the milestone's own baseline recorded on 2026-08-18 (93.0),
despite five days of unrelated tree growth in between, which is what makes density rather than raw
count the number worth trusting. Full measurement history in `notes/unsafe-obligations.md`'s table.

**The ratchet, cinched**: `<!--count-at-most:unsafe-density-outside-arch-->` lowered from 100 to 97
in the same commit (`notes/unsafe-obligations.md`, `notes/counted-claims.md`,
`notes/register-of-measures.md`), 7 points of headroom above the 90.8 this reduction reached -- the
same absolute headroom the original 100-vs-93 ceiling carried, not a zero-headroom ceiling that
would fail the next legitimate unsafe block anywhere outside `arch/`. The reasoning for why this
measurement gets headroom (unlike `unsafe-thread-safety-claims`' zero-headroom ceiling) is recorded
in `notes/unsafe-obligations.md` beside the marker.

## What is still open

**`user/`'s remaining unsafe is still largely unread.** This lane read and fixed one real cluster
(the shared-page volatile-access pattern); it did not do the broader survey this milestone's own
BUGS section calls for ("nobody has read enough of it to say whether that number is raw shared-page
handling, a missing safe wrapper, or something else"). That survey is still the next lane's cheapest
useful output.

**Other candidate clusters, named for a follow-on lane rather than re-derived from scratch:**

- **`crates/user_rt` itself** (31 unsafe blocks per the 2026-08-18 baseline, not re-measured here):
  the runtime crate every program links against is exactly where a shared, checked abstraction pays
  for itself most, and `MappedWindow` is now a precedent living there for whatever's found.
- **`ipc`** (44 blocks at the same baseline): the rendezvous state machine crate; unread by this
  lane, worth checking whether its unsafe is genuinely per-call-site distinct or another §94 shape.
- **Any other DMA-page or shared-frame driver this lane did not touch.** The seven programs migrated
  were found by searching for the exact `r8`/`w8`/`r16`/`w16`/`r32` naming convention; a driver using
  different accessor names but the same underlying pattern (raw offset into a fixed-VA page, no
  bounds check) would not have been caught by that search and is worth a second pass with a broader
  net (e.g. grep for `read_volatile`/`write_volatile` directly rather than by function name).

**This block still sets no target number**, per its own original text -- the ratchet moves by
measured reduction, not by picking a floor in advance.

**In brief.** Reduce the hand-written `unsafe` this tree carries outside `kernel/src/arch/`, and lower
the ceiling after each reduction so the ground gained cannot be given back quietly.

## The measurement it starts from

Taken on `main` 2026-08-18 and to be re-taken by 134 rather than trusted here:

| | count |
|---|---|
| `unsafe { }` blocks, ours, `vendor/` excluded | 893 |
| `unsafe fn` | 53 |
| `unsafe impl` | 28 |
| `// SAFETY:` comments | 885 |

By location: `kernel/src/arch/` 139, kernel outside `arch/` 203, `user/` 285, `ipc` 44, `user_heap`
41, `user_rt` 31.

**`arch/` is not a target and this block will not accept a reduction there.** Rule 1 says
architecture-specific code lives under `arch/`, and unsafe is what architecture is made of. Driving
that number down means either writing the assembly wrong or moving it somewhere it does not belong,
and both are worse than the number.

## What counts as a reduction, which is the whole design of this block

**The metric is gameable and the obvious way to game it is invisible.** Moving three `unsafe` blocks
into one helper function reduces the count by two and reduces the risk by nothing: the same
invariants are asserted by the same code in a different place. A milestone that rewarded that would
make the tree worse while the graph went the right way, and the graph would be the reason.

**So the test is not the token count, it is the number of distinct invariants asserted by hand.**

**§94 is the worked example and it is what a real reduction looks like.** The trap instruction was
inlined at **48 sites in 7 variants** across 58 panic handlers, each one a hand-written assertion of
the same invariant. Lifting it into `user_rt::trap()` left one. That is 47 chances to write it
differently, removed. §94 states the general form: *a per-binary item whose body is copied verbatim
into every binary is not per-binary; only its declaration is,* and copying it is asserting the same
invariant N times by hand, which §61 says a `// SAFETY:` comment must never be.

**And §94 found the cost of the copy by counting the copies**: one of the 58 handlers was different.
`terminal_sink_caretaker` called `exit()` and never trapped, so a panic there reported `EVENT_EXIT`
where every other program reports `EVENT_FAULT`. **A supervisor would have been told a panicking
program finished cleanly.** That is the argument for this milestone in one incident: the duplication
was not only ugly, it was already wrong in one place and nobody knew.

So a reduction qualifies when it does one of these:

- **Collapses N hand-written assertions of one invariant into one**, the §94 shape. Best available,
  and the only one that reliably reduces risk rather than moving it.
- **Replaces raw pointer arithmetic with a typed abstraction** whose invariant the compiler or Kani
  holds, so the assertion stops being a comment. Rung one on the ladder.
- **Deletes unsafe that was never needed**, which is the cheapest and rarest.

It does not qualify when it merely relocates unsafe, wraps it in a function whose safety argument is
the same argument, or hides it behind a macro.

## The ratchet

After each reduction, **lower the ceiling to the new count in the same commit.** A ceiling left above
the true number is not a ratchet, it is a budget, and a budget gets spent by whoever arrives next
without knowing it was won.

## Why it matters

**The verified-Rust claim in this project's own thesis is measured here.** DECISIONS §14 calls this a
verified-Rust capability microkernel; `unsafe` is where that claim is suspended, and 203 suspensions
outside the architecture layer is the honest size of the gap. Every one of them is a place where the
proofs and the type system are standing aside and a person's comment is the whole argument.

## BUGS

- **This block sets no target number and should not, until 134 measures whether the ceiling fires on
  honest work.** `script/lint` has already had three checks deleted for the signature "only ever
  rejects legitimate work", and a ceiling cinched past what the tree can sustain would be the fourth.
  The first lane should report what a realistic floor looks like rather than picking one here.
- **`user/`'s 285 is unexplained.** A userspace program in a capability system arguably needs little
  unsafe, and nobody has read enough of it to say whether that number is raw shared-page handling, a
  missing safe wrapper, or something else. That reading is the first lane's cheapest useful output
  and it may change what this milestone does.
- **A reduction can be real and still not show in the count.** Proving an existing `unsafe` block's
  invariant with Kani leaves the block where it is and makes it safer; the metric cannot see that.
  Do not let the number decide which work is worth doing.
