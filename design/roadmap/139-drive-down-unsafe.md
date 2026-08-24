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

## Round 2 (2026-08-24): `user_rt`, `ipc`, and the broader `read_volatile`/`write_volatile` sweep

The three concrete next steps round 1 named, taken in the order it suggested.

**`crates/user_rt`'s `SYS_INVOKE` round trip collapsed, a second real §94-shaped reduction.** Six
methods (`recv`, `recv_cap`, `recv_fault`, `call`, `survey`, `list`), each duplicated once per
architecture, had each hand-rolled its own `asm!` block asserting the identical invariant
("`svc`/`ecall` traps to the kernel, which validates before acting") at a register layout that
differed only in which of the five return words the caller happened to read back: twelve
hand-written copies of one assertion. `invoke5` (new, private to the crate, one per architecture)
holds the trap once; every caller above it, including `invoke` itself, is now a safe wrapper with no
`asm!` of its own -- the exact "collapse N hand-written assertions of one invariant into one" shape
this block's own text names as the best available reduction. One honest behavioural note recorded in
`notes/unsafe-obligations.md` and in the code: three of the collapsed functions (`recv`, `recv_cap`,
`recv_fault`) used to leave one input register unset for the kernel to read as whatever value
happened to be there (harmless, since those methods read no input words); routing them through the
shared primitive means they now pass an explicit `0`, a strict tightening rather than a behaviour
change. **Measured from the diff: 14 `unsafe {` blocks removed, 9 added, net -5**, entirely inside
`crates/user_rt/src/lib.rs`.

**`crates/ipc` read in full: no reduction found, and that is the milestone's own predicted outcome
for at least one target.** Production code carries exactly three `unsafe` blocks, one each inside
`send`, `recv` and `remove_sender`, and each already asserts a genuinely different fact (which of
two queues, which node, under what caller contract) rather than the same fact copied three times --
there is no §94 shape to collapse here. The other 41 sites the crate's `unsafe {` count includes
(baseline "44 blocks") are doc examples, `#[cfg(kani)]` proof harnesses and `#[cfg(test)]` unit
tests, each deliberately exercising a distinct state-machine transition; collapsing those would
either be impossible (a test suite's whole value is that each call is a different scenario) or would
be exactly the "hides it behind a macro" anti-pattern this block refuses. The crate's own doc
comments already do the milestone's other job for comments rather than code: the shared proof
obligations are stated once, at the module and section level ("stated once here rather than
re-derived at each of the eleven/twenty-odd sites"), with each call site's own comment adding only
what is particular to it. Nothing to migrate; reported honestly rather than forcing a relocation to
move a number.

**The broader `read_volatile`/`write_volatile` sweep round 1's own BUGS section asked for, run for
real this time.** Grepping directly for `read_volatile`/`write_volatile` across `user/src/` (rather
than by the `r8`/`w8`/`r16` naming convention round 1 searched by name) surfaced roughly thirty
files; most turned out not to be the pattern (device register blocks with their own poll loops,
framebuffer/graphics code, or programs whose entire point is deliberately invalid or one-off memory
access -- see "What is still open" below for the accounting). One cluster was a clean, large match:
eight programs (`rm`, `fs_file_caretaker`, `sink`, `fs_subtree_caretaker`, `fs_nameset_caretaker`,
`login_test_client`, `fs_test_client`, `swish`) each hand-rolled a `put_page`/`get_page`-shaped
byte-copy loop over the page shared with the FS server, every one asserting "this VA is a mapped
page of this size" by hand in near-identical wording (`fs_nameset_caretaker` carries a second,
read-only window for its name set; `fs_test_client` carries five such helpers over one window sized
to `fs::TRANSFER_MAX` rather than one page). Migrated onto the **existing**
`user_rt::mapped_window::MappedWindow` (round 1's type, reused rather than duplicated, per this
block's own instruction). **21 `unsafe {` blocks removed, 10 added, net -11** across the nine files.
`fs_subtree_caretaker.rs` alone is flat (1 block before and after: one hand-rolled function traded
for one window construction), the same "still real by criterion 2" case `smb_server.rs` was in round
1: raw pointer arithmetic replaced by a typed, bounds-checked abstraction, even though the file's own
count does not move.

**Combined round 2: 35 `unsafe {` blocks removed, 19 added, net -16.** Measured from the diff against
this round's own base commit (`a269403e`), the same discipline round 1 used and for the same reason:
a tree-wide census is contaminated by unrelated concurrent growth. This round's paired measurement
happens to be uncontaminated regardless -- nothing else landed on this branch between the base commit
and this reduction, so the tree-wide census confirms the diff exactly: 792 blocks outside `arch/` at
the base commit, 776 after, precisely -16. Density (the ceiling's actual unit) moved only 90 to 89
truncated, because this reduction also removed lines (duplicated `asm!` blocks and SAFETY comments
along with the blocks themselves), so the denominator moved with the numerator for the first time
this ceiling has had to account for.

**The ratchet, cinched again**: `<!--count-at-most:unsafe-density-outside-arch-->` lowered from 97 to
96 in the same commit (`notes/unsafe-obligations.md`, `notes/counted-claims.md`,
`notes/register-of-measures.md`), keeping the same 7-point headroom the 100-vs-93 and 97-vs-90
ceilings both carried, now above the 89 this round reached.

## What is still open

**`crates/ipc`'s unsafe is settled**: read in full, genuinely per-call-site distinct, no further work
indicated there (see round 2 above).

**The broader `user/` survey this milestone's BUGS section calls for is still not complete**, and is
now a better-informed job than it was after round 1: the `read_volatile`/`write_volatile` grep
(rather than the round-1 name-based search) is the right net, and round 2's pass through its results
sorted the non-FS hits into rough categories a follow-on lane can use rather than re-deriving:

- **Device register blocks with their own poll/wait loops**: `console.rs` (UART FR/DR), `input.rs`
  (a PL011/NS16550 pair behind `rd`/`wr` helpers -- yet another naming variant on the same
  read/write-a-fixed-offset shape, confirming the point a broader net exists to make), `driver.rs`
  (a `NonNull<u8>`-based UART driver, a different idiom already), `clock.rs` (RTC registers),
  `jh7110_trng.rs` (TRNG registers). These read a small, hardware-defined register set rather than a
  data buffer; whether `MappedWindow`'s bounds check is the right fit for a register block (as
  opposed to a page of caller data) or whether these want their own idiom is an open question, not a
  settled "yes, migrate."
- **Framebuffer/graphics code**: `display.rs`, `painter.rs`, `window.rs`, `compositor.rs`,
  `display_terminal.rs`. Likely several distinct, dynamically-many surfaces rather than one static
  page per program, and pixel-level access at real volume (thousands of writes per frame), so a
  bounds check per pixel write is a real performance question this lane did not measure -- flagged
  rather than migrated blind, per this project's "elegance and performance beat implementation
  convenience" tenet, which cuts the other way when the convenient answer is also the slow one.
- **`swish.rs`'s other two windows**: `OUT_VA`/`LINE_VA` (`stage`/`read_line`, talking to the
  terminal) and `jf_load`/`jf_store` (the job frame, parametrized by a runtime `va` rather than a
  fixed constant -- actually a *better* `MappedWindow` fit than the static case, since `new` already
  takes a runtime base). Left alone only because this round already touched `swish.rs` once (its
  `FS_VA` cluster) and a shell is worth changing minimally per sitting.
- **`heeder.rs` is done** (migrated this round, see above); nothing else in that shape remains
  outstanding there.
- **`disk_surveyor.rs`'s `ROSTER_VA`**: a single shared `u64` flag at a fixed VA, not a byte-copy
  loop, so lower value (one invariant asserted twice, not N times), but the same underlying pattern.
  Not migrated; small enough for whoever picks up this list next.
- **`net_stack.rs`'s `a_r8`/`a_r16`/`a_w16`/`a_w8`** (4 unsafe blocks, 14 call sites): the exact
  `a_r8`/`a_w8` naming variant `mapped_window.rs`'s own doc comment already names as one of the
  shapes round 1 collapsed -- except this file was not one of round 1's seven and is still
  unmigrated. Harder than the FS cluster: the VA is not a fixed constant but `socket_va(sid) =
  0x00A0_0000 + sid * 0x1000`, a different page per open socket, and callers pass an already-offset
  absolute VA rather than a (window, offset) pair, so migrating cleanly means restructuring call
  sites to separate the per-socket base from the field offset, not just swapping the four functions'
  bodies. Real candidate, deliberately not attempted this round given the size of what else this
  round already touched.
- **Deliberately not migration candidates, named so nobody re-derives them and wastes a look**:
  `hello.rs` (tests `.bss` zeroing and `.data` writability on purpose; the raw access *is* the test),
  `flaky.rs` and `outlaw.rs` (deliberately touch a bad/unauthorized address to provoke a fault; a
  bounds-checked wrapper would defeat the point), `budgeter.rs` and `swapper.rs` (single one-off
  writes, not a repeated hand-written invariant -- nothing to collapse).
- **`login_test_client.rs`'s `PAGE_VA`** uses `core::slice::from_raw_parts_mut` rather than
  `read_volatile`/`write_volatile`, so the grep this round ran legitimately did not catch it; it is a
  different pattern (an ordinary, non-volatile slice reference into shared memory) and out of this
  sweep's scope by construction, not by oversight.

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
