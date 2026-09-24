# The confirmation run, 2026-08-22

*(An appendix of [notes/load-sensitive-assertions.md](../load-sensitive-assertions.md), which holds
the register and the rules. Moved there verbatim on 2026-09-24; "this page" and "above" in the text
below meant the single note it came from.)*

## The confirmation run, 2026-08-22: 45 of 45 green, the block closes

Milestone 62's own text named exactly one thing left after the 2026-08-18 disposition: a repeat
count under load of the tree as it now stands, at the same 45-run standard the first acceptance run
set, because this block's own BUGS section says a flake cannot be shown fixed by a green run and the
same rule applies to the change that removed one. This is that run.

`script/repeat-under-load -n 45 -s 8`, the same recipe and the same host as 2026-08-17, run against
the tree with both the assertion disposition and the migration-drain fix in place.

- Host: Mac15,3, 8 cores, Darwin 25.6.0 arm64. Tree `50a0e7cb`.
- 45 runs, 112 minutes of wall clock, 2026-08-23T00:57:46Z to 2026-08-23T02:49:34Z.
- One-minute load average across the whole loop: **4.8 low, 90.1 peak** (684 samples at 10 s
  cadence). Both ends are outside the 2026-08-17 run's 26.1 to 63.0 band: the low end catches a run
  before its spinners had ramped the average up, and the high end is harsher than anything the
  first acceptance run saw, three separate runs touching 90.
- A few runs shared the host with a neighbouring QEMU (peak QEMUs seen reached 3 in run 14 and 2 in
  five others); most ran alone. Recorded rather than smoothed over, per the first run's own
  convention.

**Result: 45 of 45 green.** `ticks_arrive_at_the_configured_rate` ran on both ISA legs in every run
(90 legs total) and never printed `UNMEASURED`: the retry budget found a miss-free window on its
first attempt every single time at this load. `the_handler_keeps_up_when_no_lock_is_held` is gone
from the suite entirely, so it contributed neither a red nor a false pass by construction.
`a_migrated_kernel_thread_keeps_its_hart_pointer`, the migration-drain fix's own target, was green
in all 45 runs. No other assertion in the suite went red.

**This is a stronger result than the 2026-08-18 interim run's own numbers predicted, and the honest
reading is that 18 runs was too small a sample to bet on.** That run saw `UNMEASURED` on 9 of 36
legs (25%) and flagged the rate itself as the thing that wanted a bigger count before anyone acted
on it. Forty-five runs, 90 legs of the same assertion, found zero. Both are true statements about
different sample sizes; the second is the one this block's acceptance standard asks for.

**What this closes.** design/roadmap/62-time-sensitive-tests.md's one remaining item is answered:
the repeat count under load, at the block's own standard, on the current tree, is 45 of 45. The
block moves to BUILT.

**What it does not close**, and both caveats were already on record before this run:

- **One host is one host.** Nothing here says anything about a GitHub Actions runner, a different
  QEMU build, or a load shape this recipe does not produce. That was true of the 2026-08-17 run and
  stays true of this one; it is the standard the block set, not a wider claim than that.
- **Zero red in a finite sample bounds a rate, it does not prove one is impossible.** By the rule of
  three, 0 failures in 45 full-suite runs is consistent with a true failure rate as high as roughly
  3/45 (about 6.7%) at 95% confidence; 0 `UNMEASURED` results in 90 legs of the specific assertion
  bounds that narrower quantity at roughly 3/90 (about 3.3%). Both are compatible with the
  2026-08-17 run's observed 9-of-90-leg rate if the disposition lowered the rate rather than
  eliminating the failure mode outright, and both are compatible with a true rate near zero. Only
  more runs, or a different host, would narrow it further, and nothing in this milestone's own
  acceptance standard asks for that: 45 was the number the first run set and this run matched it.
