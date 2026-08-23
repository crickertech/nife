# 49. Users, login, and attribution: what identity is for once it stops being authority

**Status: NOT-STARTED.**

**Gate: NONE.** Milestone 47 landed 2026-08-22. The attribution fork is decided (DECISIONS §109):
channel, not capability. What remains is a real component to build (login), not a further decision.

**A named prerequisite for milestone 152 (durable delegation)**, minted 2026-08-22: a scheduled job
registered by a specific user (milestone 129's #387) needs a durable principal to be supervised by,
and login-produces-capabilities is where that principal would first exist. 152 gates on this
milestone rather than guessing at identity itself, and inherits §109's channel-shaped answer to how
attribution composes with its own per-user sessions.

**In brief.** Unix's uid does four different jobs at once. Three of them are already answered here,
structurally and without anyone having declared it; the fourth has no mechanism whatsoever. This
milestone writes down the first three, builds a login service that produces capabilities instead of
changing an identity field, and then decides what to do about the fourth.

**Why it matters.** Users and groups **are** Unix's ambient authority mechanism. A process's authority
comes from who it belongs to rather than from what it was given, which makes every program a confused
deputy by default; `setuid` is that idea in its purest form, a program running with the union of its
owner's authority and its invoker's intent, and it has been a security disaster for fifty years.
Saying "we do not have uids" is not the interesting claim. The interesting claim is that *the work a
uid does still has to get done*, and here it gets done by four different mechanisms rather than one
overloaded number.

## The starting position: the tree is already identity-free, by accident of good design

Verified rather than assumed: **no `uid` or `gid` appears anywhere in our logic.** The vendored
RedoxFS on-disk `Node` carries the fields and `create_node` inherits them from the parent, because
that is the format; nothing ever reads them for an access decision. The `std` PAL lists permissions
under Unsupported, and `set_permissions` refuses rather than pretending. So this milestone documents
and completes a position the code already holds instead of migrating to a new one.

## What each of the uid's four jobs becomes

| Unix uses uid for | Here | Status |
|---|---|---|
| **Authorization** | Capabilities. There is no check to bypass because there is no check | Built |
| **Isolation between humans** | Milestone 47's per-shell root. Two people's shells hold different directory capabilities and neither can *name* the other's files | Built (never demonstrated multi-user) |
| **Resource accounting** | The untyped budget. A user's allowance is the region they were granted; `run --mem 16` splits from it | Built |
| **Attribution** (*who did this?*) | A channel (DECISIONS §109) | Decided, not yet built |

Isolation is the one worth dwelling on, because it is stronger than what a uid buys. Unix isolates by
*refusing* a request that names another user's file; the name is still sayable, the check is still
code that can be wrong, and root skips it. Here no capability reaching those files exists in that
shell, so the request cannot be phrased. A check that cannot be wrong because it is not performed.

## There is no root, and that is a statable property

Milestone 22 did something Unix structurally cannot: `root_supervisor` **gives its authority away**, deleting
its untyped once the sub-servers are running. The consequence is worth stating plainly next to the
benchmarks, because it is the kind of claim a demonstrator exists to make: **there is no point after
boot at which any principal can do everything**, not as a policy or a hardening measure but because no
capability naming everything survives. Unix's root is always one `sudo` away by construction.

## Groups are a delegation pattern, not a mechanism to build

Sharing is two parties holding the same capability, or capabilities derived from a common one;
nothing needs to be added to support it. Managed sharing (revocable, narrower for some holders,
auditable) is a **caretaker**, and `fs_file_caretaker` is already that shape: a component holding a
resource and serving several clients on its own terms. So this milestone builds no group mechanism
and instead documents the two patterns, because the alternative is someone inventing a group table
later.

## Login: authentication produces capabilities

Unix login authenticates and then mutates an identity field. Here it authenticates and then **hands
over a capability set**: a root directory, a budget, a terminal. That is a better failure mode as well
as a cleaner model, since a compromised login service leaks *what it can grant* rather than the
ability to become anyone. It is the powerbox pattern with the human at one end, and it needs a real
answer to a question we have never faced: **who gets which capabilities at startup**, which is
currently a build-time fact baked into `root_supervisor`.

## Attribution is the actual work, and the one place Unix does something we do not

A capability says what you may do. It says nothing about who did it. Unix gets audit almost free
because the uid is present at every syscall and doubles as the answer. Measured boot (§22) establishes
*what code* is running and capabilities establish *what it can reach*, but nothing records *who
asked*, and that gap is real rather than rhetorical.

**Decided (calef, 2026-08-22, DECISIONS §109): channel.** A server that wants to know who is asking
gives each principal its own endpoint, established once, and logs which one a request arrived on.
This tree has already faced this exact question three times, independently, under different names
(the compositor's shared-endpoint identity, the FS server's confinement, the fault endpoint's sender
trust) and answered it the same way every time without anyone naming it as one policy: give each
principal its own channel rather than badge a shared one (seL4's mechanism, never built here). It
also composes for free with milestone 152's durable per-user sessions: once a user's session exists,
every service reached through it already has a per-user channel, which is attribution at exactly the
granularity audit wants. §109 has the full reasoning, including where the channel model's own cost
eventually bites (roughly tens of concurrently-durable sessions against `MAX_REGIONS`, a number
comfortably beyond this tree's realistic target and cheap to raise if ever approached).

**Sequencing.** After 47 (isolation is 47's per-shell root, and login hands out exactly what 47
defines; 47 landed 2026-08-22). The documentation and the group/caretaker write-up are cheap; the
login service and the channel-shaped attribution logging are both ordinary builds now, no longer
contingent on a fork. **Effort: 2 lanes estimated**, noting estimates for unbuilt work are guesses on
a history-calibrated scale.
