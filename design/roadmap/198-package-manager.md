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

## Scoped 2026-09-19

A scoping lane (`milestone/198-package-manager-scoping`) built nothing and wrote the forks as
proposals, one decision each, for calef to rule on separately. The status does not move: nothing is
built, and the gate line above is left as minted because changing it is the first proposal's ask.

| Fork | Proposal | Shape |
|---|---|---|
| The gate | [what-the-package-manager-waits-on.md](proposals/what-the-package-manager-waits-on.md) | **Recommends** `Gate: DECISION` alone. The loop is real (198 waits on 23, whose residual waits on a customer, who waits on 198), and nothing in a first slice needs the split's timing. One real dependency was found, on a downloadable toolchain rather than on the split |
| Package format | [what-a-package-is-on-disk-and-on-the-wire.md](proposals/what-a-package-is-on-disk-and-on-the-wire.md) | Options, no winner: members of the boot archive, one archive file per package, or content-addressed. The measurement table is already a name-to-digest document |
| Activation | [installing-a-package-mutates-or-composes.md](proposals/installing-a-package-mutates-or-composes.md) | Options, no winner: mutate, compose a union view, or only widen what may be spawned. The program namespace is sealed at boot, and the spawner gives the file service away, so nothing that builds processes can read an installed program today |
| Trust (found, not briefed) | [what-vouches-for-a-package-the-image-did-not-carry.md](proposals/what-vouches-for-a-package-the-image-did-not-carry.md) | Options, no winner: the image always, a publisher's signature checked in userspace, or the owner. The measured chain makes every runtime-installed package unvouched by construction |
| Trivial install | [what-trivial-install-means.md](proposals/what-trivial-install-means.md) | **Recommends** a QEMU run bundle a stranger can use with no Rust toolchain as the first rung, and an x86-64 UEFI PC as the second. Carries the proposed first slice |

**The proposed first slice needs none of the three irreversible rulings**: packages as host-side
recipes, image composition from a declared set, and a run bundle tested by the stranger harness and
not published until calef says so. Its details and what it unblocks are in the trivial-install
proposal.

## BUGS

- **This block prices nothing.** A package manager is a large piece of work and the estimate is not
  attempted; the sequencing claim is that it gates a customer, not that it is cheap.
- **It does not decide the format, the activation shape, or the repository split.** The scoping
  lane found the split's timing is not needed at all (see the gate proposal); the format,
  activation and trust forks are proposals awaiting calef, not decisions.
- ~~**"Trivial install" is undefined on purpose and that is a real gap**, not a subtlety. Nobody has
  written what a stranger's first ten minutes look like.~~ **Written 2026-09-19** in the
  trivial-install proposal, from the tree and from commands run that day. What remains undefined is
  calef's ruling on it.
- **No real board gives a stranger a prompt today.** x86-64 has no interactive boot (milestone 182)
  and no USB keyboard (milestone 242); radon's prompt input is unconfirmed on silicon. So the only
  interactive install this milestone can offer soon is QEMU, and that limit is outside this
  milestone's reach.
- **A third party cannot author a package without cloning this repository**, because the `nife-dev`
  toolchain, the target specifications and the linker script exist only as build steps inside it
  (`scripts/build-ripgrep.sh` is the one out-of-tree build and it needs them). §151's
  "third-party programs" needs a downloadable toolchain, which nothing tracks yet.
- **The cold build time a stranger pays was not measured**, because two other lanes were gating on
  the machine when this block was scoped. The first slice's stranger-harness run should measure it
  alongside the bundle.
- **Packages do not by themselves run `git` or `nano`.** Milestone 205 (no argument vector) and the
  raw-input primitive of milestones 169 and 170 still stand in front of both.
- **nife cannot build software**, so a package is a thing produced by a host toolchain and consumed
  by the target. Every packaging idea borrowed from a self-hosting system needs that translation
  checked rather than assumed.

## Index row

calef, 2026-08-30: no third party sees nife until there is a package manager and a trivial
install, and he wants both **early, to make our own lives easier**. That makes packaging a **precondition on principle 1's ranking function** rather than an item under it: the customer path
is vacant partly because a second customer could not be accepted if one appeared. The early half
is what earns it, since the builders pay for its absence today, hand-wiring per program what a
package would install once (milestone 40 already ships "installed by the package that owns it",
against no package). Gate: DECISION, MILESTONE 23, inherited from milestone 39, which now has a
consumer.
