# 139. Who may read the cycle counter, and by what authority

**Status: PROPOSED.**

**The number is provisional.** This lane could not see the other lanes running beside it, so 139 is
a claim on the next free slot rather than a mint; the integrator assigns the real one at merge, and
two lanes collided on a number the day before this was written. Cite it as
`design/decisions/139-cycle-counter-authority.md` until it lands.

Written 2026-09-02 by a research lane briefed to answer milestone 75
(who may read the cycle counter), which is `Gate: DECISION` and has been NOT-STARTED since
2026-08-03. It implements nothing. Milestone 75's own block is the question; this is the evidence and
the options, and the parts that are calef's are marked as his.

## What is being decided

**May a program running at EL0 read a cycle counter, and if so by what authority?** Three registers,
one question, three different answers today:

| | the fine counter | opened to EL0 by | state in this tree |
|---|---|---|---|
| aarch64 | `PMCCNTR_EL0` | `PMUSERENR_EL0.CR` or `.EN` | never written by this kernel |
| riscv64 | the `cycle` CSR | `scounteren.CY` (and `mcounteren.CY` in firmware) | never cleared by this kernel |
| `x86_64` | the TSC, via `rdtsc` | `CR4.TSD` clear | **open, and load-bearing** |

Milestone 74 (cycle counters) needs the answer before its
aarch64 half can land, and milestone 147 (a profiler that holds exactly the counters it was granted)
cannot be scoped at all until the grant unit exists.

## The premise was half false, and that is the most useful thing in this document

Milestone 75's block frames this as a decision about whether to **open** something that is closed.
Three checks say the framing is wrong in three different directions, and each one changes what the
decision has to cover.

### 1. On `x86_64` the counter is already open to every program, and nothing decided that

`kernel/src/arch/x86_64/boot.s:178` and `:385` are the only writes to `CR4` in the tree, and both
are `or eax, 1 << 5`, which is `PAE`. `TSD` is bit 2 and is never touched, so it holds its reset
value of clear, and ring 3 may execute `rdtsc`. `notes/x86-port.md` states this in its own words and
is the record that it was noticed rather than overlooked:

> `now()` is `rdtsc`, and ring 3 may read it because `CR4.TSD` is clear at reset and this kernel does
> not change it. That is the same shape as aarch64 needing `CNTKCTL_EL1.EL0VCTEN` and RISC-V needing
> `scounteren.TM`, with the difference that here the permissive state is the default and the kernel
> would have to act to *close* it.
>
> -- notes/x86-port.md

So one of the three supported architectures has already answered milestone 75 with option 1, by
inheritance. **A decision that says "closed unless granted" is not a decision to open something; on
`x86_64` it is a decision to close something programs already use.**

### 2. And closing it on `x86_64` would take the clock away, because there is only one register

`crates/user_rt`'s `now()` on `x86_64` **is** `rdtsc`. There is no coarse alternative on that
architecture the way `CNTVCT_EL0` is the coarse alternative on aarch64. So on `x86_64` the §10
clock exception (§10 says there is "no ambient authority", and notes/abi.md records the counter as
its one eyes-open exception) and this milestone's question are the **same register**. Setting
`CR4.TSD` today would break `Instant`, `thread::sleep`, the random seed, smoltcp's timestamps, and
the benchmark harness, all at once.

This is not an argument for leaving it open. It is the statement of what closing it costs, and the
shape of the fix already exists in this tree: §43 (reading the clock is a page) put the wall clock
in a page rather than a register, and a coarse monotonic value published in a page is the same move
one axis over. Nothing here proposes building that; it is named so the `x86_64` row is a decision
with a price rather than an exception with no plan.

### 3. On aarch64 and riscv64 the tree does not *establish* that the counter is closed, it assumes it

This is the finding worth acting on regardless of which option wins.

**aarch64.** Nothing in the tree writes `PMUSERENR_EL0`; the grep for it returns milestone 75's own
block, milestone 147's, and nothing else. Arm's register description says of every field in it,
`EN`, `CR`, `SW` and the rest:

> On a Warm reset, this field resets to an architecturally UNKNOWN value.

and of the cycle counter with `CR` and `EN` both 0, that EL0 reads "are disabled" and "generate an
exception to EL1, or to EL2 when EL2 is implemented and enabled for the current Security state".
Both quotations are from the `PMUSERENR_EL0` page of the Arm system-register reference at
`https://arm.jonpalmisc.com/latest_sysreg/AArch64-pmuserenr_el0`, read 2026-09-02. The two sentences
together say the trap is conditional on a value this kernel never sets.

Linux hit exactly this and fixed it by writing the register explicitly. Its commit
*"arm64: kernel: enforce pmuserenr_el0 initialization and restore"* (lkml.iu.edu archive
`1601.3/03556.html`, read 2026-09-02) says:

> The pmuserenr_el0 register value is architecturally UNKNOWN on reset.

and describes the exposure as platforms where "the pmu is not probed, therefore the pmuserenr_el0
register is not reset in the kernel, which means that its value retains the reset value that is
architecturally UNKNOWN".

Under QEMU this is almost certainly zero and the trap almost certainly fires. **On argon, the
Jetson TX1 that milestone 127 (the seL4 machine) is about, it is whatever TF-A and the boot ROM
left**, and nobody here has looked. This lane did not run a spike to find out; see BUGS.

**riscv64.** `kernel/src/arch/riscv64/timer.rs:182` opens the time CSR with
`csrs scounteren, TM`, a set of bit 1 and nothing else. The comment four lines above it says:

> CY (cycle) and IR (instret) stay closed.

Nothing clears them. They stay closed only if firmware left them clear, which is the identical
mistake that file's own comment records having found and fixed two paragraphs earlier: `user_rt`
documented U-mode `rdtime` as working "because the kernel sets scounteren.TM"; the kernel never set
it, and it worked on OpenSBI's default. **The same sentence, about the same register, is now true of
`CY` and untrue of `TM` only because somebody went and looked.** This is a claim stated in a comment
that the code does not establish, which is rung four wearing rung one's clothes.

**What follows from all three.** Part of milestone 75 is not a decision at all. Whatever authority
model wins, the kernel has to **write** these registers rather than inherit them, or the answer is
firmware's on every board. That part is a defect fix and this document recommends it outright.

### 4. And 74's aarch64 half is blocked on more than this

Milestone 127's other prerequisite, the EL2 to EL1 entry drop, is **PR #650 and is open, not
merged**, with `mergeStateStatus: DIRTY` as of 2026-09-02. Its diff does carry the `MDCR_EL2 = 0`
write and names `MDCR_EL2.TPM` as the trap that would otherwise catch every EL1 access to
`PMCCNTR_EL0`. So the EL2 half of the path exists and is real, and it is one merge away rather than
landed. Nothing in this document depends on that PR, but a plan that assumed it had landed would be
a day early.

## What this tree already does in the analogous case

Four analogues, and they do not all point the same way, which is why this needed reading rather than
recalling.

- **The generic timer is ambient, deliberately, and the record says why.** notes/abi.md calls it "the
  one ambient thing" and defends it: "A monotonic counter grants no authority to *affect* anything,
  only to observe the passage of time". `crates/uptime` inherits it and its module docs make the
  point that the program "needed no manifest field, no new capability, and no wiring".
- **The wall clock is a capability, expressed in objects the kernel already had.** §43 gives read as
  a read-only page, set as a writable page, and propose as an endpoint, with "**No new syscall, no
  new method number, no new object type**". That is the shape a cheap answer here would want to
  copy: an authority expressed in existing objects rather than a new type.
- **Entropy and the clock are both services, reached by capability**, so "a program that needs a
  privileged read asks a service" is the tree's normal case, not an exotic one.
- **`CNTKCTL_EL1.EL0VCTEN` and `scounteren.TM` are per-machine bits, set once at init.** There is no
  precedent in this tree for a per-thread system-register bit maintained across a context switch.
  That is the one piece of machinery option 4 below needs and the tree does not have.

## Prior art, read rather than recalled

**seL4, which is the one that matters, has both answers and ships the weaker one.** Its build option
`KernelArmExportPMUUser` is documented on `docs.sel4.systems/projects/sel4/configurations.html`
(read 2026-09-02) as:

> Grant user access to the performance monitoring unit. While useful for benchmarking, this option
> opens the possibility of timing channels.

It defaults off, and the same page records that `KernelVerificationBuild` excludes options of this
kind. So **seL4's published 413/426-cycle numbers are produced by a configuration seL4 does not
verify and does not recommend for production**, which is worth knowing before treating them as the
standard to match.

**seL4's own community has proposed exactly option 2 and has not landed it.** RFC-16, *"New
capability for the PMU"*, Krishnan Winter, proposed 2024-02-02, is `seL4/rfcs` PR #22, still open,
file `src/proposed/0160-pmu.md`. Read in full 2026-09-02. It says:

> Present profiling support uses the PMU through an ad-hoc interface that is designed for debugging
> and is consequently only available in a specific benchmarking configuration of the kernel. The
> same interface cannot be used in a production system as it is inherently insecure.

and

> Obviously the PMU presents a covert channel that exposes information about execution of user-level
> components (as well as the kernel). Therefore, PMU access needs to be explicitly authorised, which
> means we need an access-control model for the PMU.

and, on the current ARM situation:

> Additionally, on ARM systems, the only way to get access to the PMU from user-space is to
> configure the kernel to export access to the PMU registers, making the PMU an uncontrolled
> resource.

Its shape is a new object `seL4_PMU` with **badged** capabilities, the badge naming which counters
are authorised, and a blocking invocation. Its own unresolved questions include "How will the PMU
object affect verification? Initially it will not be available in verification builds of seL4".

**Linux has both answers too, and the arm64 one is the interesting half.** The global answer is
`perf_event_paranoid`, documented at `kernel.org/doc/html/latest/admin-guide/sysctl/kernel.html`
(read 2026-09-02) as controlling "use of the performance events system by unprivileged users
(without CAP_PERFMON)", default 2, with `-1` allowing "(almost) all events by all users". It is a
global sysctl, not a per-target grant, which is the criticism milestone 147 already makes of it.

The arm64 half is closer to what this decision needs. The commit *"arm64: perf: Enable PMU counter
userspace access for perf event"* (lkml.rescloud.iu.edu archive `2105.2/02527.html`, read
2026-09-02) enables `PMUSERENR_EL0`'s `ER` and `CR` bits **per task, on the context-switch hook**,
and states its reason:

> Only support user access when explicitly requested on open and only for a thread bound events.
> This avoids some of the information leaks x86 has and simplifies the implementation.

Two things fall out of that sentence and both bear on this decision. Per-thread, opt-in, maintained
at context switch is the **mainstream modern answer**, not an exotic one. And the "information leaks
x86 has" that Linux is avoiding are the consequence of the always-on `rdtsc` that **this tree has
inherited on `x86_64`** by the same default.

**L4Re: I could not source this.** The searches returned a virtualization paper and secondary
summaries rather than an L4Re or Fiasco.OC authority on PMU access control. Recorded as not
established rather than paraphrased.

## The options, with what each costs

Milestone 75's block names three. There is a fourth, it is the one the prior art converged on, and it
did not exist in the block.

### Option 1: ambient, like the generic timer

Set `PMUSERENR_EL0.CR` once at init, set `scounteren.CY`, leave `CR4.TSD` clear.

- **Cost to build:** aarch64 one `msr` in `timer::init` or `cpu` init; riscv64 one more bit in the
  existing `csrs`; `x86_64` nothing at all, since it is the state today. Call it three instructions.
- **Cost to the claim:** it spends §10's exception a second time on an instrument roughly 160x finer
  (0.25 ns against 41 ns), and it is the configuration seL4 declines to verify and declines to ship
  on by default. It is also the one that cannot be walked back: an ambient opening becomes something
  programs depend on, which milestone 75's own scope note names as the worst outcome.
- **What it is honest about:** it is what we already do on `x86_64`, so choosing it makes the tree
  consistent rather than making it worse.

### Option 2: a first-class capability object

A PMU object, a grant in the spawn path, a checked invocation. seL4's RFC-16 shape.

- **Cost to build:** a new object type, a new method number, spawn-path wiring, `caps` output, and
  Kani reach. Nothing in this tree prices at a morning. Milestone 147 says the counter-set and
  target-naming parts have "no precedent in this tree to price from".
- **Cost at the measurement:** this is the one that decides it. If the read is an invocation, the
  measured operation now contains a syscall, which is option 3's defect arriving through a different
  door. It is only free if the capability's *effect* is to open the register, at which point the
  capability is a grant of option 4 and the object is bookkeeping around it.
- **Cost to reverse:** highest on the list. A new object type and method number is the syscall
  surface, which §10 and §16 put in the expensive category, and milestone 147 would build on it.

### Option 3: kernel-mediated

EL0 asks the kernel to time an operation; the register never opens.

- **Refused, and 75 already refused it**, correctly: the measurement then contains the syscall it is
  trying to measure. Recorded so it stays visibly rejected.
- **One thing it is right for, which 75 does not say.** On riscv64 the SBI PMU route (`EID 0x504D55`)
  is inherently this shape: SBI calls are made from S-mode, so a U-mode program cannot make one and
  the kernel is in the path by construction. So RISC-V's cheap-read story is the `cycle` CSR and
  `scounteren.CY`, not SBI, and milestone 74's RISC-V half will want both for different jobs.

### Option 4: a per-thread grant, enforced at the context switch

The thread that was granted it runs with the counter open; every other thread runs with it closed.
The kernel writes the enable on the switch, the same way it writes the address-space root.

- **Cost at the measurement: zero.** The read stays one `mrs`, no syscall, no trap. It is the same
  instrument seL4's published numbers were taken with, which is what comparability requires.
- **Cost on the context-switch path: one comparison, and one `msr` only when the value changes.**
  That is exactly the shape `kernel/src/arch/aarch64/mmu.rs`'s `switch_user_root` already has (it
  early-returns when `TTBR0_EL1` already holds the wanted value), called from `sched.rs:1870`. If no
  thread is granted, the value never changes and the whole cost is a compare.
- **Cost to build:** a bit on the TCB, a write at the switch site on three architectures, and a way
  to set the bit. The last part is the expensive one: setting it is a syscall-surface change, either
  a field on TCB configure or a new spawn-path input, and that is calef's rather than a lane's.
- **The `x86_64` asymmetry survives this option and has to be decided separately.** `CR4.TSD` is
  writable per switch too, but closing it for ungranted threads removes `user_rt::now()` from every
  x86 program, so option 4 on `x86_64` is blocked behind giving that architecture a second time
  source. Until then `x86_64` is option 1 whatever the other two do, and a scope note should say so
  rather than letting §19 (architectural parity) report a gap it cannot close.
- **What it does not do:** it does not name a target the way milestone 147 wants. It says "this
  thread may read the counter", not "this profiler may read that subtree's counters". 147's work is
  still 147's.

## Recommendation

Split three ways, because the parts have different costs and different owners.

**Recommended outright, and reversible: close what we claim is closed.** Independent of the
authority question, and before any of milestone 74 lands:

1. Write `PMUSERENR_EL0 = 0` explicitly in aarch64 CPU init, per-core, rather than inheriting an
   architecturally UNKNOWN value. This is Linux's fix, for Linux's reason.
2. Clear `scounteren.CY` and `.IR` explicitly in the riscv64 per-hart timer init, so the comment
   that says they "stay closed" is made true by the code that says it.
3. Record in `notes/x86-port.md` and in `crates/user_rt`'s `now()` that on `x86_64` the cycle counter
   is ambient today, that this was inherited rather than chosen, and what closing it would cost.

Rung one is not available here (a register cannot be made unrepresentable), so this is rung two done
at init, plus a `BUGS` line where the reader meets it. It is three small writes, it is not the
decision, and it is the difference between a claim and a fact on argon.

**Recommended, and calef's to confirm because it touches the syscall surface: option 4.** It is the
only option that keeps the measured path free of a syscall while making the authority checkable, it
is what Linux arm64 converged on for the same reason, and it costs a compare on a path that already
does exactly this compare for `TTBR0_EL1`. Option 2 is the more seL4-shaped answer and is what
milestone 147 would eventually want; it is also unbuilt in seL4 after two and a half years, and
choosing it now buys a new object type before there is a second consumer, which is the speculative
abstraction both 74's and 147's scope notes already refuse.

**Options rather than a recommendation, because it is irreversible: how the grant is expressed.**
A field on TCB configure, a new spawn input, or a badge on an existing capability are three shapes
with three different syscall-surface costs, and a lane should not pick one. Nor should a lane decide
the `x86_64` row, since keeping `rdtsc` ambient there is a published confinement position and not
only an implementation state.

## Does this touch fatal risk 7?

**Yes, and the honest form of the answer is that it touches a claim the tree does not currently
make.** `notes/confinement-claims.md` enumerates 26 claims and names three more that are "stated
nowhere". The strings `timing`, `side channel` and `covert` appear in that note zero times, and zero
times in `DECISIONS.md` and in `design/fatal-risks.md`. So nothing in the confinement enumeration is
falsified by any answer here, because timing isolation is not among the things nife claims.

That absence is the finding, and it is the same category milestone 202 (every confinement test is a
ritual until somebody breaks the confinement) already found three members of. seL4 states the
position explicitly and in one clause ("this option opens the possibility of timing channels"), and
this tree, which will publish cycle-denominated numbers against seL4's, states nothing.

**What this decision should therefore also produce, whichever option wins: one row in
`notes/confinement-claims.md` stating what nife does not claim.** A confined component's *timing* is
not confined. That belongs beside the row saying a confined device's values are not confined, for
exactly the same reason: so nobody reads the capability rows as covering it.

## What it costs the benchmark story to say no

Real, and smaller than it looks, and it is worth being exact about who pays.

- **Milestone 25's `sel4bench` comparability is the part that genuinely needs it.** seL4's published
  413 and 426 are single-shot PMU measurements taken from user level. Reproducing that instrument on
  argon needs a user-level cycle read; a kernel-mediated timing of the same operation is not the same
  measurement and would not referee anything.
- **Our own numbers do not need it.** notes/pmu.md's whole point is that the long-loop generic-timer
  method is valid and is what survives virtualization: "Both are valid; they fail under different
  conditions." Milestone 168 (a multi-tasking workload benchmark) is a workload benchmark rather than
  a single-operation one, so it is a long-loop measurement and risk 4's decisive experiment is **not**
  blocked by a "no" here. That is worth saying plainly, because the brief that produced this document
  assumed otherwise, and the chain from this decision to risk 4 is weaker than it looks.
- **Milestone 74's most-cited payoff survives a no.** Turning "roughly 1,120 cycles at an assumed
  3.2 GHz" into a read number needs the counter read *somewhere*, and the kernel may read
  `PMCCNTR_EL0` at EL1 with no EL0 opening at all. What a no costs is the seL4-identical instrument,
  not cycles as a unit.

So a no is affordable for everything except the one comparison milestone 127 bought a board for.

## What is blocked until this is answered

- **Milestone 74's aarch64 half**, by its own gate. Its riscv64 SBI half is not, and neither is a
  kernel-side EL1 read.
- **Milestone 147**, entirely, by its own gate, since it cannot know what a grant unit is.
- **Nothing else.** Milestone 168 and risk 4 are not blocked, per the section above.

## BUGS

- **No spike was run.** The claim that an EL0 `mrs x0, pmccntr_el0` traps today under QEMU is
  inferred from Arm's register description plus the absence of any write to `PMUSERENR_EL0` in this
  tree; it was not observed. It was not run because the answer that matters is on argon, where the
  reset value is UNKNOWN and no emulator can report it, and because the aarch64 EL0 read is the one
  measurement that a QEMU run would answer least usefully. The `x86_64` claim was not spiked either
  and rests on reading `boot.s`'s two `CR4` writes and on notes/x86-port.md's own statement.
- **The context-switch cost is priced by shape, not measured.** "One compare, one `msr` on change" is
  read off `switch_user_root`'s structure. Nobody has measured what an added `msr` costs on the
  switch path on any of the three architectures, and on `x86_64` a `CR4` write is serializing and
  would not be free.
- **The riscv64 `mcounteren` half is untested.** Even with `scounteren.CY` set, U-mode reads of the
  `cycle` CSR require `mcounteren.CY` from firmware, which is OpenSBI's on radon and is not ours.
  Whether it is set there is unknown and is a bench check, the same shape as milestone 127's
  "`PMCCNTR_EL0` readable at EL1" item.
- **L4Re is missing from the prior art** and it is the one gap in that section. See above.
