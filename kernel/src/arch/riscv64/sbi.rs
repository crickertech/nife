//! **One SBI call**: the single `ecall` every Supervisor Binary Interface (SBI) request in the kernel
//! goes through.
//!
//! *Name provisional (a lane minted it 2026-09-25; names are calef's). `sbi` is the specification's
//! own name for the interface.*
//!
//! Until 2026-09-25 this `ecall` was written out seven times, once per request (hart start, IPI,
//! three remote fences, set-timer, system reset) plus two private `sbi_call` helpers in `isa.rs` and
//! `pmu.rs`, each passing only the registers its call reads. They agreed, and nothing made them. One
//! helper means one place that states the calling convention, and one function a proof stubs to
//! reach the logic around a firmware call (see [`super::instructions`] for the pattern and
//! notes/kernel-proofs.md for the caveat that a stub is an assumption about the firmware).

use core::arch::asm;

/// What the firmware hands back: `a0` is the error (zero is success), `a1` the value. SBI v0.2
/// onwards returns this pair from every call.
#[derive(Clone, Copy)]
pub(super) struct SbiRet {
    pub error: isize,
    pub value: usize,
}

/// One SBI call: extension `eid` in `a7`, function `fid` in `a6`, arguments in `a0` through `a5`.
///
/// Every call passes all six arguments; a function that takes fewer ignores the rest (SBI spec,
/// "Binary Encoding"), so the unused ones are zero rather than whatever the registers held. The
/// firmware preserves every register except `a0` and `a1`, which is what the operand list states;
/// `nostack` is right because M-mode runs on its own stack.
///
/// `#[inline(always)]`: the six `li`s for a call's constant arguments fold into its caller exactly as
/// they did when each site wrote its own `asm!`.
///
/// # Safety
/// An `ecall` can do anything the firmware implements, including start a hart at an arbitrary
/// address or reset the machine. The caller must name a call whose effects it accounts for, and its
/// `// SAFETY:` comment is where that accounting is written.
#[inline(always)]
pub(super) unsafe fn call(eid: usize, fid: usize, args: [usize; 6]) -> SbiRet {
    let error: isize;
    let value: usize;
    // SAFETY: the caller's contract. The operands are the SBI calling convention: a7/a6 name the
    // call, a0..a5 carry its arguments, a0/a1 carry the result and nothing else is written.
    unsafe {
        asm!(
            "ecall",
            in("a7") eid,
            in("a6") fid,
            inout("a0") args[0] => error,
            inout("a1") args[1] => value,
            in("a2") args[2],
            in("a3") args[3],
            in("a4") args[4],
            in("a5") args[5],
            options(nostack),
        );
    }
    SbiRet { error, value }
}
