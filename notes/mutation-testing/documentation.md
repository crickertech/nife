# `documentation`: a crate scored with a third of its tests compiled away

An appendix of [notes/mutation-testing.md](../mutation-testing.md), moved there verbatim on 2026-09-24.

## 2026-09-13: `documentation`'s 52% was a crate scored with a third of its tests compiled away

Milestone 280. The section above named `manual` (ratified `documentation` on 2026-09-13) as the
second of the two crates carrying the tree's fall from 92.4%, at **56 caught, 60 missed** in the
round-robin sample. `uefi_loader` went first and turned out to be arithmetic; this one was expected
to be the real gap, because it is ordinary host-testable `no_std` Rust with no allocator on the read
path.

**It was both, and the measurement bug was the larger half.** This is the `system_initializer`
result (milestone 244) and the `uefi_loader` result (2026-09-04) a **third** time, one level further
in again: not a crate the host suite cannot compile, and not a target the build graph never selects,
but a **Cargo feature** that a single-package test run does not turn on.

`cargo-mutants` tests one package at a time. `--test-workspace` is off by default and turning it on
would run the whole workspace suite once per mutant, so the command behind every number this sweep
has ever published for this crate is `cargo test -p documentation` with that crate's **default**
features. `builder` is not one: it gates the index **writer**, which allocates, and it is off by
default so the guest program links no allocator. Ninety-two of the crate's 1,043 mutants live behind
it, and nothing could ever have killed one.

**The writer was not the cost, though, and that is what nobody had noticed.** `builder` also gates
**six of the crate's twelve index tests**, because a test that reads an index needs `build` to make
one to read. So the run compiled away half the suite and then scored the *reader* against what was
left: `lookup`, `postings`, `page_record`, `score`, `Ranked::offer`, `search` and `location` had no
test at all in the configuration being measured.

### The numbers, whole-crate, same machine, same pinned tool

| | mutants | caught | missed | timeout | unviable | killed |
|---|---|---|---|---|---|---|
| as the sweep measures it, before | 1,043 | 499 | 455 | 48 | 41 | **54.6%** |
| with `builder` compiled, before any new test | 1,043 | 733 | 211 | 58 | 41 | **78.9%** |
| **after milestone 280** | 1,056 | 908 | 47 | 60 | 41 | **95.4%** |

"Killed" is caught plus timeout over viable, the same arithmetic as the baseline table above. The
published 52% was a one-eighth sample; the whole-crate figure in that same configuration is 54.6%,
so the sample was honest about a crate being measured wrongly.

### The fix, and why it is not `--all-features`

**A dev-dependency on itself.** Cargo allows a package to depend on itself for tests only, and
resolver 2 unifies the feature across the test build, so `cargo test -p documentation` now compiles
the builder and runs all twelve index tests while `cargo build -p documentation` still does not.
Nothing a consumer sees changes, because a dev dependency is not transitive: `user` and `swish` link
no allocator and did not have to say so.

**`--all-features` was measured rather than assumed, and it fails.**
`cargo check -p uefi_loader --all-features` panics in that crate's build script without
`NIFE_UEFI_KERNEL`, by design, because the UEFI application embeds the kernel it boots. Three
packages in this workspace declare a feature (`documentation`, `kernel`, `uefi_loader`) and they want
three different answers, so one global switch is wrong for at least two of them.

**`script/lint` gained the third gate in this family**, beside the two the earlier two findings left.
It derives from `cargo metadata` that every feature of every mutated crate is enabled when that
crate's own tests run, and takes `required-features` as the one honest exemption: a feature whose
only job is to gate targets already excluded (`uefi_loader`'s `uefi`) has no library code to compile.
Verified in both directions, by removing the dev-dependency and watching it name the crate and the
feature.

**`script/coverage` hit exactly this failure on this crate two weeks earlier and fixed only itself**,
which is the reason a gate was worth the twenty lines. On 2026-08-30 (PR #590, milestone 87) the
coverage floor reported 33.3% on an index reader that is round-tripped against its own writer,
because `builder` was off. It grew `--features documentation/builder` and a comment saying that a
feature gating something un-buildable *"is a trap for every workspace-wide sweep, not just this
one"*. The mutation config, whose own head comment asks the next person to keep the two lists in
step, was not touched, and fourteen days later published 52% for the same crate for the same reason.
That is rung four doing what rung four does.

### Two features the sweep found that no test could have

Both are the same shape and it is worth naming, because a mutation run is the only instrument in the
tree that sees it: **a value computed correctly and read by nobody.** There is no output for a test
to assert, so the code cannot fail; the tell is a *cluster* of survivors inside one function,
including the mutant that replaces the whole function with `()`.

- **Table column alignment.** `read_align` parsed `:---`, `---:` and `:---:` off the delimiter row
  into a per-column array from the first day, and `flush_table` padded every cell on the right
  whatever that array held, so all three rendered identically to `---`. Sixteen survivors. It is
  honoured now, and resets with `delimited`, because both are properties of one delimiter row.
- **Tab indentation.** `indent_of` returns a byte offset and a column count, counting a tab as four
  columns; every caller took the offset and dropped the count, including the one that uses it as a
  margin. Invisible on this corpus, which is why only a sweep could have found it: every
  tab-indented line in this repository's markdown is inside a fence, where that code does not run.

### The ledger for this crate

Forty-nine tests were added across four passes (the crate had 32 and has 81), each written against a
named survivor rather than against a feature list. The dispositions of what the final run still reports, using this note's own
vocabulary:

| | count | what they are |
|---|---|---|
| killed | 164 | the fall in missed mutants from 211, each kill verified by the sweep itself |
| equivalent | 17 | proved unable to differ; the groups are below |
| hang | 60 | every timeout is a loop counter or a loop bound, listed below |
| deferred | 30 | real gaps whose test needs a fixture sitting on an exact byte boundary |

**The equivalence groups, because a verdict reached by reading is wrong about ten percent of the
time and these are the ones to re-check first.**

- **`lookup`'s `here` (3).** `(h.terms - lo * per).min(per)` is the count of records to search on the
  final page. Every mutant of it yields a value **at least** the true one, and `.min(per)` caps them
  all at the 128 records the page buffer holds, so none can under-read. The records past the real
  end are the builder's zero padding, where `cmp` reads a length byte of zero and compares an empty
  slice against a non-empty key, which is `Less` and never `Equal`. Verified by applying one of
  them: Rust's left-associativity turns `terms - lo * per` into `(terms - lo) + per`, not the
  underflow the mutant looks like.
- **The output cursor where nothing reads it (5).** `Renderer::col` has two consumers: a wrap
  decision, and `close_line`'s "is a line open" test. Inside `rule` and `flush_table`'s row loops
  the next thing to touch it is `close_line`, and every row ends with `col += width` for a width of
  at least one, so a wrong value cannot reach zero and cannot reach a wrap. The mutants
  that **can** are killed: a blank line inside a fence (the one code line of zero visible width), an
  image followed by text that wraps, and a quoted paragraph at two levels.
- **Absorbed by the code after them (8).** `line_done`'s explicit space-skip after a block quote
  marker is redundant with the `indent_of` call on the next line, twice; `heading_at` guarantees a
  space at the index `heading` slices from, and `inline` skips leading spaces, so an off-by-one
  there emits the same words; `read_align` starting at the `|` instead of past it produces an empty
  first spec, which it already skips; `flush_table`'s `vis > width` assigns the same value under
  `>=`; the column-selection `c < cols[r]` under `<=` reads a `(0, 0)` cell that is the same as its
  own else branch; `search`'s `done < count` under `<=` runs one more iteration that asks for zero
  postings and breaks; and `finish`'s `used > 0` under `>=` ends the document by classifying a line
  of no bytes, which emits nothing.
- **`title_of`'s line walk (1).** `while start < text.len()` under `<=` runs one extra iteration on
  an empty slice, which is never a heading.

**The deferrals, and what each one's test would cost.** Thirty, and they are gaps rather than
equivalents, recorded as gaps:

- **The inline scanner's `i + 1` forms at the exact last byte of a line (19).** Every one is a
  lookahead guarded by `i + 1 < end`, and the mutants either drop the guard or read one byte past
  the classified span. A test needs a line whose final byte is the opening half of a two-byte
  marker: `~`, `!` or `*` as the last character, with the construct it would open truncated by the
  line ending. Fifteen of the nineteen also need the depth counter at its bound at the same time.
- **The table arenas at their exact fill points (5).** `take_table_row`'s row buffer flushes when
  the cell text would pass 8,192 bytes, and the mutants move that threshold by one or change which
  sum is compared. A test has to construct a table whose cumulative cell text lands on 8,192
  exactly, which pins an implementation constant rather than a property, and `read_align`'s cell
  walk wants the same.
- **The output cursor where it feeds a wrap (4).** `unit`, `unit_bytes`, `unit_wrapped` and
  `before_unit`, the four places `col` is read again before `line_start` resets it. The three passes
  above killed the ones that reach zero or move a visible wrap; what is left needs the cursor wrong
  by an amount that lands on a wrap boundary.
- **`past_quote`'s loop bound (2).** Its three length tests are separated by the blank-quoted-line
  test above; the two on the loop condition itself want a fence opened at a depth greater than the
  markers any line inside it carries, at the exact byte where the line ends.

A mutation of any of these changes behaviour only for an input this repository's markdown does not
contain. That is why the deferral is honest rather than tidy, and it is also the limit of the
renderer's own argument: it was written instead of taking `pulldown-cmark` on the grounds that the
input set *is* this repository, so a gap outside that set is exactly the cost that bargain has.

**The timeouts are hangs, and the evidence is that every one of them is loop control.** Fifty-seven
of the 60 are `+=` or `-=` on a loop counter or the bound of a `while`, spread over `inline` (12),
`unit_wrapped` (6), `lookup` (5), `line_done` (5), `closer` (4), `read_align` (3), `pad` (3), and
eleven others. The remaining three replace a whole function whose return value is the loop's step:
`char_len -> 0` and `closer -> Some(0)`/`Some(1)`, each of which leaves the caller advancing by
nothing. A mutant that makes a loop stop advancing hangs rather than lying, which is
the tests noticing; the note's scope section is explicit that a timeout on a mutant that could not
hang would be triaged as a survivor instead, and none of these is that.
