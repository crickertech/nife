//! **The timer, `x86_64`.** A calibrated TSC for reading time, and the local APIC timer for the tick.
//!
//! # Why this took a calibration loop when the other two took a register read
//!
//! aarch64 has the architected generic timer: `CNTPCT_EL0` counts and `CNTFRQ_EL0` **says how
//! fast**. RISC-V has `time`, and its rate is stated in the device tree. In both cases the machine
//! tells you the number.
//!
//! x86 has at least four clocks and no architected way to ask any of them its rate on the parts
//! this has to run on. `CPUID` leaf 0x15 reports the TSC's ratio to a "core crystal" whose frequency
//! leaf 0x16 may or may not give, and neither leaf exists on everything. The local APIC timer counts
//! the bus clock, which nothing reports at all. So the frequency of the clock everything is measured
//! against has itself to be **measured**, against the one device on a PC whose rate is a fixed
//! number: the 8254 PIT, at 1193182 Hz, unchanged since 1981 because it was derived by dividing the
//! NTSC colour-burst frequency and every clone copied it.
//!
//! # What is calibrated against what
//!
//! One PIT interval, ten milliseconds, timed by polling. Across it, both the TSC and the local APIC
//! timer's countdown are read, which gives both frequencies from one wait. That is deliberate: two
//! separate calibrations would spend twice the boot time and produce two answers that disagree by
//! however much the two waits differed.
//!
//! # BUGS
//!
//! - **The TSC is assumed invariant and this does not check.** `CPUID.80000007H:EDX[8]` says whether
//!   the TSC keeps a constant rate across frequency and idle-state changes; on anything older, or on
//!   a machine that lies, `now()` drifts against wall time when the CPU idles. QEMU's TSC is
//!   invariant. A real machine must have the bit checked, and milestone 87's is where that gets
//!   tested rather than argued.
//! - **One calibration, no averaging.** A single 10 ms window on a busy host under TCG can be off by
//!   a per cent or so. The other two architectures read an exact number, so nothing above this has
//!   ever had to think about calibration error; anything that benchmarks on x86 will.
//! - **`init` panics if the local APIC is not up.** The ordering (APIC, then timer) is a real
//!   constraint and is enforced loudly rather than producing a timer that never fires.

use core::sync::atomic::{AtomicU64, Ordering};

use super::irq;
use super::port::{in8, out8};

/// Ticks per second the scheduler asks for. The same 100 Hz both other architectures use.
pub const TICK_HZ: u64 = 100;

/// **The 8254 PIT's input frequency**, 1193182 Hz, and the one rate on a PC that is a fixed number
/// rather than something to measure. It is 315/22 MHz divided by 12, which is to say it descends
/// from the NTSC colour-burst crystal that made 1981's parts cheap.
const PIT_HZ: u64 = 1_193_182;

/// How long the calibration window is. Ten milliseconds is a compromise: long enough that the
/// polling loop's own overhead is noise, short enough that the PIT's 16-bit counter holds it (its
/// maximum is about 54.9 ms) and that boot does not visibly pause.
const CALIBRATION_MS: u64 = 10;

/// PIT channel 2's data port. Channel 2 is the one to use, and the reason is that it is the only
/// channel whose **gate is under software control** (port 0x61 bit 0) and whose **output can be
/// polled** (bit 5). Channels 0 and 1 are wired to the interrupt controller and to DRAM refresh, so
/// timing against either means taking interrupts, which is what this runs before.
const PIT_CHANNEL2: u16 = 0x42;
/// The PIT's mode/command register.
const PIT_COMMAND: u16 = 0x43;
/// The "system control port B" that gates channel 2 and reports its output. Bit 0 is the gate, bit 1
/// is the speaker (which stays off), and bit 5 reads channel 2's output.
const PIT_GATE_PORT: u16 = 0x61;

/// Channel 2, access mode lobyte/hibyte, operating mode 0 (interrupt on terminal count), binary.
/// Mode 0 is the one that counts down once and raises its output line, which is exactly a
/// one-shot stopwatch.
const PIT_CHANNEL2_ONESHOT: u8 = 0b1011_0010;

/// PIT **channel 0**'s data port. Channel 0 is the one whose output is wired to the interrupt
/// controller, which is why calibration cannot use it (it would mean taking interrupts) and why
/// proving the IO APIC works can use nothing else.
const PIT_CHANNEL0: u16 = 0x40;

/// Channel 0, access mode lobyte/hibyte, operating mode 2 (rate generator), binary. Mode 2 pulses
/// the output line once per reload and then reloads itself, which is a periodic interrupt source
/// rather than the one-shot mode 0 the calibration uses.
const PIT_CHANNEL0_RATE: u8 = 0b0011_0100;

/// **The legacy ISA IRQ number channel 0's output carries.** Zero, on every PC.
///
/// It is emphatically **not** the IO APIC input the line arrives on: the MADT's interrupt source
/// overrides say the PIT is wired to global system interrupt 2, because pin 0 carries the 8259
/// cascade. See `arch/x86_64/irq.rs`.
pub const PIT_IRQ: u32 = 0;

/// The TSC's frequency in hertz, established by [`init_frequency`]. Zero until then.
static TSC_HZ: AtomicU64 = AtomicU64::new(0);

/// Whether [`TSC_HZ`] came from `CPUID` leaf `0x15` (`true`) or from PIT calibration (`false`).
/// Set by [`init_frequency`] alongside `TSC_HZ` itself; meaningless before that (reads `false`,
/// the calibration path's own value, which is also what every QEMU boot actually takes: see
/// `arch::x86_64::isa::tsc_crystal_hz`'s own docs for why leaf 0x15 is empirically unavailable
/// under `-cpu max`).
static TSC_HZ_FROM_CPUID: core::sync::atomic::AtomicBool = core::sync::atomic::AtomicBool::new(false);

/// The local APIC timer's frequency in hertz, at the divider `irq` programs. Zero until measured.
static APIC_TIMER_HZ: AtomicU64 = AtomicU64::new(0);

/// Scheduler ticks taken since the timer was armed.
static TICKS: AtomicU64 = AtomicU64::new(0);

/// Read the time-stamp counter.
///
/// `rdtsc` returns a 64-bit value in `edx:eax`, which is why this is two 32-bit outputs shifted
/// together rather than one register. **Not serialising**: the CPU may execute it out of order with
/// respect to surrounding instructions, so a measurement of a short interval wants `lfence` or
/// `rdtscp` around it. The calibration below measures ten milliseconds, where a few tens of cycles
/// of reordering is not measurable, so it does not pay for the fence.
fn rdtsc() -> u64 {
    let low: u32;
    let high: u32;
    // SAFETY: reads a counter. No memory effect, no flag effect.
    unsafe {
        core::arch::asm!(
            "rdtsc",
            out("eax") low,
            out("edx") high,
            options(nomem, nostack, preserves_flags),
        );
    }
    ((high as u64) << 32) | low as u64
}

/// Wait `CALIBRATION_MS` by polling PIT channel 2, and return what the TSC and the APIC timer did
/// across it.
///
/// Returns `(tsc_delta, apic_delta)`. The APIC timer **counts down**, so its delta is
/// `start - end`; a caller that got the subtraction the other way round would compute an enormous
/// frequency and a timer that never fires, which is why it is done here rather than at the call
/// site.
fn measure_against_the_pit() -> (u64, u32) {
    let count = (PIT_HZ * CALIBRATION_MS / 1000) as u16;

    // Start the APIC timer as a free-running masked counter from its maximum, so it cannot wrap
    // during a window this short: at any plausible bus clock, 0xffffffff ticks divided by 16 is
    // minutes, not milliseconds.
    irq::start_timer_for_calibration(u32::MAX);
    let apic_start = irq::timer_current_count();

    // SAFETY: the PIT and its gate port are fixed ISA devices present on every x86 machine. The
    // speaker bit is explicitly cleared, so nothing audible happens; the gate is toggled low then
    // high, which is what arms channel 2's countdown.
    let tsc_start = unsafe {
        let gate = in8(PIT_GATE_PORT) & 0xfc; // clear the gate and the speaker
        out8(PIT_GATE_PORT, gate); // gate low: the channel is held
        out8(PIT_COMMAND, PIT_CHANNEL2_ONESHOT);
        out8(PIT_CHANNEL2, count as u8);
        out8(PIT_CHANNEL2, (count >> 8) as u8);
        let t = rdtsc();
        out8(PIT_GATE_PORT, gate | 1); // gate high: it starts counting
        t
    };

    // Poll channel 2's output. Mode 0 holds it low while counting and raises it at terminal count.
    // Bounded by nothing, deliberately: this is before any interrupt exists, the PIT is not
    // optional on a PC, and a bound here would silently produce a wrong frequency rather than a
    // visible hang. If it never returns, the machine has no PIT and that is worth finding out
    // loudly.
    // SAFETY: reading the gate port, which has no read side effects.
    while unsafe { in8(PIT_GATE_PORT) } & 0x20 == 0 {
        core::hint::spin_loop();
    }

    let tsc_end = rdtsc();
    let apic_end = irq::timer_current_count();

    // SAFETY: put the gate back down so channel 2 is not left running.
    unsafe {
        let gate = in8(PIT_GATE_PORT) & 0xfc;
        out8(PIT_GATE_PORT, gate);
    }

    (
        tsc_end.wrapping_sub(tsc_start),
        apic_start.wrapping_sub(apic_end),
    )
}

/// **Establish the TSC's rate and measure the local APIC timer against the PIT.**
///
/// Takes the portable arch contract's boot-info-pointer argument, which x86 ignores; the numbers
/// come from the machine rather than from a table, which is the shape of the difference this
/// module's header is about.
///
/// **The TSC rate itself is asked for before it is measured** (milestone 161's `cntfrq`
/// follow-up): `isa::tsc_crystal_hz` reads `CPUID` leaf `0x15` first, and only the PIT-measured
/// delta is used if that comes back `None`. The local APIC timer's rate has no `CPUID`
/// equivalent at all, so the PIT window always runs regardless of which source wins the TSC
/// number; the one window prices both. See [`crate::arch::x86_64::isa::tsc_crystal_hz`] for why
/// this project's own QEMU invocation always takes the calibrated path.
///
/// # Panics
/// If the local APIC is not up. The order is APIC then timer, and a timer calibrated against a
/// counter that is not running would produce a plausible TSC frequency and a nonsense tick period.
pub fn init_frequency(boot_info_pointer: usize) {
    let _ = boot_info_pointer;
    assert!(
        irq::local_apic_ready(),
        "the timer calibrates the local APIC's counter, so the APIC must be up first",
    );

    let (tsc_delta, apic_delta) = measure_against_the_pit();
    let per_second = 1000 / CALIBRATION_MS;
    APIC_TIMER_HZ.store(apic_delta as u64 * per_second, Ordering::Relaxed);

    match super::isa::tsc_crystal_hz() {
        Some(hz) => {
            TSC_HZ.store(hz, Ordering::Relaxed);
            TSC_HZ_FROM_CPUID.store(true, Ordering::Relaxed);
        }
        None => {
            TSC_HZ.store(tsc_delta * per_second, Ordering::Relaxed);
            TSC_HZ_FROM_CPUID.store(false, Ordering::Relaxed);
        }
    }
}

/// **Start PIT channel 0 pulsing its interrupt line at about `hz`**, and report the rate actually
/// programmed.
///
/// The rate is "about" because the divisor is an integer: the PIT counts down from it at
/// [`PIT_HZ`], so only the frequencies that divide 1193182 exactly are exact. 100 Hz is 11932.4,
/// which rounds to a real rate of 100.0035 Hz. Reporting the achieved rate rather than the asked-for
/// one is what keeps a boot print from claiming a number the hardware is not producing.
///
/// **This does not route the interrupt anywhere.** The line goes to both interrupt controllers; the
/// 8259s are masked, and the IO APIC delivers nothing until `irq::enable` arms the redirection entry
/// the MADT's override names. This is the device half, and it is here because `timer.rs` is where
/// the PIT lives.
///
/// **Provisional name** (milestone 161).
pub fn start_pit_ticking(hz: u64) -> u64 {
    let divisor = (PIT_HZ / hz).clamp(1, u16::MAX as u64) as u16;

    // SAFETY: the PIT is a fixed ISA device present on every x86 machine, and these three writes
    // are its documented programming sequence: the mode/command byte first (which latches the
    // access mode), then the divisor's low byte and high byte in that order, because the access
    // mode just selected says there are two of them.
    unsafe {
        out8(PIT_COMMAND, PIT_CHANNEL0_RATE);
        out8(PIT_CHANNEL0, divisor as u8);
        out8(PIT_CHANNEL0, (divisor >> 8) as u8);
    }

    PIT_HZ / divisor as u64
}

/// The counter's current value: the TSC.
pub fn now() -> u64 {
    rdtsc()
}

/// How many counter ticks make a second, or `None` if [`init_frequency`] has not run yet.
///
/// Unlike [`frequency`], does not panic: this is for a caller (the timebase page,
/// `kernel::user::x86_timebase_page_phys`) that must handle "not measured yet" as a normal,
/// representable state rather than a bug to crash on. In practice this is never `None` by the
/// time any process is loaded: `init_frequency` runs early in the boot tour, well before the
/// first call to `kernel::user::load`.
pub fn frequency_checked() -> Option<u64> {
    let hz = TSC_HZ.load(Ordering::Relaxed);
    (hz != 0).then_some(hz)
}

/// How many counter ticks make a second.
///
/// # Panics
/// If [`init_frequency`] has not run. Returning zero would make every duration computed from it
/// either zero or a division by zero, arbitrarily far from the missing call.
pub fn frequency() -> u64 {
    frequency_checked().expect("the TSC frequency has not been measured yet")
}

/// Whether [`frequency`]'s number came from `CPUID` leaf `0x15` or from PIT calibration, for the
/// boot print and this milestone's own evidence. `"uncalibrated"` before [`init_frequency`] has
/// run, which [`frequency`] itself would panic on; this never panics.
pub fn frequency_source() -> &'static str {
    if TSC_HZ.load(Ordering::Relaxed) == 0 {
        "uncalibrated"
    } else if TSC_HZ_FROM_CPUID.load(Ordering::Relaxed) {
        "cpuid leaf 0x15"
    } else {
        "PIT calibration"
    }
}

/// Counter ticks between scheduler ticks.
pub fn interval() -> u64 {
    frequency() / TICK_HZ
}

/// The local APIC timer's measured frequency, for the boot print. Zero before calibration.
pub fn apic_timer_frequency() -> u64 {
    APIC_TIMER_HZ.load(Ordering::Relaxed)
}

/// **Arm the periodic tick.** The local APIC timer, at [`TICK_HZ`], on the vector `irq` chose.
///
/// # Panics
/// If [`init_frequency`] has not run, for the same reason [`frequency`] does.
pub fn init() {
    let apic_hz = APIC_TIMER_HZ.load(Ordering::Relaxed);
    assert!(
        apic_hz != 0,
        "the local APIC timer's rate has not been measured yet (call init_frequency first)",
    );
    let count = (apic_hz / TICK_HZ) as u32;
    assert!(
        count != 0,
        "the measured APIC timer rate is below the tick rate, which cannot be right",
    );
    irq::arm_periodic_timer(count);
}

/// **Take one tick.** Called from the trap handler when [`irq::TIMER_VECTOR`] arrives.
///
/// It does not write the EOI: the trap handler does that for every interrupt vector, in one place,
/// because a missed EOI is a hang rather than an error and one place is easier to be sure about
/// than one per handler.
pub fn tick() {
    TICKS.fetch_add(1, Ordering::Relaxed);
}

/// Scheduler ticks taken so far.
pub fn ticks() -> u64 {
    TICKS.load(Ordering::Relaxed)
}

/// Spin for `counter_ticks` of the TSC.
pub fn spin_for(counter_ticks: u64) {
    let start = now();
    while now().wrapping_sub(start) < counter_ticks {
        core::hint::spin_loop();
    }
}

/// Scheduler ticks taken on `cpu`.
///
/// # BUGS
/// **There is one CPU, so this ignores its argument and reports the global count.** The other two
/// architectures keep a per-CPU array so a before/after pair names one CPU even if the caller
/// migrated; nothing can migrate here yet. It becomes wrong the moment SMP lands, which is why it
/// says so rather than looking finished.
#[allow(dead_code)]
pub fn ticks_on(cpu: usize) -> u64 {
    let _ = cpu;
    ticks()
}
