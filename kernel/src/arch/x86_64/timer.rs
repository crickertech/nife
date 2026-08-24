//! **The timer, `x86_64`.** Not built; this module states the shape it will take and refuses to
//! guess.
//!
//! # Why there is nothing here yet, and it is not laziness
//!
//! The other two architectures have exactly one obvious clock each. aarch64 has the architected
//! generic timer: `CNTPCT_EL0` counts, `CNTFRQ_EL0` says how fast, `CNTP_TVAL_EL0` arms. RISC-V has
//! `time` and the SBI `set_timer` call. In both cases the counter's rate is *stated* by the machine.
//!
//! x86 has at least four (the PIT, the local APIC timer, the HPET, and the TSC), none of them
//! architecturally self-describing in the way those are, and the one everything actually uses (the
//! TSC) has to be **calibrated against another one** because `CPUID` only reports its frequency on
//! some parts. The shape of a correct answer is: bring up the local APIC timer as the interrupt
//! source, calibrate the TSC against the PIT (or against the APIC timer's own known divisor), and
//! check `CPUID.80000007H:EDX[8]` for an invariant TSC before trusting it across idle states.
//!
//! Every function below would have to *return a number*, and a wrong number here is the worst kind
//! of bug this kernel can have: the scheduler's preemption budget, every benchmark, and the
//! wall-clock service are all downstream of it, and each would be quietly wrong rather than
//! visibly broken. So these panic. See design/roadmap/161-x86-64-kernel-port.md.

/// Ticks per second the scheduler asks for. The same 100 Hz both other architectures use; stated
/// here because it is a policy constant rather than a hardware fact, so it is knowable now.
pub const TICK_HZ: u64 = 100;

macro_rules! not_yet {
    ($name:literal) => {
        unimplemented!(concat!(
            "x86_64 timer::",
            $name,
            ": no clock is brought up (milestone 161). See this module's header for why a guess is \
             worse than a panic."
        ))
    };
}

/// The counter's current value.
pub fn now() -> u64 {
    not_yet!("now")
}

/// How many counter ticks make a second.
pub fn frequency() -> u64 {
    not_yet!("frequency")
}

/// Counter ticks between scheduler ticks.
#[allow(dead_code)]
pub fn interval() -> u64 {
    not_yet!("interval")
}

/// Learn the counter's rate. Takes the portable arch contract's device-tree argument, which x86
/// does not have; calibration is what goes here.
#[allow(dead_code)]
pub fn init_frequency(dtb_ptr: usize) {
    let _ = dtb_ptr;
    not_yet!("init_frequency")
}

/// Arm the periodic timer interrupt.
#[allow(dead_code)]
pub fn init() {
    not_yet!("init")
}

/// Scheduler ticks taken on this CPU so far.
#[allow(dead_code)]
pub fn ticks() -> u64 {
    not_yet!("ticks")
}

/// Spin for `counter_ticks` of the counter.
#[allow(dead_code)]
pub fn spin_for(counter_ticks: u64) {
    let _ = counter_ticks;
    not_yet!("spin_for")
}

/// Scheduler ticks taken on `cpu`. Read by index rather than by "this CPU" so that a before/after
/// pair names one CPU even if the caller migrated between them; see the RISC-V twin for the
/// reasoning, which is portable.
#[allow(dead_code)]
pub fn ticks_on(cpu: usize) -> u64 {
    let _ = cpu;
    not_yet!("ticks_on")
}
