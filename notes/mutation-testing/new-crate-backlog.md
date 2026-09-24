# The new-crate backlog: a loop that waits

An appendix of [notes/mutation-testing.md](../mutation-testing.md), moved there verbatim on 2026-09-24.

## 2026-09-20: milestone 326 part 3, the new-crate backlog

Part 3 is the rest of the 1.9-point gap between the like-for-like 93.6% and the corpus 91.7%: the
26 crates that did not exist at the August baseline, of which the seven worst in the tree are all
members. The block says to take it as a worklist and not to try to close it, so this section
accounts for those seven and stops there.

Same discipline as the 2026-09-19 section above, and it is the discipline rather than the number
that makes the section worth reading: every crate was re-derived with `script/mutation -p <crate>`
on this lane's own worktree rather than read out of the census, **every kill was verified by
re-running the sweep and watching the mutant die**, and every equivalence claim below is a mutant
the second run still reports.

| crate | before | after | killed | equivalent | excluded | recorded gap |
|---|---|---|---|---|---|---|
| `work_steal_slot` | 0 | 0 | 0 | 0 | 0 | 0 |
| `memory_corruption_canary_gate` | 8 | 2 | 6 | 2 | 1 | 0 |
| `soak_page` | 7 | 0 | 7 | 0 | 0 | 0 |
| `jh7110_entropy` | 23 | 10 | 13 | 6 | 0 | 4 |
| `multicast_dns_protocol` | n/a | n/a | n/a | n/a | n/a | n/a |
| `job_mix` | 10 | 4 | 6 | 3 | 0 | 1 |
| `schedule_store` | 8 | 0 | 8 | 0 | 0 | 0 |
| **total** | **56** | **16** | **40** | **11** | **1** | **5** |

`before` and `after` count **missed plus timeouts**, which is what `script/mutation --report` lists
as "the survivors themselves". The one exclusion is a category rather than a crate's business and is
counted where it was found. Four of the six `jh7110_entropy` equivalents are one line.

**Two of the census's seven numbers were artifacts of the instrument, and the block predicted one of
them.** `work_steal_slot` read 54.2% because its loom model was counted; the `interleavings::` entry
part 1 added removes it, and the crate now reports **14 mutants, 13 caught, 1 unviable, no
survivors**, with nothing owed. `multicast_dns_protocol` (the census's `mdns_proto`, renamed by
milestone 265 (`_proto` is a truncation)) read 77.3% with 82 survivors and **is not in this tree**:
milestone 298 (retire the multicast DNS responder and its two crates) retired both of them on
2026-09-15, six weeks after the baseline and a day
after the census. Its row closes by deletion. `memory_corruption_canary_gate`, the other crate the
block flagged as suspect, was genuinely 50.0% once the loom mutants were out, which is worse than
the 66.7% it was flagged at rather than better.

### The pattern the whole part turned out to be about: a loop that waits

**Eight of the fifty-six survivors were not wrong answers, they were deadlocks**, and they are the
reason this section is ordered the way it is rather than worst-first. A spin loop is broken by
making it never accept, and a function that gathers is broken by making it never advance; neither
produces a wrong return value, because neither returns. The suite's answer to that is to hang, and
`cargo mutants` can only report a suite that did not finish as a timeout, which it cannot
distinguish from a slow one.

This file's standing reading is that such a timeout is the tests noticing rather than missing, and
that stays true. What part 3 adds is that **noticing by hanging is worth converting into noticing by
failing, where the crate lets you**, because the two are very different for the human who runs
`cargo test` and gets no output at all.

- **`memory_corruption_canary_gate`: converted, all four.** Every host test body now runs on a
  worker with a five-second deadline, so `arm` and `disarm` failing to accept is a stated assertion
  (`a spin loop never made progress, so the gate deadlocked`) instead of a hung binary. A panic
  inside the body drops the sender, so a real assertion failure and a hang are told apart rather
  than both reading as a hang. Liveness is a property this crate owes the kernel more than the host:
  `arm` spins on a core its owner cannot be preempted from.
- **`job_mix`: removed, by deleting the loop.** Its one timeout was `jobs_of_kind`'s hand-rolled
  index under `*=`, which never advances. A `for &k in &MIX` has no increment to lose, so the mutant
  does not exist rather than being caught: rung one of AGENTS.md's ladder where the old code sat at
  rung zero.
- **`jh7110_entropy`: not converted, and the reason is measured.** `Pool::take`'s four are the same
  shape, and `take_returns_rather_than_spinning` now states the property on a deadline. The
  classification does not move, because **`Pool`'s own doctest calls `take` directly**, `cargo test`
  runs doctests, and there is nowhere in a doctest to put a deadline. Confirmed by hand-applying the
  `self.cursor != self.filled` mutant and running `cargo test -p jh7110_entropy --doc`, which sat at
  `has been running for over 60 seconds`. Recorded as four gaps rather than argued away; what would
  close them is a way to tell a deadlock-timeout from a slow-test one, which cargo-mutants 27.1.0
  does not offer (its whole set of limits is the clock, per milestone 277's own check).

### The exclusion, which is measured and not assumed

`.cargo/mutants.toml` grows a third `exclude_re` entry, `tests::`, and it belongs with the two part 1
added rather than beside them by coincidence.

cargo-mutants already declines to mutate anything under a plain `#[cfg(test)]`. It does not
recognise the compound form, and the five loom crates have to write `#[cfg(all(test, not(loom)))]`
because the loom model is the other half of the same `test` cfg. So a helper function in one of
those modules is mutated where the identical helper in an ordinary crate is not, which is how
`memory_corruption_canary_gate`'s new deadline helper came back MISSED the moment it was written.

**Measured, in the form the seven-questions rule asks for**: `cargo mutants -p calendar --list`
returns 395 mutants and none of them is `tests::*`, although `calendar`'s test module has a
`fields` helper of exactly the shape that was mutated here.

A mutant of a test helper is never a defect in the shipped system, which makes this a **stronger**
exclusion than the two above it rather than a weaker one: `verification::` and `interleavings::`
at least name real code with a checker of its own, and this names code that is compiled into
nothing.

### `work_steal_slot`: no survivors, and the census's 54.2% was the loom model

**Before and after: 14 mutants, 13 caught, 1 unviable.** Nothing owed and nothing done. The block
listed this crate first and called its number suspect; the number was the `mod interleavings` block,
which part 1's exclusion removes, and what is left is a crate whose tests kill everything.

### `memory_corruption_canary_gate`: 8 survivors, 6 killed, 2 equivalent

**Before: 8 caught, 4 missed, 4 timeouts, 2 unviable (50.0% of viable). After: 14 caught, 2 missed,
2 unviable (87.5%).** The census read 66.7%; the difference is the loom mutants leaving the count.

**The four timeouts are the deadline conversion above.** They are `arm`'s two acceptance tests
(`seen == DISARMED` under `!=`, and the `||` under `&&`), `disarm`'s `seen == ARMED` under `!=`, and
deleting `ArmGuard`'s `Drop`, which strands the gate in `ARMING` so the next `disarm` never returns.

**The two real misses were both the re-arm transition, and nothing in the suite armed an already
armed gate.** Every other path reaches `arm` from `DISARMED`, and `DISARMED` is the one state where
an `arm` that returns its guard *without* winning the compare-exchange is invisible: the gate is not
`ARMED` either way, so `armed_hint` and `try_check` answer the same. From `ARMED` the same mistake
leaves the old plan readable and admits a check pass **while the plan is being rewritten**, which is
the torn plan the module documentation is about and the bug this crate exists to have fixed. Closed
by `rearming_an_armed_gate_takes_it_out_of_armed`, which kills `&&`-to-`||` outright and takes
`seen == ARMED` under `!=` into the deadline.

**The two equivalents are `pause`, and they are two functions rather than one.**

- **`#[cfg(not(loom))] fn pause` under `()` (1).** The body is `core::hint::spin_loop()`, a
  scheduling hint with no semantic effect: the loop still spins, and the protocol cannot observe
  whether the hint was issued. Equivalent by construction, which is exactly why the same function
  can be written twice under two cfgs.
- **`#[cfg(loom)] fn pause` under `()` (1).** The loom twin is `loom::thread::yield_now()`, and
  removing it is **not** equivalent under loom, where the yield is what tells the model checker the
  loop is waiting. It is unreachable rather than equivalent: `cargo test` never compiles it. The
  same object as the `interleavings::` module one level down, and it cannot be excluded the same
  way, because a free function's mutant name carries no module path and the two `pause` mutants are
  therefore indistinguishable by regex. `script/interleaving-check` is its checker.

### `soak_page`: 7 survivors, 7 killed, none left

**Before: 15 caught, 7 missed (68.2%). After: 22 caught, 0 missed (100.0%).** The whole crate is
four `const fn`s and a transform, so this one is short and the finding is sharp.

**Five of the seven were offsets that give two workers the same eight bytes**: `rounds` collapsed to
a constant, striding by one instead of eight, dividing instead of multiplying, and `mismatches` and
`wakes` counting backwards through their arrays. The two existing tests check that each offset fits
in the page and that the **ends** of each array clear the **start** of the next, and every one of
those five mutants passes both. The page's entire lock-free argument is that each `u64` has exactly
one writer; a collision is not a slow workload, it is a silent one, and the module documentation
says so two paragraphs before the functions that were wrong.

Closed by `every_slot_is_its_own_aligned_word`, which asserts distinctness and 8-alignment across
all 192 slots. **Distinctness and not values**, deliberately: the kernel and the workload both reach
the page through these functions, so any injective, aligned, in-page assignment is a correct one,
and pinning the arithmetic would test the code against itself.

**The seventh was `answer`'s `^` becoming `|`**, and it is the more interesting one. The three
existing assertions (neighbouring sequence numbers differ, no answer is zero, no answer echoes its
input) all pass under `|`, because a mask still varies and still never produces zero. What it stops
being is a **bijection**, and injectivity is the whole reason the transform exists: "a reply
carrying *some* value is not mistaken for a reply carrying the *right* value". Closed by
`the_answers_share_no_bit_in_common`, which intersects sixty-four answers: a mask leaves its own
bits standing in every output, a xor clears a bit as often as it sets one. A total tell rather than
a sampled one, and cheaper than hunting a collision.

### `schedule_store`: 8 survivors, 8 killed, none left

**Before: 29 caught, 8 missed, 5 unviable (78.4%). After: 37 caught, 0 missed (100.0%).**

**Six were a `>` that could become `>=` with nothing noticing, and they are one blind spot rather
than six.** Every refusal test in this crate hands the code a value **one past** the limit: a name
of `MAX_IDENTITY_LEN + 1` bytes, `MAX_IDENTITIES + 1` identities, a three-byte buffer for a
five-byte name. A value one past the limit is refused either way. Nothing anywhere asked what
happens **at** the limit, so a store that quietly lost the 64th byte of a name, the eighth identity,
or the last byte a buffer had room for would have passed the whole suite. Closed by
`the_last_thing_that_fits_still_fits`, through both the parse and the render half of each bound.

**The buffer case is the one that is not merely a lost feature.** The bound is
`n + name.len() + 1 > buf.len()`, where the `+ 1` is the newline that has not been written yet, and
two mutants (`-` and `*` in place of that `+`) drop the term. The check then passes on a buffer with
room for the name but not its terminator, and the next line indexes one past the end of a caller's
buffer. In a `no_std` crate the kernel links, that is a panic rather than a wrong answer.
`the_buffer_bound_counts_the_newline_it_has_not_written_yet` holds both sides: a buffer of exactly
the document's length renders, one byte less is refused.

**The other two were `Error::line`, which no test called at all.** The refusal tests compare whole
variants (`Err(Error::NameTooLong(1))`), so the accessor a caller actually reads the line through
was never exercised, and a `line` returning a constant was invisible. A configuration error pointing
at the wrong line is precisely what this crate's 1-based convention exists to prevent, and it
inherits that convention from `timetable::Error` with the reason attached ("a configuration error
nobody can find in the file is one nobody will fix"). `each_error_carries_the_line_it_is_about`
reads it on line 3 and line 9, neither of which is a constant a mutant would reach for.
