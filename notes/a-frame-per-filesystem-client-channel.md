# A frame per filesystem client channel: the fork

Written 2026-09-26 (UTC) by lane `milestone/fs-client-page`, for milestone 599 (a frame per
filesystem client channel), whose number is provisional. The block is
[design/roadmap/599-a-frame-per-filesystem-client-channel.md](../design/roadmap/599-a-frame-per-filesystem-client-channel.md).
The file name is a lane's coinage and provisional.

What is being decided: how the file server learns which client's frame a request's bytes are
in. Every way to give each client its own frame runs into that question, and every answer the tree
could build either changes the kernel's endpoint semantics or adds a protocol. That makes it
calef's, so the lane stopped here instead of building one.

What is blocked until it is answered: all of milestone 599's wiring, and through it milestone
47 (navigation and naming)'s set grant at the prompt (`rm *.txt`), which names this as its
prerequisite in `notes/a-set-grant-at-the-prompt.md`. Nothing else is blocked. The witness test can
be built before the ruling, because its attacker is the same under every option.

## Why the proposal's one-line fix does not work on its own

The proposal (case A of [shared-page-audit.md](shared-page-audit.md)) says to allocate the frame
"in `spawn_fs_client` / `start_granted*` / `narrow_dir` rather than in `ensure`". That fixes the
client side only. The server is the other holder of every channel, and it:

- receives on one endpoint (`redoxfs_server.rs`, `recv_cap(FILE)`), because this kernel has no
  receive over a set of endpoints (DECISIONS §27, "Why a process and not a check inside the FS
  server");
- learns nothing about its caller. `RECV_CAP` hands it two words and a one-shot Reply capability,
  and the serve loop's own comment says so: "endpoint-only naming means we never learn who they
  are, only how to answer";
- maps one window at `FILE_PAGE`, `fs::TRANSFER_PAGES` (16) pages wide, and reads every name and
  every `WRITE` payload from its start.

So if two clients each had a private frame, the server would still read one window, and there is
nothing in a request that could tell it which frame to look in. A per-client frame the server can
read needs one of the mechanisms below.

## The premise, checked

It is true, and it is a correctness bug as well as a confinement one. Honest clients race too:
a client stages its name or data in the page and *then* calls, so its use straddles the call. While
the server is blocked on the block server serving someone else, a second client's staging lands on
top. The tree already designs around this. `notes/pipes/the-file-end.md` is why the shell
backs a `>` redirection itself rather than handing it to a process: "two client processes using it
at once race", and `ls > out.txt` would corrupt both. Today's safety is that the shell keeps one FS
client active at a time. A set grant at the prompt would run a checking caretaker beside the shell's
own `>` writer in one pipeline, which is two active clients by construction.

Not yet verified by a run: that the substitution lands on a real boot with two live clients. That is
the witness test's job, and it is the next piece of this milestone either way.

## What the tree does in the analogous cases

- The compositor gives each client its own control page and surface, and gets away with one
  doorbell because its messages are content-free. Every per-client fact is read from that client's
  own page, which the compositor maps separately, and it reads *all* of them per frame. That works
  for a small fixed set of clients known at spawn. It does not transfer: the file server would have
  to know which page to read, which is this question again.
- The network stack has the same shape of bug and an open proposal for it,
  [every-client-of-a-network-stack-shares-its-socket-numbers](../design/roadmap/proposals/every-client-of-a-network-stack-shares-its-socket-numbers.md).
  Its option 1 is badged endpoints. A ruling on option A below would decide that option there too.
- DECISIONS §27 records badged endpoints as the alternative to the caretaker and did not take
  it. §101 (notification objects) names badged capabilities as a later fork, and §148 refused the
  method name `BADGE` to keep it free for this.

## The options

All costs below are computed from constants in the tree, not measured on a boot, except where a
benchmark note is cited. The per-channel figure is the same under A, C and D: one channel is
`fs::TRANSFER_PAGES` = 16 pages, 64 KiB of physically contiguous memory per concurrent client,
where today the whole boot shares one 64 KiB channel. A directory job's region is
`DIR_JOB_REGION_PAGES` = 96 pages (384 KiB), so carving the channel from the job's own region adds
about 17% to it.

### A. Badged endpoint capabilities (seL4's answer)

A derived endpoint capability carries a badge word the kernel delivers to the receiver on
`RECV_CAP`. The server maps K windows and reads request `b` from `FILE_PAGE + b * TRANSFER_MAX`. The
progenitor holds a pool of K (badged endpoint, frame) pairs and hands one to each caretaker chain,
taking it back at reap.

- `filesystem_protocol`'s message layout and opcodes do not change.
- The syscall surface does. A method to mint a badged endpoint, and a fourth return value from
  `RECV_CAP` (it already returns the first word, the Reply slot and the second word).
- Per request: nothing added. No extra rendezvous, no copy.
- Server address space: K windows of 64 KiB. K = 8 is 512 KiB, inside the 8 MiB above `BLK_PAGE` that
  `redoxfs_server.rs` says nothing else in the server maps.
- K is a fixed bound on concurrent FS clients per boot, and the pool is new state in the progenitor.
  A server that could accept windows at run time instead needs an attach verb, which is a wire
  change on top.
- Reuse: the network stack's option 1, and any later server with the same shape.

### B. The kernel moves the server's window to the caller's frame at the rendezvous

An endpoint capability is bound, when it is minted, to the caller's channel frame. At `CALL`, the
kernel remaps the receiver's `FILE_PAGE` window to that frame before the server runs. This is
classic L4's receive window for map items (recalled from memory of the L4 X.2 and Fiasco
specifications, not read for this note), which seL4 did not keep.

- `filesystem_protocol` unchanged, the server's code unchanged, one `FILE_PAGE` as today.
- A new kernel semantics on endpoints, outside the model §10 established: a capability that
  changes the receiver's address space as a side effect of IPC.
- Per request: 16 page-table entries rewritten and a TLB invalidation for the window, on the IPC
  fastpath. Not measured. The icount tripwire would price it.

### C. No kernel change: private client frames plus a staging token

Each confined program gets a private frame shared only with its caretaker. The caretaker copies a
request into the server's page and the reply out of it. It holds a token (a rendezvous used as a
lock) from before staging until after the copy-out, so no two writers of the server page overlap.

- `filesystem_protocol` unchanged, the kernel unchanged. The token convention is new, and under
  rule 7 it is a crate, since every writer of the server page must agree on it.
- Per request: two extra rendezvous (take and return the token), about 0.7 to 1.4 µs at the 337 to
  705 ns round trips `notes/benchmarks/cross-os-primitives.md` and
  `notes/benchmarks/calibration-against-sel4.md` record, plus a copy of up to 64 KiB each way.
- It is cooperative. It holds only while every process that maps the server page takes the
  token. Today that set includes the shell and every kernel-harness client (`spawn_fs_client`,
  `start_std`, the sinks), and each would need converting. A holder that skips the token reopens
  the bug with no error anywhere.
- Would we choose it if A and C cost the same to build? No. It has more moving parts, a copy on
  every byte and a rule nothing enforces. Its case is effort: no kernel change. That is its only
  advantage.

### Refused

- A channel index in the request word. A wire change, and it proves nothing: a client can name
  another client's index. Something must bind the index to the caller, and that is option A.
- A file server per client. One block device and one RedoxFS image; two servers mounting it
  would corrupt it.
- The server copies the name out first thing. It narrows the window to the time before the
  server's receive and does not close it. The audit's finding 2 already took this kind of narrowing
  for the nameset caretaker and recorded the residue.
- Notification objects (§101, milestone 151). They multiplex one endpoint with async signals.
  They do not let a server receive on several endpoints or learn who called, so on their own they
  answer nothing here.
- Keep serialising by construction (one FS client active at a time, as the shell does). That is
  the status quo, and the set grant is exactly the case it cannot cover.

## Reversibility, and who has acted on it

A and B change the syscall surface, which every future program is written against: the expensive
side of the line. C is reversible in code but adds a cross-program convention that is not. Nobody
has built against any of them. The network-stack proposal is waiting on the same question in A's
form. Per the rule for irreversible forks, this note gives options rather than a recommendation.
