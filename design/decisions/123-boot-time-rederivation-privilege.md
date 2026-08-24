# 123. Boot-time re-derivation: what grants the privilege, and how it dies after one use

**Status: PROPOSED.** Raised from milestone 152's own BUGS section (durable delegation), which named
this exact gap: "a privileged, boot-only operation... [but] what object grants that privilege, and
how it is scoped so it cannot be invoked again after boot, is not worked out." The number is
**provisional**, minted by this lane against the current tree; the integrator assigns the real
section number at merge, per this project's own convention for `DECISIONS.md` numbering.

## The question

Milestone 152 decided that boot-time bring-up of a durable session must be **re-derivation, not
restoration**: capabilities do not survive a reboot, so the kernel cannot "reload" a session's old
authority, it can only rebuild it fresh, the way a login would, but with no live person presenting
credentials at that moment. The roadmap doc says this should be "a privileged, boot-only operation,
in the same shape as `root_supervisor` handing out its authority once at boot and never again, not a
standing 'impersonate any user' capability left lying around afterward." It does not say what object
holds that privilege, what it is granted, or what mechanism actually prevents it from being invoked a
second time. That is what this decision answers.

This is worth being careful about because of what the operation actually is: something that can stand
up a durable session, and therefore a bundle of authority equivalent to a live login, **for any user
named in the on-disk schedule store, without that user presenting a credential.** If this capability
persists anywhere reachable after boot finishes, it is a master key. The whole design has to survive
the question "what stops this from being invoked again at 3pm."

## What else was considered

**(a) `root_supervisor`'s own shape, applied to session re-derivation instead of server construction.**
A dedicated boot-only process holds exactly the authority needed to re-derive durable sessions from
the on-disk schedule store, uses it once during bring-up, and then deletes every capability that
authority is made of, including its own copy of whatever names the store. After that its cspace holds
nothing that can re-derive a session, the same way `root_supervisor` proves at the end of its own run
that it holds nothing that can `RETYPE` or `RETYPE_OBJ` (`user/src/root_supervisor.rs` lines 168-182).
This is the recommended option below.

**(b) A persistent "master" capability retained by the kernel or an init process for the whole boot's
life, gated by a runtime check (e.g. "boot phase == early").** Rejected. This is weaker on this
tree's own terms (AGENTS.md's ladder, "the ladder, strongest first"): a runtime flag checked before
honoring the capability is rung 2, "a gate that fails loudly," not rung 1, "make the wrong state
unrepresentable." A capability that still exists, gated by a boolean, is one stray code path, one
missed check, or one future refactor away from being invoked outside the window the boolean was meant
to enforce. Capability deletion has no equivalent failure mode: there is no code path that can
re-invoke a capability that has been `cap_delete`d, because the slot is empty. The flag approach also
adds new kernel-visible state (a boot-phase indicator something can read or, worse, race) that
`root_supervisor`'s shape needs zero of. This is exactly the "make the wrong state unrepresentable vs.
a gate that fails loudly" tradeoff AGENTS.md's ladder names, and the ladder's own ordering answers it.

**(c) A capability minted with a kernel-enforced single-use bit (invoke once, then the kernel marks
the slot dead).** Considered and rejected as unnecessary machinery. This would be a new kernel
primitive on the capability surface, which AGENTS.md treats as one of the expensive, hard-to-undo
categories ("the syscall surface... is a boundary rather than a habit"). It also solves a problem
`cap_delete` already solves for free: a capability the holder has deleted from its own cspace is
already "invoke once, never again," without a new bit, a new invariant to prove, or a new Kani
harness. Building a bespoke single-use mechanism when ordinary deletion gives the same guarantee is
the kind of "more abstraction, more machinery" this tree's elegance tenet warns against, not more
elegant for having a named feature.

**(d) Trigger re-derivation lazily, on the first scheduled job's fire time, rather than all-at-once at
boot.** Considered briefly and set aside as a scheduling-shape question rather than a privilege-shape
one; it does not change what grants the privilege or how it is scoped, only when the privileged
process runs. It is compatible with the recommendation below (the boot-only process could re-derive
everything at once, or could itself be structured to defer per-session work) and is not this
decision's fork. Noted so it is not silently assumed away.

## What this tree already does in the analogous case

`root_supervisor.rs` is exactly this shape, already built and running: it holds `ROOT_UT` (a
construction budget) and a report endpoint, builds two child servers from that budget, delegates the
wiring capabilities to them, then calls `cap_delete` on every one of its own copies including
`ROOT_UT` itself. It then proves the deletion worked by invoking `RETYPE` and `RETYPE_OBJ` on the
now-deleted `ROOT_UT` and asserting both fail with `NoSuchSlot` (`invoke(ROOT_UT, abi::untyped::RETYPE, ...)`
returning a negative code, lines 174-176), reporting that proof over its own `REPORT` endpoint rather
than merely asserting it in a comment. This is not a one-off: `cap_delete`-after-use is an established
idiom across the tree, not invented for this decision. `login.rs`'s `mint()` deletes its own copy of
`tcb`, `ready`, and `narrow_ep` once a caretaker is stood up and handed its narrowed authority
(lines 597-623); `spawner.rs`, `credentialer.rs`, `swapper.rs`, `builder.rs`, `timetable.rs`, and
`crates/system_initializer` all do the same thing at the point a piece of authority has been handed
off and the holder no longer needs its own copy. The pattern this decision recommends is not new to
the tree; it is the tree's standard answer to "how do you make sure a capability is never used again,"
applied to a case (boot) that has not needed it before.

## What is prior art outside the tree

UEFI's `ExitBootServices` is the same shape at the firmware layer: the boot-time services table (disk
and console access, memory allocation) is explicitly and irrevocably torn down once control passes to
the OS, and calling into it afterward is undefined by spec, not merely discouraged. Measured/secure
boot chains generally follow the same rule: a privilege exercised once during a trusted bring-up
window and never reachable again, rather than a standing credential checked at every use. Neither is
read in depth here; the in-tree precedent (`root_supervisor`) is doing the real work of this decision,
and this section is included only because AGENTS.md's six questions ask for it explicitly.

## Is the premise true?

Checked rather than assumed. A repo-wide grep for `impersonate` finds exactly two hits outside this
milestone's own roadmap doc, both unrelated (a comment about badge-less capabilities in
`design/decisions/101-notification-objects.md`, and a comment about NVMe completion-tag wraparound in
`kernel/src/nvme.rs`). A grep for `re-derivation`/`rederivation`/`boot-only` finds only this
milestone's own roadmap doc and unrelated prose in other notes (`notes/x86-port.md`,
`notes/arch-audit.md`, `notes/frames.md`, none describing session re-derivation). There is no
existing boot-time re-derivation mechanism, no "impersonate" capability, and no session-revival code
path anywhere in the tree today. This is genuinely unbuilt, exactly as milestone 152's BUGS section
says.

The sibling on-disk schedule store this operation would read from also does not exist yet: there is
no `design/decisions/122-*` file in this tree as of this writing, and no per-user schedule persistence
format anywhere in `crates/` or `user/src/`. That dependency is real and is called out explicitly
below rather than guessed at.

## What each option costs

**The recommended shape (a), concretely:**

- **What it needs granted.** A construction budget (an `Untyped`, sized for however many durable
  sessions the store names, the same kind of budget `login.rs`'s `mint()` spends to build a
  caretaker) and read access to the on-disk schedule store from milestone 152's second piece. That
  second piece is a sibling decision (the store's format, write path, and read-at-boot path) that may
  not land before this one; this decision does not depend on knowing its shape, only on there being
  *some* read capability to grant, exactly as `login.rs`'s caretaker is handed a narrowed file-service
  endpoint without this decision needing to know the file service's internals.
- **What it must NOT be granted.** Network access (nothing about re-deriving local sessions needs it).
  The ability to re-derive on demand after the boot window closes; that is the entire scoping
  question and is answered below, not by omission here. Anything that would let it enumerate users
  rather than iterate the store it was hard-wired to read at construction (milestone 126's
  enumeration-is-authority rule, already invoked by 152's own reattachment design for the same
  reason).
- **Whether it needs a new dedicated process or folds into an existing one.** Either is viable and
  this decision does not need to pick: a small dedicated boot-only process (provisional name only,
  something like `session_reviver`; naming is calef's call per AGENTS.md, not this lane's) that runs
  once between `system_initializer`'s early boot and normal service startup, or a phase folded into
  `root_supervisor` or `system_initializer` itself, given the store-read capability and the
  construction budget as part of its own boot endowment. The cost difference between "new process"
  and "new phase in an existing one" is a few hundred lines either way and is not the load-bearing
  question; the load-bearing question is what happens to the capability once re-derivation finishes,
  which is the same in both cases.
- **The scoping mechanism itself, and why it is not merely "happens not to be invoked again."** The
  process (whichever one holds this) re-derives every session the store names, then calls
  `cap_delete` on the store-read capability and the construction budget, in the same breath
  `root_supervisor` deletes `ROOT_UT`. After that, its own cspace has no slot that names either
  capability: not a flag saying "do not use this," not a check that runs before use, but an empty
  slot. `NoSuchSlot` is what the kernel returns to *any* invocation attempt against a deleted
  capability, unconditionally, because the capability table is what the kernel consults and there is
  nothing in it. There is no runtime check to bypass, no phase variable to race, and no way for a
  later bug in this same process (or a compromise of it) to reconstruct the deleted capability, because
  nothing else in the system was ever the store-read capability's owner and therefore nothing else can
  re-derive or re-grant it. This is the same proof shape `root_supervisor` already performs and
  reports (attempt the operations the deleted capability would have permitted, over its own
  proof-reporting endpoint, and assert they now fail): a re-deriver could do the same, attempting a
  further store read or a further session build after its deletion pass and reporting the failure,
  which would make the "cannot be invoked again" claim a demonstrated fact rather than an assertion,
  exactly what this decision is closing.

**Cost of option (b), the runtime-flag alternative, measured against this:** it needs a new piece of
kernel-visible state (a boot-phase indicator) that does not exist today, a check inserted at whatever
invocation point the master capability's method dispatches through, and an argument for why that check
cannot be skipped, raced, or reached through a path someone adds later without noticing it is
security-relevant. None of that is quantifiable in lines of code today because no such flag exists to
measure; the point is qualitative and it is the one the ladder already settles: an added check is
weaker than an absent capability, full stop, regardless of how few lines the check would be.

## How reversible is this, and who has already acted on it

**Nobody has acted on this yet.** Milestone 152's design is worked out but "nothing here is built"
(the roadmap doc's own header), and this decision is about a mechanism, not a fact that has left the
machine. It is cheap to change: no code exists yet that depends on the answer, no wire format, no
persisted secret.

But the shape recommended here (a) needs **zero new kernel primitives**. `cap_delete`, `Untyped::SPLIT`,
and ordinary capability derivation are all §16's existing mechanism; nothing about re-deriving a
session at boot requires a new syscall, a new object type, or a change to the capability model. That
is a strong point in its favor independent of the scoping argument above: it is buildable today, on
the mechanism this tree already has and has already proven correct (§16's amendments, and the Kani
harness pinning `SPLIT`'s rights-inheritance invariant), rather than needing a new primitive designed,
proved, and added to the syscall surface first. Option (b) or (c), by contrast, would each need
exactly that: (b) a new kind of kernel-checked runtime state, (c) a new capability method. Both are the
expensive kind of decision under AGENTS.md's "move fast on what can be undone" tenet (the syscall
surface is named there explicitly as a boundary, not a habit), so even setting the scoping argument
aside, (a) is the only option of the three that does not itself require a further, slower decision
before it could be built.

## The recommendation

**Option (a): a boot-only process, holding a construction budget and read access to the schedule
store, that re-derives every durable session the store names and then deletes both capabilities from
its own cspace, in `root_supervisor`'s exact shape.** The scoping mechanism is local capability
deletion, proven the same way `root_supervisor` already proves it: by attempting the now-forbidden
operation afterward and reporting the failure. Whether this lives as a new dedicated process
(provisional name, calef's to ratify) or a phase inside `root_supervisor`/`system_initializer` is a
smaller, more reversible question this decision does not need to settle; either shape gets the same
scoping property, which is the part that matters.

## What is blocked until this is answered

Milestone 152's third BUGS gap stays open, and the boot-time half of durable delegation cannot be
built, until: (1) whether this is a new process or a phase of an existing one is decided (a smaller
fork, likely reversible enough for whoever is holding the problem at build time to just pick), and
(2) a provisional name is proposed if a new process is chosen. Neither blocks the *design* answered
here, which is the scoping mechanism itself.

## What this does NOT decide

- **The schedule store's on-disk format, write path, or read-at-boot path.** That is milestone 152's
  second piece, tracked separately (the sibling decision this lane was told not to touch). This
  decision only assumes some read capability over that store exists to grant; it does not shape what
  the store looks like.
- **Credential revocation's own mechanism**, or the consequence of revoking credentials. That is
  already settled, [DECISIONS §108](108-credential-revocation-kills-durable-session.md); this
  decision does not touch it and the two are independent (108 is about tearing a session down, this
  is about standing one back up at boot).
- **Whether a new process is warranted versus folding into an existing boot-time component.** Named
  above as a smaller, more reversible fork left open on purpose.
- **The exact name of any new process**, which is calef's call per AGENTS.md and is not proposed here
  as anything more than a provisional placeholder for discussion.
