# `measured_boot`: five survivors proved equivalent

An appendix of [notes/mutation-testing.md](../mutation-testing.md), moved there verbatim on 2026-09-24.

## 2026-09-03: `measured_boot` re-run, and its five survivors proved equivalent rather than argued

Milestone 246 moved measured boot's load-or-refuse decision into this crate, so the crate was re-run
whole: **151 mutants, 133 caught, 5 missed, 4 timeouts, 9 unviable.**

**The new function is one mutant and it is caught.** `verdict` is the decision that says whether
unmeasured code may run, and `cargo mutants`' only operator on a function returning a struct is to
replace the body with `Default::default()`. Without a `Default` impl that does not compile, so the
first run scored it **unviable** and the tool said nothing at all about the line. `Verdict` now
derives `Default`, which is both the fail-safe value and exactly the dangerous wrong answer here (an
absence where there was a refusal); `cargo mutants --in-diff` over that lane's diff went from
*1 unviable, 0 tested* to *1 caught*. **A crate can score 100% on a function the tool never
mutated**, which is the general lesson: an unviable mutant is a hole in the measurement, not a pass.

**The five missed are provably equivalent**, which upgrades this crate's row in the ledger above from
"argued" to proved. The section on scope says an equivalence verdict reached by reading is wrong
about ten percent of the time; these five are algebraic identities rather than readings:

- `Sha256::compress`, `ch = (e & f) ^ ((!e) & g)`, `^` to `|`. The two operands are disjoint by
  construction (`e` and `!e` cannot both be set in a bit position), and `^` and `|` agree wherever
  the operands never both hold.
- `Sha256::compress`, `maj = (a & b) ^ (a & c) ^ (b & c)`, `^` to `|`, twice. Count the set inputs in
  one bit position: at 0 or 1 every pair-AND is 0, at 2 exactly one is 1, at 3 all three are 1. XOR
  and OR agree on all four counts (0, 0, 1, 1), which is why both spellings of `maj` compute the
  bit-majority.
- `parse_hex`, `(hi << 4) | lo`, `|` to `^`. `lo` is a nibble and `hi << 4` has its low four bits
  clear, so the operands are disjoint and the two operators agree.
- `Sha256::update`, `bytes.len() < want` to `<=`. In the equal case both arms yield `want`.

**No test can kill any of the five**, so nothing here is deferred and nothing is a gap. The four
timeouts are `update`'s loop-control mutants, which hang rather than lie, and are the same family
this note records elsewhere.
