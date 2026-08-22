# 152. Durable delegation: authority that outlives the session that requested it

**Status: NOT-STARTED.** Minted 2026-08-22, from a milestone 129 discussion: calef wants nife to
support multiple users, and wants the jobs a user schedules to carry capabilities that reflect that
user's own authority. Working through what that requires surfaced a gap this tree has not needed to
close before, and #387 (milestone 129's `--mem` grant, held pending this) is where it was found. The
design below was worked out the same day, in conversation; nothing here is built.

**Gate: MILESTONE 49.** The design fork (what durably represents a user) is answered below; what
remains is that the answer needs a real identity to attach to, and milestone 49 (users, login, and
attribution) is where identity first exists. Sequenced after 49, not blocked on a further decision
of this milestone's own.

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

## BUGS

- **`smb_server` has no session/connection separation to build this against.** Checked directly
  (`user/src/smb_server.rs`): today's server is a single accept-serve-close loop (`serve()`),
  one connection at a time, with no object that could persist past a disconnect even in principle.
  Building this means splitting what is currently one thing into two: a transient per-connection
  protocol handler (dies with the socket, as today) and a durable per-login object that outlives it.
  That is real structural work, not a lifecycle-rule change, and it has not been scoped yet.
- **The on-disk, per-user schedule store (point 4) has no format, no write path, and no read-at-boot
  path.** Named as a gap here; none of the three is designed yet.
- **Boot-time re-derivation's own mechanism is asserted, not designed.** "A privileged, boot-only
  operation" is the right shape by analogy to `root_supervisor`, but what object grants that
  privilege, and how it is scoped so it cannot be invoked again after boot, is not worked out.
