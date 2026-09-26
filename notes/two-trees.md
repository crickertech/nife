# A shell holding two trees (milestone 154's live half)

*Provisional name for this note, like everything new in it; naming is calef's.*

Milestone 154 (a process that holds two directory capabilities) proved the mechanism first: one
confined program, two `fs_subtree_caretaker`s, `/a/x` and `/b/y` both resolving and `/a/../b`
refused (`kernel/src/user/multi_dir_namespace_tests.rs`, the `fs_test_client` witness). §126 (a real,
single, moving cwd) then decided how a two-grant holder moves. This note is the half after that: the
real `swish` builtins and the per-command grant planner working across two trees, and the one
question still between that and a live interactive shell.

The pure half is `crates/grant_plan` (`Holdings::anchor`, `resolve_steps`, `render`, and a `which`
on `FileGrant` and `DirGrant`); the requests are `components/src/swish.rs` (`Nav`, `Tree`, the
`two_trees` witness); the guest proof is
`a_shell_holding_two_trees_moves_between_them_and_crosses_neither`.

## One anchor, used by everything that resolves a path

Before this there were three resolvers: `Holdings::resolve` (labels, then binds), the shell's own
`plan_path`/`walk_steps` (binds only), and the planner's `designate` (neither: an absolute token was
walked literally from the root). They agreed only because no shell held two trees and nobody had
typed `wc /recent/x` with `recent` bound.

`Holdings::anchor(from_root, steps)` is now the one place a step sequence learns where it starts:

| the token | starts at | steps left |
|---|---|---|
| relative | where the shell stands, in the tree it stands in | all of them |
| `/<label>/...`, two-grant shell | that tree's root | all but the label |
| `/<bound name>/...` | the bound position, in its tree | all but the name |
| any other absolute, one-grant shell | the sole root | all of them |
| any other absolute, two-grant shell | refused, `NotAName` | |

`plan_path`, `walk_steps` and `designate` all go through it. The shell opens handles for the
anchor's own position from its tree's root and then walks the steps left. So `..` past a bound name
climbs the real tree, as milestone 47 (navigation and naming) built it, and `/a/../b` meets `a`'s
root with nothing to pop.

## A handle means nothing without its endpoint

Two caretakers number their handles independently. Handle 3 from `a` and handle 3 from `b` are
different directories, and a request that sends one on the other's endpoint names the wrong
directory rather than failing. So a `Walk` carries the `Tree` (slot and rights) it walked in, and
every request on a walk's handles goes to `name_call_in(w.tree, ...)`. `Nav::name_call` without a
tree means the tree the shell stands in, which is what `here()` is a handle in.

The system's own files are pinned to the first tree whichever tree the shell stands in: `apropos`
opens the manual store there, and `caps <image>` reads the activation table there. A second tree is
somebody else's files, and a search that answered differently after `cd /b` would be a worse tool.

## What prints a position prints its label

A two-grant shell has no unlabeled root, so `/logs` names nothing in it. `pwd`, a directory grant's
`caps` preview and a bound name's row all go through `Holdings::render`, which leads with the label:
`/b/logs`. Each rendering resolves back to the same `(which, pos)`, which a host test holds.

## EXAMPLES

In the two-tree witness (grant `a` over the fixture's `sub`, `b` over `other`), each line is typed
through the prompt's own `builtin`:

```
pwd                  /a
ls                   inner ...          (no secret)
cd /b                                   pwd is /b
ls                   secret ...         (no inner)
cd ..                refused: at your root, and pwd is still /b
cd /a/../b           refused: at your root, and pwd is still /b
cd /secret           refused: not a name (no unlabeled root)
wc < /b/secret       planned with which = b, opened in b, reads the secret's body
rm /b/secret         planned, then refused at delivery (see BUGS)
bind /b bee          ls /bee lists b
bind /b a            refused: a grant label cannot be rebound
cd                   pwd is /a (home is where it started, §126)
```

## How an interactive shell would learn it holds a second grant (PROPOSED)

The witness is told its labels by its own code and its slots by its wiring. An interactive shell is
told neither: `_start`'s three words are the role, the directory rights and the clock slot, and
`holdings()` answers `second: None` whenever `Nav::second` is unset, which the interactive path
never sets. What is being decided is how presence, the two labels and the second tree's rights reach
it. Blocked until then: a live interactive two-grant shell, and so the boot path's own verification
under `script/swish-check`.

1. **Presence by a named slot, labels and rights from a shared constant** (recommended). Init puts
   the second endpoint at a named slot (as `RUN_UNVOUCHED_SLOT` and `NETWORK_SLOT` do) and the shell
   probes it at `_start`, the one moment the probe is sound. The policy (subtree name, both labels,
   rights) becomes one `const` in a crate both depend on, per the rule that anything two binaries
   agree on is a crate, replacing the `second_dir` parameter every entry point passes `None` to. It
   spends no wire word and moves the clock back to a fixed position. Its cost: a policy change is a
   rebuild, which is already true of the `&'static str` the boot takes today.
2. **A read-only page at a named slot**, carrying labels and rights: the shape of §111 (inert
   configuration is a validated page). More general, since a boot could choose at run time, and it is a layout two programs must
   agree on for a question nothing yet asks at run time.
3. **A fourth `START` word, or the clock slot and a second-dir slot packed into `x2`.** It changes the
   entry ABI of every role for one role's need, and it still carries no labels.

Two facts belong beside the choice, because they bear on the policy question (what the second tree
*is*) more than on the transport. The interactive shell's first tree is the whole filesystem
(`g.fs_ep`, unnarrowed), so a second subtree of the same disk is not disjoint from it: it is also
reachable under the first tree's root. And §126's model has no unlabeled root, so the day a second
grant lands, every absolute path typed at that prompt needs a label (`/docs` becomes `/<label>/docs`).
Both suggest the natural second tree is a different filesystem rather than a subtree of this one.

## BUGS

- `rm` into the second tree is refused at delivery. The progenitor builds a directory grant's
  caretaker by descending from the one filesystem it holds, and the spawn words say nothing about
  which tree. Sending them would hand the program a same-named directory in the first tree, so
  `dir_grant` refuses instead. Carrying the tree on the wire only has a sender once an interactive
  shell holds two trees, so it waits on the proposal above.
- `spawnproto::DIR2_BIT` still has no emitter and no decoder in init. A second directory grant
  for one *spawned* program needs a manifest declaring two directory operands, and none does. §170
  (how a foreign program is told what to do), ruled 2026-09-26 to grant a named file or directory
  per the manifest, makes the manifest the only source of one.
- The `caps` rows for a two-grant shell print slots 4 and 5, which is where the unmerged boot
  code puts them. The recommended option moves the second to a named slot, and the row moves with it.
