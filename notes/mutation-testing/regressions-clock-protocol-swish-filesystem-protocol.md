# The 2026-09-14 regressions: `clock_protocol`, `swish`, `filesystem_protocol`

An appendix of [notes/mutation-testing.md](../mutation-testing.md), moved there verbatim on 2026-09-24.

### `clock_protocol`: 6 survivors, 3 were the loom model, 3 are equivalent and none is a gap

**Before: 54 caught, 6 missed, 7 timeouts, 5 unviable (91.0% of viable). After: 54 caught, 3 missed
(95.3%).** **Nothing was killed here and no test was written, which is the correct outcome rather
than a shortfall**: three survivors left with the `interleavings::` exclusion above, and the other
three cannot differ from the original under `cargo test`. The rate moved because the measurement
stopped counting a file the suite never compiles, not because the suite got better.

- **`spin_hint` replaced by `()`.** Under `not(loom)` it is `core::hint::spin_loop()`, which is
  documented as a hint with no effect on program semantics, so removing it cannot change any
  observable behaviour of a host test. Under `--cfg loom` it is `yield_now()` and it is
  load-bearing, for liveness rather than correctness: loom's scheduler is cooperative and a spin
  that never yields starves the writer it waits on. That is `script/interleaving-check`'s property
  and not this suite's.
- **`publish`'s `compare_exchange_weak(s, s + 1, ..)` under `*`.** `s * 1` is `s`, so the claim
  leaves the sequence **even**: the odd marker that tells a reader a write is in flight is never
  set. Single-threaded that is invisible, because the publish completes before any read begins and
  the final sequence is `claimed + 2` either way; this is the "single-threaded blindness" pattern
  named at the top of this file, and it is recorded **equivalent-under-harness** rather than
  excluded, so it stays visible.
  **It is killed by the gate that owns it, and that was measured rather than assumed**: with the
  mutation applied, `script/interleaving-check -p clock_protocol` fails **four** of its harnesses
  (`a_reader_never_sees_half_a_publish`, `a_racing_reader_sees_an_unrecognised_page_or_a_whole_one`,
  `the_generation_a_reader_sees_matches_the_pair_it_read`,
  `two_writers_serialise_rather_than_corrupt_the_page`). A host test cannot carry a concurrency
  claim; the loom model can, and here it does.
- **`policy::decide`'s `proposed_nanos > current_nanos` under `>=`.** At equality the two branches
  compute the same thing: the `if` arm tests `0 > MAX_STEP_FORWARD_NANOS` and the `else` arm tests
  `0 > MAX_STEP_BACKWARD_NANOS`, both constants are non-zero, and both fall through to
  `status::ACCEPTED`. Equivalent for every input, not merely untested.

**The 7 timeouts are the seqlock's retry loops**, the same `dtb` cursor family one level up: `s & 1
!= 0` under `==`, `&` under `|` and `^`, and the reader's sequence comparison, each of which turns
a bounded retry into one that never exits. The baseline's `clock_protocol` row recorded 7 hangs of 9
survivors for the same reason.

### `swish`: 20 survivors, 16 killed, 4 equivalent

**Before: 157 caught, 20 missed, 11 timeouts, 12 unviable (89.4% of viable). After: 173 caught, 4
missed (97.9%).** Every survivor was in the shell's printers or its two smallest accessors, which
is what a crate that is mostly rendering should be expected to produce.

**One of them is a test that could not fail, and it is the one to read.** `write_found` right-aligns
a match count and starts every title in the same column, and
`a_search_answer_names_pages_a_reader_can_type` asserted
`cols == [APROPOS_TITLE, APROPOS_TITLE]`: the rendering compared against the constant it is derived
from, which is an identity. Six mutants lived in that one line, one for every `+` in
`const APROPOS_TITLE: usize = 2 + APROPOS_COUNT + 2 + 28 + 2`, because changing the definition moved
both sides of the assertion together. The fix is the literal 38, which is what a reader at eighty
columns actually gets. A seventh survivor was next to it: the digit-counting loop's `count / 10`
under `%`, invisible because every fixture used a two-digit count, so a new test uses a three-digit
one and pins the column under the widest row.

**The rest are the ordinary shape.** `write_batch` had no caller in the crate's own tests, so its
whole body could be `()`: a sweep would have shown a person no set while handing each batch a real
one, which is exactly the property the batching lane exists to make visible. `Status::from_code` had
no caller either, so it could be `Default::default()` with two arms deleted; it crosses an atomic
cell as a `u64`, so the round trip is the contract. `write_apropos`'s `offered() > results().len()`
under `>=` printed the truncation tail when nothing was truncated, which no test forbade.
`Sequence::is_plain` could be a constant `true`, and that one is worth naming: it is the
pre-connector shape test, so a `true` there routes `date && wc` down the single-command path and
drops everything after the first connector. Every fixture asked it only of a one-segment line.

**And one kill is honest about being a side effect.** `write_duration`'s
`(nanos % SEC) / MILLI` under `+` is killed by a new assertion that the printer is total on every
`u64`, which it earns on its own merits (the shell hands it `end - start` off a counter it does not
own, and a clock that went backwards produces a number nobody chose). It kills the mutant by
overflow rather than by disagreement, and the two mutants in the smaller units, where the addition
cannot overflow, are equivalent instead.

**The four equivalents.**

- **`write_duration`'s `(nanos % MILLI) / MICRO` and `nanos % MICRO` under `+` (2).** The fraction
  is printed digit by digit as `t / 100 % 10`, `t / 10 % 10`, `t % 10`, which is `t mod 1000`. Since
  `MILLI` and `MICRO` are exact multiples of the unit below, adding one adds exactly 1000 to the
  quotient, and `(t + 1000) mod 1000 == t mod 1000`. The mutants print the same three digits for
  every input that does not overflow, and in these two branches the input cannot.
- **`write_refusal`'s `Refusal::NoSuchProgram => {}` arm deleted (1).** That arm is reached only
  when the guard above it fails, which is exactly when `spec.prog` is empty. Deleting it sends that
  case to `_`, which prints a prefix only `if let Some(p) = Prog::from_name(spec.prog)`, and
  `Prog::from_name(b"")` falls to its `_ => None`. Same output, by two routes.
- **`Sequence::is_empty` under a constant `false` (1).** The function is documented "never true" and
  the invariant holds: `split` always produces at least one segment, so `self.n == 0` is `false` for
  every value this type can take. `-> true` and `==` under `!=` are killed by the assertions added
  beside `is_plain`; this third one cannot be, because it is what the function already does.

**The 11 timeouts are hangs**, the same loop-control family as `dtb` and `clock_protocol`: `echo`'s
word cursor under `-=` and `*=`, `write_found`'s digit loop under `>=`, `pad`'s `left -= take` under
`/=`, and `split`'s segment cursor under `*=`. Each stops the loop advancing.

### `filesystem_protocol`: 59 survivors, 19 killed, 38 equivalent, 2 recorded gaps

**Before: 529 caught, 59 missed, 10 timeouts, 47 unviable (90.1% of viable). After: 548 caught, 40
missed (93.3%).** This is the crate the baseline already found dominated by equivalents (36 of 46
under its old name `fs_proto`), and the census's 59 have the same shape: a contract crate is mostly
constants and packing, and most of the mutants a tool can make there cannot change a value.

**The nineteen real gaps, and four of them are witness bits that report nothing.**

- **`fixture::twodir` had no distinctness test at all (5).** Five of its six bits could each become
  zero, and two of them are the structural finding `notes/dir-capability.md` records (the endpoint
  is the boundary, witnessed with two live caretakers). A witness bit of zero is a probe that
  reports nothing while its boot passes. Closed by `the_two_grant_bits_are_distinct`, in the shape
  the escape, directory and navigation fixtures already use.
- **`fixture::navscape`'s list had gone stale again (3).** `BIND_REACHED_TARGET`,
  `BIND_ASCEND_REACHES_REAL_PARENT` and `BIND_STOPS_AT_TRUE_ROOT` (2026-08-30) were never added to
  `the_navigation_bits_are_distinct`, whose body already carries a comment about the *previous* six
  that were added late. That is rung four of AGENTS.md's ladder failing the way rung four does, and
  this time the mutation run is what noticed rather than a reader. The three are added; the
  mechanism that would stop it recurring is named in milestone 326's handoff rather than built here.
- **`fixture::throughput::name` had no caller (9).** The whole function as `None`, as `Some("")` and
  as `Some("xyzzy")`, plus each of its six arms deleted. The bench boot prints these names beside
  the numbers, so two phases sharing one is two rows a reader cannot tell apart. Closed by
  `every_throughput_phase_tag_has_its_own_name`, which also pins the count against `PHASES`.
- **`statfs::free_bytes` replaced by `0` (1).** Its only assertion was
  `free_bytes(4096, 0) == 0`, which the mutant satisfies. A volume with room would have reported
  itself full to everything that asks before it writes.
- **`statfs::encode`'s `out.len() < LEN` under `<=` (1).** The tests covered `LEN - 1` and 64 and
  never `LEN` itself, which is the one size a caller sizing its page from the constant would hand
  it.

**The thirty-eight equivalents, in four groups, so a later reader can re-check them by group rather
than one at a time.**

- **Packed wire words whose fields are bit-disjoint (8).** `blk::req`, `fs::req` (two), 
  `fs::rename_dst`, `grant::spec`, `nameset::encode`, `xattr::spec` and `xattr::reply` all build one
  word as `(field << shift) | (other & mask)`. Each shift clears exactly the bits the mask keeps, so
  `|` and `^` are the same function on those operands. The disjointness is not assumed: it is what
  `a_handle_never_collides_with_the_opcode_or_length`, `a_granted_name_survives_the_two_argument_words`
  and `a_rename_carries_two_directories_and_two_lengths_without_them_bleeding` already prove, and
  `nameset`'s type bit is `1 << 7` against a length the encoder refuses above `grant::MAX_NAME`.
- **Unions of distinct single-bit rights (14).** `dir::ALL`, `dir::REMOVE_TREE`, `Verb::mutates`'s
  mutating mask, and four `needs_any`/`needs_all` rows in `verb::TABLE`. Every operand is a distinct
  `1 << n`, so `^` is `|`; and that the bits are distinct is itself pinned, by
  `undefined_rights_bits_cannot_be_smuggled_into_a_root` and by every non-degenerate `1 << n` in
  `dir` being a caught mutant.
- **The attrs witness union (7).** `fixture::attrs::EXPECTED` is the eight distinct bits of its own
  module ored together, and `the_attribute_bits_are_distinct` is what makes the disjointness a fact.
- **`1 << 0` under `>>` (9).** `dir::ENUMERATE`, `dirent::IS_DIR`, `grant::READ`, and the first bit
  of six fixture witness sets. Both sides are 1. The degenerate case this file's patterns section
  already names, recorded rather than excluded so it stays visible if a constant ever moves off zero.

**And two recorded gaps, which are the same object as the Kani harnesses one paragraph down.**
`verb`'s second `const _: () = { .. }` walks `TABLE` asserting each row sits at its own opcode, and
its loop bound `i < TABLE.len()` survives under `==` and under `>`: both make the loop body never
run, so the block compiles and checks nothing. **No `cargo test` can see this**, because the checker
is `rustc` and the evidence of success is that the build happened. The comment above it already says
a runtime test cannot do this job (the three caretakers are `no_std` binaries). It is recorded here
rather than excluded because the exclusion would have to be by line and would hide anything else
that lands there.

**The 10 timeouts are hangs**, all of them iterator cursors: four `Iterator::next` implementations
replaced by `Some(Default::default())`, which never consumes its buffer, and six `at += ..` under
`*=` in the record walkers and the name packers. A walk that stops advancing loops forever, which is
the tests noticing.
