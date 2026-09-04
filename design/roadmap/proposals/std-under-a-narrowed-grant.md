# Every `std` test grants the mount root, so a walk that over-asks for rights still passes

**Status: PROPOSED 2026-09-03.** Written by the milestone 247 sweep, from milestone 122's block.

**Gate: NONE.** Both halves exist. `fs_subtree_caretaker` narrows a grant today, and the `std`
platform abstraction layer (PAL) runs under the test runner on every architecture. Nothing is owed
by another milestone and no decision is waiting.

**In brief.** Spawn a `std` program on an `fs_subtree_caretaker` endpoint rather than on the mount
root, and run the existing PAL tests against it. That exercises the rights discipline under a
narrowed grant, which nothing does now: every `std` test today is handed the root, so a path
operation that asks for more rights than it needs is indistinguishable from one that asks for
exactly what it needs. Both succeed.

## Why this matters

This is a test that would have caught a real defect, and the block says so: `readdir` nearly
shipped asking for `dir::ALL`. Under the root grant that code passes every test in the tree, and it
fails the first time a program is confined, which is the case the whole capability model exists to
serve. A test suite that cannot tell a confined program from an unconfined one is not testing the
property this system claims.

It also compounds. The PAL is being extended a piece at a time, and each new operation picks its
rights by whatever the author thought was needed. Under the current fixture nobody finds out. Under
a narrowed one, over-asking is a failing test at the moment it is written, which is rung two of
AGENTS.md's ladder rather than rung four, and rung four is where `readdir` was caught by somebody
happening to read the diff.

Milestone 122 already recorded that `OPENDIR` has no way to say "attenuate to whatever you have",
so a held directory asks for `dir::ALL` and probes one right at a time when a narrowed grant
refuses (`design/decisions/98-opendir-cannot-attenuate.md`). That probing path is currently
unexercised by any test, for exactly the same reason. This fixture is what runs it.

## Where it came from

Milestone 122's Follow-on: *"Spawn a `std` program on an `fs_subtree_caretaker` endpoint and run
the PAL against it, so the rights discipline is exercised under a narrowed grant. Every std test
today grants the mount root, which means a walk that over-asks for rights passes all of them. The
PAL has already come close once: `readdir` nearly shipped asking for `dir::ALL`."*
