# 155. A provisioning tool: create an identity and its home subtree together

**Status: NOT-STARTED.** Minted 2026-08-23, surfaced by DECISIONS §117 rather than invented ahead
of a need: deciding that a principal's subtree is created at provisioning time, not auto-vivified
at first login, means a provisioning tool has to exist to do that creating. Checked before minting
this: nothing outside test harness code (`user/src/credentialer_test_client.rs`,
`kernel/src/user/credential_tests.rs`) calls the credential service's `PROVISION` endpoint today,
so this is a real gap and not a restatement of existing work.

**Gate: NONE.** Nothing here needs a decision beyond §117's, which is already made. What is missing
is the program itself.

## What this is, in brief

Unix's `useradd`, one level down: a tool that authenticates as (or is trusted by) an operator,
takes an identity and a secret, and does two things as one act rather than two: `PUT`s them into
the credential store over `cred_proto`'s `PROVISION` endpoint, and creates that identity's home
subtree (named by the identity string itself, DECISIONS §117) under whatever directory capability
holds the principal tree.

## What it needs, named rather than designed here

- **A capability contract.** The tool needs `WRITE`/`GRANT` on the credential store's provision
  endpoint and a directory capability wide enough to create a new subtree under. Both are
  administrative authority in a system that otherwise minimizes it, so who may run this tool (and
  how that is itself provisioned, since the tool needs the same capability discipline as everything
  else here) is this milestone's own first design question, not assumed by minting the number.
- **What happens if the identity already exists.** `cred_proto`'s own semantics for a duplicate
  `PUT` govern this; check them before assuming re-provisioning is either idempotent or refused.
- **Whether subtree creation and credential `PUT` are one transaction or two steps that can fail
  independently.** If they are two steps, a failure between them leaves an identity with no home or
  a subtree with no identity, and this milestone has to say which failure mode is acceptable and
  why, or how it's prevented.

## What it does not decide

Deprovisioning (removing an identity and reclaiming its subtree) is not this milestone's scope;
`user/src/login.rs`'s own BUGS already names reclamation as an open bound and this tool's removal
half, if it gets one, should be sequenced against that rather than invented independently here.

## Prior art

Unix `useradd`/`adduser` (the shape, not the mechanism: no uid, no `/etc/passwd` line, a
capability-shaped grant instead). See DECISIONS §117 for why the subtree name itself needs no
separate lookup.
