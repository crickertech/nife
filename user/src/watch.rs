//! **`watch`: redraw a live domain instead of printing it once** (milestone 126,
//! design/roadmap/126-who-else-is-running.md, notes/process-view.md).
//!
//! The whole program is `ps`'s own loop (`abi::rendezvous::SURVEY`, `crates/ps`'s `collect`), run a
//! bounded number of times, with `crates/watch`'s `REDRAW` prefix ahead of each table so the terminal
//! shows the latest snapshot in place rather than the whole history scrolling past. What lives here
//! is the syscall, the interval, and the two sinks; the redraw itself and the table are `crates/ps`
//! and `crates/watch`, both of which run on the host in milliseconds.
//!
//! Name: provisional, along with `crates/watch`. `watch` is the name upstream `procps` already ships
//! (`dpkg -L procps` lists `/usr/bin/watch`), which the naming tenet calls the best name available
//! for a standard term a reader already knows; flagged provisional anyway because this program is a
//! narrower thing than upstream's (one fixed built-in view, not an arbitrary command line) and calef
//! may want that difference visible in the name. See `crates/watch`'s module docs for what was
//! narrowed and why.
//!
//! # It is `ps`, redrawn, and nothing wider
//!
//! Real `watch` re-runs an arbitrary command. That needs a program to hold authority to spawn another
//! program by name, which in this system is the shell's own capability
//! (`grant_plan::spawnproto`) and is granted to nothing the shell spawns: an interruptible
//! (`^C`-stoppable) child is built with **no capabilities in its cspace at all**
//! (`crates/system_initializer`'s `spawn_service`), so there is no route today from "a program is
//! running" to "that program can start a second one". Building that route is new spawn-protocol
//! machinery, the same category of gap `top`, `pwdx` and `w` are blocked on, and it is not this
//! program's to close. So this `watch` redraws the one thing it can already reach without any of
//! that: the supervision domain it was spawned into, exactly what `ps` lists. See `crates/watch`'s
//! module docs for the full argument, including why "watch ps" is real Unix's own most common
//! invocation of the tool and not a consolation prize.
//!
//! # Bounded rather than interruptible
//!
//! An interruptible job gets the shared job frame and nothing else; this program needs a domain
//! capability and a report sink for its whole run, so it cannot be spawned that way without teaching
//! init's supervised-spawn path to endow capabilities too, which is a decision for whoever needs that
//! generally and not for one milestone's `watch`. So a bare `watch N` redraws `N` times (clamped to
//! `[1, watch::MAX_ITERATIONS]`, see [`watch::clamp_iterations`]) and exits on its own; its manifest
//! declares `interruptible: false`, the same as `ps`, `pgrep` and `date`, so no `^C` tier reaches it
//! at all (DECISIONS §24) and a bare shell simply waits for it to finish its own count. See `BUGS`.
//!
//! # Capability contract
//!
//! Identical to `ps`'s, with one addition: an argument.
//!
//! | slot | what | why |
//! |---|---|---|
//! | 0 | the output sink, `WRITE` | where the redrawn frames go |
//! | 7 | the process domain, `ENUMERATE` | the supervision endpoint whose members it may **name** |
//! | 8 | the diagnostics sink, `WRITE` | where a refusal goes, so `>` cannot swallow it |
//! | `a0`/`x1` | the redraw count | typed at the prompt (`watch 5`); clamped, never refused |
//!
//! # EXAMPLES
//!
//! ```text
//! $ watch 5
//!          TID  STATE
//!            5  running
//!            9  blocked
//!   (redrawn four more times, in place, over roughly two seconds, then the prompt returns)
//! ```
//!
//! And the case with no Unix equivalent, same as `ps`'s:
//!
//! ```text
//! $ watch 5        watch: this process holds no process-domain capability
//! ```
//!
//! # BUGS
//!
//! - **No `^C` mid-run.** This program is not spawned as an interruptible job (see the module docs
//!   for why), so the shell blocks on its single result stream until it finishes its own count. A
//!   person who typed too large a count waits it out; there is no way to cut a `watch` short today
//!   short of the shell's own forcible teardown of a *stuck* command, which this program is not
//!   (it always terminates on its own after `clamp_iterations(count)` frames).
//! - **The interval is fixed** (`watch::INTERVAL_NANOS`, half a second) and not settable from the
//!   command line. `ArgSpec` carries one integer and it is spent on the count; a second selector
//!   needs the positional arity milestone 47 defers, the same limitation `crates/pgrep`'s `BUGS`
//!   already names for its own missing pattern argument.
//! - **The interval is a yield-spin, not a sleep**, because this kernel has neither: see
//!   `user/src/timetable.rs`'s module docs, which name the four consumers already waiting on
//!   milestone 106's timed-wait fork. This is the fifth, and unlike the other four its whole purpose
//!   is to wait, so the cost is more visible here than anywhere else in the tree: a five-frame
//!   `watch` burns a core for roughly two seconds to do what a real timer would do for nothing.
//! - **A domain that becomes refused partway through a run (rather than at the very first survey)
//!   stops the loop silently**, with no further complaint. The diagnostics stream is checked and
//!   closed once, before the loop starts (DECISIONS §67's ordering rule: everything this program has
//!   to complain about is said before a byte of output, and its reader drains diagnostics to
//!   end-of-stream before it reads anything else), so there is nowhere left to say a second thing by
//!   the time a later survey could fail. In practice this needs the domain's own endpoint to be
//!   destroyed mid-run, which nothing in this tree does to a live supervision endpoint today.
//! - **Every other limitation `ps` has, this has too**, because it is `ps`'s own walk: no `CMD`
//!   column (a process has no name here), and the domain is a subtree rather than the whole machine
//!   unless the shell was itself granted the whole machine's domain. See `crates/ps`'s `BUGS`.

#![no_std]
// Program entry points, not the crates/ library surface milestone 68's ratchet tracks
// (DECISIONS §107): each `[[bin]]` is its own crate root with one `_start`, and 58 of them
// documenting an OS-facing ABI entry point is not what the lint is for.
#![allow(missing_docs)]
#![no_main]

use core::sync::atomic::{AtomicBool, Ordering};

use user_rt::{exit, granted, monotonic_nanos, send, survey, yield_now};

/// The output sink: where redrawn frames go. Slot 0 is where every spawned program's output lands.
const REPORT: u64 = 0;

/// **The process domain**: a supervision endpoint, `ENUMERATE`. `ps`'s own [`DOMAIN_SLOT`], reused
/// rather than renamed: a shell that grants `watch` what it grants `ps` should not have to know the
/// two programs disagree about which slot that authority lands in.
const DOMAIN_SLOT: u64 = grant_plan::DOMAIN_SLOT;

/// The declared second stream (DECISIONS §67): complaints about the run, never about the domain.
const DIAG_SLOT: u64 = grant_plan::DIAGNOSTICS_SLOT;

/// Whether this process was granted a second stream, decided once in [`_start`].
static HAS_DIAG: AtomicBool = AtomicBool::new(false);

#[unsafe(no_mangle)]
pub extern "C" fn _start(_x0: u64, count: u64, _x2: u64) -> ! {
    HAS_DIAG.store(granted(DIAG_SLOT), Ordering::Relaxed);

    // **The first walk decides everything.** A refusal here is permanent (the domain grant this
    // process holds does not change between one syscall and the next), so it is said once, on
    // diagnostics, and the program stops rather than spending its count on identical refusals. An
    // empty domain is not refused and the loop below keeps re-surveying it, in case a member
    // appears.
    let mut rows = [ps::Row::default(); ps::MAX_ROWS];
    let first = ps::collect(&mut rows, &mut |cursor| survey(DOMAIN_SLOT, cursor));

    // Say the complaint (if any) and close the stream *before* a byte of output, same rule `ps`
    // follows and for the same reason: the reader on the other end drains diagnostics to
    // end-of-stream before it reads anything else.
    first.write_diagnostics(&mut |bytes| write_on(diag_slot(), bytes));
    diag_end();

    if first.refused() {
        // Nothing at all on the report stream, matching `ps`'s own `write_report`: a refused
        // `watch > log.txt` leaves the file empty rather than a plausible-looking blank screen.
        send(REPORT, byte_sink_proto::eof(), 0, 0);
        exit();
    }

    let n = watch::clamp_iterations(count);
    watch::frame(&first, &mut |bytes| write_on(REPORT, bytes));

    let mut i = 1u64;
    while i < n {
        wait_interval();
        let mut rows_i = [ps::Row::default(); ps::MAX_ROWS];
        let s = ps::collect(&mut rows_i, &mut |cursor| survey(DOMAIN_SLOT, cursor));
        // See `BUGS`: a mid-run refusal ends the loop with nothing further said, because the
        // diagnostics stream is already closed by the time this can happen.
        if s.refused() {
            break;
        }
        watch::frame(&s, &mut |bytes| write_on(REPORT, bytes));
        i += 1;
    }

    send(REPORT, byte_sink_proto::eof(), 0, 0);
    exit();
}

/// Spin-yield until [`watch::INTERVAL_NANOS`] has passed. There is no timed wait in this kernel
/// (`user/src/timetable.rs`'s module docs name the gap and its other consumers); this is the same
/// shape, one line at a time.
fn wait_interval() {
    let deadline = monotonic_nanos().saturating_add(watch::INTERVAL_NANOS);
    while monotonic_nanos() < deadline {
        yield_now();
    }
}

/// Which endpoint a complaint goes to: the declared second stream when there is one, and the output
/// otherwise. `ps`'s own fallback, verbatim: a `watch` nobody gave a diagnostic sink still has
/// something to say, and in-band is where it used to say it.
fn diag_slot() -> u64 {
    if HAS_DIAG.load(Ordering::Relaxed) {
        DIAG_SLOT
    } else {
        REPORT
    }
}

/// **Close the second stream**, which is not tidiness: its reader waits for the end before it reads
/// the first, so a `watch` that exited without this would leave the prompt blocked.
fn diag_end() {
    if HAS_DIAG.load(Ordering::Relaxed) {
        send(DIAG_SLOT, byte_sink_proto::eof(), 0, 0);
    }
}

/// Write bytes to an endpoint under the sink contract, `ps`'s own chunking. No newline is added: the
/// listing already carries its own.
fn write_on(slot: u64, bytes: &[u8]) {
    let mut rest = bytes;
    while !rest.is_empty() {
        let (w0, w1, w2, n) = byte_sink_proto::pack(rest);
        send(slot, w0, w1, w2);
        rest = &rest[n..];
    }
}

user_rt::panic_handler!();
