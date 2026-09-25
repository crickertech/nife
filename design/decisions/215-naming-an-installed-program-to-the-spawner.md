---
status: PROPOSED
raised: 2026-09-24
---

# 215. How the shell names an installed program to the spawner

Raised 2026-09-24 by milestone 198 (a package manager, and the trivial install that makes a second
customer possible)'s rung 3a consumer lane (`milestone/198-rung-3a-consumer`), which built the fetch
and the digest check and stopped here. *(Section number provisional until the merge queue lands it.
Written by a lane, on the maintainer's instruction to write this fork up rather than invent it.)*

## What is being decided

§208 (installing a package is granting it, and the activation set is versioned) ruled that
installing makes a package's digest and manifest *spawnable*. It also named the cost: "naming a
program that was not there at boot is a change two programs agree on." This section is that change.
**One question: when a person types the name of an installed program, what travels from the shell to
the process that builds it?**

Today a closed enum answers it. The shell resolves a name through `grant_plan::Prog` and sends its
integer id as word 0 of a `spawnproto` request (`crates/grant_plan/src/spawnproto.rs`). The
progenitor indexes `progs[p.id()]`, a table it filled once at boot from the measured archive
(`crates/system_initializer/src/lib.rs`, `spawn_service`). An installed program has no variant, no
id, and no row.

## Is the premise true? Two corrections to the record first

- **The progenitor keeps the file service for the life of the boot.** §208 and milestone 507
  (installing a package: mutate, compose, or widen what can be spawned) both say "the spawner gives
  the file service away", and `system_initializer`'s module documentation says so too. The code
  stopped doing that in milestone 31 (a capability shell) phase 3 (2026-08-17), which kept `WRITE | GRANT` on the RedoxFS
  service so the progenitor could build `fs_subtree_caretaker`s (`Channels.fs`, and the comment
  "**The filesystem stays**"). So the spawner *can* read an installed program today. What it cannot
  do is be asked for one. That makes 507's "hard part" half a fact and half stale, and it is why the
  fork below is about naming rather than reach.
- **`PROG_COUNT` is 14, not 13.** Both earlier records quote 13.

## The options

| | What travels | Who builds the process | Wire change | Measured cost |
|---|---|---|---|---|
| **A. A name on the spawn request** | A new `spawnproto` flag (`NAME_BIT`, provisional) saying "two more `SEND`s carry a name of at most 32 bytes" | The progenitor: it looks the name up in the activation table on RedoxFS, reads the package with the file service it already holds, re-checks the member digest, and builds with `build_child` as for any program | Yes: the shell and the progenitor | No new capability slot at rest or at peak (the file service is already one of the fifteen). Reading `uptime` costs 22 pages (89,168 bytes stripped) while it builds, against `INIT_OWN_PAGES` = 128 and a job region of 40 |
| **B. A launcher in the sealed namespace** | Nothing new: `package` (provisional) is one more `Prog` row, declared the way milestone 150 (adding a program should not need eight hand-maintained lists) declares every row | The launcher, from bytes it read, with a `--mem` grant, as `login` builds sessions (milestone 233 (`login` dies on every boot)) | No | No argument vector exists (milestone 205 (how a foreign program is told what to do)), so the shell cannot tell the launcher which package: the integer argument would have to be an index. The child gets at most what the launcher holds, so an installed program's authority is capped by one manifest, not its own. Its death goes to the launcher, not to `job_undertaker`, and `^C` and pipelines would need re-plumbing through it |
| **C. Ids assigned at install** | The same integer word, now `>= PROG_COUNT` for an installed program | The progenitor, indexing the activation table | Shape unchanged, meaning changed | The shell must read the activation table to resolve a name, which it does not hold today. An id means a different program after a rollback, so a spawn in flight across a rollback runs whatever the new set put there. Milestone 150 pinned the shipped wire ids, and this reuses their space |

## The seven questions

1. **What else was considered, and why did each lose?** Nothing loses yet, because this is the
   syscall-adjacent kind of fork AGENTS.md says to give options on. What each costs is above. B's
   argument problem is the sharpest: it cannot be built well until milestone 205 lands, and it
   builds a second spawner beside the progenitor, which §208 argued against when it chose one
   authority over two.
2. **What does the tree already do?** Two analogues point different ways. `DIR_BIT` is A's exact
   shape: data too big for a word, carried by "expect two more `SEND`s". Milestone 47 (navigation
   and naming)'s `PATH` lane priced `NAME_BIT` in those words and stopped because it was a wire
   change. `login` is B's shape: a program that builds processes from a blob it was handed.
3. **Prior art outside the tree.** Recalled, not re-read this session, and marked so: Fuchsia
   resolves components by URL string at the resolver (A's shape); Plan 9's `exec` takes a path in
   the caller's namespace (A, with the namespace as the table); Genode's Sculpt routes a downloaded
   component through a runtime configuration (B-like, one launcher). Milestone 507's own table read
   the Fuchsia and Sculpt pages and cited them.
4. **Is the premise true?** Checked above. It moved: reach was the stated blocker and it is not one.
5. **What does each cost, measured?** The table's last column. The slot count is from the comment
   beside `spawn_dir_grant` (peak fifteen of sixteen). The page counts are
   `system_initializer`'s constants and `cargo xtask package packages/uptime.recipe` on 2026-09-24.
6. **How reversible, and who has acted on it?** Nobody outside this repository has a shell or a
   progenitor, and both ship in one image, so any of the three can be changed by one image. That is
   less expensive than an external wire format. It is still one: every future shell and every
   future spawner is written against it, and a third-party shell is what milestone 39 (repository structure for a loosely-coupled OS)'s split
   exists to allow.
7. **Would we still choose it if all three cost the same?** Not asked of a winner, because none is
   named. If calef picks A, the non-effort case is §208's own: one authority builds processes and
   vouches for them, and installing widens what that authority may be asked for.

## The two questions A would bring with it

Both are reversible until something outside this repository reads them, so each carries a
recommendation. They are not blocking on their own; they block only after A.

- **The activation table's shape.** Recommended: text, one `<name> <package stem> <digest>` line per
  active program, a file per generation plus a one-line `current` naming the generation. That is
  `measured_boot`'s manifest shape with one column added, which is what the progenitor already
  parses, and it makes rollback rewriting one line. A binary table would be a second parser on the
  progenitor's path for no measured gain.
- **Where a program's manifest travels.** Still §197 (a package is one archive file)'s open question,
  unanswered here. Rung 3a's `uptime` needs only an output sink, so a first cut can refuse any
  installed program whose manifest asks for more than `Prog::Uptime`'s, and say so in a `BUGS`
  section, rather than answer §197 by accident.

## What is blocked, and what happens if calef says no

**Blocked until answered**: installing, running, surviving a reboot, rolling back and removing,
which is all of rung 3a after "verified by digest". The fetch and the check are built and gated
(notes/packages.md). A "no" to all three leaves nife able to verify a package it cannot run, which is
milestone 507's "no to all three" outcome one step later: rung 3 stops, and with it fatal risk 8's
only route to a verdict.

**Blocked independently, and not this section's**: the booted system has no network. The progenitor
never builds `net_stack`, so nothing at the prompt holds a `Stack` capability, and every network test
today runs in the kernel's test harness. That is filed as the proposal
`design/roadmap/proposals/the-booted-system-has-no-network.md`.
