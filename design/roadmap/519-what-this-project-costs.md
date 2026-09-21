# 519. What this project costs, tracked where it cannot rot

**Status: NOT-STARTED.** Minted 2026-09-21 by calef, from a conversation about what nife would have
to be to change anyone else's behaviour. Every honest answer began with a cost number this tree
cannot state. *(Number provisional until the merge queue lands it.)*

**Gate: NONE.** Everything it needs is either already produced or already known.

## Why this is a milestone and not a note

`AGENTS.md`'s second principle says the method is a result. A method's result is a **cost**, and this
tree measures only **scale**: milestones, lines, harnesses, proofs, and since 2026-09-20 a velocity
chart. None of it answers what an outsider asks, which is what a trustworthy system costs to build
this way.

The literature this project is compared against is denominated in exactly that unit. seL4 reports
about eleven person-years plus nine more at a 20:1 proof-to-code ratio; Atmosphere reports 1.5
person-years on verification alone at 3.32:1. Those numbers move institutions in a way a feature
list does not.

**And the inputs are known today and unrecoverable tomorrow**, which is what makes this urgent rather
than tidy. See the deadline below.

## What is known, as of 2026-09-21

| input | figure | source |
|---|---|---|
| Human effort | **about ten person-weeks** | calef works on nife full time, so one calendar week is one person-week. His own statement, 2026-09-21 |
| Inference | **$200 a month since 2026-07-12**, about $470 to date | calef, 2026-09-21 |
| Hardware | **$89.99** for argon, the Jetson TX1 | milestone 127 (the seL4 machine: a Jetson TX1)'s own record |
| Other hardware | unpriced in this tree | radon, the UART adapters and the smart plugs; xenon was not bought for this project |
| Machine effort | per-lane tokens, tool calls and wall clock | the session's task records, which this tree does not keep |

**The shape those numbers make is the finding, and it should be stated before anyone optimises the
wrong term.** Cash to date is under a thousand dollars all in, against about ten person-weeks of one
experienced engineer's full attention. At any plausible rate for that person's time, **the human cost
is on the order of 99% of the economic cost and the machines are a rounding error.** The claim this
project can honestly make is not that software became cheap; it is that the scarce input is still a
person, and what changed is how much one person's attention can be made to carry.

## The deadline, which decides the sequencing

Every other column in `notes/project-metrics/weekly.csv` is computed from a git revision, so history
backfills by reading the tree at a past commit: that is how the mutation census and the velocity
column were both filled in retroactively.

**Token and wall-clock records are not in git.** They live in session task records outside the
repository and they are discarded. The ten weeks behind us are already unrecoverable, and every week
that passes without capture is another one lost. **The small version now beats the complete version
later**, because this week's numbers exist today and will not exist next week.

## What to build

1. **Columns in the weekly series**, in `script/metrics`, because the deck exists and its page already
   has the conventions for being honest about its own numbers:

   | column | source | note |
   |---|---|---|
   | `human_person_weeks` | 1.0 per calendar week, by calef's standing datum | corrected by hand when it stops being true |
   | `lane_tokens` | task records, summed per week | the physical quantity |
   | `lane_wall_clock_hours` | task records | conflates queueing; say so |
   | `price_per_mtok_at_date` | recorded by hand when it changes | what makes dollars re-derivable |
   | `cash_spend` | subscription plus dated purchases | what was actually paid |

2. **A ledger for what is not weekly**: hardware purchases with dates and prices, and the
   subscription's start date and rate. One committed file, appended to, never regenerated.

3. **One chart, and it is a ratio rather than a total.** Machine effort per milestone built, beside
   the velocity chart. A total says the project is busy; a ratio says whether the method is getting
   cheaper, which is the claim.

4. **The mechanism that keeps it current**, which is the half that usually fails. Weekly capture must
   not depend on anybody remembering: the metrics workflow already runs on a schedule, and the
   question a lane has to answer is what it can read without a human present. **If the honest answer
   is that some input needs a person once a week, say so and make the gap visible rather than
   pretending otherwise** (`script/cadence-check` is this tree's existing shape for a thing that is
   due and has not happened).

## What this must not become

**Not time tracking for calef.** A person required to log hours to satisfy a metric stops
volunteering the honest ones, and the metric is worth less than what it displaces. His standing datum
is accurate enough for every comparison named above; the only thing ever asked of him is a correction
when it stops being true.

**Not a single headline number.** Three audiences want three units: person-weeks for the comparison
against the literature, tokens with a dated price for the argument that the method is getting
cheaper, and cash for calef's own budgeting.

**Not a dollar figure that hides which dollar it means.** This project pays a fixed subscription, so
its marginal cost per lane is zero. A retail-API figure is a **shadow price**: what somebody else
would pay to reproduce the work. Both are legitimate, and quoting one while implying the other is the
dishonest version.

## BUGS

- **The ten person-weeks is a statement, not a measurement**, and nothing will ever make it one
  without time tracking, which this milestone refuses. It is accurate to the bucket the comparisons
  need and no better.
- **Machine effort cannot be attributed to a milestone yet.** Nothing links a lane to the milestone it
  worked on, though the claim commit already names the number. Until that join exists, per-component
  cost is not derivable and only per-week totals are.

## Index row

The method is half this project's claim, and a method's result is a cost; this tree measures only
scale, and the inputs that would answer it are discarded weekly.
