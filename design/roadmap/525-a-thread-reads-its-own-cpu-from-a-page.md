# 525. A thread reads its own CPU from a page

**Status: BUILT.** The number is **provisional**: the integrator mints it at merge, and 524 onward
is contested between branches that cannot see each other.

## What this is

calef ruled on 2026-09-21 that two different questions get two different mechanisms: a thread
observing **another** thread is a selector on the rendezvous surface, and a thread observing
**itself** is a per-thread page, on the shape Linux's `rseq(2)` uses. That ruling's
`design/decisions/` section was on another branch when this was built, so it is named here rather
than cited. This block is the self half.

The kernel writes the running core's id into a page it owns and has mapped read-only into the
thread's address space. Userspace reads it with an ordinary load. No syscall, no crossing, no
capability to hold.

## Why a load and not a crossing

The consumer decides it: a memory allocator keeping a per-CPU cache asks which core it is on **once
per allocation**, which is millions of times a second. An IPC round trip measures about 705 ns in
this tree and a bare syscall is cheaper than that, but neither is in the same decade as a load.

## Why a page and not a register, checked against our own targets

Linux's account is that the register path is x86-specific and the vDSO approach is impossible on
AArch64. That was confirmed here rather than taken as given, because the answer had to be about
these three:

| target | is there a user-readable core id? |
|---|---|
| aarch64 | No. `MPIDR_EL1` is EL1 and above. `TPIDRRO_EL0` is EL1-writable and EL0-readable and **could** carry one; this tree uses it for nothing, so it was refused rather than overlooked. |
| riscv64 | **No, and nothing to abuse.** `mhartid` is M-mode; S-mode has `sscratch` and U-mode has nothing. This is the target that decides it. |
| x86_64 | Yes: `RDPID`, and `LSL` against a GDT limit before it. Which is why the Linux vDSO's `getcpu` is an x86 story and arm64 ships no `__vdso_getcpu`. |

Architectural parity is a gate, §19 (architectural parity is a tenet). A mechanism that is a
register on one target and a page on another is two mechanisms to learn, test and document, to save
one load. The page is also the shape that survives the next fact, which was the deciding argument in
the ruling itself: calef expects a third and a fourth per-thread fact, and a page has room where a
register has none.

## What was built

- **`crates/current_cpu_protocol`** (name provisional): the layout, the magic, the sentinel, the
  per-architecture address, the writer and the reader. Nine host tests and three doctests.
- **`kernel::user::AddressSpace::attach_current_cpu_page`**: one frame from the global allocator,
  stamped and mapped `user_rodata`. Called from `AddressSpace::new` (every space the kernel builds,
  which is what made the six hand-built spawn paths free rather than six edits) and from
  `sched::configure_thread_control_block` (every space userspace builds and a TCB then binds).
  Idempotent, and silent on failure by design: a space with no page reads as unknown at the reader
  rather than refusing to load.
- **`kernel::sched::schedule`**: the write, off the borrow the switch already takes for the incoming
  thread's page-table root.
- **`user_mode_runtime::current_cpu`**: the reader.
- **`fixtures/src/current_cpu_reader`** (name provisional) and
  **`kernel/src/user/current_cpu_tests.rs`**: three kernel tests on all three architectures.

## The three questions this had to answer, and what it answered

**Per thread or per CPU.** Per thread, and here that is free: `Tcb::CONFIGURE` consumes the
address-space capability, so no two TCBs name one space: §105 (`std::thread::spawn` stays
declined).
A page per address space **is** a page per thread, at a fixed address, with no registration syscall
and no circularity. The circular shape the other answer would have had (a shared page indexed by the
CPU the thread is trying to learn) never arises. **What has to change when §105 is lifted** is in the
crate's `BUGS` section, beside the field it constrains.

**What an unset value reads as.** `None`, in two distinguishable ways, because zero is a valid CPU
id and calef ruled the same day that a wrong number is worse than no number. A frame nobody prepared
fails the magic check. A prepared page whose thread has never been switched in carries `u64::MAX`,
written at build time rather than left as a zero. No code inside the thread can observe the second
state, because a thread cannot execute an instruction without having been switched in, and that same
argument is why the first read is never stale.

**The memory ordering.** A relaxed store and a relaxed load, with no fence of this feature's own.
The counterpart is the scheduler's existing release/acquire handoff. The reason it is enough is
structural rather than clever: the core that writes the word is the core that is about to run the
thread, so writer and reader are the same hardware thread and program order does the work; two
successive writers (a thread moving between cores) are separated by the run-queue handoff that
already publishes the thread's saved context. An aligned 64-bit access is single-copy-atomic on all
three targets, so nothing tears.

## What it costs on the switch path

**Measured by removing the one line and rebuilding**, rather than against the recorded baselines,
because those turned out to be stale for reasons that are not this change's (see BUGS). Same tree,
same flags, the store commented out and back:

| ISA | `ipc_call_reply` without | with | this line costs |
|---|---|---|---|
| aarch64 | 7104 | 7172 | **+68 bytes, +1.0%** |
| riscv64 | 6004 | 6082 | **+78 bytes, +1.3%** |
| x86_64 | 8234 | 8350 | **+116 bytes, +1.4%** |

`syscall_entry` is byte-for-byte identical with the line and without it on all three, which is what
says the measurement is measuring the right thing: this code is not on that path.

**The comparison that matters is with `Thread::last_cpu`**, which sits two lines away in the same
function and is behind `feature = "soak_test"` because shipping it unconditionally cost 5.7% of
`ipc_fastpath` on aarch64, over the 5% bound of milestone 132 (the fast path's footprint). That
field pays for a `trace::record` call, which drags a whole symbol into the closure; this pays for a
load, a branch and a store. Roughly a fifth of the cost, and that is why this one ships in every
build while that one does not.

**A third measurement, because the obvious suspect was wrong.** `cpu::id()` is a division by
`size_of::<PerCpu>()`, so it looked like the expensive half; replacing it with a constant saved
**12 bytes on aarch64** and similar elsewhere. The cost is the `Option<PageFrame>` load, the branch
and the store, not the id. A cached per-core id would buy about 12 bytes and is not worth a field.

## BUGS

- **A space that never binds a TCB has no page**, and `user_mode_runtime::current_cpu` faults on an
  unmapped read there rather than answering `None`. Every space that runs a thread gets one; the
  uncovered shape is a bare address-space object with no thread to ask. Deliberately not mapped in
  `user_address_space_create`, because doing that for the timebase page cost two regressions and the
  comment recording them is still beside that function.
- **The value can be stale the instruction after it is read**, and nothing here fixes that. `rseq`'s
  restartable sequences are Linux's answer and are not attempted. A per-CPU cache built on this must
  be correct when the answer is the previous core's.
- **`CPU_ID_BOUND` is a compile-time constant, not the live online set.** Correct for sizing an
  array, useless for iterating one. Iterating the online set is the other question.
- **`script/fastpath-footprint`'s recorded baselines are stale on `main`, and this lane did not
  touch them.** Every leg reads over its baseline here, `syscall_entry` included, and that set is
  provably not this change's: it is byte-for-byte identical with this line and without it
  (riscv64 `syscall_entry` reads +4.6% against a 5% bound either way). The gate still passes on all
  three, so nothing is broken, but the headroom the next lane inherits is smaller than the file
  says. `maintainer/icount-baselines-stale` is an existing branch on the same subject and is where
  this belongs rather than here; re-saving baselines is a statement that a change is intended, and
  this lane is not the one that can make it about someone else's growth.
- **Nothing measures the allocator this was built for**, because there is no per-CPU allocator in
  this tree yet. The 20x and 35x figures in the crate docs are Linux's, about Linux, and are cited
  as the reason the shape was chosen rather than as a claim about nife.

## Follow-on

- **Proposed.** `design/roadmap/proposals/a-per-cpu-allocator-is-what-the-current-cpu-page-was-for.md`: the consumer this
  page exists for, and the only thing that turns the mechanism into a measured number.
- **Recorded.** What has to change when two TCBs can share an address space: in
  `crates/current_cpu_protocol`'s `BUGS` section, beside the field it constrains.
- **Recorded.** The fault rather than `None` for an unbound space: in `user_mode_runtime::current_cpu`'s
  own `BUGS` section, where a caller meets it.

## Index row

**Built:** 2026-09-21

A thread learns which core it is running on by loading a word from a page the kernel wrote, with no
syscall, because the consumer that decides the shape is a memory allocator asking once per
allocation. The register path was checked against all three targets rather than taken from Linux's
account of its own: riscv64 has no user-readable core id at all, aarch64 has only a register this
tree reserves for nothing, and x86_64's `RDPID` would have made the mechanism an x86 story with a
page underneath it everywhere else. The unset state is representable in two distinguishable ways,
because zero is a valid CPU id and a wrong number is worse than no number.
