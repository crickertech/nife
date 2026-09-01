# 212. `script/falsifications` walks `crates/` only, so the ratio it prints is not the tree's

**Status: NOT-STARTED.** Minted 2026-08-31 from milestone 197's (`user/` and `xtask` are out of reach
of the prover) lane. *(Number provisional until the merge queue lands it.)*

**Gate: NONE.** Everything it needs exists; the walk is wrong rather than missing.

**In brief.** DECISIONS §134 (a harness carries a machine-replayable falsification record) says the
`unfalsified` count is *"the claim's honest denominator"*. **It is currently the wrong denominator.**

`script/falsifications` walks `crates/` and nothing else. So:

- Milestone 197's record and patch live under `user/` and are **not counted**.
- Milestone 193's two kernel harnesses, the first proofs ever written over `kernel/src`, **carry no
  `Falsification:` record at all**, and nothing reports their absence.
- Milestone 202's `kernel/falsifications/` directory exists on `main` and **nothing sweeps it**.

**The number is printed as a fraction of the tree and is a fraction of one directory**, and the
script does not know the difference. That is worse than an undercount: §134's whole argument is that
a claim about proofs must not rest on a written claim about proofs, and this is a mechanical claim
resting on an unstated scope.

## What it needs

**Derive the walk from `cargo metadata`**, the way `script/lint`'s verify-table check already does.
A hand-maintained directory list is the same defect one level up, and `script/verify` has recorded
that failure twice: `mdns_proto` and then `jh7110_trng`, both carrying harnesses nothing ran.

**Teach `--sweep` that a package can have binaries.** It shells `cargo kani -p <crate>`, which for
`user` selects all 68 programs. Milestone 197 solved the same problem for `script/verify` by deriving
a `--bin` list from a grep rather than writing one down, *"because a list would be one name short the
first time somebody adds a harness to a 69th program"*. The sweep wants the same treatment.

**Decide what a kernel record costs before promising one.** A kernel falsification cannot be swept
until milestone 210 (no kernel test can be run by name) lands, so the kernel rows may have to count
as a known gap rather than as records for now. Saying so explicitly is better than a denominator that
quietly excludes them.

## BUGS

- **This makes the ratio worse before it makes it better.** Counting `user/` and `kernel/` will drop
  the percentage, because those harnesses have fewer records, and that is the point: the current
  number flatters by omission.
- **It does not fix the kernel sweep**, which is milestone 210's. Until then a kernel record can be
  written and not replayed, which is exactly the `attested` state §134 defines and should be used
  rather than worked around.
