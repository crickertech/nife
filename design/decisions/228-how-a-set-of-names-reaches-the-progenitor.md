---
status: PROPOSED
raised: 2026-09-26
---

# 228. How a set of matched names reaches the progenitor

Raised 2026-09-26 by the maintainer, from milestone 47 (navigation and naming)'s lane
`milestone/47-navigation`, which wrote the options up in
[notes/a-set-grant-at-the-prompt.md](../../notes/a-set-grant-at-the-prompt.md). The options, the
costs and the lane's reasoning are there and are not copied here. *(Section number provisional until
the merge queue lands it.)*

## What is being decided

How the shell asks the progenitor to build a set caretaker, so that a pattern matching two or more
names can be granted at a real prompt. It is a change to `spawnproto`, which the shell and the
progenitor both read, so this section gives options only.

## The premise, checked 2026-09-26

It is wider than the record said. The record said `xargs <program>` stops after its first batch.
In fact any pattern that matches more than one name is refused before anything spawns, `rm *.txt`
over two files included: `components/src/swish.rs` answers "a set of names is delivered by a
nameset caretaker, and the progenitor builds the subtree one". `DIR_BIT` in
`crates/grant_plan/src/spawnproto.rs` carries two words of directory and name, and no bit announces
a set. The caretaker itself exists and is proven (`components/src/fs_nameset_caretaker.rs`), but only
a kernel test wires it. It needs up to 137 bytes of encoded set (`filesystem_protocol::nameset::BYTES`)
in a read-only frame.

## The options

The note's table has the detail. In one line each:

1. Leave it. A pattern designates one name or is refused, and `xargs <program>` never runs a batch.
2. The set as data words (provisional `SET_BIT`). Up to six more `SEND`s of three words carry the
   encoded set; the progenitor checks the encoding, writes it into a frame it retypes, and maps it
   read-only for the caretaker. One transient capability slot.
3. The set as a frame the shell owns, §219 (how the shell names an installed program to the
   spawner) option D's shape. The shell keeps write access, so the progenitor has to copy before
   mapping, which makes it option 2 with a page in the middle.
4. Batches of one, no wire change: `xargs` plans one name per spawn through the subtree caretaker
   that already works. It does nothing for a plain `rm *.txt`, and the note says plainly that it is
   recommended only as effort.

The lane recommends option 2, because the carrier is data the progenitor checks rather than a page
someone else can still write. That is recorded as the lane's view; this section does not push it,
because the fork is a wire format.

## The seven questions

1. What else was considered. All four are in the note, each with the place it fails.
2. What this tree does in the analogous case. `DIR_BIT` announces extra `SEND`s carrying data, and
   milestone 154 (a process that holds two directory capabilities) already grew `spawnproto` the
   same way with `DIR2_BIT`. Option 2 is the third use of that shape, not a new mechanism.
3. Prior art, from memory and not re-read: Unix `execve` copies the expanded argument strings into
   the new process, so the caller cannot change them afterwards. That is option 2's property, and
   option 3's race is the one Unix avoids by copying.
4. Is the premise true. Yes, and wider, as above.
5. Cost. Three costs are unmeasured, and the note lists them: whether the directory-granted spawn
   path's capability-table peak stays under the measured 23 of 24
   (`kernel::cap::CAPABILITY_TABLE_PEAK_MEASURED`), the progenitor's pages for carrying the set
   caretaker's ELF, and the frame build itself. `script/swish-check` fails loudly on the first.
6. Reversibility. Irreversible in AGENTS.md's sense: two programs agree on the bit. Who has acted on
   it: nobody yet, because nothing has been built.
7. Equal cost. Option 4 is the only one chosen for effort, and it says so. Between 2 and 3 at equal
   cost, 2 still wins, because 3 must copy anyway.

## What it depends on

A prerequisite with no milestone until the lane filed it:
[a frame per filesystem client channel](../roadmap/proposals/a-frame-per-filesystem-client-channel.md).
The file service shares one read-write staging frame with every client. A set caretaker built at
the prompt, beside a `>` caretaker in the same pipeline, makes the check-then-use window that
`notes/shared-page-audit.md` called unreachable a live one. Options 2 and 3 should wait for that
work or land with it. Option 4 does not need it.

## What is blocked until this is answered

Milestone 109 (`xargs`: batching a grant too large to hand over) past its first batch, and every
multi-name pattern at the prompt. It is the second of milestone 47's three open items.
