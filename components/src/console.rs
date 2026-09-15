//! The console server: a whole program in one job.
//!
//! Milestone 19f.3, the second program lifted out of `hello` into its own binary (after the `least_authority_demo`).
//! It owns a device mapping of the PL011 UART and one request/reply channel. A client writes text
//! into a page it shares with the server, SENDs the length on the request endpoint, and the server
//! copies that many bytes to the UART one at a time and ACKs on the reply endpoint. The kernel never
//! touches the bytes: a driver at EL0, confined by the same capability walls as any workload. A bad
//! length faults the *server* (a read out of its own mapping), not the kernel.
//!
//! Its whole authority is three things the progenitor hands it: the request endpoint (slot 0, RECV), the reply
//! endpoint (slot 1, SEND), and the UART registers, plus the shared page mapped read-only. It has no
//! role selector; a standalone binary needs none. It shares the `user` package's `link.ld` but not a
//! line of hello's code.
//!
//! The syscall runtime (`send`/`recv`) comes from the shared `user_mode_runtime` crate (19f.6).
//!
//! Name: ratified 2026-07-30 (calef, DECISIONS §39), among the names recorded there as always
//! right.

#![no_std]
// Program entry points, not the crates/ library surface milestone 68's ratchet tracks
// (DECISIONS §107): each `[[bin]]` is its own crate root with one `_start`, and 58 of them
// documenting an OS-facing ABI entry point is not what the lint is for.
#![allow(missing_docs)]
#![no_main]

use user_mode_runtime::{recv, send};

/// The PL011's register block, migrated onto `tock_registers` (milestone 139 round 5): every
/// offset checked at compile time instead of asserted by a hand-written comment, matching
/// `kernel/src/drivers/pl011.rs`'s own idiom for the identical hardware. Only the two registers
/// `uart_put` needs (DR, FR): the kernel's own driver configures the device at boot, so this
/// program only ever transmits. The RISC-V half below stays on plain volatile access, for the
/// reason `kernel/src/drivers/ns16550.rs`'s own module doc gives: the NS16550's register *stride*
/// is a runtime value (QEMU spaces its emulated registers one byte apart; the JH7110's real
/// hardware spaces them four bytes apart, `reg-shift = <2>`), which `register_structs!`'s
/// compile-time-fixed layout cannot express. The PL011 has no such knob, so it is the one half of
/// this file the macro actually fits.
#[cfg(target_arch = "aarch64")]
mod pl011 {
    use tock_registers::registers::{ReadOnly, WriteOnly};
    use tock_registers::{register_bitfields, register_structs};

    register_bitfields! {
        u32,
        /// Flag register.
        pub FR [
            /// Transmit FIFO full. Writing to DR while this is set would drop the byte.
            TXFF OFFSET(5) NUMBITS(1) [],
        ],
    }

    register_structs! {
        /// The PL011's memory-mapped register block, the same layout
        /// `kernel/src/drivers/pl011.rs` verifies at compile time for the identical hardware.
        #[allow(non_snake_case)]
        pub RegisterBlock {
            (0x00 => pub DR: WriteOnly<u32>),
            (0x04 => _reserved0),
            (0x18 => pub FR: ReadOnly<u32, FR::Register>),
            (0x1c => @END),
        }
    }
}

/// The request endpoint (slot 0): the server RECVs a byte count on it.
const REQUEST: u64 = 0;
/// The reply endpoint (slot 1): the server SENDs the acked count back on it.
const REPLY: u64 = 1;

/// The page the client writes text into, mapped read-only in the server's space. Must match what
/// the client (the progenitor, or the shell) maps and what the progenitor hands the server (`CON_SHARED_VA`).
const SHARED_VA: u64 = 0x0060_0000;
/// How much of it there is. One frame, which is what `console_service` maps, and the bound every
/// byte count from a client is clamped to.
const PAGE: u64 = 4096;
/// The server's device mapping of the UART registers. Must match the progenitor's `CON_UART_VA`.
// Unused on x86_64: there is no page for it to name (`user::UART_PHYS` is zero, DECISIONS §121),
// so the arm below traps instead of reading. Kept unconditional rather than cfg'd out because the
// address is the wiring's fact, agreed with the progenitor, and hiding it on one architecture would make the
// two sides of that agreement look like two different constants.
#[cfg_attr(target_arch = "x86_64", allow(dead_code))]
const UART_VA: u64 = 0x0070_0000;

#[unsafe(no_mangle)]
pub extern "C" fn _start(_x0: u64, _x1: u64, _x2: u64) -> ! {
    loop {
        // Block until a client hands us a length.
        let (len, _, _) = recv(REQUEST);

        // **Clamp to the page, because the length is the CLIENT's** (milestone 43,
        // notes/shared-page-audit.md finding 3). This used to be unbounded, with a comment calling
        // an over-long count "a driver bug" and shrugging it off as a crashed process. The count
        // does not come from the driver; it arrives in the request word from whoever holds WRITE
        // on the request endpoint, so `u64::MAX` was a one-message kill of a server every other
        // client of this console shares. `line_editor` clamps at all four of its length sites; the
        // asymmetry was the finding.
        let len = len.min(PAGE);

        // Copy that many bytes from the shared page to the UART, one at a time, exactly as the
        // kernel's PL011 driver used to. The difference is only where this code runs.
        let shared = SHARED_VA as *const u8;
        for i in 0..len {
            // SAFETY: the shared page is mapped read-only in our address space and `len` is
            // clamped to it above, so every offset is inside the one frame the wiring mapped.
            let byte = unsafe { core::ptr::read_volatile(shared.add(i as usize)) };
            uart_put(byte);
        }

        // Acknowledge with the count actually printed, not the count asked for: a client that
        // asked for more than a page learns that fewer bytes went out rather than being told its
        // whole request was honoured.
        send(REPLY, len, 0, 0);
    }
}

/// Transmit one byte, spinning while the transmit path is busy. The register layout is the one
/// arch-specific thing a UART driver is *for*: aarch64's `virt` has a PL011 (32-bit registers, the
/// transmit-FIFO-full flag in the Flag Register), RISC-V's has an NS16550 (byte registers, the
/// transmit-holding-empty flag in the Line Status Register). The kernel configured the device at
/// boot; we only transmit.
#[cfg(target_arch = "aarch64")]
fn uart_put(byte: u8) {
    use pl011::{FR, RegisterBlock};
    use tock_registers::interfaces::{Readable, Writeable};

    // SAFETY: UART_VA is our device mapping of the PL011, handed to us at spawn, for the whole
    // lifetime of this process. This is the same invariant the hand-written read_volatile/
    // write_volatile calls used to assert by comment; register_structs! now checks every offset
    // above at compile time instead (kernel/src/drivers/pl011.rs's own comment: "an off-by-four
    // here is a build error rather than a mystery at runtime", true here for the identical reason).
    let regs = unsafe { &*(UART_VA as *const RegisterBlock) };
    while regs.FR.is_set(FR::TXFF) {
        core::hint::spin_loop();
    }
    regs.DR.set(byte as u32);
}

/// The NS16550 twin (RISC-V): byte registers, Transmit Holding Register Empty in the LSR.
#[cfg(target_arch = "riscv64")]
fn uart_put(byte: u8) {
    const THR: u64 = 0x00; // transmit holding register
    const LSR: u64 = 0x05; // line status register
    const LSR_THRE: u8 = 1 << 5; // transmit holding register empty
    // SAFETY: UART_VA is our device mapping of the NS16550, handed to us at spawn.
    unsafe {
        while core::ptr::read_volatile((UART_VA + LSR) as *const u8) & LSR_THRE == 0 {
            core::hint::spin_loop();
        }
        core::ptr::write_volatile((UART_VA + THR) as *mut u8, byte);
    }
}

/// **The x86 twin: COM1 by port I/O, not memory** (milestone 299, DECISIONS §121 reversed
/// 2026-09-15). The other two arms differ in a register layout; this one differs in kind. COM1's
/// 16550 lives at I/O ports `0x3F8..=0x3FF`, reached only by `in`/`out`, which ring 3 may execute
/// only for a port it holds a capability to. This process holds the `(0x3F8, 8)` port range the
/// progenitor delegated it, so the kernel's TSS I/O bitmap permits exactly these eight ports and no
/// others; an `out` to any other port would fault.
///
/// The kernel configured the device at boot (baud divisor, 8N1), so this only transmits: spin until
/// the Transmit Holding Register is empty, then write the byte. Same shape as the NS16550 arm above,
/// with `outb`/`inb` in place of the volatile MMIO because the registers are ports.
#[cfg(target_arch = "x86_64")]
fn uart_put(byte: u8) {
    use user_mode_runtime::{inb, outb};
    const THR: u16 = 0x3F8; // transmit holding register (COM1 base)
    const LSR: u16 = 0x3FD; // line status register (base + 5)
    const LSR_THRE: u8 = 1 << 5; // transmit holding register empty
    while inb(LSR) & LSR_THRE == 0 {
        core::hint::spin_loop();
    }
    outb(THR, byte);
}

user_mode_runtime::panic_handler!();
