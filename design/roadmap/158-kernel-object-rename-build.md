# 158. Build the eleven kernel object and identifier renames DECISIONS §113 decided

**Status: PARTIAL.** Minted 2026-08-23, from calef asking what the milestone was that renamed
the kernel components and finding the answer was "none": DECISIONS §113 (eleven kernel object and
identifier names move from contraction or borrowed jargon to the plain, standard term) was decided
2026-08-23 and never turned into tracked build work. Checked directly before minting this:
`kernel/src/cap.rs`'s `Object` enum still reads
`Aspace(u64)`, not `AddressSpace(u64)` -- the decision is real, the rename is not.

**Gate: NONE.** Nothing here needs deciding; §113 already settled every name. This is the mechanical
build DECISIONS §113 itself flagged as separate, future work: "each touches a real, measured
surface... and is left to whoever executes it, tracked as its own piece of work per name."

## What was built

`Endpoint`/`EpId`/`EpFail` -> `Rendezvous`/`RendezvousId`/`RendezvousFailure`, the biggest,
most-cited name, done to completion per this block's own instruction to do that one first and
measure before committing to the rest. **The real surface was much larger than this block's own
table**, once re-measured fresh rather than trusted from 2026-08-23's count: not just the 82
occurrences of the type name across 22 `.rs` files, but a whole ABI module
(`abi::endpoint` -> `abi::rendezvous`, the `SEND`/`RECV`/`SEND_CAP`/`RECV_CAP`/`CALL`/`REAP`/`SURVEY`
method-number namespace every user program and the kernel's `syscall.rs` dispatch on), an
`objtype::ENDPOINT` -> `objtype::RENDEZVOUS` object-kind discriminant, `MAX_ENDPOINTS` ->
`MAX_RENDEZVOUS`, and a family of lowercase function/field names spelling the same concept
(`create_endpoint` -> `create_rendezvous`, `endpoint_cap` -> `rendezvous_cap`, `endpoint_of` ->
`rendezvous_of`, the `endpoints` field on `IpcTables` -> `rendezvous_table`, and several test-only
helpers and test names). `EpId` alone was 216 occurrences across 43 files once the type alias's own
call sites were counted, well past the milestone doc's "follows Endpoint's rename" note.

Scoped deliberately to the identifier surface (types, functions, constants, module paths, struct
fields, test names, backtick-quoted doc-comment citations of those identifiers), the same rung
milestone 98's `IpcTables` rename drew: informal lowercase prose that uses "endpoint" as a
descriptive English word without naming a specific identifier (the bulk of it in `kernel/src/user.rs`
and the consumer crates/programs that merely narrate "an endpoint capability") was left alone in
files outside the object's own implementation, matching 98's own recorded scope call rather than
sweeping every prose sentence tree-wide. `crates/glob`'s unrelated "endpoint" (a character-range
boundary) was checked and correctly left untouched -- a real in-tree homonym, not a hit.

Verified: every host-testable crate builds and its full test suite (including doctests) passes; the
kernel builds clean for both `aarch64-unknown-none-softfloat` and `riscv64imac-unknown-none-elf`;
`user`, `fs_server` and `xtask` build clean; the two Kani-proof-bearing crates that reference the
renamed type (`crates/ipc`, 6 harnesses, and `crates/capability`, 12 harnesses) both verify
successfully post-rename, satisfying milestone 69's proof obligation. `notes/ipc-naming.md` updated
to describe `Rendezvous` as the current name, with a provenance line for a reader who remembers
`Endpoint`, and its "Family resemblance" section left untouched because it never named the old
identifier (it compares the underlying *model* to Mach's port and QNX's channel, not the type name).

`Tcb`/`Tid` -> `ThreadControlBlock`/`ThreadId`, with companions `TcbPtr` -> `ThreadControlBlockPointer`
and `TidSet` -> `ThreadIdSet`, done to completion in a separate lane (milestone/158-tcb-rename). The
strict PascalCase count landed close to this block's own stale table for once (`Tcb` was 12 files/33
occurrences against an estimated 12/28; `Tid` was 13 files/82 against an estimated 12/76), but that
comparison undersells the real surface the same way it did for `Endpoint`: neither of those counts
saw the lowercase identifier family at all. Real surface, re-measured: an ABI module
(`abi::tcb` -> `abi::thread_control_block`, the `CONFIGURE`/`CAP_INSERT`/`START` method-number
namespace every spawn path in the kernel and every child-building user program dispatches through),
an `objtype::TCB` -> `objtype::THREAD_CONTROL_BLOCK` object-kind discriminant, and a family of
lowercase function and field names spelling the same concept (`tcb_cap` -> `thread_control_block_cap`,
`create_tcb` -> `create_thread_control_block`, `configure_tcb` -> `configure_thread_control_block`,
`tcb_insert_cap` -> `thread_control_block_insert_cap`, `start_tcb` -> `start_thread_control_block`,
`tcb_ptr` -> `thread_control_block_ptr`, `tcb_configure`/`tcb_cap_insert` in `syscall.rs`'s dispatch,
`tcb_start` in `supervision_proto` and `system_initializer`, and the `Thread` struct's own
`tcb_kmem`/`tcb_region` fields), plus the `Tid` half's own compound names (`current_tid` ->
`current_thread_id`, `set_current_tid` -> `set_current_thread_id`, `write_tid` -> `write_thread_id`
in `crates/ps`, and five test names built the same way). 47 files changed, 385 insertions, 376
deletions.

Left alone, matching `Endpoint`'s own scope call: bare short local variable and parameter names
that are exactly `tid` or `tcb` with no further word attached (the same restraint the `Endpoint`
rename used, keeping `ep` unchanged throughout `kernel/src/sched.rs`); informal lowercase "tcb"/"TCB" prose describing
the concept in English rather than naming the identifier (`user/src/hello.rs`'s "(endpoint | aspace
| tcb)" list stayed lowercase, matching how "endpoint" stayed lowercase there after that rename);
and one historical citation in `crates/abi/src/lib.rs`'s own crate-naming rationale, which names
`Tcb`/`Aspace`/`Untyped` as the abbreviations a naming review "sank" -- renaming that citation to
`ThreadControlBlock` would have made the sentence describe the winning name as the one that lost,
so it was reverted to the old name it is actually citing. `notes/tcb.md` (the object's own note)
got a provenance line the same way `notes/ipc-naming.md` did; its "acronym collision" section (TCB
as Thread Control Block vs. Trusted Computing Base) is unaffected either way, since both senses are
already informal usage. Verified the same four ways as `Endpoint`: full host test suite green
(including doctests), both kernel targets build clean, `user`/`fs_server`/`xtask` build clean, and
`crates/ipc` (6 harnesses) plus `crates/capability` (12 harnesses) both re-verify under Kani
post-rename.

## What is still open

Three names remain, re-measured fresh rather than carried over from this block's stale table:

| Was | Becomes | Fresh count (`.rs` files, occurrences) |
|---|---|---|
| `Untyped` | `MemoryRegion` | 25 files, 60 occurrences (capitalized token only; likely larger once `crates/regions`' own lowercase vocabulary and any `untyped::`-module ABI surface are counted, the same pattern that grew `Endpoint`'s and `Tcb`'s real scope past their measured tables) |
| `Aspace` | `AddressSpace` | 9 files, 17 occurrences, plus its companion `FreeVas` -> `FreeAddressSpace` (`kernel/src/thread.rs`'s `FREE_STACK_VAS` / `struct FreeVas`, unmeasured separately) |
| `Frame` | `PageFrame` | 46 files, 157 occurrences (capitalized token only; `Frame` is also the compositor's own word for a screen update, so this rename needs the same care `Endpoint` needed distinguishing `crates/glob`'s range endpoints -- check every file before renaming, not just the capitalized-token list) |

Given `Endpoint` and `Tcb` both turned out to touch an ABI module, an object-kind constant and a
family of lowercase identifiers well beyond their measured tables, the same should be expected of
these three, especially `Untyped` (its own `crates/regions` already independently converged on
"region" vocabulary per DECISIONS §113's own finding). Each is its own lane per this block's own
sequencing instruction: "an inconsistent tree where `Aspace` is renamed and `Endpoint` is not is a
worse intermediate state than finishing one name completely and stopping" applies symmetrically --
finish `Untyped`, `Aspace`, or `Frame` completely in its own lane rather than partially touching
several.

## The seven renames, and what each one actually touches

| Was | Becomes | Measured surface |
|---|---|---|
| `Aspace` | `AddressSpace` | -- |
| `Untyped` | `MemoryRegion` | -- |
| `Endpoint` | `Rendezvous` | 82 occurrences across 22 `.rs` files (measured fresh; 216 for `EpId` across 43 files) |
| `Frame` | `PageFrame` | -- |
| `Tcb` | `ThreadControlBlock` | 33 occurrences across 12 `.rs` files (measured fresh; the lowercase identifier family below is well past this) |
| `EpId` | `RendezvousId` | 216 occurrences across 43 files, once the type alias's own call sites were counted |
| `Tid` | `ThreadId` | 82 occurrences across 13 `.rs` files (measured fresh) |

Plus the four companions §113 also decided in the same entry: `TcbPtr` -> `ThreadControlBlockPointer`
(11 occurrences, `kernel/src/sched.rs` only), `TidSet` -> `ThreadIdSet` (6 occurrences,
`kernel/src/user/survey_tests.rs` only), `EpFail` -> `RendezvousFailure`, `FreeVas` ->
`FreeAddressSpace` (still unmeasured; open with `Aspace`).

**Re-measure before starting, not from this table.** §113's own count is from 2026-08-23 and this
tree changes fast; a lane that trusts a stale number here repeats §76's own recorded mistake.

## What has to be true before this is safe to do in one pass

**Real collisions checked and named by §113 itself, still true and worth restating so a lane does
not have to re-derive them:**

- `Frame` collides with the compositor's own use of "frame" for a rendered screen update
  (`crates/compositor/src/lib.rs`). Renaming the kernel object to `PageFrame` is what resolves this;
  do not rename compositor's own "frame" language to avoid the opposite collision.
- `Untyped`/`Region` collides with the compositor's own use of "region" for a damaged screen
  rectangle, the same file. `MemoryRegion` is deliberately not bare `Region` for this reason.
- `Endpoint` has real prior art (seL4's own term) that some doc comments and notes may still cite
  approvingly; a rename should not silently rewrite a historical citation that correctly described
  what the tree used to be called. Read `notes/ipc-naming.md` before touching it -- §113 cites it
  directly and it may need updating rather than blind search-and-replace.

## Scope and sequencing

**Pure rename, no behaviour change**, the same discipline milestone 98's `IpcTables` rename used
(itself one of the smaller names in this same family -- read that milestone's own completed work,
once merged, as the template for how to gate and verify a rename of this shape here). Milestone
69's proof obligation applies to any renamed type a Kani harness references by name.

**Do the biggest, most-cited name first and measure the real diff before committing to doing all
seven in one lane.** `Endpoint`/`Rendezvous` is both the largest measured surface and the one most
likely to touch generated or macro-derived code (IPC message types, proof harnesses) in ways a
smaller rename like `Tcb`/`ThreadControlBlock` will not. If the full set turns out too large for one
safe, reviewable commit, split by name rather than doing all seven partially -- an inconsistent tree
where `Aspace` is renamed and `Endpoint` is not is a worse intermediate state than finishing one name
completely and stopping.

**Rename `EpId`/`TcbPtr`/`TidSet`/`EpFail`/`FreeVas` together with the type they name**, not as a
separate pass -- §113 decided them as companions specifically so a type and its own identifier never
disagree mid-rename.

## What this does not decide

Whether `Endpoint`'s rename can land in the same commit as `EpId`'s, or must be sequenced, is an
implementation judgment for whoever builds this, guided by how large the diff actually turns out to
be once measured -- not decided here.

## What it unblocks

Nothing else is gated on this; it closes the gap between a decision calef made and the code
actually reflecting it, which is its own reason to exist per DECISIONS §113's whole argument: a
name only works if a reader meets it, and today's reader still meets `Aspace`.
