//! The boot ladder's gate (milestone 268, item 6).

use std::process::Command;

use crate::archive::{initrd_path, initrd_riscv, initrd_x86, riscv_initrd_path, x86_initrd_path};
use crate::host::{flag_value, run};
use crate::{RISCV_TARGET, RUNNER, TARGET, X86_TARGET, profile_dir, user};

/// **Boot every architecture's default kernel and fail if it does not climb the whole ladder.**
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
/// - **The `swish` prompt**, the ladder's top rung and the stated terminal state of a default
///   boot, on all three architectures. See "The top rung" below.
///
/// **`--inject` asserts the opposite**, and it is why this gate can be trusted. It rebuilds each
/// kernel with `--features self_test_injection`, which makes one check report failure, and
/// requires the run to come back **red**. A gate that has only ever been seen green is a gate
/// nobody has tested, and this is the cheapest honest way to test one: milestone 268's proof
/// condition is exactly that the same failure injected into any architecture turns the verdict red
/// there and fails this.
///
/// # The top rung
///
/// **The archive is attached on all three architectures and the watch runs to `Stage::Prompt`**
/// (2026-09-19). It did not before, and the reason was the machine rather than the gate: `x86_64`
/// could not reach a prompt at all. DECISIONS §149 (may the kernel answer on an endpoint) was
/// resolved 2026-09-15, and milestone 299 (the x86 port-range capability) built the thing that
/// makes `console` a userspace driver there. Between them the rung is reachable everywhere, and
/// asserting it is no longer the two-of-three shape milestone 268 (every architecture boots the
/// same way) exists to remove.
///
/// **No disk, still.** Nothing here types, so nothing here needs a filesystem; `<` and `>` are
/// `cargo xtask shell-check`'s business. The archive is the one thing a prompt cannot be reached
/// without: the kernel hands the machine to the progenitor, which loads the console, the line
/// discipline, the input driver and `swish` out of it by name.
///
/// **What this costs, measured on patagonia against a warm target directory** (2026-09-19):
///
/// | | verdict only (before) | prompt (now) |
/// |---|---|---|
/// | aarch64, boot | 2.8 s | 2.9 s |
/// | riscv64, boot | 2.6 s | 2.8 s |
/// | `x86_64`, boot | 2.9 s | 2.7 s |
/// | whole gate, wall | 19.4 s | 25.6 s |
///
/// **The boot costs nothing and the archive costs six seconds**, which is the number that decided
/// this. The prompt follows the verdict within a few hundred milliseconds on every architecture
/// (the progenitor loads and measures every program it builds while the emulator is already
/// running), so the whole of the difference is packing three archives, and two of those three
/// packers are a no-op against a tree `script/test` has already built. A third of a pre-push
/// gate's twenty seconds, for the rung that is the stated terminal state of a boot, was worth
/// taking.
///
/// **It is the PVH `-kernel` boot on `x86_64`, not the UEFI image.** `shell-check`'s `x86_64` leg
/// boots `BOOTX64.EFI` under OVMF and pays about six minutes for it, because under firmware the
/// console server waits for every byte to be painted on the screen. That is milestone 400 (the
/// shell on the firmware's screen), and what its fidelity buys `shell-check` is the loader, the
/// firmware memory map and the screen tee. It buys this gate nothing it asserts: the prompt
/// arrives on COM1 either way, and this is a pre-push gate. So the two legs boot different images
/// on purpose, and the slow one is the one that types.
///
/// # BUGS
///
/// - **`quiet_after` is suppressed from `Stage::Tour` up**, which on riscv64 is *below* the prompt:
///   its tour finishes and then it hands over. So a riscv64 boot that reaches the tour and then
///   wedges short of the prompt is caught by the 180-second cap rather than by the 30-second quiet
///   window, and takes that long to report. The exemption is `watch::Policy::quiet_after`'s and is
///   right for its own reason (a default aarch64 or `x86_64` boot that halts after the tour is a
///   good boot, not a quiet one); the cost lands here.
/// - **The prompt rung is the shell's banner, not the `$ `.** `boot_ladder::PROMPT`'s own `BUGS`
///   says why (two bytes is too weak to key on in a log that has just carried a kilobyte of hex),
///   and the consequence is this gate's: it proves `swish` started and printed, not that a prompt
///   was offered or that anything could be typed at it. `cargo xtask shell-check` makes the
///   stronger claim by typing, on every architecture, and this one does not duplicate it.
/// - **One emulator was found orphaned after about fifteen `x86_64` boots** (2026-09-19),
///   reparented to `launchd` and still running half an hour later, holding this lane's kernel and
///   archive. The kill below is meant to prevent exactly that and works on every run anybody has
///   watched, so the mechanism is not known and is not guessed at here. What a reader should do is
///   `AGENTS.md`'s standing rule rather than anything specific to this gate: `pgrep -l qemu` after
///   a session that ran it, and walk `ps -o pid,ppid` up before killing anything, because
///   somebody's gate in flight looks the same from outside.
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
    // The archive packer per architecture, and where it writes. Both halves are needed: the
    // packer has to run **before** the kernel build, because it writes the measurement manifest
    // `kernel/build.rs` reads, at phase B.1 of milestone 22 (trusted init), and a kernel built
    // against a stale one
    // refuses to load the progenitor and never reaches a prompt.
    for (i, (arch, target, runner, pack, archive)) in [
        (
            "aarch64",
            TARGET,
            RUNNER,
            user as fn() -> bool,
            initrd_path as fn() -> String,
        ),
        (
            "riscv64",
            RISCV_TARGET,
            "helpers/qemu-runner-riscv64.sh",
            initrd_riscv as fn() -> bool,
            riscv_initrd_path as fn() -> String,
        ),
        (
            "x86_64",
            X86_TARGET,
            "helpers/qemu-runner-x86_64.sh",
            initrd_x86 as fn() -> bool,
            x86_initrd_path as fn() -> String,
        ),
    ]
    .into_iter()
    .enumerate()
    {
        if legs[i] && !boot_check_leg(arch, target, runner, inject, pack, archive) {
            ok = false;
        }
    }
    if ok {
        eprintln!();
        // Two sentences rather than one with a hole in it: an injected run **stops at the red
        // verdict** and never reaches the prompt, so a single wording would claim a rung that leg
        // deliberately did not climb.
        eprintln!(
            "{}",
            if inject {
                "boot-check: every architecture's self-test verdict came back RED, as the \
                 injection asked"
            } else {
                "boot-check: every architecture climbed the ladder to a shell prompt, and the \
                 self-test verdict on the way was green"
            },
        );
    }
    ok
}

/// One architecture's leg of [`boot_check`]. Packs the archive, builds, boots, watches for the
/// prompt, kills QEMU, and says what it found either way.
fn boot_check_leg(
    arch: &str,
    target: &str,
    runner: &str,
    inject: bool,
    pack: fn() -> bool,
    archive: fn() -> String,
) -> bool {
    use board_console::progress::{Failure, Stage};
    use board_console::watch::{Outcome, Policy};

    eprintln!();
    eprintln!(
        "--- boot-check ({arch}): boot the default kernel and climb the ladder to the prompt{} ---",
        if inject {
            ", with a failure injected"
        } else {
            ""
        },
    );

    // Before the kernel, always: see the table in [`boot_check`].
    if !pack() {
        eprintln!("boot-check ({arch}): could not pack the userspace archive");
        return false;
    }

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
    // **The archive, which is the one thing a prompt cannot be reached without**, and nothing else:
    // no disk, because this gate types nothing and so needs no filesystem.
    //
    // The two removals are not tidiness. `host::cargo` sets `NIFE_INITRD`, `NIFE_DISK` and
    // `NIFE_NET` **in this process's own environment** for every cargo invocation it makes, and
    // the archive packers above go through it, so by the time the runner is spawned it would
    // inherit a `NIFE_DISK` naming a file `mkdisk` may never have written. The aarch64 and
    // `x86_64` runners treat a set-but-missing `NIFE_DISK` as fatal (`does not exist (run mkdisk
    // first)`) and the riscv64 one ignores it, so this gate went red on two of three legs the
    // first time it packed an archive. Setting the environment a child is spawned with, here,
    // beats inheriting whatever a sibling left behind; it also means an exported `NIFE_DISK` in a
    // developer's shell cannot change what this gate boots.
    cmd.env("NIFE_INITRD", archive());
    cmd.env_remove("NIFE_DISK");
    cmd.env_remove("NIFE_NET");
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
        // **The top rung, on every architecture** (2026-09-19). `Stage::SelfTest` until then,
        // because `x86_64` could reach no prompt; see this module's "The top rung".
        until: Some(Stage::Prompt),
        // Each rung follows the one below it within milliseconds up to the verdict, and the
        // hand-over then loads and measures every program the progenitor builds, which is the one
        // gap in the ladder with real work in it. Half a minute of silence is a wedge rather than
        // slowness at any point in that.
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
    // `>= Stage::Prompt` rather than `== `: the rungs are ordered, so a boot that went further
    // still climbed past this one, and an architecture that grows a rung above the prompt should
    // not turn this gate red on the day it lands.
    let green = matches!(session.outcome, Outcome::Reached(_))
        && session.progress.reached() >= Stage::Prompt
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
                "boot-check ({arch}): FAILED. The default boot did not climb the ladder to a \
                 `swish` prompt with a green self-test verdict on the way. The log is the \
                 diagnosis, and the rung named above says where it stopped: `nife machine:` says \
                 how far discovery got, `nife self-test:` names whatever did not pass, and a boot \
                 that reached the verdict and no further did not get userspace up."
            );
        }
        eprintln!("boot-check ({arch}): log at {log_path}");
    }
    pass
}
