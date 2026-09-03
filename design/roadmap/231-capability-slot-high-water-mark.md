# 231. Nothing counts how many capability slots a boot actually uses, so the wall is always a surprise

**Status: NOT-STARTED.** Minted 2026-09-02 by calef, from milestone 230's (`script/shell-check` is
red on `main`, on both architectures, and nothing says so) own `BUGS`. *(Number provisional until
the merge queue lands it.)*

**Gate: NONE.** The number exists at runtime; nothing reads it.

**In brief.** `CAPABILITY_TABLE_SLOTS` has been raised three times, and **every raise was reactive,
after a silent failure that named something else.**

- **16 to 17**, milestone 49 (users and attribution). The symptom was that the first login against a
  freshly built service answered `login_proto::DENIED` instead of `OK`, **on a correct password**.
  Nothing said "out of slots".
- **17 to 24**, milestone 230, today. The symptom was that init trapped with no message and `main`
  could not boot interactively for five days. `MemoryRegion::RETYPE` answers `OutOfMemory` for both
  "region out of pages" and "table full", so even the error did not distinguish them.

Between those, the constant spent time at `28 // TEMP: generous bisection value`, was restored to 17
on two true observations, and the restoration shipped a system that could not boot. That episode is
milestone 230's account and is worth reading before this one is designed.

**The measured high-water mark is 21**, in init, during `build_child` for `credentialer`. Nobody knew
that until somebody instrumented four boots to find it.

## What it needs

**A boot that reports the peak it reached, against the ceiling it had.** That is the whole idea. A
line saying the boot used 21 of 24 turns a cliff into a gauge, and it is the difference between
milestone 230's four instrumented boots and one ordinary one.

Three things this block does not decide, named so they are decided rather than discovered:

- **Where the count lives.** A per-table peak in the kernel is the obvious place, and it is also the
  hottest structure in the system; whether the counting is free enough to be unconditional, or wants
  a feature gate the way milestone 221's soak counters did after `script/fastpath-footprint` caught
  them at 5.7%, is a measurement rather than a preference.
- **What it does as it approaches.** Reporting a peak is passive. Failing loudly at some margin is a
  gate, and this tree has deleted three checks for the "only ever rejects legitimate work" signature,
  so a threshold picked from one boot would be the fourth.
- **Whether `script/shell-check` should assert on it**, now that milestone 230 put that check into
  `script/gates` and CI. An assertion there would catch the next raise's need before a merge rather
  than after, which is the failure this milestone exists to end.

## Why the headroom is not the answer

Milestone 230 left three slots of margin deliberately, because both previous raises set the number to
exactly what that day's boot needed and both times the next addition hit the wall in the same
silence. **Three slots is a guess standing in for a mechanism**, and its own block says so. This
milestone is what would replace it.

## BUGS

- **This does not make the wall impossible, only visible.** A boot that needs a twenty-fifth slot
  still fails; it just fails saying so.
- **It says nothing about the other fixed-size tables.** `MAX_THREADS`, `MAX_REGIONS` and
  `nifefs::NAME_LEN` are the same shape, and whether one mechanism should serve all of them is a
  question this block leaves open rather than answers by scope creep.
- **The peak is workload-dependent.** The number a boot reports is the number *that* boot reached,
  and a richer initrd reaches a different one, so a single green figure is not a guarantee about
  every configuration.
