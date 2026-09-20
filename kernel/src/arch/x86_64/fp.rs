//! **A thread's floating-point and SSE registers, x86_64.** The register file, the control-register
//! bits that decide who may touch it, and nothing else: the policy is [`crate::fp`]'s.
//!
//! # Four bits, and one of them has a CVE
//!
//! - **`CR4.OSFXSR`** says the operating system knows about `fxsave`/`fxrstor` and about the `xmm`
//!   registers. Clear, every SSE instruction raises `#UD`, which is why this kernel's `-sse`
//!   userspace has been safe until now without any of this existing.
//! - **`CR4.OSXMMEXCPT`** says an unmasked SIMD floating-point exception should arrive as `#XM`
//!   (vector 19) rather than as `#UD`. Set together with `OSFXSR`; a kernel that sets one without
//!   the other reports arithmetic as a bad opcode.
//! - **`CR0.EM` and `CR0.MP`** are the 80387 emulation pair. `EM` must be **clear** or SSE raises
//!   `#UD` instead of the `#NM` this module wants, and `MP` is set so that `wait`/`fwait` respects
//!   `TS` like everything else.
//! - **`CR0.TS`** is the one that matters: set, the next FP or SSE instruction raises `#NM` (vector
//!   7, "device not available"). It is the enable bit here, written per thread.
//!
//! **`TS` is also the mechanism behind LazyFP, CVE-2018-3665**, and this module is why
//! [`crate::fp`]'s header says what it says. The vulnerability was not `TS` itself: it was using
//! `TS` to defer the *restore*, so that one thread ran with another thread's `xmm` registers still
//! in the file and the `#NM` as the only thing between them. Speculative execution reads them
//! before the fault is delivered. Here `TS` answers "has this thread ever wanted the unit", which is
//! a performance question, and the registers are always either the running thread's or
//! [`FpState::INITIAL`], so there is nothing behind the trap to speculate at.
//!
//! # BUGS
//!
//! - **`fxsave`, not `xsave`.** This saves the 512-byte legacy area: x87, `MXCSR`, and
//!   `xmm0`-`xmm15`. A thread that used **AVX** would have `ymm` upper halves that this does not
//!   move, and AVX-512's `zmm` likewise. That is safe today only because `CR4.OSXSAVE` is clear, so
//!   `xgetbv` and every VEX-encoded instruction raise `#UD` and no thread can get into that state;
//!   the day this kernel enables `XCR0`, this file has to grow an `xsave` path with it, and that
//!   coupling is stated here because nothing enforces it.
//! - **`xsave`'s init optimisation is left on the table.** `xsave` records in `XSTATE_BV` which
//!   components are actually in use and skips the rest, which would make the save of a thread that
//!   touched one `xmm` register much cheaper than 512 bytes. `fxsave` has no such thing. Worth
//!   revisiting with the `XCR0` work above rather than before it.

use core::arch::asm;

/// **How many vector registers this architecture saves**: `xmm0`-`xmm15`, sixteen rather than the
/// other two ISAs' thirty-two. See the aarch64 twin.
#[cfg(test)]
pub const REGISTERS: usize = 16;

/// **A thread's FP/SSE register file**, plus the flag that says whether any of it is worth moving.
///
/// `#[repr(C, align(16))]` is a contract with `fp.s` and with the hardware: `fxsave` faults on a
/// destination that is not 16-byte aligned, and the area sits at offset 16.
#[repr(C, align(16))]
#[derive(Clone, Copy)]
pub struct FpState {
    /// Nonzero once this thread has executed an FP or SSE instruction. The save and the restore
    /// are both conditioned on it.
    live: u64,
    /// Padding, so the `fxsave` area lands at offset 16 and 16-byte aligned. Never read.
    _pad: u64,
    /// The `FXSAVE` area: x87 control and status, `MXCSR`, `ST0`-`ST7`, `xmm0`-`xmm15`. 512 bytes,
    /// laid out by the CPU rather than by this file.
    area: [u8; 512],
}

const _: () = assert!(size_of::<FpState>() == 528);
const _: () = assert!(core::mem::offset_of!(FpState, area) == 16);

/// `MXCSR` in the `FXSAVE` area.
const MXCSR: usize = 24;
/// `xmm0` in the `FXSAVE` area; the sixteen registers follow at 16 bytes each.
const XMM0: usize = 160;

impl FpState {
    /// **The state a thread starts with, and the state the registers are scrubbed to.**
    ///
    /// Not all zeros, and this is the one architecture where that matters. The x87 control word
    /// (offset 0) is `0x037f`, the value `finit` installs: extended precision, round to nearest,
    /// every exception masked. `MXCSR` (offset 24) is `0x1f80`, likewise all six SIMD exceptions
    /// masked and round to nearest. A zeroed `MXCSR` would unmask every exception, so the first
    /// underflow in a scrubbed thread would be a `#XM` nobody asked for.
    pub const INITIAL: Self = {
        let mut area = [0u8; 512];
        // x87 control word, little endian.
        area[0] = 0x7f;
        area[1] = 0x03;
        // MXCSR, little endian.
        area[MXCSR] = 0x80;
        area[MXCSR + 1] = 0x1f;
        FpState {
            live: 0,
            _pad: 0,
            area,
        }
    };

    /// A fresh thread's state.
    pub const fn new() -> Self {
        Self::INITIAL
    }

    /// Has this thread ever executed an FP or SSE instruction?
    pub fn live(&self) -> bool {
        self.live != 0
    }

    /// Record that it has. Called once, from the `#NM` the set `TS` produced.
    pub fn set_live(&mut self) {
        self.live = 1;
    }
}

impl Default for FpState {
    fn default() -> Self {
        Self::new()
    }
}

/// `CR0.MP`, the monitor-coprocessor bit: `wait`/`fwait` honours `TS`.
const CR0_MP: u64 = 1 << 1;
/// `CR0.EM`, 80387 emulation. **Must be clear**, or SSE raises `#UD` rather than `#NM`.
const CR0_EM: u64 = 1 << 2;
/// `CR0.TS`, task switched. Set means the next FP or SSE instruction raises `#NM`.
const CR0_TS: u64 = 1 << 3;
/// `CR0.NE`, native x87 exception reporting: an unmasked x87 exception arrives as `#MF` rather than
/// through the ancient external `FERR#`/IRQ 13 route, which no machine this kernel runs on wires.
const CR0_NE: u64 = 1 << 5;
/// `CR4.OSFXSR`: `fxsave`/`fxrstor` and the `xmm` registers are available.
const CR4_OSFXSR: u64 = 1 << 9;
/// `CR4.OSXMMEXCPT`: an unmasked SIMD exception arrives as `#XM` rather than `#UD`.
const CR4_OSXMMEXCPT: u64 = 1 << 10;

/// **Put this CPU into the state the rest of this module assumes**: SSE available to the kernel's
/// own save path, and the unit shut for whichever thread runs next.
///
/// Called from `sched::init` and `sched::adopt_secondary_idle` rather than from `arch::init`, for
/// the reason written at the call site: the invariant is about threads, and one architecture's boot
/// path does not pass through `arch::init` at all.
pub fn init() {
    let mut cr4 = read_cr4();
    cr4 |= CR4_OSFXSR | CR4_OSXMMEXCPT;
    write_cr4(cr4);

    let mut cr0 = read_cr0();
    cr0 &= !CR0_EM;
    cr0 |= CR0_MP | CR0_NE | CR0_TS;
    write_cr0(cr0);
}

/// Let the current CPU execute FP and SSE instructions: clear `CR0.TS`.
///
/// `clts` is one byte and exists for exactly this, which is the clearest evidence available that
/// the whole `TS` mechanism was designed for the lazy scheme this kernel deliberately does not use.
pub fn enable() {
    // SAFETY: clears one bit of `CR0`. It names no memory; the widest consequence of getting it
    // wrong is a thread executing an FP instruction, or trapping one it should not have.
    unsafe { asm!("clts", options(nomem, nostack, preserves_flags)) };
}

/// Is the FP unit open on this CPU right now?
pub fn is_enabled() -> bool {
    read_cr0() & CR0_TS == 0
}

/// Take the unit away again: set `CR0.TS`.
///
/// **The caller owes the scrub.** That sentence is this architecture's in particular: see the
/// module header on CVE-2018-3665.
pub fn disable() {
    write_cr0(read_cr0() | CR0_TS);
}

/// Copy the live register file into `state`.
///
/// # Safety
/// `state` must be a valid, writable, 16-byte-aligned `FpState`, and `CR0.TS` must be clear. The
/// only caller is [`crate::fp::hand_over`].
pub unsafe fn save(state: *mut FpState) {
    // SAFETY: forwarded. `fp_save` writes 512 bytes at `state + 16` and touches nothing else.
    unsafe { fp_save(state) }
}

/// Load `state` into the live register file.
///
/// # Safety
/// As [`save`], readable rather than writable. `state`'s `MXCSR` field must be one this CPU's
/// `MXCSR_MASK` allows, or `fxrstor` raises `#GP`; every `FpState` this kernel builds comes from
/// [`FpState::INITIAL`] or from a [`save`], and both satisfy that.
pub unsafe fn restore(state: *const FpState) {
    // SAFETY: forwarded. `fp_restore` reads 512 bytes at `state + 16` and writes only registers.
    unsafe { fp_restore(state) }
}

unsafe extern "C" {
    /// The `FXSAVE` area out to memory. See fp.s.
    fn fp_save(state: *mut FpState);
    /// The mirror. See fp.s.
    fn fp_restore(state: *const FpState);
}

fn read_cr0() -> u64 {
    let cr0: u64;
    // SAFETY: reads one control register into a local.
    unsafe { asm!("mov {}, cr0", out(reg) cr0, options(nomem, nostack, preserves_flags)) };
    cr0
}

fn write_cr0(value: u64) {
    // SAFETY: the callers set and clear only the FP-related bits above, each read back out of the
    // live register first, so nothing else in `CR0` (paging, write protect) is disturbed.
    unsafe { asm!("mov cr0, {}", in(reg) value, options(nomem, nostack, preserves_flags)) };
}

fn read_cr4() -> u64 {
    let cr4: u64;
    // SAFETY: reads one control register into a local.
    unsafe { asm!("mov {}, cr4", out(reg) cr4, options(nomem, nostack, preserves_flags)) };
    cr4
}

fn write_cr4(value: u64) {
    // SAFETY: as `write_cr0`; read-modify-write of two bits, so `PAE` and the rest are untouched.
    unsafe { asm!("mov cr4, {}", in(reg) value, options(nomem, nostack, preserves_flags)) };
}

/// **One harmless SSE instruction, to take the first-use trap on purpose.** Tests only.
///
/// `xorps xmm0, xmm0` zeroes a register the caller is about to overwrite anyway.
#[cfg(test)]
pub fn touch() {
    // SAFETY: writes one `xmm` register. Under a set `CR0.TS` this raises `#NM`, which
    // `exceptions.rs` turns into an enable, and is re-executed after it.
    unsafe {
        asm!(
            ".arch .default",
            "xorps xmm0, xmm0",
            options(nomem, nostack, preserves_flags),
        );
    }
}

#[cfg(test)]
impl FpState {
    /// Write [`crate::fp::register_pattern`] across all sixteen `xmm` registers, each one's
    /// complement in the high half so the full 128 bits are exercised.
    pub fn set_pattern(&mut self, seed: u64) {
        for index in 0..16 {
            let low = crate::fp::register_pattern(seed, index);
            let at = XMM0 + index * 16;
            self.area[at..at + 8].copy_from_slice(&low.to_le_bytes());
            self.area[at + 8..at + 16].copy_from_slice(&(!low).to_le_bytes());
        }
    }

    /// Is that pattern still there, in every `xmm` register?
    pub fn has_pattern(&self, seed: u64) -> bool {
        (0..16).all(|index| self.lane_low(index) == crate::fp::register_pattern(seed, index))
    }

    /// The low 64 bits of one `xmm` register, so a test can name the one that went wrong.
    pub fn lane_low(&self, index: usize) -> u64 {
        let at = XMM0 + index * 16;
        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(&self.area[at..at + 8]);
        u64::from_le_bytes(bytes)
    }

    /// Do the registers hold [`Self::INITIAL`]'s values, i.e. has the file been scrubbed?
    ///
    /// The `xmm` half only. The x87 stack and the tag word are also restored by the scrub, and
    /// `fxsave` writes a handful of status fields (the last instruction pointer among them) that
    /// are not the thread's data and would make a byte-for-byte comparison against `INITIAL` fail
    /// for reasons that are not leaks.
    pub fn is_scrubbed(&self) -> bool {
        (0..16).all(|index| {
            let at = XMM0 + index * 16;
            self.area[at..at + 16].iter().all(|&byte| byte == 0)
        })
    }
}
