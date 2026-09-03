# 233. `login` dies on every boot, and the boot says it is ready

**Status: NOT-STARTED.** Minted 2026-09-02 by the maintainer, from milestone 230's
(`script/shell-check` is red on `main`, on both architectures, and nothing says so) lane, which found
it the moment that check could see straight. *(Number provisional until the merge queue lands it.)*

**Gate: NONE.** The cause is measured and the fix is understood; what it costs is the interesting
part.

**In brief.** The `login` thread dies on **every** boot, on **both** architectures, and has been
doing so unnoticed.

Measured rather than deduced: instrumenting `login::fail` to fault at `0xFA11_0000 + step` gives
`far 0xfa110001`, which is step 1, `nifefs::Fs::parse` refusing the archive. `login`'s `_start` reads
the archive length from `a1`, and `crates/system_initializer` starts it with
`thread_control_block_start(login_tcb, 0, 0, 0)` and endows it with **no mapping of the archive**.

**And the boot prints `init: login ready`, with a generated password**, because init measured the
identity provisioning rather than login's survival. So the line is true about what it checked and
false about what a reader takes it to mean.

That makes this the sharpest member of a family this tree found four of in one day (milestone 232,
audit every check against two questions): not a check that failed to run, but a check that **passed
while the thing it named was dead**.

## Why it is coupled to milestone 231

Handing `login` a mapping of the archive costs init capability slots, **at exactly the peak
milestone 230 measured and sized the table against**: 21 of 24, with three slots of deliberate
headroom that its own block calls a guess standing in for a mechanism.

So the fix and the accounting move together, and milestone 231 (nothing counts how many capability
slots a boot actually uses) is the accounting. Doing this one blind is how the table gets raised a
fourth time, reactively, after another silent failure.

## What it needs

- **`login` receives what it needs to parse the archive**, or stops needing it. Which of those is
  right is not obvious and this block does not decide: a service that must read the whole archive to
  answer a password is a different design from one handed only what it must know.
- **The boot's own report stops overstating.** `init: login ready` should mean login is ready. What
  init currently measures is worth keeping; it is the sentence that is wrong.
- **An assertion that no user thread was killed during the boot**, which milestone 230's lane
  identified as the obvious ratchet and deliberately did not add, because it would have been red on
  both architectures until this milestone lands. It belongs here.

## BUGS

- **How long this has been true is unknown.** Nobody has bisected it, and unlike milestone 230's
  five days there is no green-to-red transition to search for, since the check that would have
  noticed was itself not running.
- **The fix may not fit.** If the archive mapping costs more slots than the headroom holds, this
  milestone raises `CAPABILITY_TABLE_SLOTS` again, which is the outcome milestone 231 exists to make
  visible rather than surprising.
- **Nothing else that init starts has been checked this way.** `login` was found because a gate
  happened to look; the same question has not been asked of the other services init builds.
