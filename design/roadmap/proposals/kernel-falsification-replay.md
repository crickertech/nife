# Six kernel confinement claims have falsification records that nothing can replay

**Status: PROPOSED 2026-09-03.** Written by the milestone 247 sweep, from milestone 210's block.

**Gate: DECISION.** Two of the three pieces are decisions rather than plumbing, and both are about
conventions §134 already fixed for crates. Where kernel records live is the sharper one:
`kernel/falsifications/` exists today and reads as a refusal of §134's per-crate rule, and somebody
has to say whether it is an exception or a mistake before a sweep is written against either answer.

**In brief.** `script/falsifications` replays a proof's falsification by running `cargo kani
--harness <name> --exact`. Kernel tests have no equivalent. Three pieces close it: extend §134's
`Falsification:` comment convention from `#[kani::proof]` to `#[test_case]`, teach
`script/falsifications` a second replay verb (`cargo xtask test --arch <a> --test <name>`, per
architecture, beside the Kani one), and settle where kernel records live against §134's per-crate
rule.

## Why this matters

A falsification record is the thing that separates a proof from a green light nobody has tested. The
whole point of the convention is that a claim is only believed once somebody has watched it fail on
purpose, and the replay verb is what keeps that true after the day it was written. Milestone 202
falsified six kernel confinement claims by hand, twenty-five runs' worth of work, and none of that
is repeatable by a script today. So those six claims have already decayed to the state the
convention exists to prevent: assertions with a story about a falsification that nobody can run
again.

It also makes the tree's own honesty metric wrong in a way milestone 212 already found once. §134
calls the unfalsified count the claim's honest denominator; 212 fixed the walk so it covers packages
rather than one directory, and explicitly handed the kernel half back here. Until this is done, the
ratio counts what it can reach and the kernel is outside it.

The pieces are cheap now that milestone 210 shipped. `cargo xtask test --test <substring>` already
runs one kernel test by name, which was the missing primitive; a filtered run is 8.6 seconds. What
is left is the convention and the sweep.

## Where it came from

Milestone 210 (no kernel test can be run by name) named it, and milestone 212 handed it back here
rather than absorbing it. 210's follow-on: *"Build the sweep that replays kernel falsification
records. Three pieces, each a decision rather than plumbing [...] Milestone 212 handed this back
here, and six kernel confinement claims from milestone 202 have no replay mechanism until it is
done."*
