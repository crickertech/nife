# One shared frame per filesystem client channel, not one per service

**Status: PROPOSED 2026-09-26.** Raised by milestone 47 (navigation and naming)'s lane
`milestone/47-navigation`, while pricing how a matched pattern could reach the progenitor
(notes/a-set-grant-at-the-prompt.md). The work was first proposed as case A of
notes/shared-page-audit.md by milestone 43 (a second security audit), and no milestone was minted
for it. This file gives it a place a script can see. The stem is a lane's coinage and provisional.

**Gate: NONE.** It is a wiring change inside the tree, with no wire format and no new kernel
object.

## The finding, checked 2026-09-26

The file service shares one frame with every client a boot wires, read-write in each of them. On
the interactive boot that is the progenitor's `BootEndowment::fs_page`. The shell maps it at
`SH_FS_VA`, and every caretaker `build_caretaker` builds maps it at `FS_CLIENT_PAGE_VA`
(`crates/system_initializer/src/lib.rs`). What keeps two clients apart is which of them is blocked,
not what they can map.

The audit fixed the one place that check-then-use mattered, in `fs_nameset_caretaker`, and named
the residue. The caretaker checks a name, writes it back and forwards. Another writer of the frame
can still land between that write and the server's read. The audit called this unreachable because
the shell was the only holder.

## Why it matters now

Two things moved since the audit. The progenitor has built subtree caretakers for the prompt since
2026-08-17, so the frame already has several live holders on a real boot. And the proposed set
grant at the prompt would put the checking caretaker among them, beside a `>` file caretaker in the
same pipeline. That proposal names this work as its prerequisite.

## Exit criterion

Each client channel gets its own staging frame, allocated where the channel is built rather than
memoised per service. A witness test runs two live confined clients on one file service. One of
them rewrites the other's name mid-request. The test fails before the change and passes after it,
on every architecture `script/test` boots.

## Index row

The file service shares one read-write staging frame with every client, so clients are kept apart
by scheduling rather than by mapping. Proposed: a frame per client channel, with a two-client
witness.
