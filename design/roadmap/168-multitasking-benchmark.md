# 168. A multi-tasking workload benchmark: the number that would decide the event-kernel question

**Status: NOT-STARTED.** Minted 2026-08-25, from [DECISIONS §96](../decisions/96-process-kernel-or-event-kernel.md)'s own recommendation: *"Build the instrument that could decide it. The blocker is that a multi-tasking workload is the only place the difference appears, and we have none."*

**Gate: HARDWARE, MILESTONE 127.** §96's own text: *"milestone 127's TX1 is where such a number would finally mean something."* Milestone 127 is itself `NOT-STARTED` as of this writing (its kit was in transit, estimated arrival 2026-08-19 to -26; check its own current status before starting, since that window has since passed). This milestone cannot produce a meaningful number without it: a multi-tasking difference is exactly the kind of result that needs real hardware scheduling behavior, not TCG or HVF timing.

## What this is for

DECISIONS §96 asks whether nife should stay a process kernel (what it is today: every thread gets its own kernel stack) or move to an event kernel (one stack per core, explicit continuations), the model seL4, OKL4 and NOVA all eventually adopted. §96 found three of the four inputs to that question already settled by measurement (memory savings are negligible at this project's scale; stack-shrinking is closed off, no slack remains; the verification argument doesn't transfer, since Kani never reaches `kernel/src` here). The fourth input — performance — is the one live, unmeasurable argument: the paper §96 cites (Warton, on Pistachio) found event kernels roughly tied with process kernels on micro-benchmarks but **20% better on a real multi-tasking workload (AIM7)**. Every instrument this project currently owns (`ipc_rtt`, `ipc_rtt_el0`, the icount tripwire, milestone 132's footprint gate) is a micro-benchmark, and would show approximately nothing for this question.

**This milestone is that missing instrument, and nothing else.** It does not decide §96; it produces the number §96 needs to decide itself.

## What it needs

- **A real multi-tasking workload**, not a single-operation timing. The cited paper's own instrument is AIM7 (see `notes/l4-lessons.md`'s citation for the exact numbers this project has already quoted from it: *"20% performance advantage on a multi-tasking workload (AIM7)"*); whoever builds this should read the paper's own methodology rather than guess at AIM7's shape from the name, and decide whether to port something AIM7-equivalent or design a workload that exercises the same property (many threads, contended scheduling, real context-switch pressure) in a way that fits this project's own capability model.
- **Real hardware**, per the gate above — this specific difference does not show up under QEMU TCG or Apple's HVF, the same reason `sel4bench` (milestone 25) is deferred to real silicon.
- **A place to run it that has real PMU access**, matching what milestone 25's own `sel4bench` piece already needs from milestone 127's machine, so the two pieces of hardware-gated work should likely be sequenced together rather than treated as unrelated.

## Why it matters, beyond §96

**Milestone 25's cross-OS comparison has the same hole**, and this milestone closes it too rather than duplicating it. Checked directly: milestone 25 is explicitly a set of EL0-measured *primitive* benchmarks (single syscall, single context switch, single IPC round trip, single map, single spawn) compared against lmbench and `sel4bench` — every one of them a micro-benchmark in the same sense §96 means the word. Milestone 25's own remaining piece (`sel4bench`) is also single-operation PMU timing, not a multi-tasking workload. So neither milestone currently has an instrument that could show what a real multi-tasking difference looks like, and building one here serves both.

## What this does not decide

Whether nife should actually switch kernel models. That is DECISIONS §96's own question, and it stays open until this milestone's number exists (or until a real customer-path workload starts creating threads in the hundreds, the other condition §96 names for reopening early).
