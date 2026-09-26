---
status: DECIDED
raised: 2026-09-26
decided: 2026-09-26
ratified_by: calef
---

# 221. The boot prompt is the owner's console

Raised 2026-09-26 (UTC) by milestone 198 (a package manager) rung 3a's installer lane, as
[notes/who-may-write-the-activation-set.md](../../notes/who-may-write-the-activation-set.md), and
answered by calef the same day. Written by the lane `milestone/198-owner-console` on the
maintainer's instruction, because §219 (how the shell names an installed program to the spawner) is
at its prose cap. *(Section number provisional until the merge queue lands it.)*

## The ruling

calef, 2026-09-26 (UTC), *"Yes"* to both of the maintainer's questions:

1. The boot prompt is the owner's root console. The shell on the console before any login is
   the machine owner's, and whoever holds it is the owner, as with single-user or recovery mode
   elsewhere. It keeps §219's gate D2: the grant `crates/system_initializer` made provisionally on
   2026-09-26 is the rule. It may write the activation set, including vouching for bytes no source
   carries (§195 (a reviewed recipe vouches for a package) clause 3).
2. A `login` session gets D2 only for an identity on an owner-written list, empty by default.
   Until this ruling every session got it. This is calef's multi-user consequence in §219 made a
   mechanism: a machine can have users who may not run new native code.

## What it answers

The note's first question: is the boot prompt allowed to vouch its own bytes? Yes, so the note's
option A holds. The note recorded a hole: the prompt can write `activation/` directly, because it
holds the file service's root endpoint and the server cannot tell its clients apart. That hole is
closed by definition rather than by a filter. The session that can write the table is the owner,
and the owner may. Options B (a root caretaker in front of the shell) and C (the server reserves the name)
are not built, and the note's second question lapses.

The recorded cost. On a machine whose console strangers can reach, anyone at the console is the
owner. That is the same trade single-user mode makes, and it is the reason a machine that must
defend its console needs the console itself defended, not a filter behind it.

## What was built on it

Lane `milestone/198-owner-console`, 2026-09-26. Names provisional.

- The list. `login_protocol::RUN_UNVOUCHED_LIST`, a file named `may-run-unvouched` at the root
  of the file service, one identity per line, `#` comments allowed. `login` reads it on every
  login, after authentication, and delegates D2 only to a listed identity. No file, an unreadable
  one, or one over a page lists nobody. The root is where this tree keeps what it knows about an
  identity (§117 (a principal's subtree is named by its identity)). It is also the one place a
  session cannot name, because a session is confined to its own subtree. The owner edits it at the boot
  prompt with `echo chris >> may-run-unvouched`.
- `vouch <path>`. The boot prompt asks the progenitor to record the file's digest in a new
  activation generation, under the path's last component, with `owner` in the package column
  (`activation_set::OWNER`). The bytes then run vouched with the installed manifest until a
  manifest travels in the executable (§197 (a package is one archive file), M2). A vouch is a
  generation, so `package rollback` undoes it. The proposal it was promoted from,
  `vouch-for-a-local-build`, recommended exactly that shape.

## What else was considered for the list, and why each lost

- An attribute in the credential store. It holds secrets and is sealed at boot
  (`components/src/credentialer.rs`): nothing can write it afterwards, which is its whole security
  argument. An owner's edit would need a reboot and a provisioning path to carry it.
- A file in each identity's own subtree. The session can write its own subtree, so it would
  grant itself the capability.
- A list the progenitor reads at boot and hands `login`. It works, and an owner's edit then
  waits for a reboot. Reading at login costs one open and one read of a page per login, and `login`
  already holds the root endpoint.
- Every session, as before. Refused by the ruling.

## What stays open

- Every activation verb is the spawn endpoint's. `install`, `remove`, `rollback` and `vouch` go
  to whoever holds it, which is the boot prompt alone, so today that is the owner. A session given a
  spawn endpoint would be the owner too. Before one is, the verbs need a presentation of their own,
  the way D2's `RUN_UNVOUCHED_BIT` has one. Recorded in `grant_plan::spawnproto`'s BUGS.
- Until the manifest note lands, a vouched build holds `uptime`'s manifest
  (`grant_plan::INSTALLED_MANIFEST_OF`): the output and nothing else. So vouching a build today
  narrows it (an unvouched one also gets the clock and configuration pages). The grant is the
  manifest's; the manifest is a stand-in.
