# The gated, dated and owed rows, argued

*An appendix to [the register of measures](../register-of-measures.md). The name is provisional.*

The ceilings are rows 2 through 5 and 7 and 8, and reading their thresholds together is the
useful part, because they are six different kinds of number. 5% is a tolerance. 4,096 is a hardware
fact. 24,576 is a configuration constant. The high-water limits are margins over an observed
maximum. 17 is today's tree exactly. And 94 per 10,000 (lowered from 100, then 97, then 96, then
95, then 94, by milestone 139; round 5 reduced the block count further but left the truncated
density and therefore the ceiling unchanged, see notes/unsafe-obligations.md) is a **claim about
the tree that was false until shortly before it was written**, which makes it the
only one that expresses a direction rather than a limit. That distinction is what `count-at-most`
exists for; see notes/unsafe-obligations.md for the measurement behind it.

Two of these are the register doing its job on itself: the unsafe rows did not exist when milestone
134 opened, and the `unsafe fn` count that would have been a third turned out to be **already
derived** by `script/lint`'s `==> unsafe fn contracts` check. Finding a number already tracked is as
much a result as finding one that is not.

**Row 7's density mixes kernel and userspace unsafe into one population, and nothing here gates the
split.** `script/metrics` (the unsafe-census-by-trust-boundary milestone, provisional) now tracks
`unsafe_trust_kernel`/`unsafe_trust_userspace` alongside it, weekly, with no ceiling of their own:
see notes/project-metrics.md's "The same unsafe blocks, by trust boundary" for the numbers, why row
7's 94 answers a different question than either half, and the recommendation on which one a future
ceiling belongs on.

**The milestone 168 row was the register's first `dated` row that had never been taken**, until
five boots of radon on 2026-09-16. Those boots measured the curve's shape (a knee near four tasks,
then a plateau with no decline through 32) with an instrument that kept the best of three repeats,
and showed that `tasks=4` under that rule was not a number: 29.4% across boots of one image. On
2026-09-19 the instrument changed to the median of 21 repeats and gained a page-mapping job and a
process-creation job, so **the 2026-09-16 date is a date for a different instrument**, and the row
says so rather than letting the date imply the current one has been taken. The next radon evening
re-dates it, and its result is what `design/fatal-risks.md`'s risk 4 and
`design/decisions/96-process-kernel-or-event-kernel.md` are waiting for.

**The filesystem row is the one on the customer path**, and it is the clearest case in the register
for why `dated` is a finding rather than a filing. Milestone 55 is a Time Machine target the
family's Macs back up to. A three-times regression in sequential write would show up as a backup
that used to finish overnight and now does not, reported by a person rather than by CI, and nothing
in this tree would have said a word. It is `dated` because taking it needs a boot with a disk
attached, which is not a thing to put on every push; the honest promotion is a scheduled run rather
than a gate, and it wants a lane.

**The cross-OS row is the register earning its keep on its first pass.** Its section in
notes/benchmarks/cross-os-primitives.md, "The first cross-OS numbers (nife vs Linux vs macOS)", **carried no date
until 2026-09-24**, and the numbers in it are the ones a stranger is most likely to quote back at us: they are
the comparison against Linux and macOS. A dated measurement with no date is a `gated` row's opposite
and a `dated` row's failure mode at once, and nothing in this tree would have said so. Dating it
means re-taking it, because nobody now knows which run it was; that is a small lane and it is named
in this milestone's handoff.

**The arch row is the odd one and it is deliberate.** There is no ceiling on unsafe inside
`kernel/src/arch/`, because driving that number down means either writing assembly wrong or moving
it out of `arch/`, and rule 1 says arch code belongs there. A target would be a gate pushing against
the architecture. But an unmarked number in a note is exactly the snapshot this whole register is
against, so `script/lint` prints it on every run: on screen every build, asserted never. **A number
with a consumer gets a relation; a number with only a reader gets printed.**

**Which of these calef's `PMCCFILTR_EL0` ruling touches:** M5 and M9 on aarch64, and M12 (seL4's
413 and 426 are TX1 cycle counts, so the comparison is exactly the number the filter decides). M6
to M8 count events rather than cycles, and `PMCCFILTR_EL0` filters only the cycle counter; each event
counter has its own filter in `PMEVTYPER<n>_EL0`, which will raise the same question when a driver
first writes one.

They are **not duplicated into this table**, because they already have a home that carries each
one's instrument, its prediction, and what its outcome settles:
design/roadmap/134-the-measurements-that-decide.md. Two open kernel decisions were waiting on the
Tier A half of them, and the block's own correction is worth knowing before anyone reaches for
hardware: §95 and §96 both recommend waiting for the TX1, and **both over-gated**, because the
experiments that produce a verdict need no silicon. Tier A's results are summarized above; Tier B
remains genuinely gated on the counters and the board.
