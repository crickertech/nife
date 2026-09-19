//! **The one thing this tool sends, and the log entry that makes sending it safe** (milestone 324,
//! parts 1 and 4).
//!
//! Every other module here reads. This one writes, and it exists because calef ruled on 2026-09-19
//! that the invariant in `script/board-console`'s header changes from *"it reads and never writes
//! to the board"* to **it writes only what a named mode sends, and every byte it sends is printed
//! into the log.**
//!
//! The second clause is the half that makes the first one safe, and it is why this module owns both
//! the decision and the write. A bench log is the artifact a run is judged from, so a byte the
//! board received that the capture does not show makes the capture a lie about the run. There is no
//! path through this file that puts a byte on the wire without having already written what it is
//! about to do into the log: [`Escape::observe_line`] is the only sender, and it writes the
//! announcement before it writes the byte.
//!
//! # What it is for
//!
//! Milestone 249 made a board reboot itself every two minutes so that fifty draws of the
//! thread-placement lottery fit in an evening instead of nine draws filling one. Its escape is a
//! byte on the console UART: `kernel/src/soak.rs` polls the NS16550's data-ready bit every five
//! seconds and again in the grace window before each reset, and any byte at all disarms the loop.
//! Until now the only thing that could press that key was a person, because this tool held the port
//! and could not write. So a series ended when somebody was at the bench, which is the constraint
//! milestone 249 built the escape to remove and could not finish removing.
//!
//! - `--stop` ends the series at the next draw.
//! - `--stop-after <n>` ends it at the n-th, which is 249's own wording for what a writing mode
//!   buys: *a series with exactly the sample it was asked for.*
//!
//! `--stop` is `--stop-after 1` and is not a second mechanism. That is worth stating rather than
//! implementing twice, because the two flags then cannot disagree about anything.
//!
//! # Why it sends on the arming announcement and on nothing else
//!
//! This is the whole of the safety argument, and it is a mechanism rather than a caution.
//!
//! `kernel/src/soak.rs`'s `arm_reboot` does two things in this order: it calls
//! `console::discard_rx`, which throws away anything already sitting in the UART, and then it
//! prints the banner this module matches on. The drain has a reason of its own (a console that sat
//! in front of a person may hold a byte U-Boot's countdown collected, and firing the escape on boot
//! 1 of 50 would produce no distribution and no explanation), and it has a consequence nobody had
//! cause to write down: **a byte sent before that banner is a byte the kernel deliberately
//! discards.**
//!
//! So a sender that fired on `soak-test: started` would lose the race almost every time. That line
//! is printed *before* `arm_reboot`, with four more `println!`s between them: about a kilobyte, so
//! roughly ninety milliseconds at 115200 baud, and a host that reads a line and answers it is
//! comfortably inside ninety milliseconds. The failure would be silent and would look exactly like
//! a board whose receive path is miswired: no acknowledgement, and a series that keeps rebooting.
//!
//! The arming banner closes that, and it closes three more things at the same time, which is why it
//! is the only trigger here:
//!
//! - **It proves the drain has already happened**, so the byte cannot be discarded.
//! - **It proves a reboot loop exists to stop.** A plain `--features soak_test` kernel never prints
//!   it and never polls for an escape, so a byte sent at one would sit in the FIFO forever, which is
//!   writing to a board with nothing to gain.
//! - **It proves the board is past its firmware.** The hazard the ruling is actually about is an
//!   open keyboard beside U-Boot's autoboot countdown; milestone 249's lane sent this escape by
//!   detaching the console, hit the countdown with it, and paid a power cycle. A kernel that has
//!   printed this banner has been running for the length of a boot tour, so the countdown is long
//!   over. The gate rather than the byte is what makes that impossible.
//! - **It is one per boot of a rebooting build**, so counting them counts draws.
//!
//! The cost of the gate is one sentence long and is the honest half of it: a session that attaches
//! to a board already mid-draw sees no banner until that draw's reboot, so `--stop` waits out the
//! remainder of the current draw. That is up to `REBOOT_AFTER_SECONDS`, two minutes, and it buys
//! the four properties above.
//!
//! # Counting draws, and why this count is the lottery's count
//!
//! [`lottery`](crate::lottery) opens a draw on `soak-test: started`. This module counts arming
//! banners. In a `--features reboot_soak_test` capture those are one-to-one and in that order, so a
//! log produced by `--stop-after n` tallies as exactly n draws, which is the property the flag
//! promises and which `a_stop_after_n_log_tallies_as_n_draws` asserts. They differ only for a build
//! with no reboot loop, where the lottery has nothing to draw and this module has nothing to stop.
//!
//! # What confirms it
//!
//! The kernel prints its own line when the escape lands: `soak-test-reboot: DISARMED`, from either
//! the beat poll or the grace window. This module waits for it, for [`CONFIRM_WINDOW`], and the
//! session's exit status says whether it came.
//!
//! **That closes a gap `kernel/src/soak.rs` states in its own `BUGS` and cannot close from where it
//! stands**: *"the escape is a poll of one bit and nothing verifies that the bit can ever be set
//! [...] Nothing in this kernel can prove otherwise, because a UART cannot receive a byte it
//! sends."* A host holding the far end of the cable is not under that limit. It sends a byte it did
//! not print and reads back an acknowledgement the board printed, so a confirmed `--stop` is the
//! first evidence in this tree that the board's receive path works end to end. The procedure that
//! kernel BUGS entry names as the substitute (press a key on the first boot and confirm `DISARMED`
//! before walking away) becomes something a script does and records.
//!
//! # BUGS
//!
//! **No byte from this module has ever reached a board.** Milestone 324's lane had no board
//! attached to it: radon and xenon were both unreachable, so everything here is exercised against
//! captures and against a `Vec<u8>` standing in for the port. What that proves is the decision (when
//! a byte is sent, and that it is sent only then), the exact bytes that would go on the wire, and
//! the log entries beside them. What it does not prove is that a write to the descriptor reaches
//! the UART, nor that the kernel's poll finds it. The first real `--stop` is the experiment, and its
//! `DISARMED` line is the result; until then this is a tested decision attached to an untested wire.
//!
//! **A session that attaches mid-draw loses that draw.** Stated above and repeated here because it
//! is what a person meets: `--stop` on a board already soaking waits for the next boot's arming
//! banner, which is up to two minutes away. Attaching at power-on has no such gap.
//!
//! **An unconfirmed send is three faults wearing one report.** The byte may not have left the host,
//! the board's receive path may be dead (`kernel/src/soak.rs`'s own BUGS), or the kernel may be
//! wedged in a way the beat has not yet shown. The tool says the byte went out and was not
//! acknowledged, and cannot say which of the three it is. The log's surrounding beats are what
//! separates them by hand: beats still arriving with no `DISARMED` points at the receive path.
//!
//! **The markers here are string literals checked against `kernel/src/soak.rs` by a reader, not by
//! the compiler.** `boot_ladder` exists for exactly this problem and holds the boot tour's markers
//! as shared constants; the soak's markers were never hoisted into it, so this module repeats what
//! [`progress`](crate::progress) already does. Hoisting `START_MARKER`, `REBOOT_MARKER` and the
//! arming and disarming lines into `boot_ladder` would make the kernel and this module agree by
//! construction, and it is a change to the kernel that milestone 324's parts 1 and 4 do not cover.
//!
//! **The firmware-refused case exits 0 for `--stop-after n` as well as for `--stop`.** A board whose
//! OpenSBI refuses SRST reset type 1 prints `soak-test-reboot: FAILED` and never reboots, so there
//! is no loop to stop and this module ends the session saying so. For `--stop` that is the goal
//! reached by another route. For `--stop-after 50` it means the series never happened, and a zero
//! exit understates that; the log and the summary line say it in words, which is the weaker rung.

use std::io::{self, Write};
use std::time::{Duration, Instant};

/// **The byte `--stop` puts on the wire: a carriage return.**
///
/// The kernel's contract is any byte at all, so this is a choice inside a contract that is already
/// fixed rather than anything two programs agree on. It is recorded here because it is the one
/// value in this file a board will ever see.
///
/// **Carriage return because it is what the kernel's own instruction describes.**
/// `kernel/src/soak.rs` prints *"press any key on this console"*, and the key a person at a bench
/// presses when told that is Enter. A tool whose byte is the one the documented manual procedure
/// produces is running the same experiment the procedure runs, rather than a similar one.
///
/// **`NUL` was the other candidate and was refused, narrowly.** It is the most inert byte there is:
/// U-Boot ignores it, a shell line discipline ignores it, and it still sets the NS16550's
/// data-ready bit, which is all the escape reads. It loses on the failure mode rather than on the
/// success one. A carriage return that is not acknowledged means the receive path is at fault; a
/// `NUL` that is not acknowledged could also be a USB-serial bridge or a driver treating a frame of
/// eight zero bits as something other than a character, and a report that cannot separate those is
/// worse than one that can. The inertness it buys is already bought by [`Escape`]'s gate, which
/// sends nothing until the board has said it is running an armed soak; there is no state left for
/// an inert byte to be inert in.
pub const ESCAPE_BYTE: u8 = b'\r';

/// **How long to wait for the board's `DISARMED` line before giving up on it.**
///
/// Fifteen seconds, which is three of `kernel/src/soak.rs`'s five-second beats, chosen the same way
/// [`Policy::quiet_after`](crate::watch::Policy::quiet_after)'s default was: the escape is polled
/// once per beat, so three beats is three chances and a fourth would only make an unreachable
/// receive path take longer to report.
///
/// The byte is sent just after the arming banner and the first beat follows within five seconds, so
/// a healthy board answers well inside this.
pub const CONFIRM_WINDOW: Duration = Duration::from_secs(15);

/// The arming announcement's distinctive phrase, from `kernel/src/soak.rs`'s `arm_reboot`.
///
/// Printed **after** `console::discard_rx`, which is the whole reason this is the trigger; the
/// module header has the argument. Matched on this phrase rather than on the `soak-test-reboot:`
/// prefix because that prefix is also on the grace-window announcement, the disarm and the
/// firmware refusal, and only this line means *the loop is armed and the drain is behind us*.
const ARMED: &str = "THIS BUILD REBOOTS THE BOARD";

/// The kernel's acknowledgement, printed from the beat poll and from the grace window alike.
///
/// The prefix is carried so that the word alone, which is a word a person might well write in a log
/// annotation, cannot be mistaken for the board saying it.
const DISARMED: &str = "soak-test-reboot: DISARMED";

/// The firmware refusing the cold reboot, which means there was never a loop to stop.
const REFUSED: &str = "soak-test-reboot: FAILED";

/// How a writing mode ended, which is what the exit status is computed from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Report {
    /// The session ended before the n-th armed draw came round. Nothing was written to the board.
    ///
    /// This is every case the ruling did not cover, arriving as one answer: a board that is powered
    /// off, sitting at a U-Boot prompt, running a kernel with no soak in it, running a soak with no
    /// reboot loop, or simply not that far along when the deadline hit. **In none of them is a byte
    /// sent**, because the only thing that sends one is the board announcing an armed reboot loop.
    NotSent,
    /// The byte went out and the board has not acknowledged it.
    Sent,
    /// The board printed its own `DISARMED` line. The series is over and the board knows it.
    Confirmed,
    /// The write failed, carrying what the operating system said. Nothing reached the board, and
    /// the log says so on the line after the one that said what was about to be sent.
    Failed(String),
    /// The board's firmware refused the cold reboot, so no reboot loop ever existed to stop.
    /// Nothing was written.
    NothingToStop,
}

impl Report {
    /// Whether this is an ending that got what `--stop` asked for.
    ///
    /// [`Report::NothingToStop`] counts: a board that cannot reboot itself is a board that will not
    /// reboot itself, which is the state the flag exists to reach. See this module's `BUGS` for why
    /// that reads worse for `--stop-after n` than for `--stop`.
    #[must_use]
    pub fn reached_the_goal(&self) -> bool {
        matches!(self, Report::Confirmed | Report::NothingToStop)
    }

    /// One line for the session summary, in the vocabulary the log already uses.
    #[must_use]
    pub fn describe(&self) -> String {
        match self {
            Report::NotSent => {
                "no byte was sent: no armed reboot loop announced itself before the session ended"
                    .to_string()
            }
            Report::Sent => format!(
                "sent the escape ({}) and the board did not acknowledge it within {}s",
                render(ESCAPE_BYTE),
                CONFIRM_WINDOW.as_secs()
            ),
            Report::Confirmed => {
                "the board acknowledged the escape and disarmed its reboot loop".to_string()
            }
            Report::Failed(why) => format!("the escape could not be written to the port: {why}"),
            Report::NothingToStop => {
                "nothing to stop: the firmware refused the cold reboot, so this board was never \
                 going to reboot itself"
                    .to_string()
            }
        }
    }
}

/// **The decision to write, and the writing, in one object that cannot do the second without the
/// first.**
///
/// Fed complete lines as they arrive. It counts the board's arming announcements, sends one byte at
/// the n-th, and then waits for the kernel to say it found it.
///
/// The port is `&mut dyn Write` rather than a `File` so that a host test can hand it a `Vec<u8>`
/// and assert on the exact bytes a board would have received. That is as close to the wire as
/// anything in this crate gets without a board on the other end of it.
pub struct Escape<'a> {
    /// Which armed draw to send on. `--stop` is 1.
    after: usize,
    /// Arming announcements seen in this session.
    draws: usize,
    /// When the byte went out, which starts [`CONFIRM_WINDOW`].
    sent_at: Option<Instant>,
    report: Report,
    port: &'a mut dyn Write,
}

impl<'a> Escape<'a> {
    /// Stop after the `after`-th armed draw this session sees. `after` is clamped up to 1, because
    /// stopping after zero draws is not a thing a board can be asked for.
    pub fn new(after: usize, port: &'a mut dyn Write) -> Self {
        Self {
            after: after.max(1),
            draws: 0,
            sent_at: None,
            report: Report::NotSent,
            port,
        }
    }

    /// What happened, so far.
    #[must_use]
    pub fn report(&self) -> &Report {
        &self.report
    }

    /// Armed draws counted so far, which is how many samples the series will have.
    #[must_use]
    pub fn draws(&self) -> usize {
        self.draws
    }

    /// Whether the session has nothing left to wait for.
    ///
    /// True once the board has acknowledged, once the confirmation window has closed on an
    /// unacknowledged send, once a write has failed, and once the firmware has said there is no
    /// loop. False while waiting for the n-th draw, which is what lets the deadline do its job.
    #[must_use]
    pub fn finished(&self) -> bool {
        match &self.report {
            Report::Confirmed | Report::Failed(_) | Report::NothingToStop => true,
            Report::Sent => self
                .sent_at
                .is_some_and(|at| at.elapsed() >= CONFIRM_WINDOW),
            Report::NotSent => false,
        }
    }

    /// Offer one **complete** line from the board, plus how far into the session it arrived.
    ///
    /// Complete lines only. A partial tail may ratchet a stage, because a substring match is
    /// monotone, but it may not be allowed to send a byte: the arming banner's phrase could arrive
    /// as the tail of a line that turns out to say something else, and this is the one decision in
    /// the crate that cannot be taken back once it is wrong.
    ///
    /// `log` is the session's log, and this is the only place a byte is written to the board. The
    /// announcement goes into the log **before** the write to the port, so the ordering of the two
    /// failures is the readable one: a log that says a byte is going out and then says the write
    /// failed is honest, where a byte on the wire that the log never mentions is the thing the
    /// ruling forbids.
    ///
    /// # Errors
    ///
    /// The log's error, if the log cannot be written. A failure to write to the *port* is recorded
    /// in the report and logged rather than returned, because the session is still worth finishing:
    /// the board is still talking and the operator still wants what it says.
    pub fn observe_line(
        &mut self,
        line: &str,
        at: Duration,
        log: &mut dyn Write,
    ) -> io::Result<()> {
        let line = line.trim_end_matches(['\r', '\n']);

        // Acknowledgement first, so a board that disarms for any reason (somebody at the bench
        // pressed a key too) is believed rather than raced.
        if line.contains(DISARMED) {
            if self.report == Report::Sent {
                self.report = Report::Confirmed;
                writeln!(
                    log,
                    "\nboard-console: the board acknowledged the escape and will not reboot itself \
                     again. Its own line is above; the soak keeps running."
                )?;
            }
            return Ok(());
        }
        if !matches!(self.report, Report::NotSent) {
            return Ok(());
        }
        if line.contains(REFUSED) {
            self.report = Report::NothingToStop;
            writeln!(
                log,
                "\nboard-console: no byte sent. The board's firmware refused the cold reboot (its \
                 own line is above), so there is no reboot loop to stop."
            )?;
            return Ok(());
        }
        if !line.contains(ARMED) {
            return Ok(());
        }

        self.draws += 1;
        if self.draws < self.after {
            return Ok(());
        }
        self.send(at, log)
    }

    /// Write the announcement, then the byte, then the outcome. Never any other order.
    fn send(&mut self, at: Duration, log: &mut dyn Write) -> io::Result<()> {
        writeln!(
            log,
            "\nboard-console: sending the soak escape to the board now: 1 byte, {}. Draw {} of {} \
             armed its reboot at +{:.1}s and this is the sample the series was asked for.",
            render(ESCAPE_BYTE),
            self.draws,
            self.after,
            at.as_secs_f64()
        )?;
        log.flush()?;

        match self
            .port
            .write_all(&[ESCAPE_BYTE])
            .and_then(|()| self.port.flush())
        {
            Ok(()) => {
                self.sent_at = Some(Instant::now());
                self.report = Report::Sent;
                writeln!(
                    log,
                    "board-console: sent 1 byte, {}. Waiting up to {}s for the kernel to say it \
                     found it.",
                    render(ESCAPE_BYTE),
                    CONFIRM_WINDOW.as_secs()
                )?;
            }
            Err(e) => {
                writeln!(
                    log,
                    "board-console: the write failed and nothing reached the board: {e}"
                )?;
                self.report = Report::Failed(e.to_string());
            }
        }
        log.flush()
    }
}

/// A byte as a reader should meet it in a log: never as itself.
///
/// The escape is a control character, and a log that carried it raw would put a bare carriage
/// return in the middle of the capture, where it would move the cursor of anyone reading with
/// `cat`, break the line the recogniser splits on if the capture is ever replayed, and be invisible
/// in the one artifact whose job is to show what was sent.
fn render(byte: u8) -> String {
    format!("0x{byte:02x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The kernel's three arming lines, abbreviated to the one this module matches plus enough
    /// around it to be a realistic line. Quoted from `kernel/src/soak.rs`'s `arm_reboot`.
    const ARM_LINE: &str = "soak-test-reboot: THIS BUILD REBOOTS THE BOARD. It soaks for 120s, \
                            then asks the firmware for a cold reboot (SBI SRST reset type 1) and \
                            draws the thread-placement lottery again, forever.";

    const START_LINE: &str =
        "soak-test: started 4 groups of one responder, 3 callers, 1 grinder and 1 tick waiter";

    const BEAT_LINE: &str =
        "soak-test: t=5s beat=1 rounds=100 rate=20/s workers=20 refused=0 mismatch=0 stalled=0";

    const DISARM_LINE: &str = "soak-test-reboot: DISARMED at t=5s: a byte arrived on this console. \
                               This board will not reboot itself again.";

    /// Feed a script of lines and hand back what went to the board and what went to the log.
    fn run(after: usize, lines: &[&str]) -> (Vec<u8>, String, Report) {
        let mut port: Vec<u8> = Vec::new();
        let mut log: Vec<u8> = Vec::new();
        let report = {
            let mut escape = Escape::new(after, &mut port);
            for (n, line) in lines.iter().enumerate() {
                escape
                    .observe_line(line, Duration::from_secs(n as u64), &mut log)
                    .expect("the log is a Vec and cannot fail");
            }
            escape.report().clone()
        };
        (port, String::from_utf8(log).expect("ASCII"), report)
    }

    /// **The whole of the safety argument, as an assertion.** A boot that gets as far as the soak
    /// starting, and a board sitting at a U-Boot prompt, and a kernel printing its banner, are all
    /// states in which nothing may be written, because none of them has announced an armed reboot
    /// loop. This is the test that would fail if somebody moved the trigger earlier.
    #[test]
    fn nothing_is_written_until_the_board_announces_an_armed_reboot_loop() {
        let (port, log, report) = run(
            1,
            &[
                "U-Boot SPL 2021.10 (Feb 12 2023 - 20:24:34 +0800)",
                "StarFive # ",
                "Starting kernel ...",
                "nife on RISC-V (rv64, S-mode, Sv39)",
                "nife: the capability core runs on RISC-V.",
                START_LINE,
                BEAT_LINE,
            ],
        );
        assert!(
            port.is_empty(),
            "a byte reached the board before an arming banner did: {port:?}"
        );
        assert_eq!(report, Report::NotSent);
        assert!(log.is_empty(), "nothing was sent, so nothing is announced");
    }

    /// The `soak-test: started` race the module header is about. `arm_reboot`'s drain runs after
    /// that line is printed, so a sender that fired on it would have its byte discarded; the
    /// assertion is that the byte waits for the banner that follows the drain.
    #[test]
    fn the_byte_waits_for_the_line_printed_after_the_kernel_drains_the_uart() {
        let (port, _, _) = run(1, &[START_LINE]);
        assert!(port.is_empty(), "sent into the drain");

        let (port, _, report) = run(1, &[START_LINE, ARM_LINE]);
        assert_eq!(port, vec![ESCAPE_BYTE]);
        assert_eq!(report, Report::Sent);
    }

    /// Exactly one byte, exactly once, whatever else the board says afterwards.
    #[test]
    fn one_byte_goes_out_and_only_one() {
        let (port, _, _) = run(1, &[ARM_LINE, BEAT_LINE, BEAT_LINE, ARM_LINE, BEAT_LINE]);
        assert_eq!(port, vec![b'\r'], "one carriage return and nothing else");
    }

    /// `--stop-after n` sends on the n-th armed draw and not before it.
    #[test]
    fn stop_after_n_sends_on_the_nth_armed_draw() {
        for n in 1..=4usize {
            let mut lines = Vec::new();
            for _ in 0..n - 1 {
                lines.push(ARM_LINE);
                lines.push(BEAT_LINE);
            }
            let (port, _, report) = run(n, &lines);
            assert!(port.is_empty(), "sent after {} of {n} draws", n - 1);
            assert_eq!(report, Report::NotSent);

            lines.push(ARM_LINE);
            let (port, _, report) = run(n, &lines);
            assert_eq!(port, vec![ESCAPE_BYTE], "did not send on draw {n}");
            assert_eq!(report, Report::Sent);
        }
    }

    /// `--stop` and `--stop-after 1` are one mechanism, and zero is read as one.
    #[test]
    fn stop_is_stop_after_one_and_zero_is_not_a_thing_to_ask_for() {
        let (one, _, _) = run(1, &[ARM_LINE]);
        let (zero, _, _) = run(0, &[ARM_LINE]);
        assert_eq!(one, zero);
        assert_eq!(one, vec![ESCAPE_BYTE]);
    }

    /// **The invariant, asserted rather than described.** The log names the byte, in hex, before
    /// the byte is on the wire, and names it again afterwards.
    #[test]
    fn every_byte_sent_is_printed_into_the_log() {
        let (port, log, _) = run(1, &[ARM_LINE]);
        assert_eq!(port, vec![ESCAPE_BYTE]);
        assert!(
            log.contains("0x0d"),
            "the log does not name the byte: {log}"
        );
        assert!(
            log.find("sending the soak escape").unwrap() < log.find("sent 1 byte").unwrap(),
            "the announcement must precede the write, so a failed write reads correctly"
        );
        assert!(
            !log.contains('\r'),
            "the log renders the byte and never carries it"
        );
    }

    /// A port that refuses the write is reported, and the log still shows what was attempted. The
    /// session is not abandoned: the board is still talking.
    #[test]
    fn a_write_that_fails_is_recorded_rather_than_hidden() {
        struct Broken;
        impl Write for Broken {
            fn write(&mut self, _: &[u8]) -> io::Result<usize> {
                Err(io::Error::new(
                    io::ErrorKind::BrokenPipe,
                    "the adapter went away",
                ))
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }

        let mut port = Broken;
        let mut log: Vec<u8> = Vec::new();
        let mut escape = Escape::new(1, &mut port);
        escape
            .observe_line(ARM_LINE, Duration::from_secs(1), &mut log)
            .expect("the log still writes");
        let log = String::from_utf8(log).expect("ASCII");

        assert!(matches!(escape.report(), Report::Failed(_)));
        assert!(escape.finished(), "there is nothing left to wait for");
        assert!(log.contains("sending the soak escape"));
        assert!(log.contains("the write failed and nothing reached the board"));
        assert!(log.contains("the adapter went away"));
    }

    /// The board's own acknowledgement is what turns a send into a result.
    #[test]
    fn the_boards_disarmed_line_confirms_the_send() {
        let (_, log, report) = run(1, &[ARM_LINE, BEAT_LINE, DISARM_LINE]);
        assert_eq!(report, Report::Confirmed);
        assert!(report.reached_the_goal());
        assert!(log.contains("acknowledged the escape"));
    }

    /// A `DISARMED` that arrives with nothing sent is somebody at the bench, and is not this tool's
    /// result to claim.
    #[test]
    fn a_disarm_nobody_sent_for_is_not_claimed_as_a_confirmation() {
        let (port, _, report) = run(2, &[ARM_LINE, DISARM_LINE]);
        assert!(port.is_empty());
        assert_eq!(report, Report::NotSent, "the tool sent nothing to confirm");
    }

    /// The firmware refusing the reset means there was never a loop, so nothing is written.
    #[test]
    fn a_firmware_that_refuses_the_reset_leaves_nothing_to_stop() {
        let refused = "soak-test-reboot: FAILED: the firmware refused a cold reboot and returned \
                       sbiret.error=-2 (-2 is SBI_ERR_NOT_SUPPORTED).";
        let (port, log, report) = run(3, &[ARM_LINE, BEAT_LINE, refused]);
        assert!(port.is_empty());
        assert_eq!(report, Report::NothingToStop);
        assert!(report.reached_the_goal());
        assert!(log.contains("no reboot loop to stop"));
    }

    /// Only complete lines decide. A tail that happens to end inside the arming phrase must not be
    /// enough, which is why `watch` offers this module lines and never the partial tail.
    #[test]
    fn the_confirmation_window_is_what_ends_an_unacknowledged_send() {
        let mut port: Vec<u8> = Vec::new();
        let mut log: Vec<u8> = Vec::new();
        let mut escape = Escape::new(1, &mut port);
        escape
            .observe_line(ARM_LINE, Duration::ZERO, &mut log)
            .expect("the log is a Vec");
        assert_eq!(*escape.report(), Report::Sent);
        assert!(
            !escape.finished(),
            "a send just made is still waiting for its answer"
        );
    }

    /// **The log's own annotations must be inert if the log is ever replayed.** A capture is read
    /// back by [`progress`](crate::progress), which matches markers anywhere in a line, so a
    /// sentence this module writes into the log must not spell one. The board-facing half of the
    /// same hazard is that the escape is one byte and cannot spell anything at all.
    #[test]
    fn nothing_this_module_writes_into_a_log_reads_as_a_board_marker() {
        let (_, sent, _) = run(1, &[ARM_LINE, DISARM_LINE]);
        let (_, refused, _) = run(
            1,
            &["soak-test-reboot: FAILED: the firmware refused a cold reboot"],
        );
        let mut broken_port = Vec::new();
        let mut broken_log: Vec<u8> = Vec::new();
        Escape::new(1, &mut broken_port)
            .observe_line(ARM_LINE, Duration::ZERO, &mut broken_log)
            .expect("the log is a Vec");

        for log in [
            sent.as_str(),
            refused.as_str(),
            std::str::from_utf8(&broken_log).expect("ASCII"),
        ] {
            // Only the lines this module wrote, which are the ones beginning `board-console:`.
            for line in log.lines().filter(|l| l.starts_with("board-console:")) {
                let mut progress = crate::progress::BootProgress::new();
                progress.observe_line(line);
                assert_eq!(
                    progress.reached(),
                    crate::progress::Stage::Cold,
                    "an annotation reads as a boot stage: {line}"
                );
                assert!(
                    progress.failure().is_none(),
                    "an annotation reads as a failure: {line}"
                );
            }
        }
    }

    /// **The count this module keeps is the count [`lottery`](crate::lottery) keeps.** A log
    /// produced by `--stop-after n` has n draws in it by the tally's reckoning, which is the
    /// property the flag promises: a series with exactly the sample it was asked for.
    #[test]
    fn a_stop_after_n_log_tallies_as_n_draws() {
        const N: usize = 3;
        let mut capture = String::new();
        let mut port: Vec<u8> = Vec::new();
        let mut log: Vec<u8> = Vec::new();
        let mut escape = Escape::new(N, &mut port);

        // A rebooting series as the board prints it: SPL, the soak starting, the arming banner, a
        // beat, and the reset that opens the next draw.
        for draw in 0..N {
            for line in [
                "U-Boot SPL 2021.10 (Feb 12 2023 - 20:24:34 +0800)",
                START_LINE,
                ARM_LINE,
                BEAT_LINE,
            ] {
                capture.push_str(line);
                capture.push('\n');
                escape
                    .observe_line(line, Duration::from_secs(draw as u64), &mut log)
                    .expect("the log is a Vec");
            }
            if draw + 1 < N {
                let rebooting = "soak-test-reboot: rebooting now (SBI SRST system_reset, reset \
                                 type 1, cold reboot).";
                capture.push_str(rebooting);
                capture.push('\n');
            }
        }
        assert_eq!(escape.draws(), N);
        drop(escape);
        assert_eq!(port, vec![ESCAPE_BYTE], "sent once, on the third draw");

        capture.push_str(DISARM_LINE);
        capture.push('\n');
        let series = crate::lottery::tally(&capture);
        assert_eq!(
            series.draws.len(),
            N,
            "the tally and the stop mode must count the same thing"
        );
    }
}
