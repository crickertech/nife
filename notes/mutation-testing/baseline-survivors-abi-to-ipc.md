# The baseline survivors, crate by crate: `abi` to `ipc`

An appendix of [notes/mutation-testing.md](../mutation-testing.md), moved there verbatim on 2026-09-24.

### The hand-triaged crates

- **abi** (5): rights bits and the fault slot, killed by `rights_are_distinct_single_bits` and
  `the_fault_slot_is_inside_the_capability_table`; `1 << 0` equivalent as above.
- **asid** (2): both in `free`'s release-mode range guard, equivalent-under-harness
  (debug_assert shielding, above).
- **block_roster** (3): the header-only page, killed by
  `a_header_only_page_is_an_empty_roster_not_a_short_one`; `capacity_of`'s `<` to `<=` is
  equivalent (`len == HEADER_BYTES` yields capacity 0 down both arms).
- **c_seam** (5): verdict bits, killed by `the_verdict_bits_are_distinct_single_bits`; the
  `1 << 0` sibling equivalent.
- **calendar** (18): two real parser edges (a fraction scan that could read one past the end;
  an offset-colon check whose index could rot to `i - 3`, which lands on the seconds colon in
  every well-formed input, so `+05300` parsed), killed by `parser_edges_the_mutation_run_found`;
  the absolute weekday, unix zero without `-0`, the three `Formatted` impls, and one message per
  refusal, each with its own test. The audit added two more. `from_hm`'s `hours < 0` is **not**
  masked by the `!= 0` clauses, only its `<=` siblings are: rotted to `hours == 0` it refuses
  `from_hm(-5, -30)`, which is how -05:30 is written, and accepts `from_hm(-5, 30)` as -07:30. Every
  test used positive hours, which is one row of the guard's truth table. And the six-byte guard
  ahead of the offset is a length check, not a trailing-bytes check: rotted to `<` it refuses the
  same inputs and renames one refusal, so `+05:60x` is now pinned as `BadOffset`. Equivalent:
  `from_hm`'s two `<` to `<=` mutants (masked by the `!= 0` clauses beside them), `Writer::byte`'s capacity guard (`FMT_CAP` is two bytes over the
  longest output, so the boundary is unreachable), `Writer::offset`'s sign at zero (both format
  paths branch to `Z`/`UTC` before a zero offset can reach it), and the redundant `+ with -` at
  the offset-length guard (the `number`/`expect` helpers bounds-check behind it, so every path
  still errors identically).
- **capability** (11): rights bits, `from_bits` masking (an OR there turns undefined bits into
  defined rights), idempotent union, and `insert_at` landing in the named slot; killed by
  `rights_bits_are_the_wire_format` and `insert_at_fills_exactly_the_named_slot`.
- **clock_protocol** (10): the request wire format and the sanity window's seconds-times-a-billion
  arithmetic, killed by `the_request_word_is_the_wire_format_it_claims` and
  `the_sanity_window_is_where_it_says`. Equivalent: the CAS's `s + 1` (single-threaded blindness,
  above; nothing observes the odd window, and the sequence still advances by two) and `decide`'s `>`
  at equal timestamps (a zero step is accepted down both branches). The other seven seqlock mutants
  are **not** equivalent and are not survivors: flipping either spin guard or the reread check makes
  a single-threaded `read` or `publish` spin forever, so all seven are timeouts.
- **cred** (21): the longest legal identity and secret were never exercised end to end, the
  memory ceiling could become either a divide or an add, and only the divide was caught, because
  the test that was supposed to hold it named the ceiling as `Cost::MAX_M_KIB` on both sides of its
  own assertion: `1024 / 1024` is 1, which falls below `MIN_M_COST` and fails, but `1024 + 1024` is
  2048, which is a legal cost, so every symbolic check passed while every real policy was refused.
  The ceiling is now pinned as `1_048_576`, hand-computed, which is the nifefs lesson applied to
  a constant instead of an image. The redacting `Debug` could also be
  replaced with one that prints nothing; killed by `the_longest_identity_and_secret_are_legal`,
  `the_cost_is_what_it_says_up_to_the_real_ceiling`, `a_store_is_empty_until_it_is_not`, and
  `debug_prints_the_redaction_and_nothing_secret`. Equivalent: `MAX_P_COST`'s exact value (any
  `p` large enough to notice it already fails the `m_kib < p * 8` check first).

- **cred_proto** (6 after the `proofs::` exclusion): the request word pinned as one exact number
  with opcode `SEAL` (every prior test used opcode 1, so an `op` returning the constant 1
  passed), the smallest page the layout fits accepted at both ends, and `wipe`'s bound asserted
  from both sides. Equivalent: the two `|` to `^` mutants in `req` (the three fields are masked
  into disjoint bit ranges, and `x | y == x ^ y` whenever `x & y == 0`).
- **coremark** (2): both equivalent by arithmetic. The list tie-break's `>` to `>=` shifts an
  equal u16 past an equal u16, which is bytewise identity, so the published-CRC pin cannot see
  it; the fsm counter tops out at 256, so bits 16 and up are zero down both shift directions.

- **compositor** (66, the largest cluster): the pattern generators mixed bits no test compared
  against a known answer, so every `&` could become `|` and every shift could reverse. Killed by
  five tests: two hand-computed pixels per generator at coordinates whose bit patterns
  distinguish the operators, the surface checksum pinned to an independently computed FNV-1a
  value plus a read count, the window digest cross-checked row-major, stride and
  `MAX_SURFACE_BYTES` as exact numbers, and a zero-width rect asserted empty. Equivalent (14, audited one by one):
  min/max selections at equal operands, `|` vs `^` over disjoint masked bit fields, `intersect`'s
  early return (the arithmetic path returns EMPTY anyway), and a max-accumulate's `>` at equality.

- **nifefs** (12) and **dma_validator** (12): all 24 were real gaps, none equivalent, which
  fits both crates' role as trust-boundary parsers. Two recurring causes: layout constants with
  no independent pin (every test compared an image against the constant it was built from, so
  both sides moved together; the documented values are now hand-computed in the tests), and
  boundaries never hit exactly (a file ending exactly at the image end now round-trips; one byte
  under is truncated). dma_validator's ring tests had all used batch indices where `slot * 2`,
  `slot + 2`, `idx % 8` and `idx / 8` coincide; a batch starting at index 11 separates all four,
  with poisoned slots to catch a walk landing anywhere but the declared next. Six of the subtlest
  kills were verified by applying the mutation by hand and watching the test fail.

- **frames** (5): three real (a stuck `Some(true)` from `is_used`, `index_of` refusing the base
  frame, a zero-size `mark_used` rounding up to one frame), killed by
  `the_base_frame_and_the_empty_range_are_exact`. Equivalent: the alloc hint (an optimization; a
  scan starting on the just-used frame finds the same next free one) and `alloc_contiguous`'s
  early return (no run of zero or of more than `total` ever matches, so the scan reproduces it).

- **elf** (10): all real, none equivalent. Every fixture set PF_R, so `is_readable`'s mask could
  become any operator and the validator's execute-only branch was dead in the suite; an
  execute-only segment now asserts both sides, the header-table bounds get their exact edges, and
  `u16le` is pinned on bytes whose halves differ (every field in the old fixtures had a zero high
  byte, so reading the wrong neighbour byte read the same).
- **entropy_protocol** (3): `op` pinned with a non-GET opcode (`GET` is 1, so a body replaced by the
  constant 1 passed every round trip). Equivalent: `|` vs `^` over disjoint masked operands, and
  `want`'s `>` at `n == MAX_BYTES`, where both branches return the same 8.

- **graphics_protocol** (22): the test pattern's channel math had no pinned pixel, so a wrong buffer
  could only be wrong the same way on both sides; five hand-computed pixels, a one-bit-change
  digest test (an FNV whose xor became or collides exactly where it matters), and the errno's
  minus sign. Equivalent (7): OR-vs-XOR in `req` and the `rect` packing, where every field is
  masked into disjoint bits.
- **glob** (21): the first pass wrote both of its new tests inside `#[cfg(kani)] mod verification`
  instead of `mod tests`, so `cargo test` never compiled them and they killed nothing. The
  paragraph that claimed fourteen was describing tests that had never run, and `.cargo/mutants.toml`
  already records why: `mod verification` is invisible to `cargo test`, which is the reason mutants
  inside it are excluded. Both tests are good and both kill once moved, and moving them takes
  eleven of the twenty-one. **`script/lint` now fails on a `#[test]` inside a `#[cfg(kani)]`
  module**, because there are 22 such modules in the tree and this failure mode reports a coverage
  it does not have. The step count is the DoS-bound contract (Kani proves only that it stays under
  `cost_bound`, which a counter stuck at zero also satisfies), so each class feature now costs
  exactly its own scan; and `[A-\]]` matches at its endpoint, resumes after the escape, and a class
  ending in a bare escape is unterminated rather than a read past the end. The remaining three
  needed arithmetic nobody would guess at. `scanned += after_hi - after_lo` charges exactly the
  bytes a range skips, so a resume index landing *inside* the range re-scans one at a time what it
  should have stepped over and the ledger balances; no step assertion can see it. Membership can:
  resuming at the `-` makes `[a-c]` match a hyphen, resuming at the high end makes `[z-a]` match
  `a`, and a wider class is a larger grant. The other coincidence is `2 + 2 == 2 * 2`, because at
  the head of a one-member class `scanned` is 2 and the tail is 2 bytes, so the addition and the
  multiplication agree and so do `4 - 2` and `4 / 2`; one member in front (`[xa-c]`) separates all
  four. The 7 timeouts are all detected hangs, every one a cursor that stops advancing. No
  equivalent mutants and nothing deferred: 21 of 21.

- **dtb** (52, second largest): one recurring cause. Every test tree declared the fixtures' 2/2
  cell layout, so the `#address-cells` match arms could be deleted, inheritance could stop at the
  default, and a `reg` could decode with its own node's widths instead of its parent's, all
  invisibly. Closed in two passes, nineteen `hostile.rs` tests in all: the first eleven covered
  the 1-cell layout, inheritance, parent-vs-own decode, the compatible walker's root guard, prop
  lookup, a reservation block with entries (including one at address zero, which an `&&` rotted
  to `||` reads as the terminator), initrd's widths and exclusive end, and the header comparisons
  met exactly. **A second pass audited the first against the survivor list and found thirteen not
  actually killed**, most of them the `reserved_memory_regions` cluster nothing had ever called
  off the fixtures; eight more tests closed those, three verified by applying the mutation and
  watching the one test fail. The audit is the honest caveat on hand-triage: reasoning about
  which test kills which mutant is fallible, and the weekly rerun is the check on it. The 20
  timeouts are detected hangs, each confirmed by applying the mutation and watching the suite fail
  to finish. Fourteen are a cursor `+=` becoming `-=`, where the walker bounces between two offsets
  that both read as an empty-named `FDT_BEGIN_NODE`. The other six are a plain `+` becoming `-` in
  `let value_at = at + 8`, a different mechanism worth naming: `at` lands back on the same
  `FDT_PROP` token and the walker re-reads one property forever. Equivalent: `cells()`'s `|` vs `^`
  (ORs into freshly shifted zeros), confirmed by reading the masks and by running it.
- **fs_proto** (72 after the `proofs::` exclusion; the largest crate): two findings. Most
  survivors were equivalent, not gaps, and a later pass audited that claim **site by site rather
  than in aggregate**, because "the masks are disjoint" is a sentence about 26 different lines.
  Every request word, spec word and rights bundle ORs fields the code masks into disjoint bit ranges
  (`fs::req` splits 63:56, 55:40 and 39:0; `grant::spec` puts the length under `0xff` and the rights
  above bit 8; `xattr::spec` and `xattr::reply` split at bit 32; the six `dir` rights and the eight
  `attrs` claims are distinct single bits), so `|` and `^` agree on every reachable input. All 26
  were checked against the constants and 14 hand-mutated, one per distinct expression. Two more are
  the `verb::TABLE` order check inside a `const _: ()` block, where both mutants only make the loop
  vacuous and a `const` item has no runtime observable, so no test can reach them; the property is
  pinned at runtime by `every_verb_has_a_row_that_says_what_its_words_mean`. **The audit found one
  claim that was wrong.** `xattr::store::write_record`'s guard is not a mask, and with its second
  `||` turned into `&&` the value limit stops being enforced, because the name clause beside it is
  dead in both callers. That limit is load-bearing: `set` and `remove` re-emit records whose lengths
  come off the blob rather than from a bounds-checked caller, and a value length is a `u16` reaching
  65535 against a `MAX_VALUE` of 3072, killed by
  `a_record_wider_than_the_contract_is_refused_when_a_blob_is_rewritten`. The real gaps were the rights bundles as numbers
  (an `&` in `REMOVE_TREE` collapsed it to zero), one sentence per explained errno, the verb
  predicates as an exact partition, both dirent length limits, `pack_name` handed a
  seventeen-byte name (the mutated bound indexes past the packed word and panics), the nameset
  cursor byte for byte, the attribute record's exact bytes at an exactly sized buffer, and every
  witness verdict module pinned to distinct single bits, because both ends of a QEMU test build
  their words from the same constants and a shifted-to-zero claim silently stops being checked.

- **gpt, second pass** (18 more): the interrupted run's resume reached the modules the first pass
  never got to (entry, guid, header, span, the MBR validator), and the survivors were the same
  two shapes. Boundaries never met exactly: a header the size of its block, a partition on the
  first usable block, an exact-fit name buffer, entry sizes other than 128 (every fixture uses
  128, so the size guard had never been *judged*). And values every fixture shared: the
  protective record always in MBR slot 0, where `i * 16` is immune to arithmetic mutation, and
  every partition name ASCII, whose UTF-16 high byte is zero, which hid an off-by-one in name
  decode. One test each; `check_protective_mbr` also needed one bad block through the wrapper,
  since every existing call handed it a good one. Equivalent: the hex-nibble `|` vs `^` (disjoint
  bits), the span guard on a 64-bit host (it stops being equivalent the day anything builds for a
  32-bit target, which is exactly when you want the signal), `1 << 0` under shift direction, and `<`
  vs `<=` at the backup-reserve underflow guard (a disk that small dies on another arm of the same
  refusal). The audit found one more real gap of the first shape: `check_partitions`'
  `last_lba < first_lba` rotted to `<=` refuses a partition of exactly one block, which is legal
  because `last_lba` is inclusive, and every partition in both suites spans thousands of blocks, so
  the check had only ever been met from far away.

- **intrusive** (3): the run queue's `len` was asserted after every push and never after a pop
  (`-=` could become `+=`), and `is_empty` was only met empty; both now asserted mid-drain in
  `fifo_order`.
- **ipc** (5): the manual `PartialEq` impls were only ever compared equal, so a body stuck at
  `true` passed; distinct variants now must disagree and the hang-dump `Debug` strings are
  pinned. Equivalent: `one_queue_invariant` replaced with `true`, because the API maintains the
  invariant, so no reachable endpoint state returns false; the checker exists for kernel-side
  debug assertions whose states are built by kernel code this crate cannot construct.
