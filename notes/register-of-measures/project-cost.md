# What this project costs

*An appendix to [the register of measures](../register-of-measures.md). The name is provisional.*

## What this project costs

**Three units, and deliberately no fourth that adds them up.** Milestone 519 (what this project
costs, tracked where it cannot rot) refuses a single headline number, because three audiences want
three different ones: person-weeks for the comparison against the verification literature, tokens
with a dated price for the argument that the method is getting cheaper, and cash for calef's own
budgeting. The columns are `human_person_weeks`, `lane_tokens` with `lane_wall_clock_hours` and
`price_per_mtok_at_date`, and `cash_spend`.

**The shape those numbers make is the finding.** Cash to date is under a thousand dollars all in,
against eleven person-weeks of one experienced engineer's full attention (milestone 519's block
says "about ten", written two days before this column existed; the column counts eleven calendar
weeks, 2026W29 through the current partial 2026W39, and the difference is arithmetic rather than a
correction). At any plausible rate
for that person's time, **the human cost is on the order of 99% of the economic cost and the machines
are a rounding error.** The claim this project can honestly make is not that software became cheap.
It is that the scarce input is still a person, and what changed is how much one person's attention
can be made to carry.

### The chart is a ratio, and it is the one that answers the claim

A total would say the project was busy. Millions of tokens per milestone built says whether the
method got **cheaper**, which is the claim principle 2 of `AGENTS.md` actually makes. It fell from
307 in 2026W34 to 73 and 68 in 2026W36 and 2026W38. Four things before anyone quotes that:

- **A milestone is not a fixed unit**, which the velocity section says at length and which this
  inherits with interest, because here it is the denominator. A week that built one build-flag fix
  and a week that built a three-architecture bring-up divide by the same 1.
- **2026W35's 1,021 is the highest and its week built 8 milestones**, against 2026W36's 49. Both
  numbers are real. Whether that week was expensive or merely undeclared is not answerable from here,
  and the pull-request series above is the first place to look.
- **The current week is always understated.** Its tokens accumulate all week and its milestones land
  in a burst near the end.
- **Four weeks are marked as not captured rather than drawn as zero.** See the deadline below.

### The deadline, and what was already gone when the capture started

Every other column on this page is computed from a git revision, so history backfills by reading the
tree at a past commit: that is how the mutation census, the velocity column and the coverage cells
were all filled in retroactively. **Token and wall-clock records are not in git.** They live in the
agent harness's session records under `~/.claude/projects/`, on one laptop, outside this repository,
and nothing promises to keep them.

The first capture ran on **2026-09-21** and read 477 record streams. It found **2026W34 through
2026W39**, and found **nothing at all for 2026W29 through 2026W33**, which is the first five weeks of
the project against a first commit on 2026-07-12. Those weeks are recorded as **absent, not zero**,
in the CSV and on the chart, because an absent week averaged in as a zero is a lie that ends up in a
published figure.

**2026W37 is captured and should still be treated as suspect.** It reads 12.2 machine-hours against
110 to 133 on either side, in a week whose roadmap says 13 milestones were built. Either that week
really was quiet, or its records were rotated away before anyone looked. Nothing can tell those apart
after the fact, and that is the argument for the snapshot described under *How it stays current*.

### What `lane_wall_clock_hours` is, and what it conflates

The sum, over every record stream, of the gap between consecutive responses, dropping any gap longer
than thirty minutes. A stream is one session or one subagent lane, so **parallel lanes add**: four
lanes working for an hour is four hours here, and a week can exceed 168. That is the point, since it
is an effort figure rather than an elapsed one.

It **conflates queueing with work**. A lane waiting on the merge queue, on a gate, or on a rate limit
is indistinguishable here from one thinking. And the thirty-minute cut is a judgement, not a
measurement: a longer gap is a session left open overnight, and counting it would put a sleeping
laptop in the total. `IDLE_GAP_S=600 script/effort` shows how much the choice moves.

### Two dollars, and which one is which

This is the refusal milestone 519 is sharpest about. **This project pays a fixed monthly
subscription, so the marginal cost of one more lane is zero dollars.** `cash_spend` is what was
actually paid: $200 a month since 2026-07-12, plus $271.81 of hardware on 2026-08-15.

`price_per_mtok_at_date` is a **shadow price**, and it is a rate rather than a total on purpose. It
is that week's own token mix priced at vendor list rates, so a reader who wants the shadow figure
multiplies it by `lane_tokens` and knows exactly what they have multiplied. Quoting one of these two
numbers while implying the other is the dishonest version, so neither ever appears here without its
label. Both rates and purchases live in
[`notes/project-metrics/ledger.md`](../project-metrics/ledger.md), which is appended to by hand and
never regenerated.

**The blended rate moves with the mix, not only with prices.** It runs from $0.30 to $0.78 per
million tokens across the captured weeks while no published price changed, because this tree's work
is split roughly half and half between `claude-opus-5` and `claude-sonnet-5` by token, and because a
cache read lists at a tenth of an input token. A week that cached well reads cheap.

**$200 of real spend is outside this series and that is not rounding.** calef dates the subscription
2026-07-12, a Sunday, which is 2026W28; the first commit is 2026-07-13 UTC and the series starts at
2026W29. A week gets a row only when a commit fell in it, so `cash_spend` sums to $671.81 against
$871.81 actually paid. `script/metrics` prints the discrepancy on every run rather than folding it
into a neighbouring week.

### `human_person_weeks` is a statement, and will stay one

1.0 per calendar week, because calef works on nife full time. It is not a measurement and nothing
will make it one without time tracking, which milestone 519 refuses on the grounds that a person
required to log hours stops volunteering the honest ones, and the metric is then worth less than what
it displaced. The only thing ever asked of him is a **correction** when the datum stops being true,
and the column is carried through every rewrite so that a hand-edited week survives the next
backfill. It is accurate to the bucket the comparisons need (seL4 at about eleven person-years plus
nine more for the proof; Atmosphere at 1.5 person-years on verification) and no better.

## What a turn costs

**A turn costs roughly the size of its context, not the size of its thought.** Measured on
2026-09-24 across every session record this machine holds: **cache reads are about 98% of all tokens
spent**, cache creation about 1%, uncached input effectively 0%, and **output about 0.1%**. The mean
prompt per request runs between 230,000 and 359,000 tokens across the captured weeks, and the
largest single request in every one of them sits between 933,000 and 1,000,000 against a 1M window.

**That is not a claim that 98% is waste, and reading it that way would be wrong.** Carrying context
is what makes a long session coherent: it is why a lane can be told a hazard once, why a maintainer
can resolve a conflict without re-reading the tree, and why the method in `AGENTS.md` principle 2
works at all. The finding is about **where the bill goes**, not about whether the spending buys
anything. What it does say is that the lever everyone reaches for first is the wrong one: choosing a
cheaper model prices the output, and output is a tenth of one per cent of the tokens.

**The columns are flows and a peak, never a stock.** `lane_turns`, `lane_cache_read_share_pct` and
`lane_context_per_turn_mean` are per-week events over that week's requests;
`lane_context_per_turn_peak` is the largest single request in the week and is a maximum rather than a
sum, so adding two of them would invent a request nobody made. This page has already paid for
confusing the two, one series up: *"this chart does not reconcile with the `Built` stock, and that is
the design"* above is the same distinction, found the hard way on 2026-09-23 by diffing a snapshot
between two rows and getting a different answer from the flow. The rule is not re-argued here; it is
cited.

## Why the mean is charted, and what happened to the median and the peak

**Peak is measured and not drawn, because it saturates.** Every captured week peaks within 7% of the
model's 1M context window. That answers "did a session run to the wall this week" (yes, every week),
which is worth knowing and is worth exactly one bit; a chart of it would be a flat line read as a
trend. It stays in `notes/project-metrics/context-per-turn.csv` and in `effort.csv`, where a week that fell
well short would be visible as news.

**Median was refused, and the reason is the instrument rather than taste.** `script/effort` survives
a pruned transcript by merging every column as a maximum against an older machine-local snapshot,
which works because a sum and a peak both only go up as more of a week is seen. A median does not
combine that way: two honest partial views of a week cannot produce the week's median without the
per-request numbers underneath, and those are precisely what gets discarded. Storing one would mean
keeping a per-request history, which is a much larger promise than this measurement has earned.

**So the mean is the series.** It is arithmetic over columns the file already carries (input plus
cache write plus cache read, over `requests`), which is why it is derived at write time rather than
stored twice.

## This panel is the checkpoint, and that is the design rather than a side effect

calef asked on 2026-09-24: *"How do we set a checkpoint to re-evaluate that will not be forgotten?"*
The question was about a decision being deliberately deferred, which is **whether to impose a session
length limit on the agent harness, and at what threshold**. No threshold has been set, and
`AGENTS.md`'s *measure first, then decide* is the reason: a threshold chosen before the data exists
is no better than one chosen under attachment, and one measurement is not a distribution.

A checkpoint that lives in a calendar reminder or a conversation is in exactly the medium this
project keeps abolishing. **This panel is the mechanism instead: nobody has to remember to look,
because the number arrives in front of whoever reads this page.** Each week it either moves or it
does not, and a reader who has never heard of the deferred decision still sees the quantity it
depends on.

The decision itself has a home with a trigger, in `design/roadmap/proposals/`, and the trigger is
stated as *enough weeks to show a distribution, judged by reading these bars* rather than as a week
count, because a count is the same threshold-chosen-in-ignorance the tenet refuses.

## This instrument has the same deadline as the rest of the cost section

Stated here rather than only in the proposal, because a reader meets the number here.

- **These records are not in git.** They live under `~/.claude/projects/` on one laptop and nothing
  promises to keep them. `script/effort`'s header records **2026W29 through 2026W33 as
  unrecoverable**, which is the first five weeks of this project already gone, and a per-turn context
  history has exactly the same deadline.
- **It cannot backfill.** Every other column on this page is computed from a git revision. These
  four cannot be, so a week nobody captured is a week nobody will ever capture.
- **It measures one machine.** With a second contributor, or work done anywhere but patagonia, this
  is a sample of one workstation presented as a project figure. It is honest today because there is
  one machine; it stops being honest silently.
- **`--snapshot` is what makes forgetting survivable** and it is rung two of `AGENTS.md`'s ladder,
  not rung four. The plist is in "How it stays current" below.
