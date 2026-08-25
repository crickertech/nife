//! **`watch`: redraw instead of scroll** (milestone 126, design/roadmap/126-who-else-is-running.md).
//!
//! This is the program's logic, lifted out so it runs on the host in milliseconds; `user/src/watch.rs`
//! is the syscall, the interval loop, and nothing else. The crate and the program share a name
//! deliberately, the same split `ps`, `line_editor` and `compositor` already are.
//!
//! Name: provisional. `watch` is upstream `procps`'s own name for the program this is a narrower
//! version of (`dpkg -L procps` lists `/usr/bin/watch`), which the naming tenet calls the best name
//! available for a standard term a reader already knows. Flagged provisional anyway because this
//! program is genuinely narrower than upstream's (one fixed built-in view, not an arbitrary command
//! line), and calef may want that difference visible in the name; see "What this is not" below for
//! the argument the eventual name would need to survive.
//!
//! # What this is not
//!
//! Real `watch` re-runs an arbitrary command line. That needs a program to hold authority to spawn
//! another program by name, and in this system spawning is the shell's own capability
//! (`grant_plan::spawnproto`), granted by init to the shell and to nothing it spawns: an
//! interruptible child is built with **no capabilities in its cspace at all**
//! (`crates/system_initializer`'s `spawn_service`, "no capabilities in its cspace; it reports
//! through the frame and exits"), so there is no way today for a spawned program to become a second
//! spawner without new spawn-delegation machinery this milestone does not design. That is the same
//! shape of gap `top`, `pwdx` and `w` are blocked on: real, and not this program's to close.
//!
//! So this `watch` redraws one fixed, built-in view: **the supervision domain it was spawned into**,
//! exactly what [`ps`] already lists, reusing [`ps::collect`] and [`ps::Survey`] rather than
//! inventing a second vocabulary for "what a domain looks like". It is a live-updating `ps`, which
//! is also the single most common real-world invocation of the Linux tool (`watch ps`, `watch date`,
//! `watch df`, are the FAQ's own top three, and this tree already has `date` and could point a
//! second `watch` mode at it later; nothing here forecloses that). Provisional, flagged in the
//! milestone's report: a fixed built-in is the "or a fixed small set" the roadmap's own text allows,
//! not a claim that this is the only view `watch` should ever have.
//!
//! # The redraw itself
//!
//! [`REDRAW`] is `CSI 2J` (erase the whole display) then `CSI H` (home the cursor), which
//! `video_terminal::Vt` already implements for the line discipline's own `^L`
//! (`crates/video_terminal/src/lib.rs`'s note on `CSI n J`). `watch` is simply the first program in
//! this tree to emit it **on purpose**, as its entire reason for existing, rather than as one key a
//! line discipline forwards. [`frame`] is the whole of that idea: the prefix, then whatever `ps`'s
//! own `Survey::write_report` would have printed for the same walk.
//!
//! # Bounded rather than interruptible, and that is a scope decision, not an oversight
//!
//! `watch` needs a domain capability and a report sink for its whole life, which is exactly what an
//! **interruptible** (`^C`-stoppable, DECISIONS §24) child in this system is built *without*: init's
//! `spawn_service` gives a supervised job the shared job frame and nothing else, on the reasoning
//! that the shell owns its region and tears it down itself. Making `watch` interruptible would mean
//! teaching that path to endow capabilities too, which is new spawn-protocol machinery and a decision
//! that belongs to whoever answers it for every future interruptible-and-capable program, not to one
//! milestone's `watch`.
//!
//! So this `watch` is **bounded**: it redraws a fixed number of times, typed at the prompt
//! (`ArgSpec::Required`), and exits on its own. See `user/src/watch.rs`'s `BUGS` for what that costs
//! a person watching something that will not resolve inside the count they typed.
//!
//! # There is no sleep in this kernel, and this program is another consumer of that gap
//!
//! `user/src/timetable.rs` already says it: "There is no sleep, no timeout and no deadline anywhere
//! in this kernel, so a process that wants to act at a time can only yield and re-read the counter."
//! `watch`'s interval is [`INTERVAL_NANOS`], held against `user_rt::monotonic_nanos()` in a
//! yield-spin loop, exactly `timetable`'s shape. It is milestone 106's sixth named consumer (the
//! block already counted five: `net_stack`'s retransmit window, `thread::sleep`, `RECV`'s no-timeout
//! limitation, the shell's `^C` poll, and `timetable` itself); the fix is the same one line
//! everywhere once that fork is decided.
//!
//! # EXAMPLES
//!
//! ```text
//! $ watch 5             redraws the domain's table five times, once every INTERVAL_NANOS, then exits
//! $ watch 5 > log.txt   the file holds all five frames concatenated, escape codes and all: a
//!                       redirected `watch` cannot un-write the frames it already sent, which is the
//!                       same "the file is not the screen" fact `rm -v | wc` already lives with
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
//! - **A refused domain is fatal rather than retried.** `watch` collects once before it decides
//!   whether to loop at all; a domain grant does not change between one syscall and the next, so a
//!   refusal on the first survey is reported once (on the diagnostics stream, the same as `ps`) and
//!   the program exits without spending its count on N identical refusals. This is `ps`'s own
//!   "refused loudly" rule, not a `watch`-specific idea; see `crates/ps`'s own module docs.
//! - **An empty domain still redraws.** Unlike a refusal, an empty domain is not a permanent fact
//!   (a spawn could add a member before the next frame), so `watch` keeps looping and each frame
//!   prints [`ps::Survey::complaint`]'s "this domain holds no processes" sentence in place of a
//!   table, the same wording `ps`'s own diagnostics stream would use, redrawn each interval.
#![cfg_attr(not(test), no_std)]

pub use ps::{MAX_ROWS, Row, Survey, collect};

/// **Erase the whole display, then home the cursor** (`CSI 2J` `CSI H`). Feeding this before a frame
/// is what makes the next write a redraw instead of a fifth scrolled-past line; `video_terminal::Vt`
/// already parses both sequences for the line discipline's `^L`; see its module docs.
pub const REDRAW: &[u8] = b"\x1b[2J\x1b[H";

/// **The default and only interval today** (no `ArgSpec` register is spent on it; see the module
/// docs). Half a second: fast enough that a person watching a short-lived domain sees it change,
/// slow enough that the yield-spin loop this kernel's missing timed wait forces on every interval
/// user does not visibly burn a core the way `timetable`'s un-throttled fire loop can.
pub const INTERVAL_NANOS: u64 = 500_000_000;

/// **How many redraws a bare `watch N` may ask for.** A ceiling exists because every frame this
/// program writes travels through the shell's `drain_text` loop, which gives up after
/// `MAX_OUTPUT_CHUNKS` (4,096) sixteen-byte messages and calls the stream truncated
/// (`user/src/swish.rs`); this bound keeps a plausible `watch` run (a domain of a few dozen threads)
/// well under that regardless of how the shell's own limit moves later. It is generous rather than
/// tight: a person who wants more than this many redraws of a static demo domain is better served by
/// milestone 106's timed wait than by a bigger ceiling here.
pub const MAX_ITERATIONS: u64 = 200;

/// **What a bare `watch 0` (or nothing sane) means.** Zero redraws would exit having shown nothing,
/// which is a program that ran and said nothing about why; one redraw is the smallest run that is
/// still `watch` rather than a silent no-op, so a zero count is *not* a refusal (the count register
/// cannot express one; see `crates/grant_plan`'s `ArgSpec`, which only distinguishes "an argument was
/// typed" from "it was not") and is instead clamped up, the same "a caller that got the selector
/// wrong wants the default" reasoning `user/src/date.rs`'s `format_of` already uses for an
/// unrecognised format.
pub fn clamp_iterations(requested: u64) -> u64 {
    requested.clamp(1, MAX_ITERATIONS)
}

/// **One redrawn frame of `survey`'s table**: [`REDRAW`], then exactly what `ps` would have printed
/// for the same walk, or (when there is nothing to print) the one-line reason a person would
/// otherwise be staring at a blank, unexplained screen for. `out` is called with a `\n`-terminated
/// sentence in that second case and never mixes the two: a survey either has rows to show or it has a
/// [`Survey::complaint`], never both, which is `ps::Survey`'s own invariant and not something this
/// function checks twice.
pub fn frame(survey: &Survey<'_>, out: &mut dyn FnMut(&[u8])) {
    out(REDRAW);
    if let Some(clause) = survey.complaint() {
        // A refusal never reaches here (the caller checks `refused()` once, before the loop even
        // starts; see the module docs), so the only complaint a frame can carry is the benign one:
        // "this domain holds no processes". Printed with the program's own name, matching every
        // other diagnostic sentence in this tree (`ps: `, `date: `, ...).
        out(b"watch: ");
        out(clause.as_bytes());
        out(b"\n");
        return;
    }
    survey.write_report(out);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(tid: u64, state: u64) -> Row {
        Row { tid, state }
    }

    /// Everything currently on the grid, all rows concatenated, so a test can ask "does this glyph
    /// appear anywhere on screen" without assuming which row or column a line landed at. Needed
    /// because this engine's `LF` does not return the carriage (`video_terminal`'s own
    /// `the_bottom_row_scrolls`-adjacent test pins that as `LF alone must not return the carriage`),
    /// so a multi-line table's rows do not start at column 0 the way a naive reader would expect;
    /// asserting on exact row/column position would be testing that quirk, not `watch`.
    fn grid_text(vt: &video_terminal::Vt, rows: u32) -> String {
        let mut s = String::new();
        let mut buf = [0u8; video_terminal::MAX_COLS];
        for r in 0..rows {
            vt.row_bytes(r, &mut buf);
            s.push_str(core::str::from_utf8(&buf).unwrap());
        }
        s
    }

    /// **A refresh with new rows shows only the new rows, with no trace of the old ones**, checked
    /// against the real terminal engine rather than against this crate's own byte buffer. Two frames
    /// are fed through one `video_terminal::Vt`, the same struct `display_terminal.rs` renders a real
    /// screen with: a wide first frame (a header plus two threads, tid 5 and tid 9), then a narrow
    /// second frame (header plus tid 9 alone). Both frames together are five lines of raw text, well
    /// under the eight-row grid, so nothing here is close to scrolling off the grid on its own; the
    /// only way tid 5's digit can vanish from the screen is [`REDRAW`]'s `CSI 2J` actually erasing it.
    /// A `watch` that only overwrote (no erase) would leave `5` sitting in the grid forever, since the
    /// second frame never writes over that cell at all.
    #[test]
    fn a_second_frame_erases_the_first_rather_than_leaving_it_on_screen() {
        let mut vt = video_terminal::Vt::new(video_terminal::MAX_COLS as u32, 8);

        // `collect` fills its buffer from `read`, so the buffer below starts empty; the rows it ends
        // up holding come entirely from the closure, which is the fake "kernel" this host test drives
        // instead of a real `SURVEY` syscall. `next_cursor` must strictly increase and `0` is reserved
        // for `DONE` (`collect` checks `next == DONE` before it checks `next <= cursor`, which is what
        // lets an empty domain answer `DONE` on its very first call), so real entries are numbered
        // from 1.
        let rows_a = [row(5, abi::survey::RUNNING), row(9, abi::survey::BLOCKED)];
        let mut buf_a = [Row::default(); MAX_ROWS];
        let survey_a = collect(&mut buf_a, &mut {
            let mut calls = 0u64;
            move |_cursor| {
                let out = if calls == 0 {
                    (1, rows_a[0].tid, rows_a[0].state)
                } else if calls == 1 {
                    (2, rows_a[1].tid, rows_a[1].state)
                } else {
                    (abi::survey::DONE as i64, 0, 0)
                };
                calls += 1;
                out
            }
        });
        frame(&survey_a, &mut |bytes| vt.feed(bytes));

        // Sanity check before the real assertion: if this fails, the test's own frame construction is
        // wrong, not `watch`.
        let after_a = grid_text(&vt, 8);
        assert!(
            after_a.contains('5') && after_a.contains('9'),
            "both threads should be on screen after the first frame: {after_a:?}"
        );

        let rows_b = [row(9, abi::survey::BLOCKED)];
        let mut buf_b = [Row::default(); MAX_ROWS];
        let survey_b = collect(&mut buf_b, &mut {
            let mut calls = 0u64;
            move |_cursor| {
                let out = if calls == 0 {
                    (1, rows_b[0].tid, rows_b[0].state)
                } else {
                    (abi::survey::DONE as i64, 0, 0)
                };
                calls += 1;
                out
            }
        });
        frame(&survey_b, &mut |bytes| vt.feed(bytes));

        // The real assertion: tid 5 is gone from the whole screen, not merely from wherever the
        // second frame happened to write. tid 9 is still there, unmoved, because it is in both
        // frames.
        let after_b = grid_text(&vt, 8);
        assert!(
            !after_b.contains('5'),
            "tid 5's line from the first frame is still on screen: watch overwrote instead of \
             erasing: {after_b:?}"
        );
        assert!(
            after_b.contains('9'),
            "tid 9's line should still be on screen in the second frame: {after_b:?}"
        );
    }

    /// A refusal is never fed to [`frame`] by the real program (`user/src/watch.rs` checks
    /// `refused()` once, before it ever loops), but [`Survey::complaint`] still answers correctly for
    /// one, and this pins that a refused survey's complaint is never the empty-domain sentence a
    /// caller might otherwise confuse it with.
    #[test]
    fn a_refusal_and_an_empty_domain_report_different_complaints() {
        let mut buf = [Row::default(); MAX_ROWS];
        let refused = collect(&mut buf, &mut |_| (abi::Error::NotPermitted as i64, 0, 0));
        assert!(refused.refused());

        let mut buf2 = [Row::default(); MAX_ROWS];
        let empty = collect(&mut buf2, &mut |_| (abi::survey::DONE as i64, 0, 0));
        assert!(!empty.refused());
        assert_ne!(refused.complaint(), empty.complaint());
    }

    /// [`clamp_iterations`]'s three cases: too low is raised, too high is capped, and an ordinary
    /// count passes through unchanged. `MAX_ITERATIONS` is a real ceiling, not a suggestion, and this
    /// is the only test that would notice it silently moving.
    #[test]
    fn clamp_iterations_bounds_both_directions() {
        assert_eq!(clamp_iterations(0), 1);
        assert_eq!(clamp_iterations(1), 1);
        assert_eq!(clamp_iterations(5), 5);
        assert_eq!(clamp_iterations(MAX_ITERATIONS), MAX_ITERATIONS);
        assert_eq!(clamp_iterations(MAX_ITERATIONS + 1), MAX_ITERATIONS);
        assert_eq!(clamp_iterations(u64::MAX), MAX_ITERATIONS);
    }
}
