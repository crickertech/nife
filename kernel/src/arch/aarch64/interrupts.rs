//! Masking and unmasking interrupts.
//!
//! On aarch64 this is the `I` bit of **`PSTATE.DAIF`**:
//!
//! | Bit | Name | Masks |
//! |-----|------|-------|
//! | 9   | `D`  | Debug exceptions |
//! | 8   | `A`  | SError (system errors) |
//! | 7   | `I`  | **IRQ** |
//! | 6   | `F`  | FIQ |
//!
//! Set the bit to **mask** (disable), clear it to **unmask** (enable). Note the polarity:
//! a `1` means "masked", which is the opposite of how most people read a flag called `I`.
//!
//! `msr daifset` / `msr daifclr` take a 4-bit immediate where bit 1 is `I`, so masking
//! IRQs is `msr daifset, #2`. These are single instructions that touch only the bits you
//! name, so there is no read-modify-write window to lose a race in.
//!
//! We only touch `I`. FIQ and SError stay as the boot protocol left them.

use super::instructions;

/// Bit 7 of DAIF, as read back by `mrs`.
const I_BIT: u64 = 1 << 7;

/// Are IRQs currently *unmasked*, i.e. can one fire right now?
pub fn is_enabled() -> bool {
    instructions::read_daif() & I_BIT == 0
}

/// Mask IRQs, and report whether they had been enabled.
///
/// The return value is the whole point. Callers must **restore** this state rather than
/// blindly enabling, or a lock taken inside an interrupt handler will re-enable interrupts
/// on release, inside a handler, which is a fine way to get a fault you cannot explain.
#[must_use = "the previous interrupt state must be restored, not discarded"]
pub fn disable() -> bool {
    let was_enabled = is_enabled();

    // If an IRQ fires between the read above and this instruction, it simply runs, and the state
    // we read is still the truth about what to restore.
    instructions::mask_irqs();

    was_enabled
}

/// Put IRQs back the way [`disable`] found them.
pub fn restore(was_enabled: bool) {
    if was_enabled {
        // Unmasking only when they were unmasked before.
        instructions::unmask_irqs();
    }
}

/// Unconditionally unmask IRQs.
///
/// Only for the places that legitimately turn interrupts on for the first time (the boot, a
/// secondary coming online, a thread's trampoline) and for tests. Everywhere else wants [`disable`]
/// + [`restore`].
pub fn enable() {
    instructions::unmask_irqs();
}
