//! **How far did the boot get?**, decided from the console text alone.
//!
//! Every marker below is quoted from something in this tree rather than remembered: the bench
//! runbook in `notes/visionfive2.md` ("What appears, in order, on a good day" and the
//! failure-triage ladder), and `kernel/src/main.rs` and `kernel/src/panic.rs` for the lines that
//! are ours. Nothing here was invented, which matters more than usual, because a recogniser that
//! matches text no board ever prints fails in the direction that looks like success.
//!
//! **And every one of them has now been checked against the board**, which is a different and
//! better claim than "quoted from documentation" and was not available when this was written.
//! calef captured a full boot and a full failure on 2026-09-01; both are in
//! `tests/fixtures/captured/` and both are asserted on. The documentation was right about the
//! seven markers it named. It was silent about two things the board does, and both are here now:
//! U-Boot refusing outright before the kernel runs, and the line that means our whole boot tour
//! finished rather than merely started.

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
    /// The boot tour ran to its end (`nife: the capability core runs on ...`).
    ///
    /// A stronger signal than the banner and a different claim: the banner is printed before the
    /// device tree is touched, so it says the console works and nothing else, while this says
    /// paging, traps, the timer, the frame allocator, SMP and the scheduler all came up. It is
    /// **not** the default to wait for, because only the milestone-tour build prints it; a shell
    /// or a test build reaches its banner and then does something else entirely.
    Tour,
    /// A sustained workload announced itself and is expected to keep speaking (milestone 219).
    ///
    /// **This is the only stage after which silence is a failure again.** Every stage below it is
    /// a step in a boot that ends with the kernel halting in `wfi`, so quiet after [`Stage::Tour`]
    /// is how a good boot ends and reporting it as a hang would fail every one. A soak is the
    /// opposite contract: `kernel/src/soak.rs` prints a heartbeat on the wall clock every five
    /// seconds whatever the workload is doing, so a gap says the thing that prints is itself
    /// wedged. See `watch::Policy::quiet_after`, which is where that asymmetry is implemented.
    Soak,
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
            Stage::Tour => "boot tour complete",
            Stage::Soak => "soak running",
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
    /// `### ERROR ### Please RESET the board ###`: U-Boot gave up before the kernel ever ran,
    /// carrying the line before it, which is where U-Boot says why.
    ///
    /// **This is the third outcome, and it is the one documentation did not have.** Captured on
    /// 2026-09-01 from the extlinux path, where `Device tree not found or missing FDT support` is
    /// the reason and is exactly the caveat `notes/visionfive2.md` records about U-Boot's fallback
    /// DTB addresses. It follows `Moving Image from`, so a recogniser that stopped at the stages
    /// would have watched the image load and then called the silence a hang. It is not a hang. The
    /// board is sitting at a firmware error waiting to be reset, and saying so is the difference
    /// between resetting it and going looking for a multicore bug.
    UBootRefused(String),
}

impl Failure {
    /// One line naming what went wrong, for the report and the exit message.
    #[must_use]
    pub fn describe(&self) -> String {
        match self {
            Failure::BadImageMagic => {
                "U-Boot rejected the image header (Bad Linux RISCV Image magic!)".to_string()
            }
            // Worded carefully, because this is the one failure that is not a defect. The gate
            // did its job: it noticed that the archive on the card is not the one this kernel was
            // built to vouch for, and halted instead of running it. A report that read like a
            // crash would send somebody debugging the boot mechanism, when what is wrong is that
            // two files came from different builds.
            Failure::MeasuredBootRefused => "the measured-boot gate refused the archive and \
                 halted, which is the gate working rather than a crash: the kernel and the archive \
                 on the card are from different builds. Rebuild both with script/board-image, \
                 which orders those steps"
                .to_string(),
            Failure::KernelPanic(message) => format!("the kernel panicked: {message}"),
            Failure::UBootRefused(reason) if reason.is_empty() => {
                "U-Boot gave up before the kernel ran and wants the board reset".to_string()
            }
            Failure::UBootRefused(reason) => {
                format!("U-Boot gave up before the kernel ran and wants the board reset: {reason}")
            }
        }
    }
}

/// One soak heartbeat's numbers (milestone 219), as the kernel printed them.
///
/// Every field is cumulative except [`rate`](Self::rate), which is the last interval's. The one a
/// later run is compared against is [`rounds`](Self::rounds); the rest are what make it
/// interpretable, and [`refused`](Self::refused) is the one that is a finding rather than a
/// statistic.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct SoakBeat {
    /// Seconds since the soak started, by the kernel's own timer.
    pub seconds: u64,
    /// Which heartbeat this is. A gap in this sequence means console output was lost, which is a
    /// different fault from the workload stopping and is worth being able to tell apart.
    pub beat: u64,
    /// **Cumulative IPC round trips completed by every worker.** The comparable number.
    pub rounds: u64,
    /// Round trips per second over the last interval.
    pub rate: u64,
    /// Cumulative refused wakes. Expected to be zero, and a nonzero value is the defect
    /// `design/fatal-risks.md`'s multicore entry exists for.
    pub refused: u64,
    /// Cumulative wrong replies seen by callers. Expected to be zero.
    pub mismatches: u64,
    /// How many workers made no progress in the last interval. Expected to be zero.
    pub stalled: u64,
    /// **Cumulative times a thread ran on a different core than the one it last ran on.** The
    /// honest cross-core handoff count; see `kernel/src/soak.rs` and `notes/soak.md` for the
    /// measured reason a steady-state workload barely moves this at all.
    pub crossings: u64,
    /// Cumulative wakes and placements that named a remote core. Narrower than
    /// [`crossings`](Self::crossings) and kept beside it because the two disagreeing is the
    /// finding: a rendezvous wake queues its peer locally, so a migration it performs is invisible
    /// here.
    pub remote: u64,
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
    userspace_ran: bool,
    banner_line: Option<String>,
    /// The most recent soak heartbeat's numbers (milestone 219), so the tool can put the run's own
    /// figure in its summary instead of making a reader go back to a log for it. `None` until a
    /// heartbeat has been seen and parsed.
    soak: Option<SoakBeat>,
    /// The last complete non-empty line, kept for exactly one reason: U-Boot's `### ERROR ###`
    /// says that it gave up and the line before it says why, and a reader handed only the first
    /// half has to go back to the log to learn anything.
    last_line: String,
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

    /// Whether userspace init built its child (`init/build  : ...`, `kernel/src/main.rs`).
    ///
    /// Not a stage, and deliberately, because the ladder has to stay a ladder: a kernel with no
    /// archive on the card runs its whole tour and never reaches this, so putting it below
    /// [`Stage::Tour`] would make reaching the tour imply something that did not happen. It is a
    /// detail of a successful boot, like [`Self::relocated`], and it is the difference between the
    /// two successful captures.
    #[must_use]
    pub fn userspace_ran(&self) -> bool {
        self.userspace_ran
    }

    /// Whether U-Boot said it moved the image (`Moving Image from ...`).
    ///
    /// Not a stage, because the runbook lists it as a *discriminator* rather than a step: it is
    /// what you check when `Starting kernel ...` is followed by silence.
    #[must_use]
    pub fn relocated(&self) -> bool {
        self.relocated
    }

    /// The latest soak heartbeat, if this session saw one.
    ///
    /// **This is the number milestone 219 is for.** A soak that ends with nothing printed proves
    /// very little; one that ends with a round-trip total is something a later run can be compared
    /// against. It is a progress figure and nothing else: see
    /// `design/roadmap/219-a-workload-that-does-not-stop.md` for why a clean run is weak evidence.
    #[must_use]
    pub fn soak(&self) -> Option<&SoakBeat> {
        self.soak.as_ref()
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
        if line.contains("init/build") {
            self.userspace_ran = true;
        }
        if let Some(at) = line.find("nife: the capability core runs on ") {
            self.reach(Stage::Tour);
            if complete && self.banner_line.is_none() {
                self.banner_line = Some(line[at..].to_string());
            }
        }
        // The soak (milestone 219). `soak: started` is `kernel/src/soak.rs`'s `START_MARKER`, and
        // the two agree by one of them being tested against the other's text rather than by both
        // being remembered. Ratcheting on the START line rather than on any `soak:` line is
        // deliberate: a `soak: FAILED` line reaches the failure arm below and should not also be
        // read as the workload having got going.
        if line.contains("soak: started") {
            self.reach(Stage::Soak);
        }
        if complete && line.contains("soak: t=") {
            self.observe_soak_beat(line);
        }

        // Failures. Recorded once: the first thing that went wrong is the one worth reporting,
        // and everything after it is downstream.
        if self.failure.is_none() {
            if line.contains("Bad Linux RISCV Image magic!") {
                self.failure = Some(Failure::BadImageMagic);
            } else if line.contains("MEASURED BOOT REFUSED") {
                self.failure = Some(Failure::MeasuredBootRefused);
            } else if line.contains("### ERROR ### Please RESET the board ###") {
                // Deliberately reads `last_line` before the update below, because U-Boot puts the
                // refusal on one line and its reason on the one before.
                self.failure = Some(Failure::UBootRefused(self.last_line.clone()));
            } else if complete && let Some(at) = line.find("[PANIC] ") {
                self.failure = Some(Failure::KernelPanic(
                    line[at + "[PANIC] ".len()..].to_string(),
                ));
            }
        }

        // Last, and only for a complete line: this is what the NEXT line may need, so recording a
        // partial here would hand the refusal a truncated reason.
        if complete && !line.trim().is_empty() {
            self.last_line = line.trim().to_string();
        }
    }

    /// Pull the numbers out of one `soak: t=... beat=... rounds=...` line.
    ///
    /// Field-name-directed rather than positional, so adding a field to the kernel's heartbeat does
    /// not silently shift what this reads. A field that is missing or unparseable leaves the
    /// previous value in place rather than zeroing it, because a garbled line on a serial link is
    /// a lost measurement, not a measurement of zero.
    fn observe_soak_beat(&mut self, line: &str) {
        let field = |name: &str| -> Option<u64> {
            let at = line.find(name)?;
            let rest = &line[at + name.len()..];
            let end = rest
                .find(|c: char| !c.is_ascii_digit())
                .unwrap_or(rest.len());
            rest[..end].parse().ok()
        };
        let beat = self.soak.get_or_insert_with(SoakBeat::default);
        for (name, slot) in [
            ("t=", &mut beat.seconds),
            ("beat=", &mut beat.beat),
            ("rounds=", &mut beat.rounds),
            ("rate=", &mut beat.rate),
            ("refused=", &mut beat.refused),
            ("mismatch=", &mut beat.mismatches),
            ("stalled=", &mut beat.stalled),
            ("crossings=", &mut beat.crossings),
            ("remote=", &mut beat.remote),
        ] {
            if let Some(v) = field(name) {
                *slot = v;
            }
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

    /// **The real thing**: bytes off the wire on 2026-09-01, control characters and all, fed one
    /// byte at a time. Everything else in this file is a unit test; this is the only one that
    /// says the markers are the text a board prints.
    #[test]
    fn the_captured_boot_runs_the_whole_tour() {
        let progress = run(include_str!(
            "../tests/fixtures/captured/vf2-2026-09-01-manual-boot.log"
        ));
        assert_eq!(progress.reached(), Stage::Tour);
        assert_eq!(progress.failure(), None);
        assert!(progress.relocated());
        assert!(
            !progress.userspace_ran(),
            "this card carried no archive, and the tour says so"
        );
        assert_eq!(
            progress.banner_line(),
            Some("nife on RISC-V (rv64, S-mode, Sv39)")
        );
    }

    /// The third outcome, and the one no documentation in this tree described: U-Boot gives up
    /// **after** loading and relocating the image and **before** the kernel runs. It is not a
    /// hang, and reporting it as one would send somebody hunting a multicore bug in a kernel that
    /// never executed an instruction.
    #[test]
    fn the_captured_extlinux_failure_is_u_boot_giving_up() {
        let progress = run(include_str!(
            "../tests/fixtures/captured/vf2-2026-09-01-extlinux-refused.log"
        ));
        assert_eq!(progress.reached(), Stage::UBoot);
        assert!(progress.relocated(), "the image did load and relocate");
        assert_eq!(
            progress.failure(),
            Some(&Failure::UBootRefused(
                "Device tree not found or missing FDT support".to_string()
            )),
            "the reason is the line before the ERROR, and a reader needs it"
        );
    }

    /// The banner is not the end of the story, and the two captures differ by exactly this: both
    /// reach U-Boot, one reaches the banner and then the tour, the other never reaches either.
    #[test]
    fn the_tour_line_outranks_the_banner() {
        let mut progress = BootProgress::new();
        progress.observe_line("nife on RISC-V (rv64, S-mode, Sv39)");
        assert_eq!(progress.reached(), Stage::Banner);
        progress.observe_line("nife: the capability core runs on RISC-V.");
        assert_eq!(progress.reached(), Stage::Tour);
    }

    /// The trust boundary refusing, captured rather than imagined. Note where it got to: past the
    /// banner and well into the tour, which is what makes "reached the banner" a useless test for
    /// whether a boot worked.
    #[test]
    fn the_captured_refusal_gets_past_the_banner_before_refusing() {
        let progress = run(include_str!(
            "../tests/fixtures/captured/vf2-2026-09-01-measured-boot-refused.log"
        ));
        assert_eq!(progress.reached(), Stage::Banner);
        assert_eq!(progress.failure(), Some(&Failure::MeasuredBootRefused));
        assert!(
            !progress.userspace_ran(),
            "it halted rather than handing the archive to init"
        );
        // The wording is load-bearing: this is the gate working, and a message that read like a
        // crash would send somebody debugging the boot mechanism instead of their build.
        let said = progress.failure().unwrap().describe();
        assert!(said.contains("the gate working"));
        assert!(said.contains("script/board-image"));
    }

    /// The other successful capture, and the difference between the two: this one had an archive
    /// on the card, so `init` built a child and a userspace driver came up.
    #[test]
    fn the_captured_userspace_boot_ran_a_child() {
        let progress = run(include_str!(
            "../tests/fixtures/captured/vf2-2026-09-01-userspace.log"
        ));
        assert_eq!(progress.reached(), Stage::Tour);
        assert_eq!(progress.failure(), None);
        assert!(progress.userspace_ran());
    }

    #[test]
    fn a_stale_card_is_named_at_u_boot() {
        let progress = run(include_str!(
            "../tests/fixtures/synthetic/vf2-bad-magic.log"
        ));
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
