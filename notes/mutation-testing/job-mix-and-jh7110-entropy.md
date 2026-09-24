# `job_mix` and `jh7110_entropy`

An appendix of [notes/mutation-testing.md](../mutation-testing.md), moved there verbatim on 2026-09-24.

### `job_mix`: 10 survivors, 6 killed, 3 equivalent, 1 recorded gap

**Before: 41 caught, 9 missed, 1 timeout, 3 unviable (80.4% of viable). After: 38 caught, 4 missed,
7 unviable (90.5%).** The caught count falls because four of the survivors became **unviable**, and
that is the result rather than an accounting artifact: they no longer compile.

**Five of the ten sat in a gap a compile-time assertion had left open**, which is the finding here.
`TASK_BUDGET_PAGES` is `1 + 8 + max(MAP_REGION_PAGES, CHILD_PAGES)`, and the file already carried
`const _: () = assert!(TASK_BUDGET_PAGES > MAP_REGION_PAGES && TASK_BUDGET_PAGES > CHILD_PAGES);`
with a comment citing AGENTS.md's ladder. The assertion is true of 19, of 24 and of 144, so every
one of the five mutants compiled and passed: `1 * 8` for `1 + 8`, `9 * 16` for `9 + 16`, and three
ways of picking the **smaller** region, which leaves a task running `MAP` nine pages short of what
the paragraph above the constant promises it.

**The rung was right and the assertion was too weak**, which is worth separating because the
temptation is to read this as an argument for a test. It is not: the two bounds the sizing argument
actually makes are still relations between constants in one file. They are now written as such, at
least `1 + 8 + MAP_REGION_PAGES`, at least `1 + 8 + CHILD_PAGES`, and strictly less than
`1 + 8 + MAP_REGION_PAGES + CHILD_PAGES`, the last because a task runs one job at a time and gives
its region back. Four mutants stopped compiling.

**Milestone 250 (an unviable mutant is a hole in the measurement that reads as a pass) applies
here and is named rather than dodged**: so moving four mutants from MISSED to unviable improves the rate partly by
shrinking the denominator. What makes it the right move here is that the thing which now refuses
them is the compiler, which is a stronger checker than a test and runs on every build.

**`order` keeps three survivors and gains one test**, and the test is about the draw rather than the
output. The function takes the LCG's **high** bits because the low ones cycle short; taking the low
ones makes every draw a multiple of 2^33, which is zero modulo every power-of-two swap index, so the
last position receives the mix's first job on every seed. Nothing else in the suite noticed:
`an_order_is_a_permutation_of_the_mix` still holds, and
`distinct_seeds_do_not_all_walk_the_mix_in_lockstep` still counts 90-odd differing orders.
`no_position_in_the_order_is_pinned_to_one_job` sweeps 512 seeds and asks that every job kind reach
every position.

**The three that remain.**

- **`while i > 1` under `>= 1` (equivalent).** The extra pass has `i` at 0, so `j` is
  `x % 1`, which is 0, and the swap is `out.swap(0, 0)`. One more LCG step on a value nothing reads
  afterwards, and a self-swap.
- **`MAP_REGION_PAGES > CHILD_PAGES` under `>=` (equivalent).** `if a > b { a } else { b }` and
  `if a >= b { a } else { b }` differ only where `a == b`, and there both branches yield the same
  value. Equivalent for every pair of operands, not merely for 16 and 10.
- **`% (i + 1)` under `% i` (equivalent in contract, and this one is an argument rather than a
  proof).** The mutant is Sattolo's algorithm: it still produces a permutation, still
  deterministically from the seed, and still puts no two tasks in lockstep, which is the whole of
  what `order` is documented to promise. What it loses is uniformity over all `16!` permutations,
  drawing from the `15!` cyclic ones instead, and no task in this benchmark can tell. **Re-check it
  if `order` is ever asked for a uniform shuffle**, at which point it is a defect and not an
  equivalence.

**The recorded gap is a finding about the code, not about the tests**, and it is in a `BUGS` section
on `order` where a reader meets the function. `seed | 1` exists to keep the LCG off its degenerate
state, and it does that by discarding bit zero, so **`order(2k)` and `order(2k + 1)` are the same
permutation for every `k`**. Measured, by asserting it for eight pairs. Nothing in the tree hits it,
because `kernel/src/job_mix.rs` hands every task "a distinct, odd, well-spread seed" and an odd seed
passes through `| 1` unchanged; a caller seeding tasks by consecutive index would put every adjacent
pair in lockstep, which is the one property this function exists to prevent. It is recorded rather
than fixed because changing the arithmetic changes every permutation this function has ever
produced, and a published benchmark number is a fact that has left the machine.

### `jh7110_entropy`: 23 survivors, 13 killed, 6 equivalent, 4 recorded gaps

**Before: 59 caught, 19 missed, 4 timeouts, 3 unviable (72.0% of viable). After: 72 caught, 6
missed, 4 timeouts (87.8%).**

**Fifteen of the nineteen were a `<<` becoming a `>>` in a register bit constant**, which takes a
named bit to **zero**. At that point `stat & STAT_SEEDED` is false for every status word the device
will ever present, and every existing test still passes, because the tests sample `interpret`'s
verdicts and never name the bit they are exercising. Thirteen died to one new test.

The test is a transcription check and says so in its own doc comment. These constants came off the
TRM's register tables by hand, one line at a time, so the failure to guard against is not a clever
one: it is a bit copied to the wrong line, which reads as a device permanently in mission mode, or
never seeded, or reporting a lockup the silicon never raised. **The distinctness half is the part a
reader should weigh more**, because two constants twenty lines apart, both `1 << 3`, look correct
individually and there is no other check in the tree that would catch it.

**The six that remain are all one arithmetic fact each, and all six are equivalent.**

- **`IE_RAND_RDY_EN` and `ISTAT_RAND_RDY`, both `1 << 0`, under `>>` (2).** `1 >> 0` is `1`. The
  degenerate shift this file's patterns section already names, in the one crate where two constants
  happen to sit at bit zero.
- **`ISTAT_ALL`'s four `|` under `^` (4).** The five bits it unions are bits 0 through 4 of `ISTAT`
  and no two are the same bit, so on disjoint operands `|` and `^` are the same function. The
  argument is the same one `capability::note_peak` carries above, and because an argument a reader
  has to take on trust is worse than one they can run,
  `the_clear_mask_is_every_named_istat_bit_and_nothing_else` now asserts the disjointness the claim
  rests on: five bits set, each named constant present.

**The four timeouts are `Pool::take`'s gather loop and stay recorded gaps**, for the doctest reason
given at the top of this section. They are a `refill` that reports success without filling, `got < n`
widened to `got <= n`, the spent-buffer test inverted, and `got += run` turned into a multiply; each
leaves a pass of the loop that makes no progress, and none of them returns at all.
`take_returns_rather_than_spinning` states the property on a five-second deadline, which is worth
having for the human who runs `cargo test`, and does not move the classification.
