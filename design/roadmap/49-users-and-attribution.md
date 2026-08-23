# 49. Users, login, and attribution: what identity is for once it stops being authority

**Status: PARTIAL.** A login service exists, proven end to end (see "What is built" below), and it
authenticates exactly one login path against the credential service milestone 56 already built. It
is not wired into the interactive boot, it attenuates every principal to the same subtree rather than
a per-identity one, and it hands back two of the three capabilities the milestone's own text names
(a directory and a budget; not a terminal). See BUGS for the full remainder and where each piece is
headed.

**Gate: NONE.** Milestone 47 landed 2026-08-22. The attribution fork is decided (DECISIONS §109):
channel, not capability. What remained was a real component to build (login); a first slice of it now
exists.

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
| **Attribution** (*who did this?*) | A channel (DECISIONS §109) | Decided and built: `login` mints a fresh channel per successful login and records who established it (see BUGS on the scope of what is proven) |

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
answer to a question we have never faced: **who gets which capabilities at startup**, which used to be
a build-time fact baked into `crates/system_initializer`'s boot wiring and is, for the one login path
this milestone builds, a fact decided at run time instead (see "What is built" below).

## What is built

**`login` (provisional name), `user/src/login.rs`, and its wire contract `crates/login_proto`
(provisional).** It holds the credential service's verify endpoint (milestone 56, unmodified: neither
`cred_proto` nor `credentialer.rs` changed), the file service's root directory capability, and a
construction budget. A client presents an identity and a secret over `login_proto`; on a match, `login`
builds a fresh `fs_subtree_caretaker` (the same construction `crates/system_initializer` performs for
a directory-granted spawn) and splits a fresh budget, then delegates both to the caller over
`abi::endpoint::SEND_CAP`. Two different successful logins are therefore two different endpoint
*objects*, not two views of one shared endpoint, which is the channel-shaped attribution DECISIONS
§109 decided on rather than the badge mechanism it refused. `login` also sends one attribution record
per successful login (identity, sequence number) on its own audit endpoint, which is the "establishes
a channel and can say who it belongs to" half of §109's property, made checkable rather than merely
claimed.

Proven end to end by `kernel/src/user/login_tests.rs` (both ISAs, `login_test_client.rs`): a correct
identity and secret produce a directory capability that answers a real `READDIR` and a budget that
retypes a real page (not merely that the capabilities arrived); a wrong secret is refused and nothing
follows the refusal, which the client checks by never calling `RECV_CAP` on that path rather than by
asserting a negative; and two different identities each get an independently working channel, correctly
named in the audit trail and in the order they were established. A fourth test proves the service
survives past what used to be a hard, silent ceiling of eight logins ever (a leaked cspace slot per
login; see BUGS), by taking the shared instance's login count to nine and checking each one's
directory and budget work.

See `user/src/login.rs`'s own BUGS for the itemised remainder (per-principal subtree scoping, the
terminal, boot integration, measured-boot consultation, and reclamation), summarised in this
milestone's BUGS below.

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
eventually bites (roughly tens of concurrently-durable sessions against `MAX_REGIONS`). This is a
general-purpose OS, not just a file server, so that headroom should be measured against cron's
uncapped, resource-bound norm rather than a consumer file-sharing throttle; §109 corrects an
earlier draft that used the wrong comparison. Raising `MAX_REGIONS` again, the same cheap move
already made once, is the expected response as durable sessions are actually built and measured.

**Sequencing.** After 47 (isolation is 47's per-shell root, and login hands out exactly what 47
defines; 47 landed 2026-08-22). The documentation and the group/caretaker write-up are cheap; a first
slice of the login service and its channel-shaped attribution logging landed in one lane. **Remaining
effort: at least 1-2 further lanes**, for the multi-user and boot-integration work named in BUGS below;
that estimate is a guess on the same history-calibrated scale the original one was, not a measurement.

## BUGS

Named here rather than only at the component, because a reader of the milestone should meet the scope
in the same place they meet the status line. Each item is also recorded where the reader meets the
feature (`user/src/login.rs`'s own BUGS, more precisely worded per item).

- **Every principal is attenuated to the same subtree**, `fs_proto::fixture::tree::SUB`, with the
  same rights. Milestone 47's per-shell root already builds the isolation mechanism (a
  `fs_subtree_caretaker` per grant); the wiring from an authenticated *identity* to a *specific*
  subtree name is **decided (DECISIONS §117, 2026-08-23)**: the identity string itself, used
  directly, created at provisioning time rather than auto-vivified at login. Building it needs
  milestone 155 (a provisioning tool) first, since provision-time creation has nothing to create
  the subtree today.
- **No terminal.** The roadmap's own text names three things a login hands back; `login` hands back
  two. A terminal in this system is a singleton hardware-backed resource wired once at interactive
  boot; minting a second one, or multiplexing the one that exists across logins, is unscoped follow-on.
- **Not wired into the interactive boot.** `login` is spawned directly by the kernel's guest test
  harness, the same way `credentialer` is, and is not reachable from `crates/system_initializer::boot`'s
  real prompt. Replacing the shell's build-time endowment with a real login prompt is the largest
  remaining piece of this milestone and the natural next lane.
- **`login` does not consult `measured_boot::PROGRAM_MEASUREMENTS` before building a caretaker.**
  Milestone 104's discipline (init refuses to load a program whose bytes do not match the archive's
  measurement table) does not extend to this non-init loader. Fixing it well means deciding how a
  loader outside the boot chain joins that chain at all, which is a design question and not a one-line
  patch.
- **A caretaker's construction memory is never reclaimed**; there is no logout. A real deployment
  needs a teardown path, which is real work this slice does not build. (A narrower, related bug was
  fixed in this same lane: `mint()` used to also leak one of `login`'s own sixteen cspace slots per
  successful login, which is a fixed table and not a splittable budget, bounding the service to
  exactly eight logins ever regardless of how generously `CONSTRUCTION_UT` was sized. `mint()` now
  drops its own copy of the region capability once the caretaker has confirmed descent, the same
  `cap_delete`-not-`DESTROY` pattern `root_supervisor` and `system_initializer::boot` already use.
  That removes the cspace ceiling; the memory one above is unaffected and is the real, still-open
  bound.)
- **The audit endpoint proves establishment, not per-request attribution.** DECISIONS §109 describes
  both a server establishing a channel and (separately) a server logging which channel a later request
  arrived on. `login` is only the first half. No server in this tree today needs the second: every
  existing multi-client server either serves exactly one principal by construction
  (`fs_subtree_caretaker`) or is anonymous by design (the credential service). Wiring the second half
  into a real multi-tenant consumer is follow-on for whenever such a consumer exists.
- **One client at a time.** `login`'s request and result endpoints are each a single endpoint, the same
  structural limit `credentialer.rs` documents for its own verify page. A second concurrent caller
  needs a channel per client, `fs_proto`'s answer, copied here when a second concurrent caller exists.
