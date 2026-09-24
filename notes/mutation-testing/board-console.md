# `board_console`

An appendix of [notes/mutation-testing.md](../mutation-testing.md), moved there verbatim on 2026-09-24.

## 2026-09-20: `board_console`, milestone 326 (turn a mutation score upward) part 3

`board_console` is one of the new-crate backlog's worst rates, at **238 viable, 43 survivors, 79.4%
caught** in the 2026-09-19 census. Untaken until now. Same discipline as the sections above: every
number below is re-derived with `script/mutation -p board_console` on this lane's own worktree
rather than trusted from the census, every kill was verified by re-running the sweep and watching
the mutant die, and both equivalence claims are mutants the second run still reports.

**Before: 242 caught, 43 missed, 6 timeouts, 31 unviable (83.2% of viable, 85.2% counting a timeout
as a kill). After: 283 caught, 4 missed, 6 timeouts, 31 unviable (96.6%, 98.6% counting timeouts).**
The census's own count differs from this run's (238 viable against this run's 291) for the reason
its own `BUGS` section gives: the tool version and the tree both move between a census and a lane
taking it. This run's `before` column is what this branch measured before anything changed, which is
the honest comparison point for the `after` beside it.

| module | survivors | killed by a test | equivalent | recorded gap |
|---|---|---|---|---|
| `board.rs` | 3 | 3 | 0 | 0 |
| `lottery.rs` | 8 | 8 | 0 | 0 |
| `port.rs` | 12 | 9 | 0 | 3 |
| `progress.rs` | 9 | 8 | 1 | 0 |
| `screen.rs` | 2 | 2 | 0 | 0 |
| `stop.rs` | 2 | 2 | 0 | 0 |
| `watch.rs` | 7 | 7 | 0 | 0 |
| **total** | **43** | **39** | **1** | **3** |

The 6 timeouts (all `screen::parse_pixmap`'s header-token loop, `+=` becoming `*=` or `-=`, or a
comparison flipped so the whitespace-and-comment skip never reaches a byte that ends it) are not in
the 43: this file's standing reading applies unchanged, they are the tests noticing by hanging
rather than by failing, and `cargo-mutants`' own 20-second per-mutant bound is the deadline. Nothing
here converts them, because nothing in this crate's suite calls the parser through a doctest or any
other path a deadline-wrapped test could not also reach; nothing here needed the conversion either.

**`board.rs`, 3 killed.** `Sign::seen_in`'s `None if complete => rest` arm (a complete line with no
trailing whitespace after a `WordAfter` prefix, e.g. `U-Boot 2021.10` with nothing following it) had
no test with a *complete* line short enough to hit it; the existing test only fed that shape as a
partial line. `Rung`'s `PartialEq` was only ever exercised through `<`, which goes through
`partial_cmp`/`Ord` rather than `eq`, so `assert_ne!` on two different depths was the one assertion
missing. `Profile::keys` was only ever checked for *emptiness* (`XENON.keys().count() == 0`), which
an always-empty iterator also satisfies; checking `RADON.keys()`'s actual contents closes it.

**`lottery.rs`, 8 killed.** Two were `parse_core_line` itself, never called directly by any existing
test (only through `tally`, whose fixtures happen not to probe the exact byte offset or a malformed
token). The `+`-to-`-` mutant shifts the slice nine bytes early; every fixture in the suite has
enough real content shifted into view that the guard filters it back out, so the miss needed a
token planted at exactly the wrong distance to catch (`core=0 G9 threads=1 R2`, engineered so the
shifted read lands inside `G9`). The `||`-to-`&&` mutant makes the "skip a malformed token" guard
unsatisfiable (an empty remainder is vacuously all-digits, so "not all digits AND empty" can never
hold), so a token like `Grinder` starts reading as a grinder; the test that catches it is the token
`||` was guarding against. The other six are all `Series::report`, none of them exercised because
every existing test asserts specific substrings are present and never that an *absent* one stays
absent: the "never reached the workload" line printed when `attempts == draws.len()` (`>` to `>=`),
the distribution header printed only when there is nothing to show it for (`!judged.is_empty()`
deleted), a bucket's `==` filter flipped to `!=` (invisible in the old test because the wrong bucket
produced the *same* substring the right one would have, just in a different row), a single rate
rendered as a degenerate range (`lo == hi` guard disabled), and the exclusion footer printed when
nothing was excluded (`<` to `<=`). Closed with `Series`/`Draw` built directly rather than through
`tally`, since both are `pub` structs with `pub` fields kept exactly for this: pinning the report to
values chosen for the assertion rather than to whatever a fixture happens to contain.

**`port.rs`, 9 killed, 3 recorded gaps.** `candidates()` read `/dev` directly, so nothing about its
result could be pinned down by a host test without depending on what happens to be plugged into the
machine running it; per this crate's own `BUGS`, that is "this lane's own Mac has the CH343 attached
... CI has nothing". Following the same move `choose`/`pick` already made, the walk is now `scan(dir:
&Path)`, with `candidates()` as a one-line `scan(Path::new("/dev"))`, and `scan` is tested against a
temporary directory built for the purpose (a real prefix, a look-alike with the wrong prefix, and a
sort check) rather than against `/dev`. The `stty()` boundary itself (nine mutants collapsed to one
line: this module's own header calls it "the whole IO residue... on purpose") had never been called
by a test directly; every existing test reaches it only through `open`/`configure` against a regular
file, whose complaint text the tests check only for *presence*, which every mutant's `Some(...)`
also produces. A direct call with a flag no `stty` accepts is a real, portable failure with no board
required, and checking the exit status is neither the empty string nor the mutants' `"xyzzy"`
placeholder kills all nine at once, none of them by asserting exact OS-specific wording. **Two of the
three port.rs mutants left are `candidates`'s trivial delegation**, and are recorded gaps rather than
closed: `scan`'s tests prove the logic, but `candidates() = scan(Path::new("/dev"))` itself still
depends on what is plugged into the machine running `cargo test` to be observably non-trivial, the
same limitation the module's own `BUGS` already names for `configure`/`confirm_speed`. **The third is
`confirm_speed`'s whole body replaced with `None`.** Every path this test can build (a regular file,
a missing path, invalid UTF-8) makes the underlying `stty -f <path>` fail (`ok=false`, checked
directly against `/dev/null`: `stty: /dev/null isn't a terminal`), which already returns `None` by
the function's own `if !ok { return None; }`, so the mutant is indistinguishable from correct code on
every input a host test can construct. Reaching the `Some(...)` arm needs a file `stty` treats as a
real tty, which is exactly the "No test opens a real serial device" limitation this crate's `BUGS`
already carries; closing it needs a pseudo-terminal pair (`posix_openpt`), which is a dependency
decision under DECISIONS §46 (thin primitives or whole subsystems) that a mutation-triage lane has no
authority to take.

**`progress.rs`, 8 killed, 1 equivalent.** `machine_line`'s capture guard was `&&`-to-`||`, the same
"kept from first arrival" property `banner_line` already has a test for, `machine_line` did not.
`Stage::label` and its `Display` impl (a whole-body replacement to `""` or a no-op write) had never
been asserted on directly; every report reads them by composing a string around them, which does not
pin the label's own text down. `Failure::describe`'s one match guard (`reason.is_empty()`, forced to
`true` or to `false`) had a test for each *shape* of `Failure::FirmwareRefused` but neither called
`.describe()`. `LineFeeder::tail`'s whole body was never read back directly, only through the
`Feeding::tail` it also produces. **The one equivalent is `BootProgress::reach`'s `>` becoming
`>=`.** The extra branch fires only when `stage == self.reached`, which for the one variant carrying
data (`Stage::Firmware(&'static Rung)`) can only hold when the two are the *same* rung: within one
session every `Stage::Firmware` comes from the one profile a `BootProgress` was built with, and
`every_profiles_depths_count_from_one_without_gaps` (in `board`, and depth is `Rung`'s whole identity)
is what makes a depth point at exactly one rung in that profile. Reassigning `self.reached` to a
value observably identical to what it already held is not observable. Recorded at the function
itself rather than only here, per this file's own convention.

**`screen.rs`, 2 killed.** `ReadError`'s `Display` (a no-op write, same shape as `Stage`'s above) had
only ever been checked by `PartialEq` on the *value*, never by reading what a person at a bench
would see. The header's three-way `max != 255 || width == 0 || height == 0` had one existing test
for the `max` term alone (`b"P6\n2 2\n254\n"`, both dimensions valid); an `||` flipped to `&&`
anywhere in the chain survives that test because the single true term already short-circuits the
*correct* code. Closed with `width == 0` and `height == 0` each alone, the other dimension valid,
which a flipped `&&` fails on regardless of which of the two `||`s it replaced.

**`stop.rs`, 2 killed.** `Report::describe`'s whole body (`String::new()` or `"xyzzy".into()`) was
never called by any test; every existing one checks `Escape::report()`'s *value*, and `describe()`'s
wording is only ever read by `Session::summary`, which composes rather than asserts on it. One test
covering all five variants, checking a distinguishing substring each.

**`watch.rs`, 7 killed.** `Session::summary`'s whole body, the same shape as `Report::describe` one
level down and never called directly for the same reason; `Session`'s fields are all `pub`, so it is
built directly rather than watched through a real session. `bytes += chunk.len()` becoming `*=` was
invisible because the one existing assertion on `.bytes` checks the *zero* case, where `0 * n` and
`0 + n` agree for `n = 0`; the fix asserts `session.bytes == sink.len()` on a real transcript, where
every byte read is also a byte written and the two counts have to agree by construction. The tail
flush's `!feeder.tail().is_empty()` deleted (so the flush fires only on an *empty* tail, a no-op) had
never been exercised: nothing in the suite fed a source that ends mid-line, which this module's own
header names as the reason the flush exists at all (`[PANIC] ...` with no trailing newline, followed
by a halted machine). `reader`'s `Err(e) if e.kind() == io::ErrorKind::Interrupted` (all three of the
guard forced `true`, forced `false`, and its `==` flipped to `!=`) had never been reached by any
test, because nothing in the suite hands the reader thread a real `io::Error`. Closed by a source
that returns `Interrupted` once and a genuine error after it, checking that the interrupted read was
retried (no `Failed` sent, the loop kept going) and the real one was not (propagated as the `Err`
`watch` returns).

**No defect in shipped behaviour was found.** Every survivor here was a missing test, or one of the
two documented, measured equivalences above.
