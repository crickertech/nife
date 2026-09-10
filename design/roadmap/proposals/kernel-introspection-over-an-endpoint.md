# Kernel introspection over an endpoint, rather than one syscall per fact

**Status: PROPOSED 2026-09-09.** Raised by calef in conversation while deciding how `swish` reaches
a console on x86_64: *"Would it be useful to have the kernel be an IPC service so that it can expose
things only it knows? ... Or is there a security concern?"*

**Gate: DECISION.** This is a design fork about the kernel's shape, not work to schedule.

## In brief

Some facts exist only in the kernel: preemption counts, scheduler state, the machine description
milestone 268 prints at boot. Today there is **no way for a program to ask for any of them**, and the
obvious route is a syscall per fact, which grows a surface DECISIONS §10 and §16 exist to keep
narrow.

The alternative is a **kernel thread parked on a rendezvous**, answering questions. That mechanism
was verified while deciding the console case (`design/decisions/149-kernel-served-console-endpoint.md`):
`ipc::Rendezvous` is generic over `T: Node` and has no privilege level in it, and kernel threads
already exist. **It needs no new object type and no new syscall number.**

## The argument that makes this more than tidiness

**In a capability system, an endpoint is a better security primitive than a syscall, and it is easy
to get this backwards.** A syscall is reachable by every thread in the system, so restricting who may
read the scheduler's state means writing a check inside the kernel and a rule about who should call
it. An endpoint is reachable only by a program that was handed the capability, and the right can be
revoked. One is rung one of AGENTS.md's ladder; the other is rung four wearing a syscall's clothes.

It also changes the growth rate of the boundary. Over a syscall, each new fact the kernel learns is a
new method to design, document and never remove. Over an endpoint, it is a message.

## What argues against it

- **The kernel becomes a parser of untrusted input**, inside the trusted computing base. A syscall is
  already that, so this is more of it rather than new, but it is code that must be verified rather
  than argued about, and milestone 193 only recently put `kernel/src` within reach of the prover.
- **It erodes §14's minimal-core claim.** seL4's position is that the kernel provides mechanism and
  no services. Read-only introspection is the cheapest possible violation of that, since there is no
  userspace owner these facts could belong to, but it is not zero and a demonstrator's claims are
  what it sells.
- **"Only the kernel knows it" is sometimes a smell.** For some of these facts the right answer may
  be to give a userspace owner the data rather than to build a way to ask the kernel.

## Why it is not decided yet, deliberately

`design/decisions/149-kernel-served-console-endpoint.md` **refused to settle this** while deciding the
console, on the grounds that the console is *forced* by hardware and this is *chosen*, and that
deciding a chosen thing on the momentum of a forced one is how a narrow surface stops being narrow.
It said the question should be answered when a real consumer exists rather than in the abstract.

**Milestone 269 is that consumer**, and this proposal should be promoted to a `DECISIONS` section
when that milestone is taken rather than before.

## BUGS

- **No fact has been named as the first one to expose.** Preemption counts are the example calef
  raised, and nobody has checked whether a program would actually want them.
- **The denial-of-service shape is inherited from §149 and not re-examined here.** A read-only
  service that never blocks inside a handler is a weaker version of the same problem, but "weaker" is
  an assertion until somebody looks.
