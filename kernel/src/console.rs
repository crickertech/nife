//! The kernel console, and `print!` / `println!`.
//!
//! There is deliberately no global mutable state here. A `Pl011` handle is just a
//! pointer, so we mint a fresh one per call rather than keeping a
//! `static mut CONSOLE`. The real state lives in the hardware, not in our memory.

use core::fmt::Write;

// The early console UART, selected by architecture at compile time. Two concrete drivers, not a
// trait: there are exactly two, they are chosen here and nowhere else, and a trait would be an
// abstraction ahead of a third requirement (DECISIONS.md, rules 2/3). aarch64's `virt` has a PL011;
// RISC-V's has an NS16550. Both expose `new`/`init`/`impl Write`, so the console code below names
// neither. See notes/riscv-port.md.
#[cfg(target_arch = "riscv64")]
use crate::drivers::ns16550::Ns16550 as ConsoleUart;
#[cfg(target_arch = "aarch64")]
use crate::drivers::pl011::Pl011 as ConsoleUart;
// x86_64's `q35` (and milestone 87's OptiPlex, through its Dell C4PDJ module) has the SAME NS16550
// the RISC-V board does, at an I/O PORT rather than a memory address. That is a difference in how
// the eight registers are reached and in nothing else, so it is the same driver with a different
// register space rather than a third one; see drivers/ns16550.rs and arch/x86_64/port.rs.
#[cfg(target_arch = "x86_64")]
type ConsoleUart = crate::drivers::ns16550::Ns16550<crate::arch::PortIo>;
use machine_discovery::framebuffer::Framebuffer;
use screen_console::ScreenConsole;

use crate::sync::{IrqSafeMutex, rank};

/// The console UART's **physical** address on QEMU's `virt` machine.
#[cfg(target_arch = "aarch64")]
const UART_PHYS: u64 = 0x0900_0000; // PL011
#[cfg(target_arch = "riscv64")]
const UART_PHYS: u64 = 0x1000_0000; // NS16550
/// **A port number, not a physical address**, which is why it does not go through `phys_to_virt`
/// below: x86's I/O space has no page tables in front of it and nothing to translate. COM1 has been
/// at 0x3f8 since the PC/AT and is there on QEMU's `q35`.
#[cfg(target_arch = "x86_64")]
const UART_PORT: usize = crate::arch::mmu::COM1_PORT;

/// The console UART's node name in the device tree, pinned beside `UART_PHYS` and carrying its
/// address in the unit suffix. Same hardcode-with-a-witness stance as the address itself: on
/// riscv, both QEMU `virt` and the JH7110 spell UART0 exactly this way, and the fixture test
/// (`crates/machine_discovery/tests/riscv64_jh7110.rs`) is the witness; on aarch64 it is QEMU `virt`'s PL011
/// node, witnessed by `crates/dtb/tests/qemu_aarch64_virt.rs`. Two readers: this file's
/// `configure_from_dtb` (riscv, the register shape) and `memory::init` (both, the interrupt
/// line), which is why it is `pub(crate)` rather than local to either.
#[cfg(target_arch = "aarch64")]
pub(crate) const UART_NODE: &[u8] = b"pl011@9000000";
#[cfg(target_arch = "riscv64")]
pub(crate) const UART_NODE: &[u8] = b"serial@10000000";
/// x86 has no device tree, so there is no node to name. The empty slice keeps the constant's shape
/// across the three architectures for the portable readers (`memory::init`); nothing on x86 looks
/// the console up by name, because ACPI does not describe a legacy COM port that way.
#[cfg(target_arch = "x86_64")]
pub(crate) const UART_NODE: &[u8] = b"";

/// The console UART's address, as the kernel sees it.
///
/// **Hardcoded on purpose, and it should stay that way.** Not a TODO.
///
/// Everywhere else we insist the machine tell us what it is rather than guessing
/// (notes/device-tree.md). The console is the one place we can't, and the reason is a
/// chicken-and-egg: the device tree parser is the code most likely to have a bug, and
/// `println!` is how you would debug it. So the console has to come up *before* the
/// device tree is parsed, which means the console cannot depend on it.
///
/// A new board needs a different constant here, and that is the correct shape: a per-board
/// early-console address, chosen at compile time, that gets us far enough to read the tree that
/// tells us everything else.
///
/// **This is a virtual address.** It lives in the kernel's direct map at `pa | KERNEL_VA_BASE`; on
/// aarch64 boot.s maps it before any Rust runs and `mmu::init` preserves it. On RISC-V the kernel
/// currently runs bare (identity map), so `phys_to_virt` is the identity until the Sv39 step.
#[cfg(not(target_arch = "x86_64"))]
const UART_BASE: usize = crate::arch::mmu::phys_to_virt(UART_PHYS) as usize;
/// `x86_64`'s console base is a port number and needs no translation; see [`UART_PORT`].
#[cfg(target_arch = "x86_64")]
const UART_BASE: usize = UART_PORT;

/// **The screen half of the console** (milestone 243), when the machine has one.
///
/// A framebuffer this kernel was told about by whoever booted it, plus the cursor walking across
/// it. Held here, inside [`KernelConsole`], rather than under a lock of its own, and that is the
/// whole reason this struct exists: **the UART lock is what serialises the screen too.** A second
/// mutex would need a rank below `rank::CONSOLE` and would be taken on every `print!` for no
/// benefit, and `force_unlock` (the panic path) would then have two locks to break instead of one.
struct Screen {
    /// The cursor and the geometry. Holds no pixels; see the `screen_console` crate.
    console: ScreenConsole,
    /// The framebuffer's address **in the kernel's direct map**, and its length in bytes.
    ///
    /// A raw address rather than a `&'static mut [u8]`, because a slice would be a live mutable
    /// borrow of device memory sitting in a `static` for the life of the machine. It is turned into
    /// a slice for the duration of one write and no longer.
    pixels: usize,
    len: usize,
}

/// The kernel console: a UART, and since milestone 243 optionally a screen.
///
/// **Both, not either.** A machine with a serial port and a monitor should say the same thing on
/// both, because the person at the bench and the gate reading the wire are looking for the same
/// line, and a console that chose one of them would be a console somebody has to configure.
struct KernelConsole {
    uart: ConsoleUart,
    screen: Option<Screen>,
}

impl core::fmt::Write for KernelConsole {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.uart.write_str(s)?;
        if let Some(screen) = self.screen.as_mut() {
            // SAFETY: `pixels` is the direct-map address of a framebuffer whose physical range was
            // validated by `machine_discovery::framebuffer::Framebuffer::span` before it was
            // recorded, and which `arch::mmu::map_everything` maps for the life of the kernel.
            // `len` is that same span. Nothing else in the kernel writes to it: the display service
            // hands *userspace* framebuffers out, and this one is not among them (see
            // `attach_screen`).
            let bytes =
                unsafe { core::slice::from_raw_parts_mut(screen.pixels as *mut u8, screen.len) };
            screen.console.write(bytes, s);
        }
        Ok(())
    }
}

/// The console.
///
/// It used to be lock-free: we minted a fresh handle per `print!`, since the handle is just a
/// pointer and the real state lives in the hardware. That was fine with no interrupts. It stops
/// being fine the moment an interrupt handler can print in the middle of somebody else's
/// `write_str`, because the UART is written **one byte at a time** and the two writers would splice
/// into each other mid-word.
///
/// SAFETY: `UART_BASE` is the documented UART address on QEMU `virt`, and nothing else in the kernel
/// touches it.
static CONSOLE: IrqSafeMutex<KernelConsole> = IrqSafeMutex::new(
    rank::CONSOLE,
    KernelConsole {
        uart: unsafe { ConsoleUart::new(UART_BASE) },
        screen: None,
    },
);

pub fn init() {
    CONSOLE.lock().uart.init();
}

/// **Start printing to a screen as well as to the UART** (milestone 243).
///
/// `found` is what the boot stage before this kernel measured and wrote into the boot handoff;
/// `virt` is where that physical framebuffer is readable, which on every caller so far is the
/// direct map. Returns the grid it came up with, in character cells, or `None` when the geometry
/// cannot hold one.
///
/// **It clears the screen**, which is the one visible side effect and is deliberate: whatever the
/// firmware left there (a vendor logo, a boot menu, the loader's own four lines) is not this
/// kernel's, and a boot tour written over a splash is a boot tour nobody can read. The cost is that
/// the loader's lines are gone by the time the kernel's first line appears, so a machine that
/// clears its screen and then says nothing has failed *between* the two, which is itself a reading.
///
/// # Safety
///
/// `virt` must name `found.span()` bytes of mapped, writable memory that is a framebuffer and not
/// anything else, for the life of the kernel. Nothing here can check that: a physical address out
/// of a boot handoff is an assertion by a previous stage, on the same footing as the memory map.
#[cfg_attr(
    not(target_arch = "x86_64"),
    allow(
        dead_code,
        reason = "milestone 157's U-Boot handoff is the aarch64 caller, and it does \
                               not exist yet; the console half is deliberately arch-neutral"
    )
)]
pub unsafe fn attach_screen(found: Framebuffer, virt: u64) -> Option<(u32, u32)> {
    let mut console = ScreenConsole::new(found)?;
    let len = console.span();
    let mut guard = CONSOLE.lock();
    // SAFETY: this function's own contract, forwarded.
    let pixels = unsafe { core::slice::from_raw_parts_mut(virt as *mut u8, len) };
    console.clear(pixels);
    let size = console.size();
    guard.screen = Some(Screen {
        console,
        pixels: virt as usize,
        len,
    });
    Some(size)
}

/// **Re-shape the console UART from the device tree** (RISC-V; the VisionFive 2 prep,
/// notes/visionfive2.md). The JH7110's UART0 sits at QEMU's address with different silicon behind
/// it: a `DesignWare` 8250 with registers four bytes apart, 32-bit accesses, a 24 MHz clock, and the
/// DW busy quirk. Those four facts come from the serial node's `reg-shift`, `reg-io-width`,
/// `clock-frequency` and `compatible`, read here and handed to the driver in one piece.
///
/// **Called immediately after [`init`], before the first `println!`**, so no output is ever
/// produced with a stale stride; on the board, a byte read of LSR at unshifted offset 5 lands in
/// the IER word and the transmit poll spins forever, which is why the ordering matters. Between
/// `init` and this call the console is misconfigured *for the board* and correct for QEMU; nothing
/// prints in that window, and a panic inside it is one of the fault paths that were always dark
/// before the UART worked at all.
///
/// The **address** stays the compile-time constant above, deliberately (see `UART_BASE`): parsing
/// the tree for the address would make the console depend on the parser it exists to debug. This
/// function reads only the *shape*, fails toward the defaults on any absent property, and a tree
/// without the node (or an unreadable tree) changes nothing, so QEMU `virt` behaves as it always
/// has. An unstated `clock-frequency` (mainline JH7110 trees express the clock as a phandle)
/// yields divisor 0: leave the divisor and line controls exactly as U-Boot programmed them, which
/// is the correct move on a board whose firmware just printed a prompt at 115200.
#[cfg(target_arch = "riscv64")]
pub fn configure_from_dtb() {
    use crate::drivers::ns16550::Shape;

    // Nothing has parsed the tree yet on this boot; the magic check inside `device_tree` is what
    // makes a garbage pointer survivable, and on failure the console simply keeps its defaults.
    let Ok(dt) = crate::device_tree() else {
        return;
    };

    let u32_prop = |name: &[u8]| -> Option<u32> {
        match dt.node_prop(UART_NODE, name) {
            Ok(Some(bytes)) if bytes.len() >= 4 => {
                Some(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
            }
            _ => None,
        }
    };

    let default = Shape::QEMU_VIRT;
    let shape = Shape {
        reg_shift: u32_prop(b"reg-shift").map_or(default.reg_shift, |v| v as u8),
        reg_io_width: u32_prop(b"reg-io-width").map_or(default.reg_io_width, |v| v as u8),
        // A stated clock is the only licence to reprogram the divisor; see the field's doc.
        divisor: u32_prop(b"clock-frequency").map_or(0, Shape::divisor_for),
        dw_busy_quirk: matches!(
            dt.node_prop(UART_NODE, b"compatible"),
            Ok(Some(compat)) if compat.split(|&b| b == 0).any(|s| s == b"snps,dw-apb-uart")
        ),
    };

    CONSOLE.lock().uart.configure(shape);
}

/// Turn on the console UART's receive interrupt (RISC-V, milestone 20). After this the NS16550 raises
/// its line into the PLIC whenever a keystroke is waiting.
///
/// The kernel arms the device and then stays out of the way: the *byte* is read by the userspace
/// input driver, which `riscv_shell_boot` hands the NS16550's registers as a `DeviceFrame`
/// capability. There used to be a kernel-side `rx_read` here to drain it, from milestone 20 when
/// the input path was still in the kernel; milestone 41 deleted it, along with `Ns16550::read_byte`
/// and `LSR_DR`, after removing the crate-wide riscv `allow(dead_code)` showed they had no caller
/// in *any* configuration, `--features shell` included.
///
/// riscv-only: the aarch64 console stays polling, and its `ConsoleUart` (a PL011) has no such method.
#[cfg(target_arch = "riscv64")]
pub fn rx_enable() {
    CONSOLE.lock().uart.enable_rx_interrupt();
}

/// **Has anyone typed at the console?** (Milestone 249.) True while a byte sits unread in the
/// console UART's receive buffer; reads nothing, so the answer stays true until [`discard_rx`]
/// takes it.
///
/// This exists for exactly one caller, `soak::watch`, and it is the escape from a self-rebooting
/// soak. The board is running a workload with no console reader, so there is no shell to type a
/// command at and no input driver holding the device; what there is, on every bench run, is a
/// serial cable and a terminal. Polling LSR turns that cable into a stop button that costs one
/// register read every five seconds and needs nothing else to be wired up.
///
/// riscv64 only, and so is the feature: the reset it is the escape from is SBI's, and the PL011 the
/// aarch64 console drives has no equivalent method here.
#[cfg(all(target_arch = "riscv64", feature = "reboot_soak"))]
pub fn rx_waiting() -> bool {
    CONSOLE.lock().uart.rx_waiting()
}

/// Throw away whatever is already in the console UART's receive buffer, so that [`rx_waiting`]
/// answers about what arrives from now on. Called once, when a rebooting soak arms itself; see
/// `Ns16550::discard_rx` for why U-Boot's leftovers are the thing being cleared.
#[cfg(all(target_arch = "riscv64", feature = "reboot_soak"))]
pub fn discard_rx() {
    CONSOLE.lock().uart.discard_rx();
}

/// **Raise and lower the console UART's own interrupt line, for the RISC-V interrupt-delivery
/// tests.** Test builds only.
///
/// aarch64 raises a test interrupt with a GIC SGI, which needs no device at all. RISC-V has no SGI:
/// the only software-raised interrupt it has is the SBI's IPI, which arrives as a *software*
/// interrupt (`scause` = 1) down a different arm of the trap dispatcher than a device's, so it would
/// not exercise the PLIC claim/route/complete path at all. What it has instead is a device whose
/// line software can assert without any transfer or external stimulus: setting the 16550's
/// transmit-empty interrupt enable makes it interrupt at once, because the transmitter of a polling
/// console is always empty. Two register writes up, one down.
///
/// The console is the right device for it precisely because the kernel does not drive it by
/// interrupt: transmit is polled (`write_byte` spins on `LSR`), so an asserted transmit line has no
/// other consumer to disturb, and lowering it restores exactly the state `init` left.
///
/// See `kernel::sched::tests` and notes/interrupts.md.
#[cfg(all(test, target_arch = "riscv64"))]
pub fn raise_uart_interrupt() {
    CONSOLE.lock().uart.enable_tx_interrupt();
}

/// Quiet the line [`raise_uart_interrupt`] raised. Test builds only.
#[cfg(all(test, target_arch = "riscv64"))]
pub fn quiet_uart_interrupt() {
    CONSOLE.lock().uart.disable_interrupts();
}

/// Break the console lock open. **Panic and fault paths only.**
///
/// # Safety
///
/// If we fault in the middle of a `println!`, the fault handler's own attempt to print
/// would take this lock again and hang, and we would lose the only message that mattered.
/// So the panic path breaks the lock first. Output may be spliced. That is a fine price
/// for getting the message out at all.
///
/// See sync.rs, and DECISIONS.md §9.
pub unsafe fn force_unlock() {
    // SAFETY: this function's own `# Safety` contract is exactly the one this call needs; it forwards, it does not weaken.
    unsafe { CONSOLE.force_unlock() }
}

/// **Bytes handed to the console transmitter since boot** (first-silicon diagnostics, 2026-08-15).
///
/// Counted after each `write_str` completes, so a count here means the driver's bounded world has
/// already accepted the bytes: `write_byte`'s THRE poll is unbounded, so a wedged transmitter
/// shows as a *hanging* print, never as a count with no wire bytes behind it. The point is the
/// contrapositive at a bench: VisionFive 2 boots 7 through 9 ended with kernel state proving the
/// tour's printing steps ran while the serial record showed none of their lines, and nothing
/// could say whether the bytes were ever emitted. The riscv diag line prints this counter, so
/// boot 11's dumps carry "how much output has the kernel pushed" beside "what the wire showed".
static TX_BYTES: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(0);

/// The [`TX_BYTES`] counter. Diagnostic; racy reads are fine.
#[cfg_attr(target_arch = "aarch64", allow(dead_code))] // the riscv diag line is the reader today
pub fn tx_bytes() -> u64 {
    TX_BYTES.load(core::sync::atomic::Ordering::Relaxed)
}

/// The counting shim between `write_fmt` and the driver: forwards each fragment, then counts it.
struct CountedWrites<'a>(&'a mut KernelConsole);

impl core::fmt::Write for CountedWrites<'_> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.0.write_str(s)?;
        TX_BYTES.fetch_add(s.len() as u64, core::sync::atomic::Ordering::Relaxed);
        // What a test says about itself is evidence the harness can act on: a test that announces
        // a skip and then returns is counted as a pass, and this is where that becomes visible.
        // Test builds only. See testing::note_printed.
        #[cfg(test)]
        crate::testing::note_printed(s);
        Ok(())
    }
}

#[doc(hidden)]
pub fn _print(args: core::fmt::Arguments) {
    // Output is forward progress: it keeps the test hang-watchdog's heartbeat alive so a slow but
    // live test is not mistaken for a deadlock (test builds only; see testing::note_progress).
    #[cfg(test)]
    crate::testing::note_progress();
    // Writing to a UART cannot fail in any way we can act on, so drop the Result.
    let _ = CountedWrites(&mut CONSOLE.lock()).write_fmt(args);
}

/// Write formatted text to the kernel console, `core::fmt` syntax, no newline.
///
/// Takes the console lock for the duration of one call, so a message cannot be interleaved with
/// another core's. It cannot fail: a UART write has no error a kernel could act on, so the `Result`
/// is dropped in `console::_print` rather than propagated to every call site.
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::console::_print(format_args!($($arg)*)));
}

/// [`print!`] with a trailing newline. The no-argument form writes just the newline.
#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

#[cfg(test)]
mod tests {
    //! Tests for the console.

    /// The panic path must be able to print even if the console lock is held.
    ///
    /// Otherwise a fault taken in the middle of a `println!` deadlocks in the fault
    /// handler, and we lose the one message that mattered.
    #[test_case]
    fn console_lock_can_be_busted() {
        // SAFETY: this is exactly the panic path's move, done deliberately.
        unsafe { crate::console::force_unlock() };

        // If force_unlock left the lock in a bad state, this hangs and the test times out
        // rather than failing, which is its own kind of signal.
        crate::println!("    (console still works after force_unlock)");
    }
}
