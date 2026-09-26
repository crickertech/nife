# 599. A frame per filesystem client channel

**Status: PARTIAL.** Minted 2026-09-26 by lane `milestone/fs-client-page`, promoted from the
proposal `a-frame-per-filesystem-client-channel`, which milestone 47 (navigation and naming)'s lane
wrote on `milestone/47-navigation` (#1343). The work was first named as case A of
`notes/shared-page-audit.md` by milestone 43 (a second security audit) and never got a number.
The witness was built 2026-09-26 by the same lane and reproduces the defect; the wiring waits on the
decision below. *(Number and title provisional: the integrator mints the number at merge, and the
title is a draft until an architect names it.)*

**Gate: DECISION.** Ruled 2026-09-26 (option A, badged endpoints); see the status paragraph. The proposal said NONE, "a wiring change inside the tree". Reading the server
says otherwise: the file server receives on one endpoint, learns nothing about its caller, and reads
every request from one window. A per-client frame it can read needs badged endpoints, a kernel
remap at the rendezvous, or a token convention among the page's writers. The first two change the
syscall surface and the third adds a cross-program protocol. The options are in
`notes/a-frame-per-filesystem-client-channel.md`, and that fork is calef's. No section is written
in `design/decisions/` yet; the integrator mints one from that note.

## The finding, checked 2026-09-26

The file service shares one read-write staging channel (`fs::TRANSFER_PAGES`, 16 pages, 64 KiB)
with every client a boot wires. In the kernel harness it is `FILE_SHARED`, memoised by
`fs_service::ensure`. On the interactive boot it is the progenitor's `BootEndowment::fs_page`,
which the shell maps at `SH_FS_VA` and every `build_caretaker` chain maps at `FS_CLIENT_PAGE_VA`,
caretaker and confined program both (`crates/system_initializer/src/lib.rs`). What keeps clients
apart is which of them is blocked, not what they can map.

It is a correctness bug as well as a confinement one. A client stages its bytes and then calls, so
two honest clients active at once overwrite each other; `notes/pipes/the-file-end.md` is why the
shell writes a `>` file itself rather than letting a second process do it. Today every path keeps
one FS client active at a time. The set grant at the prompt would not.

## What is left

- PROPOSED: how the server tells channels apart. Options A (badged endpoints), B (the kernel
  moves the server's window to the caller's frame) and C (private client frames plus a staging
  token, no kernel change), with costs, in `notes/a-frame-per-filesystem-client-channel.md`.
  calef's call; it blocks everything below except the witness.
- BUILT: the witness. Two live clients on one file service, one rewriting the other's name
  mid-request, on every architecture `script/test` boots
  (`kernel/src/user/fs_shared_page_tests.rs`, the two `fs_test_client` roles it drives, and
  `fs_service::start_shared_frame_witness`). It forces the interleaving with a handshake rather than
  a race, so it reproduces deterministically, and it asserts the substitution the shared frame
  permits. The day the wiring below lands, that assertion must invert to expect isolation, which is
  what makes this test the gate on the fix. The live defect it reproduces is recorded in
  `redoxfs_server`'s `BUGS`.
- The wiring, in the shape the ruling picks: a channel allocated where each chain is built
  (`fs_service`'s `spawn_fs_client`, `start_granted*`, `narrow_dir`, `start_std*`, the sinks; the
  progenitor's `build_caretaker` and the shell's grant) instead of one memoised per service.
  `wait_for_caretaker`'s ordering hazard goes with it, since a caretaker's staging page would be
  reachable only by its own chain.

## What it costs

From constants, not a boot: 64 KiB of contiguous memory per concurrent client channel, where the
whole boot shares one 64 KiB channel today. Carved from a directory job's region
(`DIR_JOB_REGION_PAGES`, 96 pages), that adds about 17% to each job. Per-request cost depends on the
option: nothing added under A, a window remap under B, two rendezvous and a copy under C.

## What it unblocks

Milestone 47 (navigation and naming)'s set grant at the prompt, which names this as its
prerequisite. A ruling on option A would also decide option 1 of the proposal
`every-client-of-a-network-stack-shares-its-socket-numbers`, which is the same question for sockets.

## Follow-on

- **Done.** The witness, `kernel/src/user/fs_shared_page_tests.rs`, with its two `fs_test_client`
  roles and `fs_service::start_shared_frame_witness`. It reproduces the defect on all three
  architectures.
- **Recorded.** The live defect it reproduces is in `redoxfs_server`'s module `BUGS`, beside the
  file channel it is a property of.
- **Decision.** calef ruled option A (badged endpoint capabilities) on 2026-09-26; a maintainer is
  recording it under `design/decisions/`. The gate above is answered.
- **Outstanding.** The badged-endpoint build: a `BADGE` method (provisional) to mint a badged
  endpoint, the badge as a fourth `RECV_CAP` return value, the file server's K windows, and the
  progenitor's pool of (badged endpoint, frame) pairs taken back at reap. Checked 2026-09-26: the
  witness is built and the decision is ruled, so this is the only piece left before BUILT.

## Index row

The file service shares one read-write staging channel with every client, so clients are kept
apart by scheduling rather than by mapping. A per-client frame needs the server to know its caller,
which is a kernel or protocol fork for calef.
