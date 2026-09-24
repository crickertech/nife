# The three principles, and what makes each one hold

*Appendix to [`AGENTS.md`](../../AGENTS.md), which carries the principles themselves as rules. This
file carries their evidence: the measurements, the failures that confirmed them, and calef's own
wording. A reader who only needs to act can stay in `AGENTS.md`; this is where the argument is
checked or challenged. Moved here 2026-09-23 (UTC) on calef's authorization, unchanged in substance.*

These are not aspirations. Each names a mechanism that keeps it true when nobody is watching, which
is the only kind of principle a free software project can enforce: a volunteer cannot be made to
care, so the work has to carry the standard on its own.

## 1. The ranking function is the shortest path to a system a customer runs

**A customer running it is the only test that cannot be gamed.** A benchmark can be chosen, a gate
can be written to pass, a note can describe a system that no longer exists. A backup somebody
depends on either works on a Tuesday or it does not, and the failure arrives as their own data
missing rather than as a red check.

**This principle has now been confirmed in the only way that counts, which is by failing.** It was
written on 2026-08-05 pointing at milestone 55, a Time Machine target the family's Macs back up to,
whose own block called it *"The actual goal, and probably the largest single piece of work in the
project."* On **2026-08-30** calef reported that the family's backups run on **borg over SSH on
cordoba**, with **Immich** for images, built with the existing Linux ecosystem while nife was not
ready; Time Machine and SMB are both out of that path. Journey 2 was retired the same day and
milestone 55's premise went with it.

That is not the principle failing. A customer with a real deadline went elsewhere because this
system could not meet it, which is **the principle working**, and is the outcome it exists to make
visible early rather than late.

**calef is the first customer, not the audience** (his correction on this section, 2026-08-05:
*"It isn't about me running it. It is about customers. I'm just the first customer."*). That
distinction is
load-bearing rather than modest. "The architect runs it" ranks work by one person's convenience and
has no answer when that person's taste and a stranger's needs diverge; "a customer runs it" ranks it
by what anyone taking this system on would require. The two agree today because there is exactly one
customer. They stop agreeing the moment there are two, and the wording that survives that is the one
worth writing now.

What that means concretely, and it is a reordering rather than a slogan:

- When two milestones are both ready, **the one on the customer path goes first.** **As of
  2026-08-30 that path is vacant**, and saying so plainly is the point: a roadmap that still named
  one would be ranking by a workload nobody runs. It was 54 (a network file service a Mac can mount)
  and 55 (Time Machine) until that date; both are repriced in their own blocks.
- **The first customer was too big, and that is the lesson worth carrying.** A family backup server
  is among the largest things a home system can be asked to be: a filesystem it did not format, a
  network protocol, crash consistency, and somebody's only copy. This principle said to rank by the
  shortest path to a customer, and the path chosen was one of the longest available. **A first
  customer should be something nife can plausibly be adequate at within a milestone or two.**
- **While the path is vacant the tie breaks toward design/fatal-risks.md**, nine claims that, if
  false, mean the project should stop. It is a stand-in for a customer, not a replacement: a real
  workload with a real user outranks everything on it the moment one exists.
- **And the path is vacant for a second reason, which is ours rather than the customer's** (calef,
  2026-08-30): *"I don't think we expose nife to third parties (aka other customers) until we have a
  package manager and a trivial install process."* So there is a **precondition on the ranking
  function itself**. Package management and an install story are not items on the customer path; they
  are what makes one possible, and until they exist a second customer cannot be accepted if one
  appeared. He wants them **early, for our own sake as much as anyone's**: the people building this
  are the ones repeatedly hand-wiring what a package would install.
- A milestone off the path is not thereby worthless. Verification, parity and the analysis tooling
  are what make the demonstrator a demonstrator. But when they compete for a lane, the tie breaks
  toward the thing that gets a real workload running.
- **Security and performance are not separate goals; they are what "runs it" means.** No customer
  runs a backup server they do not trust with the only copy, and none runs one that takes a week.
  That is why the audit cadence, the confinement claims and the benchmark tripwire are on this path
  rather than beside it.
- Naming is on this path too, and it is the least obvious member. A person using the system meets a
  name before they meet anything else, and in a capability system the name is often the only thing
  that says what a program may *do*.

**The honest caveat: the system is not ready for a customer, this one included, and 2026-08-30
settled how far off that is rather than leaving it as a feeling.** Saying the principle out loud is
what stops the roadmap drifting into a collection of interesting kernels, and the drift is a live
risk now rather than a hypothetical one, because with no customer named there is nothing but this
principle and the fatal-risk list holding the ordering together.

## 2. The method is a result, and it is currently undocumented

Measured on **2026-08-30**, from a first commit on 2026-07-12: **49 days, 103 milestones built of
193, 65 crates, 69 user programs, ~194,000 lines of Rust, 145 Kani proof harnesses, 3,099
commits**, on **three** architectures, with a booting kernel on real RISC-V silicon, a shell, a
filesystem, a network stack and a compositor.

**That line count includes comments**, and `kernel/src` measures 40% of them, so a size comparison
against another project belongs in code lines. (The superseded 2026-08-05 figures are in git.)

That is not a normal rate for one architect, and the reason is that the work is done by many agents
in parallel lanes with one person reviewing architecture and outcomes. **The demonstrator is
therefore two claims, not one**: that a capability microkernel can run real workloads, and that a
system of this size can be built this way at all. The second is at least as interesting to a
stranger, and nothing in this tree currently states it.

**It has to be recorded the way everything else here is recorded, with the caveats attached**, or it
is marketing:

- The numbers above are **size and rate, not quality.** 63 built milestones is a count of blocks
  marked BUILT, and this tree found nine of them misrecorded in a single sweep (§76). Take the number
  as a scale, never as a claim about correctness.
- **What makes it work is not speed.** It is the gates, the proofs, the honest `BUGS` sections and
  the review discipline. The same method without them produces a great deal of code that nobody can
  trust, faster. Every failure recorded in this file is evidence for that: the lane that squashed
  against `origin/main` and staged four other lanes' files, the blind `sed` that rewrote the row
  recording a name's refusal, the three agents that clobbered work with `git reset --hard` in one
  day.
- **The bottleneck moves, and pretending otherwise wastes the method.** On 2026-08-04 the constraint
  stopped being how fast lanes could produce and became how fast one merge queue could land, and
  eleven lanes made that worse rather than better.

## 3. A newcomer must be able to succeed without asking anyone

This is the principle that most of this file already serves without naming it, and it is the one that
inverts hardest for a project like this one. In a company a high standard can be enforced through
people, because they are paid and can be managed. **Here the only enforcement is that the work
answers its own questions**, because a contributor who has to ask will simply leave, and will do so
silently.

So a standard that is not also generous produces an empty repository. That is why:

- **The documentation standard is FreeBSD's**: task-oriented, in-tree, real `EXAMPLES`, and an honest
  `BUGS` section next to the feature rather than in a tracker. A page without a worked example has
  not finished explaining itself.
- **`BUGS` sections are not modesty, they are the mechanism.** A newcomer who hits a limitation the
  docs named will trust the docs. One who hits a limitation the docs hid will not trust anything
  again, and there is no relationship to fall back on.
- **A name is a claim, and the reader meets it first.** `script/names --unratified` is a worklist
  rather than a wall precisely so that an unratified name never blocks anyone's build.
- **Every decision has a written reason.** `design/decisions/` records why, including for decisions
  that were refused, so a newcomer can disagree with an argument rather than with an authority.
- **Anything that only works because someone knows it is a defect.** That is the previous section's
  ladder, read from the newcomer's side.

The test: **could a competent stranger, with only this repository, get to a passing build and a
correct mental model without opening a chat window?** Where the answer is no, that is a bug in the
tree and not in the stranger.
