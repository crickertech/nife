//! **Interrupt masking, RISC-V.** The S-mode analog of aarch64's `PSTATE.DAIF` handling.
//!
//! On RISC-V, S-mode interrupts are gated by one bit, `SIE` (Supervisor Interrupt Enable) in the
//! `sstatus` CSR. Setting it lets enabled sources (in `sie`) interrupt; clearing it masks them all.
//! These are leaf primitives with no dependencies, so they are implemented for real even though the
//! rest of the RISC-V trap path is not yet built. They are correct standard rv64, but unexercised
//! until the kernel boots on RISC-V.

use super::instructions;

/// `sstatus.SIE`, bit 1. Small enough for the CSR immediate-form instructions.
const SIE: usize = 1 << 1;

/// Are S-mode interrupts currently enabled?
pub fn is_enabled() -> bool {
    instructions::read_sstatus() as usize & SIE != 0
}

/// Mask S-mode interrupts, returning whether they were enabled before (for [`restore`]). Atomic:
/// `csrrci` reads the old `sstatus` and clears `SIE` in one instruction, so no interrupt can land
/// between the read and the clear.
pub fn disable() -> bool {
    instructions::read_and_clear_sstatus_sie() as usize & SIE != 0
}

/// Restore the interrupt-enable state [`disable`] reported. The paired half of `disable`; nesting
/// composes because each `disable` returns the exact bit its `restore` puts back.
pub fn restore(was_enabled: bool) {
    if was_enabled {
        enable();
    } else {
        instructions::clear_sstatus_sie();
    }
}

/// Unmask S-mode interrupts.
pub fn enable() {
    instructions::set_sstatus_sie();
}
