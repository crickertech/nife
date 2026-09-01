//! **The loop that cannot hang**, which is the whole difficulty of this milestone.
//!
//! Reading a serial port is one `open` and one `read`. Knowing when to stop reading is the part
//! that has kept every board milestone waiting on a person with eyes, and it has three answers,
//! all of which have to be here at once: the boot got where it was going, the board announced a
//! failure, or nothing more is coming.
//!
//! # Why the read happens on another thread
//!
//! Because the deadline must hold **whatever the reader does**. `port::configure` asks the tty
//! layer for a read timeout (`min 0 time 10`), and when that works a read returns on its own after
//! a tenth of a second. When it does not work, and there are several ordinary ways for it not to
//! (an `stty` that failed and was only warned about, a device that ignores the setting, a file or
//! a pipe standing in for a port), a read blocks forever and a single-threaded loop blocks with it.
//!
//! So the read runs on a worker and this loop waits on a channel with a timeout. The worker may
//! still be parked in `read` when we decide to stop; that is fine, because deciding to stop means
//! returning, and the process exits, and the descriptor goes with it. What must never happen is
//! this loop waiting on the board's goodwill, and it never does. This is `CLAUDE.md`'s
//! `Never leave QEMU running` rule wearing different clothes: an emulator that never exits and a
//! board that never speaks are the same bug from the tool's side.

use std::io::{self, Read, Write};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::thread;
use std::time::{Duration, Instant};

use crate::progress::{BootProgress, Failure, LineFeeder, Stage};

/// When to stop.
#[derive(Debug, Clone)]
pub struct Policy {
    /// The hard cap. The session always ends by here, and this is the promise that the tool does
    /// not hang.
    pub total: Duration,
    /// Stop early once the boot reaches this stage, if a stage was asked for.
    ///
    /// `None` means "watch for the whole duration", which is what sustained stress wants:
    /// `design/fatal-risks.md`'s multicore entry needs a long look, not a boot check.
    pub until: Option<Stage>,
    /// Stop if the board falls silent for this long **after it has said something**.
    ///
    /// The "after" is load-bearing. Before the first byte, silence means nobody has pressed power
    /// yet, and a tool that gave up on that would be unusable at a bench where the operator is
    /// three feet from the switch. Once bytes have flowed, silence is the runbook's
    /// `Starting kernel ...`-then-nothing row, which is what a multicore hang looks like.
    ///
    /// `None` disables it.
    pub quiet_after: Option<Duration>,
}

impl Default for Policy {
    fn default() -> Self {
        Self {
            total: Duration::from_secs(120),
            until: Some(Stage::Banner),
            quiet_after: Some(Duration::from_secs(15)),
        }
    }
}

/// Why the session ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// The stage the caller asked for arrived.
    Reached(Stage),
    /// The board said outright that something was wrong.
    Announced(Failure),
    /// It spoke, then stopped, for longer than [`Policy::quiet_after`].
    WentQuiet,
    /// The source ended: a replayed log ran out, or the device reported end of file.
    Ended,
    /// [`Policy::total`] elapsed.
    RanOut,
}

/// Everything one session learned, whether it succeeded or not.
#[derive(Debug)]
pub struct Session {
    /// Why it ended.
    pub outcome: Outcome,
    /// How far the boot got, and what it announced.
    pub progress: BootProgress,
    /// Bytes read from the port, which is the first thing to look at when a session is confusing:
    /// zero bytes is a wiring or a power question, not a software one.
    pub bytes: u64,
    /// Wall time from the first read to the decision.
    pub elapsed: Duration,
    /// The stage that was asked for, kept so the exit status can be computed here rather than
    /// re-derived by every caller.
    pub wanted: Option<Stage>,
}

impl Session {
    /// The process exit status this session deserves.
    ///
    /// `0` reached what was asked for (or, with no stage asked for, watched the whole duration
    /// without the board announcing a failure). `1` the board announced a failure. `2` it went
    /// quiet. `3` the time ran out with the requested stage unreached. Callers add `4` for a port
    /// that could not be opened, which is not a session at all.
    #[must_use]
    pub fn exit_code(&self) -> i32 {
        match &self.outcome {
            Outcome::Reached(_) => 0,
            Outcome::Announced(_) => 1,
            Outcome::WentQuiet => 2,
            // With no stage requested, running out of time IS the plan, and so is a replayed log
            // reaching its end. Neither is a failure unless something was being waited for.
            Outcome::RanOut | Outcome::Ended => {
                if self.wanted.is_some() {
                    3
                } else {
                    0
                }
            }
        }
    }

    /// One line a person can read, and the line the tool prints last.
    #[must_use]
    pub fn summary(&self) -> String {
        let reached = self.progress.reached();
        let head = match &self.outcome {
            Outcome::Reached(stage) => format!("reached {stage}"),
            Outcome::Announced(failure) => format!("failed: {}", failure.describe()),
            Outcome::WentQuiet => format!("went quiet after {reached}"),
            Outcome::Ended => format!("input ended after {reached}"),
            Outcome::RanOut => format!("time ran out after {reached}"),
        };
        format!(
            "{head} ({} bytes in {:.1}s)",
            self.bytes,
            self.elapsed.as_secs_f64()
        )
    }
}

/// What the reader worker hands back.
enum Read1 {
    Bytes(Vec<u8>),
    /// The source is finished. A file or a pipe reports this; a serial port configured with
    /// `min 0 time 10` does not, which is why `stream_never_ends` exists.
    Ended,
    Failed(io::Error),
}

/// Watch a source until the policy says to stop, copying every byte to `sink` as it arrives.
///
/// `stream_never_ends` says how to read a zero-byte result. For a serial port it is `true`: a
/// timed-out `read` returns nothing and the board is simply not talking yet. For a replayed log it
/// is `false`, where nothing means the file is over.
///
/// `sink` is written and flushed per chunk, deliberately, and the caller is expected to have it
/// pointed at a file. A console session whose evidence exists only in a terminal that has since
/// scrolled is the failure this tree keeps writing down; flushing per chunk is what makes the log
/// survive the tool being killed.
///
/// # Errors
///
/// Returns the sink's error if the log cannot be written, and the source's error if the read
/// failed for a reason other than reaching the end. A read error does **not** discard the session:
/// callers get the error, and the bytes are already in the log.
pub fn watch<R>(
    source: R,
    sink: &mut dyn Write,
    policy: &Policy,
    stream_never_ends: bool,
) -> io::Result<Session>
where
    R: Read + Send + 'static,
{
    let (tx, rx) = mpsc::channel();
    // Detached on purpose: it is never joined. It may be parked in `read` when this function
    // returns, and the only thing that unparks it is the process exiting, which closes the
    // descriptor. Joining it would reintroduce exactly the hang this design removes.
    thread::spawn(move || reader(source, &tx, stream_never_ends));

    let started = Instant::now();
    let mut feeder = LineFeeder::new();
    let mut progress = BootProgress::new();
    let mut bytes = 0u64;
    let mut spoke_at: Option<Instant> = None;
    let mut outcome = Outcome::RanOut;
    let mut error: Option<io::Error> = None;

    loop {
        let left = policy.total.saturating_sub(started.elapsed());
        if left.is_zero() {
            break;
        }
        // Never wait longer than the shortest thing we owe an answer about. The 250 ms floor keeps
        // the loop responsive without busy-waiting; nothing here depends on it being that number.
        let wait = left.min(Duration::from_millis(250));

        match rx.recv_timeout(wait) {
            Ok(Read1::Bytes(chunk)) => {
                bytes += chunk.len() as u64;
                spoke_at = Some(Instant::now());
                sink.write_all(&chunk)?;
                sink.flush()?;

                let feeding = feeder.feed(&chunk);
                for line in &feeding.lines {
                    progress.observe_line(line);
                }
                // The incomplete tail too, or U-Boot's newline-less `StarFive #` prompt is never
                // seen. Offered as a *partial*, which is a weaker kind of evidence for the reasons
                // `observe_partial` records; safe to re-offer as it grows, because the ratchet
                // only moves forward.
                progress.observe_partial(&feeding.tail);

                // Failure first, so a chunk that both reaches the banner and announces a refusal
                // (which is exactly what boot 12 looked like) reports the refusal.
                if let Some(failure) = progress.failure() {
                    outcome = Outcome::Announced(failure.clone());
                    break;
                }
                if let Some(wanted) = policy.until
                    && progress.reached() >= wanted
                {
                    outcome = Outcome::Reached(progress.reached());
                    break;
                }
            }
            Ok(Read1::Ended) => {
                outcome = Outcome::Ended;
                break;
            }
            Ok(Read1::Failed(e)) => {
                error = Some(e);
                outcome = Outcome::Ended;
                break;
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => {
                outcome = Outcome::Ended;
                break;
            }
        }

        if let (Some(limit), Some(last)) = (policy.quiet_after, spoke_at)
            && last.elapsed() >= limit
        {
            outcome = Outcome::WentQuiet;
            break;
        }
    }

    // Nothing more is coming, so the tail is as complete as it will ever be. This is what catches
    // a last line printed without a newline, which a panic on a machine that then halts can be.
    // It can only ever turn an inconclusive ending into a named failure, never the other way.
    if !feeder.tail().is_empty() {
        progress.observe_line(feeder.tail());
        if let Some(failure) = progress.failure()
            && matches!(
                outcome,
                Outcome::RanOut | Outcome::WentQuiet | Outcome::Ended
            )
        {
            outcome = Outcome::Announced(failure.clone());
        }
    }

    let session = Session {
        outcome,
        progress,
        bytes,
        elapsed: started.elapsed(),
        wanted: policy.until,
    };
    match error {
        Some(e) => Err(io::Error::new(
            e.kind(),
            format!("{}: {e}", session.summary()),
        )),
        None => Ok(session),
    }
}

fn reader<R: Read>(mut source: R, tx: &mpsc::Sender<Read1>, stream_never_ends: bool) {
    let mut buffer = [0u8; 4096];
    loop {
        match source.read(&mut buffer) {
            Ok(0) => {
                if stream_never_ends {
                    // A configured port with `min 0 time 10` already blocked for a tenth of a
                    // second before returning nothing. If the configuration did not take, this
                    // sleep is what stops the loop spinning a core while the board is quiet.
                    thread::sleep(Duration::from_millis(20));
                    continue;
                }
                let _ = tx.send(Read1::Ended);
                return;
            }
            Ok(n) => {
                if tx.send(Read1::Bytes(buffer[..n].to_vec())).is_err() {
                    return;
                }
            }
            Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
            Err(e) => {
                let _ = tx.send(Read1::Failed(e));
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A source that blocks forever without ever yielding a byte, which is what a board that is
    /// powered off looks like through a port whose read timeout did not take. This is the one the
    /// whole threaded design exists for.
    struct NeverSpeaks;

    impl Read for NeverSpeaks {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            thread::sleep(Duration::from_secs(3600));
            Ok(0)
        }
    }

    /// Speaks once, then blocks forever: the runbook's `Starting kernel ...`-then-silence row, and
    /// what a multicore hang looks like from the far end of a serial cable.
    struct SpeaksThenStops(Option<&'static [u8]>);

    impl Read for SpeaksThenStops {
        fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
            match self.0.take() {
                Some(text) => {
                    out[..text.len()].copy_from_slice(text);
                    Ok(text.len())
                }
                None => {
                    thread::sleep(Duration::from_secs(3600));
                    Ok(0)
                }
            }
        }
    }

    fn quick(until: Option<Stage>, quiet_after: Option<Duration>) -> Policy {
        Policy {
            total: Duration::from_millis(1500),
            until,
            quiet_after,
        }
    }

    #[test]
    fn a_good_boot_is_recognised_and_stops_early() {
        let log = include_bytes!("../tests/fixtures/vf2-good-boot.log");
        let mut sink = Vec::new();
        let policy = Policy {
            total: Duration::from_secs(10),
            ..Policy::default()
        };
        let session = watch(&log[..], &mut sink, &policy, false).unwrap();
        assert_eq!(session.outcome, Outcome::Reached(Stage::Banner));
        assert_eq!(session.exit_code(), 0);
        assert!(!sink.is_empty());
    }

    /// The evidence has to be on disk whatever happened, and this is the case where it matters
    /// most: the session failed and the only account of it is the log.
    #[test]
    fn the_log_is_written_even_when_the_boot_fails() {
        let log = include_bytes!("../tests/fixtures/vf2-bad-magic.log");
        let mut sink = Vec::new();
        let session = watch(&log[..], &mut sink, &Policy::default(), false).unwrap();
        assert_eq!(session.outcome, Outcome::Announced(Failure::BadImageMagic));
        assert_eq!(session.exit_code(), 1);
        assert!(String::from_utf8_lossy(&sink).contains("Bad Linux RISCV Image magic!"));
    }

    /// The banner arrives and the refusal is on the next line. Stopping at the banner would report
    /// a boot that halted at the trust boundary as a success, which is boot 12 exactly.
    #[test]
    fn a_failure_beats_the_stage_it_was_waiting_for() {
        let log = include_bytes!("../tests/fixtures/vf2-measured-refusal.log");
        let mut sink = Vec::new();
        let session = watch(&log[..], &mut sink, &Policy::default(), false).unwrap();
        assert_eq!(
            session.outcome,
            Outcome::Announced(Failure::MeasuredBootRefused)
        );
    }

    /// The property the milestone is actually about: whatever the board does, the tool returns.
    #[test]
    fn a_source_that_never_speaks_does_not_hang_the_tool() {
        let mut sink = Vec::new();
        let started = Instant::now();
        let session = watch(
            NeverSpeaks,
            &mut sink,
            &quick(Some(Stage::Banner), None),
            true,
        )
        .unwrap();
        assert_eq!(session.outcome, Outcome::RanOut);
        assert_eq!(session.exit_code(), 3);
        assert_eq!(session.bytes, 0);
        assert!(started.elapsed() < Duration::from_secs(10));
    }

    /// Silence only counts once the board has spoken. A tool that started the clock at `open`
    /// would give up while the operator was still walking to the power switch.
    #[test]
    fn silence_before_the_first_byte_is_not_a_hang() {
        let mut sink = Vec::new();
        let policy = quick(Some(Stage::Banner), Some(Duration::from_millis(200)));
        let session = watch(NeverSpeaks, &mut sink, &policy, true).unwrap();
        assert_eq!(session.outcome, Outcome::RanOut);
    }

    #[test]
    fn silence_after_the_handoff_is_a_hang() {
        let mut sink = Vec::new();
        let policy = Policy {
            total: Duration::from_secs(10),
            until: Some(Stage::Banner),
            quiet_after: Some(Duration::from_millis(300)),
        };
        let source = SpeaksThenStops(Some(b"Moving Image from 0x40200000\nStarting kernel ...\n"));
        let session = watch(source, &mut sink, &policy, true).unwrap();
        assert_eq!(session.outcome, Outcome::WentQuiet);
        assert_eq!(session.exit_code(), 2);
        assert_eq!(session.progress.reached(), Stage::Handoff);
        assert!(session.progress.relocated());
    }

    /// Sustained watching, which is what `design/fatal-risks.md`'s multicore entry needs: no stage
    /// to wait for, so running the clock out is the plan rather than a disappointment.
    #[test]
    fn watching_for_a_duration_ends_successfully() {
        let mut sink = Vec::new();
        let session = watch(NeverSpeaks, &mut sink, &quick(None, None), true).unwrap();
        assert_eq!(session.outcome, Outcome::RanOut);
        assert_eq!(session.exit_code(), 0);
    }

    /// A replayed log that ends before the banner is not a hang, and saying so keeps the two
    /// apart in the exit status.
    #[test]
    fn a_replay_that_runs_out_ends_rather_than_going_quiet() {
        let log = include_bytes!("../tests/fixtures/vf2-handoff-silence.log");
        let mut sink = Vec::new();
        let session = watch(&log[..], &mut sink, &Policy::default(), false).unwrap();
        assert_eq!(session.outcome, Outcome::Ended);
        assert_eq!(session.exit_code(), 3);
        assert_eq!(session.progress.reached(), Stage::Handoff);
    }
}
