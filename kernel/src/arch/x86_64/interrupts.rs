//! **Interrupt masking, `x86_64`.** The third implementation of the arch contract aarch64 states with
//! `PSTATE.DAIF` and RISC-V with `sstatus.SIE`.
//!
//! On x86 the switch is `RFLAGS.IF` (bit 9), set by `sti` and cleared by `cli`. Reading it means
//! reading RFLAGS, which has no `mov` form: `pushfq` puts it on the stack and `popq` takes it back.
//!
//! **The one hazard specific to this architecture** is that `sti` does not take effect until after
//! the *next* instruction retires. That delay exists so `sti; hlt` cannot race (an interrupt arriving
//! between the two would otherwise leave the CPU halted with nothing to wake it), and it is exactly
//! why `wait_for_interrupt` in this module's parent spells that pair out rather than calling
//! [`enable`] and then halting: two separate calls give the optimiser and the linker room to put
//! something between them, and the something is a hang.

use core::arch::asm;

/// `RFLAGS.IF`, bit 9: the interrupt-enable flag.
const IF: u64 = 1 << 9;

/// Read RFLAGS. There is no instruction that moves it to a register, so it goes via the stack;
/// `options(nomem)` is therefore wrong here even though nothing durable is written.
fn flags() -> u64 {
    let rflags: u64;
    // SAFETY: pushes and pops one word of this thread's own stack, leaving it as found.
    unsafe { asm!("pushfq", "pop {}", out(reg) rflags, options(preserves_flags)) };
    rflags
}

/// Are interrupts currently enabled?
pub fn enabled() -> bool {
    flags() & IF != 0
}

/// Mask interrupts, returning whether they were enabled before (for [`restore`]).
///
/// **Not atomic in the way the RISC-V twin is, and it does not need to be.** RISC-V clears its bit
/// and reports the old value in one `csrrci`; here the read and the clear are two instructions. An
/// interrupt landing between them is harmless: it is delivered while interrupts are still enabled,
/// which is what the caller's own prior state already permitted, and the value read is still the
/// state at the moment the caller asked. What would be a bug is the opposite order.
pub fn disable() -> bool {
    let was = enabled();
    // SAFETY: clears RFLAGS.IF. No memory effect.
    unsafe { asm!("cli", options(nomem, nostack)) };
    was
}

/// Restore the interrupt-enable state [`disable`] reported. Nesting composes because each `disable`
/// returns the exact bit its `restore` puts back.
pub fn restore(was_enabled: bool) {
    if was_enabled {
        enable();
    } else {
        // SAFETY: clears RFLAGS.IF. No memory effect.
        unsafe { asm!("cli", options(nomem, nostack)) };
    }
}

/// Unmask interrupts. Takes effect after the following instruction retires; see the module header.
pub fn enable() {
    // SAFETY: sets RFLAGS.IF. No memory effect.
    unsafe { asm!("sti", options(nomem, nostack)) };
}
