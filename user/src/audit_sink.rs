//! **Drains `login`'s audit trail so the service never blocks on it** (milestone 49's terminal
//! update, wiring the login stack into the real interactive boot).
//!
//! `user/src/login.rs` sends one [`login_proto::ATTRIBUTED`] message per successful login on its
//! `AUDIT` endpoint, and that send is a plain, blocking rendezvous (`crates/ipc`'s own model): it
//! does not return until something receives it. Nothing in a real interactive boot was reading that
//! endpoint before this program existed, so `login`'s very first successful login would have parked
//! its whole thread inside that `send` forever, unable to reclaim the connection it just served or
//! accept the next one. This process is the receiver, on `job_undertaker`'s own pattern: one
//! endpoint capability, `READ`, and nothing else.
//!
//! # What it does with a message once it has one
//!
//! Discards it. This is the honest, considered choice for this slice, not an oversight: printing
//! the record (who logged in, in order) would need a `WRITE` view of the terminal, and handing that
//! out to a *third* process at exactly the moment milestone 49's own terminal update is building
//! careful single-holder semantics for the interactive terminal (`user/src/login.rs`'s "The
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
//! Name: provisional, minted 2026-08-27 for milestone 49's boot-wiring update and not yet put to
//! calef. `audit_sink`, a noun, on `byte_sink_proto`/`terminal_sink_caretaker`'s own use of "sink"
//! as this tree's term for "the end of a stream nobody reads further", which is exactly this
//! program's role for `login`'s own audit stream.

#![no_std]
// Program entry points, not the crates/ library surface milestone 68's ratchet tracks
// (DECISIONS §107): each `[[bin]]` is its own crate root with one `_start`, and 58 of them
// documenting an OS-facing ABI entry point is not what the lint is for.
#![allow(missing_docs)]
#![no_main]

use user_rt::recv;

/// `login`'s own `AUDIT` endpoint, `READ`.
const AUDIT: u64 = 0;

#[unsafe(no_mangle)]
pub extern "C" fn _start(_a0: u64, _a1: u64, _a2: u64) -> ! {
    loop {
        recv(AUDIT);
    }
}

user_rt::panic_handler!();
