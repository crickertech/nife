# 302. A baseline records what it was saved against, and a stale one fails loudly

**Status: NOT-STARTED.** Promoted 2026-09-16 from
`design/roadmap/proposals/a-toolchain-bump-that-leaves-the-baselines-stale.md`, which was Decision 2
of the finding milestone 299's lane surfaced (originally
`design/roadmap/proposals/icount-baselines-drift-after-a-toolchain-bump.md`, which is now
milestone 577 (the icount baselines predate the pinned nightly), promoted and superseded in one act
on 2026-09-23, on PR #883). Milestone
300 executed Decision 1, decompose and re-baseline. *(Number provisional until the merge queue lands
it.)*

**Gate: NONE.** calef ruled on 2026-09-16, which is what cleared the `DECISION` gate this carried as
a proposal.

## What was ruled, and why it is one mechanism rather than two

Two rulings arrived together and turned out to be the same change:

- **Fail loudly**, option 2 below, rather than option 1's automatic re-baseline.
- **An attribution ledger**: a `--save` records its reason *in the baseline file*, which is rung
  three, a record where the reader already is, instead of rung four, a commit message nobody
  re-reads.

**They are one mechanism, because fail-loudly is not implementable without the ledger.** A check can
only say "these numbers are stale for this nightly" if the file records which nightly produced them,
and nothing does: `bench/baseline-<arch>.txt`'s header names `.qemu-version` as the thing the numbers
are relative to and is silent about the toolchain. So the ledger is not a prerequisite to sequence
first, it is the half that makes the other half possible. Filing them separately would put two lanes
in one header format, which is the collision this tree pays for repeatedly.

## The finding, and why it survives its own correction

`script/toolchain-bump` / `toolchain-bump.yml` raises the pinned nightly and changes only
`rust-toolchain.toml`. It does not re-save `bench/baseline-{aarch64,riscv64,x86_64}.txt`, on the
theory that a new nightly's codegen may move the deterministic icount counts the baselines gate
against. When the bump moves the numbers and the floor is not moved with it, the tripwire's headroom
erodes silently, and the next unrelated PR that adds a few percent trips the +10% gate for a reason
that is not its own.

Milestone 300 measured this window and found the drift was **not** the nightly: `nightly-2026-08-27`
and `nightly-2026-09-15` emit byte-identical icount on the same code, and the whole ~+6.5% move was
one code change (milestone 139's cycle-counter grant at the context switch). So the specific alarm
that motivated the proposal was a false attribution. **The proposal survives anyway**, because the
mechanism it names is real and unguarded: a nightly that *does* move codegen would erode the headroom
exactly as feared, and nothing in the tree would notice until a lane chased an unrelated regression
into it months later. The fix should not depend on which cause happens to bite.

## What to build

**The format is comment lines, ratified by calef 2026-09-16.** The baseline files already carry a `#`
header and every existing reader skips it, so this is additive rather than a format break:

    # saved: nightly-2026-09-15, qemu 11.1.1, 2026-09-16
    # why: milestone 300 recovered the cycle-counter switch-tuple residual (~33 ticks/switch)

A structured `key: value` block was considered and refused: stricter and uglier for a file a human
reads, where prose stays readable and `--check` needs only the first line.

1. **`--save` writes the provenance**, into the header of each `bench/baseline-<arch>.txt` it
   rewrites: the pinned nightly, the QEMU version, the date, and one line saying why this save
   happened. That `why` is the ledger, and it is precisely what milestone 300 had to reconstruct by
   hand out of commit messages.
2. **`--check` fails loudly on a stale baseline**, comparing the recorded nightly against
   `rust-toolchain.toml`'s pin and failing in those words when they disagree. This is Decision 2: it
   turns silent erosion into a red gate a human clears deliberately, at the cost of a manual step on
   every bump that moves the numbers and a no-op on every bump that does not.
3. **One writer, one reader.** The header is a format two programs agree on, so it is written in
   `xtask`'s save path and parsed in its check path, and not re-derived anywhere else.

**Option 1, a bump that re-baselines itself, was refused**, and the refusal is now measured rather
than predicted. `44890a8a` blessed a ~5 to ~8% x86_64 regression into the floor while attributing it
to the nightly, a cause milestone 300 later measured at **~0**. An automatic re-baseline is that
failure with nobody in the loop at all.

## BUGS

- **Option 1 lets automation commit a floor without a human reading the diff**, which is exactly the
  care a baseline save has always carried (`bench/baseline-aarch64.txt`'s own header: "a statement
  that a performance change is intended and understood"). A re-save that also launders a real
  regression into the floor is the failure milestone 300 exists to prevent, one level out; option 1
  would need the decomposition 300 did by hand to be at least partly automatic, or it re-opens the
  hole.
- **This is measured on one machine, one QEMU.** The numbers are TCG icount on the dev Mac. A
  different runner moves them again, which is the argument *for* mechanizing the re-baseline rather
  than trusting a human to eyeball a percentage, and also the argument that the mechanized floor is
  only as portable as the runner that saved it.


## Index row

`bench/baseline-<arch>.txt` records what its numbers mean relative to QEMU and says nothing about the
toolchain, so nothing can tell a baseline that is stale for the current nightly from one that is
current. This makes `--save` write its own provenance into the file's comment header (nightly, QEMU,
date, and one line of why) and makes `--check` fail loudly when that recorded nightly and
`rust-toolchain.toml`'s pin disagree. Promoted from Decision 2 of milestone 300's finding with
calef's attribution-ledger ruling folded in, because the two are one mechanism: fail-loudly cannot be
implemented without the ledger that records the nightly. Option 1, an automatic re-baseline on every
bump, was refused, and `44890a8a` is the measured vindication.
