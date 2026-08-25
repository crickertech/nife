# 175. Split `user/`: `components/` for services, `fixtures/` for test and benchmark programs

**Status: NOT-STARTED.** Minted 2026-08-25, from calef asking when nife development should split
into different repositories. [Milestone 39](39-repository-structure.md)'s own analysis already
answered the bigger question (monorepo now, distribution as a separate manifest repo later, gated
on milestone 23 forcing it, and 23's residual piece was declined for want of a customer rather than
forcing anything, the same shape as [DECISIONS §105](../decisions/105-thread-spawn-decline-for-now.md)):
not yet, by the tree's own stated trigger ("when a component is first built outside this tree, or a
binary is first distributed to someone who cannot rebuild it"), which has not happened. This
milestone is 39's own **"cheap first move, which commits to none of the four options"**, re-scoped
against how much bigger the strain has gotten since 39 last measured it.

**Gate: NONE.** A directory restructure and an `xtask`/initrd-packing update, no design decision
and no kernel change.

## The strain, measured fresh rather than trusted from 39's own numbers

Milestone 39 measured `user/` on 2026-07-30 (28 `[[bin]]` targets, 9,324 lines) and
`design/what-a-distribution-packages.md` re-measured it four days later (48 targets, 16,309 lines),
already close to doubled. Measured again now: **65 `[[bin]]` targets, 25,277 lines.** The strain 39
named has not stopped: `user/` is still one crate doing two incompatible jobs, a collection of
programs and a shared library (`net_transport` and others still live as modules beside the programs
that consume them), so no component can depend on part of it without every other program rebuilding
when any shared module changes.

## What "components" and "fixtures" means, per milestone 39's own naming argument

Not "daemons": a Unix daemon is defined by what it detaches from (no controlling terminal, inherited
ambient authority, a pid file), and nife deliberately has none of those
([DECISIONS §10](../decisions/10-capability-microkernel.md)). Milestone 39's vocabulary, already
argued for and not re-litigated here: a **component** is the shippable unit (a binary plus its
manifest), a **service** is what it offers over a contract. "Server" stays a fine role word inside a
component; "daemon" does not appear.

**This milestone does not pre-classify all 65 programs**, deliberately: milestone 39 named
illustrative examples on a tree less than half this size (`net_stack`, `display`, `compositor`,
`line_editor` as components; `heeder`, `spinner`, `flaky`, `allocator_exerciser`, `worker`,
`builder`, `coremark`, `os_primitives_benchmarker` as fixtures), and a fresh, correct classification
of the current 65 is real work for whoever builds this, not something to guess at from names in a
minting doc. **A third category may turn out to be real and worth naming**: several current programs
are neither long-running services nor test-only fixtures but interactive, user-invoked tools
(`wc`, `rm`, `date`, `ps`, `pgrep`, `watch`, among others) closer to what a Unix `/bin` holds.
Whoever builds this should check whether milestone 39's two-way split still fits the tree or whether
a third directory (`tools/`, or similar, not decided here) is honest about what's actually there,
rather than forcing a fit.

## What it needs

- An audit of all 65 current `[[bin]]` targets, classified by what they actually are (checked
  against each program's own module doc, not guessed from its name).
- The directory move itself, plus updating `xtask`'s `--bin` lists and the initrd packing that reads
  them.
- **Done as one mechanical commit, audited, not folded into feature work.** Milestone 39's own
  warning, kept verbatim because the failure it names already happened once: "a union merge in
  exactly that code dropped a `--bin` flag on 2026-07-29 and duplicated a loop header the same day."
  A rename touching two generated lists (`xtask`'s bin list, the initrd packer) is exactly the shape
  of change that silently drops an entry when it collides with unrelated work landing at the same
  time.
- Re-checking `crates/` against milestone 39's own three-audience split (kernel proof crates, wire
  contracts, userspace runtime) is explicitly out of scope here: 39 named it as a separate strain,
  and folding it into this milestone would undo the "one thing at a time" discipline the split is
  supposed to buy.

## Why it matters

Directly: ends the crate-is-both-a-program-collection-and-a-library problem 39 named, so a component
can express "I need this dependency but not that one" without handing it to all 65 siblings, and
milestone 39's own §10/§46 packaging observations (the manifest and measured-boot hash are already
three quarters of a package format) get a real home to grow into if `basalt` ever moves past being a
placeholder.

Indirectly: it is evidence for re-reading milestone 39's bigger recommendation (repo split, gated on
milestone 23) against current numbers, which its own text asks for, without executing that
recommendation itself.

## What this does not decide

Whether nife ever splits into multiple repositories (milestone 39's own question, still correctly
gated and still correctly undecided); the exact directory name for the third category if one turns
out to be real (`tools/`, `bin/`, or something else); and whether `net_transport` and any other
still-module shared code gets lifted into its own crate as part of this move or left for a follow-on,
matching how `virtio`/`socket_proto`/`supervision_proto` were each lifted separately under Rule 7.
