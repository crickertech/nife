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

## Round 3 (2026-08-24): `swish`'s remaining windows, `disk_surveyor`'s flag, `net_stack`'s cluster,
and two investigations that ended in a recommendation rather than a migration

Round 2's own handoff list named five items precisely enough that this round did not have to
re-derive a net; it took the three it named as clear migrations and investigated the two it named as
open questions.

**`swish.rs`'s remaining two windows.** `OUT_VA`/`LINE_VA` (`stage`/`read_line`, the shell's terminal
pages) and the job frame `spawn_interruptible`/`watch` signal through (`jf_load`/`jf_store`,
parametrized by a runtime `va`, "actually a *better* `MappedWindow` fit than the FS cluster, since
`new` already takes a runtime base" in round 2's own words). The terminal pair is flat by block count
(2 removed, 2 added at the two `const` window declarations) and still a real reduction by criterion 2
-- the "typed abstraction replaces raw pointer arithmetic" case `smb_server.rs` and
`fs_subtree_caretaker.rs` were in rounds 1 and 2. The job frame collapses for real: `jf_load`/
`jf_store` were two functions with their own `// SAFETY:` comments, called eight times combined
across `spawn_interruptible` and `watch`; one `MappedWindow`, constructed once right after the frame
is mapped, replaced both. **4 `unsafe {` blocks removed, 3 added, net -1**, in `user/src/swish.rs`
alone.

**`disk_surveyor.rs`'s `ROSTER_VA`.** A single shared `u64` flag at a fixed VA the program maps
itself at runtime (`Frame::MAP`, not a boot-time wiring), read once in `ROLE_HOLDER`, read again
after the kernel deliberately revokes the mapping (the module's own negative control: the second
read must fault), and written once in `ROLE_PROBE` (refused by the kernel; the mapping is read-only).
The two deliberate-fault sites are the one honest exception recorded where a reader meets it:
`MappedWindow`'s bounds check cannot catch either fault, because offset 0 is inside the declared
window both times, so the real hardware fault happens inside `read`/`write` at exactly the access the
hand-written version made, and the test's behaviour is unchanged by the migration. **3 `unsafe {`
blocks removed, 2 added, net -1**, in `user/src/disk_surveyor.rs` alone.

**`net_stack.rs`'s `a_r8`/`a_r16`/`a_w16`/`a_w8` cluster**, the exact naming variant
`user_rt::mapped_window`'s own doc comment already named as a shape round 1's search should have
caught and did not. Genuinely harder than the FS cluster, as round 2 flagged: the VA is not a fixed
constant but `socket_va(sid) = 0x00A0_0000 + sid * 0x1000`, a different page per open socket, and
every caller computed an absolute VA (`sk.va + OFF_X`) rather than holding a `(window, offset)` pair.
Migrating cleanly meant restructuring the socket-lifecycle state itself, not just swapping the four
functions' bodies for wrapper calls that still took a raw absolute VA (which would not have moved the
invariant at all, per this milestone's own instruction): `Sock.va: u64` (0 meaning "no frame") became
`Sock.window: Option<MappedWindow>` (`None` meaning the same thing), and the parallel `frame_va:
[u64; MAX_SOCKETS]` array became `frame_window: [Option<MappedWindow>; MAX_SOCKETS]`, constructed
once in `OP_ATTACH_FRAME` right after the kernel maps the frame -- the one place in the whole socket
lifecycle that needs to assert the invariant, instead of every one of the four functions' bodies.
Every downstream call site (`read_dst`, `udp_sendto`, `sock_recv`, `tcp_connect`, `tcp_accept`,
`udp_bind`, `tcp_send`) now takes or holds a `MappedWindow` rather than a raw VA, so the
restructuring reaches the caller side. One further site collapsed for the same reason though it was
never named `a_w8`: `sock_recv`'s payload-write loop had its own hand-rolled `write_volatile`,
identical in shape, folded into the same window. **5 `unsafe {` blocks removed (the four functions'
bodies plus the hand-rolled loop), 1 added (the window construction in `OP_ATTACH_FRAME`), net -4**,
in `user/src/net_stack.rs` alone. `script/test`'s aarch64 and riscv64 net suites (DHCP, UDP, TCP
connect/accept/listen, the mDNS responder) passed clean, which is the load-bearing evidence here: the
restructuring touches per-socket lifecycle state, exactly the kind of change where a mistake shows up
as a flaky network test rather than a compile error.

**Combined round 3: 12 `unsafe {` blocks removed, 6 added, net -6.** Measured from the diff against
this round's own base commit (`f731894d`), uncontaminated: nothing else landed on this branch between
the base commit and this reduction, so the tree-wide census confirms it exactly: 776 blocks outside
`arch/` at the base commit (89 per 10,000, matching round 2's own final reading), 770 after, exactly
-6. Density moved 89 to 88 (truncated); the line count moved by only 15, mostly the comments
explaining the new windows, so the denominator barely moved this round, unlike round 2's
`asm!`-collapse.

**The ratchet, cinched a third time**: `<!--count-at-most:unsafe-density-outside-arch-->` lowered
from 96 to 95 in the same commit (`notes/unsafe-obligations.md`, `notes/counted-claims.md`,
`notes/register-of-measures.md`), keeping the same 7-point headroom every ceiling in this milestone
has carried, now above the 88 this round reached.

**Investigation: device-register blocks (`console.rs`, `input.rs`, `driver.rs`, `clock.rs`,
`jh7110_trng.rs`) -- genuinely per-driver distinct, and `MappedWindow` is the wrong fit; a stronger
idiom already lives in this tree and is unused by these five files.** Read in full. `driver.rs` is
already a different idiom (a raw `*const u8` with `.add(offset)`, not a named accessor pair) and
needs nothing. `clock.rs`'s two RTC drivers (`pl031_unix_nanos`, `goldfish_unix_nanos`) are each a
single one-shot function with its own `// SAFETY:` comment, called once; there is no duplication
inside the file to collapse. `console.rs`, `input.rs` and `jh7110_trng.rs` each already collapse
their own register access to one or two functions (`uart_put`; `rd`/`wr`; `r32`/`w32`) with one
`// SAFETY:` comment apiece, reused at every call site -- the §94 shape is already applied *within*
each file. What is not collapsed is *across* files: the wording of each file's comment rhymes
("this VA is our device mapping, handed to us at spawn, for the whole lifetime of this process"),
which looks like the same invariant copied, but reading closely it is not: each asserts a genuinely
different fact (a different VA, a different device, a different offset table), the same shape
`crates/ipc`'s three call sites turned out to have in round 2 ("which of two queues, which node,
under what caller contract... there is no §94 shape to collapse here"). A shared `RegisterBlock`
type wrapping these would relocate the assertion from "a local `rd`/`wr` function" to "a shared
type's constructor," not collapse it, which is this milestone's own named anti-pattern.

But the investigation did not end at "nothing to do." **The tree already has the right idiom for
this shape, in the kernel, for the very same two devices.** `kernel/src/drivers/pl011.rs` and
`kernel/src/drivers/ns16550.rs` drive the identical PL011/NS16550 hardware these five userspace
programs also drive, using `tock-registers`' `register_structs!`/`register_bitfields!` macros: one
`unsafe` block for the whole driver (the base-pointer construction), and every register offset
checked **at compile time** rather than asserted by hand, which is a stronger property than
`MappedWindow`'s runtime bounds check gives (`kernel/src/drivers/pl011.rs`'s own comment: "an
off-by-four here is a build error rather than a mystery at runtime"). This is exactly the "what does
this tree already do in the analogous case" answer AGENTS.md's fork-readiness section asks for, and
it settles the "MappedWindow or a new RegisterBlock type" question this round's brief posed: neither.
`tock-registers` is already a tree dependency (§46's "thin architectural primitive" category), and
migrating `console.rs`/`input.rs`/`jh7110_trng.rs` onto it would be a real reduction with a stronger
property than either alternative -- but it is new scope this round did not take, for two reasons
worth naming rather than a guess at effort: adding `tock-registers` to the `user` crate's dependency
graph is a dependency decision (rule 6, `DECISIONS.md` §46) this round did not have standing to make
unilaterally, and the three files it would touch gate boot output and keyboard input, which is
exactly the kind of blast radius this round's brief said to weigh before attempting a risky
restructuring. Left as a named follow-on: migrate `console.rs`, `input.rs` and `jh7110_trng.rs` onto
`tock_registers::register_structs!`, matching `kernel/src/drivers/pl011.rs`'s and
`kernel/src/drivers/ns16550.rs`'s own shape; `clock.rs` and `driver.rs` need nothing, per above.

**Investigation: framebuffer/graphics code (`display.rs`, `painter.rs`, `window.rs`, `compositor.rs`,
`display_terminal.rs`) -- one decisive structural finding, and the performance question narrowed but
not settled.** Read in full. **The part of this pipeline that would actually run at real per-frame
volume carries no per-pixel unsafe at all, migrated or not.** `compositor.rs`'s `paint`/`serve_frame`
call `compositor::composite(screen(), &srcs[..n], n, damage)`, the crate's host-tested pure logic,
over ordinary safe `&mut [u32]`/`&[u32]` slices obtained by exactly **one** `unsafe` call per frame
(`screen()`, `source(i)`), not one per pixel: the compositor's hot loop already is the "assert the
invariant once, then use safe indexing" idiom `MappedWindow` generalizes, just spelled as a slice
rather than as that type. This is the decisive part of round 2's "is a bounds check per pixel write a
real performance question" framing, because the answer is: not here, since there is no per-pixel
check (bounds or otherwise) on this path today.

What remains is genuinely per-pixel `unsafe`: `painter.rs`'s and `window.rs`'s `px_write`/`px_read`
(a client painting and then digesting its own surface, `graphics_proto::PIXELS` = 8,192 accesses per
run for `painter.rs`, up to 2,048 for the largest window in `compositor::SCENE`), `display.rs`'s
`surface_pixel` (the driver's own post-flush digest, also 8,192 accesses per run, sharing its
`dma_read`/`dma_write` pair with a few dozen one-off virtqueue-field writes that are not the
performance question), and `display_terminal.rs`'s `paint`, which is the one file among the five
where the question is live rather than one-shot: it repaints a damaged rectangle on every terminal
update, driven by keystrokes rather than a fixed frame rate, so its volume is bounded by typing speed
and paste size rather than by a boot-time test, unlike the other four. None of these is the
"thousands of writes per frame, sixty times a second" sustained path round 2's handoff named; that
path turned out not to exist in this file set. But "bounded and one-shot" or "bounded by typing
speed" is a structural characterization, not a measurement: this round did not obtain an actual
instruction-count or cycle number for a `MappedWindow`-checked pixel write versus the current raw
one, because no such micro-benchmark exists yet and building one was out of this round's reasonable
scope alongside the three migrations above. **Left as a narrowed follow-on** rather than a migration
on a guess: the question a next lane needs to answer is no longer "does a bounds check survive 60fps"
(it does not need to, because nothing here runs at 60fps) but "does a bounds check cost enough at
2,048-8,192 accesses per one-shot run, or per keystroke-driven repaint, to matter" -- a much smaller
question, worth `script/icount` or `script/bench` rather than reasoning about it further.

## Round 4 (2026-08-24): measuring `MappedWindow`'s bounds-check cost, then migrating the
framebuffer/graphics cluster round 3 narrowed to

Round 3's own conclusion set this round's exact scope: `painter.rs`'s and `window.rs`'s
`px_write`/`px_read`, `display.rs`'s `surface_pixel`, and `display_terminal.rs`'s `paint`, all
bounded (2,048-8,192 accesses per one-shot test run, or a keystroke-driven repaint far smaller than
that), none of them the sustained 60fps path the milestone's original framing worried about. The
open question was no longer structural, it was a number: does `MappedWindow`'s bounds check cost
enough at these volumes to matter, "worth `script/icount` or `script/bench` rather than reasoning
about it further" in round 3's own words. This round got that number, then migrated on it.

**The measurement.** A temporary comparison loop over a page-sized buffer, one raw `write_volatile`
against one loop performing `MappedWindow::check`'s own arithmetic first
(`off.checked_add(size).is_some_and(|end| end <= len)`), at three volumes: 56 (one glyph cell,
`bitmap_font::GLYPH_W * GLYPH_H`, `display_terminal.rs`'s smallest keystroke-driven repaint), 2,048
(`window.rs`'s largest `compositor::SCENE` surface) and 8,192 (`graphics_proto::PIXELS`, the
one-shot digest volume `painter.rs`, `display.rs` and `window.rs`'s own largest surface all share).
Run via `script/bench` (icount, deterministic instruction counts, the right instrument for a
path-length question rather than a magnitude one; notes/benchmarks.md's own table names icount for
exactly this job), both ISAs:

| | aarch64 (ticks/access) | riscv64 (ticks/access) |
|---|---|---|
| raw `write_volatile` | 8 | ~1.4 |
| `MappedWindow`-checked | 12 | ~2.0 |
| delta | 4 | ~0.6 |

Flat across all three volumes on both ISAs (the check's cost does not depend on how many accesses
follow it, only on the arithmetic itself), so the total overhead scales linearly and stays small in
absolute terms even at the largest bounded volume: 8,192 accesses costs ~29,000 extra aarch64 ticks,
under 30 `ipc_rtt` round trips (1,017 ticks each) -- inside a test that already pays several real
round trips (at least one `CALL` to a driver or the compositor) and, for `display.rs`, a real device
DMA completion at ~200 us wall clock (`fs_read`'s own reading of what a completion-interrupt wait
costs). **Negligible on both ISAs, settling round 3's open question**: the bounds check is not worth
avoiding at any volume this cluster actually sees.

The comparison itself was not kept in the tree. It answered a one-time design question rather than
measuring an ongoing primitive worth regression-gating forever (the same reasoning `bench.rs`'s
`map_new` shootdown probe uses for staying a `bench-probe:` line rather than a baseline row), and
keeping it would have cost this milestone's own ceiling two more `unsafe {` blocks (one per
comparison loop) for a question that is now closed -- working directly against the number this round
exists to lower. The numbers above are the reproducible record instead.

**The migration, on that number.** All four sites round 3 named, migrated onto `MappedWindow`:

- `painter.rs`'s `px_write`/`px_read`: a `const WINDOW` at `SURFACE_VA`, sized to
  `gfx::SURFACE_BYTES`, the same shape round 1's cluster used. **2 `unsafe {` blocks removed, 1
  added, net -1.**
- `window.rs`'s `px_write`/`px_read`: **not** a `const`, unlike `painter.rs`'s twin. The compositor's
  `spawn_client` maps a different frame count per client (`SCENE[i].frames()`,
  `kernel/src/user/compositor_service.rs`), not knowable at this program's compile time, so the
  window is constructed once in `_start`, sized to `bytes` (this client's own `w * h * 4`, already
  computed and bound-checked against `compositor::MAX_SURFACE_BYTES` before any pixel is painted --
  milestone 43, notes/shared-page-audit.md finding 4), and threaded through both call sites
  (`px_write`, and `px_read` via a closure into `compositor::surface_checksum`). The genuinely
  per-client, runtime-sized case `net_stack.rs`'s socket cluster was in round 3. **2 removed, 1
  added, net -1.**
- `display.rs`'s `dma_write`/`dma_read`: round 3 scoped this to `surface_pixel` alone, but
  `surface_pixel` is one of dozens of callers of a shared pair of functions that already collapse the
  DMA region's invariant into one hand-written assertion apiece; migrating only `surface_pixel`'s own
  call would have **duplicated** that invariant (a second `MappedWindow` beside the untouched
  `dma_write`/`dma_read`) rather than collapsed it, the wrong direction for this milestone. Migrating
  the two shared functions instead -- a `const WINDOW` over the whole DMA region
  (`DMA_FRAMES * 4096` bytes), with `dma_write`/`dma_read`'s bodies becoming `WINDOW.write`/
  `WINDOW.read` -- covers `surface_pixel` for free and bounds-checks every one of the few dozen
  virtqueue-field one-off writes in the file too, which round 3's own text flagged as "not the
  performance question and could be migrated independently": here it came along for free rather than
  as separate scope. **2 removed, 1 added, net -1.**
- `display_terminal.rs`'s `paint`: a window constructed once in `_start`, sized to
  `gfx::SURFACE_BYTES` in `MODE_DISPLAY` (this process maps that many bytes itself) or to
  `stride * h` in `MODE_WINDOW` (the compositor's own published geometry, the same per-client
  reasoning as `window.rs`'s), held in the `Wiring` struct rather than declared as a file-level
  `const` for the same reason `window.rs`'s is not one. **1 removed, 1 added, net 0** -- flat by
  block count, still a real reduction by criterion 2 (raw pointer arithmetic replaced by a typed,
  bounds-checked abstraction), the same case `swish.rs`'s terminal pair and `smb_server.rs` were.

**Combined round 4: 7 `unsafe {` blocks removed, 4 added, net -3.** Measured from the diff against
this round's own base commit (`757562a3`), uncontaminated: nothing else landed on this branch between
the base commit and this reduction, so the tree-wide census confirms it exactly: 782 blocks outside
`arch/` at the base commit (88 per 10,000, matching round 3's own final reading despite 12 blocks of
unrelated tree growth landing in between, 770 to 782 -- density absorbed it the same way round 1 and
round 2's readings did), 779 after, exactly -3. Density moved 88 to 87 (truncated);
the line count moved by +19 net (mostly the new `SAFETY` comments explaining each window's
invariant), so the denominator barely moved this round, like round 3's.

**The ratchet, cinched a fourth time**: `<!--count-at-most:unsafe-density-outside-arch-->` lowered
from 95 to 94 in the same commit (`notes/unsafe-obligations.md`, `notes/counted-claims.md`,
`notes/register-of-measures.md`), keeping the same 7-point headroom every ceiling in this milestone
has carried, now above the 87 this round reached.

**The framebuffer/graphics investigation is now settled, not just narrowed.** `compositor.rs`'s
per-frame hot path still carries no per-pixel `unsafe` at all (round 3's finding, unchanged); the
five genuinely per-pixel sites round 2 and round 3 identified are now four migrations and one
structural non-issue (`compositor.rs` itself needed nothing). Nothing about this cluster remains
open.

## What is still open

**`crates/ipc`'s unsafe is settled**: read in full, genuinely per-call-site distinct, no further work
indicated there (see round 2 above).

**The broader `user/` survey this milestone's BUGS section calls for is still not complete**, and is
now a better-informed job than it was after round 1: the `read_volatile`/`write_volatile` grep
(rather than the round-1 name-based search) is the right net, and round 2's pass through its results
sorted the non-FS hits into rough categories a follow-on lane can use rather than re-deriving:

- **`swish.rs`'s other two windows, `disk_surveyor.rs`'s `ROSTER_VA`, and `net_stack.rs`'s
  `a_r8`/`a_r16`/`a_w16`/`a_w8` cluster are all done** (round 3, see above). Nothing else in these
  three files' shape remains outstanding.
- **Framebuffer/graphics code is done** (round 4, see above): `painter.rs`, `window.rs`,
  `display.rs` and `display_terminal.rs` are all migrated, and the measured bounds-check cost that
  round 3 left open (negligible at every volume this cluster sees, on both ISAs) is recorded there.
  Nothing else in this cluster remains outstanding.
- **`heeder.rs` is done** (round 2); nothing else in that shape remains outstanding there.
- **Device register blocks, investigated rather than migrated (round 3)**: `console.rs`, `input.rs`,
  `clock.rs` and `jh7110_trng.rs` are genuinely per-driver distinct at the invariant level, so
  `MappedWindow` is the wrong fit; `driver.rs` is already a different idiom and needs nothing. The
  real follow-on is concrete: migrate `console.rs`, `input.rs` and `jh7110_trng.rs` onto
  `tock_registers::register_structs!`/`register_bitfields!`, matching the idiom
  `kernel/src/drivers/pl011.rs` and `kernel/src/drivers/ns16550.rs` already use for the identical
  hardware, which checks every register offset at compile time rather than by hand-written comment
  (a stronger property than either `MappedWindow`'s runtime check or the status quo). Out of this
  round's scope because it is a new dependency for the `user` crate (a decision, not a lane's call)
  touching files that gate boot output and keyboard input. See the investigation above for the full
  reasoning and the `ipc`-round-2 precedent it rests on.
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
