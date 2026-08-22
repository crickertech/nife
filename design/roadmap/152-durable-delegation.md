# 152. Durable delegation: authority that outlives the session that requested it

**Status: NOT-STARTED.** Minted 2026-08-22, from a milestone 129 discussion: calef wants nife to
support multiple users, and wants the jobs a user schedules to carry capabilities that reflect that
user's own authority. Working through what that requires surfaced a gap this tree has not needed to
close before, and #387 (milestone 129's `--mem` grant, held pending this) is where it was found.

**Gate: DECISION, MILESTONE 49.** Sequenced after 49 (users, login, and attribution), because a
durable delegation has to be supervised by something durable, and the only durable principal this
fork can name is the identity 49 has not built yet. The fork below is design-shaped, not
implementation-shaped: what should own a durable delegation, structurally, before anything is built
against it.

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
job still needs to fire tomorrow, on authority derived from what Chris was allowed *at registration
time* (or re-checked against what he's allowed *now*, which is a different, harder answer). Whatever
holds that authority between registration and firing has to outlive the connection.

## The existing rule this collides with, and what it implies

**DECISIONS §92 already decided the general shape of "a component holding derived authority," and
its rule is the opposite of durable**: *"A caretaker holds authority derived from a grant made to
one client. When the grantee dies, the derived authority should die."* `fs_subtree_caretaker` is
supervised by, and dies with, the client it serves (§40's subtree-death rule). That is the right
rule for the case it was built for (a directory grant narrowed for one live command), and it is
exactly the wrong rule for a scheduled job, which is supposed to survive its registrar disconnecting.

**This does not mean §92 is wrong; it means a durable delegation needs a different supervisor than a
live connection.** §92's own logic says the authority should die when *its client* dies — so for a
scheduled job to survive a disconnect, its "client," in the supervision sense, cannot be the
transient per-connection session. It has to be something durable: an account object, a principal
that exists independent of any one login, that a session merely *authenticates against* rather than
*is*. That durable principal is exactly what milestone 49 does not yet build ("who gets which
capabilities at startup... is currently a build-time fact baked into `root_supervisor`"), which is
why this milestone gates on it rather than solving identity itself.

**§92's own BUGS section already named the adjacent gap**: *"This says nothing about a caretaker
with no client, because none exists yet."* A durable delegation is close to that case: a caretaker
(or caretaker-shaped component) whose supervising "client" is a durable account rather than a single
live connection is not quite "no client," but it is the same shape of question §92 left unanswered,
now with a real motivating case instead of a hypothetical.

## What has to be decided, once 49 exists

Not answered here, on purpose, per this project's own rule that a fork of this shape gets options
and costs rather than a picked winner (it reshapes how every future long-lived delegation is
supervised, which is the "who else has already acted on this" question the *move fast on what can
be undone* tenet asks):

- **What durable object represents "a user," independent of any one connection**, and who
  supervises it? (A process that outlives individual logins? A kernel object with its own
  lifetime, closer to how `Untyped` regions are owned rather than session-scoped?)
- **How does a delegation's authority get re-derived or copied at registration time**: a snapshot of
  what the user could grant at that moment (durable but potentially stale if their access later
  narrows), or a live re-check against the durable principal each time the job fires (correct but
  needs the durable principal to still exist and be askable)?
- **Revocation**: if a user's access is disabled, should their already-registered jobs stop firing
  automatically (cascading revocation through whatever durably holds their delegated authority), or
  keep running on the authority granted at registration time until explicitly swept? §16's
  region-ownership revocation model is the natural mechanism to extend *if* a durable delegation is
  built as a region reachable from the durable principal's own ownership, the same way `Untyped
  DESTROY` already cascades through everything retyped from a region.

## What this unblocks

#387's runtime-registration question (milestone 129) can be answered once this is: Option 3's
mechanism (a registrar can never grant more than its own `Held`) still holds regardless of how this
resolves, but *what the registrar is* (a fixed system component, or a durable per-user principal)
depends on this milestone's answer.

## BUGS

- **No prior art surveyed yet.** This doc names the in-tree collision (§92) and the in-tree
  prerequisite (49) but has not yet checked how systems with a real multi-user, long-lived-job
  story (systemd user units and `loginctl linger`, in particular, which exists to solve exactly
  "keep a user's stuff running after they log out") answer this, or at what cost. Worth doing before
  the fork above is decided rather than after.
