# 228. The cycle counters are closed by assumption, and on two architectures the assumption is a comment

**Status: BUILT 2026-09-02.** Minted 2026-09-02 by the maintainer, from the research lane that
produced DECISIONS 139 (who may read the cycle counter, and by what authority), which found this
while checking that decision's premise. *(Number provisional until the merge queue lands it.)*

It was minted `Gate: NONE`, and that held: this was a defect fix, deliberately independent of the
authority decision, and it is what the tree should do whichever way that decision goes.

**In brief.** Milestone 75 (who may read the cycle counter, and by what authority) is framed as a
decision about whether to **open** something closed. Checking that premise found the tree does not
establish it is closed at all:

- **aarch64: `PMUSERENR_EL0` is never written.** Arm states every field "resets to an architecturally
  UNKNOWN value", so whether EL0 may read `PMCCNTR_EL0` on **argon** depends on what its firmware
  left behind. This is precisely the bug Linux fixed in *"arm64: kernel: enforce pmuserenr_el0
  initialization and restore"*, for the same reason.
- **riscv64: the comment says the bits stay closed and nothing closes them.**
  `kernel/src/arch/riscv64/timer.rs` executes `csrs scounteren, TM`, which **sets** one bit and
  clears none, four lines below a comment reading *"CY (cycle) and IR (instret) stay closed"*. If
  firmware left them set they remain set. That is the identical latent-firmware-default shape the
  same file already records having found for `TM` itself.
- **x86_64: the counter is ambient, and that was inherited rather than chosen.** `CR4.TSD` is bit 2,
  never touched, so it holds its clear reset value and ring 3 may `rdtsc`. `notes/x86-port.md`
  records this as noticed rather than overlooked.

## What it needs

**Make the claim true on the two architectures where it is cheap, and make the third one honest.**

- Write `PMUSERENR_EL0 = 0` explicitly in aarch64 CPU init, per core.
- Clear `scounteren.CY` and `.IR` explicitly in the riscv64 per-hart timer init, so the comment is
  made true by the code beside it.
- **Do not touch `CR4.TSD` on x86_64.** `crates/user_rt`'s `now()` on that architecture **is**
  `rdtsc`, and there is no coarse alternative there the way `CNTVCT_EL0` is on aarch64. Closing it
  today would break `Instant`, `thread::sleep`, the random seed, smoltcp's timestamps and the
  benchmark harness at once. Record the position where a reader meets it instead: in
  `notes/x86-port.md` and beside `now()`, saying it is ambient, that it was inherited, and what
  closing it would cost.

**This changes no policy.** Every architecture ends where the tree already believes it is; two of
them stop depending on firmware to agree.

## What was built

2026-09-02. Three changes, one per architecture, and the third is not a code change.

**aarch64: `PMUSERENR_EL0 = 0`, per core.** In `kernel/src/arch/aarch64/timer.rs`, beside the
existing `CNTKCTL_EL1.EL0VCTEN` write rather than in a PMU driver that does not exist. The two
writes are one decision seen twice (may EL0 read the coarse counter, may it read the fine one), and
on riscv64 both answers live in a single CSR, so putting the aarch64 pair at one site is what makes
the three architectures legible side by side. `timer::init` is the per-core init: `smp.rs`'s
`secondary_main` calls it, which is what "per core" required, since `PMUSERENR_EL0` is banked per PE.

**The write is gated on `ID_AA64DFR0_EL1.PMUVer`**, and that was not in the brief. `PMUSERENR_EL0`
exists only when FEAT_PMUv3 does; without it a direct access is UNDEFINED, so an unguarded `msr`
would take an undefined-instruction exception on the first line of every core's timer init on a part
without a PMUv3. Linux writes it unguarded in `__cpu_setup`, so the precedent was available for
either choice; the guard was taken because DECISIONS §61 asks a `SAFETY` comment to be an assertion,
and "this `msr` cannot be undefined" is only true with the check above it. `PMUVer` of `0` means no
PMU and `0xf` means an IMPLEMENTATION DEFINED PMU that does not follow PMUv3, and neither carries the
register. Both are boards with no EL0 cycle-counter door to close.

**riscv64: `csrs scounteren, TM` became `csrw scounteren, TM`.** One instruction, and it turns the
comment four lines above it into the thing the code does. The whole-register write also clears the
`HPM` bits (3..31) for the U-mode hardware performance counters, which nothing in this tree reads and
which nothing ever claimed were open; a zero in this CSR can only take a U-mode read permission away,
never add one, so the wider write cannot open anything the narrower one would have shut.

**x86_64: nothing in the kernel, and two records.** The position is written where a reader meets it:
a `BUGS` section on `crates/user_rt`'s `x86_64` `now()`, and a subsection of `notes/x86-port.md`
carrying the three-architecture table. Both say the same three things, which are what the brief asked
for: the cycle counter is ambient here, it was inherited from the reset value rather than chosen, and
closing it today costs `Instant`, `thread::sleep`, the random seed, smoltcp's timestamps and the
benchmark harness at once, because on this architecture `now()` **is** `rdtsc` and there is no coarse
alternative to fall back to.

### How it was verified, and what verification is not available

**Nothing broke.** `script/test` is green on all three architectures, which is the load-bearing
result: had anything in this tree been reading a counter it did not have permission to read, closing
these bits is exactly what would have surfaced it, and the brief asked for that to be reported
loudly. There was nothing to report.

**The aarch64 write was confirmed by disassembly, not by inference.** `msr pmuserenr_el0, xzr` is
present in the built image at the call site, so the register is written rather than merely intended.

**What no run here can establish** is the value the write replaces. QEMU's reset value is almost
certainly zero, so a green run under QEMU is consistent with both the old code and the new one; that
is the whole reason this milestone exists. The values firmware actually leaves on **argon** and
**radon** stay unknown until somebody reads them at a bench, and that is milestone 127's list.

## Why it is worth doing before the decision rather than after

Because the decision is about what to **grant**, and a grant means nothing while the default is
unknown. DECISIONS 139's own recommendation puts this first for that reason, and it is the
difference between a claim and a fact on argon, whose firmware nobody has read.

## BUGS

- **It does not close the x86_64 hole**, and says so rather than implying three architectures now
  agree. Closing that one needs a coarse monotonic source that does not exist, of the shape
  DECISIONS §43 (reading the clock is a page) already used for the wall clock.
- **Nothing here checks the bits stay closed.** A later change could set them and no gate would
  notice, which is the same shape as the GICv2 assumption milestone 227 (a GICv3 driver, because
  GICv2 boots and silently loses every interrupt) was minted from. A boot-time assertion was
  considered and not written: reading `PMUSERENR_EL0` back proves only that this line ran, and the
  drift worth catching is a *later* write elsewhere, which only a periodic check or a review habit
  would see.
- **The actual firmware values on argon and radon are still unknown**, and reading them belongs on
  milestone 127's bench list beside its existing `PMCCNTR_EL0` item.
