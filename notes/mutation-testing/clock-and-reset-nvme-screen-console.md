# `jh7110_clock_and_reset`, `non_volatile_memory_express`, `screen_console`

An appendix of [notes/mutation-testing.md](../mutation-testing.md), moved there verbatim on 2026-09-24.

## 2026-09-21: `jh7110_clock_and_reset`, `non_volatile_memory_express`, `screen_console`, milestone 326 (turn a mutation score upward) part 3

Three more crates off the seventeen the Follow-on section left outstanding after the 2026-09-20
lanes. **Re-derived against this file and each crate's own history before picking anything**, per
the rule the block gained on 2026-09-21: `counter_frequency_protocol`, `cpu_set`,
`environment_protocol`, `login_protocol`, `thread_wake_handshake` and `uefi_loader` were checked and
found already at 100% of viable mutants (their census-era survivors were closed by other work in the
intervening week); `documentation` was checked and set aside as its own lane's worth of work (98
survivors, 1056 mutants, and a resolver-feature history already spanning two of this file's own
sections); `firmware_configuration`, `portable_executable`, `sealed_pair` and `stick_maker` are new
enough that no census has ever measured them and were left for a lane that can give them their own
attention. The three below were chosen because each had a real, moderate, unrecorded survivor count
and no complicating history.

Same discipline as every section above: every number is re-derived with `script/mutation -p <crate>`
on this lane's own worktree rather than trusted from the census, every kill was verified by
re-running the sweep and watching the mutant die, and every equivalence claim is a mutant the second
run still reports.

| crate | before | after | killed by a test | equivalent | excluded | recorded gap |
|---|---|---|---|---|---|---|
| `jh7110_clock_and_reset` | 12 | 0 | 12 | 0 | 0 | 0 |
| `non_volatile_memory_express` | 26 | 7 | 19 | 7 | 0 | 0 |
| `screen_console` | 9 | 2 | 7 | 2 | 0 | 0 |
| **total** | **47** | **9** | **38** | **9** | **0** | **0** |

**`screen_console`'s `before` does not match the 2026-09-19 census** (7 survivors there, 9 here),
which is exactly the shape the Follow-on section's own rule warns a lane to check for: the crate
grew between the census and this lane (116 mutants now against roughly 55 tested at the census date),
and re-deriving rather than triaging the census number directly is what caught it.

### `jh7110_clock_and_reset`: 12 survivors, all 12 killed

**Before: 53 caught, 12 missed, 0 timeouts, 3 unviable. After: 65 caught, 0 missed, 3 unviable.**
Matches the 2026-09-19 census row for row.

**Three were one blind spot: every reset id this crate ever asks for lands in word zero.**
`Domain::reset_bit`'s `word = (id / 32) as u64 * 4` and the two offsets it feeds
(`assert_offset`, `status_offset`) all survived (`*` to `/`, and both `+`s to `-`) because the STG
domain has 23 resets, so `id / 32` is always `0` for every id this crate has, and `0 * 4` and
`0 / 4` are both `0`. The arithmetic is written general on purpose (the module doc names the SYS
domain's 126 resets as the reason), so `a_reset_word_past_the_first_multiplies_the_word_index_by_four`
constructs a 128-reset `Domain` directly (a public struct, no fixture needed) and asks for id 35,
which is word 1: `(35 / 32) * 4 == 4`, and the two `-` mutants and the `*` mutant all disagree with
that.

**Two were the same blind spot one function over: `reset_mask` is private and every caller of
`was_already_up` happened to leave it either 0 or unneeded.** `reset_mask_recovers_the_bit_that_
changed_between_the_two_assert_words` calls it directly (a private `const fn` on the crate's own
root module, reachable from `tests` the same way every other private helper in this crate already
is) with `before = 0b1010, after = 0b0010`, which is `0b1000` under `^` and disagrees with all four
mutants (`with 0`, `with 1`, `^` to `|`, `^` to `&`) in one assertion.

**Three were `was_already_up`'s reset half, untested from either direction.** Every existing fixture
with `clocks_running() == true` also had the reset already fully released, so `!self.had_reset ||
... == 0` was always `false || true`, and deleting the `!` (making it `true || true`) changes
nothing when the second half is already true. Two new reports close it: one with the reset bit still
set in `reset_assert_before` (kills the deleted `!`, and, as a side effect, the `&` to `^` mutant,
because at that operand pair `&` and `^` also happen to compute the same nonzero value), and one
with the target bit clear but an unrelated bit set elsewhere in the word (kills `&` to `|`, since `&`
correctly ignores the noise bit and `|` does not).

**Two were `discover_as`'s bounds guard on a name index it read out of the tree, and the fixture set
had never carried a tree malformed the way the guard exists to catch.** `Some(i) if i < n => i` (`i
< n` relaxed to `true`, and `<` widened to `<=`) survived because every existing `.dts` fixture's
`reg-names` list names exactly as many windows as `reg` has. A new fixture,
`jh7110-clkgen-vendor-mismatched-names.dts`, gives the vendor clkgen node two `reg` windows but
three `reg-names` entries with `"stg"` last (index 2, one past the two the array holds); `discover`
over it must fall back to the corroborated [`STG_BASE`] constant rather than read the zeroed slot an
off-by-one would land on, which is what `a_reg_names_entry_past_the_end_of_reg_is_refused_rather_
than_read` checks.

### `non_volatile_memory_express`: 26 survivors, 19 killed, 7 equivalent

**Before: 137 caught, 26 missed, 0 timeouts, 10 unviable (84.0% of viable). After: 156 caught, 7
missed, 10 unviable (95.7%).** Matches the 2026-09-19 census row for row.

**`Command::write` had never been called by a single test.** `transfer_command`'s own test
(`the_data_plane_refuses_a_block_the_namespace_does_not_have`) always passes `write: false`, so
every mutant in `write`'s body (the LBA split under `>>`/`<<`, and the block-count-minus-one under
`-`/`+`/`/`) survived regardless of what it did. `a_write_command_encodes_the_spec_dwords` mirrors
the existing read-command test exactly, dword for dword, and closes all three.

**`CqState::head` had a caller, but every caller only ever asserted `head() < entries`, which a
constant `0` or a constant `1` both satisfy.** `head_reports_the_actual_next_slot_not_a_constant`
asserts the actual sequence (`0`, then `1`, then `2` across two pops) instead.

**`cc_enabled` had no caller at all**, so its whole body (both `|`s, both `<<`s, and the two
whole-function stubs) survived. `cc_enabled_sets_en_and_the_admin_queue_entry_sizes` reads the EN
bit and both entry-size fields back out of the returned word, which kills the two stubs and both
`<<`-to-`>>` mutants; the two `|`-to-`^` mutants are equivalent, below.

**Two boundaries in `parse_identify_namespace`, untested at the boundary itself.** The truncation
check (`data.len() < 384`) had only ever been tried far below the boundary (`&[0u8; 100]`), which
cannot tell `<` from `<=`; `identify_namespace_accepts_the_minimum_length_and_refuses_one_byte_less`
sits exactly on it. The LBA-format-table offset (`128 + 4 * flbas as usize`) had only ever been
tried at `flbas == 0`, where `128 + 4*0` and `128 - 4*0` are both `128`;
`identify_namespace_reads_the_lba_format_table_at_flbas_not_format_zero` uses `flbas == 1` and plants
a different, still-valid LBADS byte at format 0's slot so a lane reading the wrong one is caught by
value.

**`IdentifyNamespace::blocks_per`, two survivors from one test that never varied its `unit`.** The
existing test calls `blocks_per(4096)` only, and 4096 is a multiple of every mask the three mutants
(`|| ` to `&&`, and the mask's `- 1` to `+ 1` or `/ 1`) could produce, so none of them show up.
`blocks_per_masks_by_the_actual_block_size` adds `blocks_per(0)` (kills the `||`-to-`&&`, since `0 ==
0` is the only thing keeping that branch true once the mask term is forced to `0`) and
`blocks_per(1536)` (`1536` shares a bit with the wrong masks `513` and `512` but not with the correct
`511`, which kills both arithmetic mutants at once).

**`Handoff::pack`'s `dstrd` shift, and `Handoff::unpack`'s minimum-queue-depth boundary, both
untested at the one case that would show them.** The existing round-trip test uses `dstrd: 0`, where
`<< 32` and `>> 32` both give `0`; `pack_shifts_dstrd_left_not_right` uses `dstrd: MAX_DSTRD` (8) and
reads the high word back. `Handoff::unpack`'s `entries < 2` (`SqState::new`'s own doc says why 2 is
the spec's floor: a one-slot ring cannot tell full from empty) had only ever been tried at `entries:
1`, which `<` and the mutant `<=` both refuse; the same assertion in
`the_handoff_survives_three_words_and_refuses_nonsense` now also checks `entries: 2` is accepted.

**Seven are equivalent, and all seven are the same shape: `| -> ^` on a pair of bit fields packed
into disjoint halves of a word by construction, not by the values a test happened to choose.** A
left shift by `n` always zeroes the low `n` bits of its result, and a value that only ever occupies
those `n` bits (because it is narrower, or because it too was shifted by at least `n`) can therefore
never share a bit with it: `|` and `^` compute the identical word on every input these functions
ever combine, not only the ones any test tries.

- **`cc_enabled`, 2 mutants.** `1` (bit 0), `6 << 16` (bits 17–18) and `4 << 20` (bit 22) are three
  fixed literals with no shared bit between any pair; `cc_enableds_three_fields_share_no_bit_so_or_
  and_xor_agree` checks all three pairs directly, since the function takes no argument and these are
  the only operands it will ever have.
- **`Command::create_io_cq` and `Command::create_io_sq`, 3 mutants.** `qid as u32` (or the literal
  `1`) against `(entries as u32 - 1) << 16` (or `(cqid as u32) << 16`): the left side is a `u16`
  promoted to `u32`, so it never sets a bit past 15; the right side is shifted left by 16, so it
  never sets a bit below 16. True for every `u16` qid/cqid, not only the ones the existing test
  picks; `queue_creation_words_pack_disjoint_halves_so_or_and_xor_agree` checks it at `0xffff`, the
  widest either half can be.
- **`Handoff::pack`, 2 mutants.** `entries` (bits 0–15), `blocks_per as u64 << 16` (bits 16–31) and
  `dstrd as u64 << 32` (bits 32–63): the same shift argument, twice, checked at each field's widest
  value in `pack_words_three_fields_share_no_bit_so_or_and_xor_agree`.

### `screen_console`: 9 survivors, 7 killed, 2 equivalent

**Before: 104 caught, 9 missed, 0 timeouts, 3 unviable (92.0% of viable). After: 111 caught, 2
missed, 3 unviable (98.2%).** Does **not** match the 2026-09-19 census's 7 (see above); this crate
grew between the census and this lane.

**`ScreenConsole::span` had no caller anywhere in the suite** (`Framebuffer::span` and
`Aperture::span` are different methods the tests do call, which is what let this one hide), so both
stub mutants survived. `console_span_is_the_underlying_screens_span` calls it directly.

**`clear`'s whole body had never been inspected**, because every test that calls `clear` immediately
writes text that covers the whole grid, so a stubbed-out `clear` and the real one produce the same
final picture. `clear_paints_the_whole_surface_in_the_background_colour` poisons the buffer first and
checks every pixel right after `clear`, before anything else runs.

**The `b'\r'` match arm had never been sent a carriage return.** Deleting it falls through to the
default arm (a no-op) and then, because the match does not return in that case, draws a blank glyph
at the un-reset column and advances past it, which is a different final picture from "reset the
column, draw nothing". `a_carriage_return_resets_the_column_without_drawing_a_glyph` sends `"a\rb"`
and checks that `b` landed at column 0 (overwriting `a`) rather than column 1.

**`scroll`'s three-way guard, all three branches untested at the boundary.** `live > pixels.len() ||
band >= live`: every fixture's pixel buffer (4096 * 16 bytes, fixed) is far larger than any `live`
region these tests compute, so `live > pixels.len()` was never true and its `==`/`>=` mutants
survived; every fixture had at least two rows, so `band >= live` was never true either, and the
`||`-to-`&&` mutant survived alongside it.
`scrolling_a_one_row_console_leaves_it_alone_rather_than_blanking_it` builds a one-row console (where
`band == live` by construction: one row's band is the whole live region) and writes past its last
column; under the `&&` mutant, the early return is skipped and `scroll` paints the *entire* live
region to the background colour before the next character is drawn, silently blanking everything the
console had just written, which the test catches by checking the two characters written before the
wrap are still there. `scroll_accepts_a_buffer_that_is_exactly_as_long_as_the_live_region` sizes a
buffer to exactly `live` bytes (as a `const`, since this crate is unconditionally `no_std` and has no
`Vec` to size at runtime) and checks it gets scrolled rather than refused, which `>` and `==`/`>=`
disagree about at exactly that one point.

**Two are equivalent, the same disjoint-bit-field shape as `non_volatile_memory_express` above.**
`Aperture::to_words`'s two words each pack a `u32` (`width`, `stride`) in the low half against a
value shifted left by 32 (`height`, `order`) in the high half; `to_words_two_halves_share_no_bit_so_
or_and_xor_agree` checks it at `u32::MAX` on both sides, the widest either half can be.

### No defect in shipped behaviour was found in any of the three

Every survivor across all three crates closed as a missing test or a demonstrated equivalence; none
required an exclusion or a recorded gap, and none is a behaviour question left open the way
`timetable::Unbacked`'s `BUGS` section is.

**The Follow-on section's "seventeen crates" does not reconcile cleanly with what this lane found,
and that gap is itself worth recording rather than silently rounding away.** Walking the current
tree's mutation-scoped crates (`script/mutation --list`, 66 crates) against `.cargo/mutants-baseline.txt`
(the 38 baseline crates, which carries every rename applied since) finds **29** crates outside the
baseline, not 26: some are genuinely new since the 2026-09-14 census
(`firmware_configuration`, `portable_executable`, `sealed_pair`, `stick_maker`, none of which any
census has ever measured), and `component_plan`, `credentialer`, `loaded_image_check`, `pgrep`,
`pmap`, `ps` and `uptime` were apparently never picked up by the count the block's "seventeen" was
computed from. None of the eight already-closed-at-0% crates checked at the top of this section
(`counter_frequency_protocol`, `cpu_set`, `environment_protocol`, `login_protocol`,
`thread_wake_handshake`, `uefi_loader`, plus `timetable` and `work_steal_slot` from earlier parts)
explains the gap either; they are exactly the ones the "seventeen" figure should already have
excluded.

**What is actually left, re-derived rather than assumed:**

- **A real, moderate survivor count, not yet re-measured against the current tree:**
  `component_plan`, `credentialer`, `loaded_image_check`, `pgrep`, `pmap`, `ps`, `uptime`.
- **Its own lane, for size and history:** `documentation` (98 survivors at last measurement, 1056
  mutants, and a resolver-feature story spanning two sections of this file already).
- **Never census'd, no `before` number exists yet:** `firmware_configuration`, `portable_executable`,
  `sealed_pair`, `stick_maker`.

Twelve crates, not fourteen or seventeen. A lane taking the next piece of this should re-derive this
list itself with `script/mutation --list` against the baseline file rather than trust either number.
