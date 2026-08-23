# 154. A process that holds two directory capabilities

**Status: NOT-STARTED.** Minted 2026-08-23, proposed independently by two milestones that converge
on the same gap: milestone 47's `bind` ("It is blocked on a second grant") and milestone 64's
`File::open` fork ("tier two, anything that traverses, needs a namespace to resolve *against*, and
that is 47's unbuilt half"). Both name the identical missing primitive; this gives it one home
instead of two.

**Gate: NONE.** Nothing here is a design fork; §50 already decided namespace composition over
stored paths, and 47's absolute-paths work already proved the resolver lives in the client's
runtime (built 2026-08-18). What's missing is mechanical: nothing in the system today grants a
*second* directory capability to one process.

## The gap, in both milestones' own words

**47**: "A shell holds **one** directory capability, so a namespace assembled from what it holds has
exactly one member and every bind is an alias inside one tree. The interesting case, and the only
one that pays for the mechanism, is a union of **two** grants... Nothing in this system grants a
second directory capability to one process. `fs_service::start_granted_dir` starts one caretaker and
hands one endpoint; a second means a second caretaker, a second slot, and a spawn-protocol position
to say which is which."

**64**: "Tier two, anything that traverses, needs a namespace to resolve against, and that is 47's
unbuilt half. `Path::new("assets").join("x.png")`, an absolute path, or a program wanting two
directories all land here."

## The deliverable

Both milestones already name it identically: **one process, two subtrees, `/a/x` and `/b/y` both
resolving, `/a/../b` refused, and neither caretaker able to see the other's tree.** Concretely:

- A second `fs_subtree_caretaker` (or equivalent), a second cspace slot, and a spawn-protocol
  position to say which directory a grant is (an endowment question, per 47's own environment
  section, "expensive" in the same sense that section already prices).
- The negative control that only a union can state: `/a/../b` refused, proving neither subtree can
  name the other's parent.
- `caps` gains a namespace section with more than one row, which 47's own text says is currently
  empty precisely because one root has one row.

## What it unblocks

- **Milestone 47's `bind`** falls out as a name on a `Cwd` per entry once this exists; the mount
  table itself is "the cheap half" per 47's own finding, already priced.
- **Milestone 64's tier-two `File::open`** (anything that joins a path or wants more than the one
  granted directory) gets a namespace to resolve against.
- **Milestone 47's `PATH`** work, which is the same question scaled to programs rather than files,
  needs this before its own four open sub-questions are worth deciding in detail.

## What this does not decide

The spawn-protocol encoding for "which directory is which" is a real wire-format question (two
programs, the shell and init, must agree), in the same category 47's environment-variable section
already prices as reversible-but-real. Left to whoever builds this, following the existing
`DIR_BIT`/`GRANT_WORDS` precedent rather than inventing a new shape.
