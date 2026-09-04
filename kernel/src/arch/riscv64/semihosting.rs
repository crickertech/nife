//! **Asking the host to terminate, RISC-V.** The test harness's exit path.
//!
//! The module keeps the aarch64 name (`arch::semihosting::exit`) because it is the arch contract the
//! test harness calls, but the mechanism is different: not ARM semihosting, but QEMU virt's
//! `sifive_test` finisher, an MMIO word at `0x10_0000` that exits QEMU. Writing `PASS` exits 0;
//! writing `FAIL | (code << 16)` exits non-zero. That maps exactly onto our success/failure codes,
//! so the harness works unchanged. The finisher is reached through the kernel's direct map (paging is
//! on by the time any test runs; `mmu::map_everything` maps this page device-typed). See
//! notes/riscv-port.md.
//!
//! Under the `board` feature (milestone 16a), the target is the VisionFive 2 rather than QEMU `virt`,
//! and the finisher does not exist: a store to `0x10_0000` is a bus error. The board exit prints a
//! fixed UART marker line (`NIFE-TEST-EXIT: PASS` / `NIFE-TEST-EXIT: FAIL <code>`) so a harness on
//! the serial line can read the verdict, then calls SBI SRST shutdown so the run terminates cleanly.
//! See notes/visionfive2.md, "The test suite where semihosting allows".
//!
//! **This file is also where the SBI SRST *reboot* lives** (milestone 249), which is a stretch of
//! the module's name and is here anyway. SRST is one extension with one function and a type
//! argument; shutdown and cold reboot differ by that argument and by nothing else, so splitting
//! them across two files would mean two copies of the extension id, the function id and the calling
//! convention, to describe one `ecall`. [`reboot`] is not an exit path and does not pretend to be:
//! it returns to its caller when the firmware refuses.

// Everything below is reachable only from the test harness and the test-mode panic arm, both
// `cfg(test)`, exactly as on aarch64. `not(test)` rather than a blanket allow, so the test build
// still holds this file to the dead-code gate.

use core::arch::asm;

/// The harness's success code (a passing exit).
#[cfg_attr(not(test), allow(dead_code))]
pub const EXIT_SUCCESS: u32 = 0;
/// The harness's failure code (any non-zero exit).
#[cfg_attr(not(test), allow(dead_code))]
pub const EXIT_FAILURE: u32 = 1;

// ---- QEMU `virt` exit: the `sifive_test` finisher ----

/// The `sifive_test` finisher's **physical** address on QEMU's `virt` machine. Reached through the
/// direct map at run time, since paging is on (bare-mode identity is long gone).
#[cfg_attr(all(not(test), not(feature = "board")), allow(dead_code))]
#[cfg(not(feature = "board"))]
const SIFIVE_TEST_PHYS: u64 = 0x10_0000;
/// Write this to exit QEMU with status 0.
#[cfg_attr(all(not(test), not(feature = "board")), allow(dead_code))]
#[cfg(not(feature = "board"))]
const FINISHER_PASS: u32 = 0x5555;
/// Base value for a failing exit; the caller's code is packed into the high half.
#[cfg_attr(all(not(test), not(feature = "board")), allow(dead_code))]
#[cfg(not(feature = "board"))]
const FINISHER_FAIL: u32 = 0x3333;

/// Terminate the QEMU guest with `code` (0 = success). Drives the `sifive_test` finisher: `PASS` for
/// a clean exit, `FAIL` with the code in the high bits otherwise.
#[cfg_attr(all(not(test), not(feature = "board")), allow(dead_code))]
#[cfg(not(feature = "board"))]
pub fn exit(code: u32) -> ! {
    let word = if code == 0 {
        FINISHER_PASS
    } else {
        FINISHER_FAIL | (code << 16)
    };
    // SAFETY: an MMIO store to the finisher register through the direct map (mapped device-typed by
    // mmu::map_everything). The write exits QEMU, so nothing after it runs.
    let reg = super::mmu::phys_to_virt(SIFIVE_TEST_PHYS) as *mut u32;
    // SAFETY: as the comment above says: an MMIO store to the finisher register through the direct map, which `mmu::map_everything` mapped device-typed. The write exits QEMU, so nothing after it runs.
    unsafe { core::ptr::write_volatile(reg, word) };

    // The finisher terminates the guest; if it somehow does not, stop rather than run on.
    loop {
        // SAFETY: wait-for-interrupt is always safe.
        unsafe { asm!("wfi", options(nomem, nostack)) };
    }
}

// ---- Board exit: UART marker + SBI SRST shutdown ----

/// The SBI SRST extension id, "SRST" in ASCII.
#[cfg(feature = "board")]
const SBI_SRST_EID: usize = 0x5352_5354;
/// The SBI SRST `system_reset` function id.
#[cfg(feature = "board")]
const SBI_SYSTEM_RESET_FID: usize = 0;
/// SRST reset type: shutdown (power off the board).
#[cfg(feature = "board")]
const SRST_RESET_TYPE_SHUTDOWN: usize = 0;
/// SRST reset type: **cold reboot** (milestone 249). SBI v0.3's SRST extension defines 0 as
/// shutdown, 1 as cold reboot and 2 as warm reboot, and an implementation is permitted to support
/// any subset: an unsupported type comes back as `SBI_ERR_NOT_SUPPORTED` rather than as a reset
/// that quietly does the wrong thing.
///
/// **Whether radon's OpenSBI implements this one is unverified.** The shutdown path above is in
/// use, so the extension exists and the `ecall` reaches it; that says nothing about which types
/// this vendor firmware build accepts, and nobody in this tree has asked it. [`reboot`] returns
/// the firmware's own error code so the answer is read off a console rather than assumed. See
/// notes/soak.md, "Verifying the reset before anything is left unattended".
#[cfg(feature = "reboot_soak")]
const SRST_RESET_TYPE_COLD_REBOOT: usize = 1;
/// SRST reset reason: none (no additional reason specified).
#[cfg(feature = "board")]
const SRST_RESET_REASON_NONE: usize = 0;

/// Call SBI SRST `system_reset` with `reset_type`. An `ecall` from S-mode traps to OpenSBI in
/// M-mode.
///
/// **It returns only on failure**, which is the SRST contract: a successful reset never comes back,
/// so a return is the firmware saying no. The value is `sbiret.error` out of `a0`, and the one a
/// caller should expect is `SBI_ERR_NOT_SUPPORTED` (-2) from an implementation that does not do the
/// type asked for.
///
/// `a1` is written by the call as `sbiret.value` and is discarded, which is why it is an
/// `inlateout` rather than an `in`: an `in`-only register the callee writes is exactly the shape
/// that produces a miscompile nobody sees until the value is used.
#[cfg(feature = "board")]
fn sbi_system_reset(reset_type: usize) -> isize {
    let error: isize;
    // SAFETY: an SBI call. a7 = extension id (SRST), a6 = function id (system_reset), a0 = reset
    // type, a1 = reset reason (none). The firmware returns sbiret in a0 (the error, taken) and a1
    // (the value, discarded); nothing else is touched.
    unsafe {
        asm!(
            "ecall",
            in("a7") SBI_SRST_EID,
            in("a6") SBI_SYSTEM_RESET_FID,
            inlateout("a0") reset_type => error,
            inlateout("a1") SRST_RESET_REASON_NONE => _,
            options(nostack),
        );
    }
    error
}

/// **Ask the firmware for a cold reboot** (milestone 249), and return only if it refuses.
///
/// This is one constant away from the shutdown the board exit already performs, and the whole of
/// what makes a soak able to draw the boot lottery more than once an evening. The caller is
/// `soak::watch`, behind `--features reboot_soak`, and it is reached only after the escape in
/// `console::rx_waiting` has been checked twice.
///
/// The return value is the firmware's `sbiret.error`. Nothing here interprets it or prints it: the
/// marker vocabulary a console log is read with lives in `kernel/src/soak.rs` beside every other
/// `soak-reboot:` line, so this stays a call to the firmware and the caller stays the only place
/// that speaks to a reader.
///
/// Name provisional (milestone 249): calef names public items.
#[cfg(feature = "reboot_soak")]
pub fn reboot() -> isize {
    sbi_system_reset(SRST_RESET_TYPE_COLD_REBOOT)
}

/// Terminate the board run with `code` (0 = success). Prints a fixed UART marker line so a harness
/// on the serial line can read the verdict, then calls SBI SRST to shut the board down. The
/// `sifive_test` finisher does not exist on the VisionFive 2, so this is the silicon exit path.
#[cfg_attr(all(not(test), feature = "board"), allow(dead_code))]
#[cfg(feature = "board")]
pub fn exit(code: u32) -> ! {
    // Print the marker before calling SBI: once the firmware begins shutdown the UART stops
    // draining, so anything printed after the ecall may never reach the wire.
    if code == 0 {
        crate::println!("NIFE-TEST-EXIT: PASS");
    } else {
        crate::println!("NIFE-TEST-EXIT: FAIL {}", code);
    }

    sbi_system_reset(SRST_RESET_TYPE_SHUTDOWN);

    // SBI SRST should not return. If it does, stop rather than run on.
    loop {
        // SAFETY: wait-for-interrupt is always safe.
        unsafe { asm!("wfi", options(nomem, nostack)) };
    }
}
