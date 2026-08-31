# 198. A package manager, and the trivial install that makes a second customer possible

**Status: NOT-STARTED.** Minted 2026-08-30 by calef. *(Number provisional until the merge queue
lands it.)*

**Gate: DECISION, MILESTONE 23.** Milestone 39 (repository structure for a loosely-coupled OS)
carries the structural fork with a recommendation and no ruling, and inherits milestone 23's gate;
this block does not re-open either. What it adds is that the decision now has a consumer, which it
did not when 39 was filed as `RECORDED`.

**In brief.** calef, 2026-08-30: *"I don't think we expose nife to third parties (aka other
customers) until we have a package manager and a trivial install process."* And, in the same breath,
that he wants it **early, to make our own lives easier**.

Both halves matter and they point the same way.

## This is a precondition on principle 1, not an item under it

AGENTS.md ranks work by the shortest path to a system a customer runs. As of 2026-08-30 that path is
vacant, and the reason is now two reasons: the first customer's deadline passed and they went to
borg over SSH, **and this system could not accept a second one if it appeared.**

That makes packaging structurally different from the milestones it sits beside. It is not on the
customer path; **it is the door.** A roadmap that ranks by the shortest path to a customer while
being unable to take one is ranking against a door it has not built.

## The second half is the one that earns it early

The third-party argument alone would justify deferring this indefinitely, since there is no third
party and no date for one. **The reason to do it early is that the builders are the ones paying for
its absence today**, hand-wiring per program what a package would install once:

- Milestone 40 (documentation as a system service) already ships *"installed by the package that
  owns it"* and its per-package index shard, against no package that exists.
- Milestone 47's conclusion is that **installing a program is granting it into a namespace**, which
  is a packaging statement with no packager.
- `crates/system_initializer` spends a capability through a syscall per program, per architecture,
  and every new program edits it.
- Milestone 150 (adding a program should not need eight hand-maintained lists) is the same complaint
  from the other end, and the count is the argument: **eight** lists, hand-maintained, per program.

## What already exists to build on, so this does not start cold

- **Milestone 39** has the structural recommendation: monorepo now with the distribution as a
  separate manifest repo, executed as multiple workspaces.
- **`design/haiku-bfs-and-packages.md`** is the prior art the roadmap already tells you to read
  first. Haiku's `packagefs` **activates** packages rather than installing them, composing a
  filesystem view from read-only package files rather than letting installers mutate shared
  directories. It arrived near milestone 47's conclusion from an entirely different motive, atomic
  and rollback-able installs, which is the useful kind of convergence.
- **`design/what-a-distribution-packages.md`** is the speculation about the units, and is explicitly
  labelled as speculation.
- **DECISIONS §135** (running GPL software is aggregation) makes packages the channel for copyleft,
  so this milestone is also what unblocks `git` and `nano` arriving the honest way rather than being
  built into an image.

## What "trivial install" has to mean, and it is the harder half

A package manager without an install story is a mechanism nobody reaches. This block deliberately
does not specify one, because nothing here has met a stranger yet, but it names the constraint:
**a person with hardware and no prior knowledge of this project reaches a running system.** That is
principle 3's test applied to running rather than to building, and today the answer is a
`cargo xtask` invocation on a development machine, which is not an install.

## BUGS

- **This block prices nothing.** A package manager is a large piece of work and the estimate is not
  attempted; the sequencing claim is that it gates a customer, not that it is cheap.
- **It does not decide the format, the activation shape, or the repository split.** Those are
  milestone 39's and `design/haiku-bfs-and-packages.md`'s, and jumping to them here would be exactly
  the abstraction-ahead-of-requirement this tree refuses.
- **"Trivial install" is undefined on purpose and that is a real gap**, not a subtlety. Nobody has
  written what a stranger's first ten minutes look like, and until somebody does, this milestone's
  second half cannot be completed or even scoped.
- **nife cannot build software**, so a package is a thing produced by a host toolchain and consumed
  by the target. Every packaging idea borrowed from a self-hosting system needs that translation
  checked rather than assumed.
