# 154. A process that holds two directory capabilities

**Status: PARTIAL.** Minted 2026-08-23, proposed independently by two milestones that converge
on the same gap: milestone 47's `bind` ("It is blocked on a second grant") and milestone 64's
`File::open` fork ("tier two, anything that traverses, needs a namespace to resolve *against*, and
that is 47's unbuilt half"). Both name the identical missing primitive; this gives it one home
instead of two. Built 2026-08-23: the core mechanism (a process holding and resolving against
two directory capabilities at once), proven end to end on both ISAs. [DECISIONS
§126](../decisions/126-two-directory-cwd.md) decided 2026-08-25 how a two-grant process moves
between them: a real, single, moving `cwd`, not the shadowed-union ambiguity 47's own questions
still leave open. All three of §126's "still open" items now have a first increment, 2026-08-25
(see "What is now decided, and what is still open" below for what each does and does not yet
cover): `grant_plan::Holdings`/`caps` display the `(which, pos)` shape; `crates/system_initializer::boot`
can construct and deliver a second, disjoint directory capability at boot; and
`grant_plan::spawnproto` carries a second directory grant on the wire. Built 2026-09-26: the
shell itself holds two trees ("The shell across two trees" below): the real `swish` builtins and
the per-command grant planner resolve, walk, open and print across both, proven on the real wire.
What is left is how an *interactive* shell learns it holds a second grant, which is an architect's
call and is PROPOSED below, and what waits on it.

**Gate: DECISION.** Unminted, and a lane does not mint one. How presence, labels and rights reach an interactive shell is a
contract between init and the shell, and what the second tree should be is calef's boot-time policy
(§126 (a real, single, moving cwd) reserved it). The options, with a recommendation, are in
[notes/two-trees.md](../../notes/two-trees.md), "How an interactive shell would learn it holds a
second grant". §50 (namespace composition, not stored paths) already decided the rest, so
nothing else here is a fork.

## The gap, in both milestones' own words

47: "A shell holds one directory capability, so a namespace assembled from what it holds has
exactly one member and every bind is an alias inside one tree. The interesting case, and the only
one that pays for the mechanism, is a union of two grants... Nothing in this system grants a
second directory capability to one process. `fs_service::start_granted_dir` starts one caretaker and
hands one endpoint; a second means a second caretaker, a second slot, and a spawn-protocol position
to say which is which."

64: "Tier two, anything that traverses, needs a namespace to resolve against, and that is 47's
unbuilt half. `Path::new("assets").join("x.png")`, an absolute path, or a program wanting two
directories all land here."

## The deliverable

Both milestones already name it identically: one process, two subtrees, `/a/x` and `/b/y` both
resolving, `/a/../b` refused, and neither caretaker able to see the other's tree. Concretely:

- A second `fs_subtree_caretaker` (or equivalent), a second cspace slot, and a spawn-protocol
  position to say which directory a grant is (an endowment question, per 47's own environment
  section, "expensive" in the same sense that section already prices).
- The negative control that only a union can state: `/a/../b` refused, proving neither subtree can
  name the other's parent.
- `caps` gains a namespace section with more than one row, which 47's own text says is currently
  empty precisely because one root has one row.

## What it unblocks

- Milestone 47's `bind` falls out as a name on a `Cwd` per entry once this exists; the mount
  table itself is "the cheap half" per 47's own finding, already priced.
- Milestone 64 (enough `std` to run somebody else's crate): its tier-two `File::open`, anything
  that joins a path or wants more than the one granted directory, gets a namespace to resolve
  against.
- Milestone 47's `PATH` work, which is the same question scaled to programs rather than files,
  needs this before its own four open sub-questions are worth deciding in detail.

## What this does not decide

The spawn-protocol encoding for "which directory is which" is a real wire-format question (two
programs, the shell and init, must agree), in the same category 47's environment-variable section
already prices as reversible-but-real. Left to whoever builds this, following the existing
`DIR_BIT`/`GRANT_WORDS` precedent rather than inventing a new shape.

## What was built

Two second-level pieces, host-tested and guest-tested rather than left as design:

- `grant_plan::nav::TwoRoots` (provisional name), pure and host-tested in `crates/grant_plan`:
  composes exactly two labeled directory roots. Each label selects one grant's root by an exact
  match on an absolute path's first component; everything after that resolves through the
  existing `Cwd::apply`/`Cwd::ascend`, unmodified. That is the whole mechanism, and it is why
  `/a/../b` refuses for free: selecting `a` leaves nothing above `a`'s own root to pop, so `b` is
  never reached to be a question. It is deliberately not `bind`: it composes two fixed labels,
  not an ordered, shadowable union, and 47's four open questions (shadowing, enumeration, whether
  `$PATH` survives as a string) are untouched.
- `kernel::user::fs_service::start_granted_two_dirs`, the endowment mechanism itself: wires a
  second `fs_subtree_caretaker` alongside the first, for one confined program, and delivers both
  narrowed endpoints into two distinct cspace slots (slot 0 is always the first grant, slot 1 the
  second, and that ordering is the spawn-protocol position this milestone decides, deliberately
  the smallest possible answer rather than a new wire word). Both caretakers share the one FS
  server a boot has, and both narrowed endpoints map the same shared file-channel frame, safe for
  the reason `narrow_dir`'s own doc already gives one level narrower: the confined program is one
  thread of control with at most one `CALL` in flight.
- The guest proof, `kernel/src/user/multi_dir_namespace_tests.rs` (one module for both ISAs,
  `dir_capability_tests`' reason): a new `fs_test_client` role (`ROLE_TWO_DIR`) holds both grants
  at once, told nothing beyond which cspace slot is which. It proves the deliverable literally:
  `/a/inner` and `/b/secret` (the roadmap block's `/a/x` and `/b/y`) each resolve through
  `TwoRoots` and then open for real over the caretaker that resolution named; `/a/../b` is refused
  by `TwoRoots::resolve` before any request is sent; and, independent of `TwoRoots` entirely, grant
  A's endpoint cannot open the name that exists only in grant B's subtree and the reverse, which is
  the wire-level witness that the endpoint is the boundary (notes/dir-capability.md's structural
  finding), demonstrated here with two live caretakers instead of inferred from one.

## What is now decided, and what is still open

[DECISIONS §126](../decisions/126-two-directory-cwd.md) closed the ambiguity the first bullet
below used to name. A two-grant shell gets a real, single, moving `cwd`: state `(which, pos)`
in place of one-grant `Holdings`' bare `Cwd`, a bare relative name resolves against `pos` inside
whichever tree `which` currently names, an absolute `/a/...`/`/b/...` path both resolves and moves
between trees, and `..` at either tree's own root refuses exactly the way one-grant `Cwd::apply`
already refuses at its own root today. That is a real, single answer, not a per-caller choice, and
it does not answer 47's four open questions (unions and shadowing across more than two labeled
sources, enumeration, the compile-time-set-to-runtime-lookup gap, whether `$PATH` survives as a
string): those stay exactly as open as they were, since this decision only ever concerned two
disjoint, individually-labeled trees with one position at a time.

- `caps`'s display and `Holdings` are extended, host-tested, to §126's shape (2026-08-25).
  `grant_plan::Holdings` gains `second: Option<SecondDir>` (provisional name), carrying both
  labels and `which` beside the existing `cwd`/`pos`; a one-grant `Holdings` (`second: None`)
  resolves and prints exactly as it always has, byte for byte (pinned by
  `a_one_grant_holdings_resolves_exactly_as_before` and
  `holding_a_directory_changes_exactly_one_line_of_the_endowment`). `Holdings::resolve` is the
  `(which, pos)` combinator itself, built on a new `nav::TwoRoots::resolve_from`/`apply_from` pair
  (relative stays in the current tree, absolute crosses by label, `..` refuses at either tree's own
  root; each a host test in `crates/grant_plan/src/nav.rs`). `crates/swish::write_holdings` prints
  two directory rows and a namespace section with both labels when `second` is `Some`, marking
  which tree `cwd` currently stands in and printing the other at its own root (there is only one
  remembered position, per §126's "real, single, moving" cwd): this is milestone 154's own line,
  "`caps` gains a namespace section with more than one row", made real and tested
  (`a_second_grant_prints_two_rows_and_a_namespace_section`).

  What this increment did not reach, per-command grants carrying their tree, was closed on
  2026-09-26 by "The shell across two trees" below.

- `crates/system_initializer::boot` can construct and deliver a second, disjoint directory
  capability to the shell at boot, mechanically, for real (2026-08-25). `boot` takes a new
  `second_dir: Option<SecondDirGrant>` parameter; `Some` builds a second `fs_subtree_caretaker`
  (`build_caretaker`, the same function `spawn_service`'s dynamic `rm`-style grants already use)
  narrowed to `SecondDirGrant::name`, and delivers its endpoint into the shell's capability table
  at the slot after the filesystem pair, pushing the clock (already told to the shell numerically
  rather than assumed, per its own existing convention) one slot further out. This is the real init
  both boards run (`user/src/system_initializer.rs`, `fixtures/src/hello.rs`'s `init_boot` role), not a
  kernel-side test harness.

  Both real entry points pass `None`. What the second subtree should *be* remains calef's
  boot-time policy call (DECISIONS §126), unanswered by this increment on purpose. Two further
  gaps, recorded rather than hidden. First, this exact path is unverified against a real boot:
  `script/swish-check` is the only thing that runs a real init, nothing types a second grant
  through it, and the capability-table headroom at the point this builds a caretaker is the same
  spot a past bug already found tight (`boot`'s own `# BUGS` note says so; watch for "reaches
  userspace and prints nothing" first). Second, and this is the sharper gap: nothing tells the
  shell process it has a second grant at all. `_start`'s three `START` words (role, argument,
  clock slot) are already fully spoken for, so a shell built with a second grant today would hold
  a capability its own `Nav` has no way to learn the label or slot of. `components/src/swish.rs`'s
  `holdings()` therefore still always reports `second: None`. Closing that gap is a real
  shell-to-init wire question of its own (a fourth `START` word, or packing the clock slot and a
  second-dir slot into the same word) and deserves its own decision rather than a quick encoding
  chosen under this lane's time pressure.

- The shell-to-init spawn-protocol encoding is built (2026-08-25), following `DIR_BIT`'s own
  precedent exactly as this block said it should: `grant_plan::spawnproto` gains `DIR2_BIT` and
  `Wiring::dir2`, a second bit rather than a count, round-tripping independent of the other six
  flags (host-tested, `the_wiring_flags_do_not_collide` extended and
  `a_second_directory_grant_is_a_second_bit_not_a_count` added). `crates/system_initializer::spawn_service`'s
  init-side decode is *not yet* extended to build a second caretaker when `dir2` is set: nothing on
  the shell side ever constructs a two-directory `Endowment` to set the bit with (no manifest declares
  two directory operands), so this is the wire format alone, ahead of an emitter, in exactly the
  shape `DIR_BIT` itself was built in before anything could construct a one-directory grant either.
## The shell across two trees, built 2026-09-26

Lane `milestone/154-two-directories`. The detail is [notes/two-trees.md](../../notes/two-trees.md)
(provisional name).

- One resolver. `grant_plan::Holdings::anchor` (provisional) decides where a step sequence
  starts: the current tree, a grant label's root, a bound name's position, or the sole root. The
  shell's walk, its `plan_path` and the planner's `designate` all use it. `designate` used to root
  every absolute token literally, so `wc /recent/x` ignored a bind that `ls /recent` honoured; that
  is fixed as a consequence, and host-tested.
- Per-command grants carry their tree. `FileGrant` and `DirGrant` gain a required `which`, so
  `wc < /b/secret` plans a grant against `b` and the shell opens it in `b`.
- A walk carries its endpoint. `swish`'s `Nav` holds a second `Tree` (slot and rights), and every
  request on a walk's handles goes to that walk's tree, because two caretakers number handles
  independently. The manual and the activation table stay pinned to the first tree.
- Positions print with their label (`Holdings::render`): `pwd`, a directory grant's preview and
  a bound row say `/b/logs`, which resolves back to the same place.
- A correction. `nav::Cwd::ascend` left the popped component behind the depth, so the derived
  `PartialEq` called two routes to one position different. It now zeroes it.
- The guest proof is a new `swish` role (`ROLE_TWO_TREES`, provisional) over
  `start_granted_two_dirs`' two live caretakers:
  `a_shell_holding_two_trees_moves_between_them_and_crosses_neither`, an exact thirteen-bit set with
  four never-allowed bits. One module for every ISA with an initrd, `dir_capability_tests`' reason.

## Follow-on

- **Outstanding.** PROPOSED, an architect's call: how an interactive shell learns it holds a second
  grant (presence, both labels and the second tree's rights). Options and a recommendation, a named
  slot probed at `_start` with the policy in one shared constant, are in notes/two-trees.md. Blocked
  on it: a live interactive two-grant shell, since every real entry point passes `second_dir: None`
  and the interactive path never sets a second tree; and with that, the boot path's verification
  under `script/swish-check`, which `crates/system_initializer` records in the code. Checked
  2026-09-26.
- **Decision.** What the second tree should be is calef's boot-time policy, reserved by
  `design/decisions/126-two-directory-cwd.md` and unanswered there. The note records two facts that
  bear on it: the interactive first tree is the whole filesystem, so a
  subtree of it is not disjoint, and a second grant puts a label on every absolute path typed at
  that prompt.
- **Recorded.** Two limitations wait on the proposal, both in `notes/two-trees.md`'s BUGS. `rm`
  into the second tree is refused at delivery (and at `dir_grant` in `components/src/swish.rs`),
  because the spawn words do not say which tree and the progenitor would build the caretaker in the
  first. And `spawnproto::DIR2_BIT` has no emitter and no decoder in init: no manifest declares two
  directory operands, and §170 (how a foreign program is told what to do), as ruled 2026-09-26,
  grants a named directory per the manifest.
- **Done.** `bind` was built by milestone 47 (navigation and naming) on 2026-08-26 and now points
  into either tree: the two-tree witness binds a name to `/b` and lists `b` through it, and binding
  a grant label is refused.
- **Milestone 47.** The four open namespace questions (shadowing across more than two labeled
  sources, enumeration, the compile-time-to-runtime lookup gap, and whether `$PATH` survives as a
  string) are untouched here and stay 47's, which its own block confirms.
- **Recorded.** The two-root type, the second-directory field and its grant all ship as provisional
  names, said so where they live, and naming is calef's. So do this lane's: `Holdings::anchor`,
  `resolve_steps`, `render`, `PLACE_MAX`, `swish`'s `Tree` and `ROLE_TWO_TREES`, the
  `fixture::twotrees` bits, and notes/two-trees.md.

## Index row

Milestones 47 (`bind`) and 64 (tier-two `File::open`) independently named the same missing
primitive: nothing today grants a second directory capability to one process. Built
2026-08-23: the core mechanism (`grant_plan::nav::TwoRoots`, `fs_service::start_granted_two_dirs`);
built 2026-09-26: the real shell and its grant planner across two trees, on the real wire. Still
open: how an interactive shell learns of a second grant (PROPOSED), and the live boot behind it.
