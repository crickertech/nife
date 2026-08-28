# 188. The IPC fastpath: the gate measures a shape userspace does not use, and three cheaper cuts come before a hand-written path

**Status: NOT-STARTED.** Minted 2026-08-28, calef, out of the lane that gated the fastpath footprint
on the third architecture (pull request #574). The provisional framing he gave it was *"a
hand-maintained IPC fastpath, so the common case stops paying for the general one."* The title
changed because the scoping work below found that the premise needs checking before the fastpath
does: the gate that says we are over target is measuring an IPC shape that essentially no userspace
program in this tree performs, and the largest single item it reports on aarch64 is a symbol of
which 94% is never fetched.

**Gate: DECISION.** Phases 1 to 3 below are a lane's own call and need nobody. Phase 4, a second
hand-written path through the kernel's IPC, is calef's: it is a standing verification obligation and
a permanent maintenance cost, and this block recommends against starting it until phases 1 to 3 have
reported and milestone 74 (cycle counters) on milestone 127 (the seL4 machine) can observe whether
it bought anything.

## The starting numbers, and where each comes from

`bench/fastpath-{aarch64,riscv64}.txt` on `main` at fc7b04e7, and `bench/fastpath-x86_64.txt` from
pull request #574, unmerged at the time of writing. Written by `script/fastpath-footprint`, which
milestone 132 (the fast path's footprint) owns.

| | `ipc_fastpath` | `syscall_entry` | total |
|---|---|---|---|
| aarch64 | 5,788 | 3,304 | 9,092 (8.88 KiB) |
| riscv64 | 5,106 | 1,870 | 6,976 (6.81 KiB) |
| x86_64 | 6,639 | 1,637 | 8,276 (8.08 KiB) |

The target is in notes/benchmarks.md's own words, and it is not restated here because that section
states it better than a paraphrase would: **under 4 KiB of fastpath instructions and under 1 KiB of
data touched per IPC**, expressed as a fraction of the smallest L1i among the machines this project
runs on so that it tracks the board list rather than a number somebody liked. Every architecture is
over it on `ipc_fastpath` alone, before entry is counted. Read that section and the Liedtke argument
above it before touching any of this; both `script/fastpath-footprint`'s header and milestone 132's
block carry the same argument in shorter form.

**Two of the figures this milestone was briefed with are stale, and the corrections matter.**
`riscv64`'s `syscall_entry` is 1,870 and not 1,866. And milestone 132's "the largest single item is
`syscall::dispatch` at 2,024 bytes", which is where this milestone's whole premise came from, was
true on 2026-08-18 and is not true now. Measured in this lane's own worktree from a release aarch64
kernel built at fc7b04e7:

```
$ llvm-objdump -d --no-show-raw-insn target/aarch64-unknown-none-softfloat/release/kernel
   2020  exception_vectors
   1160  kernel::syscall::dispatch
    124  exception_dispatch
```

`dispatch` is **1,160 bytes**, down 864 from 2,024, and aarch64's `syscall_entry` is down by exactly
that same 864. Milestone 156 (extract the rest and ratchet both ways) did that, by moving the rare
administrative arms out of `invoke` into `#[inline(never)]` functions. That is the cheap option this
milestone was asked to price, and it has already been built once and measured.

## Finding 1: the gate measures the SEND/RECV shape, and userspace overwhelmingly uses CALL/reply

`script/fastpath-footprint`'s root list is `ipc_send`, `ipc_recv`, `schedule`, `finish_switch` and
`current_cap`. So `ipc_fastpath` is the closure over a **SEND/RECV** round trip: two endpoints, a
one-way message each way, four `svc`s from EL0. That is exactly the shape of the `ipc_rtt_el0`
benchmark, whose own doc comment says so (*"a server (RECV request, SEND reply) and a client that
self-times a loop of SEND-then-RECV"*).

It is not the shape the system runs. Counting imports of `crates/user_rt`'s helpers across
`user/src` and `crates`:

| helper | files importing it |
|---|---|
| `call` (`Rendezvous::CALL`) | 25 |
| `recv_cap` (`Rendezvous::RECV_CAP`) | 15 |
| `reply` (`Reply::REPLY`) | 11 |
| both `send` and `recv` in one program | 12 |

24 of `user/`'s 68 `[[bin]]` programs import `call`, plus one crate. The 12 that import both
`send` and `recv` include the sink protocol's streaming shape and the supervisors, which are not
request/response at all. The kernel's
own benchmark suite agrees with userspace and not with the gate: `call_reply`'s doc comment calls it
*"the one-endpoint shape real services use"*, and milestone 23's queue-broker note adds that the
steady state of a capability swap over a stable endpoint *"is [`call_reply`] above, instruction for
instruction"*.

**Measuring the shape userspace actually uses makes the gap worse, not better.** Re-running the same
closure with `ipc_call`, `ipc_recv_cap` and `ipc_reply` as roots in place of `ipc_send`/`ipc_recv`,
on the same binary, same cold list, same script logic:

| closure | aarch64 bytes | over the 4 KiB target by |
|---|---|---|
| SEND/RECV roots (what the gate measures) | 5,788 | 45% |
| CALL/reply roots (what a quarter of the programs run) | **7,516** | **88%** |

The difference is not a different code path bolted on; it is that `ipc_recv_cap` (1,828 bytes) and
`ipc_call` (1,692) are each larger than `ipc_recv` (1,320) and `ipc_send` (952), because they carry
the reply-capability mint, the capability-table insert into the server, and the `WaitRole::Reply`
parking that DECISIONS §12 (a one-shot reply capability) requires.

**And icount cannot see any of it**, which is the Liedtke argument arriving as a measurement rather
than as a quotation. Per-iteration icount ticks from `bench/baseline-*.txt`:

| | `ipc_rtt` | `call_reply` | delta |
|---|---|---|---|
| aarch64 | 1,017.3 | 1,040.1 | +2.2% |
| riscv64 | 170.0 | 174.9 | +2.9% |
| x86_64 | 16,734.6 | 17,150.6 | +2.5% |

The CALL/reply shape costs 2 to 3% more instructions on all three architectures and occupies **30%
more instruction footprint**. A tripwire on retired instructions is structurally blind to that gap,
which is precisely the hole milestone 132 was built to close, and it is currently pointed at the
wrong shape.

## Finding 2: aarch64's `syscall_entry` is dominated by a symbol that is 94% never fetched

`exception_vectors` is 2,020 of aarch64's 3,304, and it is not code in the ordinary sense. It is the
hardware's vector table: `kernel/src/arch/aarch64/vectors.s` builds it as sixteen `VECTOR_ENTRY`
macros at `.balign 0x80`, with the comment *"The hardware requires 2048-byte alignment. 16 entries x
128 bytes = 2048."* A syscall from EL0 traps to exactly one of those sixteen entries (index 8, lower
EL AArch64 synchronous) and fetches at most 128 bytes of the table. The other 1,892 are page faults,
IRQs, kernel-mode traps and the AArch32 entries this kernel will never support.

So aarch64's honest entry footprint is about **1,412 bytes** (one 128-byte vector entry, plus
`exception_dispatch` at 124, plus `dispatch` at 1,160), not 3,304. Two consequences, and the second
is why this is in the milestone rather than in a bug report:

- The three architectures' `syscall_entry` figures are even less comparable than pull request #574
  already recorded them to be. That note explains why x86_64's 1,637 is small (a `syscall`
  instruction reads `IA32_LSTAR` and consults no IDT entry, so its entry set is four symbols); it
  does not yet say that aarch64's 3,304 is large for the mirror-image reason.
- **A lane could "improve" aarch64's number by 1,892 bytes without changing one fetched
  instruction.** Counting one vector entry instead of the whole table is a correct accounting fix
  that would look like a 57% win. That is a gate that can be gamed, which is the one property
  AGENTS.md's ranking function says a measurement must not have.

`exception_restore` (92 bytes) runs on the return leg of every syscall and is in neither figure. It
is a small under-count in the other direction and should be added when the table is fixed.

## Finding 3: where the bytes are, per symbol

The whole closure, aarch64, both shapes, so a reader can see what a fastpath would be skipping
rather than guess at it.

| symbol | SEND/RECV closure | CALL/reply closure |
|---|---|---|
| `sched::ipc_recv_cap` | | 1,828 |
| `sched::ipc_call` | | 1,692 |
| `sched::ipc_recv` | 1,320 | |
| `sched::schedule` | 1,244 | 1,244 |
| `sched::ipc_send` | 952 | |
| `sched::finish_switch` | 824 | 824 |
| `sched::ipc_reply` | | 480 |
| `sched::wake` | 452 | 452 |
| `sched::current_cap` | 372 | 372 |
| `kmem::recycle` | 280 | 280 |
| `memcpy` | 272 | 272 |
| `switch_to` | 72 | 72 |
| **total** | **5,788** | **7,516** |

`schedule` + `finish_switch` + `wake` + `kmem::recycle` + `switch_to` is **2,872 bytes**, 38% of the
CALL/reply closure, and it is the general scheduler being asked to re-pick a thread that the IPC
already knows the identity of. That, and not `syscall::dispatch`, is the largest structurally
skippable block on this path. `syscall::dispatch` at 1,160 is now about an eighth of an honestly
counted round trip (7,516 of closure plus the ~1,412 of entry that is actually fetched), and 864 of
its former bulk has already been removed by the cheap method.

## What a fastpath would skip, and what it cannot skip

**Skippable. The first item is the only one with a measured size; the rest are read off the source
and a phase-3 lane should price them rather than assume them.**

- **The general scheduler re-selection, 2,872 bytes.** A rendezvous that finds a waiting partner
  knows which thread runs next. seL4's fastpath switches directly to it. Ours calls `schedule()`,
  which re-runs policy, and then `finish_switch`, which also carries the corpse-reaping branch that
  milestone 132 already had to classify as cold to keep the number honest.
- **`trace::record`**, called on every serve and every block in `ipc_call` and `ipc_reply`. It has
  no symbol of its own in the closure, so its bytes are inlined into the callers and are part of
  their 1,692 and 480 rather than a line of their own.
- **`kmem::recycle`, 280 bytes, if a successful rendezvous never reaches it.** The gate's closure
  includes it because `finish_switch` calls it; whether a switch that came from a completed IPC can
  reach it is a question phase 3 should answer, since a wrong answer here is the same silent
  mis-classification the cold list already risks.
- **The error-return plumbing.** `take_ipc_aborted()` is called after every rendezvous method in
  `syscall::invoke` and is a second load and branch on a path that has already succeeded.

**Not skippable, and this is the half that makes it a design milestone.**

- **The capability lookup.** `sched::current_cap(slot)` at 372 bytes is the bounds check that *is*
  the security mechanism; `syscall::invoke`'s own doc comment says so. A fastpath that caches or
  elides it is not a fastpath, it is a hole.
- **The rights check.** `SEND` needs `WRITE`, `RECV` needs `READ`, `SEND_CAP` needs `GRANT` on the
  delegated capability and a subset check on the narrowing. `CALL` needs `WRITE`.
- **The one-shot Reply mint and its consume-on-use.** DECISIONS §12 is the whole reason a CALL is
  answerable exactly once; the fastpath still has to mint the capability, insert it into the
  server's sixteen-slot table, and handle the table being full.
- **The `WaitRole::Reply` guard.** `ipc_reply` refuses any thread not parked as a reply waiter, and
  its comment records why: a stale reply landing on an ordinary receiver would clobber a mailbox and
  double-enqueue the one intrusive link. That guard is on the fast path by construction.
- **Generational thread-id revalidation**, which notes/benchmarks.md names as one of the three
  things ours does that seL4's fastpath does not.

## What correctness must not be lost, and how it would be proved

**The proofs are on the general structure, which is exactly the structure a fastpath exists to
bypass.** `crates/ipc` carries six `#[kani::proof]` harnesses over `Rendezvous`: that send and recv
and signal each preserve the one-queue invariant, that a send rendezvouses **iff** a receiver
waited and with exactly that receiver, that a pending signal is drained before a queued sender, and
that a collected sender is forgotten by the rendezvous, which is the rendezvous half of the one-shot
Reply guarantee. `crates/capability` carries twelve more. `script/verify` runs them.

None of them prove anything about `kernel/src/sched.rs`. They prove the pure decision core that
`sched` calls into. A hand-written fastpath has two possible relationships to that, and choosing
between them is most of phase 4's design:

1. **The fastpath calls `Rendezvous::send`/`recv` too.** The proofs continue to cover it for free.
   It also keeps the part of the cost that lives in the queue manipulation, so the win shrinks to
   the scheduler bypass and the skipped error plumbing. This is the option that should be measured
   first, because it may be most of the win for none of the verification cost.
2. **The fastpath replicates the decision.** Then it needs its own harnesses, plus something the
   tree does not have today: **an equivalence proof.** The shape is tractable precisely because
   `crates/ipc` is a pure-logic crate the solver already handles: a harness that seeds a
   nondeterministic `Rendezvous` with `kani::any()`, runs the fastpath's predicate and the general
   `send` over the same state, and asserts the same outcome and the same resulting queue state.
   That harness, not a test, is what would make a second path safe to keep.

**The invariants both paths must hold, stated so a phase-4 lane can turn each into an assertion.**
A message is delivered to exactly one receiver or to none. A capability is never widened. A Reply is
consumed on first use. A blocked thread appears on at most one queue. A thread's mailbox is written
only while it is parked in a role that expects that write. The first four already have harnesses in
`crates/ipc` or `crates/capability`; the fifth is `ipc_reply`'s `WaitRole::Reply` guard and is
currently held by a comment and a runtime check.

## How the two paths stay in agreement

**This is the honest reason not to do phase 4 lightly, and it is a cost that never ends.** seL4's
fastpath is hand-maintained, and hand-maintained means every future change to the IPC surface must
be made twice or must be proved not to reach the fast path.

A differential test that drives every operation through both paths and compares is the obvious
answer and **it is not sufficient on its own**, for the reason this tree already records about the
`fastpath-footprint` cold list: a wrong entry is silent. A fastpath's whole design is a predicate
that says "this case is simple enough"; a test can only exercise the cases somebody wrote down, and
the failure mode is a case that takes the fast path and should not have. What is sufficient is the
pair: the equivalence proof above for the decision, plus a differential test for the plumbing the
proof does not model (the capability table, the mailbox, the trap frame).

Three mechanisms are available and they are the AGENTS.md ladder in order, strongest first:

1. **Make divergence unrepresentable.** If the fastpath and the slow path both reach one function
   for the check they share, there is no second copy to drift. This is what option 1 above buys.
2. **A gate that fails loudly.** The equivalence harness in `script/verify`, plus a differential
   test in the kernel's own suite.
3. **A written record at the thing itself.** A comment on every rights check saying which path also
   performs it. This is the floor and should not be the plan.

## The alternatives, priced

Per AGENTS.md's six questions, and refusals with reasons, because the refusals are the useful half.

**A. Extract cold arms with `#[inline(never)]`. Already built; measured.** Milestone 156 did this to
`invoke`'s administrative arms and records its own before and after: `syscall_entry` 4,168 to 3,480
on aarch64 and 2,914 to 2,210 on riscv64, a 16.5% and a 24% cut. Further work since has taken those
to 3,304 and 1,870, so the entry figure is down 20.7% and 35.8% from the pre-156 baseline in total.
Over the same period `ipc_fastpath` went from 5,780 to 5,788, which is eight bytes: **the cheap
method has delivered its win on the entry figure and has not been tried at all on the fastpath
closure**, where 88% of the overrun now lives. That is phase 3.

**B. Reorder the dispatch decode so common opcodes are found first. Refused, and it is worth saying
why loudly.** It would not move this number by a byte. `script/fastpath-footprint` sums whole symbol
sizes; reordering a match changes which instructions execute, not which symbol they live in. It is
also already effectively done: `SYS_INVOKE` is one of four syscall numbers, `Object::Rendezvous` is
the first arm of `invoke`'s object match, and `SEND`/`RECV`/`SEND_CAP`/`RECV_CAP`/`CALL` are method
numbers 0 through 4. A lane that reached for this would spend a day and report a flat number.

**C. Shrink argument validation on the hot path. Refused.** The validation on this path is the
capability lookup and the rights checks, which the section above establishes cannot be skipped
without turning the fastpath into a vulnerability. There is nothing else to shrink: DECISIONS §10
(process model: capability-based, microkernel) means no pointer crosses the boundary, so there is no
user-memory validation on this path to begin with.

**D. Split dispatch into a hot decoder that tail-calls a cold one. Subsumed by A**, which is the
same idea with the extraction done per method rather than per decoder, and which is already built.

**E. Fix what is measured before optimising it. This is the recommendation**, and it is phases 1 and
2. It shrinks nothing and it is a precondition for every other option being judgeable.

**F. A hand-written fastpath, seL4's shape.** Phase 4. Deferred, not refused.

## Phasing

Each phase is a lane that can be picked up without reading the others. Phases 1 and 2 are hours,
phase 3 is a day, phase 4 is a project.

**Phase 1: measure the shape the system runs.** Add `ipc_call`, `ipc_recv_cap` and `ipc_reply` to
`script/fastpath-footprint`'s roots, or better, report the SEND/RECV and CALL/reply closures as two
named numbers rather than merging them, since they are two shapes and averaging them would hide
both, which is the reasoning the script already applies to `ipc_fastpath` and `syscall_entry`.
Re-record all three baselines with `--save`. Also correct notes/benchmarks.md's comparison section,
which says *"our EL0 path issues `SEND`, `RECV`, `SEND`, `RECV`"*: that is true of the `ipc_rtt_el0`
benchmark and false of every real service, which uses `CALL`. The corrected count is **three
syscalls to seL4's two** (client `CALL`, server `RECV_CAP`, server `REPLY`), not four to two, and
the residual one is the `ReplyRecv` fusion this tree does not have. That is a live open question
below, not a defect.

**Phase 2: make the entry figure honest.** Count one 128-byte vector entry rather than the whole
2 KiB aarch64 table, add `exception_restore`, and record in the script beside the `ENTRY` table why
each architecture's set is what it is (pull request #574 established that pattern for x86_64; this
extends it to the other two). Re-record. Expect aarch64's `syscall_entry` to drop to roughly 1,412
and **expect nothing to have got faster**, which is the point: the commit message must say the
number moved because the accounting was wrong.

While in that file, check it against milestone 186 (derive the architecture list). The script's
`arches` default is a hardcoded `"aarch64 riscv64"`, which is the exact defect 186's worklist
exists for, and pull request #574 fixed one instance of it by hand rather than by derivation. Phase
2 is the natural moment to take whichever mechanism 186 lands; taking it here without waiting for
186 would be a second hand-fix.

**Phase 3: apply milestone 156's method to the closure.** The extraction that took 864 bytes out of
`dispatch` has never been tried on `ipc_call`, `ipc_recv_cap`, `schedule` or `finish_switch`. Each
has cold tails that a successful rendezvous never reaches: the aborted-rendezvous returns, the
capability-table-full path, the corpse reaping in `finish_switch` that milestone 132 already had to
special-case as cold in the closure walk rather than in the code. Measure first, extract second,
and report the number even if it is small, because a small number here is what decides phase 4.

**Phase 4: the hand-written fastpath. Do not start this without phase 3's number and calef's
decision.** Build option 1 first (a fast path that still calls the proved `Rendezvous` methods and
skips only the scheduler and the error plumbing), measure it, and only then consider option 2 and
the equivalence proof it requires.

## The recommendation, with the numbers behind it

**Take phases 1 to 3. Do not commit to phase 4 yet.**

The reasoning is not a preference for less work; AGENTS.md's elegance tenet explicitly refuses that
argument, and the test it gives is *would I still choose this if both options were the same amount
of work?* The answer here is still yes, for three reasons that survive the effort question:

- **The premise this milestone was minted on has moved.** `syscall::dispatch` was the obvious target
  when it was 2,024 bytes and the largest single item. At 1,160 it is a sixth of the CALL/reply
  closure, and the largest structurally skippable block is now 2,872 bytes of general scheduling.
  A fastpath aimed at the decoder would be aimed at the thing that already shrank.
- **We do not currently know what a fastpath would be measured against.** The gate reports 5,788 for
  a shape a quarter of the programs do not run, 7,516 for the shape they do, and 1,892 bytes of
  aarch64's entry figure are never fetched. Building a second path against a number in that state
  means the before-and-after cannot be trusted, and a benchmark that cannot be trusted is worse
  here than no benchmark, because it will be quoted.
- **The payoff is unobservable on the hardware this project owns today, and this must be said
  plainly.** `script/fastpath-footprint` bounds a quantity, not a harm: nothing in this tree models
  a cache, and the HVF host's L1i is several times the boards'. `script/bench`'s icount tripwire
  models no cache either, and the 2 to 3% `call_reply` delta above is exactly what it can see of a
  30% footprint difference. **A milestone claiming a fastpath speedup today would be claiming a
  number nobody can measure.** Milestone 74 (cycle counters) on milestone 127 (the seL4 machine),
  with PMU cache-miss events on a 48 KB L1i, is what turns this from arithmetic into measurement,
  and milestone 25 (cross-OS performance comparison) is where the result would be published.

Phases 1 to 3 are each defensible on their own terms whatever phase 4 turns out to be worth: a gate
pointed at the right shape, an entry figure that cannot be gamed by 1,892 bytes, and the cheap
extraction applied where it has not been tried. If phase 3 gets the CALL/reply closure under 4 KiB,
phase 4 is not worth building and this milestone closes having said so, which is a better outcome
than a heroic one.

## Open questions, and the one that is calef's

**`ReplyRecv` fusion is a syscall-surface change and is explicitly not proposed here.** seL4's round
trip is two syscalls because it fuses reply-and-wait; ours is three because a server issues `REPLY`
and then `RECV_CAP` separately. Fusing them would be a new method on `Reply` or a new one on
`Rendezvous`, and DECISIONS §10 and §16 govern that surface: it is a boundary every future program
is written against, and AGENTS.md puts anything two programs agree on in the irreversible category.
**It is named here so it is tracked and not so it is planned.** Its cost is one trap per round trip,
which the tree has never measured on its own (notes/benchmarks.md says so in its own words: the
kernel-side `call_reply` bench measures the fused shape, but *"there is no EL0 twin of it, so the
structurally matched comparison to seL4's published pair is not currently measured at all"*).
Measuring it is phase 1 work; deciding it is calef's.

**Left open, for a lane rather than for calef.** Whether the two closures should be two gated
numbers or one, and if one, which. Whether the fastpath predicate should be a function both paths
call or a duplicated condition with an equivalence proof. Whether the data-footprint half of the
target (under 1 KiB touched per IPC), which milestone 132 records as measured by nothing at all, can
be estimated from the structures this path touches without waiting for a PMU.

## What this cannot deliver

- **No speedup anyone can currently observe.** Everything above is static footprint. The instrument
  that would show the harm does not exist on the machines in this house.
- **No answer for x86_64's `ipc_fastpath` being the largest of the three.** Pull request #574
  established that this is the same portable Rust in a denser ISA rather than a different path, so
  there is no x86_64-specific work hiding here, and milestone 161 (the x86_64 kernel port) owns
  anything that turns out to be.
- **No change to the syscall surface.** A fastpath changes how a call is serviced, not what calls
  exist.
- **Nothing about the data half of Liedtke's pair.** The under-1-KiB target has no instrument and
  this milestone does not build one.

## BUGS

Not started; nothing built yet to carry its own `BUGS` section. Three limitations of the *existing*
gate are recorded above rather than here, because a reader meets them in
`script/fastpath-footprint` and milestone 132 first: the root list measures a shape userspace does
not run, the aarch64 entry figure counts sixteen vector entries where a syscall fetches one, and
`exception_restore` is counted by neither figure.

## Scope note

Phases 1 to 3 are tooling, a baseline re-record and a mechanical extraction: no syscall surface, no
dependency, no wire format, no `DECISIONS` section. Phase 4 is none of those things and would need
its own scope note, its own `DECISIONS` section, and calef.

Names: nothing new is minted here. If phase 1 splits the gated figure in two, the two names are
calef's like every other name in the tree, and a lane should ship provisional ones and say so.
