//! **The interrupt controller, `x86_64`: the local APIC and the IO APIC.**
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
//! Built: the local APIC (enable, EOI, the timer's LVT), masking the legacy 8259 PICs, the
//! calibration counter `timer.rs` needs, and **the IO APIC's redirection table**, so a real device
//! line reaches the kernel on a vector this module chose. The boot tour proves both halves: the
//! local APIC's own timer, and the PIT arriving through the IO APIC.
//!
//! Not built: IPIs, so no secondary CPU can be started, and no interrupt is routed to any CPU but
//! the boot one.
//!
//! # The two obligations the tables state and code must honour
//!
//! **The 8259s are still there.** The MADT's `PCAT_COMPAT` flag says so on every PC, and QEMU's
//! `q35` sets it. They are wired, they will raise interrupts on vectors 8..15 (their power-on
//! default, which overlaps the CPU's own exception vectors), and nothing drives them.
//! [`init_local_apic`] masks both before the local APIC is enabled, in that order, because the
//! reverse leaves a window where an unowned interrupt can arrive at a live IDT. **They are masked
//! rather than remapped, and the IO APIC does not coexist with them being live**: the same device
//! line reaches both controllers, so an unmasked 8259 would deliver a second copy of every
//! interrupt the redirection table routes, on a vector that is an exception number.
//!
//! **A legacy IRQ number is not an IO APIC input.** The MADT's interrupt source overrides rewire
//! them, and on essentially every PC the timer's IRQ 0 arrives as global system interrupt 2,
//! because the PIT is wired to the IO APIC's pin 2 while pin 0 carries the 8259 cascade. The
//! resolution is `machine_discovery::acpi::isa_irq_table`, host-tested, and [`record_isa_routing`]
//! is what hands the answer to this module. **Nothing here may take a legacy IRQ number as a pin
//! number**, and the failure mode if it did is the quiet one: a redirection entry armed on a line
//! nothing drives, no interrupts, and no error.
//!
//! # BUGS
//!
//! - **One IO APIC, and only the boot CPU.** A machine with several IO APICs divides the global
//!   interrupt space between them by `gsi_base`; this takes the first the MADT lists and refuses a
//!   GSI outside its range rather than looking for a second. Every redirection entry is programmed
//!   in physical destination mode at the boot CPU's local APIC id, so nothing is distributed and
//!   nothing is affine to a CPU that does not exist yet.
//! - **The redirection table is written through the boot map's cacheable alias** when the tour
//!   arms a line before `mmu::init` runs, the same as the local APIC's registers already are. It
//!   works on QEMU and on real hardware (the MMIO hole is uncacheable by MTRR whatever the page
//!   tables say), but it is a mapping this code does not control. After `mmu::init` the page is
//!   device-typed by name; see `arch/x86_64/mmu.rs`.
//! - **Nothing masks a routed line on the way out.** A GSI armed by [`enable`] stays armed until
//!   something calls [`mask_gsi`]; there is no owner registry and no revocation, because there is
//!   no device driver on this architecture to own one yet.

use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};

use machine_discovery::acpi::{ISA_IRQ_COUNT, IsaIrqRouting};

use super::mmu::phys_to_virt;
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
    /// Interrupt Command Register, low word: the vector, the delivery mode, and the destination
    /// shorthand. **Writing this word is what sends the IPI**, so the high word must already be in
    /// place. See [`super::send_ipi`].
    pub const ICR_LOW: u64 = 0x300;
    /// Interrupt Command Register, high word: the destination local APIC id, in bits 31:24.
    pub const ICR_HIGH: u64 = 0x310;
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

/// **The vector a reschedule inter-processor interrupt arrives on**, and this architecture's
/// counterpart of aarch64's `sched::RESCHED_SGI` and RISC-V's SBI software interrupt.
///
/// 0x21, immediately after the timer and well below [`GSI_VECTOR_BASE`], so the local APIC's own
/// sources stay grouped in 0x20..0x2f the way [`gsi_vector`]'s comment promises.
///
/// **Name provisional** (milestone 161, roadmap item 4): calef names public items.
pub const RESCHEDULE_VECTOR: u8 = 0x21;

/// **The vector `raise_self_interrupt` uses for the scheduler's own interrupt-delivery tests.**
///
/// 0x22, in the same local-APIC band. It is a *test* fixture rather than a device line, and it has
/// to be one: see [`raise_self_interrupt`] for why x86 cannot use its console UART the way RISC-V
/// does, and why a self-IPI is the honest analog of aarch64's software-generated interrupt.
///
/// **Name provisional** (milestone 161, roadmap item 4).
pub const SELF_TEST_VECTOR: u8 = 0x22;
/// A second test vector, so two tests cannot see each other's routes (aarch64's two SGIs).
///
/// **Name provisional** (milestone 161, roadmap item 4).
pub const SELF_TEST_VECTOR_B: u8 = 0x23;

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

/// Where this CPU's local APIC is, as a virtual address. Zero until [`init_local_apic`] runs.
///
/// A static rather than a parameter because every accessor below needs it and the alternative is
/// threading it through the whole interrupt path. It is written once during single-threaded boot.
///
/// **It is a direct-map address and stays valid across `mmu::init`'s `CR3` switch**, which is why
/// this can be computed once. That is not free: `boot.s` installs the direct map at the same base
/// the fine map uses precisely so that `phys_to_virt` never changes meaning (`arch/x86_64/mmu.rs`).
/// What the fine map *does* change is the memory type, from the boot map's cacheable to device.
static LOCAL_APIC: AtomicU64 = AtomicU64::new(0);

/// The **physical** address the local APIC was found at, kept beside the virtual one so that
/// `mmu::init` can map exactly that page device-typed rather than mapping the architectural default
/// and hoping firmware left it there. Zero until [`init_local_apic`] runs.
static LOCAL_APIC_PHYS: AtomicU64 = AtomicU64::new(0);

/// Where this machine's local APIC is, physically, or `None` if ACPI has not said yet.
///
/// **Provisional name** (milestone 161): `mmu::LOCAL_APIC_PHYS` is the architectural *default*
/// constant and this is what the machine actually reported, which is a distinction worth a better
/// pair of names than these two.
pub fn local_apic_phys() -> Option<u64> {
    match LOCAL_APIC_PHYS.load(Ordering::Relaxed) {
        0 => None,
        base => Some(base),
    }
}

/// Read a local APIC register.
fn read(offset: u64) -> u32 {
    let base = LOCAL_APIC.load(Ordering::Relaxed);
    debug_assert!(base != 0, "the local APIC has not been located");
    // SAFETY: an aligned 32-bit MMIO read of a register the APIC defines, at the address the ACPI
    // MADT stated, reached through the direct map (see `mmu::phys_to_virt`).
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
/// `base` must be this machine's real local APIC address. The direct map covers it from `boot.s`
/// on, and `mmu::init` re-maps that page device-typed rather than moving it.
pub unsafe fn init_local_apic(base: u64) {
    mask_the_8259s();

    LOCAL_APIC_PHYS.store(base, Ordering::Relaxed);
    LOCAL_APIC.store(phys_to_virt(base), Ordering::Relaxed);

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
pub fn mask_timer() {
    write(reg::LVT_TIMER, LVT_MASKED | TIMER_VECTOR as u32);
}

// ---------------------------------------------------------------------------------------------
// The IO APIC: where a *device* line becomes a vector.
// ---------------------------------------------------------------------------------------------

/// **The IO APIC's registers are not a flat array, and that is the first thing to know about it.**
///
/// The whole device is two 32-bit words in the memory map: an index register and a data window.
/// To touch register `n` you write `n` to [`IOREGSEL`] and then read or write [`IOWIN`], which
/// makes every access a *pair* and makes the device stateful. Two CPUs doing this concurrently
/// would interleave and each would read the other's register; nothing here is concurrent yet
/// (single-threaded boot, one CPU), and this note is where the next person finds out that a lock
/// belongs here the moment SMP lands.
mod io_reg {
    /// The index register, at offset 0. Write the number of the register you want.
    pub const IOREGSEL: u64 = 0x00;
    /// The data window, at offset 0x10. Reads and writes land on whatever [`IOREGSEL`] last named.
    pub const IOWIN: u64 = 0x10;

    /// Register 0: this IO APIC's id, in bits 27:24.
    pub const ID: u32 = 0x00;
    /// Register 1: the version in bits 7:0, and in bits 23:16 **the number of redirection entries
    /// minus one**. That "minus one" is the field's definition, not an off-by-one: a 24-entry part
    /// reports 23.
    pub const VERSION: u32 = 0x01;
    /// Register 0x10: the first word of redirection entry 0. Each entry is **two** consecutive
    /// registers, low word first, so entry `n` is at `0x10 + 2 * n`.
    pub const REDIRECTION_BASE: u32 = 0x10;
}

/// Redirection entry bit 16: masked. Every entry powers on masked, which is why arming one is a
/// deliberate act and why [`init_io_apic`] masking them all again is belt and braces rather than
/// necessity.
const REDIR_MASKED: u32 = 1 << 16;
/// Redirection entry bit 15: level triggered rather than edge triggered.
const REDIR_LEVEL: u32 = 1 << 15;
/// Redirection entry bit 13: the input pin is asserted low rather than high.
const REDIR_ACTIVE_LOW: u32 = 1 << 13;

/// **The first vector the IO APIC's lines are routed to.** 0x30, which leaves 0x20..0x2f for the
/// local APIC's own sources (the timer at [`TIMER_VECTOR`], and later the thermal, performance,
/// error and inter-processor vectors, which are LVT entries rather than redirection entries).
///
/// A GSI's vector is this plus the GSI, which is flat and reversible: a stray vector in a fault
/// report names its line by subtraction. That costs the ability to prioritise (x86 priority is the
/// vector's top four bits, so a flat map gives 0x30..0x47 two priority classes and no say in
/// which line is in which). Nothing here has a priority policy to express yet.
///
/// **Provisional name** (milestone 161), along with [`gsi_vector`] and the IO APIC entry points
/// below.
pub const GSI_VECTOR_BASE: u8 = 0x30;

/// The most redirection entries this kernel will use. Real parts have 24 (the 82093AA, QEMU's q35,
/// the ICH-era chipsets); the field could report up to 256, and this cap is the number that still
/// fits in the vector space above [`GSI_VECTOR_BASE`]. It bounds [`is_device_vector`] and the mask
/// loop in [`init_io_apic`] against a version register saying something absurd, and it is the
/// reason [`gsi_vector`] cannot silently wrap onto an exception vector.
const MAX_REDIRECTION_ENTRIES: u32 = 256 - GSI_VECTOR_BASE as u32;

/// Where this machine's IO APIC is, as a virtual address. Zero until [`init_io_apic`] runs. The
/// same direct-map reasoning as [`LOCAL_APIC`].
static IO_APIC: AtomicU64 = AtomicU64::new(0);
/// The **physical** address the IO APIC was found at, kept for `mmu::init` for the reason
/// [`LOCAL_APIC_PHYS`] is.
static IO_APIC_PHYS: AtomicU64 = AtomicU64::new(0);
/// The first global system interrupt this IO APIC owns. Usually 0, and a machine with several
/// parts divides the space by giving each a different base.
static IO_APIC_GSI_BASE: AtomicU32 = AtomicU32::new(0);
/// How many redirection entries it has, read from its version register. Zero until then.
static IO_APIC_ENTRIES: AtomicU32 = AtomicU32::new(0);

/// **The sixteen legacy ISA IRQs as the MADT resolved them**, packed one to a word so the table can
/// live in a static without a lock: the GSI in bits 15:0, "active low" in bit 16, "level triggered"
/// in bit 17. [`NO_ROUTING`] means [`record_isa_routing`] has not run.
///
/// Packed rather than held as a `[IsaIrqRouting; 16]` behind a mutex because it is written once,
/// during single-threaded boot, and read from interrupt-controller code where taking a lock would
/// be the more surprising thing.
static ISA_ROUTING: [AtomicU32; ISA_IRQ_COUNT] =
    [const { AtomicU32::new(NO_ROUTING) }; ISA_IRQ_COUNT];

/// The value in [`ISA_ROUTING`] that means "the MADT has not been read".
const NO_ROUTING: u32 = u32::MAX;

/// Bit 16 of a packed [`ISA_ROUTING`] word.
const PACKED_ACTIVE_LOW: u32 = 1 << 16;
/// Bit 17 of a packed [`ISA_ROUTING`] word.
const PACKED_LEVEL: u32 = 1 << 17;

/// Write the IO APIC's index register, then its data window.
fn io_apic_write(index: u32, value: u32) {
    let base = IO_APIC.load(Ordering::Relaxed);
    debug_assert!(base != 0, "the IO APIC has not been located");
    // SAFETY: two aligned 32-bit MMIO writes to the two registers this device has, at the address
    // the ACPI MADT stated, reached through the direct map. The index write must land before the
    // data write, which `write_volatile` guarantees against another volatile access and TSO
    // guarantees against the device.
    unsafe {
        core::ptr::write_volatile((base + io_reg::IOREGSEL) as *mut u32, index);
        core::ptr::write_volatile((base + io_reg::IOWIN) as *mut u32, value);
    }
}

/// Write the IO APIC's index register, then read its data window.
fn io_apic_read(index: u32) -> u32 {
    let base = IO_APIC.load(Ordering::Relaxed);
    debug_assert!(base != 0, "the IO APIC has not been located");
    // SAFETY: as `io_apic_write`.
    unsafe {
        core::ptr::write_volatile((base + io_reg::IOREGSEL) as *mut u32, index);
        core::ptr::read_volatile((base + io_reg::IOWIN) as *const u32)
    }
}

/// **Bring up the IO APIC at physical address `base`, owning global system interrupts from
/// `gsi_base` up.** Both come from the ACPI MADT.
///
/// Every redirection entry is masked on the way in. They power on masked, so this changes nothing
/// on a cold boot; it matters on a warm one, where firmware may have armed a line for its own use
/// and left it armed, and an inherited interrupt arriving at a vector this kernel has not assigned
/// is a puzzle with no clue in it.
///
/// # Safety
/// `base` must be this machine's real IO APIC address, and the direct map must cover it.
pub unsafe fn init_io_apic(base: u64, gsi_base: u32) {
    IO_APIC_PHYS.store(base, Ordering::Relaxed);
    IO_APIC.store(phys_to_virt(base), Ordering::Relaxed);
    IO_APIC_GSI_BASE.store(gsi_base, Ordering::Relaxed);

    // The version register's bits 23:16 are the entry count *minus one*.
    let entries = (((io_apic_read(io_reg::VERSION) >> 16) & 0xff) + 1).min(MAX_REDIRECTION_ENTRIES);
    IO_APIC_ENTRIES.store(entries, Ordering::Relaxed);

    for entry in 0..entries {
        let index = io_reg::REDIRECTION_BASE + 2 * entry;
        io_apic_write(index + 1, 0);
        io_apic_write(index, REDIR_MASKED);
    }
}

/// Where this machine's IO APIC is, physically, or `None` if [`init_io_apic`] has not run. Read by
/// `mmu::init` so the fine map covers the address the machine reported rather than the constant.
pub fn io_apic_phys() -> Option<u64> {
    match IO_APIC_PHYS.load(Ordering::Relaxed) {
        0 => None,
        base => Some(base),
    }
}

/// This IO APIC's id, from its own register rather than from the MADT. Printed at boot because a
/// disagreement between the two is a firmware bug worth seeing rather than averaging over.
pub fn io_apic_id() -> u8 {
    ((io_apic_read(io_reg::ID) >> 24) & 0x0f) as u8
}

/// Its version register's low byte. 0x11 on the discrete 82093AA and on QEMU's q35, 0x20 on the
/// ICH-era parts.
pub fn io_apic_version() -> u8 {
    io_apic_read(io_reg::VERSION) as u8
}

/// How many redirection entries it has. Zero until [`init_io_apic`] has run.
pub fn io_apic_entries() -> u32 {
    IO_APIC_ENTRIES.load(Ordering::Relaxed)
}

/// **The vector a global system interrupt is routed to.** See [`GSI_VECTOR_BASE`] for why the map
/// is flat.
pub const fn gsi_vector(gsi: u32) -> u8 {
    GSI_VECTOR_BASE.wrapping_add(gsi as u8)
}

/// Is `vector` one of the IO APIC's? The trap handler asks, so that a device interrupt is counted
/// as routed rather than as an unowned vector nothing claimed.
pub fn is_device_vector(vector: u64) -> bool {
    let base = GSI_VECTOR_BASE as u64;
    vector >= base && vector < base + io_apic_entries() as u64
}

/// The redirection-table index for `gsi`, or `None` when this IO APIC does not own that GSI.
///
/// **Not the same number as the GSI**, on a machine with more than one IO APIC: each owns a slice
/// of the global space starting at its own `gsi_base`, and the index is the offset into that slice.
fn redirection_index(gsi: u32) -> Option<u32> {
    let base = IO_APIC_GSI_BASE.load(Ordering::Relaxed);
    let entries = IO_APIC_ENTRIES.load(Ordering::Relaxed);
    let index = gsi.checked_sub(base)?;
    (index < entries).then_some(index)
}

/// **Route `gsi` to `vector` on the local APIC `dest_apic_id`, and unmask it.**
///
/// Fixed delivery mode and physical destination mode: the vector is delivered to exactly the local
/// APIC whose id is named, rather than to a logical group or to whichever CPU is running at the
/// lowest priority. That is the simplest thing that is correct on one CPU and stays correct on
/// several; distributing interrupts is a policy, and there is nothing here to have one yet.
///
/// The high word is written **first**, and the low word (which carries the mask bit) last, so the
/// destination is already in place at the instant the line goes live. The reverse order leaves a
/// window in which an interrupt is delivered to whatever CPU the previous value named.
///
/// # Panics
/// If this IO APIC does not own `gsi`. Silently doing nothing would be an unarmed line that looks
/// exactly like a device that never interrupts.
pub fn route_gsi(gsi: u32, vector: u8, active_low: bool, level_triggered: bool, dest_apic_id: u8) {
    let index = redirection_index(gsi).unwrap_or_else(|| {
        panic!(
            "gsi {gsi} is outside the IO APIC's range (base {}, {} entries)",
            IO_APIC_GSI_BASE.load(Ordering::Relaxed),
            IO_APIC_ENTRIES.load(Ordering::Relaxed),
        )
    });
    let at = io_reg::REDIRECTION_BASE + 2 * index;

    let mut low = vector as u32;
    if active_low {
        low |= REDIR_ACTIVE_LOW;
    }
    if level_triggered {
        low |= REDIR_LEVEL;
    }

    io_apic_write(at + 1, (dest_apic_id as u32) << 24);
    io_apic_write(at, low);
}

/// Mask `gsi` at the IO APIC, leaving the rest of its entry alone.
///
/// # Panics
/// As [`route_gsi`]: a GSI this part does not own is a caller bug, not a no-op.
pub fn mask_gsi(gsi: u32) {
    let index = redirection_index(gsi).unwrap_or_else(|| {
        panic!("gsi {gsi} is outside the IO APIC's range");
    });
    let at = io_reg::REDIRECTION_BASE + 2 * index;
    io_apic_write(at, io_apic_read(at) | REDIR_MASKED);
}

/// **Record how the MADT's interrupt source overrides resolve the sixteen legacy ISA IRQs**, so
/// that [`enable`] can take a legacy number the way the arch contract's callers do.
///
/// Called once, after ACPI has been walked and before any line is armed. Without it [`enable`]
/// falls back to the identity map, which is wrong for the timer on every PC ever built.
pub fn record_isa_routing(table: &[IsaIrqRouting; ISA_IRQ_COUNT]) {
    for (irq, routing) in table.iter().enumerate() {
        let mut packed = routing.gsi & 0xffff;
        if routing.active_low {
            packed |= PACKED_ACTIVE_LOW;
        }
        if routing.level_triggered {
            packed |= PACKED_LEVEL;
        }
        ISA_ROUTING[irq].store(packed, Ordering::Relaxed);
    }
}

/// How legacy IRQ `irq` reaches this machine's IO APIC, falling back to the ISA bus's own defaults
/// when [`record_isa_routing`] has not run or the number is not a legacy IRQ.
pub fn isa_routing(irq: u32) -> IsaIrqRouting {
    let fallback = IsaIrqRouting::isa_default(irq as u8);
    let Some(slot) = ISA_ROUTING.get(irq as usize) else {
        return fallback;
    };
    match slot.load(Ordering::Relaxed) {
        NO_ROUTING => fallback,
        packed => IsaIrqRouting {
            gsi: packed & 0xffff,
            active_low: packed & PACKED_ACTIVE_LOW != 0,
            level_triggered: packed & PACKED_LEVEL != 0,
        },
    }
}

// ---------------------------------------------------------------------------------------------
// The portable arch contract's names.
// ---------------------------------------------------------------------------------------------

/// Bring up the interrupt controller.
///
/// # BUGS
/// **Unimplemented, and it is the argument rather than the work.** The IO APIC's bring-up is
/// [`init_io_apic`], which takes the address and global-interrupt base the ACPI MADT supplied; this
/// name is the arch contract's no-argument one, written for two architectures that read those facts
/// out of a device tree the shared `memory::init` had already parsed. Wiring it up is the device
/// discovery seam (roadmap item 0), not this module.
#[allow(dead_code)]
pub fn init() {
    unimplemented!("x86_64 irq::init: see init_io_apic, which takes the MADT's address")
}

/// Bring up this CPU's local interrupt interface. See [`init_local_apic`], which is the same
/// operation with the address the MADT gave rather than none.
#[allow(dead_code)]
pub fn init_this_cpu() {
    unimplemented!("x86_64 irq::init_this_cpu: see init_local_apic (milestone 161)")
}

/// **Unmask interrupt `intid` at the controller**, where `intid` is a *legacy IRQ number* the way
/// the arch contract's other two implementations take an INTID or a PLIC source.
///
/// The translation is the whole point of this function: [`isa_routing`] turns the legacy number
/// into the GSI the MADT says it actually arrives on, plus that line's polarity and trigger mode,
/// and only then is a redirection entry written. IRQ 0 becomes GSI 2 here, and a version of this
/// that skipped the step would arm the 8259 cascade and report success.
///
/// The vector is [`gsi_vector`]'s, and the destination is the boot CPU.
///
/// # An intid on this architecture is one of two things
///
/// **A local APIC source names itself by its vector**, because there is no controller input to
/// name: [`RESCHEDULE_VECTOR`], [`SELF_TEST_VECTOR`] and its `_B` twin are raised by writing the
/// ICR, and the ICR takes a vector. There is nothing to unmask, so this is a **no-op** for them,
/// and that is a real answer rather than a shrug: the line is already deliverable the moment the
/// local APIC is enabled, which is what `RFLAGS.IF` then gates.
///
/// **Everything else is a legacy IRQ**, 0..15, and goes through the translation above.
///
/// The two ranges cannot collide, which is what makes one function able to take both:
/// [`GSI_VECTOR_BASE`]'s doc reserves 0x20..0x2f for the local APIC's own sources, and a legacy IRQ
/// number never reaches 0x20. **This was a real bug until userspace arrived** (milestone 161, item
/// 4's hand-off): `spawn_init` enables `user::INIT_TEST_SGI`, which on x86 *is*
/// `SELF_TEST_VECTOR`, and routing 34 as though it were a legacy IRQ panicked with
/// "gsi 34 is outside the IO APIC's range". Nothing had ever called `enable` with a local-APIC
/// number before, because nothing above the arch layer had run.
pub fn enable(intid: u32) {
    if is_local_apic_source(intid) {
        return;
    }
    let routing = isa_routing(intid);
    route_gsi(
        routing.gsi,
        gsi_vector(routing.gsi),
        routing.active_low,
        routing.level_triggered,
        local_apic_id(),
    );
}

/// **Does this intid name a local APIC source rather than a controller input?** See [`enable`].
///
/// The range is `TIMER_VECTOR..GSI_VECTOR_BASE`, spelled from those two constants rather than as
/// `0x20..0x30`, so that moving either one moves this with it. A hard-coded pair here would be the
/// third place the same two numbers are written down.
fn is_local_apic_source(intid: u32) -> bool {
    (TIMER_VECTOR as u32..GSI_VECTOR_BASE as u32).contains(&intid)
}

/// **ICR delivery mode `Fixed`**, bits 10:8 = 000: deliver `vector` to the destination, exactly as
/// if a device line had raised it. The other modes (NMI, INIT, STARTUP) are SMP bring-up's, not
/// this.
const ICR_FIXED: u32 = 0b000 << 8;
/// ICR bit 14, the level bit. Must be 1 for every delivery mode except INIT de-assert, which no
/// modern part uses; a zero here is silently ignored on some steppings and not on others.
const ICR_ASSERT: u32 = 1 << 14;
/// ICR destination shorthand bits 19:18 = 01: **self**, no destination field consulted.
const ICR_SELF: u32 = 0b01 << 18;
/// ICR bit 12, delivery status: set by the APIC while a previous IPI is still being sent. Read-only,
/// and polled before writing a new one.
const ICR_PENDING: u32 = 1 << 12;

/// Wait for any previous IPI to be accepted. The ICR is one register per local APIC, so writing it
/// while a send is outstanding loses one of the two.
fn wait_for_ipi_delivery() {
    while read(reg::ICR_LOW) & ICR_PENDING != 0 {
        core::hint::spin_loop();
    }
}

/// **Send `vector` to the local APIC whose id is `dest_apic_id`.**
///
/// The write to [`reg::ICR_LOW`] is what sends it, so the destination goes in the high word first;
/// a version of this that wrote them the other way round would send to whoever the previous IPI
/// named, which is a bug that only appears once there is a second CPU to get it wrong about.
///
/// **Name provisional** (milestone 161, roadmap item 4).
pub fn send_ipi(dest_apic_id: u8, vector: u8) {
    wait_for_ipi_delivery();
    write(reg::ICR_HIGH, (dest_apic_id as u32) << 24);
    write(reg::ICR_LOW, ICR_FIXED | ICR_ASSERT | vector as u32);
}

/// **Raise `vector` on the CPU executing this**, through the local APIC's self shorthand.
///
/// This is x86's answer to a question the other two architectures answer very differently, and the
/// asymmetry is worth stating because `sched`'s interrupt-delivery tests depend on it. aarch64 can
/// raise a software-generated interrupt on itself with no device at all. RISC-V cannot raise
/// anything: `sip.SEIP` is read-only to S-mode and the PLIC's pending block is read-only by
/// specification, so its tests assert the console UART's transmit-empty line instead. x86 is
/// aarch64's case rather than RISC-V's: the local APIC will deliver any vector to itself on demand,
/// through a real delivery path (the ICR, the IRR, the ISR, an EOI), so the interrupt is the
/// hardware's and not a function call wearing a handler's name.
///
/// It is deliberately **not** the console UART's line. COM1's interrupt is discoverable here
/// (`Acpi::isa_irqs[4]`) and unwired, and asserting a device this port has no driver for would prove
/// less than this does while being able to fail for reasons unrelated to the kernel.
///
/// **Name provisional** (milestone 161, roadmap item 4).
pub fn raise_self_interrupt(vector: u8) {
    wait_for_ipi_delivery();
    // No destination word: the `self` shorthand tells the APIC to ignore it, and writing one would
    // be a claim about which CPU this is that the shorthand exists to avoid making.
    write(
        reg::ICR_LOW,
        ICR_SELF | ICR_FIXED | ICR_ASSERT | vector as u32,
    );
}

/// Send a reschedule inter-processor interrupt to `target_cpu`, whose handler drains its inbox and
/// serves any outstanding work-steal request (`sched::drain_inbox`, `sched::serve_steal_request`).
/// The x86 counterpart of aarch64's reschedule SGI and RISC-V's SBI IPI.
///
/// # BUGS
/// **The logical cpu id is used as the destination local APIC id**, which is true on the one CPU
/// this port brings up (the boot CPU is logical 0 and QEMU gives it APIC id 0) and is not true in
/// general: the two numbers are independent and the MADT states the mapping. SMP bring-up
/// (milestone 161, roadmap item 5) is what has to build that table, because it is the same table
/// INIT-SIPI-SIPI needs to name a CPU to start. Nothing calls this today: every caller in `sched`
/// is guarded by "the target is another core", and there is no other core.
pub fn send_reschedule(target_cpu: usize) {
    debug_assert!(
        local_apic_ready(),
        "a reschedule IPI before the local APIC is up has nothing to send it with"
    );
    send_ipi(target_cpu as u8, RESCHEDULE_VECTOR);
}
