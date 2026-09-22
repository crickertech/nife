//! The `cargo xtask bench` command: the cross-OS measurements, and the check that compares a
//! run against the recorded baselines.

use std::process::Command;
use std::sync::atomic::Ordering;

use crate::archive::{initrd_path, initrd_riscv, riscv_initrd_path};
use crate::disk::{disk_path, mkdisk, mkredoxfs, redoxfs_server_build};
use crate::host::{flag_value, kernel_elf, run, workspace_root};
use crate::{RELEASE, RISCV_TARGET, RUNNER, TARGET, X86_TARGET, cargo_profiled, user};

/// The microbenchmarks (milestone 21; design/roadmap/21-benchmarks.md).
///
/// Two instruments:
/// - default: TCG with `-icount`, where virtual time is a deterministic function of instructions
///   executed. Counts are exact and reproducible; `--check` diffs them against
///   `bench/baseline-aarch64.txt` and fails on drift, `--save` rewrites the baseline (a deliberate act,
///   committed alongside whatever changed the numbers).
/// - `--real`: HVF, natively on the host core. Real caches and TLBs, statistical numbers,
///   reported in nanoseconds, never gating.
///
/// The bench kernel never exits on its own (semihosting does not work under HVF; see `test`).
/// We own the QEMU child, watch its output for `bench: done`, and kill it: one exit mechanism
/// for both accelerators.
pub(crate) fn bench() -> bool {
    let check = std::env::args().any(|a| a == "--check");
    let save = std::env::args().any(|a| a == "--save");
    // `--release` builds an optimized kernel and userspace, for a fair cross-OS comparison (the debug
    // default is fine for the icount gate, whose counts are path length, but not for magnitudes next
    // to release Linux). Release changes instruction counts, so it never runs under icount and never
    // gates: it implies `--real` (HVF magnitudes only).
    let release = std::env::args().any(|a| a == "--release");
    RELEASE.store(release, Ordering::Relaxed);
    let real = release || std::env::args().any(|a| a == "--real");
    if real && (check || save) {
        let why = if release { "--release" } else { "--real" };
        eprintln!("bench: {why} numbers are statistical and never gate; no --check/--save");
        return false;
    }

    // E3 (milestone 134, design/roadmap/134-the-measurements-that-decide.md): build with an extra
    // kernel feature alongside `bench`, so the padded-fastpath experiment can be measured with the
    // same harness as everything else rather than a one-off. `--real`-only for the same reason
    // `--release` is: there is no cache under TCG for a footprint change to perturb, so a gated
    // run with this set would just measure instruction-count noise and call it a finding.
    let extra_features = flag_value("--extra-features");
    if extra_features.is_some() && !real {
        eprintln!(
            "bench: --extra-features only makes sense with --real (no cache under TCG for it to move)"
        );
        return false;
    }
    let features = match &extra_features {
        Some(f) => format!("bench,{f}"),
        None => "bench".to_string(),
    };

    // The second architecture. RISC-V has its own path (its own kernel target, runner, and initrd,
    // no disk, no HVF); everything else -- the icount instrument, the parsing, the table, the
    // baseline gate -- is shared through run_bench. See bench_riscv.
    if std::env::args().any(|a| a == "--riscv") {
        return bench_riscv(check, save, &features);
    }

    // The third architecture (milestone 161; DECISIONS §121's amendment, the TSS I/O-bitmap
    // switch cost, extended by milestone 161's icount leg, 2026-08-25). **Not** the same question
    // as `icount()` below, which still refuses `--arch x86_64` for a real reason (milestone 78's
    // instrument needs a re-armed deadline timer to compare against, and this port's LAPIC timer
    // is periodic hardware reload with no such deadline to read). Pinning QEMU's virtual clock to
    // the instruction stream is a strictly weaker ask than that, and it works: measured, not
    // assumed, three consecutive boots under `-icount shift=0,sleep=off` on `q35` produced
    // byte-identical tick counts on every bench line, including the PIT-calibrated TSC frequency
    // itself. So `--x86` defaults to that instrument now, exactly like the other two ISAs, and
    // gates the same way; `--real` keeps the plain-TCG statistical path notes/benchmarks.md's
    // 2026-08-24 section already used, and the `real`+`check`/`save` refusal above already
    // covers `--x86 --real --check`.
    if std::env::args().any(|a| a == "--x86") {
        return bench_x86(real, check, save, &features);
    }

    // `--smp`: boot the full 4-hart machine under HVF so the multi-hart throughput bench
    // (`smp_throughput`, DECISIONS §28) and the FS service-path bench (`fs_read`, DECISIONS §32) have
    // cores and, for the FS one, a filesystem to work with. Both self-skip on one hart, so without
    // this flag the `--real` run is single-hart and neither builds the FS image nor prints their
    // lines. Only meaningful with `--real`.
    let smp = std::env::args().any(|a| a == "--smp");

    // For --smp, build the FS server (before user(), so initrd_aarch64 packs the redoxfs_server ELF) and the
    // RedoxFS test image the runner attaches as the second mmio disk. The fs_read bench opens it; on
    // any run without the image the bench finds no second disk and skips, so this stays out of the
    // icount gate's build entirely.
    if (smp && !redoxfs_server_build(TARGET))
        || !mkdisk()
        || !user()
        || (smp && !mkredoxfs())
        || !cargo_profiled(&[
            "build",
            "-p",
            "kernel",
            "--features",
            &features,
            "--target",
            TARGET,
        ])
    {
        return false;
    }

    // Run the kernel through the same runner script as everything else, with the accelerator
    // chosen by env and, for the deterministic instrument, icount pinning virtual time to the
    // instruction stream (sleep=off: virtual time never waits for the wall clock).
    let mut cmd = Command::new(RUNNER);
    cmd.arg(kernel_elf());
    if real {
        cmd.env("NIFE_ACCEL", "hvf");
        if smp {
            // The full machine, for the aggregate-throughput bench. The per-core primitive magnitudes
            // in this same run are then NOT per-core clean (the reap-heavy ones, spawn_el0 and
            // spawn_reap, inflate and go noisy under cross-core reap lag); read those from the default
            // single-hart run instead. See notes/benchmarks.md, the multi-hart section.
            // "4" matches the runner's default; the throughput bench reads
            // the actual online count at runtime, so this only needs to be more than one.
            cmd.env("NIFE_SMP", "4");
            eprintln!(
                "--- bench: HVF, 4 harts (for smp_throughput; primitives are not per-core here) ---"
            );
        } else {
            // One hart by default, the same choice the icount instrument makes and for a kindred
            // reason: a primitive magnitude is a PER-CORE number, and the cross-OS comparison
            // (notes/benchmarks.md) reads it as one. At `-smp 4` the reap-heavy primitives pick up
            // cross-core reap lag that has nothing to do with per-core cost (spawn_el0 ~4.8 us here
            // goes ~13.6 us and swings wildly there; spawn_reap likewise). So the default `--real`
            // run is single-hart and clean; `--real --smp` boots the whole machine for the throughput
            // bench, which needs more than one core to mean anything.
            cmd.env("NIFE_SMP", "1");
            eprintln!(
                "--- bench: HVF, single hart, per-core magnitudes (statistical; medians matter) ---"
            );
        }
    } else {
        cmd.env_remove("NIFE_ACCEL");
        cmd.args(["-icount", "shift=0,sleep=off"]);
        // One hart, the same reason the riscv path forces it (bench_riscv): a primitive benchmark
        // measures per-core path length, and the counter it reads (CNTVCT) advances with QEMU's
        // GLOBAL virtual time. Under `-icount` all vCPUs share that one clock, and an idle secondary
        // hart sitting in `wfi` jumps virtual time to the next timer tick, so with `-smp 4` the
        // measured window counts three other harts' idle jumps and load-balanced spawns, not the
        // path under test. That contamination (not any code change) is what made the counts swing
        // wildly and non-physically across today's merges: coremark, pure compute, moved 63%. See
        // notes/benchmarks.md, the 2026-07-28 attribution. The aarch64 default is 4 (SMP tests);
        // the icount bench pins 1 to match riscv and measure the primitive, not the machine.
        cmd.env("NIFE_SMP", "1");
        eprintln!(
            "--- bench: aarch64, single hart, TCG + icount (deterministic instruction counts) ---"
        );
    }
    cmd.env("NIFE_INITRD", initrd_path());
    cmd.env("NIFE_DISK", disk_path());

    run_bench(
        cmd,
        real,
        check,
        save,
        workspace_root().join("bench/baseline-aarch64.txt"),
        "qemu-system-aarch64",
    )
}

/// **The RISC-V benchmark path** (parity E's follow-up). Same primitive suite, same deterministic
/// icount instrument, on the second architecture, so the tick counts are directly comparable to the
/// aarch64 ones: both are the virtual timer advancing under `-icount`, which is instruction-clocked,
/// not wall-clock. No HVF (there is no RISC-V hypervisor on this host) and no disk (the bench boot
/// runs no virtio); it just needs the riscv initrd carrying `os_primitives_benchmarker` + `coremark`. Its baseline is a
/// separate file, since the counts differ by ISA. `cargo xtask bench --riscv [--check|--save]`.
fn bench_riscv(check: bool, save: bool, features: &str) -> bool {
    if !initrd_riscv()
        || !run(
            "cargo",
            &[
                "build",
                "-p",
                "kernel",
                // `features` and not "bench": it carries `--extra-features` too (E3's
                // `fastpath_pad`, milestone 134). This arm hardcoded "bench" until 2026-09-19, so
                // `--riscv --real --extra-features fastpath_pad` built an UN-padded kernel and
                // printed numbers for it, which is the silently-wrong shape this tree fears most.
                // riscv64 is the ISA radon runs, so it is the arm the flag mattered on.
                "--features",
                features,
                "--target",
                RISCV_TARGET,
            ],
        )
    {
        return false;
    }

    let mut cmd = Command::new("scripts/qemu-runner-riscv64.sh");
    cmd.arg(format!("target/{RISCV_TARGET}/debug/kernel"));
    // icount pins virtual time (rdtime) to the instruction stream; sleep=off so it never waits on the
    // wall clock. This is what makes the riscv counts deterministic and comparable to aarch64's.
    cmd.args(["-icount", "shift=0,sleep=off"]);
    cmd.env("NIFE_INITRD", riscv_initrd_path());
    // One hart: a primitive benchmark measures per-core cost. With more harts, a thread that waits
    // for a spawned child leaves its hart idling in `wfi`, and under `-icount` a `wfi` jumps virtual
    // time to the next timer tick, inflating the spawn primitives to timer-quantized nonsense. The
    // single-core costs are what compare to aarch64 anyway.
    cmd.env("NIFE_SMP", "1");
    eprintln!(
        "--- bench: riscv64, single hart, TCG + icount (deterministic instruction counts) ---"
    );

    run_bench(
        cmd,
        false,
        check,
        save,
        workspace_root().join("bench/baseline-riscv64.txt"),
        "qemu-system-riscv64",
    )
}

/// **The `x86_64` benchmark path** (DECISIONS §121's amendment, milestone 161 item 4; the icount
/// leg, milestone 161, 2026-08-25). Same suite, minus everything that needs a real userspace ELF:
/// `crates/user_mode_runtime` has no `x86_64` arms yet, so every `_el0` bench self-skips (`crate::
/// user::program` finds nothing in the initrd this leg never builds), and it adds one x86-only
/// bench, `tss_iomap_switch`: `bench::yield_switch` with a full I/O-permission-bitmap-sized write
/// added on every switch-in. Reading its `ns/iter` against `yield_switch`'s from the same boot is
/// §121's missing number, the dominant cost of option 1 (a port-range capability enforced by the
/// TSS bitmap) that the decision names as unmeasured. See
/// `kernel/src/arch/x86_64/segments.rs`'s `bench_write_io_bitmap`.
///
/// **Two instruments, the same split `bench()` makes for aarch64**: default is TCG + `-icount
/// shift=0,sleep=off`, gated against `bench/baseline-x86_64.txt`; `--real` is plain TCG (no
/// KVM/HVF on this ARM host to accelerate `x86_64`), statistical, never gating, the shape
/// notes/benchmarks.md's 2026-08-24 section already used for the `tss_iomap_switch` measurement
/// before this leg existed.
///
/// **This is not `icount()`'s instrument** (see the `--x86` branch in `bench()` above): that one
/// still refuses `--arch x86_64`, because milestone 78's claims compare an interrupt's arrival
/// against a deadline the kernel re-armed, and this port's LAPIC timer is a periodic hardware
/// reload with no such deadline to read. Pinning the virtual clock for a `timed()`-style duration
/// measurement needs none of that: `now()` already dispatches to `rdtsc`
/// (`kernel/src/arch/x86_64/timer.rs`), and rdtsc tracks icount's virtual clock the same way
/// `CNTVCT_EL0` and riscv64's `rdtime` do. **Measured, not assumed**: three consecutive `--x86`
/// boots under `-icount shift=0,sleep=off` produced byte-identical tick counts on every bench
/// line, including the PIT-calibrated TSC frequency itself (`bench: cntfrq 999935600` on all
/// three), so `run_bench`'s existing tick-count machinery needed no x86-specific change.
///
/// `scripts/qemu-runner-x86_64.sh` attaches no disk and builds no initrd, so this needs neither
/// `mkdisk` nor `user()`. `cargo xtask bench --x86 [--real] [--check|--save]`.
fn bench_x86(real: bool, check: bool, save: bool, features: &str) -> bool {
    if !run(
        "cargo",
        &[
            "build",
            "-p",
            "kernel",
            // As in `bench_riscv`: `features` carries `--extra-features`, which this arm also
            // dropped on the floor. `fastpath_pad` does not build on x86_64 today
            // (`script/fastpath-footprint`'s BUGS), so cargo refuses rather than mismeasures.
            "--features",
            features,
            "--target",
            X86_TARGET,
        ],
    ) {
        return false;
    }

    let mut cmd = Command::new("scripts/qemu-runner-x86_64.sh");
    cmd.arg(format!("target/{X86_TARGET}/debug/kernel"));
    if real {
        eprintln!(
            "--- bench: x86_64, single hart, plain TCG (no KVM/HVF on this host; statistical) ---"
        );
    } else {
        cmd.args(["-icount", "shift=0,sleep=off"]);
        eprintln!(
            "--- bench: x86_64, single hart, TCG + icount (deterministic instruction counts) ---"
        );
    }

    run_bench(
        cmd,
        real,
        check,
        save,
        workspace_root().join("bench/baseline-x86_64.txt"),
        "qemu-system-x86_64",
    )
}

/// The nightly `rust-toolchain.toml` pins, as the baseline header records it.
///
/// A three-line parse rather than a TOML dependency: §46 (thin primitives or whole subsystems) is the
/// rule, and one `channel = "..."` line does not justify one. `None` when the file has no channel
/// line at all, which the caller turns into `unrecorded` rather than a guess.
fn pinned_nightly() -> Option<String> {
    let text = std::fs::read_to_string(workspace_root().join("rust-toolchain.toml")).ok()?;
    text.lines()
        .find(|l| l.trim_start().starts_with("channel"))
        .and_then(|l| l.split('"').nth(1).map(str::to_owned))
}

/// The QEMU version `.qemu-version` pins, or `None` if the file is unreadable.
fn pinned_qemu() -> Option<String> {
    std::fs::read_to_string(workspace_root().join(".qemu-version"))
        .ok()
        .map(|s| s.trim().to_owned())
}

/// The version of the emulator that is actually going to run, asked of the binary itself.
///
/// **Not `.qemu-version`, and the asymmetry with `pinned_nightly` above is the whole point.**
/// `rustup` RESOLVES the compiler from `rust-toolchain.toml`, so the pin and the thing that ran are
/// one fact by construction. Nothing resolves QEMU from `.qemu-version`: it is a wish about the
/// machine, and on 2026-08-28 this machine stopped granting it (11.1.1 installed against a pinned
/// 11.0.2) with nothing to say so. An icount count is a function of the compiler that produced the
/// instructions AND the emulator that counted them, so recording the pin here would file intent
/// under the heading of provenance, which is the defect this whole mechanism exists to close.
fn running_qemu(binary: &str) -> Option<String> {
    let out = Command::new(binary).arg("--version").output().ok()?;
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .next()?
        .split_whitespace()
        .nth(3)
        .map(str::to_owned)
}

/// Run a bench kernel through `cmd`, read its `bench:` lines until `bench: done`, and report the
/// table (and, off the deterministic icount instrument, save or check against `baseline`). Shared by
/// the aarch64 and RISC-V bench paths so the parsing, the table, and the regression gate are one
/// implementation. `real` only chooses the "ns are fiction" footer.
fn run_bench(
    mut cmd: Command,
    real: bool,
    check: bool,
    save: bool,
    baseline_path: std::path::PathBuf,
    // The emulator that will count these instructions, named so the baseline can record which one
    // did. Per architecture, because the three legs run three different binaries.
    qemu_binary: &str,
) -> bool {
    cmd.stdout(std::process::Stdio::piped());

    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("bench: failed to start the runner: {e}");
            return false;
        }
    };
    let runner_pid = child.id();

    // Read lines until the guest says it is done, then kill it: it is parked in wfi and will
    // never exit by itself (deliberately; see kernel/src/bench.rs).
    use std::io::BufRead;
    let stdout = child.stdout.take().expect("piped stdout");
    let reader = std::io::BufReader::new(stdout);
    let mut results: Vec<(String, u64, u64)> = Vec::new();
    let mut cntfrq: u64 = 0;
    let mut done = false;
    for line in reader.lines() {
        let Ok(line) = line else { break };
        // **Diagnostics pass straight through, and deliberately do not become rows.** A probe
        // reports a count or a bitmask, not a duration, so putting it in the table would invite
        // `--check` to police an exact value with a 10% tolerance and would print a meaningless
        // ns/iter beside it. Echoed instead, so a reader of a bench run sees it and the baseline
        // never grows a line that is not a measurement. See `bench::map_new`'s shootdown probe.
        if let Some(probe) = line.strip_prefix("bench-probe: ") {
            eprintln!("  probe: {probe}");
            continue;
        }
        let Some(rest) = line.strip_prefix("bench: ") else {
            continue;
        };
        if rest == "done" {
            done = true;
            break;
        }
        let parts: Vec<&str> = rest.split_whitespace().collect();
        match parts.as_slice() {
            ["cntfrq", hz] => cntfrq = hz.parse().unwrap_or(0),
            [name, ticks, iters] => {
                if let (Ok(t), Ok(i)) = (ticks.parse(), iters.parse()) {
                    results.push((name.to_string(), t, i));
                }
            }
            _ => {}
        }
    }
    // Kill any QEMU the runner itself spawned before killing the runner: on `q35`
    // (`scripts/qemu-runner-x86_64.sh`) `cmd` is a *wrapper* that runs `qemu-system-x86_64` as a
    // plain foreground child rather than `exec`-ing into it (the runner's own header explains why:
    // it has to translate `isa-debug-exit`'s odd-only exit status). `child.kill()` therefore only
    // ever reaches the wrapper on that leg, and killing the wrapper first orphans the emulator
    // rather than ending it, which under `-icount sleep=off` is not an idle leak: a halted guest
    // whose virtual clock never waits on the host spins a full core forever instead of parking in
    // `hlt`. The other two runners `exec` (their own PID already *is* QEMU's), so `pkill -P` finds
    // nothing there and this is a no-op. Best-effort and silent either way: a runner that already
    // exited leaves no children to find. See AGENTS.md, "Never leave QEMU running".
    let _ = Command::new("pkill")
        .args(["-9", "-P", &runner_pid.to_string()])
        .status();
    let _ = child.kill();
    let _ = child.wait();

    if !done {
        eprintln!("bench: QEMU ended before printing `bench: done`; no results");
        return false;
    }

    // Report. icount counts are the regression currency; ns is computed for both instruments
    // (fictional under icount, real under HVF) because a human wants a magnitude to look at.
    eprintln!();
    eprintln!(
        "{:<14} {:>12} {:>8} {:>12} {:>10}",
        "benchmark", "ticks", "iters", "ticks/iter", "ns/iter"
    );
    for (name, ticks, iters) in &results {
        // `checked_div`, not `/`: a benchmark that reports zero iterations (a skip, or a future
        // diagnostic line) must not panic the whole harness after the run already happened.
        let per = ticks.checked_div(*iters).unwrap_or(0);
        let ns = (ticks * 1_000_000_000)
            .checked_div(cntfrq)
            .and_then(|v| v.checked_div(*iters))
            .unwrap_or(0);
        eprintln!("{name:<14} {ticks:>12} {iters:>8} {per:>12} {ns:>10}");
    }
    if !real {
        eprintln!("(TCG+icount: ticks are deterministic; ns are fiction. --real for magnitudes.)");
    }

    if save {
        // The header names the file it is in. It used to be the literal `bench/baseline.txt` for
        // both baselines, so the riscv one claimed to be the aarch64 one; deriving it from the path
        // makes the two agree with themselves (milestone 73).
        let stem = baseline_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("baseline.txt");
        // **The nightly these counts were produced by, written into the file that holds them.**
        // icount counts guest instructions, and a different compiler emits a different instruction
        // sequence for the same source, so the toolchain is part of what these numbers mean in
        // exactly the way `.qemu-version` already is. It went unrecorded until 2026-09-21, and the
        // cost was live: `nightly-2026-09-15` was pinned without re-recording, the tripwire's
        // headroom eroded for reasons no change was responsible for, and the tree was bumped again
        // before anyone acted. `script/lint`'s baseline-toolchain check compares this line against
        // `rust-toolchain.toml`, so the two cannot disagree silently.
        //
        // Read from the PIN rather than from the compiler that happens to be running: the pin is
        // what CI and every other machine build with, and a `RUSTUP_TOOLCHAIN` override in one
        // shell is not a fact about the repository. A save under such an override therefore writes
        // the pin, which is a lie only if the override was deliberate and then committed without
        // raising the pin, and that is the case `script/lint` catches from the other side.
        let pinned = pinned_nightly().unwrap_or_else(|| "unrecorded".into());
        // **The emulator that counted these instructions, read from the binary rather than from
        // the pin.** See `running_qemu` for why this one is asked and the toolchain one is not.
        // When the machine disagrees with `.qemu-version` the line says both, because that
        // disagreement is a fact about the numbers below and not a thing to tidy away: it is how
        // this project spent a month comparing counts from an emulator nobody had recorded.
        let ran = running_qemu(qemu_binary).unwrap_or_else(|| "unrecorded".into());
        let qemu = match pinned_qemu() {
            Some(pin) if pin != ran => format!("{ran} (run; .qemu-version pins {pin})"),
            _ => ran,
        };
        let mut out = format!(
            "# toolchain: {pinned}
# qemu: {qemu}
# bench/{stem}: deterministic icount tick counts (cargo xtask bench --save).
# Recorded against the QEMU pinned in .qemu-version. icount counts guest instructions, so the
# emulator version is part of what these numbers mean: script/qemu-check warns when the QEMU on
# PATH is not the pinned one, precisely because that is when a baseline comparison stops being
# apples to apples.
             # Updating this file is a statement that a performance change is intended and
             # understood; do it in the commit that causes the change. Checked by --check, a coarse
             # 10% tripwire (icount counts drift across builds; see notes/benchmarks.md).
",
        );
        for (name, ticks, iters) in &results {
            out.push_str(&format!(
                "{name} {ticks} {iters}
"
            ));
        }
        if let Err(e) = std::fs::write(&baseline_path, out) {
            eprintln!("bench: cannot write {}: {e}", baseline_path.display());
            return false;
        }
        eprintln!("bench: baseline saved to {}", baseline_path.display());
        return true;
    }

    if check {
        let Ok(text) = std::fs::read_to_string(&baseline_path) else {
            eprintln!(
                "bench: no baseline at {} (run `cargo xtask bench --save` first)",
                baseline_path.display()
            );
            return false;
        };
        let mut ok = true;

        // **Is this the emulator that produced the floors?** An icount count is a function of two
        // things, the compiler that emitted the instructions and the emulator that counted them,
        // and until 2026-09-21 this tree recorded neither. The compiler half is answered
        // statically by `script/lint`, because the pin is in the repository. This half cannot be:
        // nothing resolves QEMU from `.qemu-version`, so the only moment the question can be asked
        // is the moment an emulator is actually being run, which is here.
        //
        // **Compared against the emulator this run used, NOT against `.qemu-version`.** A check
        // against the pin would fail any baseline honestly recorded off-pin, which is to say it
        // would forbid the file from stating the truth; and it would still pass a comparison run
        // on a third version. The apples-to-apples question is whether the counter that produced
        // the floor is the counter reading it now.
        //
        // **`unrecorded` is a truthful answer and does not fail**, the posture
        // milestone 115 (the names that were refused) already takes for names. Every baseline in
        // the tree carries it today, because the emulator that produced those counts was never
        // written down and inventing one now would be worse than the gap. So this fires at full
        // strength from the first honest `--save` onward and never on a number nobody stamped.
        //
        // **What it means when it does fire**, and it is not "upgrade something": a re-record and
        // the pin have to be decided together, since CI builds the pinned emulator
        // (`script/ci-qemu`) and would then read these floors on it.
        let stamped_qemu = text
            .lines()
            .filter_map(|l| l.trim_start().strip_prefix("# qemu:"))
            .next()
            .and_then(|v| v.split_whitespace().next())
            .map(str::to_owned);
        match (stamped_qemu.as_deref(), running_qemu(qemu_binary)) {
            (None, _) | (Some("unrecorded"), _) => eprintln!(
                "bench: {} does not say which emulator produced it, so this comparison is \
                 assumed rather than known. The next --save records it.",
                baseline_path.display()
            ),
            (Some(want), Some(have)) if want != have => {
                let f = baseline_path.display();
                eprintln!(
                    "bench: CHECK FAIL: {f} was recorded on QEMU {want}; this run used {have}."
                );
                eprintln!(
                    "  icount counts guest instructions and a different emulator counts them \
                     differently, so a pass here would be evidence of nothing."
                );
                eprintln!(
                    "  Either run the emulator these floors were recorded on, or re-record them \
                     deliberately with --save and settle .qemu-version in the same commit."
                );
                ok = false;
            }
            (Some(want), Some(_)) => eprintln!("bench: emulator matches the baseline's ({want})"),
            (Some(_), None) => eprintln!(
                "bench: cannot ask {qemu_binary} its version; the emulator check is skipped."
            ),
        }

        // `trim_start` matters: this file's own header has INDENTED comment lines, which a
        // column-0-only check treats as data. They survive today only because they happen to split
        // into more than three tokens and fall through the destructure below. A three-word indented
        // comment would be silently parsed as a benchmark named after its first word.
        for line in text.lines().filter(|l| !l.trim_start().starts_with('#')) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            let [name, base, _iters] = parts.as_slice() else {
                continue;
            };
            let base: u64 = base.parse().unwrap_or(0);
            let Some((_, cur, _)) = results.iter().find(|(n, _, _)| n == name) else {
                eprintln!("bench: CHECK FAIL {name}: in the baseline but not in this run");
                ok = false;
                continue;
            };
            // A COARSE tripwire: 10% either way, with a small absolute floor so tiny counts do not
            // false-alarm. Not 2%: adding unrelated *live* code shifts even untouched benchmarks by
            // several percent, non-uniformly, because the compiler remakes whole-crate inlining and
            // monomorphization decisions (measured: a new bench function moved yield_switch -7% while
            // ipc_rtt went +1.8%). So icount --check catches a gross regression, "you 3x'd IPC," not
            // a 3% one; --real medians, read by a human, are the fine signal. See notes/benchmarks.md.
            let slack = (base / 10).max(64);
            let (lo, hi) = (base.saturating_sub(slack), base + slack);
            if *cur < lo || *cur > hi {
                let delta = *cur as i64 - base as i64;
                eprintln!(
                    "bench: CHECK FAIL {name}: {cur} vs baseline {base} ({delta:+} ticks,                      allowed +-{slack})"
                );
                ok = false;
            }
        }
        if ok {
            eprintln!("bench: check passed (all within 10% of baseline; coarse tripwire)");
        } else {
            eprintln!();
            eprintln!(
                "bench: a benchmark moved. If intended, rerun with --save and commit the new                  baseline WITH the change that moved it."
            );
        }
        return ok;
    }

    true
}
