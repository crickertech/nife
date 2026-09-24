# Routing identified work, and routing a decision to calef

*Appendix to [`AGENTS.md`](../../AGENTS.md), which carries the rules: a finding leaves the lane as a
proposed milestone or as a `BUGS` entry, an open decision lives in `design/decisions/`, and work
held for calef carries the `needs-architect` label and a `## What I need from you` comment. This
file carries why each of those exists and what failed without it. The watcher machinery is in
[`notes/merge-queue.md`](../../notes/merge-queue.md) and the sweep that found the category is in
[`notes/untracked-work-sweep.md`](../../notes/untracked-work-sweep.md). Moved here 2026-09-23 (UTC)
on calef's authorization, unchanged in substance.*

**Identified work leaves the lane in a tracked form, or the merge waits.** A lane that finds work it
is not doing may report it in exactly two shapes, and "worth doing someday" is neither. Either a
**proposed milestone** (provisional, the integrator mints the number at merge like every other
global name), or a **recorded limitation written where a reader meets the feature**, in the `BUGS`
section beside it, which is what the promotion triggers of §71 (a limitation is promoted when it
becomes a plan) are then measured against. A finding with no home is the integrator's cue to hold
the merge until it has one.

This is rung three of the ladder, and it is worth naming why the lower rungs fail here specifically.
A lane report is read once, by one person, on the day it is written. A pull request body is read
while the diff is open and never again. Both feel like records while you are writing them, which is
what makes this the failure that recurs: milestone 90 (a guard page under the per-CPU secondary
stacks) exists only because calef happened to be at his desk the day a report named it, and
milestone 94 (the untracked-work sweep) swept the tree for exactly this category and then left its
own inventory in a pull request body for twelve days, by which point the item-level list was gone
and had to be re-derived. See notes/untracked-work-sweep.md.

**The merge checklist grows one line**: every piece of identified work in the lane's report has a
home. `briefs/merge-and-cleanup.md` has that checklist whole, with the prune and the relink.

Two things this deliberately does not do. **It does not gate**: no check can tell an intention from
an observation in prose, and a lint that tried would be `git grep -w TODO`'s 82% false-positive rate
wearing a different hat. And **it does not touch the `BUGS` convention**, which is the FreeBSD
posture working as designed; the whole point is to route intentions *into* it rather than out of it.

**Open decisions live in a file, not in a conversation.** A decision waiting on calef that exists
only in chat scrollback is in exactly the medium milestone 94 was written to abolish, and on
2026-08-04 five of them accumulated there in one day while that milestone was being built. They go
in `design/decisions/` with `**Status: PROPOSED.**`, one section each: what is being decided, the
options, the recommendation with its reason, and what is blocked until it is answered. (They lived
briefly in `design/open-decisions.md`; milestone 114 (split `DECISIONS.md`, and give a decision a
status) absorbed that file, and the numbering is the integrator's at merge like every other section
number.)

**And work waiting on calef carries its own label and its own ask** (calef, 2026-08-04). The same
principle one level out: a pull request held for him is a decision, and a queue that exists only in
a chat message is the medium above. Two things, both at the moment the decision to hold is made and
not later, because the failure this prevents is the maintainer forgetting it is holding something:

- **The `needs-architect` label**, so the queue is `gh pr list --label needs-architect` rather than
  a paragraph somebody has to have read. **It names the role, not the person** (calef, 2026-08-05):
  he holds it today and would like a second architect tomorrow, and a mechanism that spells one name
  has that name as its failure mode. Its description carries the reason a thing lands there at all:
  outside standing merge authority, meaning the syscall surface, a new dependency, or a `DECISIONS`
  section owed.
- **A `## What I need from you` comment** naming the specific ask. Three properties make it worth
  writing, and they are what separate it from a link to a diff. It should be **answerable without
  reading the diff**, because the point is to spend calef's attention on the decision rather than on
  reconstructing it. It should **say what happens if he says no**, since a recommendation with no
  stated downside is not a recommendation. And it should **separate what is blocking from what is
  eventually his**, so a naming backlog does not get tangled with a merge decision; the gate of
  milestone 115 (the names that were ratified, and the ones that were refused) takes `unrecorded` as
  a truthful answer precisely so that provisional names never block.

**Lane count is set against the collision surface, not against queue depth** (calef, 2026-08-16,
overturning his own 2026-08-04 delegation on measured evidence). **Throughput is measured in merged
work.** The question to ask before launching is not "how deep is the queue" but **"what files will
this lane touch, and who else is in them"**. The measurement that overturned the old rule, and the
three ceilings below, are in [design/tenets/lane-count.md](design/tenets/lane-count.md).

- **Disjoint subsystems: launch freely.** Four is a reasonable working number, not a ceiling.
- **Two lanes in the test-wiring hotspot** (`kernel/src/user/tests.rs`, the QEMU runners,
  `xtask/src/main.rs`): **expect to resolve a conflict by hand**, and brief the second one to fold
  into the first's shape rather than inventing a third. It is often cheaper to sequence those two.
- **The real ceilings are elsewhere**, and they are worth naming so they are decided rather than
  discovered: the attention to read reports and resolve conflicts, the token budget, and runner
  concurrency.

**The second ceiling is memory, and it is independent of the collision surface. So: at most one full
`script/verify` at a time on this machine, and never a mutation sweep beside lanes.** When two lanes
must gate together, `VERIFY_JOBS=2` each shares the budget rather than doubling it. The tell is easy
to misread: a heavy job dying with no failing assertion, reported as a cancellation or a timing
failure rather than as memory.

**The third ceiling was disk, and the lever moved with it: lanes gate in CI rather than here**
([`briefs/gate-in-ci.md`](briefs/gate-in-ci.md)), which takes QEMU and `script/verify` off this
machine. Lanes are asynchronous, so the wall-clock cost is nobody's wait. Two habits survive: **run
any local gate from a lane's worktree rather than the main checkout**, and **prune promptly**,
because disk is the only pressure here that destroys work rather than delaying it.

**Prune a lane's worktree the moment its pull request merges**, and never prune one with uncommitted
work in it. [`briefs/merge-and-cleanup.md`](briefs/merge-and-cleanup.md) has the commands, the
order, and both recorded failures.

**The watchers run unattended as `nife-smelter[bot]` in scheduled Actions workflows** (calef,
2026-09-23; the watch that reads a machine's own lane worktrees stays per developer), and a session
confirms they are alive *and reads what they already found*, because `merge-drain.sh` posts once per
stall and then goes quiet by design (calef, 2026-08-26). `briefs/session-start.md` is that check,
deferring the queue read to `briefs/survey-the-queue.md`. A queue reports, it does not resolve.

They exist because on 2026-08-04 three duties turned out to belong to whoever happened to notice:
two green pull requests sat unmerged for hours, `main` went red with nobody assigned, and merging
one pull request staled eight others that nothing picked back up. The steward was meant to cover
this and did not, for a reason worth keeping: **it reported and never acted.** A stalled queue
announced in a message is only useful if somebody reads the message. notes/merge-queue.md has the
workflows, the plist that remains and the commands retiring the two it replaces, the token premise
tested first, what the schedule costs, and a BUGS section honest that no watcher reports its own
death.

**Do not try to route this by requesting a review.** GitHub silently refuses a review request from
the pull request's own author: `gh pr edit N --add-reviewer calef` **returns success and sets zero
reviewers**, because every pull request here is authored under calef's account by the `gh` token.
That was tried on 2026-08-04 and the silent no-op looked exactly like a working queue, which is
worse than an error. Assignees and labels do work; reviewers do not.

**Stop and bring it to calef only when it is genuinely his call:** a design fork not already
decided, a test that will not pass after real effort, a hardware or external dependency, or the
machine contradicting the plan. Otherwise proceed and report what you did.

**Keep the documentation current, because a demonstrator's docs are part of the deliverable.** Every
design decision goes in `design/decisions/`; every concept and finding gets a note in `notes/`,
indexed in `notes/README.md`. Record the *why* and the honest caveats.

**The standard to aim at is FreeBSD's** (calef, 2026-07-30), and it is four things: task-oriented,
so "how do I do X" in order with the actual commands; in-tree and versioned with the code; real
`EXAMPLES`, because a page without a worked example has not finished explaining itself; and an
honest `BUGS` section. **Name the limitation where the reader meets the feature**, not only in a
tracker. When a limitation graduates from record to plan is §71's convention: a `BUGS` entry is a
fact, a roadmap row is intent. See
[design/tenets/documentation-standard.md](design/tenets/documentation-standard.md).
