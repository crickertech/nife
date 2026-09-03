# 232. Audit every check against two questions: does anything run it, and does it block

**Status: NOT-STARTED.** Minted 2026-09-02 by calef, after four independent findings in one day, each
a different way for this tree's verification to report something that was not true. *(Number
provisional until the merge queue lands it.)*

**Gate: NONE.** It is an audit of what already exists.

**In brief.** AGENTS.md's second principle says the method works because of the gates, the proofs and
the review discipline. `design/fatal-risks.md` risk 3 (the tests do not test anything, and the
quality is illusory) stands at **MEASURED, green**, on 92.4% of viable mutants killed.

**Four findings on 2026-09-02 say that number is measuring somewhere the failures are not**, and no
two of them are the same defect:

- **Milestone 214** (a test that prints "skipping" and returns is counted as passed) found **80 sites
  across 18 files** that printed a skip line and returned, so the harness counted them as passes.
  Twenty-five tests left the pass column on x86_64 once they told the truth. A mutation sweep cannot
  see this: a test that returns early still runs and still kills nothing.
- **Milestone 222** (the one command a person runs before pushing has a leg that fails instead of
  skipping) found `script/test --hvf` failing rather than skipping, telling a contributor something
  was broken without saying whether it was theirs.
- **Milestone 230** (`script/shell-check` is red on `main`, on both architectures, and nothing says
  so) found a check red for five days that **ran in neither `script/test` nor CI**. Its cause was a
  constant restored on two true observations by a lane that was right about everything it checked and
  shipped a system that could not boot, because **no suite in `script/test` boots the real init**.
- **And then that same check, once running, found `login` dead on every boot on both
  architectures** while the boot printed `init: login ready`, because init measured the identity
  provisioning rather than login's survival.

Four shapes: a test that lies, a gate that fails instead of skipping, a check nothing runs, and a
check that passes while what it names is dead.

## What the audit is

**Two questions asked of every check in this repository, and the answers written down:**

1. **Does anything run it?** On 2026-09-02 `script/repeat-under-load` and `script/crate-probes` were
   referenced by neither CI nor `script/gates`. **`crate-probes` builds fifty crates.io crates
   against the nife `std`**, which is the instrument behind fatal risk 1 (only software written for
   nife runs on nife), and that risk is recorded **GREEN**. Its last result is whatever somebody saw
   when they last ran it by hand.
2. **Does it block?** The `main` ruleset requires eleven checks. CI runs more. On 2026-09-02
   `image permissions` (milestone 208's W^X gate, added that day), `re-falsify the harnesses this
   change can reach`, and `architect hold` all ran without being required. The third is correct, since
   a label gate should not block. The first two are worth deciding rather than inheriting: a linker
   script could reintroduce a writable-executable segment, turn that job red, and still merge.

**Then a third question, which is the one with teeth: what does a green result actually assert?**
`shell-check` was green while `login` was dead. That is not a check that failed to run or failed to
block; it is a check whose passing meant less than its name.

## What it must not become

**Not a sweep that adds checks.** `script/lint` has had three checks deleted for the "only ever
rejects legitimate work" signature, and this milestone would be a poor way to acquire the fourth,
fifth and sixth. The deliverable is an inventory and the decisions it forces, not a pile of new
gates.

**Not a rewrite of risk 3's status on one day's evidence.** Whether the mutation number should be
re-read is `design/fatal-risks.md`'s question and it is calef's. This milestone gives him the
inventory to answer it with.

## BUGS

- **An audit is a snapshot**, and the four findings above all arrived within one day of each other,
  which suggests the rate matters more than the count. Nothing here creates a mechanism that keeps
  the answer current.
- **It cannot find the fourth shape by inspection.** A check whose passing means less than its name
  is only discovered by someone asking what it proves, which is what found `login`. An inventory can
  list checks; it cannot audit their meaning without reading each one.
- **Two of the unrun checks are expensive** (`crate-probes` builds fifty crates, `repeat-under-load`
  boots QEMU repeatedly), so "run it in CI" is not automatically the answer and the audit should
  price rather than assume.
