# 154. A process that holds two directory capabilities

**Status: PARTIAL.** Minted 2026-08-23, proposed independently by two milestones that converge
on the same gap: milestone 47's `bind` ("It is blocked on a second grant") and milestone 64's
`File::open` fork ("tier two, anything that traverses, needs a namespace to resolve *against*, and
that is 47's unbuilt half"). Both name the identical missing primitive; this gives it one home
instead of two. **Built 2026-08-23**: the core mechanism (a process holding and resolving against
two directory capabilities at once), proven end to end on both ISAs. **Still open**, named below:
extending `caps`'s display, wiring a second grant into the real interactive boot, and the
shell-to-init spawn-protocol encoding for a second grant.

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

## What was built

Two second-level pieces, host-tested and guest-tested rather than left as design:

- **`grant_plan::nav::TwoRoots`** (provisional name), pure and host-tested in `crates/grant_plan`:
  composes exactly two labeled directory roots. Each label selects one grant's root by an exact
  match on an absolute path's first component; everything after that resolves through the
  existing `Cwd::apply`/`Cwd::ascend`, unmodified. That is the whole mechanism, and it is why
  `/a/../b` refuses for free: selecting `a` leaves nothing above `a`'s own root to pop, so `b` is
  never reached to be a question. It is deliberately **not** `bind`: it composes two fixed labels,
  not an ordered, shadowable union, and 47's four open questions (shadowing, enumeration, whether
  `$PATH` survives as a string) are untouched.
- **`kernel::user::fs_service::start_granted_two_dirs`**, the endowment mechanism itself: wires a
  second `fs_subtree_caretaker` alongside the first, for one confined program, and delivers both
  narrowed endpoints into two distinct cspace slots (slot 0 is always the first grant, slot 1 the
  second, and that ordering **is** the spawn-protocol position this milestone decides, deliberately
  the smallest possible answer rather than a new wire word). Both caretakers share the one FS
  server a boot has, and both narrowed endpoints map the same shared file-channel frame, safe for
  the reason `narrow_dir`'s own doc already gives one level narrower: the confined program is one
  thread of control with at most one `CALL` in flight.
- **The guest proof**, `kernel/src/user/multi_dir_namespace_tests.rs` (one module for both ISAs,
  `dir_capability_tests`' reason): a new `fs_test_client` role (`ROLE_TWO_DIR`) holds both grants
  at once, told nothing beyond which cspace slot is which. It proves the deliverable literally:
  `/a/inner` and `/b/secret` (the roadmap block's `/a/x` and `/b/y`) each resolve through
  `TwoRoots` and then open for real over the caretaker that resolution named; `/a/../b` is refused
  by `TwoRoots::resolve` before any request is sent; and, independent of `TwoRoots` entirely, grant
  A's endpoint cannot open the name that exists only in grant B's subtree and the reverse, which is
  the wire-level witness that the endpoint is the boundary (notes/dir-capability.md's structural
  finding), demonstrated here with two live caretakers instead of inferred from one.

## What is still open, named rather than decided here

- **`caps` was not extended.** The shell's own endowment display (`grant_plan::Holdings`,
  `crates/swish::write_holdings`) models "a directory capability" as one `bool`, and that field
  also drives `plan_stage`/`redirect_target`'s decision about what a child gets when a command
  names a file. Turning it into "N directories" forces exactly the ambiguity 47's four open
  questions already name as undecided (which grant a bare relative name resolves against, whether
  two grants shadow or refuse ambiguously): deciding that now would be answering 47's question
  inside 154's own gate, and this milestone's gate is NONE precisely because nothing here was
  supposed to be a design fork. Recorded as a finding for 47 rather than decided here. The
  mechanism this milestone built (`TwoRoots`, `start_granted_two_dirs`) does not depend on that
  question being answered, so nothing here is blocked on it, only `caps`' own output is.
- **The interactive shell still holds at most one directory capability.** Wiring a second grant
  into the real boot needs a boot-time decision about what the second subtree even *is*, which is
  policy rather than mechanism and is calef's to make, not a lane's.
- **The shell-to-init spawn-protocol encoding remains unbuilt**, as this block already said before
  anything here was built: `grant_plan::spawnproto`'s `DIR_BIT`/`GRANT_WORDS` carry exactly one
  directory grant today, and a second would need either a second bit or a count, decided by
  whoever wires an interactive `bind` against a real second grant.
