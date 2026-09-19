//! **Drains `login`'s audit trail so the service never blocks on it** (milestone 49's terminal
//! update, wiring the login stack into the real interactive boot).
//!
//! `components/src/login.rs` sends one [`login_protocol::ATTRIBUTED`] message per successful login on its
//! `AUDIT` endpoint, and that send is a plain, blocking rendezvous
//! (`crates/inter_process_communication`'s own model): it does not return until something receives
//! it. Nothing in a real interactive boot was reading that endpoint before this program existed, so
//! `login`'s very first successful login would have parked its whole thread inside that `send`
//! forever, unable to reclaim the connection it just served or accept the next one. This process
//! is the receiver, on `job_undertaker`'s own pattern: one endpoint capability, `READ`, and nothing
//! else.
//!
//! # What it does with a message once it has one
//!
//! Discards it. This is the honest, considered choice for this slice, not an oversight: printing
//! the record (who logged in, in order) would need a `WRITE` view of the terminal, and handing that
//! out to a *third* process at exactly the moment milestone 49's own terminal update is building
//! careful single-holder semantics for the interactive terminal (`components/src/login.rs`'s "The
//! terminal: single-session, deny cleanly") is a real complication this program's whole reason for
//! existing (unblocking `login`) does not need. See BUGS.
//!
//! # Capability contract
//!
//! - slot [`AUDIT`]: `READ` on `login`'s own `AUDIT` endpoint. Nothing else: this process cannot
//!   build anything, allocate a page, or reach any other component's memory.
//!
//! # BUGS
//!
//! **The audit trail this program drains is not surfaced anywhere.** DECISIONS §109 names two
//! properties: a server establishing a channel, and (separately) a server logging which channel a
//! later request arrived on. `login.rs`'s own BUGS already notes it only builds the first half; this
//! program is what makes the first half *safe to leave running* at a real boot, not a consumer of
//! the record it drains. A deployment that wants the attribution trail actually read (to a log file,
//! to the console, anywhere) needs a different, real consumer in this program's place, which is
//! follow-on work and not attempted here.
//!
//! Name: provisional, and ruled: calef ruled **`login_audit_receiver`** on 2026-09-13, working the
//! unratified worklist. The block stays `provisional` because the ratified name is not this file's
//! until the rename is performed, and until then `audit_sink` belongs on the worklist rather than
//! off it. Minted 2026-08-27 for milestone 49's boot-wiring update.
//!
//! **And it carries the name it becomes.** calef, ruling it: *"We would probably rename to
//! `login_audit_recorder` when it does record the login audit."* That is the condition written
//! down rather than left to whoever notices, the same shape §71 asks of a `BUGS` entry that names
//! what would promote it. Whoever gives this program a `WRITE` view and makes it keep records
//! performs that rename in the same change.
//!
//! **Why `audit_sink` went.** Half the name was honest and half was not. "Sink" is already this
//! tree's word for the end of a stream nobody reads further (`byte_sink_protocol`,
//! `terminal_sink_caretaker`), and that is exactly this program's role. But an audit trail that is
//! discarded is not an audit trail, and a reader meeting `audit_sink` in a process listing would
//! reasonably conclude the system records logins somewhere. Nothing does. That is the fault
//! `flaky` was renamed for the same day: borrowed recognition the program contradicts.
//!
//! Refused `login_audit_discarder`, which is what the program does today and was the first
//! recommendation, because it names the disposal rather than the role and would itself need
//! renaming the moment the records are kept; `login_audit_receiver` survives that day and
//! `login_audit_recorder` is the honest successor. Refused `login_audit_sink`, which fixes whose
//! audit it is and keeps the promise that something is audited. Refused `audit_drain`, the same
//! defect again, and "drain" is spent in this tree on `scripts/merge-drain.sh`.

#![no_std]
// Program entry points, not the crates/ library surface milestone 68's ratchet tracks
// (DECISIONS §107): each `[[bin]]` is its own crate root with one `_start`, and 58 of them
// documenting an OS-facing ABI entry point is not what the lint is for.
#![allow(missing_docs)]
#![no_main]

use user_mode_runtime::recv;

/// `login`'s own `AUDIT` endpoint, `READ`.
const AUDIT: u64 = 0;

#[unsafe(no_mangle)]
pub extern "C" fn _start(_a0: u64, _a1: u64, _a2: u64) -> ! {
    loop {
        recv(AUDIT);
    }
}

user_mode_runtime::panic_handler!();
