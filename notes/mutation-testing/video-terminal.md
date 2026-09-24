# `video_terminal`

An appendix of [notes/mutation-testing.md](../mutation-testing.md), moved there verbatim on 2026-09-24.

## 2026-09-20: `video_terminal`, milestone 326 (turn a mutation score upward) part 3

`video_terminal` was the largest untriaged survivor set in the tree, at **377 viable mutants, 79
survivors, 79.0% caught** in the 2026-09-19 census. Same discipline as the sections above: every
number below is re-derived with `script/mutation -p video_terminal` on this lane's own worktree
rather than trusted from the census, every kill was verified by re-running the sweep and watching
the mutant die, and every equivalence claim is a mutant the second run still reports.

**Before: 298 caught, 79 missed, 0 timeouts, 15 unviable (79.0% of viable). After: 361 caught, 16
missed, 0 timeouts, 15 unviable (95.8%).** This run's `before` column matches the census row for
row exactly, which this crate's own scope explains: it has no Kani harnesses and no loom model (no
`proofs::`, `verification::`, or `interleavings::` module anywhere in it), so it carries none of the
"never the crate's code" artifact the earlier sections warn about, and `cargo mutants -p
video_terminal --list` confirms the mutant set is genuinely `lib.rs`, `keymap.rs`, and `script.rs`.

| file | survivors | killed by a test | equivalent | recorded gap |
|---|---|---|---|---|
| `lib.rs` | 79 | 63 | 16 | 0 |
| `keymap.rs` | 0 | 0 | 0 | 0 |
| `script.rs` | 0 | 0 | 0 | 0 |
| **total** | **79** | **63** | **16** | **0** |

`keymap.rs` and `script.rs` carried no survivors at all; every mutant in either file was already
caught before this lane touched anything. All 79 survivors are in `lib.rs`, and no exclusion and no
recorded gap was needed: every one closed as a killed test or a demonstrated equivalence.

**Five tests were added, each closing coverage this crate never had before it, for 63 survivors.**

**Multi-byte UTF-8 decoding had never been tested at all (18 survivors, 17 closed here; the
eighteenth is equivalent, below).** Milestone 142 (a text display good enough that people use it
instead of a GUI) increment 2 added it, and every mutant in
`Vt::ground`'s continuation-byte branch, its shift-and-accumulate arithmetic, and all three
lead-byte masks (two-, three-, and four-byte sequences) survived because nothing in the suite had
ever fed a non-ASCII byte to `feed`. `utf8_decodes_every_sequence_length_and_recovers_from_a_bad_
one` feeds one character of each length (`é`, `日`, `🎉`), a truncated sequence (a lead byte
followed by a non-continuation byte), and a lone continuation byte that can never start one,
checking the exact `char` landed rather than just that something non-blank did. On inspection the
decoder itself is correct RFC 3629 (the lead-byte ranges, the overlong-form exclusion, the
replacement-character fallback all match); this was a coverage gap, not a defect.

**Scrollback (milestone 142 increment 2, a text display good enough that people use it instead of a
GUI) had no test at all: not `scroll_up`, not `scroll_down`, not `view_offset`, not
`scrollback_len`, and nothing scrolled far enough to read history back (42 survivors across
[`SCROLLBACK_CELLS`] itself, `Vt::cell`'s history branch, `scrollback_cell`,
`push_scrollback_row`, the two getters, and both scroll methods; 41
closed here, and the forty-second, `push_scrollback_row`'s source index, is equivalent, below).**
`scrollback_survives_the_rings_wrap_and_reads_back_in_order` feeds 350 lines through a two-row grid,
349 scrolls, comfortably past [`SCROLLBACK_ROWS`]'s 300-row cap: the ring wraps and its oldest 49
pushes are overwritten, which is the shape needed to prove the ring's modular arithmetic rather than
only its first lap. Six checkpoints (`view_offset` 1, 2, 50, 150, 299, 300) each check **both**
display rows: row 0 alone cannot distinguish `Vt::cell`'s `age = view_offset - 1 - row` from a
mutant's `+ row`, because at `row == 0` the two terms are the same value; row 1 is what makes the
sign matter, and at `view_offset == 1` row 1 comes from the *live* half of `Vt::cell` instead
(`live_row = row - view_offset`), which is what closes that arithmetic's own mutant.
[`SCROLLBACK_CELLS`] closes as a side effect of the same test: that constant sizes the `scrollback`
array, and a mutant computing it as `MAX_COLS + SCROLLBACK_ROWS` (432 cells) rather than `MAX_COLS *
SCROLLBACK_ROWS` (39,600) panics on an out-of-bounds index partway through the very first feed loop,
well before any assertion runs; a panicking test is a caught mutant, not a special case.

**`reset_to` had never been called (1 survivor, the whole function body).** Every other test builds
a `Vt` at its final size with `Vt::new` or never resizes, so the function that exists specifically
for a **runtime** geometry (`components/src/display_terminal.rs`'s own bring-up, per its own doc
comment) was completely unexercised. `reset_to_clears_everything_and_retargets_the_geometry` feeds
content that scrolls something into history, retargets to a different size, and checks the geometry,
the cursor, every cell, the damage rectangle, and that the old grid's scrollback did not survive the
retarget (`sb_len` clearing is `reset_to`'s own responsibility, separate from `scroll_down`'s).

**The size clamp had never been tested at its own boundary (2 of 4 survivors; the other 2 are
equivalent, below).** `clamp_cols`/`clamp_rows` had no test at `cols == MAX_COLS` or `cols ==
MAX_COLS + 1`, so a mutant relaxing `cols > MAX_COLS` to `cols == MAX_COLS` (which clamps only the
exact boundary value and lets anything past it through unclamped) survived.
`geometry_clamps_exactly_at_the_boundary_not_one_off_it` checks the boundary itself, one past it,
and zero, in both dimensions.

**`put` and `damage_cell`'s defensive guard had never been exercised from either side (2
survivors).** `if col >= self.cols || row >= self.rows` relaxed to `&&` only refuses a coordinate
where **both** axes are out of range; no caller in this crate ever calls either method with a
partly-out-of-range coordinate (`print` always writes at the cursor, which is always in bounds;
`erase_line` bounds its column with `.min(self.cols)` before calling `put`), so the guard was dead
as far as any public-API test could tell. `put_and_damage_cell_reject_a_partly_out_of_range_
coordinate` calls both private methods directly (the same `use super::*` access every test in this
module already has) with one axis in range and the other not, on both axes, and checks the private
`cells` field and `damage()` are both untouched.

**Sixteen are equivalent, and every one is argued from the code rather than asserted.**

- **`Attr::DEFAULT`, 2 mutants (`DEFAULT_FG | (DEFAULT_BG << 4)`).** `DEFAULT_BG` is `0`
  (`pub const DEFAULT_BG: u8 = 0;`). Shifting `0` left or right by any amount is `0` either way, and
  OR-ing or XOR-ing `7` with `0` is `7` either way, so both the `|`-to-`^` and `<<`-to-`>>` mutants
  compute the same constant no test could ever separate them on.
- **`CellRect::union`'s four min/max selections, 4 mutants**
  (`if self.col < o.col { self.col } else { o.col }` and its three siblings for `row`, `right`,
  `bottom`). A selector of this shape (`if a < b { a } else { b }`, or the `>` form for a max) gives
  the *same value* on both branches whenever `a == b`, because at that point `a` and `b` are the
  same number; `<` vs `<=` (or `>` vs `>=`) only disagree about which branch fires at that one point,
  and the branch that fires no longer matters once the two branches would return the same thing. So
  no input, not just no input a test happened to try, can distinguish the strict comparison from the
  non-strict one here.
- **The `>=` half of the size clamp, 2 mutants** (`cols > MAX_COLS as u32` in `clamp_cols`, and the
  same shape in `clamp_rows`). The same selector argument as `union`, applied to `if cols > MAX_COLS
  { MAX_COLS } else { cols }`: at `cols == MAX_COLS`, the "else" branch returns `cols`, which **is**
  `MAX_COLS`, so both branches already agree at the one point `>` and `>=` disagree about which of
  them to take. (This is the other half of the clamp's 4 survivors; the `==` half above is a real
  gap, not this one, because `==` only clamps the single boundary value and lets everything past it
  through, which is a different, genuine behaviour change.)
- **`Vt::pixel`'s `col < self.cols` in the cursor-overlay check, 1 mutant.** This term is reached
  only after `col == self.col` already matched in the same `&&` chain, and `self.col` is never `>=
  self.cols` (every cursor-moving path clamps with `.min(self.cols - 1)`/`.min(self.rows - 1)`, and
  `print`'s deferred wrap never advances `col` past `cols - 1` either; see [`Vt::erase_display`]'s
  own doc comment for the fuller statement of this invariant). So whenever `col == self.col` holds,
  `col < self.cols` already holds too, and relaxing it to `<=` cannot change which branch is taken.
- **`Vt::ground`'s continuation-byte accumulator, 1 of the 18 UTF-8-shaped mutants**
  (`(self.utf8_code << 6) | (b & 0x3f) as u32`). `self.utf8_code << 6` always has its low six bits
  zero (a left shift by six fills them with zero), and `b & 0x3f` is exactly six bits, so the two
  operands never share a set bit. OR and XOR agree whenever operands share no set bits, on every
  input, not because of anything a caller does.
- **`Vt::push_scrollback_row`'s source index, 1 mutant** (`row * cols` read as `row / cols`). This
  is a private method with exactly one call site (`Vt::line_feed`), which always passes the literal
  `0`: the row about to scroll off the top is always the grid's first row, because scrolling is what
  makes room at the *bottom*. `0 * cols` and `0 / cols` are both `0`, on the only input this function
  is ever given.
- **`Vt::csi`'s private-use/intermediate-byte arm, 1 mutant** (deleting `0x20..=0x2f | 0x3c..=0x3f
  => self.ignore = true`). Those two byte ranges are not matched by any other arm in the same
  `match` (`b'0'..=b'9'`, `b';'`, `0x40..=0x7e`), so deleting the arm routes them to the catch-all
  `_ => { self.ignore = true; }`, which does the identical thing. The two arms stay written
  separately because a reader needs to see the two families of swallowed byte (a sequence this
  engine chose not to implement, versus a stray control code) named apart, even though the code that
  runs is the same either way.
- **`Vt::erase_display`'s damage guard, 1 mutant** (`if to > from`). `to > from` holds on every
  reachable input: mode 0's `end - here = (rows - row) * cols - col >= cols - col >= 1` (since `row
  < rows` and `col < cols`, both invariants this type maintains everywhere); mode 1's `here + 1 >=
  1`; the default's `end = rows * cols >= 1` (since `cols >= 1` and `rows >= 1`, `Vt::clamp_cols`/
  `Vt::clamp_rows`'s own floor). So `to == from` is unreachable, and relaxing `>` to `>=` cannot
  change which branch fires.
- **`Vt::sgr`'s bit-independent recolouring, 3 mutants.** `(p as u8 - 30) | bright` (the 16-colour
  foreground SGR codes, 30–37) and `(p as u8 - 90) | 8` (the bright foreground codes, 90–97): both
  left operands occupy bits 0–2 only (`p - 30` and `p - 90` each range `0..=7`), `bright` and the
  literal `8` occupy bit 3 only, so OR and XOR agree on every input, the same shape as the UTF-8
  accumulator above. `p as u8 - 40` (the background codes, 40–47) read as `p as u8 + 40`: `Attr::new`
  masks its `bg` parameter with `& 0x07` before storing it, and `(p - 40)` and `(p + 40)` differ by
  exactly `80`, which is a multiple of `8`, so the two are congruent mod 8 and the masked result is
  identical for every `p` in `40..=47`.

**No defect in shipped behaviour was found.** Every survivor here was a missing test or one of the
sixteen equivalences above, each demonstrated rather than asserted, per this crate's own two
non-selector cases (`erase_display`, `pixel`) resting on an invariant named and cited at the
function itself rather than only here, matching this file's own convention.
