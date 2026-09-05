# 147. A timer a userspace service cannot hold: how the timed wait gets served instead

**Status: PROPOSED.** Raised by milestone 263's lane, 2026-09-05, because that spike's answer
invalidates the premise of a decision calef made the same day. *(Section number provisional until the
merge queue lands it.)*

**No recommendation.** This is a syscall-surface question, which is calef's under
[§10](10-capability-microkernel.md) and [§16](16-object-revocation.md), and AGENTS.md's
own rule is that an irreversible fork arrives with options rather than a winner. What this file owes
is the options with their costs, which is the six questions answered rather than argued.

## What is being decided

Milestone 106 asks for a wait that ends on either a message or a deadline. calef decided on
2026-09-05 to serve it from a **userspace timer service signalling a notification**, rather than from
a new kernel blocking primitive, and minted milestone 263 to price the prerequisite: whether such a
service can hold a timer at all.

**It cannot, on riscv64, on any machine.** So the decision needs re-making, and this is the fork.

## Why the premise failed

The full working, with the specification citations, is `notes/timer-capability.md`, and the corrected
per-architecture table is in `design/roadmap/263-can-a-timer-be-a-capability.md`. The short form:

- **riscv64: closed three independent ways.** Sstc's every enable (`mcounteren.TM`, `menvcfg.STCE`,
  `henvcfg.STCE`) gates S-mode and VS-mode, and the privileged architecture contains no U-mode
  timer-compare CSR for any of them to enable. `stimecmp` is CSR `0x14D`, and the privileged spec's
  CSR address convention makes bits `[9:8]` the lowest privilege that may access it: `0b01`,
  supervisor. So a U-mode `csrw stimecmp` is an illegal instruction by its address, independently of
  Sstc's own text. And a U-mode `ecall` raises cause 8, a different cause from the S-mode ecall the
  SBI dispatch decodes, so the SBI TIME route this kernel's tick uses is unreachable from U-mode
  too. The one `mtimecmp` per hart is already spent on the tick.
- **aarch64: open, and milestone 263's block was wrong to say otherwise.** `CNTKCTL_EL1.EL0PTEN`
  opens `CNTP_CTL_EL0`, `CNTP_CVAL_EL0` and `CNTP_TVAL_EL0` to EL0, and the bit is an EL1 register
  the scheduler may rewrite per thread, which milestones 229 and 237 already do for `PMUSERENR_EL0`.
  The scarcity is comparators: one spare (`CNTP_*`), per PE, and it traps under a hypervisor.
- **x86_64: open in principle, unverified on the machine this project owns.** The HPET has several
  comparators in MMIO and would need no new kernel mechanism at all, since a `DeviceFrame` already
  delegates an MMIO page. Nothing in this tree records xenon's HPET or its comparator count. The
  IA-PC HPET Specification 1.0a's recommended minimum is three comparators, and §2.3.5's legacy
  replacement route spends two of them when it is on, so the floor is **one** free general-purpose
  timer rather than several.
- **And there is a spare-MMIO route on both real boards, which does not rescue the option.** argon's
  Tegra X1 has fourteen one-shot-capable channels on fourteen GIC SPIs; radon's JH7110 has four on
  four PLIC lines. But **QEMU's aarch64 `virt` has no MMIO timer at all**, and QEMU is what every gate
  in this tree runs on, so §19's *"proven by the same suite"* is not met: the service would be three
  different drivers and a hole. It is the right answer for one workload on one board and the wrong
  one for `thread::sleep`.

**[§19](19-architectural-parity.md) is what makes this a decision rather than a port task.** A
capability that ships on two of three architectures is a scope note at best, and this gap is
permanent: no amount of work closes it, because the RISC-V privileged architecture has nothing to
open.

## The options

### Option 1: the fourth shape, `Timer::ARM(deadline, notification)`

The kernel owns the comparator on every architecture, which is where two of three put it anyway, and
signals a notification at the deadline on the holder's behalf. A thread blocks in `RECV` with the
notification bound to its TCB ([§101](101-notification-objects.md)) and wakes on either.

**Measured** (milestone 263's scaffold, built, gated green with `script/test`, and deleted):

| what | figure |
|---|---|
| the `Object` variant | **free**; the enum is already 24 bytes wide because of `PageFrame` |
| `ipc_fastpath` | **unchanged on all three ISAs** |
| `syscall_entry` (flat) | aarch64 +12 B, riscv64 +158 B, x86_64 +96 B |
| kernel symbol bytes | +738 B / +2,266 B / +1,386 B, all under 0.25% |
| per-tick bookkeeping | already priced in `notes/timed-wait.md`: one comparison per idle tick |

**What it costs that is not bytes.** A syscall-surface addition, permanent, that every future program
is written against. And a dependency on milestone 151, which is unbuilt: without §101's TCB binding
this buys a thread a timer it can block on and no way to block on a timer *and* a message at once,
which is milestone 106's actual complaint. **Option 1 is 151 plus one object**, and pricing it any
other way understates it.

### Option 2: a userspace service on aarch64 and x86_64, with a riscv64 scope note

Keeps the 2026-09-05 decision on two architectures and records the third as a gap.

**What it costs.** §19 exists to make this expensive, and the usual mitigating argument does not apply:
a scope note normally records a port that has not been done, and this one would record something that
cannot be done. It is also worse than two-of-three suggests, because the aarch64 half rests on the
`CNTP_*` comparator, which traps under a hypervisor and is one per PE. riscv64 programs would need a different timed wait from the other two, which is the
"a feature works on one ISA and silently not another" shape rule 5 names as the bug. It also needs
aarch64's per-thread `CNTKCTL_EL1` grant built (the machinery exists; the policy does not) and the
hypervisor caveat accepted, and it needs xenon's HPET confirmed before anything on x86_64 is real.

### Option 3: reopen milestone 51's three shapes

`SYS_SLEEP`, a timer object, or a deadline argument on `RECV`/`CALL`. `notes/timed-wait.md` prices all
three and names no winner, deliberately. This spike does not price them and does not need to: they
were never blocked on the question 263 answered.

**What it costs.** The fork calef declined to decide on 2026-09-05 comes back, which is the outcome
his decision was avoiding. It is worth saying that the reason he could avoid it was the userspace
service, and that reason is gone.

## What is blocked until this is answered

- **Milestone 106**, which is what all of this is for.
- **Milestone 151** is not blocked, and is now on the critical path of every surviving option rather
  than beside it. A notification object is a prerequisite of 1 and 2 and useful to 3.
- **The four consumers 106 names** keep spinning: `thread::sleep`, `Endpoint::RECV`'s callers, the
  shell's `^C` poll, and `net_stack`, whose retransmit backoff burns a core, at a measured ratio of
  about `10^5` to 1 against a timed wait.

## What this file does not do

**It does not recommend, and that is on purpose.** AGENTS.md: *"Recommend on reversible forks; give
options only on irreversible ones"*, and a syscall-surface addition is the named example on the
irreversible side. Milestone 263's lane priced the options and stops here.

**It also does not amend [§139](139-cycle-counter-authority.md)**, which says *"There is no precedent
in this tree for a per-thread system-register bit maintained across a context switch."* Milestones 229
and 237 built one, so that sentence is stale inside its own decision. The correction is recorded in
`notes/timer-capability.md` and in 263's block; amending a decision section is not a lane's to do.
