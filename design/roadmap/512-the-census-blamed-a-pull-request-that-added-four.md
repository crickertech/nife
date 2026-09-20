# 512. The census blamed one pull request for 55 survivors it did not write

**Status: NOT-STARTED.** *(Number provisional until the merge queue lands it.)* Promoted from the
proposal `the-census-blamed-a-pull-request-that-added-four`, filed 2026-09-19, on calef's
instruction of 2026-09-20 to give every proposal on `main` a number. The text below is the
proposal's own, unedited except for this paragraph: the argument is its author's and promotion is
not the moment to improve it. Found by milestone 438 (would a diff-scoped mutation check have caught), whose first measurement was a replay of the
pull request `design/fatal-risks.md` names, and which found four survivors where that record
accounts for 55.

**Gate: NONE.** It is a correction to one paragraph of `design/fatal-risks.md` and to milestone 438's
own premise, both already written and both readable. A lane could start today; the numbers it needs
are in milestone 438's block with the commands that produced them.

**What the record says.** `design/fatal-risks.md`'s risk 3, AMBER as of 2026-09-19: *"One crate
accounts for the fall and it was not one of the eight. `machine_discovery` went from 22 survivors to
**77**, at 86.2% ... It is the crate milestone 319 proved on 2026-09-17: the proofs landed, the
parsing around them did not get tests, and two days later the census found it."*

**What is measured.** A full sweep of the crate at `aa6a50b^1`, main immediately before milestone
319's pull request merged, finds **73 survivors already there**:

```console
$ script/mutation -p machine_discovery
686 mutants tested in 7m: 73 missed, 533 caught, 71 unviable, 9 timeouts
```

The pull request added **seven** mutants to the crate and four survivors, and 73 plus 4 is the
census's 77 exactly. It did not write them.

**Where the 22 comes from, as a reading rather than a measurement.** The `(baseline missed)` column
`script/mutation --report` prints is `.cargo/mutants-baseline.txt`, where `machine_discovery` reads
`147 22 3 40`. That file is the **2026-08-03** baseline, so a rise from 22 to 77 is six weeks of a
crate growing from 212 mutants to 693, not two days of one pull request. The per-crate numbers from
the 2026-09-14 census are not in the tree, only its aggregates, which is what made the wrong column
the available one.

**Why the correction matters more than the arithmetic.** Risk 3's verdict is *"the tree adds untested
code faster than triage removes it"*, and the evidence offered for the rate is this attribution. Take
the attribution away and the same numbers support a different reading: the code accumulated over six
weeks and **the instrument looked twice**, once on 2026-09-14 and once on 2026-09-19. That is a
cadence problem, and the record already carries the fact that supports it (*"the workflow has
succeeded exactly once, so a cadence is claimed by one data point"*) in a different paragraph from
the one that draws the conclusion.

Risk 3 stays amber either way. What changes is what would turn it green, and milestone 438 refused a
diff-scoped gate partly on this: a mechanism aimed at a rate cannot be judged against a case that was
not a rate.

**The work.** Rewrite risk 3's `machine_discovery` paragraph against the measurement, say plainly
which census each number comes from, and record the trap: **`--report`'s baseline column is
2026-08-03 and is not the previous census.** The trap is the durable half. Every future reading of
that column by anyone will make the same mistake, and the fix is either a second column or a header
that says the date out loud.

## Index row

What the record says. `design/fatal-risks.md`'s risk 3, AMBER as of 2026-09-19: *"One crate accounts
for the fall and it was not one of the eight.
