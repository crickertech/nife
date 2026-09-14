# One definition of the numbers a kernel test and its program agree on

**Status: PROPOSED 2026-09-14.** Written by milestone 293's lane, which reduced one instance of
this and then measured the rest of it.

**Gate: DECISION.** The sweep itself is mechanical and a lane could start it today; what it cannot
settle is the crate names, and there is no useful half of this work that does not create one. See
"What a lane cannot decide" below, which prices the two shapes so the question arrives answered.

**In brief.** AGENTS.md rule 7 says anything two binaries must agree on is a crate, never a copy.
**Seventy-six constants across nineteen file pairs are copies**, each held in line by a `// must
match` comment: role numbers, report tags, and report flag bits shared between a kernel test's
wiring (`kernel/src/user/*_service.rs`, `*_tests.rs`) and the program it spawns
(`fixtures/src/*.rs`, `components/src/*.rs`). `login_service.rs` and `login_test_client.rs` alone
share fifteen.

## The measurement

Constants named `ROLE_*`, `RPT_*`, `F_*` or `TERM_MAGIC`, declared **twice** under the same name,
once on each side of the spawn:

| Kernel side | Program side | Shared names |
|---|---|---|
| `kernel/src/user/login_service.rs` | `fixtures/src/login_test_client.rs` | 15 |
| `kernel/src/user/disk_tests.rs` | `components/src/disk_partitioner.rs`, `disk_surveyor.rs`, `redoxfs_server/src/bin/mkfs.rs` | 7, 7, 2 |
| `kernel/src/user/compositor_service.rs` | `fixtures/src/window.rs` | 7 |
| `kernel/src/user/credential_service.rs` | `fixtures/src/credentialer_test_client.rs` | 5 |
| `kernel/src/soak.rs` | `fixtures/src/soaker.rs` | 4 |
| `kernel/src/user/identity_provisioner_service.rs` | `components/src/identity_provisioner.rs` | 4 |
| `kernel/src/user/ntp_service.rs`, `pipeline_service.rs`, `disk_service.rs` | `components/src/ntp.rs`, `swish.rs`, `disk_surveyor.rs` | 3 each |
| six more pairs | | 2 each |

Nineteen pairs, seventy-six names, zero shared definitions.

## Why a comment is not a mechanism

It is rung four of AGENTS.md's ladder for a fact that has a rung-one answer sitting next to it. The
failure it permits is silent and specific: a role number changed on one side spawns a *different*
role, and a flag bit changed on one side makes a test assert against a bit nothing sets. Both read
as a bug in the code under test rather than as a mismatch, which is the same misdiagnosis
`credential_proto::fixture`'s own module doc warns about for the SMB account.

**This is not hypothetical, and the counter-example is in the tree.** `crates/swap_proto` already
publishes `ROLE_DIRECT`, `ROLE_QUEUED` and `ROLE_HUNG` with real doc comments, and
`kernel/src/user/live_swap_tests.rs` **re-declares all three locally anyway** under a four-word
comment. So even where the crate exists, one side was written without it. That is the strongest
argument for a gate rather than a convention.

## Prior art inside this tree

The shape is built and works, twice. `crates/swap_proto` and `crates/job_mix` both publish the role
numbers their two binaries agree on, and `crates/schedule_store` publishes the file names
`session_reviver` and `fs_test_client` both depend on, for exactly this reason and with the reason
written down. Milestone 293 did the same for the credential values three files were copying
(`credential_proto::fixture::PEOPLE`). What is missing is not a design, it is the sweep.

## What a lane cannot decide

**The crate names.** The numbers a kernel test and its test client agree on are not part of any
shipped wire contract: putting `login_test_client`'s six behaviour numbers into `login_proto` would
widen the protocol a real login client is written against with something no real client needs. So
each pair either gets a small crate of its own or a shared one, and both are calef's call. That, and
not the work, is what this waits on.

**Two shapes worth pricing before asking**, because the question should arrive answered:

1. **One crate per pair** (`login_test_contract`, `compositor_test_contract`, ...), which follows
   `swap_proto`'s precedent, keeps each contract readable in one file, and costs nineteen names.
2. **One crate for all of them**, which costs one name and makes the crate a grab bag of unrelated
   numbers, which is the shape §46 and the generic-name rule both push against.

## Where it came from

Milestone 293's lane, which was briefed to "grep the values, not just the constant names" after
the sibling lane on `fixtures/src/sink.rs` (292, unmerged as this is written) found one fact
hand-copied in three files. 293 removed the credential half of
its own instance (three copies of three identity/secret pairs, one of which said in a comment that
it had *chosen* to match another) and left the behaviour-number half, because fixing one pair inside
a hotspot three other lanes were in would have been a partial fix in the worst possible place.
