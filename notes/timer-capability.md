# Can a userspace process hold a timer?

*(Written 2026-09-05 for milestone 263, a spike. This note prices a design and does not build one:
nothing here adds a syscall or an object, and the fork it informs is calef's. Name provisional, like
everything a lane mints: `timer-capability.md` is a sibling of `timed-wait.md` rather than a second
copy of it, and the two answer different halves of one question.)*

**The answer, first.** On riscv64 a userspace timer service cannot hold a timer, and the reason is
architectural rather than a gap in this kernel. There is no comparator at any address or CSR that
U-mode may write, on any RISC-V machine, and no configuration bit anywhere in the privileged
architecture that would open one. So the userspace-timer-service answer to milestone 106 **does not
survive [§19](../design/decisions/19-architectural-parity.md) parity.**

The other two architectures come out the other way, and one of them contradicts the milestone block
that scoped this spike. aarch64 **can** grant a timer comparator to one thread rather than to all of
EL0, using machinery this tree built for a different reason two days ago.

## What each specification actually says

Three specifications, read rather than recalled, because milestone 263's own `BUGS` section says the
table it was scoped from is this tree's doc comments and *"the maintainer's reading is the thing most
likely to be wrong here."* It was wrong about one of three.

### aarch64: the block's negative is refuted

The block says `CNTKCTL_EL1` "can open EL0 access, but that is ambient across EL0 rather than held by
one process." Both halves need correcting.

**The register has four independent enables, not one, and two of them are about the comparators
rather than the counters.** From Arm's own machine-readable system-register description of
`CNTKCTL_EL1` (rendered at `arm.jonpalmisc.com/latest_sysreg/AArch64-cntkctl_el1`, read 2026-09-05;
the normative text is the Arm Architecture Reference Manual for A-profile, `CNTKCTL_EL1`):

| bit | field | what it opens to EL0 |
|---|---|---|
| 0 | `EL0PCTEN` | `CNTPCT_EL0`, `CNTPCTSS_EL0` (and conditionally `CNTFRQ_EL0`) |
| 1 | `EL0VCTEN` | `CNTVCT_EL0`, `CNTVCTSS_EL0` (and conditionally `CNTFRQ_EL0`) |
| 8 | `EL0VTEN` | **`CNTV_CTL_EL0`, `CNTV_CVAL_EL0`, `CNTV_TVAL_EL0`** |
| 9 | `EL0PTEN` | **`CNTP_CTL_EL0`, `CNTP_CVAL_EL0`, `CNTP_TVAL_EL0`** |

Of `EL0PTEN`: *"Traps EL0 accesses to the physical timer registers to EL1, or to EL2 when it is
implemented and enabled"*, and when the bit is 1, *"this control does not cause any instructions to
be trapped."* `EL0VTEN` says the same of the virtual timer registers.

**This kernel writes only bit 1.** `kernel/src/arch/aarch64/timer.rs`'s `init` sets `EL0VCTEN` and
nothing else, which is why the tree's own comments describe the timer registers as trapped: they are,
because the bit that would open them has never been written. The registers exist and are grantable;
this tree has simply never granted them.

**And the grant can be per thread, because the tree now has the machinery.**
`design/decisions/139-cycle-counter-authority.md` says, in its "what this tree already does" section,
that `CNTKCTL_EL1.EL0VCTEN` and `scounteren.TM` are *"per-machine bits, set once at init. There is no
precedent in this tree for a per-thread system-register bit maintained across a context switch. That
is the one piece of machinery option 4 below needs and the tree does not have."*

**That sentence is now stale, and its own decision is what made it stale.** Milestones 229 and 237
built exactly that: a `cycle_counter_grant` bool on `Thread`, read in `sched::schedule` by
`cycle_counter_grant_of`, and installed on the core about to run the thread by
`install_cycle_counter_grant` -> `arch::timer::set_cycle_counter_grant`, which is a cached
compare-and-skip around a `PMUSERENR_EL0` write on aarch64 and an `scounteren` write on riscv64. A
per-thread `CNTKCTL_EL1.EL0PTEN` grant is the same shape at the same call site, on a register two
lines away in the same file. The precedent §139 wanted exists.

**What is genuinely scarce is comparators, not authority.** At EL1/EL0 aarch64 offers two: the
physical timer (`CNTP_*`, GIC INTID 30) and the virtual timer (`CNTV_*`, INTID 27). This kernel's
tick owns the virtual one, and owns it *because* of a portability finding this tree paid for: the
physical timer *"traps under a hypervisor: the physical timer belongs to EL2, and a guest at EL1 that
writes `CNTP_CVAL_EL0` takes an 'Unknown reason' trap"*, found on Apple's Hypervisor.framework
(`kernel/src/arch/aarch64/timer.rs`, milestone 9). So:

- There is **exactly one spare comparator** on aarch64, the physical timer.
- It is spare **on bare metal and under QEMU/TCG, and not under a hypervisor**, which is the same
  finding that pushed the tick onto the virtual timer in the first place. A grant that works on
  argon and fails under HVF is a parity problem of its own kind.
- It is **per PE**, so a service holding it is pinned to a core, or its deadline follows it.

### riscv64: the block's negative is confirmed, and it is total

Three routes, all closed.

**Sstc's `stimecmp` is S-mode, and U-mode is not mentioned because it was never a candidate.** From
the ratified *"Sstc" Extension for Supervisor-mode Timer Interrupts, Version 1.0* (RISC-V
International; the ratified PDF at `docs.riscv.org/reference/isa/extensions/sstc/`, read 2026-09-05):

> When the TM bit in the mcounteren register is clear, attempts to access the stimecmp register while
> executing in S-mode will cause an illegal instruction exception. When this bit is set, access to
> the stimecmp register (if implemented) is permitted in S-mode.

and

> Bit 63 of menvcfg [...] named STCE (STimecmp Enable) enables stimecmp for S-mode when set to one,
> and the same bit of henvcfg enables vstimecmp for VS-mode.

Every enable in the extension gates S-mode and VS-mode. There is no U-mode enable, and no U-mode
timer-compare CSR exists anywhere in the privileged architecture to be enabled.

**The CSR numbering forecloses it independently of Sstc's text.** The RISC-V Privileged Architecture's
CSR address convention (*Control and Status Registers (CSRs)*, the CSR listing chapter) says the two
bits `csr[9:8]` *"encode the lowest privilege level that can access the CSR"*, and that *"attempts to
access a CSR without appropriate privilege level or to write a read-only register raise
illegal-instruction exceptions."* `stimecmp` is `0x14D`: bits `[9:8]` are `0b01`, supervisor.
A U-mode `csrw stimecmp` is an illegal instruction by the address it is written at, and no
configuration bit changes that.

**SBI is not reachable from U-mode either.** This kernel's tick arms through `sbi_set_timer`, an
`ecall` from S-mode to OpenSBI in M-mode. An `ecall` executed in U-mode raises *Environment call from
U-mode* (`scause`/`mcause` 8), a different cause from *Environment call from S-mode* (9); it is
delivered to S-mode when `medeleg` delegates it and to M-mode otherwise, and in neither case does it
enter the SBI dispatch, which decodes only the S-mode and M-mode ecall causes. A U-mode program
cannot make an SBI call at all, which is exactly what the milestone block said.

**The one comparator the machine has is already spent.** RISC-V gives one `mtimecmp` per hart, in
M-mode's CLINT, and SBI TIME is the multiplexer over it. There is no second one, so even a
hypothetical MMIO grant of the CLINT would be handing away the kernel's own tick rather than a spare.

### x86_64: the exception is real, but it is not confirmed on the machine this project owns

The HPET is memory-mapped, has several independent comparators, and is therefore the one timer on the
three architectures that could be handed to a process with the tree's existing `DeviceFrame`
capability and no new mechanism at all.

What is confirmed in this tree: `crates/machine_discovery/src/acpi.rs` sees an `HPET` table in the
XSDT walk and does nothing with it, and `notes/x86-port.md` records the QEMU q35 table list read on
2026-08-23 with `0x000ffe22a8 HPET (56 bytes)` in it.

What is **not** confirmed: xenon's own HPET. `notes/x86-uefi-boot.md`'s first-light section (2026-09-05)
says the boot tour printed *"the full table list"*, but the transcript is a photograph
(`art/bench/xenon-2026-09-05-first-light.jpg`) and the list is not written down in the note. Nothing
in this tree names an HPET on xenon, and nothing has read its `NUM_TIM_CAP`. So the x86_64 row is
"the architecture has a spare, and this project has not looked at its own machine's".

`design/decisions/139-cycle-counter-authority.md` already read the HPET specification for a different
purpose and its findings apply: the architectural floor is 100 ns per tick, the common part runs at
14.31818 MHz, and a *read* costs several microseconds, which is why Linux deleted the vDSO mapping.
**None of that prices arming one**, which is a write and a one-shot interrupt rather than a polled
read, so §139's conclusion (the HPET loses to a syscall as a *clock*) does not carry over to using it
as a *timer*. That distinction is worth keeping straight, because the two uses share a device and
nothing else.

## What the fourth shape costs

`Timer::ARM(deadline, notification)` -> the kernel signals that notification at the deadline.

**The bookkeeping was already priced and is not re-measured here.** `notes/timed-wait.md` measured any
deadline structure at one comparison per idle tick (1.000 comparisons and 0.000 writes over 100,000
ticks for a scan and for a sorted list, 1.004 for a wheel) and a per-thread `deadline: u64` at zero
bytes, because TCBs are page-resident with 3,352 bytes of slack. What was unpriced is the **object**:
a new `Object` variant, its creation from untyped, its dispatch arm, and whether any of it lands on
the IPC fastpath.

**Method.** A throwaway scaffold was built on 2026-09-05 and then deleted: `Object::Timer(u64)`, an
`abi::timer` module with `ARM` and `CANCEL`, a `RETYPE_OBJ` arm minting one from the caller's own
untyped, two `#[inline(never)]` dispatch bodies, a 64-entry armed-timer table with a cached earliest
deadline, and an `expire` call from `sched::on_tick` that signals through the existing
`sched::irq_notify`. `script/test` passed with it wired in (205 passed, 69 skipped, all three ISAs
plus the OVMF leg) before it was removed, so the numbers are from a working system and not a
half-compiled one.

### The object is free in bytes

`kernel/src/cap.rs` asserts `size_of::<Object>() == 24` and `size_of::<Cap>() == 32` at compile time,
and **both assertions still hold with the variant added**, because `PageFrame(u64, NonZeroU64)` is
already the widest arm. So a `Timer` capability costs **zero additional bytes per capability slot**,
zero per capability table, and zero per TCB. That is the whole of the per-thread and per-process cost.

### The IPC fastpath is untouched, measured

`script/fastpath-footprint`, before and after, on the same machine within twenty minutes:

| ISA | `ipc_fastpath` before | after | `syscall_entry` before | after | delta |
|---|---|---|---|---|---|
| aarch64 | 7,028 | **7,028** | 1,504 | 1,516 | **+12 B (+0.8%)** |
| riscv64 | 5,936 | **5,936** | 1,828 | 1,986 | **+158 B (+8.6%)** |
| x86_64 | 8,122 | **8,122** | 1,637 | 1,733 | **+96 B (+5.9%)** |

**`ipc_fastpath` does not move at all on any architecture**, which is the number that matters: an
IPC round trip fetches exactly what it fetched before. `syscall_entry` moves because that half is
measured *flat* (the decoder's own bytes are on every syscall, and its other arms are not on this
path), so one more `Object` arm in `invoke` is one more decode step's worth of bytes for every
syscall in the system whether or not it is a timer.

The riscv64 figure is the one to argue about. 158 bytes is 8.6% of that ISA's entry set, and
`script/fastpath-footprint`'s own `BUGS` names this exact mechanism as an open problem
(`design/roadmap/proposals/a-flat-entry-set-counts-bytes-no-syscall-fetches.md`). Three instances were
closed with `#[inline(never)]` in two days on 2026-09-04; both scaffold bodies here already carry it,
so this is the cost *after* that mitigation rather than before it.

### The kernel grows by under a quarter of a per cent

Sum of symbol sizes in the release kernel (`llvm-nm --print-size`), before and after:

| ISA | before | after | delta |
|---|---|---|---|
| aarch64 | 1,009,252 | 1,009,990 | **+738 B (+0.073%)** |
| riscv64 | 1,011,788 | 1,014,054 | **+2,266 B (+0.224%)** |
| x86_64 | 962,936 | 964,322 | **+1,386 B (+0.144%)** |

That is the whole feature: object variant, two methods, retype arm, expiry table, tick hook, and the
signalling call. The riscv64 figure is again the largest, for the same reason its entry set is.

### What this does not price

- **The scaffold's expiry table is a 64-entry array with a linear rescan on every arm.** That is the
  crudest of the three structures `notes/timed-wait.md` modelled and was chosen because it is the
  smallest thing that runs; a real implementation would put the deadline on the object's own page,
  the way every other page-resident object in this kernel works, and the arm would not rescan.
  The size figures above are therefore an over-estimate of the table and an under-estimate of the
  revocation and generational-naming machinery a real object needs.
- **Revocation was not built.** A real `Timer` is retyped from untyped and must die when its region
  is destroyed (`MemoryRegion::DESTROY`, object revocation), which is bookkeeping the scaffold has
  none of.
- **Nothing was measured in time**, only in bytes. `notes/timed-wait.md`'s +30/+31 instructions per
  tick is the executed-path number and it still stands; nothing here changes it.

### The dependency on milestone 151, stated

The fourth shape signals **a notification**, and notification objects are
[§101](../design/decisions/101-notification-objects.md), decided 2026-08-20 and **unbuilt** (milestone
151). What the pricing above assumes about it:

1. **That the signal target is a `Rendezvous`, not a separate object.** The scaffold signalled through
   `sched::irq_notify`, which takes a `RendezvousId`, because that is what exists today. §101's whole
   argument is that a notification should be *its own object with its own queue*, separate from the
   endpoint. If 151 builds that, `Timer::ARM`'s second argument names a `Notification` and not a
   `Rendezvous`, and the dispatch arm's capability check changes shape. **The byte figures do not
   move much; the syscall's meaning does.**
2. **That binding to a TCB is what makes the shape useful.** Milestone 106's title is met only if a
   thread blocked in `RECV` on an endpoint wakes on *either* a message or the deadline, and §101 says
   that is what TCB binding is for. Without 151 the fourth shape gives a thread a timer it can block
   on and no way to block on a timer *and* a message at once, which is milestone 106's actual
   complaint. **So the fourth shape is not independently useful: it is 151 plus one object.**
3. **That §101's carve-out still stands.** §101 anticipated a userspace timer process and named a
   kernel timed wait as the alternative for kernel needs. This spike's finding is that the userspace
   process cannot exist on riscv64, which moves the fourth shape from "an option nobody listed" to
   "the only one of the four that is buildable on all three architectures". That is a finding for
   calef, not a recommendation from this lane.

## BUGS

- **Nothing here was run on hardware.** The aarch64 refutation is a register specification plus this
  tree's own code, not an EL0 program that armed `CNTP_CVAL_EL0` and took the interrupt. The cheap
  version of that experiment (set `EL0PTEN`, have a user program write the comparator, see whether
  INTID 30 arrives) was not run, and it is the thing that would turn this section from a reading into
  a measurement. Under a hypervisor it is *expected* to fail, for the reason milestone 9 recorded.
- **The Arm citation is a rendering of Arm's machine-readable system-register description, not the
  Architecture Reference Manual PDF.** developer.arm.com's register pages render their content in
  JavaScript and returned no field text to a fetch; the rendering used is generated from Arm's own
  published XML and agrees with this tree's existing `EL0VCTEN`-is-bit-1 fact, but it is a secondary
  source and is marked as one.
- **xenon's HPET is unconfirmed.** See the x86_64 section. Reading it needs either the first-light
  photograph transcribed or the boot tour re-run with the table list captured, and neither was done
  here.
- **The "one spare comparator" claim for aarch64 is about EL1/EL0 only.** `CNTHP_*`, `CNTHV_*` and
  `CNTPS_*` exist at EL2 and EL3 and are not this kernel's to give.
- **The scaffold is gone.** The variant, the `abi::timer` module, the retype arm, the dispatch bodies,
  the expiry table and the tick hook were built to obtain the numbers and deleted. Rebuilding them is
  an hour; shipping them would have settled a syscall-surface fork by accident, which is what
  milestone 106's lane refused to do and what §10 and §16 reserve to calef.
- **`Timer`, `Timer::ARM` and `Timer::CANCEL` are the milestone block's provisional coinages**, minted
  so the measurement could exist. Names are calef's (§75).
