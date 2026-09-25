# A ruling should make the gate it answers fail until someone updates it

**Status: PROPOSED 2026-09-24.** Raised by the `maintainer/198-gate-is-stale` lane, which found
milestone 198 (a package manager, and the trivial install that makes a second customer possible)
still gated `DECISION` a day after §208 (installing is granting, and the activation set is
versioned) ruled the last of its three forks, and `design/fatal-risks.md` risk 8 repeating the
stale gate as the reason nobody can be asked to run nife. **Name provisional**: this file's stem is
a lane's coinage.

**Gate: NONE.** Every option below is a change to `script/roadmap` and to prose in roadmap blocks,
which is reversible in AGENTS.md's sense: nothing outside this repository reads a gate line.

## What went wrong, in the tree's own mechanisms

The tree already has the mechanism for the analogous case. A `MILESTONE N` gate fails
`script/roadmap --check` the day milestone N is `BUILT` ("every gate still pointing at it fails the
build instead of quietly staying stale"). **A `DECISION` gate has no such partner**: it names no
object, so nothing can tell when the decision it waits on has been made. 198's gate named its forks
in prose, as proposals; the activation one was a roadmap block (milestone 507 (installing a package: mutate, compose, or
widen what can be spawned)), and ruling it
minted a new section (§208) and set 507's own gate to `NONE`, but touched nothing that cites 507.
Rung 3 of AGENTS.md's ladder (a written record at the thing) was present in 507 and absent at 198.

## Measured, 2026-09-24, over `design/roadmap/*.md`

- **70** blocks carry a `DECISION` gate. **39** cite at least one `§N` in the gate paragraph; **31**
  cite none.
- **9** cite only sections that are already `DECIDED`: milestones 39, 137, 172, 188, 334, 340, 356,
  388, 391. Some of those are genuinely stale and some cite a ruled section as context while waiting
  on something else (milestone 39 (repository structure for a loosely-coupled OS) cites §151 (the goal of the
repository split is independent release), which ruled the split's goal and deliberately left its
  order open). **So a prose citation cannot be read as "waits on"**, which rules out the cheapest
  option below as a gate.

## Options

| | Mechanism | Rung | Cost | Why it wins or loses |
|---|---|---|---|---|
| **1. Infer from prose** | `--check` warns when every `§N` in a `DECISION` gate paragraph is `DECIDED` | 2, as a warning | An afternoon | Loses as a gate: 9 hits today, and at least one (39) is a false positive the checker cannot tell apart. Useful as a one-off sweep |
| **2. `DECISION §N` in the gate vocabulary** | A `DECISION` token may name what it waits on; `--check` fails when that section is `DECIDED`, exactly as `MILESTONE N` fails when N is `BUILT` | 2, failing | `script/roadmap`'s grammar plus a migration of the 39 citing blocks, each read by hand | **Recommended.** It is the existing `MILESTONE N` rule applied to the other kind of wait, so it adds no new idea. A fork not yet written up keeps bare `DECISION`, which stays legal |
| **3. `DECISION MILESTONE N` for forks held as roadmap blocks** | As 2, for a proposal promoted to a milestone number (507's shape): stale once that block's gate is `NONE` and a section cites it | 2 | Small, on top of 2 | Needed with 2, because 198's fork was milestone 507, not a section, until the ruling minted §208 |
| **4. At ruling time** | `script/decisions --check` lists roadmap blocks whose gate cites a section flipping to `DECIDED` | 3, a report | Small | Loses alone: a list printed once to whoever rules is rung 4 in practice |

**Recommendation: 2 with 3.** Would we still choose it if 1 cost the same? Yes: 1 cannot be made
to fail without false positives, and 2 is the same rule the tree already trusts for milestones.
If calef says no, the cost is that the next ruling may again leave a gate stale until someone reads
the block, which is what happened to 198 and to fatal risk 8 on 2026-09-23.

## Index row

A `DECISION` gate names nothing, so nothing can tell when it has been answered; milestone 198 sat
behind a ruled fork for a day. Proposed: `DECISION §N`, failing once §N is `DECIDED`, the same rule
`MILESTONE N` already follows.
