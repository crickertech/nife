//! **The supervised sub-server that dies once** (milestone 22 phase B.2).
//!
//! The thing the tree exists to keep alive. It is handed exactly one capability, a WRITE view of a
//! report endpoint, and one scalar: which attempt it is. Attempt 0 crashes on purpose; any later
//! attempt does its work and exits cleanly.
//!
//! That shape is deliberate, because it exercises **both** halves of the fault endpoint (DECISIONS
//! §26) against a real supervisor: a crash that must be restarted, and a clean exit that must not be.
//! A program that only crashed would prove the restart but not the policy.
//!
//! Name: provisional, and the second of the two adjectives on this surface (`chatty` is the
//! other, from the day before). Introduced 2026-07-29 as a supervision fixture: attempt 0 crashes
//! on purpose and any later attempt exits cleanly, which exercises both halves of the fault
//! endpoint (DECISIONS §26) against a real supervisor. Nothing records the choice. The case, and
//! it is weaker than `chatty`'s. "Flaky" is the field's established word for a test that fails
//! intermittently, and this program does not fail intermittently: it fails deterministically,
//! once, by attempt number. So the name borrows recognition it then contradicts, which is the
//! opposite of what the protected class is for. A reader who knows the word arrives with the
//! wrong model. `dies_once`, which is what the header calls it, is the shape a replacement would
//! take.

#![no_std]
// Program entry points, not the crates/ library surface milestone 68's ratchet tracks
// (DECISIONS §107): each `[[bin]]` is its own crate root with one `_start`, and 58 of them
// documenting an OS-facing ABI entry point is not what the lint is for.
#![allow(missing_docs)]
#![no_main]

// Each binary in the tree compiles the shared module but uses a different slice of it (the sub-server
// builds nothing, the supervisor holds no memory), so the unused halves are expected, not dead.

use supervision_proto::REPORT_SERVER_RAN;
use user_rt::send;

/// Our one capability.
const REPORT: u64 = 0;

/// The unmapped address the first attempt loads from. Distinctive, so the fault address in the
/// supervisor's message is visibly real rather than a zero placeholder.
const BAD_ADDR: u64 = 0x00A5_0000;

#[unsafe(no_mangle)]
pub extern "C" fn _start(_a0: u64, attempt: u64, _a2: u64) -> ! {
    send(REPORT, REPORT_SERVER_RAN, attempt, 0);

    if attempt == 0 {
        // Crash. A real load from an address nothing maps, so the kernel's fault path (not a
        // cooperative exit) is what the supervisor hears about.
        // SAFETY: deliberately unsafe. This is the fault under test.
        unsafe { core::ptr::read_volatile(BAD_ADDR as *const u64) };
    }

    // The restart did its work. Exit cleanly, which the supervisor must read as "finished."
    user_rt::exit()
}

user_rt::panic_handler!();
