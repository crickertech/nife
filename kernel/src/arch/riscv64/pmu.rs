//! **Ask firmware for a hardware cycle counter, and read it** (milestone 74, riscv64 half).
//!
//! The RISC-V hardware performance counters live in M-mode. `mcycle` and the `mhpmcounterX` CSRs
//! can only be started, stopped and configured from there (`mcountinhibit`, `mhpmeventX`), so a
//! supervisor-mode kernel cannot program them itself. The SBI **PMU** extension (EID `0x504D55`,
//! `"PMU"`) is the interface firmware exposes for exactly this, and it is the whole reason this
//! module exists rather than a `csrw`.
//!
//! # What this is for, and what it is deliberately not
//!
//! `crate::arch::timer::now()` reads the `time` CSR, a **fixed-rate reference tick**. That is the
//! OS's clock and it is the right instrument for a long loop. It is not cycles, and the literature
//! this project is compared against is denominated in cycles: milestone 74's block sets out the
//! difference and notes/pmu.md calls confusing the two a category error.
//!
//! **One counter, configured for one event, started and stopped.** Milestone 74's scope note
//! refuses a profiling framework, and the PMU can count dozens of events. Generalising this to a
//! counter set with a named target is milestone 147's, and it should wait for that second consumer.
//!
//! # The shape of the interface, from the specification
//!
//! Everything here is the RISC-V SBI Specification **v3.0** (ratified 2025-07-16), `src/ext-pmu.adoc`,
//! read 2026-09-03 from `github.com/riscv-non-isa/riscv-sbi-doc` at tag `v3.0`. Four calls:
//!
//! 1. `sbi_pmu_num_counters` (FID #0), how many logical counters this hart has.
//! 2. `sbi_pmu_counter_config_matching` (FID #2), "find a counter from this set which is not
//!    started and can monitor this event", which is how a caller gets a counter without knowing
//!    the platform's event-to-counter mapping.
//! 3. `sbi_pmu_counter_get_info` (FID #1), which says whether the counter it found is hardware or
//!    firmware and, for hardware, **which CSR reads it**.
//! 4. `sbi_pmu_counter_stop` (FID #4).
//!
//! The read itself is not an `ecall`. That is the point of step 3: firmware hands back a CSR
//! number and the counter is then read with one instruction, so a measurement does not contain the
//! call that would dominate it. (A *firmware* counter would have to be read with FID #5, an
//! `ecall` per read; this module refuses those, and [`init`] says why.)
//!
//! # Why the counter this asks for is startable at all
//!
//! OpenSBI leaves the fixed counters running and open to S-mode. Read from
//! `lib/sbi/sbi_hart.c` on `riscv-software-src/opensbi` master, 2026-09-03: it writes
//! `CSR_MCOUNTEREN, -1` ("Supervisor mode usage for all counters are enabled by default") and
//! `CSR_MCOUNTINHIBIT, 0xFFFFFFF8`, which leaves bits 0 (`CY`) and 2 (`IR`) clear, so `mcycle` and
//! `minstret` run and only the programmable counters are inhibited until S-mode asks. **That is
//! upstream OpenSBI, not radon's firmware**, and it is a claim about somebody else's build until a
//! bench reads it; notes/riscv-cycle-counters.md makes checking it step one of the procedure.
//!
//! # BUGS
//!
//! - **Nothing here has been run on silicon.** QEMU-TCG models an instruction counter that has
//!   nothing to do with cycles, so a green test under emulation proves the plumbing and says
//!   nothing about the measurement. Every number this module can produce today is an emulator's
//!   fiction. notes/riscv-cycle-counters.md carries the bench procedure that would fix that.
//! - **The boot hart only.** SBI PMU counters are per-hart: `counter_config_matching` and
//!   `counter_start` act on the calling hart, and the counter index a hart is given is that hart's.
//!   [`init`] runs once, on the hart that runs `kernel_main`, and [`cycles`] is therefore only
//!   meaningful to a reader who knows which hart it ran on. The consumer today is a single-hart
//!   bench probe. A per-hart record is real work and belongs with a caller that needs it.
//! - **A firmware counter is refused rather than used.** If firmware answers with a counter it
//!   maintains itself, this module reports no cycle counter at all. That is a deliberate refusal:
//!   reading one costs an `ecall`, and an `ecall` inside a cycle measurement measures the `ecall`.
//! - **`counter_stop` is never called on the happy path.** The counter this configures runs for the
//!   life of the boot. That is correct for a free-running cycle counter, whose value is only ever
//!   read as a difference, and it is why [`stop`] exists but is only used by the test that proves
//!   the call works.

use core::arch::asm;
use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};

use machine_discovery::riscv64::{
    CounterInfo, EID_PMU, EVENT_HW_CPU_CYCLES, PMU_CFG_AUTO_START, PMU_CFG_CLEAR_VALUE, SBI_PMU,
};

/// `sbi_pmu_num_counters`.
const FID_NUM_COUNTERS: usize = 0;
/// `sbi_pmu_counter_get_info`.
const FID_COUNTER_GET_INFO: usize = 1;
/// `sbi_pmu_counter_config_matching`.
const FID_COUNTER_CONFIG_MATCHING: usize = 2;
/// `sbi_pmu_counter_stop`.
const FID_COUNTER_STOP: usize = 4;

/// The CSR that reads the counter firmware gave us, or [`NO_CSR`].
///
/// An atomic rather than a lock: written once by [`init`] on the boot hart before anything reads
/// it, and read-only afterwards. Nothing here nests with another lock, so it earns no rank in
/// `sync::rank`.
static CYCLE_CSR: AtomicU32 = AtomicU32::new(NO_CSR);

/// "No counter", distinct from every legal CSR number (which are 12 bits).
const NO_CSR: u32 = u32::MAX;

/// The logical counter index firmware chose, for the boot line and for [`stop`]. Meaningless while
/// [`CYCLE_CSR`] is [`NO_CSR`].
static CYCLE_COUNTER_IDX: AtomicU64 = AtomicU64::new(0);

/// How wide the counter is, in bits, for the boot line. A narrow counter wraps, and a reader
/// comparing two reads across a long interval needs to know where.
static CYCLE_BITS: AtomicU32 = AtomicU32::new(0);

/// **Find a hardware counter that counts CPU cycles, start it, and remember how to read it.**
///
/// Called once, from `kernel_main`, after [`super::isa::init`] has probed which SBI extensions
/// firmware implements. Silent and harmless on a machine without the PMU extension: [`cycles`]
/// then answers `None` forever, which is the honest answer and not an error.
///
/// # Why `SKIP_MATCH` is not passed
///
/// The specification's `SBI_PMU_CFG_FLAG_SKIP_MATCH` makes firmware "unconditionally select the
/// first counter from the set" without checking it can count the event. That is the wrong request
/// here by exactly the thing this module needs: we do not know which counter counts cycles on this
/// platform, and asking firmware to match is the entire reason to make the call rather than write
/// a CSR number down.
pub fn init() {
    if !super::isa::get().sbi.extensions.contains(SBI_PMU) {
        return;
    }

    let (err, count) = sbi_call(FID_NUM_COUNTERS, [0; 5]);
    if err != 0 || count == 0 {
        return;
    }

    // The whole counter space, as one base plus a mask. `counter_idx_mask` is XLEN wide, so one
    // call covers up to 64 counters and every platform's real count is far below that; a machine
    // that reported more would leave the tail unasked, which is a smaller counter set than the
    // machine has and never a wrong answer about the counter we do get.
    let mask = if count >= 64 {
        u64::MAX as usize
    } else {
        (1usize << count) - 1
    };

    // Configure and start in one call. `CLEAR_VALUE` so the first read is not an inherited number
    // from whatever ran before us, `AUTO_START` so this is one `ecall` rather than two.
    let (err, idx) = sbi_call(
        FID_COUNTER_CONFIG_MATCHING,
        [
            0,
            mask,
            PMU_CFG_CLEAR_VALUE | PMU_CFG_AUTO_START,
            EVENT_HW_CPU_CYCLES,
            0,
        ],
    );
    if err != 0 {
        // `SBI_ERR_NOT_SUPPORTED` (-2) here means no counter on this hart can count cycles, which
        // is a legitimate machine and not a failure to report loudly. The boot line says so.
        return;
    }

    let (err, raw) = sbi_call(FID_COUNTER_GET_INFO, [idx, 0, 0, 0, 0]);
    if err != 0 {
        return;
    }
    let info = CounterInfo::from_raw(raw as u64);

    // A firmware counter is refused: see this module's BUGS. The counter firmware just started for
    // us is left running rather than stopped, because stopping it is an `ecall` whose only purpose
    // would be tidiness on a path that has already decided to report no counter.
    let (Some(csr), Some(bits)) = (info.csr(), info.bits()) else {
        return;
    };

    // A CSR this kernel cannot name in an instruction is the same outcome as no counter, for the
    // same reason: the read must be one instruction or the measurement is measuring the read.
    if read_csr(csr).is_none() {
        return;
    }

    CYCLE_COUNTER_IDX.store(idx as u64, Ordering::Relaxed);
    CYCLE_BITS.store(bits, Ordering::Relaxed);
    // Last, and with Release: a reader that sees a CSR must see the index and width written before
    // it. Every read is `Acquire` on this one location for the same reason.
    CYCLE_CSR.store(csr as u32, Ordering::Release);
}

/// **The cycle count on this hart**, or `None` when no hardware cycle counter was found.
///
/// One CSR read, no `ecall`, so a caller may put this on either side of the thing it is measuring.
///
/// **The value is only meaningful as a difference.** The specification's own note on the event says
/// these "may be variable frequency cycles, and are not counted when the CPU clock is halted", so a
/// count is not a duration and does not become one by dividing by a nominal clock. That is the
/// whole reason this exists beside the `time` CSR rather than instead of it.
pub fn cycles() -> Option<u64> {
    let csr = CYCLE_CSR.load(Ordering::Acquire);
    if csr == NO_CSR {
        return None;
    }
    read_csr(csr as u16)
}

/// How wide the counter is, in bits, or `None` when there is none. For the boot line: a counter
/// narrower than 64 bits wraps, and a reader differencing two reads needs to know where.
pub fn cycle_counter_width() -> Option<u32> {
    if CYCLE_CSR.load(Ordering::Acquire) == NO_CSR {
        return None;
    }
    Some(CYCLE_BITS.load(Ordering::Relaxed))
}

/// **Stop the counter [`init`] started.**
///
/// Not on any normal path: a free-running cycle counter read as a difference never needs stopping,
/// and this module's BUGS says so. It exists because the stop call is half of the interface
/// milestone 74 names, and a call nothing ever makes is a call nothing has ever checked.
///
/// Returns the SBI error code, so a caller can tell "stopped" from "was already stopped"
/// (`SBI_ERR_ALREADY_STOPPED`, -8) from "there was no counter" (-3, which is what an invalid index
/// earns).
#[cfg_attr(not(test), allow(dead_code))]
pub fn stop() -> isize {
    if CYCLE_CSR.load(Ordering::Acquire) == NO_CSR {
        return -3;
    }
    let idx = CYCLE_COUNTER_IDX.load(Ordering::Relaxed) as usize;
    let (err, _) = sbi_call(FID_COUNTER_STOP, [idx, 1, 0, 0, 0]);
    err
}

/// **Read one of the U/S-readable counter CSRs by number.**
///
/// RISC-V encodes the CSR as a 12-bit immediate in the instruction, so there is no "read the CSR
/// this variable names" instruction and this dispatch cannot be avoided. `None` for anything
/// outside the counter range, which is the same answer as "no counter" to every caller.
///
/// The range is the unprivileged counter block: `cycle` (`0xc00`), `instret` (`0xc02`) and
/// `hpmcounter3` through `hpmcounter31` (`0xc03` to `0xc1f`). `time` (`0xc01`) is deliberately
/// absent: firmware will never name it as a PMU counter, and a `time` read reaching this path
/// would be the fixed-rate tick wearing a cycle counter's name, which is the exact confusion
/// milestone 74 exists to remove.
fn read_csr(csr: u16) -> Option<u64> {
    macro_rules! read {
        ($name:literal) => {{
            let value: u64;
            // SAFETY: reading an unprivileged counter CSR touches no memory and changes no state,
            // which the options state. S-mode reads of these are gated by `mcounteren` in firmware;
            // this path is only reached after SBI PMU handed us the CSR number for a counter it
            // started, which is firmware saying it is readable.
            unsafe {
                asm!(
                    concat!("csrr {}, ", $name),
                    out(reg) value,
                    options(nomem, nostack, preserves_flags),
                );
            }
            value
        }};
    }

    /// Expand one arm per `hpmcounterN`, so the 29 regular cases are written once.
    macro_rules! hpm {
        ($csr:expr, $($num:literal => $name:literal,)*) => {
            match $csr {
                $($num => Some(read!($name)),)*
                _ => None,
            }
        };
    }

    hpm! { csr,
        0xc00 => "cycle",
        0xc02 => "instret",
        0xc03 => "hpmcounter3",
        0xc04 => "hpmcounter4",
        0xc05 => "hpmcounter5",
        0xc06 => "hpmcounter6",
        0xc07 => "hpmcounter7",
        0xc08 => "hpmcounter8",
        0xc09 => "hpmcounter9",
        0xc0a => "hpmcounter10",
        0xc0b => "hpmcounter11",
        0xc0c => "hpmcounter12",
        0xc0d => "hpmcounter13",
        0xc0e => "hpmcounter14",
        0xc0f => "hpmcounter15",
        0xc10 => "hpmcounter16",
        0xc11 => "hpmcounter17",
        0xc12 => "hpmcounter18",
        0xc13 => "hpmcounter19",
        0xc14 => "hpmcounter20",
        0xc15 => "hpmcounter21",
        0xc16 => "hpmcounter22",
        0xc17 => "hpmcounter23",
        0xc18 => "hpmcounter24",
        0xc19 => "hpmcounter25",
        0xc1a => "hpmcounter26",
        0xc1b => "hpmcounter27",
        0xc1c => "hpmcounter28",
        0xc1d => "hpmcounter29",
        0xc1e => "hpmcounter30",
        0xc1f => "hpmcounter31",
    }
}

/// One SBI PMU call, returning `(error, value)` from `a0` and `a1`.
///
/// Unlike `isa::sbi_call`, this one **returns the error**. The base extension's getters cannot
/// fail; every call here can, and every failure means "there is no cycle counter", which is a fact
/// the boot line reports rather than one to discard.
fn sbi_call(fid: usize, args: [usize; 5]) -> (isize, usize) {
    let error: isize;
    let value: usize;
    // SAFETY: an SBI call into M-mode firmware. a7 = extension, a6 = function, a0..a4 = arguments.
    // The firmware writes a0 (error) and a1 (value) and touches nothing else. `nostack` is right
    // for the same reason it is at the kernel's other four SBI sites: the callee runs on M-mode's
    // own stack.
    unsafe {
        asm!(
            "ecall",
            in("a7") EID_PMU,
            in("a6") fid,
            inout("a0") args[0] => error,
            inout("a1") args[1] => value,
            in("a2") args[2],
            in("a3") args[3],
            in("a4") args[4],
            options(nostack),
        );
    }
    (error, value)
}

/// The boot line, printed beside the ISA summary.
pub fn print_summary() {
    if let Some(bits) = cycle_counter_width() {
        crate::println!(
            "  cycles      : SBI PMU counter {}, CSR {:#05x}, {bits} bits",
            CYCLE_COUNTER_IDX.load(Ordering::Relaxed),
            CYCLE_CSR.load(Ordering::Relaxed),
        );
    } else if super::isa::get().sbi.extensions.contains(SBI_PMU) {
        crate::println!("  cycles      : SBI PMU present but no hardware cycle counter");
    } else {
        crate::println!("  cycles      : no SBI PMU extension; ticks only");
    }
}

#[cfg(test)]
mod tests {
    //! What only a boot can say. The `counter_info` decode, the event encoding and the config
    //! flags are proved on the host in `crates/machine_discovery/tests/riscv64_isa_strings.rs`;
    //! these are the assertions that need firmware on the other end of an `ecall`.
    //!
    //! **None of these is a claim about cycles.** QEMU-TCG's `cycle` CSR is an instruction count,
    //! so every assertion here is about the plumbing: that the call was made, that the answer was
    //! decoded, that the CSR named is readable and moves forward. The measurement is
    //! notes/riscv-cycle-counters.md's bench procedure and it has not been run.

    use super::*;

    /// **The PMU probe and the counter agree about whether there is one.**
    ///
    /// Three states, and the test's job is that they are consistent rather than that any
    /// particular one holds: no PMU extension means no counter; a PMU extension may still yield no
    /// counter (a firmware counter, or a CSR this kernel cannot name); and a counter implies a
    /// width. A machine reporting a counter with no width, or a width with no counter, would be
    /// the atomics having been written out of order.
    #[test_case]
    fn the_counter_and_the_probe_agree() {
        let has_pmu = super::super::isa::get().sbi.extensions.contains(SBI_PMU);
        let counter = cycles();

        if !has_pmu {
            assert!(
                counter.is_none(),
                "no PMU extension, so nothing can have configured a counter"
            );
            return;
        }

        assert_eq!(
            counter.is_some(),
            cycle_counter_width().is_some(),
            "the counter and its width are written together or not at all"
        );

        if let Some(bits) = cycle_counter_width() {
            assert!(
                (1..=64).contains(&bits),
                "width {bits} is outside what a 6-bit field plus one can encode"
            );
        }
    }

    /// **QEMU's OpenSBI implements the PMU extension, and it gave us the `cycle` CSR.**
    ///
    /// Asserted rather than skipped because it is the machine every merge boots, and a
    /// regression that silently lost the counter would otherwise pass the test above by taking
    /// its "no counter" branch. `sbi_probe_extension` answering yes and `counter_config_matching`
    /// then finding nothing would be a real defect on this machine.
    ///
    /// **On radon this assertion may legitimately fail**, which is the whole point of the bench
    /// procedure. It is written against QEMU because QEMU is what runs it.
    #[test_case]
    fn qemu_gives_us_the_cycle_csr() {
        let isa = super::super::isa::get();
        assert!(
            isa.sbi.extensions.contains(SBI_PMU),
            "OpenSBI has implemented SBI PMU since 0.8"
        );
        assert!(
            cycles().is_some(),
            "SBI PMU is present and no counter was configured for CPU_CYCLES"
        );
        assert_eq!(
            CYCLE_CSR.load(Ordering::Acquire),
            0xc00,
            "OpenSBI maps CPU_CYCLES to mcycle, read through the `cycle` CSR"
        );
    }

    /// **The counter moves forward.**
    ///
    /// The weakest claim that would still catch the failure that matters: a CSR read that returns a
    /// constant (the wrong CSR, an inhibited counter, a decode that landed on a reserved number)
    /// looks exactly like a working counter to every other test here.
    ///
    /// **This is not a measurement.** Under TCG the delta is an instruction count. The loop is a
    /// `time` CSR spin rather than a fixed iteration count so that the two counters are read over
    /// the same span, which is the shape the bench probe uses and the shape that would show a
    /// counter running at an implausible rate on silicon.
    #[test_case]
    fn the_counter_advances() {
        let Some(first) = cycles() else {
            return;
        };

        let start = super::super::timer::now();
        while super::super::timer::now() - start < 1000 {
            core::hint::spin_loop();
        }

        let second = cycles().expect("the counter did not vanish mid-test");
        assert!(
            second.wrapping_sub(first) > 0,
            "counter read {first} twice across a timed spin; it is not counting"
        );
    }

    /// **Stop is a real call and firmware answers it.**
    ///
    /// The one place `stop` is exercised, and it restarts the counter afterwards so no later test
    /// or bench inherits a stopped one. `SBI_SUCCESS` (0) and `SBI_ERR_ALREADY_STOPPED` (-8) are
    /// both correct answers; anything else means the counter index we are holding is not one
    /// firmware recognises, which would make every other number here suspect.
    #[test_case]
    fn stopping_the_counter_is_accepted() {
        if cycles().is_none() {
            return;
        }
        let err = stop();
        assert!(
            err == 0 || err == -8,
            "sbi_pmu_counter_stop returned {err}, which is neither success nor already-stopped"
        );

        // Put it back, through the same path that configured it, so the module is left as the rest
        // of the boot expects to find it.
        init();
        assert!(cycles().is_some(), "the counter did not come back");
    }
}
