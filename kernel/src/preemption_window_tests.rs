//! **A timed window can exclude preemption, and this is the test that says so.**
//! Milestone 541 (a timed window that excludes preemption).
//!
//! `kernel/src/bench.rs`'s `map_new` masks interrupts across its timed window, because the window
//! is roughly 250 microseconds of guest time against a 10 ms scheduler tick (`TICK_HZ` is 100 on
//! all three architectures) and whether a timer interrupt lands inside it is therefore decided by
//! the phase the boot left the timer in. On 2026-09-21 a semantically identical rewrite of one
//! expression in `schedule()` moved that phase, one preemption landed in the window, and the row
//! read +26.4% and failed the tripwire while the map path's object code was byte-identical. See
//! notes/benchmarks.md and design/roadmap/541-a-timed-window-that-excludes-preemption.md.
//!
//! **The benchmark build is not the tested build**, which is the gap this file closes. `--features
//! bench` compiles the tour out and runs no tests, so nothing in the test suite ever executes
//! `map_new`'s masked window; what the suite *can* test is the primitive that window rests on,
//! which is that `arch::interrupts::disable` actually suppresses the scheduler tick and `restore`
//! actually brings it back. If either half stopped being true, `map_new` would go back to timing
//! the scheduler and would say nothing about it.
//!
//! These tests spin against the wall clock rather than counting instructions, so they hold under
//! `-icount` and under HVF alike: the counter `now()` reads advances in both.

use crate::sched;

/// How long to spin, in counter ticks: three scheduler tick periods, so a tick is unambiguously
/// due rather than marginally due. `frequency()` is counter ticks per second and `TICK_HZ` is
/// scheduler ticks per second, so their ratio is counter ticks per scheduler tick.
fn three_tick_periods() -> u64 {
    3 * crate::arch::timer::frequency() / crate::arch::timer::TICK_HZ
}

/// **Interrupts masked means no preemption is taken, however long the window is.**
///
/// This is the property `map_new` buys. The spin is three scheduler tick periods, so without the
/// mask the timer would have fired at least twice; with it, this core's count must not move at
/// all. The count is **per-core** (`cpu::PerCpu::preemptions`) for a reason this test found the
/// hard way: on a multi-core kernel the global `sched::preemptions()` reported nine preemptions
/// inside this masked window, every one of them another core ticking its own thread.
///
/// It asserts on the counter rather than on elapsed time on purpose. A window that merely *ran
/// fast* proves nothing, because a preemption that returns promptly is cheap and one that yields a
/// full quantum to another thread is not: the two differed by a factor of seven in the 2026-09-21
/// measurements (6,670 ticks against 47,752 for one preemption on `x86_64`). The count is the
/// thing that is either zero or not.
#[test_case]
fn a_masked_window_takes_no_preemption() {
    let was_enabled = crate::arch::interrupts::disable();
    let before = sched::preemptions_here();
    crate::arch::timer::spin_for(three_tick_periods());
    let during = sched::preemptions_here();
    crate::arch::interrupts::restore(was_enabled);

    assert_eq!(
        during,
        before,
        "{} preemption(s) were taken inside a window with interrupts masked; map_new's timed \
         window is built on this not happening, and if it can happen the row is timing the \
         scheduler again",
        during - before,
    );
}

/// **And the interrupt is deferred, not lost**, which is the other half and the one a reader is
/// right to doubt.
///
/// Masking a window would be a bad trade if it dropped the tick: the scheduler would be starved
/// for as long as the window lasted and the benchmark would have bought its stable number by
/// breaking the machine. It does not. The timer interrupt is pending across the mask and is
/// delivered at `restore`, which is why `map_new`'s own probe can report a non-zero preemption
/// count for a window that took none.
///
/// The spin after `restore` is one tick period, which is generous: the pending interrupt is
/// delivered immediately, and the spin only covers the case where the mask happened to close a
/// hair before the tick was due.
#[test_case]
fn unmasking_delivers_the_tick_that_was_held() {
    // **Which core is read is fixed before the mask, not after.** The preemption this asserts on
    // may migrate this thread, and `preemptions_here()` would then read whichever core it landed
    // on: a counter that was never the one being watched. `preemptions_on` pins it to the core
    // that held the tick.
    let core = crate::cpu::id();
    let count = || sched::preemptions_on(core);
    let before = count();

    let was_enabled = crate::arch::interrupts::disable();
    crate::arch::timer::spin_for(three_tick_periods());
    crate::arch::interrupts::restore(was_enabled);

    crate::arch::timer::spin_for(three_tick_periods() / 3);
    let after = count();

    assert!(
        after > before,
        "no preemption was taken in the tick period after unmasking, so the ticks held across the \
         mask were dropped rather than deferred; masking a benchmark window would then be \
         starving the scheduler rather than excluding it",
    );
}
