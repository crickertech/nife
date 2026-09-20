//! **The floating-point and vector registers belong to one thread at a time.**
//!
//! The policy half of the per-architecture `arch::fp` modules, and the only thing that calls them.
//! Three ISAs, three completely different control registers, one rule:
//!
//! > **The FP/SIMD register file holds the running thread's data, or it holds
//! > [`FpState::INITIAL`]. It never holds a thread's data while a different thread runs.**
//!
//! Everything below is that sentence made true at the one instant it could stop being true, which
//! is the context switch. [`hand_over`] is called by the outgoing thread, on the outgoing thread's
//! stack, with interrupts masked and the scheduler's lock already released, immediately before
//! `arch::switch_to`. The switch itself is untouched: it still saves a calling convention's
//! callee-saved set and nothing else, and this runs beside it rather than inside it.
//!
//! # Why eager, and why the word "lazy" is a warning here
//!
//! The cheap-looking scheme is to leave the outgoing thread's registers in the hardware, trap when
//! the incoming thread touches them, and only then swap. It is cheap because a thread that never
//! touches FP never pays, and it is what several kernels did until 2018, when **LazyFP
//! (CVE-2018-3665)** showed that on x86 the trap is not a boundary: speculative execution past the
//! `#NM` reads the registers the trap was supposed to protect, and one thread recovers another's
//! AES round keys. The mechanism was `CR0.TS`, which is the very bit this tree's x86 half uses.
//!
//! So `CR0.TS`, `CPACR_EL1.FPEN` and `sstatus.FS` are used here to answer *"has this thread ever
//! wanted FP"*, which is a performance question, and never to answer *"whose data is in the
//! registers"*, which is a confidentiality one. The second question is answered by the registers
//! themselves always being right. That is what the `else` arm of [`hand_over`] is for: when the
//! outgoing thread had FP and the incoming one does not, the file is scrubbed to
//! [`FpState::INITIAL`] before the trap is re-armed, so there is nothing behind the trap to leak.
//!
//! # What it costs when nobody uses floating point, which is today
//!
//! Every userspace target in `targets/` is soft-float and the kernel is built `softfloat` too, so
//! no thread in this tree has ever executed an FP instruction. For those threads [`hand_over`] is
//! two loads and two predictable branches, and touches no control register at all: the expensive
//! half is behind `live`, and `live` is false. Milestone 447's block has the measured numbers.
//!
//! # BUGS
//!
//! - **`live` never clears.** A thread that used FP once saves and restores 512 bytes on every
//!   switch for the rest of its life. RISC-V's `sstatus.FS` could answer the narrower question
//!   ("have the registers been written since they were loaded") in hardware and this does not use
//!   it, because a uniform rule across three ISAs was judged worth more than one ISA's optimisation
//!   while no workload exists to measure the difference on. When one does, that is the first thing
//!   to try.
//! - **A migrating thread carries its register file through memory.** The save happens on the core
//!   the thread is leaving and the restore on the core it arrives at, which is correct and is also
//!   a kilobyte of traffic that a same-core switch does not pay. Nothing measures it.

use core::sync::atomic::{AtomicUsize, Ordering};

use crate::arch::fp::FpState;

/// **How many threads have asked for the floating-point unit.**
///
/// One per thread, ever, because `live` never clears and so the first-use trap is taken exactly
/// once. **Zero on every shipping boot**, and that number is the point of having it: it says that
/// no program in a soft-float userspace has ever needed the register file this kernel now carries
/// for it, which is the measurement milestone 447's target-flip proposal is weighed against.
pub static ENABLES: AtomicUsize = AtomicUsize::new(0);

/// **Move the register file from the outgoing thread to the incoming one.**
///
/// Called immediately before `arch::switch_to`, from the outgoing thread. Four cases, and the
/// fourth is the one that matters:
///
/// | outgoing | incoming | what happens |
/// |---|---|---|
/// | not live | not live | nothing. Two loads and two branches, which is every switch in this tree today. |
/// | live | live | save, restore. The registers never hold anything that is not one of the two threads' own. |
/// | not live | live | enable, restore. |
/// | live | not live | save, **scrub to [`FpState::INITIAL`]**, disable. |
///
/// That last row is the confidentiality one. Disabling without scrubbing is what LazyFP taught us
/// not to do (see this module's header), and it is the one arm a test can catch: two threads doing
/// FP work concurrently is the *second* row, and it fails loudly against a kernel with no save
/// path; the fourth row fails silently, so it is proved by reading the registers back after a
/// thread that used them has run.
///
/// # Safety
/// `prev` and `next` must name live, distinct `FpState`s belonging to the outgoing and incoming
/// threads. The caller holds them by the same argument `sched::schedule` makes for `prev_slot` and
/// `next_ctx`: both threads are pinned (the outgoing one is running, the incoming one is `Running`
/// with `on_cpu` set), interrupts are masked, and nothing can reap either.
pub unsafe fn hand_over(prev: *mut FpState, next: *const FpState) {
    // SAFETY: the caller's, forwarded. Both reads are of one `u64` in a pinned allocation.
    let prev_live = unsafe { (*prev).live() };
    let next_live = unsafe { (*next).live() };

    if !prev_live && !next_live {
        // The common case, and the only one on a machine where nothing uses floating point. Out
        // first so the branch predictor and the reader both meet it before the machinery.
        return;
    }

    if prev_live {
        // SAFETY: the caller's. FP is enabled on this core, because this core is running `prev` and
        // `prev.live` is exactly the condition under which the enable was installed.
        unsafe { crate::arch::fp::save(prev) };
    }

    if next_live {
        if !prev_live {
            crate::arch::fp::enable();
        }
        // SAFETY: the caller's, and FP is enabled either way by the line above or by `prev`.
        unsafe { crate::arch::fp::restore(next) };
    } else {
        // `prev_live` here, because the both-false case returned above. Scrub before disabling: the
        // trap is a performance mechanism, not a wall.
        //
        // SAFETY: `INITIAL` is a `'static` const, and FP is enabled (we just saved through it).
        unsafe { crate::arch::fp::restore(&FpState::INITIAL) };
        crate::arch::fp::disable();
    }
}

/// **The running thread just tried to use the FP unit for the first time. Let it.**
///
/// The body of every architecture's first-use trap: `CPACR_EL1.FPEN` at EL0 or EL1 on aarch64, an
/// illegal instruction under `sstatus.FS == Off` on RISC-V, `#NM` under `CR0.TS` on x86_64. Returns
/// false when there is no scheduler or no current thread to record the fact against, which the
/// caller must treat as a fault: returning to the trapping instruction with nothing changed would
/// retake the same trap forever.
///
/// **It installs [`FpState::INITIAL`] rather than leaving the registers alone**, and that is not
/// hygiene. [`hand_over`] guarantees the file holds `INITIAL` whenever a non-live thread runs, but
/// only *after* a live thread has run at least once on this core: before that, the registers hold
/// whatever the machine's reset left, which is a defined value on no architecture this kernel
/// targets. One `restore` per thread, ever, closes that.
///
/// The `live` flag is set under `IPC_TABLES`, exactly as `sched::grant_cycle_counter_to_current`
/// writes its field, and the lock is safe here for the reason it is safe in `syscall::dispatch`:
/// this is a trap from a thread that was running, so this core cannot already hold it.
pub fn enable_for_current() -> bool {
    if !crate::sched::mark_current_fp_live() {
        return false;
    }
    crate::arch::fp::enable();
    // SAFETY: `INITIAL` is a `'static` const and FP is enabled on the line above.
    unsafe { crate::arch::fp::restore(&FpState::INITIAL) };
    ENABLES.fetch_add(1, Ordering::Relaxed);
    true
}

/// A distinct 64-bit value per register, so a test can tell one thread's register file from
/// another's and from a scrubbed one. The multiplier is the golden-ratio constant `xxHash` and
/// `SplitMix64` both use; nothing here needs it to be good, only for neighbouring seeds and
/// neighbouring indices to land nowhere near each other, so that a save which copies the right
/// number of bytes to the wrong offset fails rather than passes.
#[cfg(test)]
pub fn register_pattern(seed: u64, index: usize) -> u64 {
    seed.wrapping_mul(0x9E37_79B9_7F4A_7C15)
        .wrapping_add(index as u64 + 1)
}

/// **Take the first-use trap, then fill every vector register with this thread's pattern.**
///
/// The `touch` is separate from the `restore` and has to be: `restore` is thirty-two vector loads
/// and its safety contract says the unit is already open, so provoking the enable trap with it
/// would be asking a function to fault on its first instruction as a matter of routine. One
/// throwaway instruction does the job where a reader can see it.
#[cfg(test)]
pub fn load_pattern(seed: u64) {
    crate::arch::fp::touch();
    let mut state = FpState::INITIAL;
    state.set_pattern(seed);
    // SAFETY: `state` is a local, aligned by its own `repr`, and `touch` above left the unit open.
    unsafe { crate::arch::fp::restore(&state) };
}

/// **Read the live register file without changing what this core is doing.**
///
/// Deliberately *not* [`enable_for_current`]: that one installs [`FpState::INITIAL`], which would
/// destroy the very thing a test wants to look at, and marks the calling thread `live`, which would
/// change the case [`hand_over`] takes next. This borrows the unit and hands it back exactly as
/// found, with interrupts masked so no switch can land in the middle and find the core in a state
/// its `live` flag does not describe.
#[cfg(test)]
pub fn peek_registers() -> FpState {
    let was_enabled_interrupts = crate::arch::interrupts::disable();
    let was_enabled_fp = crate::arch::fp::is_enabled();
    if !was_enabled_fp {
        crate::arch::fp::enable();
    }
    let mut state = FpState::INITIAL;
    // SAFETY: `state` is a local and the unit is open either way by the line above.
    unsafe { crate::arch::fp::save(&mut state) };
    if !was_enabled_fp {
        crate::arch::fp::disable();
    }
    crate::arch::interrupts::restore(was_enabled_interrupts);
    state
}

#[cfg(test)]
mod tests {
    use core::sync::atomic::AtomicUsize;

    use super::*;

    /// Run `body` with the vector registers borrowed and handed back exactly as found.
    ///
    /// Test order is not fixed, so a test that drives [`hand_over`] by hand cannot assume the
    /// calling thread is not already `live`: leaving the unit disabled under a thread whose flag
    /// says otherwise would make the *next* switch call `save` into a trapping unit, from inside
    /// the scheduler, with interrupts masked. Interrupts stay masked for the whole body so no
    /// switch observes the core mid-borrow.
    fn with_borrowed_registers(body: impl FnOnce()) {
        let was_enabled_interrupts = crate::arch::interrupts::disable();
        let was_enabled_fp = crate::arch::fp::is_enabled();
        let saved = if was_enabled_fp {
            let mut state = FpState::INITIAL;
            // SAFETY: a local, and the unit is open.
            unsafe { crate::arch::fp::save(&mut state) };
            Some(state)
        } else {
            None
        };

        body();

        match saved {
            Some(state) => {
                crate::arch::fp::enable();
                // SAFETY: a local, and the unit is open by the line above.
                unsafe { crate::arch::fp::restore(&state) };
            }
            None => crate::arch::fp::disable(),
        }
        crate::arch::interrupts::restore(was_enabled_interrupts);
    }

    /// **The whole milestone, in one claim**: two threads doing vector work on the same core do not
    /// see each other's registers.
    ///
    /// It fails against a kernel without a save path, which is the only property that makes it
    /// worth having. Both threads are placed on **this** core with `spawn_on` rather than by
    /// `spawn`, because DECISIONS §28 places by power-of-two-choices and two threads on two cores
    /// have two register files: the test would pass without the kernel doing anything at all. On
    /// one core they interleave over one file, and this thread (which never touches FP) sits
    /// between them, so the scrub-and-disable arm of [`hand_over`] runs between every pair of turns
    /// as well.
    #[test_case]
    fn two_threads_doing_vector_work_do_not_see_each_others_registers() {
        /// Threads that finished their turns without finding a foreign value.
        static INTACT: AtomicUsize = AtomicUsize::new(0);
        /// Threads that found one. Counted rather than asserted inside the thread, because an
        /// assertion there panics on a core the harness is not watching.
        static CLOBBERED: AtomicUsize = AtomicUsize::new(0);
        /// How many turns each thread takes. High enough that the timer has preempted both many
        /// times over, low enough to be milliseconds.
        const TURNS: usize = 200;

        INTACT.store(0, Ordering::Relaxed);
        CLOBBERED.store(0, Ordering::Relaxed);

        let here = crate::cpu::id();
        for seed in [0x1234_5678_9abc_def0_u64, 0x0fed_cba9_8765_4321_u64] {
            crate::sched::spawn_on(here, move || {
                load_pattern(seed);
                for _ in 0..TURNS {
                    crate::sched::yield_now();
                    if !peek_registers().has_pattern(seed) {
                        CLOBBERED.fetch_add(1, Ordering::Release);
                        return;
                    }
                }
                INTACT.fetch_add(1, Ordering::Release);
            })
            .expect("could not spawn a vector-arithmetic thread");
        }

        let finished = || INTACT.load(Ordering::Acquire) + CLOBBERED.load(Ordering::Acquire) == 2;
        let deadline = crate::arch::timer::now() + 2 * crate::arch::timer::frequency();
        while crate::arch::timer::now() < deadline && !finished() {
            crate::sched::yield_now();
        }

        assert!(finished(), "the vector-arithmetic threads never finished");
        assert_eq!(
            CLOBBERED.load(Ordering::Acquire),
            0,
            "a thread found another thread's values in its own vector registers",
        );
    }

    /// **The arm a concurrency test cannot see**: handing the registers to a thread that has none
    /// leaves nothing of the previous thread's behind.
    ///
    /// Two threads that both use FP catch a missing save loudly. A thread that *stops* using FP
    /// while its values sit in a register file nobody is scrubbing fails silently, forever, and is
    /// what CVE-2018-3665 was. So this drives [`hand_over`]'s fourth row directly, on synthetic
    /// states, and reads the hardware back with [`peek_registers`], which deliberately does not go
    /// through the enable trap (that would install `INITIAL` itself and prove nothing).
    #[test_case]
    fn a_thread_with_no_vector_state_finds_the_registers_scrubbed() {
        with_borrowed_registers(|| {
            let mut departing = FpState::INITIAL;
            departing.set_live();
            departing.set_pattern(0xdead_beef_cafe_f00d);
            let arriving = FpState::INITIAL;
            let mut idle = FpState::INITIAL;

            // Give the registers to the thread that has state, so they hold something worth
            // leaking. SAFETY: three locals, distinct, and interrupts are masked.
            unsafe { hand_over(&mut idle, &departing) };
            assert!(
                peek_registers().has_pattern(0xdead_beef_cafe_f00d),
                "hand_over did not install the incoming thread's registers",
            );

            // And take them away again, to a thread that has never used the unit.
            // SAFETY: as above.
            unsafe { hand_over(&mut departing, &arriving) };
            assert!(
                peek_registers().is_scrubbed(),
                "a thread's vector registers survived into a thread that has none",
            );
            assert!(
                !crate::arch::fp::is_enabled(),
                "the unit was left open for a thread that has never asked for it",
            );
        });
    }

    /// The save half, on its own: what a thread had in its registers comes back out into its
    /// [`FpState`], lane for lane.
    ///
    /// Separate from the concurrency test because a save that copies the right bytes to the *wrong
    /// lane* still passes that one whenever both threads are preempted at matching offsets.
    /// [`register_pattern`] is chosen so that a lane swap fails here.
    #[test_case]
    fn the_whole_register_file_survives_a_save_and_a_restore() {
        const SEED: u64 = 0x5555_aaaa_5555_aaaa;
        with_borrowed_registers(|| {
            let mut written = FpState::INITIAL;
            written.set_live();
            written.set_pattern(SEED);
            let mut idle = FpState::INITIAL;
            // SAFETY: two distinct locals, and interrupts are masked for the whole body.
            unsafe { hand_over(&mut idle, &written) };

            let read_back = peek_registers();
            for index in 0..32 {
                assert_eq!(
                    read_back.lane_low(index),
                    register_pattern(SEED, index),
                    "vector register {index} did not survive the round trip",
                );
            }

            // Put the unit back shut, so `with_borrowed_registers` restores from a known state.
            let arriving = FpState::INITIAL;
            // SAFETY: as above.
            unsafe { hand_over(&mut written, &arriving) };
        });
    }

    /// A thread that has never executed an FP instruction has no vector state to move, and
    /// [`hand_over`] between two such threads touches no control register at all.
    ///
    /// This is the case every switch in this tree actually takes, so it is worth asserting rather
    /// than assuming: if it ever stopped being true the cost would be a kilobyte of memory traffic
    /// per switch and the only symptom would be a slower benchmark.
    #[test_case]
    fn two_threads_that_never_used_the_unit_leave_it_shut() {
        with_borrowed_registers(|| {
            crate::arch::fp::disable();
            let mut one = FpState::INITIAL;
            let two = FpState::INITIAL;
            assert!(!one.live() && !two.live());
            // SAFETY: two distinct locals, interrupts masked.
            unsafe { hand_over(&mut one, &two) };
            assert!(
                !crate::arch::fp::is_enabled(),
                "a switch between two soft-float threads opened the floating-point unit",
            );
        });
    }
}
