# 237. The cycle-counter grant costs 192 bytes of IPC fastpath for an instrument nothing can request

**Status: NOT-STARTED.** Minted 2026-09-03 by calef, from reading why `script/fastpath-footprint`'s
aarch64 headroom had shrunk. *(Number provisional until the merge queue lands it.)*

**Gate: NONE.** The measurement is taken, the pattern exists in this tree, and nothing external
blocks it.

**In brief.** Milestone 229 (build the cycle-counter grant DECISIONS 139 decided) put a per-thread
grant on the context switch, which is where it has to be: `PMUSERENR_EL0` is one register shared by
every thread on a core, so the only moment a per-thread grant can mean anything is when a thread
starts running. The read is the enforcement, not overhead around it.

**Measured on aarch64, per symbol, against the commit the baseline was saved at:**

| symbol | baseline | now | delta |
|---|---|---|---|
| `sched::schedule` | 1244 | 1380 | **+136** |
| `sched::finish_switch` | 824 | 880 | **+56** |
| `sched::ipc_recv` | 1320 | 1324 | +4 |
| `sched::ipc_send` | 952 | 956 | +4 |
| everything else | | | 0 |
| **total** | **5788** | **5988** | **+200** |

**And its only consumer is a benchmark on a machine that has never booted nife.** DECISIONS 139's
own accounting: a user-level cycle read is what milestone 25 (cross-OS performance comparison) needs
to reproduce seL4's published 413 and 426 cycle figures on **argon**, and nothing else in the tree
wants it. The kernel may read `PMCCNTR_EL0` at EL1 with no grant at all, and milestone 168 (a
multi-tasking workload benchmark) is a long-loop measurement the generic timer already serves. Since
229's ABI was deliberately deferred, **no program can request the grant today**: the only writer is a
`#[cfg(test)]` helper.

## What to do, and why it is not deletion

calef's question was whether the code can be switched off once the benchmark is taken. It can, and
doing so would cost two things:

- **The number would stop being reproducible.** A cycle figure published against a competitor's,
  taken once with an instrument that was then removed, is a claim nobody can re-check, including us
  the next time IPC changes. `notes/register-of-measures.md` opens with exactly that complaint.
- **It would quietly reverse DECISIONS 139**, which answered who may read the counter and by what
  authority. Deleting the enforcement returns the answer to "closed for everyone", which is not the
  answer that was given.

**So: a measurement build, the way `--features soak` is.** Production never carries it; anyone can
rebuild and re-measure at any time.

- The production fastpath returns to **5852**, keeping milestone 231's slot counter, which has a real
  consumer and prints on every boot.
- The instrument stays reproducible rather than being spent once.
- DECISIONS 139's authority model still stands: the grant is how it works when built.

**The comparability question answers itself, and in our favour.** A gated build is not the production
binary, so its numbers carry a caveat, which milestone 221 (the soak never crosses cores) already
records for soak builds. But seL4's published figures come from `KernelArmExportPMUUser`, a
configuration seL4 **does not verify and does not ship on by default**. Both sides would be measuring
in a benchmarking build, which is like-for-like and more honest than comparing our production kernel
against their benchmark one.

**The residual cost is worth recording rather than hiding**: a benchmark build has 192 more bytes in
`schedule` than production, so the number it produces is slightly pessimistic about nife.
Understating ourselves is the right direction to err in a comparison we intend to publish.

## The mechanism failure this came out of, which outlives the fix

**Two lanes each measured "within bound" against the same stale baseline, and neither re-saved it.**
Milestone 231 took the aarch64 figure from 5788 to 5852, milestone 229 from 5852 to 5988. Both were
honest, both were under the 5% bound, and the bound is measured against a **stored** number, so the
growth accumulated with nothing firing. Headroom went from 3.9 points to 1.5 without anyone deciding
to spend it.

**The baseline must not be re-saved to absorb growth nobody attributed.** `bench/fastpath-aarch64.txt`
already carries one such re-record (`ff38e4a2`, "re-record the fastpath baseline that PR #316 moved
and did not record"), and its own header asks for the opposite: *"Updating this file is a statement
that a footprint change is intended and understood; do it in the commit that causes it."*

## BUGS

- **A gated path rots unless something builds it.** `soak` is built in CI for exactly this reason, and
  this feature needs the same treatment or milestone 74 will turn on a path nobody has compiled for
  weeks.
- **This block does not price milestone 74's arrival.** If 74's aarch64 half lands soon, churning the
  fastpath twice to save 192 bytes for a few weeks may be the worse trade, and that judgement belongs
  to whoever takes this.
- **Nothing here fixes the accumulating-baseline problem**, only this instance of it. Whether the
  gate should compare against `main` rather than a stored file is a separate question with its own
  costs.
