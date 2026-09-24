# The `rfence_self` baseline row

*An appendix to [`notes/benchmarks.md`](../benchmarks.md), which carries the current numbers and is written so a reader can act without opening this file. This one holds the 2026-09-21 and 2026-09-23 readings of one oscillating riscv64 row, with the dates, tables and corrections behind them. Name provisional (`notes/benchmarks/` and this stem), minted 2026-09-24 by the lane that split the note; naming is calef's.*

## 2026-09-21: `rfence_self` did not get 7.5% faster, its baseline row got 7.5% wrong

**Contested since, and the later section is the current state.** The row went back to 6476 on
2026-09-22 and measured 5991 again on 2026-09-23, where a second lane read the same movement as
compiler drift rather than as a wrong hart count. Read this section for what was checked, and the
2026-09-23 section below for why neither reading can be settled from the file.

calef asked for the fast benchmark to be chased the way a regression would be: on 2026-09-21
riscv64's `rfence_self` measured **7.49% under** its committed baseline while every other row on
every architecture sat inside noise. An unexplained improvement is the mirror image of an
unexplained regression, and the icount tripwire is one-sided by design, so nothing was ever going
to raise it.

**The answer is neither of the two interesting ones.** The tree did not get faster and the
benchmark did not stop measuring what it measures. The committed number was wrong on the day it
was committed.

### The measurement

`rfence_self` measured **5991** ticks, and 5991 is exactly the value the baseline held before
commit `28e165e2`, which raised it to 6476. `6476 - 5991 = 485`, and `485 / 6476 = 7.489%`: the
whole reported improvement is that one row returning to a number it had never left.

Run at the commit that wrote 6476, and at that commit's parent, the tree measures **5991** both
times. Every other riscv64 row at `28e165e2` reproduces what `28e165e2` recorded, several of them
to the digit (`spawn_reap 34277`, `ctx_switch 504958`, `spawn_el0 203942`). So the file came from a
real run of that tree and one row in it did not.

The number is not fragile in the ways worth ruling out, each checked rather than assumed:

| checked | result |
|---|---|
| repeat runs, same build | byte-identical full tables, 3 of 3 |
| clean `cargo clean -p kernel` rebuilds | 5991 both times |
| six spinning host processes competing for cores | 5991 (`-icount shift=0,sleep=off` is load-immune, as intended) |
| the commit that recorded 6476, and its parent | 5991 and 5991 |

6476 appears nowhere else in the tree: not in the block for milestone 447 (a thread's vector
registers are its own), which that commit re-measured, not in this note, not in any commit message.
It is an orphan.

### What did reproduce, and it is the one thing worth keeping

`rfence_self` is **sensitive to how many harts are online**, even though it fences a mask naming
only the calling hart:

| harts online | `rfence_self` ticks |
|---:|---:|
| 1 (what `--riscv` boots) | 5991 |
| 2 | 6432 |
| 4 | 7153 |

Each of those is itself deterministic across runs. The committed 6476 sits 0.7% above the two-hart
value and nowhere near the one-hart value, which is suggestive of a run made at the wrong hart count
and is not proof of one. `xtask/src/bench.rs` sets `NIFE_SMP=1` unconditionally on this leg, so the
only way to get there today is to edit that line, which is how the table above was produced.

**Why the sensitivity exists at all**: the mask names the caller, so nothing is IPI'd, but OpenSBI's
TLB path is not the same code on a platform with other harts running. The kernel side is identical
in all three rows.

### Does the fence still fence?

Yes, and nothing in the window touched it. `kernel/src/arch/riscv64/mmu.rs` has the two call sites
that matter and both are unchanged: `flush_tlb` executes a local `sfence.vma` and *then*
`sbi_remote_sfence_vma` against `online_harts_mask() & !(1 << cpu::id())`, and `flush_asid` does the
same with `sbi_remote_sfence_vma_asid`. Each skips the firmware call only when that mask is empty,
which on a one-hart boot is the correct answer and not a shortcut.

**The caveat a reader of this number needs, and it is the honest limit of the benchmark:**
`rfence_self` passes `me`, its *own* hart, precisely so the cost is "getting into firmware and back"
rather than a shootdown. On the single-hart boot `--riscv` uses, the production call sites issue
**zero** RFENCEs, which the suite's own probe prints (`map_new_remote_fences 0 over 64 iters`). So
this row prices a call the running kernel never makes in that configuration. It is a useful floor
for what an RFENCE costs; it is not the cost of a shootdown, and a change to the real shootdown path
would not move it.

### The tripwire consequence, which is why this was worth fixing rather than noting

`--check`'s slack is `base / 10`. A baseline of 6476 gives the row 647 ticks of headroom in both
directions around a true value of 5991, so the row could regress by **1132 ticks, 19% of its real
cost**, before the gate fired. The corrected row restores ~600 ticks of slack around the number the
tree actually produces.

### What was re-saved, and how

The single line `rfence_self 6476 512` was hand-edited to `rfence_self 5991 512`. **Not
`--save`**, which rewrites the whole file from one run and would have re-recorded fifteen rows this
lane did not measure for and has no mandate to move; other lanes are working on exactly that
question.

A `#` comment was put above the row saying what it is. **That comment will not survive the next
`--save`**, because `run_bench`'s save path builds the file from a fixed header plus one
`name ticks iters` line per result and preserves nothing else. That is a foot gun and this note is
where the fact lives instead; it is also a concrete second argument for item 2 of
milestone 415 (sub-tripwire drift accumulates across baseline saves), whose §190 (must a baseline
save record why it moved) asks whether a save should be obliged to record a reason. A row that
cannot carry a reason cannot carry a correction either.

### One stale comment found in passing

`xtask/src/bench.rs`'s check loop is preceded by a comment claiming indented `#` lines "are treated
as data" by "a column-0-only check". The filter it sits above is
`.filter(|l| !l.trim_start().starts_with('#'))`, which handles indented comments correctly. The
comment describes a hazard the code already fixed. Left as a finding rather than edited, because
`xtask/src/bench.rs` is shared bench scaffolding another lane is in.

## 2026-09-23: `rfence_self` has now been read two ways, and the file cannot settle it

The 2026-09-21 section above concluded that `rfence_self`'s committed 6476 "was wrong on the day it
was committed", read it as a run made at the wrong hart count, and hand-edited the row back to 5991.
Four days later the row is 6476 again, and the lane clearing PR #1112 (the `nightly-2026-09-23` bump)
measured **5991 to the tick in three reproducible runs** and read the same movement as **compiler
drift**.

**Both readings are argued from outside the file, because there is nothing inside it to argue with.**
`bench/baseline-riscv64.txt` records a number, a pinned nightly and a QEMU version. It does not
record why the number holds that value, so neither lane could check the other's cause without
re-deriving it from commit messages and git history.

### The oscillation, from committed text alone

No emulator was booted for this table. It is `git show <commit>:bench/baseline-riscv64.txt` seven
times:

| commit | date | `# toolchain:` | `rfence_self` |
|---|---|---|---:|
| `e6833cd2` | 2026-09-20 | (stamp not yet added) | 5991 |
| `28e165e2` | 2026-09-20 | (stamp not yet added) | 6476 |
| `99f13dad` | 2026-09-21 | nightly-2026-09-20 | 6476 |
| `84e29394` | 2026-09-21 | (hand edit, no re-save) | 5991 |
| `d14ad408` | 2026-09-21 | nightly-2026-09-20 | 6476 |
| `483382c9` | 2026-09-21 | nightly-2026-09-20 | 5991 |
| `c64af73e` | 2026-09-22 | nightly-2026-09-22 | 6476 |

The row alternates between two values across saves made under two different pins, with a hand
correction in the middle that a later `--save` rewrote. **Which cause is right is open**, and
deciding it needs an A/B across two nightlies at a fixed hart count, which nobody has run. Neither
reading is a reason to move the floor: `script/bench --save` commits a performance floor and is
calef's call.

### `spawn_el0` under `nightly-2026-09-23`

Measured by the same lane: up **~6% on both aarch64 and riscv64**, bit-identical across repeats,
under the 10% tripwire. Bit-identical repeats say it is the compiler and not the host, and being
under the tripwire says nothing will stop it being folded into the next floor.

### Why this is recorded here rather than proposed as work

The mechanism that would fix it already exists twice: milestone 302 (a baseline records what it was
saved against, and a stale one fails loudly) asks `--save` to write a `# why:` line, calef having
ratified that format on 2026-09-16, and item 2 of milestone 415 (sub-tripwire drift accumulates
across baseline saves) is the same obligation stated once more, with
[§190](../design/decisions/190-what-a-baseline-save-must-record.md) (must an icount baseline save
record why it moved) as its decision. Milestone 577 (the icount baselines predate the pinned
nightly) was promoted and superseded in one act on 2026-09-23 rather than become a third statement
of it, and this section is where its evidence lives.

`script/lint`'s baseline-toolchain check, added by `99f13dad`, is the half that did land, and it
fired on PR #1112 exactly as designed. It proves a floor was saved under the pinned nightly. It
cannot prove the numbers under that stamp are right, and this section is what that gap looks like
from the reader's side.
