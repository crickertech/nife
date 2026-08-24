//! **The interrupt controller, `x86_64`: the local APIC.**
//!
//! The third implementation of what aarch64 answers with a GIC and RISC-V with a PLIC, and the one
//! structural difference worth knowing before reading any of it: on x86 the interrupt controller is
//! a **pair**, and it is *also* the SMP bring-up mechanism.
//!
//! - The **local APIC** is per-CPU, at a fixed physical address every CPU sees its own copy of. It
//!   receives interrupts, acknowledges them (the EOI), owns a per-CPU timer, and is how one CPU
//!   sends an interrupt to another. There is no PSCI `CPU_ON` and no SBI `hart_start`: a secondary
//!   is started by sending it INIT and two STARTUP inter-processor interrupts *through this device*.
//! - The **IO APIC** takes device interrupt lines and routes them to a vector on some local APIC.
//!   Where it is and which lines it owns come from the ACPI MADT, which `machine_discovery::acpi`
//!   now reads.
//!
//! # What is built here and what is not
//!
//! Built: the local APIC (enable, EOI, the timer's LVT), masking the legacy 8259 PICs, and the
//! calibration counter `timer.rs` needs. That is enough for a real hardware interrupt to be
//! delivered, taken and acknowledged, which is what the boot tour proves.
//!
//! Not built: the **IO APIC**, so no *device* interrupt is routed yet, and IPIs, so no secondary CPU
//! can be started. Both are `unimplemented!()` below and each says which.
//!
//! # The two obligations the tables state and code must honour
//!
//! **The 8259s are still there.** The MADT's `PCAT_COMPAT` flag says so on every PC, and QEMU's
//! `q35` sets it. They are wired, they will raise interrupts on vectors 8..15 (their power-on
//! default, which overlaps the CPU's own exception vectors), and nothing drives them. [`init`] masks
//! both before the local APIC is enabled, in that order, because the reverse leaves a window where
//! an unowned interrupt can arrive at a live IDT.
//!
//! **A legacy IRQ number is not an IO APIC input.** The MADT's interrupt source overrides rewire
//! them, and on essentially every PC the timer's IRQ 0 arrives as global system interrupt 2. Nothing
//! here reads a legacy number, and the IO APIC work must.

use core::sync::atomic::{AtomicU64, Ordering};

use super::mmu::device_va;
use super::port::{in8, out8};

/// The local APIC's registers, as offsets from its base. 32-bit, and **all of them must be accessed
/// as aligned 32-bit words**: an 8- or 16-bit access to an APIC register is undefined.
mod reg {
    /// This CPU's local APIC id, in bits 31:24.
    pub const ID: u64 = 0x020;
    /// Version, and in bits 23:16 the number of LVT entries minus one.
    pub const VERSION: u64 = 0x030;
    /// Task Priority. Zero means "accept every vector"; anything else silently drops interrupts
    /// below that priority class, which is a very quiet way to lose them.
    pub const TPR: u64 = 0x080;
    /// End Of Interrupt. Written (with zero) to acknowledge; until then the local APIC will not
    /// deliver another interrupt of the same or lower priority.
    pub const EOI: u64 = 0x0b0;
    /// Spurious Interrupt Vector. Bit 8 is the **software enable**, and the low eight bits are the
    /// vector a spurious interrupt arrives on.
    pub const SPURIOUS: u64 = 0x0f0;
    /// Local Vector Table entry for the timer: the vector, the mask bit, and the mode.
    pub const LVT_TIMER: u64 = 0x320;
    /// What the timer counts down from.
    pub const TIMER_INITIAL: u64 = 0x380;
    /// What it is at now.
    pub const TIMER_CURRENT: u64 = 0x390;
    /// How much the timer divides the bus clock by.
    pub const TIMER_DIVIDE: u64 = 0x3e0;
}

/// Bit 8 of the spurious-interrupt register: the local APIC's software enable.
const SPURIOUS_ENABLE: u32 = 1 << 8;

/// **The vector a spurious interrupt arrives on.** 0xff by convention, and the low four bits used to
/// be required to be one on older parts, which is why every kernel picks a vector ending in 0xf.
pub const SPURIOUS_VECTOR: u8 = 0xff;

/// **The vector the local APIC timer raises.** 0x20, the first vector after the 32 the architecture
/// reserves for exceptions. Not a hardware fact: it is our choice, written into the LVT.
pub const TIMER_VECTOR: u8 = 0x20;

/// LVT bit 16: masked. Set on every entry at reset, which is why an unmasked entry is a deliberate
/// act.
const LVT_MASKED: u32 = 1 << 16;
/// LVT timer mode bits 18:17 = 01: periodic. The timer reloads its initial count and fires again,
/// rather than firing once.
const LVT_TIMER_PERIODIC: u32 = 1 << 17;

/// Timer divide configuration for "divide by 16". The encoding is not the number: bits 3, 1 and 0
/// form the value with **bit 2 skipped**, which is one of the more gratuitous layouts in the
/// architecture, so it is written as a constant rather than computed.
const TIMER_DIVIDE_16: u32 = 0b0011;

/// The 8259 PICs' command and data ports.
const PIC1_COMMAND: u16 = 0x20;
const PIC1_DATA: u16 = 0x21;
const PIC2_COMMAND: u16 = 0xa0;
const PIC2_DATA: u16 = 0xa1;

/// Where this CPU's local APIC is, as a virtual address. Zero until [`init`] runs.
///
/// A static rather than a parameter because every accessor below needs it and the alternative is
/// threading it through the whole interrupt path. It is written once during single-threaded boot.
static LOCAL_APIC: AtomicU64 = AtomicU64::new(0);

/// Read a local APIC register.
fn read(offset: u64) -> u32 {
    let base = LOCAL_APIC.load(Ordering::Relaxed);
    debug_assert!(base != 0, "the local APIC has not been located");
    // SAFETY: an aligned 32-bit MMIO read of a register the APIC defines, at the address the ACPI
    // MADT stated, reached through the identity map (see `mmu::device_va`).
    unsafe { core::ptr::read_volatile((base + offset) as *const u32) }
}

/// Write a local APIC register.
fn write(offset: u64, value: u32) {
    let base = LOCAL_APIC.load(Ordering::Relaxed);
    debug_assert!(base != 0, "the local APIC has not been located");
    // SAFETY: as `read`.
    unsafe { core::ptr::write_volatile((base + offset) as *mut u32, value) };
}

/// **Mask every line on both 8259 PICs.**
///
/// Their power-on vector base overlaps the CPU's own exception vectors (a PIC IRQ 0 arrives as
/// vector 8, which is the double fault), so an interrupt from one of these is not merely unowned, it
/// is *misread as a fault the kernel takes seriously*. Masking is enough because nothing here wants
/// to use them; a kernel that did would remap them to 0x20..0x30 first.
///
/// Done before the local APIC is enabled, deliberately: the reverse order leaves a window in which
/// the IDT is live and these are not masked.
fn mask_the_8259s() {
    // SAFETY: writing all-ones to both PICs' interrupt mask registers. The only effect is that they
    // stop raising interrupts, which is the entire intent.
    unsafe {
        out8(PIC1_DATA, 0xff);
        out8(PIC2_DATA, 0xff);
        // A read of the command port, purely to give the (emulated or real) chip a bus cycle to
        // settle. Harmless, and the traditional way to space two 8259 writes.
        let _ = in8(PIC1_COMMAND);
        let _ = in8(PIC2_COMMAND);
    }
}

/// Bring up this CPU's local APIC at physical address `base`.
///
/// `base` comes from the ACPI MADT rather than from a constant, which is the point: the address is
/// architecturally relocatable through `IA32_APIC_BASE`, and reading the table is how the kernel
/// finds out where firmware left it rather than assuming the reset default.
///
/// # Safety
/// `base` must be this machine's real local APIC address, and the identity map must still cover it.
pub unsafe fn init_local_apic(base: u64) {
    mask_the_8259s();

    LOCAL_APIC.store(device_va(base), Ordering::Relaxed);

    // Accept every priority class. The reset value is already zero on every part this has run on,
    // and writing it is one instruction against a failure (interrupts silently dropped by priority)
    // that looks exactly like a controller that is not wired up.
    write(reg::TPR, 0);

    // Software-enable, with a spurious vector the IDT has a gate for. Until this bit is set the
    // local APIC delivers nothing at all, which is the state the CPU boots in.
    write(reg::SPURIOUS, SPURIOUS_ENABLE | SPURIOUS_VECTOR as u32);
}

/// Acknowledge the interrupt being handled. **Every handler must, and the failure mode is a hang
/// rather than an error**: until the EOI is written the local APIC will not deliver another
/// interrupt of the same or lower priority, so a missed EOI means the timer ticks exactly once.
pub fn end_of_interrupt() {
    write(reg::EOI, 0);
}

/// This CPU's local APIC id, which is its hardware name and need not be its logical cpu id.
pub fn local_apic_id() -> u8 {
    (read(reg::ID) >> 24) as u8
}

/// The local APIC's version register's low byte. Printed at boot; an integrated APIC reports 0x10 or
/// higher, an external 82489DX reports below 0x10 and would mean a machine older than this kernel
/// supports.
pub fn local_apic_version() -> u8 {
    read(reg::VERSION) as u8
}

/// Is the local APIC up? False until [`init_local_apic`] has run, which is what the timer checks
/// before trying to arm anything.
pub fn local_apic_ready() -> bool {
    LOCAL_APIC.load(Ordering::Relaxed) != 0
}

/// Start the timer counting down from `count`, **masked**, so it can be measured without delivering
/// anything. This is what calibration uses: the timer is a free-running counter until something
/// unmasks it.
pub fn start_timer_for_calibration(count: u32) {
    write(reg::TIMER_DIVIDE, TIMER_DIVIDE_16);
    write(reg::LVT_TIMER, LVT_MASKED | TIMER_VECTOR as u32);
    write(reg::TIMER_INITIAL, count);
}

/// What the timer's countdown is at now.
pub fn timer_current_count() -> u32 {
    read(reg::TIMER_CURRENT)
}

/// Arm the timer to fire every `count` of its own ticks, on [`TIMER_VECTOR`], forever.
pub fn arm_periodic_timer(count: u32) {
    write(reg::TIMER_DIVIDE, TIMER_DIVIDE_16);
    write(reg::TIMER_INITIAL, count);
    // Unmasked and periodic. Written last, so the count is already loaded when delivery begins.
    write(reg::LVT_TIMER, LVT_TIMER_PERIODIC | TIMER_VECTOR as u32);
}

/// Stop the timer delivering.
#[allow(dead_code)]
pub fn mask_timer() {
    write(reg::LVT_TIMER, LVT_MASKED | TIMER_VECTOR as u32);
}

// ---------------------------------------------------------------------------------------------
// The portable arch contract's names, which the IO APIC and IPIs would implement. Neither is built.
// ---------------------------------------------------------------------------------------------

/// Bring up the interrupt controller. The local APIC's own bring-up is [`init_local_apic`], which
/// takes an address the ACPI tables supplied; this name is the arch contract's no-argument one and
/// belongs to the IO APIC, which is not built.
#[allow(dead_code)]
pub fn init() {
    unimplemented!("x86_64 irq::init: the IO APIC is not built (milestone 161)")
}

/// Bring up this CPU's local interrupt interface. See [`init_local_apic`], which is the same
/// operation with the address the MADT gave rather than none.
#[allow(dead_code)]
pub fn init_this_cpu() {
    unimplemented!("x86_64 irq::init_this_cpu: see init_local_apic (milestone 161)")
}

/// Unmask interrupt `intid` at the controller.
///
/// # BUGS
/// **Unimplemented.** This is the IO APIC's redirection table, and it needs the MADT's interrupt
/// source overrides applied first, because `intid` here is a legacy IRQ number and the IO APIC input
/// it corresponds to is very often a different number. See this module's header.
#[allow(dead_code)]
pub fn enable(intid: u32) {
    let _ = intid;
    unimplemented!("x86_64 irq::enable: the IO APIC is not built (milestone 161)")
}

/// Send a reschedule inter-processor interrupt to `target_cpu`.
///
/// # BUGS
/// **Unimplemented**, and blocked on nothing but SMP existing: the mechanism is a write to the local
/// APIC's Interrupt Command Register, which this module already has the base address for.
#[allow(dead_code)]
pub fn send_reschedule(target_cpu: usize) {
    let _ = target_cpu;
    unimplemented!("x86_64 irq::send_reschedule: no secondary CPU exists yet (milestone 161)")
}
