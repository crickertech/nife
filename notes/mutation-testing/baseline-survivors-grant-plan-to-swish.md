# The baseline survivors, crate by crate: `grant_plan` to `swish`

An appendix of [notes/mutation-testing.md](../mutation-testing.md), moved there verbatim on 2026-09-24.

- **grant_plan** (26): 25 real. The tokenizer and parser were only ever fed lines whose cursor
  arithmetic coincided at small indices (`i + 1` and `i * 1` agree at 1), so buffer ceilings, a
  trailing `--mem`, and the flag-cluster bound could all rot into indexing past the line; the
  recursion bit was only tested on `rm`, whose `r` is bit 0, where shift direction cannot matter;
  `prog_id`'s round trip used id 1, the constant the mutant substitutes, and `from_id`'s arm list
  had drifted by one program. Refusal sentences and Debug renderings pinned exactly. Equivalent:
  `RECURSIVE`'s `1 << 0`. Deferred finding, integrator's call: `NameSet`'s Debug renders numeric
  byte lists, not the legible names its own header promises; pinned as-is because the fix changes
  rendered output.

- **isa** (22): 12 real, and the shape is the wrong-accept this crate exists to prevent. Both
  riscv64 fixtures declared their narrowest hart first, so the widest-wins fold never replaced an
  answer it already held: rv32 after rv64 could keep rv64 (and boot on the rv32 machine), and a
  base-less extensions node could clobber the base under one flipped `&&`. A wide-first fixture
  exercises every replacement arm; `Missing::any` gets one single-missing case per clause;
  ASIDBits `0b0000` must decode to the 8 bits `crates/address_space_identifier` assumes. Equivalent (10): idempotent
  assignments at `<=` boundaries, disjoint-operand `|` vs `^`, `1 << 0`, and six compile-time
  duplicate-check loops whose guards are vacuous on a table that already passes, a pattern the
  weekly report will keep resurfacing (noted here so nobody re-triages it).

- **measured_boot** (9): one real (uppercase hex was dead in the suite; every test round-tripped
  through our own lowercase `to_hex`). The other eight are textbook equivalent mutants worth
  naming because they will resurface weekly: SHA-256's `ch` has operands masked disjoint by `e`
  and `!e` (`|` equals `^`), `maj` computed with OR is the same majority function as with XOR
  (they agree wherever at least two inputs agree), `update`'s boundary take is the same number
  down both branches at `len == want`, and the hex nibble combine ORs into cleared low bits.

- **paging** (39): **seventeen real, and not one of them a bug**, which is the result to want from
  page-table math. Eleven are accessors and predicates nothing inside the crate reads back:
  `Flags::bits`, `is_user_page_va` (whose `&&` to `||` mutant shorts the user-VA gate into an
  or-gate that admits an aligned *kernel* address to a user `MAP`), `PageFormat::half_base`, and
  `Mapper::root`, whose value is what the kernel writes into TTBR0/satp, so a constant there
  installs the wrong table in silicon while every walk test still passes. Four more are descriptor
  bits the portable `Flags` do not carry, so the encode/decode round-trips are structurally blind
  to them: aarch64's `TABLE_OR_PAGE` (bit 1, which `is_present` never reads, so a descriptor that
  loses it walks perfectly in software and is a translation fault at L3 and a *block* descriptor
  above it), aarch64's inner-shareable `SH` on normal memory (the `delete !` mutant hands it to
  device memory instead, leaving normal pages with a core-private view another core's write never
  invalidates), and Sv39's `D`. The last two are in the DMA domain, where every existing test
  granted exactly one page: the region shape that cannot tell a page count from the constant `1`,
  nor `grant_page`'s whole-page test from its opposite, both killed by a grant of two pages and a
  half. Equivalent: twenty-two, in two families. `1 << 0` to `1 >> 0` (`CAP_WRITE`, aarch64
  `VALID`, Sv39 `V`) and `0b00 << 6` (`AP_RW_EL1`), the degenerate shifts named above; and every
  `|` to `^` in the `Flags` constructors and in `table_entry`/`leaf_entry` on both formats, because
  the six `CAP_*` are distinct single bits and a descriptor's address mask (aarch64 bits 47:12,
  Sv39's PPN at 53:10) shares no bit with its attributes or with valid/type at bits 1:0.
- **pci** (31: 27 missed, 4 timeouts): nineteen real, and they share one cause, which is that the
  fake config space held exactly one device. Enumeration's multifunction handling was never
  exercised (nine mutants on the single line that reads the header type), the per-function "nobody
  home" check was never reached because no device had a function 1, and the fixture's 64-bit BAR
  was both unassigned and in the last slot pair, so a base that dropped its high half and a walk
  that ran one slot past BAR5 both read as correct. Killed by
  `enumeration_follows_the_multifunction_bit_and_skips_what_does_not_answer`,
  `an_empty_slot_costs_one_config_read` (the vendor-id guard changes no output at all, only read
  count, so a count is its only witness: 45 reads on a 32-slot bus against 306 without it),
  `a_64_bit_bar_in_the_first_slot_keeps_its_high_half`,
  `the_command_bits_are_the_specified_positions`, and
  `a_function_without_a_capability_list_is_not_walked`. Equivalent, seven, all one argument: the
  `|` to `^` flips in `ecam_offset`, `requester_id`, and both halves of `read_bars`'s 64-bit
  assembly are ORs of **disjoint** bit ranges (`bus:8 | dev:5 | fn:3` packs without overlap for
  every BDF a caller can produce, which is what the Kani proof assumes), and `size`'s
  `mask | 0xffff_ffff_0000_0000` is the same case against a constant. All four timeouts are
  genuine detected hangs on the BAR cursor, and a fifth mutant joined them: `i += 2` to `i *= 2`
  differs from the original only at slot 0, so the new first-slot fixture turned a silent survivor
  into a hang.
- **line_editor** (48): 45 real, and they cluster by what the terminal model hid. `Screen` asserts
  what the user sees, which is the right contract, but a test that checks only the finished line
  never proves *where the cursor is*, so the whole movement layer was mutable at will: ^E and ^F
  could be deleted, `right` could be emptied to `()` or have its `<` flipped four ways, `left`'s
  `> 0` could become `>= 0`. `control_key_movement` and `right_arrow_moves_and_stops_at_end` type a
  character after every move and close all nine. The history ring's index arithmetic
  (`(hist_next + HIST - k) % HIST`, eight mutants across `repeats_newest` and `hist_next_entry`)
  survived because no test crossed the wrap point, where a `+` for a `-` and a `%` for a `/` still
  land on a plausible entry; `wrapped_ring_walks_correctly_both_ways` walks ten commands through a
  ring of eight in both directions, `duplicate_entry_is_stored_once` pins the dedup by bell count
  (the screen cannot tell one stored copy from two, which is why `repeats_newest -> false` survived
  everything else), and `empty_lines_stay_out_of_history` covers the `len > 0` half of the same
  guard. The repaint arithmetic (`len - cur` in `start_line`, in ^L, in `yank`, in the stash
  restore) needed a cursor left off the end and *then* a keystroke, since the echo alone redraws
  the same glyphs either way. The rest are one test each: the CSI `;` arm and its `n += 1` (one of
  them a `-=` that underflows to a panic), ^W's two scan loops (whose `>` to `>=` reads `buf[a - 1]`
  off the front), the three-way split at `kill_len > 0`, and `csi_move`'s digit loop, which nothing
  had ever driven past nine columns. Equivalent (3): `req`'s `|` to `^` (opcode in bits 63:56,
  length in 31:0, disjoint), `FLAG_EOF`'s `1 << 0`, and `csi_move`'s `1` arm, which only elides a
  count: delete it and `n == 1` emits `CSI 1 D`, which ECMA-48 defines as the same motion as
  `CSI D`. That last is byte economy on a serial line, not behaviour, and the check that it is not
  an excuse is its sibling one line up: deleting the `0` arm makes a zero-column move travel a
  column, and `backspace_erases_on_screen` kills it.
- **video_terminal** (79, the largest single block in the run): **64 real, 15 equivalent, none
  deferred**, and the shape splits three ways. The geometry and colour accessors were never called
  with a *number*, only compared against another value computed the same way, so `cols()`,
  `width()`, `height()`, `colours()` and `to_pixels` could return a constant or swap an operator
  unnoticed. A 6 by 3 grid separates `6*8` from `6+8` from `6/8`; three pixels of a glyph at cell
  (1,1) separate a divide from a remainder, which agree at (0,0), where every existing pixel
  assertion sat; and a union with a rect *inside* the first is the only shape that reads the first
  operand's far edges. The parser's less-travelled arms were individually deletable because nothing
  fed them: a tab, a bare control code, a string terminated by `ESC \`, `CSI 1J`, SGR 27, 39, 49
  and 90-97. The switches that turn something *off* are the ones worth naming, because a terminal
  that only ever sets attributes passes every test while leaving a line reversed forever. Third,
  clamps and damage were asserted only where they did not bind: `\x1b[99B` and `\x1b[99;99H` now
  pin the clamps, since one too generous parks the cursor off the grid where every later write is
  silently dropped, and a bare `LF` on the bottom row with the cursor hidden is what makes
  `damage_all` non-deletable. Equivalent (15): disjoint-mask `|` versus `^` at three sites,
  `DEFAULT_BG << 4` on a zero operand, the four min/max selections in `union` at equality, SGR
  40-47's `p - 40` versus `p + 40` (40 is a multiple of 8 and the background is masked to three
  bits, so they agree for all eight legal parameters), `erase_display`'s `to > from` (true for
  every mode), two match arms that fall through to a byte-identical body, and three range guards no
  caller can reach because `col < cols` and `row < rows` are invariants of every write. Those last
  three are the ones most worth leaving visible: a new caller that broke either invariant turns all
  three back into real bugs. **The correction worth recording**: `csi`'s `>` to `==` at the
  parameter limit was argued equivalent by hand and is not. `== MAX_PARAMS` fires on the fourth
  separator, so it drops a legal `CSI 1;2;3;4 m` entirely, and only running the mutation caught it.
- **slots** (5): four real, all in the half of the API the Kani harnesses never touch. `get_mut`
  could return `None` for a live name with nothing on the host noticing, while the kernel
  `unwrap()`s it on the switch path, so the mutant is a kernel panic dressed as a lookup miss;
  `is_empty` could be stuck at `true`, stuck at `false`, or inverted, because nothing in the tree
  calls it at all, which is exactly the accessor that rots unobserved. Killed by
  `get_mut_reaches_the_live_entry_and_writes_through_it` and
  `a_table_is_empty_only_while_it_holds_nothing`. Equivalent: `name`'s `|` to `^`, where the
  generation is shifted into bits 63:32 and the slot is `< N <= u32::MAX` by the const assert in
  `new`, so the two operands never share a bit.
- **socket_protocol** (2): one real. `DATA_MAX` is `4096 - OFF_PAYLOAD` and the test only asked
  whether a full payload *fits* the frame, which `4096 / OFF_PAYLOAD` also does, so the constant
  could shrink by 3576 bytes unnoticed; it is now pinned as the whole page after the header, which
  refuses both a payload that overruns the grant and one that leaves granted bytes unreachable.
  Equivalent: `req`'s `|` to `^`, where the opcode is a byte and the socket id is shifted past it,
  and the crate's own `every_opcode_fits_in_its_byte` is what keeps the two ranges disjoint.
- **sink_proto** (2): both equivalent, no gaps. `req` ORs an opcode shifted to bits 63:56 into a
  length masked to bits 31:0. And `pack`'s `bytes.len() < INLINE_MAX` to `<=` is the boundary where
  both arms return the same number: at exactly sixteen bytes `bytes.len()` *is* `INLINE_MAX`, so no
  slice length distinguishes them.
- **user_mode_heap** (6): three real, all in the split arithmetic, and the reason nothing saw them is
  that a block's *size* is never readable. `free_bytes` is an independent counter a wrong split does
  not touch, and every test asked only for the block **count**, which the coalescing invariant
  expects to be 1. So `block_count` could return the constant 1 and pass everything; `alloc`'s
  `tail > 0` could become `>=`, writing a zero-length free node one node past the donation; and
  `bsize - front` could become `bsize + front`, inflating the remainder by twice the front padding
  and leaving the heap willing to hand out memory nobody gave it. Killed by
  `a_block_that_fits_exactly_leaves_nothing_behind` and
  `a_split_never_invents_bytes_nobody_donated` (the first donates its arena away from the process
  heap on purpose, so a mutant writing past the region lands in slack rather than in the test
  runner's own allocations). The 3 timeouts are detected hangs of the same shape: a wrong `front`
  makes `insert_free` write a node whose `next` points at itself. Two of the three fail an assertion
  before anything hangs and are recorded as timeouts only because the thrashing test spins in the
  same binary, which is the honest caveat on reading a timeout as "not caught".
- **swish** (2 real, 1 equivalent, 9 timeouts): the shell's own sentences, where a regression is
  user-visible and nothing else looks. `echo`'s `i > space` could become `>=`, emitting a
  zero-length whitespace run; in the program `out` is the terminal endpoint, so an empty write is a
  round trip carrying no bytes. `write_pwd` could be replaced with nothing at all, because no test
  called it, so a `pwd` that printed an empty line would have shipped. Equivalent: deleting
  `write_refusal`'s bare `Refusal::NoSuchProgram` arm, because the guarded arm above takes every
  non-empty program name and the `_` arm it falls into prints a prefix only when `Prog::from_name`
  resolves, which an empty name never does. The 9 timeouts are all detected hangs in `echo`'s
  two-cursor scan.
