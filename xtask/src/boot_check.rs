//! The boot ladder's gate (milestone 268, item 6).

use std::process::Command;

use crate::host::{flag_value, run};
use crate::{RISCV_TARGET, RUNNER, TARGET, X86_TARGET, profile_dir};

/// **Boot every architecture's default kernel and fail if its self-test verdict is not green.**
///
/// This is the mechanism milestone 268's block calls "the whole mechanism and the piece most
/// likely to be dropped as follow-up". Without it the milestone ships its own finding 4 back: a
/// boot check that reports by printing a word and carrying on, with nothing gating.
///
/// **What it actually asserts**, which is more than it looks like:
///
/// - The kernel boots at all, on all three architectures, in the configuration a person gets by
///   typing `cargo xtask run`. Nothing else in CI boots the default riscv64 or `x86_64` kernel; the
///   suite boots a `#[cfg(test)]` build that exits through semihosting before the tour, and
///   `shell-check` boots `--features shell` on two architectures.
/// - The machine description printed to its end (`Stage::Machine` is its *last* line).
/// - Every one of the boot self-tests passed, by reading the verdict the kernel printed rather
///   than by inferring anything from an exit status the kernel never produces: a default boot
///   parks in `wfi` or sits at a prompt and never exits.
///
/// **`--inject` asserts the opposite**, and it is why this gate can be trusted. It rebuilds each
/// kernel with `--features self_test_injection`, which makes one check report failure, and
/// requires the run to come back **red**. A gate that has only ever been seen green is a gate
/// nobody has tested, and this is the cheapest honest way to test one: milestone 268's proof
/// condition is exactly that the same failure injected into any architecture turns the verdict red
/// there and fails this.
///
/// **No initrd and no disk**, deliberately. Every rung this gate reads is printed before userspace
/// exists, so attaching either would make the gate slower and would couple it to whatever the
/// archive happens to contain that week. The rung *above* the verdict (the `swish` prompt) does
/// need an archive, and `cargo xtask shell-check` is the gate that boots it.
///
/// # BUGS
///
/// - **It does not check the prompt**, which is the ladder's top rung and milestone 268's stated
///   terminal state for a default boot. Two reasons, and only the second is a real limitation:
///   `shell-check` already boots to a prompt on aarch64 and riscv64, and `x86_64` cannot reach one
///   at all until DECISIONS §149 and milestone 182. So nothing here can assert the top rung on all
///   three, and asserting it on two would be the shape of defect this milestone exists to fix.
/// - **A red verdict ends the watch early**, so a kernel whose self-test fails and which then
///   panics reports the self-test failure and not the panic. That is the right first thing to
///   report and the log has the rest, but a reader should know the report is the *first* failure
///   rather than the worst.
/// - **Name provisional** (milestone 268). `boot-check` sits beside `shell-check` and is named the
///   same way, which is a virtue and also inherits that name's recorded problem: `script/lint`
///   runs `shellcheck`, and this family of `-check` entry points is one hyphen away from several
///   unrelated things. calef has not ruled.
pub(crate) fn boot_check() -> bool {
    let legs = match flag_value("--arch").as_deref() {
        None => [true, true, true],
        Some("aarch64") => [true, false, false],
        Some("riscv64") => [false, true, false],
        Some("x86_64") => [false, false, true],
        Some(other) => {
            eprintln!(
                "boot-check: --arch {other} is not an architecture (aarch64, riscv64 or x86_64)"
            );
            return false;
        }
    };
    let inject = std::env::args().any(|a| a == "--inject");

    // TCG only, for `shell-check`'s reason: this boot never exits, so it is killed rather than
    // waited on, and there is nothing acceleration would buy a gate that spends its time waiting
    // for a line on a serial port.
    // SAFETY: xtask is single-threaded here. This runs on the main thread before any child that
    // reads the environment is spawned, and the only thread started below is `watch`'s reader,
    // which copies pipe bytes and never touches the environment.
    unsafe { std::env::remove_var("NIFE_ACCEL") };

    let mut ok = true;
    for (i, (arch, target, runner)) in [
        ("aarch64", TARGET, RUNNER),
        ("riscv64", RISCV_TARGET, "scripts/qemu-runner-riscv64.sh"),
        ("x86_64", X86_TARGET, "scripts/qemu-runner-x86_64.sh"),
    ]
    .into_iter()
    .enumerate()
    {
        if legs[i] && !boot_check_leg(arch, target, runner, inject) {
            ok = false;
        }
    }
    if ok {
        eprintln!();
        eprintln!(
            "boot-check: every architecture reached the self-test verdict and it was {}",
            if inject {
                "RED, as the injection asked"
            } else {
                "green"
            },
        );
    }
    ok
}

/// One architecture's leg of [`boot_check`]. Builds, boots, watches for the verdict, kills QEMU,
/// and says what it found either way.
fn boot_check_leg(arch: &str, target: &str, runner: &str, inject: bool) -> bool {
    use board_console::progress::{Failure, Stage};
    use board_console::watch::{Outcome, Policy};

    eprintln!();
    eprintln!(
        "--- boot-check ({arch}): boot the default kernel and read its self-test verdict{} ---",
        if inject {
            ", with a failure injected"
        } else {
            ""
        },
    );

    let mut build = vec!["build", "-p", "kernel", "--target", target];
    if inject {
        build.extend_from_slice(&["--features", "self_test_injection"]);
    }
    if !run("cargo", &build) {
        return false;
    }

    // The runner directly rather than through `cargo run`, so the process this owns is the
    // emulator (or, on x86, its wrapper; see the kill below). A `cargo run` in between would leave
    // the emulator alive when the kill lands on cargo, which is the leak AGENTS.md's QEMU rule
    // exists about.
    let mut cmd = Command::new(runner);
    cmd.arg(format!("target/{target}/{}/kernel", profile_dir()));
    cmd.stdout(std::process::Stdio::piped());
    // The guest's own stderr is not interesting and QEMU's is, so it is left attached: a runner
    // that cannot find its emulator should say so on this terminal rather than into a pipe nobody
    // reads.
    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("boot-check ({arch}): failed to start {runner}: {e}");
            return false;
        }
    };
    let runner_pid = child.id();
    let stdout = child.stdout.take().expect("piped stdout");

    // The same recogniser `cargo xtask board-console` points at a real board, which is the point:
    // a gate that used a second reader would be gating something the bench does not measure.
    let policy = Policy {
        // Generous: a debug kernel under TCG on a loaded CI runner is slow, and the cost of a
        // too-short cap is a red build that means nothing.
        total: std::time::Duration::from_secs(180),
        until: Some(Stage::SelfTest),
        // The verdict is printed a few milliseconds after the machine description, so a board that
        // has gone quiet for half a minute in between is wedged rather than slow.
        quiet_after: Some(std::time::Duration::from_secs(30)),
        // Keep reading after the rung arrives, so a verdict followed immediately by a panic is
        // reported as the panic rather than as a success. `watch`'s own doc records the capture
        // this defends against.
        settle: std::time::Duration::from_secs(2),
        // **No prologue, and that is the honest profile for an emulator** (milestone 324 part 3).
        // There is no firmware on the `virt` or `q35` machines to print `U-Boot SPL`, so xenon's
        // empty prologue describes what this gate watches better than radon's four rungs do. It
        // changes no behaviour, since an absent marker is never matched either way; it changes
        // what a report says the tool was expecting.
        board: &board_console::board::XENON,
    };

    let log_path = format!(
        "target/boot-check-{arch}{}.log",
        if inject { "-injected" } else { "" }
    );
    let mut sink: Box<dyn std::io::Write> = match std::fs::File::create(&log_path) {
        Ok(f) => Box::new(std::io::BufWriter::new(f)),
        Err(e) => {
            eprintln!("boot-check ({arch}): cannot write {log_path}: {e}");
            let _ = child.kill();
            let _ = child.wait();
            return false;
        }
    };
    // `stream_never_ends: false`: this is a pipe, so a zero-byte read means the emulator is gone.
    let session = board_console::watch::watch(stdout, &mut *sink, &policy, false);
    let _ = sink.flush();

    // Kill any QEMU the runner spawned before killing the runner itself. On `q35` the runner is a
    // *wrapper* that runs the emulator as a plain child rather than `exec`-ing into it, so
    // `child.kill()` alone orphans the emulator rather than ending it. The other two runners
    // `exec`, so this finds nothing there. Best effort, silent either way. Same reasoning as
    // `run_bench`; see AGENTS.md, "Never leave QEMU running".
    let _ = Command::new("pkill")
        .args(["-9", "-P", &runner_pid.to_string()])
        .status();
    let _ = child.kill();
    let _ = child.wait();

    let session = match session {
        Ok(s) => s,
        Err(e) => {
            eprintln!("boot-check ({arch}): reading the console failed: {e}");
            eprintln!("boot-check ({arch}): log at {log_path}");
            return false;
        }
    };

    eprintln!("boot-check ({arch}): {}", session.summary());
    if let Some(line) = session.progress.machine_line() {
        eprintln!("boot-check ({arch}): {line}");
    }
    if let Some(line) = session.progress.self_test_line() {
        eprintln!("boot-check ({arch}): {line}");
    }

    let red = matches!(
        session.outcome,
        Outcome::Announced(Failure::SelfTestFailed(_))
    );
    let green = matches!(session.outcome, Outcome::Reached(_))
        && session.progress.reached() >= Stage::SelfTest
        && session.progress.failure().is_none();

    let pass = if inject { red } else { green };
    if !pass {
        eprintln!();
        if inject {
            eprintln!(
                "boot-check ({arch}): FAILED. A kernel built with `--features self_test_injection` \
                 must print a RED verdict and this run did not, so the gate cannot see a failing \
                 self-test and is not gating anything. This is the defect milestone 268's block \
                 predicted: a verdict nobody reads."
            );
        } else {
            eprintln!(
                "boot-check ({arch}): FAILED. The default boot did not reach a green self-test \
                 verdict. The log is the diagnosis; `nife machine:` says how far discovery got and \
                 `nife self-test:` names whatever did not pass."
            );
        }
        eprintln!("boot-check ({arch}): log at {log_path}");
    }
    pass
}
