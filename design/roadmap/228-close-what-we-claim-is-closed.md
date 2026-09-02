# 228. The cycle counters are closed by assumption, and on two architectures the assumption is a comment

**Status: NOT-STARTED.** Minted 2026-09-02 by the maintainer, from the research lane that produced
DECISIONS 139 (who may read the cycle counter, and by what authority), which found this while
checking that decision's premise. *(Number provisional until the merge queue lands it.)*

**Gate: NONE.** This is a defect fix and is deliberately independent of the authority decision. It
is what the tree should do whichever way that decision goes.

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
  GICv2 boots and silently loses every interrupt) was minted from.
- **The actual firmware values on argon and radon are still unknown**, and reading them belongs on
  milestone 127's bench list beside its existing `PMCCNTR_EL0` item.
