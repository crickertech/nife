//! The heeder: a long-running job that heeds the cooperative interrupt (DECISIONS §24).
//!
//! It works forever, one small unit at a time, and between units it reads a single word in a page
//! it shares with the shell: the interrupt flag. When the shell sets it (on the first `^C`), the
//! heeder cleans up, records that it stopped gracefully, and exits. This is the cooperative tier of
//! DECISIONS §24 made visible: the first `^C` asks the job to stop, and a job that listens does.
//!
//! Why a shared word and not an endpoint: a running computation cannot poll an endpoint (there is
//! no non-blocking receive) and cannot block on one without stalling the work the user wants to
//! interrupt. So the signal is memory the heeder reads with a plain load between units. See
//! `grant_plan::job_page_frame` and notes/grant-expression.md.
//!
//! # The heeder's world
//!
//! - the shared job frame, mapped read/write at [`JOB_PAGE_FRAME_VA`] (init maps it; the shell holds the
//!   other view). The heeder reads [`job_page_frame::INTERRUPT`] and writes the rest. No capabilities: it
//!   touches only this page and exits. Its whole authority is one shared page.
//!
//! Name: provisional. Introduced 2026-07-28 with the cooperative interrupt tier (DECISIONS §24),
//! and cited as an established family member by milestone 63 without ever being argued for
//! itself. The case. §24 splits interrupt handling into a cooperative tier and a forcible one,
//! and this program is the whole of the cooperative one: it reads the interrupt flag between
//! units of work and stops when the flag is set. So the name says the exact property the tier
//! turns on, which is that the job *heeds* rather than that it can be stopped. The agent noun
//! follows 63's family rule. Its counterpart is `spinner`, and the pair is legible together in a
//! way `cooperative_job` and `runaway_job` would not be, because the distinction is behaviour
//! under one signal rather than a kind of job.

#![no_std]
// Program entry points, not the crates/ library surface milestone 68's ratchet tracks
// (DECISIONS §107): each `[[bin]]` is its own crate root with one `_start`, and 58 of them
// documenting an OS-facing ABI entry point is not what the lint is for.
#![allow(missing_docs)]
#![no_main]

use grant_plan::job_page_frame;
use user_rt::exit;
use user_rt::mapped_window::{self, MappedWindow};

/// Where init maps the shared job frame in the child's address space. Must match the shell/init
/// wiring (below the ELF load address `0x40_0000` and the stack).
const JOB_PAGE_FRAME_VA: usize = 0x0030_0000;

// SAFETY: init mapped one page read/write at JOB_PAGE_FRAME_VA before this program runs (milestone 139
// round 2; see `user_rt::mapped_window`, which is what collapsed the hand-rolled read_volatile/
// write_volatile pair below).
const WINDOW: MappedWindow =
    unsafe { MappedWindow::new(JOB_PAGE_FRAME_VA as u64, mapped_window::PAGE) };

fn load(off: usize) -> u64 {
    WINDOW.read(off as u64)
}

fn store(off: usize, v: u64) {
    WINDOW.write(off as u64, v);
}

#[unsafe(no_mangle)]
pub extern "C" fn _start(_x0: u64, _x1: u64, _x2: u64) -> ! {
    let mut beats: u64 = 0;
    loop {
        // One unit of work. The heartbeat is liveness the shell can show; the spin makes a unit
        // take long enough that the interrupt is noticed promptly, not after a million iterations.
        beats = beats.wrapping_add(1);
        store(job_page_frame::HEARTBEAT, beats);
        for _ in 0..4096 {
            core::hint::spin_loop();
        }
        // Notice the cooperative interrupt between units.
        if load(job_page_frame::INTERRUPT) != 0 {
            store(job_page_frame::STATUS, job_page_frame::STATUS_INTERRUPTED);
            store(job_page_frame::DONE, 1); // tell the shell we are stopping cleanly, then leave
            exit();
        }
    }
}

user_rt::panic_handler!();
