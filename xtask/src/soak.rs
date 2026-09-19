//! The sustained runs under QEMU: the multicore soak (milestone 219) and the multi-tasking
//! job mix (milestone 168).
//!
//! Both return their own exit codes rather than a bool, because a rehearsal that cannot say
//! *how* it failed is not a rehearsal.

use std::path::PathBuf;
use std::process::{Command, ExitCode};
use std::sync::atomic::Ordering;

use crate::archive::{initrd_path, initrd_riscv, initrd_x86, riscv_initrd_path, x86_initrd_path};
use crate::board::{Tee, parse_duration};
use crate::disk::mkdisk;
use crate::host::workspace_root;
use crate::{
    RELEASE, RISCV_TARGET, TARGET, X86_TARGET, cargo_profiled, maybe_hvf, profile_dir, user,
};

/// **The QEMU rehearsal of milestone 168's multi-tasking workload sweep.** Boot a
/// `--features job_mix` kernel, echo its lines, stop when it says it is done, and kill it.
///
/// **This is a rehearsal and not the measurement**, and the distinction is the milestone's whole
/// gate. Under TCG the magnitudes are fiction (no caches are modelled) and under HVF the host
/// scheduler is underneath every guest thread; the number DECISIONS §96 is waiting for is taken on
/// radon, by `notes/job-mix.md`'s procedure. What this proves is that the workload runs, that the
/// sweep completes, and that the output is the shape the bench evening will read.
///
/// The marker loop is `run_bench`'s, for `run_bench`'s reason: the kernel parks in `wfi` rather
/// than exiting, so the host side owns the process and tears it down when it sees the done line.
/// See `script/job-mix`.
pub(crate) fn job_mix_sweep() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(2).collect();
    let mut arch = "aarch64".to_string();
    let mut smp: Option<String> = None;
    // `--hvf` and `--release` (added 2026-09-19 for the HVF cross-check in notes/job-mix.md): the
    // two flags the tree already spells this way, `run`'s and `bench`'s, rather than new ones. HVF
    // is aarch64 on an Apple host only, and release is what `script/board-image` builds for radon,
    // so the cross-check runs the optimisation level the board does.
    let mut hvf = false;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--hvf" => {
                hvf = true;
                i += 1;
                continue;
            }
            "--release" => {
                RELEASE.store(true, Ordering::Relaxed);
                i += 1;
                continue;
            }
            _ => {}
        }
        let value = |i: usize| -> Result<&str, ExitCode> {
            args.get(i + 1).map(String::as_str).ok_or_else(|| {
                eprintln!("job-mix: {} wants a value", args[i]);
                ExitCode::from(4)
            })
        };
        match args[i].as_str() {
            "--arch" => match value(i) {
                Ok(v) => arch = v.to_string(),
                Err(code) => return code,
            },
            "--smp" => match value(i) {
                Ok(v) => smp = Some(v.to_string()),
                Err(code) => return code,
            },
            other => {
                eprintln!("job-mix: unknown argument {other}");
                eprintln!(
                    "usage: cargo xtask job-mix [--arch aarch64|riscv64|x86_64] [--smp <n>] [--hvf] [--release]"
                );
                return ExitCode::from(4);
            }
        }
        i += 2;
    }

    if hvf && arch != "aarch64" {
        eprintln!("job-mix: --hvf runs the Apple core, so it is aarch64 only");
        return ExitCode::from(4);
    }
    if hvf {
        maybe_hvf();
    }

    let (target, runner, initrd) = match arch.as_str() {
        "aarch64" => {
            if !(mkdisk() && user()) {
                return ExitCode::from(4);
            }
            (TARGET, "scripts/qemu-runner-aarch64.sh", initrd_path())
        }
        // **`mkdisk` here too, not only on aarch64** (found 2026-09-19 by the lane that closed
        // milestone 168's sampling hole): the riscv64 runner refuses a `NIFE_DISK` naming a missing
        // file, so on a fresh worktree `--arch riscv64` died before the kernel printed a line, and
        // it only ever passed on a checkout where an aarch64 run had made the image first. The
        // sweep reads no disk; the runner's own check is what needs it.
        "riscv64" => {
            if !(mkdisk() && initrd_riscv()) {
                return ExitCode::from(4);
            }
            (
                RISCV_TARGET,
                "scripts/qemu-runner-riscv64.sh",
                riscv_initrd_path(),
            )
        }
        "x86_64" => {
            if !initrd_x86() {
                return ExitCode::from(4);
            }
            (
                X86_TARGET,
                "scripts/qemu-runner-x86_64.sh",
                x86_initrd_path(),
            )
        }
        other => {
            eprintln!("job-mix: unknown architecture {other} (aarch64, riscv64 or x86_64)");
            return ExitCode::from(4);
        }
    };

    if !cargo_profiled(&[
        "build",
        "-p",
        "kernel",
        "--features",
        "job_mix",
        "--target",
        target,
    ]) {
        return ExitCode::from(4);
    }

    let mut cmd = Command::new(runner);
    cmd.arg(format!(
        "{}/target/{target}/{}/kernel",
        workspace_root().display(),
        profile_dir()
    ));
    cmd.env("NIFE_INITRD", &initrd);
    if let Some(n) = &smp {
        cmd.env("NIFE_SMP", n);
    }
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::inherit());

    eprintln!(
        "--- job-mix: {arch}, a rehearsal; the number is taken on radon (notes/job-mix.md) ---"
    );
    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("job-mix: cannot start {runner}: {e}");
            return ExitCode::from(4);
        }
    };
    let runner_pid = child.id();
    let Some(stdout) = child.stdout.take() else {
        eprintln!("job-mix: the runner gave us no stdout to read");
        let _ = child.kill();
        return ExitCode::from(4);
    };

    use std::io::BufRead;
    let mut done = false;
    let mut points = 0usize;
    let mut failed = false;
    for line in std::io::BufReader::new(stdout).lines() {
        let Ok(line) = line else { break };
        if line.starts_with("job-mix") {
            println!("{line}");
        }
        if line.contains("job-mix: FAILED") {
            failed = true;
        }
        if line.starts_with("job-mix: tasks=") {
            points += 1;
        }
        if line.trim_end() == "job-mix: done" {
            done = true;
            break;
        }
    }
    // The children first and then the wrapper, `run_bench`'s order and for its reason: the x86
    // runner does not `exec`, so killing the wrapper alone orphans the emulator.
    let _ = Command::new("pkill")
        .args(["-9", "-P", &runner_pid.to_string()])
        .status();
    let _ = child.kill();
    let _ = child.wait();

    if failed {
        eprintln!("job-mix: the kernel refused to start the sweep; see the lines above");
        return ExitCode::from(1);
    }
    if !done {
        eprintln!("job-mix: QEMU ended before printing `job-mix: done`; {points} point(s) printed");
        return ExitCode::from(3);
    }
    eprintln!();
    eprintln!("job-mix: the sweep completed, {points} point(s).");
    eprintln!(
        "job-mix: these magnitudes are NOT the measurement. TCG models no cache and HVF puts a \
         host scheduler under every guest thread; DECISIONS \u{a7}96's number is taken on radon."
    );
    ExitCode::SUCCESS
}

/// **The QEMU half of milestone 219's sustained run.** Boot a `--features soak_test` kernel, watch it
/// with the same recogniser and the same policy `script/board-console` points at a real board, and
/// return the same exit statuses.
///
/// One recogniser, two sources, is the whole design. The alternative was a QEMU-side checker of its
/// own, and it would have drifted from the board-side one the first time either changed; milestone
/// 219's block is explicit that the workload and the console must agree about what a hang is, and
/// the cheapest way for two things to agree is for there to be one of them.
///
/// What this adds over `board_console` is only what a board does not need: building the kernel and
/// the archive, starting QEMU, and killing it afterwards. See `script/soak-test`.
///
/// Named for the command rather than for the workload (milestone 297): this function *is*
/// `script/soak-test`, where `BootProgress::soak` one crate over reports on the workload, which is
/// still a soak and keeps that spelling.
pub(crate) fn soak_test() -> ExitCode {
    use std::io::Write;
    use std::time::Duration;

    use board_console::watch::{Outcome, Policy, watch};

    let args: Vec<String> = std::env::args().skip(2).collect();
    let mut arch = "aarch64".to_string();
    let mut smp: Option<String> = None;
    let mut log: Option<PathBuf> = None;
    // A minute by default: long enough that the beat, the rate and the cross-core counters are all
    // real numbers rather than a first sample, and short enough that nobody is tempted to skip it.
    // The runs that matter are hours long and happen on a board.
    let mut policy = Policy {
        total: Duration::from_secs(60),
        until: None,
        quiet_after: Some(Duration::from_secs(15)),
        settle: Duration::from_secs(0),
    };

    let mut i = 0;
    while i < args.len() {
        let value = |i: usize| -> Result<&str, ExitCode> {
            args.get(i + 1).map(String::as_str).ok_or_else(|| {
                eprintln!("soak-test: {} wants a value", args[i]);
                ExitCode::from(4)
            })
        };
        match args[i].as_str() {
            "--arch" => match value(i) {
                Ok(v) => arch = v.to_string(),
                Err(code) => return code,
            },
            "--smp" => match value(i) {
                Ok(v) => smp = Some(v.to_string()),
                Err(code) => return code,
            },
            "--log" => match value(i) {
                Ok(v) => log = Some(PathBuf::from(v)),
                Err(code) => return code,
            },
            "--for" | "--timeout" => match value(i).map(parse_duration) {
                Ok(Some(d)) => policy.total = d,
                Ok(None) => {
                    eprintln!("soak-test: --for wants a duration like 90, 90s, 30m or 2h");
                    return ExitCode::from(4);
                }
                Err(code) => return code,
            },
            "--quiet-after" => match value(i).map(parse_duration) {
                Ok(Some(d)) => policy.quiet_after = if d.is_zero() { None } else { Some(d) },
                Ok(None) => {
                    eprintln!("soak-test: --quiet-after wants a duration, or 0 to disable");
                    return ExitCode::from(4);
                }
                Err(code) => return code,
            },
            other => {
                eprintln!("soak-test: unknown argument {other}");
                eprintln!(
                    "usage: cargo xtask soak-test [--arch aarch64|riscv64|x86_64] [--for <duration>] \
                     [--smp <n>] [--quiet-after <duration>] [--log <file>]"
                );
                return ExitCode::from(4);
            }
        }
        i += 2;
    }

    let (target, runner, initrd) = match arch.as_str() {
        "aarch64" => {
            if !(mkdisk() && user()) {
                return ExitCode::from(4);
            }
            (TARGET, "scripts/qemu-runner-aarch64.sh", initrd_path())
        }
        "riscv64" => {
            if !initrd_riscv() {
                return ExitCode::from(4);
            }
            (
                RISCV_TARGET,
                "scripts/qemu-runner-riscv64.sh",
                riscv_initrd_path(),
            )
        }
        "x86_64" => {
            if !initrd_x86() {
                return ExitCode::from(4);
            }
            (
                X86_TARGET,
                "scripts/qemu-runner-x86_64.sh",
                x86_initrd_path(),
            )
        }
        other => {
            eprintln!("soak-test: unknown architecture {other} (aarch64, riscv64 or x86_64)");
            return ExitCode::from(4);
        }
    };

    if !cargo_profiled(&[
        "build",
        "-p",
        "kernel",
        "--features",
        "soak_test",
        "--target",
        target,
    ]) {
        return ExitCode::from(4);
    }

    let log_path = log.unwrap_or_else(|| {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        PathBuf::from(format!("target/soak-test-{arch}-{stamp}.log"))
    });
    if let Some(parent) = log_path.parent()
        && let Err(e) = std::fs::create_dir_all(parent)
    {
        eprintln!("soak-test: cannot create {}: {e}", parent.display());
        return ExitCode::from(4);
    }
    let file = match std::fs::File::create(&log_path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("soak-test: cannot write {}: {e}", log_path.display());
            return ExitCode::from(4);
        }
    };
    let mut sink = Tee {
        file,
        terminal: std::io::stdout(),
    };

    let mut cmd = Command::new(runner);
    cmd.arg(format!(
        "{}/target/{target}/{}/kernel",
        workspace_root().display(),
        profile_dir()
    ));
    cmd.env("NIFE_INITRD", &initrd);
    if let Some(n) = &smp {
        cmd.env("NIFE_SMP", n);
    }
    cmd.stdout(std::process::Stdio::piped());
    // The runner's own diagnostics stay on this terminal rather than joining the captured stream:
    // the log is meant to be the guest's console and nothing else, so that a replay through
    // `script/board-console --replay` sees what a serial cable would have seen.
    cmd.stderr(std::process::Stdio::inherit());

    eprintln!(
        "--- soak-test: {arch}, up to {:?}, logging to {} ---",
        policy.total,
        log_path.display()
    );
    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("soak-test: cannot start {runner}: {e}");
            return ExitCode::from(4);
        }
    };
    let runner_pid = child.id();
    let Some(stdout) = child.stdout.take() else {
        eprintln!("soak-test: the runner gave us no stdout to read");
        let _ = child.kill();
        return ExitCode::from(4);
    };

    // `false`, not `true`: a pipe from a process really does end when that process dies, which a
    // serial port never does. Getting this bit wrong turns a QEMU that died into a board that has
    // not spoken yet.
    let session = watch(stdout, &mut sink, &policy, false);

    // Kill it whatever happened. A nife kernel that has finished its work sits in `wfi` forever and
    // QEMU with it, and this one never even finishes: leaving it running is the leak `AGENTS.md`
    // spends a whole section on.
    //
    // **The children first, then the wrapper**, the same order and for the same reason as
    // `run_bench` above: `scripts/qemu-runner-x86_64.sh` runs QEMU as a plain foreground child
    // rather than `exec`-ing into it, so killing the wrapper alone orphans the emulator. Found on
    // 2026-09-01 by an x86 soak that returned 0 and left a `qemu-system-x86_64` behind holding this
    // pipe. The other two runners `exec`, so `pkill -P` finds nothing there and costs one process.
    let _ = Command::new("pkill")
        .args(["-9", "-P", &runner_pid.to_string()])
        .status();
    let _ = child.kill();
    let _ = child.wait();
    let _ = sink.flush();

    let session = match session {
        Ok(s) => s,
        Err(e) => {
            eprintln!("soak-test: {e}");
            eprintln!("soak-test: log at {}", log_path.display());
            return ExitCode::from(4);
        }
    };

    eprintln!();
    eprintln!("soak-test: {}", session.summary());
    match session.progress.soak() {
        Some(beat) => {
            eprintln!(
                "soak-test: {} round trips in {}s ({} /s at the last beat), {} cross-core handoffs",
                beat.rounds, beat.seconds, beat.rate, beat.crossings
            );
            eprintln!(
                "soak-test: refused={} mismatch={} stalled={} (each must be 0)",
                beat.refused, beat.mismatches, beat.stalled
            );
            // Said on every clean run, on purpose, because this is the sentence the milestone's own
            // BUGS section says will otherwise be dropped when the number is quoted.
            eprintln!(
                "soak-test: a clean run is a number to compare against, NOT evidence that the \
                 concurrency is correct."
            );
            // **What the crossings are, said on every run that has any** (milestone 221). The
            // kernel prints this at the start of a soak, but a reader quoting this summary never
            // saw that, and the wrong reading is available and flattering: that the IPC workload
            // itself is migrating. It is not, and only a periodic rebalancer would make it, which
            // DECISIONS 138 (how a saturated workload is made to hand threads across cores)
            // declines.
            if beat.wakes > 0 {
                eprintln!(
                    "soak-test: the {} crossings are tick waiters being placed by the wake protocol \
                     ({} tick wakes drove them), NOT the IPC pairs migrating. See notes/soak.md.",
                    beat.crossings, beat.wakes
                );
            }
            // The gap milestone 219 measured rather than assumed, said on every run that shows it
            // because a reader who does not know it will read the round-trip total as covering
            // more than it does. See notes/soak.md.
            if beat.crossings < beat.rounds / 1000 {
                if beat.wakes == 0 {
                    eprintln!(
                        "soak-test: and it barely crossed cores ({} handoffs against {} round trips): \
                         this scheduler does not rebalance, so a saturated workload stays where it \
                         was placed. See notes/soak.md.",
                        beat.crossings, beat.rounds
                    );
                } else {
                    // The tick route was live and the machine still did not cross, which on one
                    // core is the only possible answer and on several is a finding. Both are named
                    // rather than one being assumed, because this summary cannot see the core
                    // count and the kernel's own banner can.
                    eprintln!(
                        "soak-test: and it barely crossed cores ({} handoffs against {} tick wakes), \
                         which on a single-core run is arithmetic and on a multicore one is a \
                         finding: check the core count in the soak's own start line.",
                        beat.crossings, beat.wakes
                    );
                }
            }
        }
        None => eprintln!("soak-test: no heartbeat was seen; the workload never started"),
    }
    eprintln!("soak-test: log at {}", log_path.display());

    // The one judgement that is this driver's rather than the watcher's, because it is about a
    // process and not about a board. A serial port cannot end; a pipe can, and QEMU exiting before
    // the deadline means the guest is gone. `board_console` scores `Ended` as success when nothing
    // was being waited for, which is right for a replayed capture and wrong here.
    if session.outcome == Outcome::Ended && session.elapsed + Duration::from_secs(1) < policy.total
    {
        eprintln!(
            "soak-test: QEMU exited after {:?}, before the deadline",
            session.elapsed
        );
        return ExitCode::from(3);
    }
    if session.progress.soak().is_none() {
        return ExitCode::from(3);
    }
    ExitCode::from(u8::try_from(session.exit_code()).unwrap_or(4))
}
