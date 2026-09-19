# 435. Forty citations of a proposal path that now resolve to nothing

**Status: NOT-STARTED.** Minted 2026-09-19 by milestone 434's lane, which found these while
verifying that nothing still referenced the directory it was cutting. *(Number provisional.
Milestone numbers are the integrator's at merge; on a collision the newer file moves and the older
number stands, per `design/roadmap/README.md`.)*

**Gate: NONE.** Every one of them is a path in a file already in this tree, and the hard half is
reading rather than deciding.

## What is broken

`design/roadmap/proposals/` was drained by milestone 433 and no longer exists. About **forty**
backticked citations of `design/roadmap/proposals/<slug>.md` survive it, spread across roughly
twenty-seven files: milestone blocks (296 has five, 267 and 304 have three each), two audit reports
under `design/audit-reports/`, `design/naming.md`, `design/decisions/152-port-range-capability.md`,
four notes, `kernel/src/pci.rs` and `scripts/qemu-runner-x86_64.sh`.

A citation that resolves to nothing is a broken footnote, which is the defect `script/roadmap`'s
own `**Recorded.**` path check exists to catch, one directory over. None of these is caught, because
nothing checks a backticked path outside a follow-on bullet.

**They are readable, which is why this is a milestone and not an emergency.** Milestone 433 chose
deliberately to keep the slug when it numbered each proposal, so
`proposals/<slug>.md` became `<N>-<slug>.md` and a reader resolves one with a single
`ls design/roadmap/ | grep <slug>`. That was the right call at the time and it does not make the
paths true.

## The two halves, and only one is mechanical

**The resolvable ones.** Where the slug survives on a numbered block, the rewrite is mechanical:
`design/roadmap/303-x86-64-fs-disk.md` cites
`design/roadmap/proposals/an-unclaimed-function-behind-the-iommu.md` in its body, and its own
`## Follow-on` section already names milestone 325 for the same item, so the body is simply behind
its own block.

**The ones whose slug has no numbered file are the work.** Milestone 433 recorded about thirty
proposals that earlier promotions deleted without a one-to-one successor, because the work was
folded into a cluster block. Mapping one of those wants a reading of the cluster block to find where
its content went, and a citation rewritten to the wrong block is worse than one that dangles, since
a dangling path announces itself and a wrong one does not.

## What would make it stick

A rewrite alone is rung four and will rot the same way. The cheap mechanism is the one this tree
already runs elsewhere: teach a gate that a backticked `design/roadmap/...` path has to exist, the
way `script/roadmap` already checks the paths a `**Recorded.**` bullet names. Whether that belongs
in `script/citations` or in `script/roadmap`, and how wide its noise surface should be, is the part
worth pricing before writing.

## Index row

Milestone 433 drained `design/roadmap/proposals/` and kept each proposal's slug on the numbered
block that replaced it, so a reader can still resolve one by hand. About forty backticked citations
of the old paths survive across twenty-seven files, in milestone blocks, two audit reports,
`design/naming.md`, four notes, one kernel source file and one shell script, and nothing in this
tree checks a backticked path outside a follow-on bullet. Roughly ten are mechanical, because the
slug still names a numbered block; the rest are the ones earlier promotions folded into cluster
blocks without a one-to-one successor, and each of those wants a reading of the cluster block,
since a citation rewritten to the wrong block is worse than one that dangles.
