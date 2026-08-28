//! **`pgrep`: name the members of a domain that match, and do nothing to them** (milestone 126,
//! notes/process-view.md).
//!
//! The whole program is: walk one supervision domain with `abi::rendezvous::SURVEY`, filter it, then
//! say what matched on one stream and what went wrong on the other. The walk is `crates/ps` and the
//! filter is `crates/pgrep`, both host-tested in milliseconds; what lives here is the syscall and
//! the two sinks.
//!
//! Name: recorded (milestone 126). `pgrep` is upstream's, which the naming tenet calls the best name
//! available and not one to spend a rename on; sharing it with `crates/pgrep` is the crate-and-
//! program pair the same tenet describes, as `ps` and `crates/ps` already are. That rule plus the
//! standard term produces this name, so calef ruled on the rules and never on this program.
//!
//! # There is no `pkill`, and that is the demonstration
//!
//! On Unix these are one lookup with two endings, and the split between them is convention:
//! `kill(pid)` turns a **name** into an **action**, so anything that can find a process can end it.
//!
//! **A domain names its members and does not act on them** (calef, 2026-08-17). This program holds a
//! supervision endpoint carrying `abi::rights::ENUMERATE` and nothing else. It cannot receive a
//! death on it, cannot reap through it, and there is no method anywhere that turns a tid into
//! authority over the thread it names: `abi::thread_control_block` has no `DESTROY`, and killing a live child is
//! `abi::memory_region::DESTROY` on the region it was built from, which its spawner holds (DECISIONS §24).
//!
//! So the honest statement of what this port achieves is asymmetric: **`caps pgrep` prints a scope
//! and there is no `caps pkill` to print beside it.** The side-by-side comparison milestone 126
//! originally promised does not exist, because the second program cannot. A reader who expects
//! `kill` to be a program here will not find one.
//!
//! # Capability contract
//!
//! | slot | what | why |
//! |---|---|---|
//! | 0 | the output sink, `WRITE` | where the tids go |
//! | 7 | the process domain, `ENUMERATE` | the supervision endpoint whose members it may **name** |
//! | 8 | the diagnostics sink, `WRITE` | where a refusal goes, so `>` cannot swallow it |
//!
//! **Exactly `ps`'s three, and not one right more.** That is the load-bearing half of the whole
//! argument: a program that could act on what it lists would make the demonstration a promise about
//! this source file rather than a property of the grant. There is no memory grant, no file, no
//! directory, no clock, and no argument a person could type that widens any of it.
//!
//! # EXAMPLES
//!
//! ```text
//! $ pgrep
//! 5
//! 9
//! $ pgrep | wc            the domain, counted, and no /proc was read to make it
//! $ pgrep > names.txt     the names land in the file
//! $ caps pgrep            the scope, printed before anything is spawned
//! ```
//!
//! And the two answers Unix's `pgrep` cannot tell apart, because there both are "print nothing and
//! exit 1":
//!
//! ```text
//! $ pgrep      pgrep: none of the 2 processes in this domain are dead
//! $ pgrep      pgrep: this process holds no process-domain capability
//! ```
//!
//! # BUGS
//!
//! - **At this prompt it always prints every member**, because the selector arrives in a register and
//!   nothing in this system delivers bytes from a command line to a program (`crates/pgrep`'s `BUGS`
//!   has the full reason, and `grant_plan::Prog::Date` is the precedent: three registers the shell
//!   cannot set, and the defaults taken). So `pgrep` here is `pgrep '*'`, and the filter is exercised
//!   by whoever *can* set a register, which today is `kernel::user::survey_tests`.
//! - **`pgrep` names itself**, as `running`, because it is a member of the domain it was spawned
//!   into. Truthful, the same as Unix's `pgrep` without `-x`, and recorded because the first thing a
//!   reader counts is one more than they expected.
//! - **A tid is a name and not a handle, so the answer goes stale.** By the time a person reads a
//!   line, that thread may be gone and its slot reused under a new generation. Nothing here can be
//!   done about it and nothing here needs to be: there is no operation to aim at a stale tid.

#![no_std]
// Program entry points, not the crates/ library surface milestone 68's ratchet tracks
// (DECISIONS §107): each `[[bin]]` is its own crate root with one `_start`, and 58 of them
// documenting an OS-facing ABI entry point is not what the lint is for.
#![allow(missing_docs)]
#![no_main]

use core::sync::atomic::{AtomicBool, Ordering};

use user_rt::{exit, granted, send, survey};

/// The output sink: where the tids go. Slot 0 is where every spawned program's output lands.
const REPORT: u64 = 0;

/// **The process domain**: a supervision endpoint carrying `ENUMERATE`. Everything this program can
/// see comes through here, and nothing else in the capability table names a process.
const DOMAIN_SLOT: u64 = grant_plan::DOMAIN_SLOT;

/// The declared second stream (DECISIONS §67): complaints about the run, never about the domain.
const DIAG_SLOT: u64 = grant_plan::DIAGNOSTICS_SLOT;

/// Whether this process was granted a second stream, decided once in [`_start`].
static HAS_DIAG: AtomicBool = AtomicBool::new(false);

/// `a1` is the state mask, in `grant_plan::spawnproto`'s integer-argument position: init starts every
/// child `thread_control_block_start(tcb, 0, arg, 0)`, so the endowment's `arg` is the second register and the first
/// is always zero. Zero means every state (`pgrep::Selector::from_wire`), which is what a shell that
/// cannot spell a selector sends and is why an unfiltered `pgrep` prints the whole domain rather than
/// nothing.
#[unsafe(no_mangle)]
pub extern "C" fn _start(_a0: u64, a1: u64, _a2: u64) -> ! {
    HAS_DIAG.store(granted(DIAG_SLOT), Ordering::Relaxed);

    // **Walk first, filter second, complain third, print fourth.** DECISIONS §67's reader drains
    // diagnostics to end-of-stream before it reads a byte of output, and a survey cannot know its
    // complaints up front (the domain may be refused on the first call or vanish on the tenth).
    // "Nothing matched" needs the count as well, so the filter runs before the second stream too.
    //
    // The buffer is this program's, sized at `MAX_ROWS`, which is the kernel's whole thread table:
    // even a `pgrep` handed the widest grant this system can express has room for every row, so
    // truncation is unreachable rather than handled. Two kilobytes on a twelve-page stack.
    let mut rows = [ps::Row::default(); ps::MAX_ROWS];
    let walked = ps::collect(&mut rows, &mut |cursor| survey(DOMAIN_SLOT, cursor));
    let found = pgrep::select(&walked, pgrep::Selector::from_wire(a1));

    found.write_diagnostics(&mut |bytes| write_on(diag_slot(), bytes));
    diag_end();

    found.write_report(&mut |bytes| write_on(REPORT, bytes));
    send(REPORT, byte_sink_proto::eof(), 0, 0);
    exit();
}

/// Which endpoint a complaint goes to: the declared second stream when there is one, and the output
/// otherwise. The fallback is honest rather than a silent drop, and it is what the kernel's own tests
/// spawn a program with, since they wire no second sink.
fn diag_slot() -> u64 {
    if HAS_DIAG.load(Ordering::Relaxed) {
        DIAG_SLOT
    } else {
        REPORT
    }
}

/// **Close the second stream**, which is not tidiness: its reader waits for the end before it reads
/// the first, so a `pgrep` that exited without this would leave the prompt blocked.
fn diag_end() {
    if HAS_DIAG.load(Ordering::Relaxed) {
        send(DIAG_SLOT, byte_sink_proto::eof(), 0, 0);
    }
}

/// Write bytes to an endpoint under the sink contract, sixteen at a time. No newline is added: the
/// listing already carries its own, because where a line ends is the answer's business and not the
/// transport's.
fn write_on(slot: u64, bytes: &[u8]) {
    let mut rest = bytes;
    while !rest.is_empty() {
        let (w0, w1, w2, n) = byte_sink_proto::pack(rest);
        send(slot, w0, w1, w2);
        rest = &rest[n..];
    }
}

// Nothing here should panic: `ps::collect` is total for every reader and `pgrep::select` only reads
// the rows it returned. A fault the kernel turns into a kill is the honest signal if one ever does.
//
// **This was hand-rolled with two architecture arms and lost that signal on the third.** Neither
// `cfg` matched on x86_64, so control fell to a spin loop and a panicking `pgrep` burned a thread
// forever instead of dying, which is the opposite of what the paragraph above promises. The macro
// expands to a handler over `user_rt::trap`, whose arms cover all three ISAs and whose x86_64 one
// carries the measured argument for `ud2` over `int3`. This was the last hand-rolled handler left
// outside `user_rt` after milestone 130 swept forty-eight of them.
user_rt::panic_handler!();
