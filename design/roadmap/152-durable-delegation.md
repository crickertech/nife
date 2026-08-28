# 152. Durable delegation: authority that outlives the session that requested it

**Status: PARTIAL**, updated 2026-08-24. Minted 2026-08-22, from a milestone 129 discussion: calef
wants nife to support multiple users, and wants the jobs a user schedules to carry capabilities that
reflect that user's own authority. Working through what that requires surfaced a gap this tree has
not needed to close before, and #387 (milestone 129's `--mem` grant, held pending this) is where it
was found. The design below was worked out the same day, in conversation; three of its four design
pieces are now built and tested (the durable session itself, the on-disk schedule store, and
boot-time re-derivation; see "What was built" below, both entries), and moved from `NOT-STARTED`
because the milestone is no longer nothing but a design: what remains is wiring a real registrar
against the pieces already proven (#387), which is real, separate work rather than a detail of what
is already built.

**Gate: NONE.** Cleared 2026-08-27: milestone 49 (users, login, and attribution) reached BUILT, so
the real identity this milestone's design needed to attach to now exists. The design fork itself
(what durably represents a user) was already answered below; what remains is wiring a real
registrar against the pieces already proven (#387), which is real, separate work and not a further
decision of this milestone's own -- see "What was built" and the BUGS entry below for exactly what
that is. Not attempted by this update: milestone 49's own lane was scoped to milestone 49 alone.

## In brief

Option 3 of #387's runtime-registration question already gives "a scheduled job can never hold more
authority than its registrar held" for free: `timetable::Registry::register(doc, held)` already
takes an arbitrary `Held`, so a registrar handed a narrower bundle than the scheduler's own produces
narrower jobs, structurally. Making that registrar **a user's own session** instead of a fixed
system component is a small idea with one hard consequence: a live SMB session's authority exists
only as long as the session does (`notes/smb.md`: "a session is proven at setup and unprotected
afterwards," meaning proven state doesn't persist past the connection), but a scheduled job needs to
keep firing after the user who registered it disconnects.

**Concretely**: if Chris authenticates, registers `every 24h verify_backup`, and disconnects, that
job still needs to fire tomorrow, on the authority he held at registration. Whatever holds that
authority between registration and firing has to outlive the connection.

## The existing rule this collides with, and what it implies

**DECISIONS §92 already decided the general shape of "a component holding derived authority," and
its rule is the opposite of durable**: *"A caretaker holds authority derived from a grant made to
one client. When the grantee dies, the derived authority should die."* `fs_subtree_caretaker` is
supervised by, and dies with, the client it serves (§40's subtree-death rule). That is the right
rule for the case it was built for (a directory grant narrowed for one live command), and it is
exactly the wrong rule for a scheduled job, which is supposed to survive its registrar disconnecting.

**This does not mean §92 is wrong; it means a durable delegation needs a different supervisor than a
live connection.** §92's own logic says the authority should die when *its client* dies. The design
below answers what that client should be for a durable delegation, rather than inventing a new
supervision rule.

**§92's own BUGS section already named the adjacent gap**: *"This says nothing about a caretaker
with no client, because none exists yet."* This milestone is that case, with a real motivating case
instead of a hypothetical.

## The design, worked out 2026-08-22

Four pieces, discussed in order and each answered before moving to the next.

### The durable principal is the session itself, kept alive by its own live children

**No new kind of object.** The durable thing a scheduled job's authority is supervised by is the
user's own login session, and what keeps it alive past a disconnect is exactly the rule
`Untyped::DESTROY` already has (§16): *a parent with live children refuses to be destroyed, and
becomes destroyable again once they're gone.* A session that refuses to tear down while it still has
scheduled-job children is that rule, applied to a session instead of a region. Nothing here needed
before was a snapshot of the user's authority or a live re-check against an external identity
source: the job's capability is the *same* live one from the original login, continuing to exist
because nothing tore it down.

**This was checked against real prior art rather than assumed.** systemd's `loginctl enable-linger`
solves almost exactly this problem (keep a user's session manager running after logout so pending
timers keep firing), but it is a **static, explicit admin toggle**: an account either lingers or it
does not, independent of whether anything is actually pending. The design here is the **dynamic**
version instead: a session persists *because* it has scheduled work, and becomes destroyable once it
does not, which is more minimal (nothing outlives its own reason to exist) and maps onto §16's
existing mechanism directly rather than needing a new opt-in flag and a new place to store it.
[loginctl (systemd)](https://www.freedesktop.org/software/systemd/man/latest/loginctl.html).

### Reattachment on reconnect, via a scoped lookup, not enumeration

When a user reconnects and a durable session already exists for them (kept alive by pending jobs),
the new connection needs to find and attach to *that* session rather than minting a second one
alongside it, or the design degrades into either session proliferation or an orphaned session with
no way back to it.

**This cannot be built as "list all sessions and find the match."** This tree already refuses that
shape on principle (milestone 126: enumeration is itself authority). It has to be a targeted lookup:
given a *proven* identity, return the one record for it, never a list. The natural home is the
credential service (milestone 56's credentialer) or a login broker built beside it (milestone 49
already describes login as "authentication produces capabilities"): a small table, identity to
durable-session capability (or nothing yet), where a successful authentication returns the existing
entry or creates and records a fresh one. Additive to what 56 already does, not a new kind of ambient
power: still "prove who you are, get back exactly your own thing," never a directory of everyone's.

### Decided: disabling a user's login credentials kills their durable session

**calef, 2026-08-22: yes.** Revoking credentials cascades to killing the durable session, which
cascades to everything derived from it (§40's subtree-death rule), including every scheduled job the
session was supervising. One action, one consequence, using mechanism that already exists rather
than inventing a second revocation path that has to be kept in sync with the first. Recorded as
[DECISIONS §108](../decisions/108-credential-revocation-kills-durable-session.md).

### Boot-time bring-up is re-derivation, not restoration

**Capabilities do not survive a reboot; nothing in the kernel does.** So bring-up at boot cannot be
"reload the session's old authority from disk," because there is no such operation. It has to be
**re-derivation**: boot re-establishes each durable session's authority fresh, the same way a login
would, without a live person presenting credentials at that moment.

**This needs a durable, on-disk record that does not exist yet.** Milestone 129's own "Still to
build" list already names this gap without solving it: *"Calendar syntax, wall-clock entries,
persistence... none started."* The runtime-registered, per-user schedule needs its own durable store
(today's `timetable.conf` is the compile-time equivalent, baked into the image), written when a job
is registered during a live session, read back at boot.

**Trust at boot does not come from a fresh secret challenge, because nobody is presenting one.** It
comes from the durable store itself only ever having been written by an already-authenticated
action: the user proved who they were once, at registration time, and boot only has to trust that
the store is authentic and untampered, which is what measured boot (§22, already one of milestone
49's three answered pillars) and the credential store's own existing persistence (milestone 56's
sealed store) are for. Boot-time re-derivation is a privileged, boot-only operation, in the same
shape as `root_supervisor` handing out its authority once at boot and never again, not a standing
"impersonate any user" capability left lying around afterward.

**This belongs to 152, not to milestone 129's own "persistence" line.** 129's item is really about
the schedule's data format (calendar syntax, wall-clock semantics); this is about the durable
session's lifecycle at its most extreme boundary, the kernel itself restarting. 129's BUGS entry
points here for the mechanism.

## What this unblocks

#387's runtime-registration question (milestone 129) can be answered once 49 exists: the registrar
is a user's durable session, kept alive by §16's live-children rule, reattached on reconnect through
the credentialer's identity lookup, torn down (cascading to its scheduled jobs) when credentials are
revoked, and re-derived rather than restored at boot from a durable, measured-boot-trusted store.

## What was built (2026-08-24, `smb_server`'s session/connection split)

The first BUGS item below (`smb_server` had no session/connection separation to build this against)
is closed. `user/src/smb_server.rs` now splits what used to be one accept-serve-close loop into two
objects with two different lifetimes, exactly the shape that BUGS entry named:

- **The transient per-connection protocol handler is unchanged**: `serve_connection` still rebuilds
  `smb_proto::server::Connection` on every accepted socket and drops it when the function returns.
  Nothing about the wire protocol moved.
- **`DurableSession` is the new, second half**: a capability budget (`Untyped::SPLIT` off this
  process's own budget, the same operation `login.rs`'s `mint()` uses) built once in `_start`,
  **before** the accept loop, and never rebuilt or torn down by any connection `serve` accepts and
  closes afterward. What keeps it alive past a disconnect is DECISIONS §16's existing rule and
  nothing new: `Untyped::DESTROY` on it refuses while it has a live child and succeeds once it does
  not. `DurableSession::mint_pending_job` is the one primitive a future scheduled-job registrar
  (milestone 129/#387) would hold a job's authority against; this lane wires no real registrar.
- **Proven on every boot that wires the authenticated share**, on both aarch64 and riscv64, under
  the existing SMB gate (`kernel::user::tests::a_host_process_connects_to_the_guest_and_is_answered`
  and its riscv64 twin): `open_durable_session_or_die` opens a scratch session, mints a synthetic
  pending-job child (standing in for a real scheduled job, which this lane does not register), and
  asserts `try_close` refuses while that child lives and succeeds once it is destroyed, before
  opening the real, kept session `serve` holds across every connection. A second check after the
  gate's real SMB traffic (two full negotiate-through-read connections) confirms the kept session is
  still in good standing and closes cleanly. Any of these seven properties failing halts the boot
  with its own stage code (`0xE140`-`0xE146`) rather than reporting a plain success; see
  `smb_server`'s own module header ("The durable session") and BUGS entry for the full account, and
  the two kernel gate files for what each code means.

**What this does not build, on purpose.** `DurableSession` holds no per-login narrowing of the
directory capability (this adapter still authenticates exactly one configured account; see
`smb_server`'s existing BUGS), no scheduled job is ever really registered against it, and reconnect
does not reattach to an existing session (there is only ever one, built once at boot). Those are the
milestone's second and third design pieces plus the runtime-registrar wiring (#387), all still open;
see the two entries below for what has changed about them.

## What was built (2026-08-24, the schedule store and the boot-time re-deriver)

The second and third BUGS items below (the on-disk schedule store, and boot-time re-derivation's own
mechanism) are closed, once [DECISIONS §122](../decisions/122-durable-schedule-store-format.md) and
[§123](../decisions/123-boot-time-rederivation-privilege.md) were ratified (option 1 and option (a)
respectively) and this lane built what they recommended, plus the manifest question neither decision
fully specified ([DECISIONS §125](../decisions/125-durable-schedule-manifest.md), PROPOSED,
provisional number, this lane's own finding).

- **`crates/schedule_store`** (provisional name) holds the shared names two programs agree on: the
  schedule file's own filename inside an identity's subtree (§122), the manifest's filename and
  document format (§125, this lane's own answer to "which identities"), and the render/parse
  functions for the manifest. It depends on nothing and reuses `timetable::parse` for the schedule
  document itself unchanged, exactly §122's recommendation.
- **The write path**: `user/src/fs_test_client.rs`'s new `ROLE_SCHEDULE_SEED` (this lane's own
  demonstration writer, not a real registrar; #387 remains that) `MKDIR`s one identity's subtree,
  writes its `schedule` file through ordinary `filesystem_proto::fs::CREATE`/`WRITE`, and records
  that identity in the manifest at the store's own root. `ROLE_SCHEDULE_VERIFY` reads both back
  through a **fresh** descent, independent of the re-deriver's own read, and confirms the bytes match
  exactly (the `smb_seed`/`smb_verify` shape, one level over).
- **`user/src/session_reviver.rs`** (provisional name; §123 itself floated this placeholder) is the
  boot-only re-deriver: granted a construction budget and the store-read capability, checked against
  the boot's measurement table before either is handed over
  (`kernel/src/user/session_reviver_service.rs`, §123's second hardening refinement), it reads the
  manifest by name (never `READDIR`, milestone 126's rule honored throughout), reads and parses each
  named identity's `schedule` file with the real `timetable::parse`, mints and tears down a synthetic
  per-identity session in `smb_server.rs`'s own `DurableSession` shape (proving a boot-derived
  session has the identical §16 lifecycle a live login's already does), then `cap_delete`s its own
  store-read capability and construction budget and proves both gone by attempting the now-forbidden
  operations and asserting they fail, `root_supervisor`'s own idiom.
- **This lane picked a new, dedicated process over a phase of `system_initializer`**, the smaller
  fork §123 left open: see `session_reviver.rs`'s own module doc for the reasoning (a new binary
  touches nothing else in the tree; growing `system_initializer::boot`, already the kernel's largest
  function, with a second privileged phase is real surgery on a component every boot depends on).
- **Proven on every boot with a RedoxFS disk attached**, on both aarch64 and riscv64 (no
  network/virtio dependency, so no ISA-specific wiring was needed):
  `kernel::user::session_reviver_tests::the_schedule_store_write_path_and_the_boot_time_re_deriver_agree`
  checks three properties against the re-deriver's own report (success rather than a stage-coded
  failure, the manifest's one identity actually re-derived, and the deletion proof holding), and
  `a_fresh_reader_confirms_the_store_holds_exactly_what_the_seed_wrote` is the independent witness
  that the store itself, not merely the re-deriver's reading of it, holds the right bytes.

**What this does not build, on purpose.** No real scheduled-job registrar against `DurableSession`
(#387/milestone 129's own question, explicitly out of this lane's scope: the write path above is a
kernel-test fixture, not a live session's real registration flow). No per-identity narrowing of the
re-deriver's own `FS_EP` (it holds one unnarrowed capability for its whole pass, the same bound
`login.rs`/`identity_provisioner.rs` already carry for the identical grant; see `session_reviver.rs`'s
own BUGS). No liveness watchdog for a re-deriver that hangs before its deletion pass runs (§123's
hardening addendum names this gap and explicitly declines to design it; this lane does not either).
Wiring `session_reviver` into a real boot (`crates/system_initializer::boot` or an interactive
`cargo xtask shell-check`) rather than the kernel test harness that spawns it here remains open, the
same "not wired into the interactive boot" bound several of this milestone's own dependencies already
carry.

## BUGS

- ~~`smb_server` has no session/connection separation to build this against.~~ **Built 2026-08-24**;
  see "What was built" above and `user/src/smb_server.rs`'s own module header and BUGS entry for the
  live version of this record (a limitation belongs where a reader meets the feature, not only here).
- ~~The on-disk, per-user schedule store has no format, no write path, and no read-at-boot path.~~
  **Built 2026-08-24**; see "What was built" above, [DECISIONS §122](../decisions/122-durable-schedule-store-format.md)
  (ratified) and [§125](../decisions/125-durable-schedule-manifest.md) (this lane's own manifest
  question, PROPOSED), and `crates/schedule_store`'s own module doc for the live version of this
  record.
- ~~Boot-time re-derivation's own mechanism was asserted, not designed.~~ **Built 2026-08-24**; see
  "What was built" above, [DECISIONS §123](../decisions/123-boot-time-rederivation-privilege.md)
  (ratified, option (a), plus its four hardening refinements), and `user/src/session_reviver.rs`'s
  own module doc and BUGS entry for the live version of this record, including the two hardening
  refinements this lane did not build (per-identity `FS_EP` narrowing, the liveness watchdog).
- **#387 (milestone 129's `--mem` grant) is still not answerable.** This lane built what #387 was
  waiting on (a durable session that outlives a connection, `smb_server.rs`'s `DurableSession`; a
  durable, on-disk schedule store; and boot-time re-derivation of it), not #387 itself: no scheduled
  job is ever registered against a real `DurableSession` anywhere in this tree, and
  `ROLE_SCHEDULE_SEED` is a kernel-test fixture standing in for that registration, not a live
  session's own act. Wiring a real registrar (`timetable::Registry::register` called against a
  `DurableSession`'s own `mint_pending_job`, on a live session's authority, writing through
  `crates/schedule_store`'s format on registration) is the milestone's remaining piece.
