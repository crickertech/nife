//! **The one page the soak workload and the kernel share**, and the arithmetic they both do to it
//! (milestone 219).
//!
//! The soak is a user program and the detection is in the kernel, which is the division milestone
//! 219 was briefed with and the reason this crate exists at all: the workload's own progress is a
//! fact only the workload knows, and the kernel has to read it every heartbeat without asking. A
//! shared page is the cheapest way for it to, and it costs the workload one store per round trip
//! rather than a syscall.
//!
//! Rule 7 of `AGENTS.md` ("anything two binaries must agree on is a crate") is why this is a crate
//! and not two copies of a `const`. The kernel's `soak` module writes nothing here; it only reads,
//! and the workload only writes its own slot. That asymmetry is what makes the page safe without a
//! lock: each `u64` has exactly one writer, and a `u64` store does not tear on any of the three
//! architectures.
//!
//! Milestone 221 added the third array. A waiter blocks on the tick route the soak build signals
//! from `sched::on_tick` and counts its wakes here, apart from the round-trip total, because the
//! two are not the same quantity and a reader comparing runs must not have to guess which they are
//! holding.
//!
//! # What is NOT in here
//!
//! The anomaly counters (refused wakes, deferred wakes, remote placements, steals) are the
//! *kernel's* numbers and stay in the kernel, in `sched`'s per-core trace counters. A user program
//! that could write them could also lie about them, and a soak whose tripwire the subject can
//! reach is not a tripwire.
//!
//! Name: provisional, this lane's coinage (2026-09-01, milestone 219), and calef's call. A noun for
//! the thing it describes (the one page the soak workload and the kernel share), in the shape the
//! tree already uses for a layout two binaries agree on: `block_roster`, `clock_proto`,
//! `timebase_proto`, `grant_plan::job_page_frame`. `soak_proto` was considered and refused, because
//! nothing here is a wire protocol: it is a memory layout, and the `_proto` suffix in this tree
//! names request/reply vocabularies. `soak_counters` was refused for naming half the content.
//!
//! # BUGS
//!
//! - **A worker that dies stops writing and looks exactly like a worker that wedged.** Both are
//!   failures, both are caught by the same stall check, and the kernel's report cannot tell them
//!   apart from this page alone; the thread dump it prints on failure can.

#![no_std]

/// Where the kernel maps the page in every worker's address space.
///
/// Below the ELF load address (`0x40_0000`) and below the stack (`0x50_0000`), the same
/// neighbourhood `grant_plan::job_page_frame`'s window uses, so it collides with neither.
pub const VA: u64 = 0x0020_0000;

/// The page's size, and the ceiling on everything below.
pub const PAGE: u64 = 4096;

/// How many workers the page has room for.
///
/// Three `u64` per worker costs 1.5 KiB of the 4 KiB page at this ceiling, which is the headroom
/// the host test below checks rather than assumes. It is far above any machine this project has:
/// the kernel builds one group of six per online core, so 64 covers a ten-core board with room to
/// spare, and the cap exists so that the kernel's own fixed-size arrays have a bound rather than
/// because anything is near it.
pub const MAX_WORKERS: usize = 64;

/// Byte offset of worker `i`'s completed-round-trip counter.
///
/// Written by the worker, read by the kernel. Monotone, and never reset: the whole point is that a
/// later run can be compared against an earlier one, which needs a total rather than a rate.
#[must_use]
pub const fn rounds(i: usize) -> u64 {
    (i as u64) * 8
}

/// Byte offset of worker `i`'s wrong-answer counter.
///
/// A caller that gets back something other than [`answer`] of what it sent has caught the IPC path
/// delivering the wrong message, which is a stronger finding than a hang and the one this workload
/// would most like to produce. Nonzero here fails the run.
#[must_use]
pub const fn mismatches(i: usize) -> u64 {
    ((MAX_WORKERS + i) as u64) * 8
}

/// Byte offset of worker `i`'s tick-wake counter (milestone 221).
///
/// Written by a waiter, read by the kernel, and **separate from [`rounds`] on purpose**: a waiter
/// completes no IPC round trip, so folding its wakes into the round-trip total would make the one
/// number a later run is compared against mean two different things depending on which build
/// produced it. It is its own field in the heartbeat for the same reason.
///
/// A waiter leaves [`rounds`] at zero and a caller, responder or grinder leaves this at zero, so
/// the kernel's stall check can ask one question of every worker (did either counter move?)
/// without holding a table of who plays which role.
#[must_use]
pub const fn wakes(i: usize) -> u64 {
    ((2 * MAX_WORKERS + i) as u64) * 8
}

/// The transform a responder applies to the word a caller sent.
///
/// It exists so that a reply carrying *some* value is not mistaken for a reply carrying the *right*
/// value. A stale mailbox, a wake completed against the wrong parked receive, or a reply delivered
/// to the wrong caller all produce a word; only the correct rendezvous produces this one. The
/// constant is arbitrary and the multiply-xor is chosen so that neighbouring sequence numbers give
/// answers that share no low bits, which a truncated or half-written word would not survive.
#[must_use]
pub const fn answer(sent: u64) -> u64 {
    (sent.wrapping_mul(0x9E37_79B9_7F4A_7C15)) ^ 0x5851_F42D_4C95_7F2D
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **Every offset the page defines is inside the page.** One test, because the failure it
    /// prevents is a worker writing past its mapping and taking a fault the kernel would report as
    /// a soak failure with a misleading cause.
    #[test]
    fn every_offset_fits_in_the_page() {
        for i in 0..MAX_WORKERS {
            assert!(rounds(i) + 8 <= PAGE, "rounds({i}) escapes the page");
            assert!(
                mismatches(i) + 8 <= PAGE,
                "mismatches({i}) escapes the page"
            );
            assert!(wakes(i) + 8 <= PAGE, "wakes({i}) escapes the page");
        }
    }

    /// The three counter arrays do not overlap, which a hand-written offset scheme gets wrong
    /// exactly once and then reports as a workload that never makes progress.
    #[test]
    fn the_three_counter_arrays_are_disjoint() {
        assert!(rounds(MAX_WORKERS - 1) + 8 <= mismatches(0));
        assert!(mismatches(MAX_WORKERS - 1) + 8 <= wakes(0));
    }

    /// **Neighbouring sequence numbers give unrelated answers**, which is the property that makes a
    /// wrong reply detectable rather than plausible. Checked against the two failures worth
    /// catching: an off-by-one delivery and a zeroed word.
    #[test]
    fn a_neighbouring_sequence_number_does_not_give_a_similar_answer() {
        for seq in [0u64, 1, 2, 1000, u64::MAX / 2] {
            assert_ne!(answer(seq), answer(seq.wrapping_add(1)));
            assert_ne!(answer(seq), 0, "a zeroed reply word must never be correct");
            assert_ne!(answer(seq), seq, "an echoed reply must never be correct");
        }
    }
}
