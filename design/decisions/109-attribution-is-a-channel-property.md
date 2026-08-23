# 109. Attribution is a property of a channel, not of a capability

**Status: DECIDED.** calef, 2026-08-22, on milestone 49's own named fork (worked out in
conversation, not yet built): channel.

## The question

Milestone 49's "In brief" names Unix's uid as doing four jobs at once, three already answered here
structurally. The fourth, attribution ("who did this?"), has no mechanism at all. The fork the
milestone names to settle first: does attribution become **a property of a capability** (an
invocation carries a stamped origin, seL4's badge mechanism), or **a property of a channel** (a
server logs which endpoint a request arrived on, established once when that endpoint was handed
out)?

## What else was considered, and why it loses

**Badged capabilities are not a new idea being weighed for the first time.** This exact fork has
come up three times already in this tree, independently, under different names, and lost every
time:

- **The compositor** (`notes/compositor.md`): every client shares one endpoint with no sender
  identity. Rather than badge it, the message is made to carry nothing and every per-client fact
  lives in per-client memory instead. "There are no badged capabilities here (DECISIONS §26.5
  records that decision and what would bring it back)."
- **The FS server / directory confinement** (`notes/dir-capability.md`): a shared endpoint's handle
  table is per-server, not per-client, so rights on a handle are not confinement. Badging is named
  as seL4's answer and explicitly not taken; the fix is a whole separate caretaker process, its own
  address space.
- **The fault endpoint** (`notes/supervision.md`): no badge is needed because the kernel is the only
  sender on this path. "seL4 solves the general untrusted-sender case with badged capabilities; that
  machinery returns as its own decision if a supervision endpoint ever needs trustworthy identity
  from userspace senders."

No badge field exists anywhere in the ABI today (checked directly: `crates/abi`, `crates/capability`).
In each prior case the actual requirement was confinement, not identity, and a badge answers neither
question a shared endpoint actually raised: it tells a receiver who called, never whether the
message content can be trusted, which is what compositor's and dir-capability's fixes both needed.

## Why seL4 built it and this tree has not

seL4 is a general-purpose foundation for downstream systems it does not control the shape of, some
of which legitimately want one cheap, shared endpoint serving many, perhaps many thousands of,
distinguishable clients (a resource manager fielding requests from many VMs, for instance). Minting
a badge is nearly free; minting a kernel object or a process per client is not, at that scale. This
tree has never had that scale problem: every server built so far serves a small, fixed number of
principals, and per-principal objects have been cheap enough that the tree reached for them three
times running rather than reach for a badge.

## The decision

**Channel.** A server that wants to know who is asking gives each principal its own endpoint,
established once (at login, at spawn, wherever the principal is created), and logs which one a
request arrived on. This is not a new mechanism; it is the pattern this tree already uses
everywhere identity-shaped information has mattered, generalized rather than invented.

**It composes with milestone 152 for free.** 152's durable per-user sessions already give every
downstream capability a traceable per-user origin: once a user's session exists, every service it
reaches through session-derived capabilities is reachable through a channel unique to that user.
Attribution at user granularity, which is what audit actually wants, falls out of 152's own shape
with no additional mechanism.

## What this costs, and what it does not

**Channel: no kernel change**, reuses proven machinery (§26's fault endpoint, caretaker
supervision, `SINK_BIT`-shaped delegation). The cost lands on server structure (one endpoint or
region per principal) rather than on the kernel.

**Badging would touch the IPC fast path** (the single most gated, most measured surface in this
kernel: `fastpath-footprint`, milestone 132's L1i-sized budget) for the first time in the project,
and would need new, proven kernel semantics for how a badge interacts with capability derivation,
copying and revocation. Real cost, for a feature with no current consumer.

**Where the channel model's own cost eventually bites, named rather than assumed away**: each durable
session, as milestone 152 designs it, plausibly owns at least one kernel region (`MAX_REGIONS = 256`,
`kernel/src/untyped.rs`, a system-wide concurrently-live cap), plus whatever caretakers it holds
long-term. A rough estimate, not a measurement: on the order of 50-70 concurrently-durable sessions
before the region table is a real constraint, against a system footprint of its own already
consuming some of that budget. `MAX_REGIONS` has been raised once already (16 to 256,
`notes/heap.md`) and raising it again is the first, cheap response if this tree ever approaches that
number, long before badging would be the right call.

**Comparative context, checked rather than assumed**: real desktop operating systems cap the
concurrent-connection case this tree is actually targeting (a file server) far lower than their
theoretical account-count limits. Windows Home allows 5 simultaneous SMB connections, Windows Pro
20, both contractually enforced (EULA), regardless of a UID space that is functionally unbounded.
macOS allows only 5 users logged in simultaneously, against a theoretical local-account ceiling
in the billions. The number that actually governs a file-serving role, in every mature OS surveyed,
is a small, deliberately low concurrent-session count, not total accounts. This tree's realistic
target (a family's worth of users) sits comfortably inside that same small-tens range, on the same
axis (concurrent sessions) that matters, before any of this becomes a real constraint.
[Windows connection limits (Windows OS Hub)](https://woshub.com/max-concurrent-connections-limit-windows/),
[macOS user account limits (Twocanoes)](https://twocanoes.com/knowledge-base/how-many-users-on-macos/).

## What this does not decide

**How a login service hands out per-principal channels**, and **the login service itself**, are
milestone 49's own build, not decided here. This settles only which shape attribution takes once
that exists.

## What it unblocks

Milestone 49's own gate is now clear: nothing further blocks starting the login service or the
channel-shaped attribution logging. Milestone 152 (durable delegation), gated on 49, inherits a
settled answer to how attribution composes with its own design.
