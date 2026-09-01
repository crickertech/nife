//! **How far did the boot get?**, decided from the console text alone.
//!
//! Every marker below is quoted from something in this tree rather than remembered: the bench
//! runbook in `notes/visionfive2.md` ("What appears, in order, on a good day" and the
//! failure-triage ladder), and `kernel/src/main.rs` and `kernel/src/panic.rs` for the two lines
//! that are ours. Nothing here was invented, which matters more than usual, because a recogniser
//! that matches text no board ever prints fails in the direction that looks like success.

use core::fmt;

/// How far the boot got, as an ordered ladder.
///
/// Ordered because the only question the tool ever asks is "did we reach at least here", and
/// because the sequence is what the runbook records: SPL, OpenSBI, U-Boot, handoff, ours. The
/// derived `Ord` is the comparison; do not reorder these variants without reading the runbook
/// again.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum Stage {
    /// Nothing recognisable yet. Either the board is unpowered, the adapter is on the wrong pins,
    /// or the baud is wrong; the failure-triage ladder's first row covers all three.
    #[default]
    Cold,
    /// U-Boot SPL announced itself. DRAM and the PLLs are up and we are running out of SRAM.
    Spl,
    /// OpenSBI's banner. M-mode firmware is resident and the SBI exists.
    OpenSbi,
    /// U-Boot proper: its banner, its countdown, or its `StarFive #` prompt.
    UBoot,
    /// `Starting kernel ...`. U-Boot has handed over and everything after this is ours.
    Handoff,
    /// Our own banner. The kernel's console works, which on this board is not a given: the runbook
    /// is explicit that this line is the *second* target, after the DW-8250 driver work.
    Banner,
}

impl Stage {
    /// The stage's name as a person would say it, for a report line.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Stage::Cold => "nothing recognisable",
            Stage::Spl => "U-Boot SPL",
            Stage::OpenSbi => "OpenSBI",
            Stage::UBoot => "U-Boot",
            Stage::Handoff => "kernel handoff",
            Stage::Banner => "kernel banner",
        }
    }
}

impl fmt::Display for Stage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// A failure the console text names outright, so the tool can stop now instead of waiting out a
/// deadline that will tell it nothing new.
///
/// Every one of these is a *positive* statement printed by something that is still alive. A board
/// that has gone silent is not in here; silence is the watcher's business, not the recogniser's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Failure {
    /// `Bad Linux RISCV Image magic!`: U-Boot refused the payload. Runbook row three: the file on
    /// the card is the ELF rather than the `objcopy` output, or the card is stale.
    BadImageMagic,
    /// `MEASURED BOOT REFUSED`: the kernel would not vouch for the archive it was handed. Boot 12
    /// (2026-08-15) is the worked example, and it was `script/board-image` building the pair in the
    /// wrong order rather than anything on the board.
    MeasuredBootRefused,
    /// `[PANIC] ...`: our own panic handler (`kernel/src/panic.rs`), carrying its message.
    KernelPanic(String),
}

impl Failure {
    /// One line naming what went wrong, for the report and the exit message.
    #[must_use]
    pub fn describe(&self) -> String {
        match self {
            Failure::BadImageMagic => {
                "U-Boot rejected the image header (Bad Linux RISCV Image magic!)".to_string()
            }
            Failure::MeasuredBootRefused => {
                "the kernel refused the archive at the measured-boot trust boundary".to_string()
            }
            Failure::KernelPanic(message) => format!("the kernel panicked: {message}"),
        }
    }
}

/// The ratchet: how far the boot got, and the first failure it announced.
///
/// It only ever moves forward. That is what lets the caller re-offer a partial line as more bytes
/// arrive without the recogniser double-counting, which it must do because U-Boot's `StarFive #`
/// prompt has no newline after it and would otherwise never be seen.
#[derive(Debug, Default, Clone)]
pub struct BootProgress {
    reached: Stage,
    failure: Option<Failure>,
    relocated: bool,
    banner_line: Option<String>,
}

impl BootProgress {
    /// A fresh ratchet, before any byte has arrived.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The furthest stage recognised so far.
    #[must_use]
    pub fn reached(&self) -> Stage {
        self.reached
    }

    /// The first announced failure, if the board announced one.
    #[must_use]
    pub fn failure(&self) -> Option<&Failure> {
        self.failure.as_ref()
    }

    /// Whether U-Boot said it moved the image (`Moving Image from ...`).
    ///
    /// Not a stage, because the runbook lists it as a *discriminator* rather than a step: it is
    /// what you check when `Starting kernel ...` is followed by silence.
    #[must_use]
    pub fn relocated(&self) -> bool {
        self.relocated
    }

    /// Our banner line exactly as it arrived, which is how the reader learns which architecture
    /// the board that answered actually is.
    #[must_use]
    pub fn banner_line(&self) -> Option<&str> {
        self.banner_line.as_deref()
    }

    /// Offer a line that ended with a line terminator.
    ///
    /// Idempotent: stages ratchet and the failure is recorded once, so the same text may be
    /// offered any number of times.
    pub fn observe_line(&mut self, line: &str) {
        self.observe(line, true);
    }

    /// Offer the incomplete tail, the bytes after the last line terminator.
    ///
    /// This exists because U-Boot's `StarFive #` prompt is printed with no newline after it, so a
    /// tool that waited for complete lines would sit there while the board sat waiting for it.
    ///
    /// **A partial line is not the same evidence as a complete one**, which is the distinction the
    /// two methods exist to keep, and it was found by a test feeding a good boot one byte at a
    /// time rather than by reasoning. Two things go wrong if a tail is treated as a line. `U-Boot `
    /// on its own looks like U-Boot proper right up until the next three bytes turn out to be
    /// `SPL`, so a board dying in SPL is reported two stages further along than it got. And any
    /// marker carrying a *payload* captures a truncated one: the tail `nife on ` recorded the
    /// banner as the empty string, and `[PANIC] ` recorded a panic with no message, both of them
    /// latched before the rest of the line arrived. So a tail may ratchet a stage, because a
    /// substring match is monotone and more bytes cannot unmake it; it may not settle a word
    /// boundary, and it may not capture text.
    pub fn observe_partial(&mut self, tail: &str) {
        self.observe(tail, false);
    }

    fn observe(&mut self, line: &str, complete: bool) {
        let line = line.trim_end_matches(['\r', '\n']);

        // Stages, cheapest first. `contains` rather than `starts_with` throughout, because a
        // console log interleaves output from more than one stage and a line can arrive with a
        // hart prefix or a partial line glued to its front.
        if line.contains("U-Boot SPL") {
            self.reach(Stage::Spl);
        }
        // OpenSBI's banner block ends with a "Platform Name" table, but the version line is the
        // one the runbook says to record, and it is the only line guaranteed to carry the word
        // followed by a version.
        if line.contains("OpenSBI v") {
            self.reach(Stage::OpenSbi);
        }
        if is_u_boot_proper(line, complete) || line.contains("StarFive #") {
            self.reach(Stage::UBoot);
        }
        if line.contains("Starting kernel ...") {
            self.reach(Stage::Handoff);
        }
        // `nife on ` rather than the RISC-V line specifically: `kernel/src/main.rs` prints one of
        // these per architecture, and a recogniser that only knew the VisionFive 2's would report
        // a healthy aarch64 or x86_64 board as never having booted. The full line is kept so the
        // reader sees which one answered, and only from a complete line, or it is kept truncated.
        if let Some(at) = line.find("nife on ") {
            self.reach(Stage::Banner);
            if complete && self.banner_line.is_none() {
                self.banner_line = Some(line[at..].to_string());
            }
        }

        if line.contains("Moving Image from") {
            self.relocated = true;
        }

        // Failures. Recorded once: the first thing that went wrong is the one worth reporting,
        // and everything after it is downstream.
        if self.failure.is_some() {
            return;
        }
        if line.contains("Bad Linux RISCV Image magic!") {
            self.failure = Some(Failure::BadImageMagic);
        } else if line.contains("MEASURED BOOT REFUSED") {
            self.failure = Some(Failure::MeasuredBootRefused);
        } else if complete && let Some(at) = line.find("[PANIC] ") {
            self.failure = Some(Failure::KernelPanic(
                line[at + "[PANIC] ".len()..].to_string(),
            ));
        }
    }

    fn reach(&mut self, stage: Stage) {
        if stage > self.reached {
            self.reached = stage;
        }
    }
}

/// Is this U-Boot proper announcing itself, rather than SPL or TPL?
///
/// The three banners all begin `U-Boot`, and telling them apart is the one place this recogniser
/// has to be careful: `U-Boot SPL 2021.10` and `U-Boot 2021.10` differ by one word. So: the word
/// after `U-Boot` decides, and `SPL`/`TPL` are the two that are not it.
///
/// `complete` is why this takes a second argument. In a partial line the word after `U-Boot ` may
/// not have finished arriving, and an unterminated word cannot be compared to `SPL`; the honest
/// answer there is "not yet", not "yes".
fn is_u_boot_proper(line: &str, complete: bool) -> bool {
    let Some(at) = line.find("U-Boot ") else {
        return false;
    };
    let rest = &line[at + "U-Boot ".len()..];
    let word = match rest.find(char::is_whitespace) {
        Some(end) => &rest[..end],
        // No whitespace after it: the word is finished only if the line is.
        None if complete => rest,
        None => return false,
    };
    !word.is_empty() && word != "SPL" && word != "TPL"
}

/// Splits a byte stream into lines, and hands back the incomplete tail as well.
///
/// The tail is the reason this exists rather than a call to [`str::lines`]. U-Boot's `StarFive #`
/// prompt is printed with no newline after it, so a console tool that only recognises complete
/// lines waits forever at exactly the moment the board is waiting for *it*.
#[derive(Debug, Default)]
pub struct LineFeeder {
    pending: String,
}

impl LineFeeder {
    /// A fresh splitter.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The bytes seen since the last line terminator, which have not been offered as a line.
    ///
    /// A caller that has stopped reading should hand this to [`BootProgress::observe_line`]: at
    /// the end of a session there is nothing more coming, so the tail is as complete as it will
    /// ever be. That is what catches a last line printed without a newline, which a panic followed
    /// by a halted machine can be.
    #[must_use]
    pub fn tail(&self) -> &str {
        &self.pending
    }

    /// Feed a chunk of bytes; returns every line it completed, plus the incomplete tail.
    ///
    /// Bytes are decoded lossily on purpose. A wrong baud rate produces bytes that are not UTF-8,
    /// and that is a case the tool must survive and report (runbook row two) rather than a case it
    /// may fail on.
    ///
    /// Both `\n` and `\r` end a line, because a serial console emits `\r\n` and because a firmware
    /// progress display emits bare `\r` to overwrite itself.
    pub fn feed(&mut self, bytes: &[u8]) -> Feeding {
        self.pending.push_str(&String::from_utf8_lossy(bytes));
        let mut lines = Vec::new();
        while let Some(at) = self.pending.find(['\n', '\r']) {
            let line = self.pending[..at].to_string();
            self.pending.drain(..=at);
            lines.push(line);
        }
        Feeding {
            lines,
            tail: self.pending.clone(),
        }
    }
}

/// What one chunk of bytes turned into: the lines it completed and the tail still in flight.
#[derive(Debug)]
pub struct Feeding {
    /// Lines that ended with `\n` or `\r`, in order.
    pub lines: Vec<String>,
    /// The bytes after the last line ending, which may still grow.
    pub tail: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Offer text to a fresh ratchet the way the watcher does, tail included.
    fn run(text: &str) -> BootProgress {
        let mut feeder = LineFeeder::new();
        let mut progress = BootProgress::new();
        // One byte at a time, which is the worst case a real UART delivers and the case that
        // catches a recogniser depending on chunk boundaries.
        for byte in text.as_bytes() {
            let feeding = feeder.feed(&[*byte]);
            for line in &feeding.lines {
                progress.observe_line(line);
            }
            progress.observe_partial(&feeding.tail);
        }
        progress
    }

    #[test]
    fn a_good_boot_reaches_our_banner() {
        let progress = run(include_str!("../tests/fixtures/vf2-good-boot.log"));
        assert_eq!(progress.reached(), Stage::Banner);
        assert_eq!(progress.failure(), None);
        assert!(progress.relocated());
        assert_eq!(
            progress.banner_line(),
            Some("nife on RISC-V (rv64, S-mode, Sv39)")
        );
    }

    #[test]
    fn silence_after_handoff_is_the_stage_it_stopped_at() {
        let progress = run(include_str!("../tests/fixtures/vf2-handoff-silence.log"));
        assert_eq!(progress.reached(), Stage::Handoff);
        assert_eq!(progress.failure(), None);
    }

    #[test]
    fn a_refused_archive_is_named_rather_than_timed_out() {
        let progress = run(include_str!("../tests/fixtures/vf2-measured-refusal.log"));
        assert_eq!(progress.reached(), Stage::Banner);
        assert_eq!(progress.failure(), Some(&Failure::MeasuredBootRefused));
    }

    #[test]
    fn a_stale_card_is_named_at_u_boot() {
        let progress = run(include_str!("../tests/fixtures/vf2-bad-magic.log"));
        assert_eq!(progress.reached(), Stage::UBoot);
        assert_eq!(progress.failure(), Some(&Failure::BadImageMagic));
    }

    #[test]
    fn a_panic_carries_its_message() {
        let progress = run(
            "Starting kernel ...\n\nnife on RISC-V (rv64, S-mode, Sv39)\n[PANIC] hart 3 took a load fault\n",
        );
        assert_eq!(progress.reached(), Stage::Banner);
        assert_eq!(
            progress.failure(),
            Some(&Failure::KernelPanic(
                "hart 3 took a load fault".to_string()
            ))
        );
    }

    /// The distinction the whole recogniser turns on, and the one a `contains("U-Boot")` gets
    /// wrong: SPL's banner must not be read as U-Boot proper, or a board that dies in SPL is
    /// reported two stages further along than it got.
    #[test]
    fn spl_is_not_u_boot_proper() {
        let progress = run("U-Boot SPL 2021.10 (Feb 12 2023 - 20:24:34 +0800)\n");
        assert_eq!(progress.reached(), Stage::Spl);
    }

    #[test]
    fn u_boot_proper_is_recognised_by_its_version() {
        let progress =
            run("U-Boot 2021.10 (Feb 12 2023 - 20:24:34 +0800), Build: jenkins-github\n");
        assert_eq!(progress.reached(), Stage::UBoot);
    }

    /// The prompt has no newline after it. A feeder that only reported complete lines would sit
    /// there while U-Boot sat waiting for a command.
    #[test]
    fn the_prompt_is_seen_without_a_newline() {
        let mut feeder = LineFeeder::new();
        let mut progress = BootProgress::new();
        let feeding = feeder.feed(b"Hit any key to stop autoboot:  0 \nStarFive # ");
        for line in &feeding.lines {
            progress.observe_line(line);
        }
        progress.observe_partial(&feeding.tail);
        assert_eq!(progress.reached(), Stage::UBoot);
        assert_eq!(feeding.tail, "StarFive # ");
    }

    /// A wrong baud rate is bytes, not text. The tool must keep going and log them.
    #[test]
    fn garbage_bytes_do_not_stop_the_feeder() {
        let mut feeder = LineFeeder::new();
        let mut progress = BootProgress::new();
        let feeding = feeder.feed(&[0xff, 0xfe, 0x80, b'\n', 0xc0]);
        for line in &feeding.lines {
            progress.observe_line(line);
        }
        progress.observe_partial(&feeding.tail);
        assert_eq!(progress.reached(), Stage::Cold);
        assert_eq!(feeding.lines.len(), 1);
    }

    /// A marker split across two reads is the normal case at 115200, not an edge one.
    #[test]
    fn a_marker_split_across_chunks_is_still_seen() {
        let mut feeder = LineFeeder::new();
        let mut progress = BootProgress::new();
        for chunk in [&b"Starting ke"[..], &b"rnel ...\n"[..]] {
            let feeding = feeder.feed(chunk);
            for line in &feeding.lines {
                progress.observe_line(line);
            }
            progress.observe_partial(&feeding.tail);
        }
        assert_eq!(progress.reached(), Stage::Handoff);
    }

    /// The bug the byte-at-a-time feeding found, kept as its own test because reasoning did not
    /// find it: mid-word, `U-Boot ` reads as U-Boot proper, and a payload captured from a tail is
    /// captured truncated.
    #[test]
    fn a_partial_line_does_not_settle_a_word_or_capture_text() {
        let mut progress = BootProgress::new();
        progress.observe_partial("U-Boot ");
        assert_eq!(progress.reached(), Stage::Cold);
        progress.observe_partial("U-Boot SPL 2021.10 (Feb");
        assert_eq!(progress.reached(), Stage::Spl);

        let mut progress = BootProgress::new();
        progress.observe_partial("nife on ");
        assert_eq!(progress.reached(), Stage::Banner);
        assert_eq!(progress.banner_line(), None);
        progress.observe_line("nife on RISC-V (rv64, S-mode, Sv39)");
        assert_eq!(
            progress.banner_line(),
            Some("nife on RISC-V (rv64, S-mode, Sv39)")
        );

        let mut progress = BootProgress::new();
        progress.observe_partial("[PANIC] hart 3 took");
        assert_eq!(progress.failure(), None);
        progress.observe_line("[PANIC] hart 3 took a load fault");
        assert_eq!(
            progress.failure(),
            Some(&Failure::KernelPanic(
                "hart 3 took a load fault".to_string()
            ))
        );
    }

    #[test]
    fn the_ratchet_never_goes_backwards() {
        let mut progress = BootProgress::new();
        progress.observe_line("Starting kernel ...");
        progress.observe_line("U-Boot SPL 2021.10");
        assert_eq!(progress.reached(), Stage::Handoff);
    }

    /// A carriage return alone ends a line: firmware progress indicators use it to overwrite.
    #[test]
    fn a_bare_carriage_return_ends_a_line() {
        let mut feeder = LineFeeder::new();
        let feeding =
            feeder.feed(b"Hit any key to stop autoboot:  2 \rHit any key to stop autoboot:  1 \r");
        assert_eq!(feeding.lines.len(), 2);
        assert_eq!(feeding.tail, "");
    }
}
