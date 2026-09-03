# 230. `script/shell-check` is red on `main`, on both architectures, and nothing says so

**Status: NOT-STARTED.** Minted 2026-09-02 by the maintainer, from milestone 192's (a keyboard on
real silicon) lane, which found it while proving an unrelated boot path. *(Number provisional until
the merge queue lands it.)*

**Gate: NONE.** Reproduced, bisected against the toolchain, and blamed to a specific condition.

**In brief.** With a virtio-rng attached, which both plain legs set unconditionally via `NIFE_RNG=1`,
the interactive boot **traps in init at `user_rt::trap` with no message**. The same build with the
device absent reaches a prompt normally.

Reproduced at `8167d806` under `nightly-2026-09-01` as well as `-09-02`, so it is not the toolchain
bump. The shape matches the sixteen-slot capability-table wall that `crates/system_initializer`'s own
entropy-block comment describes.

## Why it is worth a block rather than a fix in passing

**`script/shell-check` is in neither `script/test` nor CI**, which is why a fully green tree does not
contradict it. That is the same family as milestone 222 (the one command a person runs before
pushing has a leg that fails instead of skipping): a gate that exists, fails, and tells nobody.

The difference is worse. 222's leg failed loudly at the thing a contributor typed. This one is red on
`main` and **nothing runs it**, so the tree's own claim to be green is true only because nobody asks
this question. AGENTS.md's third principle is that a newcomer must be able to succeed without asking
anyone; a check that is red and unrun fails that in the quietest possible way.

## What it needs

Two things, and they are separable:

- **The defect.** A trap with no message is the hard part, and the entropy-block comment in
  `crates/system_initializer` is the lead worth following first.
- **A decision about the check itself**, which this block does not make: whether `shell-check` joins
  `script/test` or CI, and if not, what stops it going red again unnoticed. A check nobody runs is a
  check that decays, and adding it to CI is not free, since it boots an interactive shell.

## BUGS

- **The sixteen-slot wall is a hypothesis from a comment**, not a diagnosis. It matches the shape and
  nobody has confirmed it is the cause.
- **This says nothing about how long it has been red.** Nobody has bisected past the two nightlies,
  so the defect may be much older than the day it was noticed.
- **Two graphical legs are blocked behind a different bug** (milestone 177's display driver), so
  fixing this one does not make `shell-check` green in every mode.
