# 505. An x86_64 input driver that never lets the core idle

**Status: NOT-STARTED.** *(Number provisional until the merge queue lands it.)* Promoted from the
proposal `an-x86-64-input-driver-that-never-lets-the-core-idle`, filed 2026-09-19, on calef's
instruction of 2026-09-20 to give every proposal on `main` a number. The text below is the
proposal's own, unedited except for this paragraph: the argument is its author's and promotion is
not the moment to improve it.

Found by milestone 182's lane while building `script/shell-check`'s
x86_64 leg. Milestone 299 recorded x86_64's polling input driver as a latency and CPU limitation;
this is the measurement that says it is more than that.

**Gate: NONE.** As far as the lane could see, nothing gates it. The `Irq` capability, `irq_wait`/`irq_ack` and the
input driver's interrupt-driven arm all exist on aarch64 and riscv64, and milestone 299's follow-on
names what x86_64 lacks: delivering a device line (COM1's legacy IRQ 4, through the IO APIC) to a
userspace waiter, where today the kernel delivers only self-directed vectors to a driver. If that
turns out to need a new capability method or syscall, it becomes a design fork and stops there.

**In brief.** `components/src/input.rs`'s x86_64 `_start` is `loop { drain(); yield_now(); }`. A
thread that always yields is always runnable, so from the moment it starts the run queue is never
empty and `sched::run_idle` never runs again on that core. Two things live in the idle loop and both
stop:

- **The halt.** QEMU sat at 99 to 100% of a host core at an x86_64 prompt with nothing typed (76
  CPU-seconds in 78 wall-seconds, patagonia, 2026-09-19, one core under OVMF). AGENTS.md's `wfi`
  rule exists because a halted kernel spinning cost 99.7% of a core; this is the same cost at the
  prompt, and on a PC it is a fan and a battery.
- **The capability-slot gauge** (`kernel::cap::report_peak`, milestone 231). It prints the mark at
  the hand-over, 5 of 24, and never updates; the peak during `shell-check`'s script is 17, read by a
  temporary instrument. The gauge's `ABOVE` check therefore cannot fire on x86_64.

## What to build

Route IRQ 4 through the IO APIC to the `Irq` capability the progenitor already grants at slot 2 and
already has the slot layout for (`kernel::user::boot_progenitor`), arm IER's receive bit in the x86
`uart` arm (the register layout is already written there), and give x86_64 the same `_start` the
other two run: drain, arm, then `irq_wait`/drain/`irq_ack`. Then delete the x86 `_start` twin.

## How to know it worked

- `script/shell-check --arch x86_64` prints a slot gauge equal to the peak (17 of 24 today, or
  whatever it measures), and the leg's "that gauge is stale" caveat in `xtask/src/main.rs` is deleted.
- QEMU's host CPU at an idle x86_64 prompt drops to near zero, measured the way the number above was.
- `script/shell-check`'s BUGS entry and milestone 182's two BUGS entries on this are closed.
