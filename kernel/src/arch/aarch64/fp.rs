//! **A thread's floating-point and SIMD registers, aarch64.** The register file itself, the two
//! control-register writes that decide who may touch it, and nothing else: the policy that says
//! *when* to move it lives in the portable [`crate::fp`], which is the only caller.
//!
//! # What this architecture gives you, and what it does not
//!
//! `CPACR_EL1.FPEN` is a two-bit field that decides whether FP and Advanced SIMD instructions
//! execute or trap, per exception level. `0b00` traps at EL0 **and** EL1; `0b11` traps at neither.
//! Those are the only two values this kernel uses, and [`enable`] and [`disable`] are the whole of
//! the mechanism.
//!
//! What aarch64 does **not** give you is a dirty bit. RISC-V has `sstatus.FS`, a four-state machine
//! that says in hardware whether the registers have been written since they were last loaded, which
//! is exactly the question a context switch wants to ask. Here there is no such bit, so the
//! question has to be asked a step earlier and coarser: *has this thread ever executed an FP
//! instruction at all?* The answer is recorded in [`FpState::live`], it is set by the trap `FPEN =
//! 0b00` produces on the first such instruction, and it never goes back to false. That is a weaker
//! approximation than RISC-V's and it is the best the ISA offers.
//!
//! # BUGS
//!
//! - **`live` is a ratchet: a thread that touches FP once pays the save on every switch for the
//!   rest of its life**, even if it never touches FP again. Clearing it would mean knowing the
//!   registers are dead, which nothing here can know. RISC-V's `FS` could do better and this
//!   implementation does not exploit that either; see [`crate::fp`] for why the policy is uniform.
//! - **SVE and SME are not saved, and are not disabled either.** `CPACR_EL1.ZEN` and
//!   `CPACR_EL1.SMEN` are left at their reset values, which on every machine this kernel has run on
//!   means trapped. A part that reset them open would let a thread keep vector state this file does
//!   not move. Nothing in this tree emits SVE, and the check belongs with `arch::isa`'s feature
//!   reading rather than here; it is recorded rather than built.

use core::arch::asm;

/// **The whole of a thread's FP/SIMD register file**, plus the flag that says whether any of it is
/// worth moving.
///
/// `#[repr(C, align(16))]` and the padding are a contract with `fp.s`, which reaches the vector
/// area at a fixed offset of 16 and needs it 16-byte aligned for `stp q`. Reorder a field here and
/// the assembly writes the flag with `q0`.
#[repr(C, align(16))]
#[derive(Clone, Copy)]
pub struct FpState {
    /// Nonzero once this thread has executed an FP or SIMD instruction. **The save and the restore
    /// are both conditioned on it**, which is what makes a kernel full of soft-float threads pay
    /// two loads and two branches per switch instead of a kilobyte of memory traffic.
    live: u64,
    /// Padding, so the vector area below lands at offset 16 and 16-byte aligned. Never read.
    _pad: u64,
    /// `q0`-`q31`. 512 bytes, and the reason this file exists.
    q: [u128; 32],
    /// `FPCR`: rounding mode, exception masks, flush-to-zero. Architectural state, so it moves.
    fpcr: u64,
    /// `FPSR`: the cumulative exception flags a program reads to find out what its arithmetic did.
    fpsr: u64,
}

const _: () = assert!(size_of::<FpState>() == 544);
const _: () = assert!(core::mem::offset_of!(FpState, q) == 16);
const _: () = assert!(core::mem::offset_of!(FpState, fpcr) == 528);

impl FpState {
    /// **The state a thread starts with, and the state the registers are scrubbed to.**
    ///
    /// Zeros throughout, which for `FPCR` means round-to-nearest with every exception trap disabled
    /// and flush-to-zero off: the architectural reset configuration, and what a program compiled by
    /// anything sane expects to find. It doubles as the scrub value, so
    /// [`crate::fp::hand_over`] has exactly one thing to install when it takes the registers away
    /// from a thread rather than a second code path that zeroes them.
    pub const INITIAL: Self = FpState {
        live: 0,
        _pad: 0,
        q: [0; 32],
        fpcr: 0,
        fpsr: 0,
    };

    /// A fresh thread's state: no registers worth saving, and FP trapped until it asks.
    pub const fn new() -> Self {
        Self::INITIAL
    }

    /// Has this thread ever executed an FP instruction?
    pub fn live(&self) -> bool {
        self.live != 0
    }

    /// Record that it has. Called once, from the trap the disabled `FPEN` produced.
    pub fn set_live(&mut self) {
        self.live = 1;
    }
}

impl Default for FpState {
    fn default() -> Self {
        Self::new()
    }
}

/// `CPACR_EL1.FPEN`, bits 21:20. `0b11` here means "do not trap"; `0b00` means "trap at both EL0
/// and EL1".
const FPEN: u64 = 0b11 << 20;

/// **Put this core into the state the rest of this module assumes**: FP and SIMD trapped, for EL1
/// as well as EL0.
///
/// Called from `arch::init`, which every core runs (the boot core from `main`, a secondary from
/// `smp::secondary_main`), so this is per-core and not once per machine. `CPACR_EL1` is a banked
/// per-core register; a secondary that skipped this would run its threads with whatever its reset
/// left behind.
///
/// **Written rather than assumed, and that is the point.** QEMU resets `CPACR_EL1` to zero, which
/// is already the value this wants, so on the emulator this line changes nothing and could be
/// deleted without a test noticing. The architecture says the reset value is UNKNOWN. A part that
/// reset it to `0b11` would run every thread with FP open, [`crate::fp`] would never see a trap,
/// `live` would stay false on every thread, and two threads would quietly share a register file:
/// the exact failure this milestone exists to prevent, on hardware, silently, with the emulator
/// green.
pub fn init() {
    disable();
}

/// Let the current core execute FP and SIMD instructions.
///
/// The `isb` is not decoration. `CPACR_EL1` is a context-synchronizing control register: without
/// it, an FP instruction already in the pipeline may be resolved against the old value, and the
/// caller here is about to execute thirty-two `ldp q` in [`restore`].
pub fn enable() {
    // SAFETY: sets two bits in a control register that names no memory. The widest thing getting it
    // wrong can do is let a thread execute an FP instruction, or trap one it should not have; it
    // cannot hand anybody a page.
    unsafe {
        asm!(
            "mrs {t}, cpacr_el1",
            "orr {t}, {t}, {mask}",
            "msr cpacr_el1, {t}",
            "isb",
            t = out(reg) _,
            mask = in(reg) FPEN,
            options(nomem, nostack, preserves_flags),
        );
    }
}

/// Is the FP unit open on this core right now?
///
/// Exists for the test helpers in [`crate::fp`], which borrow the register file and have to put the
/// core back the way they found it. Nothing on the switch path asks: there the answer is `live`,
/// which is a fact about a thread rather than about a core, and reading `CPACR_EL1` to learn it
/// would be a system-register read per switch for something already in memory.
#[cfg(test)]
pub fn is_enabled() -> bool {
    let cpacr: u64;
    // SAFETY: reads one control register into a local.
    unsafe {
        asm!("mrs {}, cpacr_el1", out(reg) cpacr, options(nomem, nostack, preserves_flags));
    }
    cpacr & FPEN != 0
}

/// Trap FP and SIMD again, at both exception levels.
///
/// **The caller owes the scrub, not this function.** Disabling the unit does not clear the
/// registers, and a thread that cannot read them today may be able to read them tomorrow through a
/// speculation window nobody has thought of yet; that is the whole of CVE-2018-3665. The invariant
/// [`crate::fp`] keeps is that the file holds the running thread's data or the initial state, and
/// it installs the latter before calling this.
pub fn disable() {
    // SAFETY: as `enable`.
    unsafe {
        asm!(
            "mrs {t}, cpacr_el1",
            "bic {t}, {t}, {mask}",
            "msr cpacr_el1, {t}",
            "isb",
            t = out(reg) _,
            mask = in(reg) FPEN,
            options(nomem, nostack, preserves_flags),
        );
    }
}

/// Copy the live register file into `state`.
///
/// # Safety
/// `state` must be a valid, writable, 16-byte-aligned `FpState`, and FP must be enabled on this
/// core. Both are the caller's, and the only caller is [`crate::fp::hand_over`].
pub unsafe fn save(state: *mut FpState) {
    // SAFETY: forwarded. `fp_save` writes 544 bytes at `state` and touches nothing else.
    unsafe { fp_save(state) }
}

/// Load `state` into the live register file.
///
/// # Safety
/// `state` must be a valid, readable, 16-byte-aligned `FpState`, and FP must be enabled on this
/// core. Both are the caller's.
pub unsafe fn restore(state: *const FpState) {
    // SAFETY: forwarded. `fp_restore` reads 544 bytes at `state` and writes only registers.
    unsafe { fp_restore(state) }
}

unsafe extern "C" {
    /// `q0`-`q31`, `FPCR`, `FPSR` out to memory. See fp.s.
    fn fp_save(state: *mut FpState);
    /// The mirror. See fp.s.
    fn fp_restore(state: *const FpState);
}

/// **One harmless FP instruction, to take the first-use trap on purpose.**
///
/// `fmov d0, xzr` writes zero into a register [`crate::fp::load_pattern`] is about to overwrite
/// anyway, so it has no effect beyond the trap it provokes. Tests only: the shipping kernel is
/// `softfloat` and asks for the unit nowhere.
#[cfg(test)]
pub fn touch() {
    // SAFETY: writes one vector register. Under a trapping `CPACR_EL1.FPEN` this takes the enable
    // trap and is re-executed after it, which is the entire point of calling it.
    unsafe {
        asm!(
            ".arch_extension fp",
            "fmov d0, xzr",
            ".arch_extension nofp",
            options(nomem, nostack, preserves_flags),
        );
    }
}

#[cfg(test)]
impl FpState {
    /// Write [`crate::fp::register_pattern`] across all 32 lanes, each one's complement in the high
    /// half so the full 128 bits are exercised rather than only the `d` half a scalar `double`
    /// would occupy.
    pub fn set_pattern(&mut self, seed: u64) {
        for (index, lane) in self.q.iter_mut().enumerate() {
            let low = crate::fp::register_pattern(seed, index);
            *lane = (u128::from(!low) << 64) | u128::from(low);
        }
    }

    /// Is [`Self::set_pattern`]'s pattern still there, every lane of it?
    pub fn has_pattern(&self, seed: u64) -> bool {
        self.q.iter().enumerate().all(|(index, &lane)| {
            let low = crate::fp::register_pattern(seed, index);
            lane == (u128::from(!low) << 64) | u128::from(low)
        })
    }

    /// The low 64 bits of one lane, so a test can name the register that went wrong rather than
    /// reporting that something did.
    pub fn lane_low(&self, index: usize) -> u64 {
        self.q[index] as u64
    }

    /// Do the registers hold [`Self::INITIAL`]'s values, i.e. has the file been scrubbed?
    pub fn is_scrubbed(&self) -> bool {
        self.q.iter().all(|&lane| lane == 0) && self.fpcr == 0 && self.fpsr == 0
    }
}
