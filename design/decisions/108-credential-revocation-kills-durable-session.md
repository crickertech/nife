# 108. Disabling a user's login credentials kills their durable session

**Status: DECIDED.** calef, 2026-08-22, on milestone 152's durable-delegation design (worked out in
conversation, not yet built): *"disabling a user's login credentials should kill their durable
session. That seems like the right consequence."*

## The question

Milestone 152 gives a scheduled job's authority a durable supervisor: the user's own login session,
kept alive past a disconnect by the same rule `Untyped::DESTROY` already has (§16), a parent refuses
to be destroyed while it has live children, and becomes destroyable once it does not. That answers
what keeps a durable session alive. It does not answer what should tear one down: if a user's login
credentials are disabled, should their already-running durable session, and every job it supervises,
stop, or should already-registered work keep running on the authority it was granted at registration
time until something explicitly sweeps it?

Both are defensible. Revoking credentials could plausibly mean only "you may not authenticate
again," leaving existing delegated work alone; disabling an account is also plausibly meant to stop
everything acting on that person's behalf, immediately.

## The decision

**Disabling credentials kills the durable session.** One action, one consequence: revoking a user's
ability to log in also revokes everything currently running on authority derived from an earlier
login of theirs. This reuses §40's subtree-death rule rather than inventing a second revocation
path that would have to be kept in sync with the first: killing the durable session (a supervised
subtree) cascades to every scheduled job it was supervising, the same mechanism that already tears
down a component's whole subtree when its supervisor dies.

**Why this over the alternative**: a second, independent revocation path (credentials disabled, but
delegated authority left running until an admin separately notices and sweeps it) is exactly the
kind of two-places-recording-one-fact shape this project's own ladder (CLAUDE.md, "nobody remembers,
so build the mechanism that does not need them to") warns against. Coupling them means there is
nothing to forget.

## What this does not decide

**How credentials get disabled** (the credentialer's own revocation mechanism, whether that is
deleting a store record, marking it inactive, or something else) is milestone 56/49's territory, not
named here. This decision only fixes the *consequence*, once revocation happens by whatever
mechanism those milestones build.

## What it unblocks

Milestone 152's design no longer has an open revocation fork: killing a durable session is the one
and only way delegated authority (including every scheduled job it supervises) stops, and it is
triggered exactly by disabling the login credentials that authenticate it.
