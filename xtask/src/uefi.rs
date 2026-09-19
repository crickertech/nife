//! The bootable UEFI image (milestone 87): the entry real firmware can start.
//!
//! Staged at `target/esp` for a QEMU/OVMF boot or for a FAT32 stick, and booted under OVMF
//! by `uefi-boot` and `uefi-test`. See notes/x86-uefi-boot.md.

/// The `x86_64-unknown-uefi` target (milestone 87): PE/COFF rather than ELF, entered by real
/// firmware in long mode. Only `uefi_loader`'s binary half is ever built for it.
const UEFI_TARGET: &str = "x86_64-unknown-uefi";

/// **The EFI system partition, staged as a directory** (milestone 87).
///
/// A directory rather than an image, because nothing here has to build a FAT filesystem: QEMU's
/// vvfat driver synthesises one from a directory (`scripts/qemu-uefi-x86_64.sh`), and a USB stick
/// is formatted by the person holding it. That is the same fact from both ends, and it is why this
/// milestone needed no new host tooling at all.
fn esp_dir() -> std::path::PathBuf {
    workspace_root().join("target/esp")
}

/// **Build the bootable UEFI image** (milestone 87): the kernel, the userspace archive, and the
/// loader that carries both, staged where firmware looks for them.
///
/// ```text
/// cargo xtask uefi-image
/// # then, under QEMU with real firmware:
/// scripts/qemu-uefi-x86_64.sh target/esp
/// # or, on the Dell OptiPlex: copy target/esp/EFI/BOOT/BOOTX64.EFI to a FAT32 stick, same path.
/// ```
///
/// **The order is a dependency, not a preference.** `uefi_loader` embeds the kernel ELF and the
/// archive with `include_bytes!`, so both have to exist and be current before it is compiled; its
/// build script takes their paths from the environment and refuses to build without them, rather
/// than guessing at `target/`. That refusal is the mechanism keeping a stale `.efi` from being
/// possible to produce by hand.
///
/// # BUGS
///
/// - **`BOOTX64.EFI` is the removable-media fallback path**, which is what a USB stick uses and
///   what OVMF finds with no configuration. Installing to the machine's own ESP with a boot entry
///   of its own (`efibootmgr`'s job on Linux) is not done here, and is what a machine that boots
///   nife by default would need.
fn uefi_image() -> bool {
    // **The archive FIRST, then the kernel, and the order is load-bearing.** Packing the archive
    // regenerates `target/init-measure-x86_64.txt`, the manifest `kernel/build.rs` compiles in as
    // the measured-boot trust root. Kernel-first builds a kernel vouching for the PREVIOUS archive,
    // and the gate then refuses the pair at the point of handover:
    //
    //     MEASURED BOOT REFUSED: no measurement for the archive entry 'progenitor'
    //
    // **This is the second time this defect has reached a bench**, which is why the comment is here
    // rather than in a note. `script/board-image` had it for riscv64 and the VisionFive 2 refused
    // the pair on 2026-08-15 (boot 12); the fix there carries a comment saying "QEMU never hit it
    // because xtask orders these correctly", which was true of the riscv64 path and false of this
    // one. xenon refused the pair the same way on 2026-09-17, after its self-test passed.
    //
    // QEMU does not catch it because a developer who runs the kernel and the archive from one
    // working tree usually has both fresh; it bites when the kernel is already built, which is
    // every time a lane has compiled it earlier in the session.
    //
    // `initrd_x86` builds `components`, never the kernel, so the dependency runs one way only and
    // this order is the safe one as well as the correct one.
    if !initrd_x86() || !cargo_profiled(&["build", "-p", "kernel", "--target", X86_TARGET]) {
        return false;
    }

    let kernel = workspace_root()
        .join(format!("target/{X86_TARGET}/{}/kernel", profile_dir()))
        .display()
        .to_string();

    uefi_stage(
        &kernel,
        &esp_dir(),
        "the loader, the kernel and the archive",
    )
}

/// **Where the test build's EFI system partition is staged** (milestone 195).
///
/// A second directory rather than `target/esp`, and the separation is the point rather than
/// tidiness. `esp_dir()` is what the bench procedure copies to a USB stick, and a stick carries no
/// sign of which kernel is inside the one file on it (`notes/x86-uefi-boot.md`'s last `BUGS`
/// entry). If the suite run wrote over that directory, a `cargo xtask uefi-boot` followed by a copy
/// would put the *test* kernel on the machine, which boots, prints a tour, runs 200 tests and then
/// asks QEMU to exit on a port no Dell answers. Two directories make that unrepresentable.
fn uefi_test_esp_dir() -> std::path::PathBuf {
    workspace_root().join("target/esp-test")
}

/// **Build a UEFI application around one kernel ELF and stage it where firmware looks**
/// (milestone 87, split out by milestone 195 so the tour and the test suite can each have one).
///
/// The caller owns the kernel: this builds `uefi_loader` with `NIFE_UEFI_KERNEL` pointing at it,
/// and the loader's build script `include_bytes!`s both it and the archive. `what` is the phrase
/// the size line uses, because "the loader, the kernel and the archive" and "the loader, the test
/// kernel and the archive" are the one difference a reader of the transcript can act on.
fn uefi_stage(kernel: &str, esp: &std::path::Path, what: &str) -> bool {
    let mut args = std::vec![
        "build",
        "-p",
        "uefi_loader",
        "--bin",
        "uefi_loader",
        "--features",
        "uefi",
        "--target",
        UEFI_TARGET,
    ];
    if RELEASE.load(Ordering::Relaxed) {
        args.push("--release");
    }
    // Not `cargo()`: that helper exports the aarch64 runner's `NIFE_INITRD`/`NIFE_DISK`/`NIFE_NET`,
    // none of which means anything to a UEFI build, and the two archive variables would then differ
    // by one character in a way nothing would catch.
    let built = Command::new("cargo")
        .args(&args)
        .env("NIFE_UEFI_KERNEL", kernel)
        .env("NIFE_UEFI_INITRD", x86_initrd_path())
        .status()
        .map(|s| s.success())
        .unwrap_or_else(|e| {
            eprintln!("uefi-image: failed to run cargo: {e}");
            false
        });
    if !built {
        return false;
    }

    let efi = workspace_root().join(format!(
        "target/{UEFI_TARGET}/{}/uefi_loader.efi",
        profile_dir()
    ));
    // `\EFI\BOOT\BOOTX64.EFI` is the removable-media path every UEFI implementation looks for with
    // no configuration at all, which is what makes the bench procedure "copy one file to a stick".
    let boot_dir = esp.join("EFI/BOOT");
    if let Err(e) = std::fs::create_dir_all(&boot_dir) {
        eprintln!("uefi-image: cannot create {}: {e}", boot_dir.display());
        return false;
    }
    let target = boot_dir.join("BOOTX64.EFI");
    if let Err(e) = std::fs::copy(&efi, &target) {
        eprintln!(
            "uefi-image: cannot copy {} to {}: {e}",
            efi.display(),
            target.display()
        );
        return false;
    }
    let size = std::fs::metadata(&target).map(|m| m.len()).unwrap_or(0);
    eprintln!("wrote {} ({size} bytes: {what})", target.display());
    eprintln!(
        "  under QEMU with real firmware: scripts/qemu-uefi-x86_64.sh {}",
        esp.display()
    );
    eprintln!("  on the bench: copy that file to a FAT32 stick as /EFI/BOOT/BOOTX64.EFI");
    true
}

/// **Boot the UEFI image under OVMF and check the firmware path actually ran** (milestone 87).
///
/// This is the gate, and what it asserts is chosen so that it cannot pass for the wrong reason:
///
/// - **`(xsdt)` in the ACPI line.** Under QEMU's PVH loader the RSDP is found by *scanning* the
///   BIOS area, and what turns up is an ACPI 1.0 pointer with an RSDT root. Real firmware hands
///   over a revision-2 RSDP with an XSDT. So this string is only printable if the loader read the
///   UEFI configuration table and the kernel walked the 64-bit root, which is a path that had never
///   executed before this milestone.
/// - **no `rsdp 0x0`.** The PVH handoff's own tell, and the thing this loader exists to fix.
/// - **the tour's completion line.** Everything between the two: the fine page tables, the APIC,
///   the timer, the scheduler and two ring-3 processes, on a memory map from firmware rather than
///   from a hypervisor.
/// - **the ECAM window enabled from the MCFG** (milestone 165). The runner boots at 2 GiB rather
///   than the PVH runner's 256 MiB, which is what puts firmware's tables above 1 GiB, where a real
///   machine keeps them. On 2026-09-02 that boot found no ACPI at all, because the walk's reach
///   bound disagreed with `boot.s` by 4x; asserting the *end* of the chain (a PCIe window whose
///   base came from a table read at a high physical address) is what makes the whole chain a gate
///   rather than the three separate facts it is made of.
///
/// - **the shell on the screen, answering a serial keystroke** (the shell on the firmware screen,
///   milestone 198's rung 1b). The screen is read three times: the kernel's tour (milestone 243),
///   the prompt once the userspace terminal has taken the screen over, and the answer to a command
///   typed on COM1. See [`screen_watch`].
///
/// The boot is bounded by the runner script; a kernel that hangs fails this by producing none of
/// the three rather than by hanging the gate. It is stopped as soon as the screen has answered.
fn uefi_boot() -> bool {
    if !uefi_image() {
        return false;
    }
    eprintln!();
    eprintln!("--- boot under real firmware, x86_64 (QEMU q35 + OVMF) ---");

    // **The screen half** (milestone 243). A QEMU monitor on a unix socket, a poller that asks it
    // for a screendump until the tour's last line is *on the screen*, and `board_console::screen` to
    // turn the picture back into text. Everything the framebuffer path can get wrong shows up here
    // and nowhere else: the loader's `LocateProtocol`, the byte order, the stride, the mapping
    // surviving `mmu::init`, and the glyphs. The serial transcript below would be identical if the
    // screen were black.
    //
    // **And then the shell on it** (the shell on the firmware screen, milestone 198's rung 1b): the
    // same poller keeps reading until the prompt is on the screen, types one command on the SERIAL
    // line, and reads its answer back off the SCREEN. That is the whole rung in one exchange: the
    // console server writes both surfaces, the keystrokes still arrive over COM1, and what reaches
    // the monitor is the shell and not the kernel. So it needs the runner's stdin, which is COM1.
    //
    // In /tmp rather than under target/, because a unix socket path is capped at 104 bytes by the
    // OS and a worktree checkout plus `target/` gets close. Same reason `gpu_shot` does it.
    let sock = format!("/tmp/nife-uefi-screen-{}.sock", std::process::id());
    let shot =
        std::path::PathBuf::from(format!("/tmp/nife-uefi-screen-{}.ppm", std::process::id()));
    let _ = std::fs::remove_file(&sock);
    let _ = std::fs::remove_file(&shot);

    let mut child = match Command::new("scripts/qemu-uefi-x86_64.sh")
        .arg(esp_dir())
        .current_dir(workspace_root())
        .env("NIFE_SCREEN_MON", &sock)
        // **Two cores** (milestone 195), where every other x86_64 boot in this tree takes one.
        // `arch::x86_64::ap_boot` copies its real-mode trampoline to physical 0x8000, a page no
        // loader had ever asked the firmware for, so secondary cores under firmware worked or did
        // not by luck. `uefi_loader` asks for it by name now and this is what checks that the
        // asking worked. It also puts a second local APIC on the machine, which is the third thing
        // milestone 215's BUGS listed as answerable only on xenon: whether a machine with more than
        // one still delivers device interrupts to the boot core's id. The tour's `device irq` line
        // is that answer.
        //
        // The suite below stays at one core, because the two-core AP defect this tour does not
        // touch (`ap_boot`'s BUGS #3) fails one of its tests about half the time.
        .env("NIFE_SMP", "2")
        // **The bare firmware machine, whether or not a suite leg ran first.** `test()` sets
        // `NIFE_DISK` and `NIFE_NVME` in this process for the PVH leg, and a child inherits them,
        // so without this the tour would attach two devices inside `script/test` and none from a
        // bare `cargo xtask uefi-boot`. One boot with two machines is one boot nobody can compare.
        // The suite below is where the devices belong.
        .env_remove("NIFE_DISK")
        .env_remove("NIFE_NVME")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            eprintln!("uefi-boot: failed to run scripts/qemu-uefi-x86_64.sh: {e}");
            return false;
        }
    };
    // Both streams collected on threads of their own, because the watcher below decides when the
    // boot has said enough and a blocking read here would decide it instead.
    let collect = |mut from: Box<dyn std::io::Read + Send>| {
        std::thread::spawn(move || {
            let mut bytes = Vec::new();
            let _ = from.read_to_end(&mut bytes);
            String::from_utf8_lossy(&bytes).into_owned()
        })
    };
    let stdout = collect(Box::new(child.stdout.take().expect("piped stdout")));
    let stderr = collect(Box::new(child.stderr.take().expect("piped stderr")));
    let serial = child.stdin.take().expect("piped stdin");
    let watcher = {
        let (sock, shot) = (sock.clone(), shot.clone());
        std::thread::spawn(move || screen_watch(&sock, &shot, serial))
    };
    let screen = watcher.join().unwrap_or_default();
    // **Stop the machine once the screen has answered**, rather than waiting out the runner's
    // bound: the kernel never exits, and everything asserted below has been printed by the time the
    // prompt answered a command. SIGTERM to the wrapper, which is the signal `qemu-bounded.sh`
    // forwards to QEMU; its own killer is the backstop if this is lost.
    let _ = Command::new("kill")
        .args(["-TERM", &child.id().to_string()])
        .status();
    let _ = child.wait();
    let transcript = stdout.join().unwrap_or_default() + &stderr.join().unwrap_or_default();
    print!("{transcript}");

    let mut ok = true;
    for wanted in [
        // The self-test verdict and the hand-over, rather than the halt line this used to want:
        // milestone 182 made x86_64 hand the machine to the progenitor instead of halting, and
        // this boot carries the archive (`uefi_stage`'s `NIFE_UEFI_INITRD`), so under real
        // firmware it reaches both.
        boot_ladder::SELF_TEST,
        "nife: handing the system to the userspace progenitor.",
        "(xsdt)",
        "pci         : ecam at",
        // Two cores ONLINE, not two in the MADT: the difference is whether the trampoline page the
        // loader asked for was actually usable. See the `NIFE_SMP` comment above.
        "smp: 2 core(s) online",
        // And a device interrupt still landing on the boot core with two local APICs present.
        "device irq  : pit irq 0 -> gsi 2",
        // The kernel handed the screen to a userspace terminal and both halves came up (the shell
        // on the firmware screen). Said on the UART because the screen has been handed away by
        // the time it is printed.
        "served by framebuffer_driver, a 132x43 terminal on it",
    ] {
        if !transcript.contains(wanted) {
            eprintln!("uefi-boot: the boot transcript is missing {wanted:?}");
            ok = false;
        }
    }
    if transcript.contains("rsdp 0x0") {
        eprintln!("uefi-boot: the kernel was handed a zero ACPI root pointer, which is PVH's tell");
        ok = false;
    }
    // A table the walk could not reach is printed by `table_at` rather than skipped quietly, so
    // the gate can fail on it by name. This is the one failure that looks like a working boot from
    // outside: the tour completes, and the machine simply has no devices.
    if transcript.contains("outside the boot map") {
        eprintln!(
            "uefi-boot: an ACPI table was outside the boot map's reach, so this machine's APICs, \
             PCIe window or IOMMU went undiscovered"
        );
        ok = false;
    }

    // --- What was on the SCREEN: milestone 243's tour, then the shell ---
    let _ = std::fs::remove_file(&sock);
    match &screen.tour {
        Some(text) => {
            let rows = text.lines().filter(|l| !l.is_empty()).count();
            eprintln!(
                "uefi-boot: read {rows} non-blank row(s) of the tour back off the framebuffer, ending"
            );
            let tail: Vec<&str> = text.lines().filter(|l| !l.is_empty()).collect();
            for line in tail.iter().rev().take(3).rev() {
                eprintln!("uefi-boot:   | {line}");
            }
        }
        None => {
            eprintln!(
                "uefi-boot: the tour was never readable on the screen. The serial transcript above \
                 says whether the kernel ran at all; if it did, the framebuffer path is what broke \
                 (the loader's LocateProtocol, the pixel order, the stride, or the mapping \
                 surviving mmu::init). Last dump: {}",
                shot.display()
            );
            ok = false;
        }
    }
    match (&screen.prompt, &screen.answer) {
        (Some(_), Some(text)) => {
            eprintln!(
                "uefi-boot: the shell is on the screen, and `{UEFI_SCREEN_COMMAND}` typed on the \
                 serial line answered there:"
            );
            for line in text.lines().filter(|l| !l.is_empty()) {
                eprintln!("uefi-boot:   | {line}");
            }
        }
        (Some(text), None) => {
            eprintln!(
                "uefi-boot: the prompt reached the screen, but `{UEFI_SCREEN_COMMAND}` typed on the \
                 serial line never answered there. Last screen read:"
            );
            for line in text.lines().filter(|l| !l.is_empty()) {
                eprintln!("uefi-boot:   | {line}");
            }
            ok = false;
        }
        (None, _) => {
            eprintln!(
                "uefi-boot: the shell's prompt never reached the screen. If the transcript has the \
                 prompt, the console server is not writing to the screen terminal; if it has the \
                 `served by framebuffer_driver` line, the terminal came up and drew nothing \
                 readable. Last dump: {}",
                shot.display()
            );
            ok = false;
        }
    }

    if ok {
        eprintln!("uefi-boot: booted under OVMF from \\EFI\\BOOT\\BOOTX64.EFI");
    }
    ok
}

/// The line the screen has to be showing for milestone 243 to have worked.
///
/// **Near the end of the boot on purpose**: a 1280x800 screen is 100 character rows and the boot is
/// longer than that, so the early lines have scrolled off by the time anything reads the picture.
/// Asserting on a line that is still there is the difference between a gate and a flaky one.
///
/// It was the halt line, `nife x86_64: boot complete, halting.`, until milestone 182 removed the
/// halt. The self-test verdict replaced it because it is the last line **every** boot prints
/// whatever it hands over to next; it sits a few dozen rows above the bottom, well inside the 100.
const UEFI_SCREEN_MARKER: &str = boot_ladder::SELF_TEST;

/// The command `uefi-boot` types on the serial line once the prompt is on the screen, and
/// [`UEFI_SCREEN_ANSWER`] the line it must print there. Words that appear nowhere in the boot, so
/// finding the answer cannot be finding something the tour said.
const UEFI_SCREEN_COMMAND: &str = "echo typed on the wire";
/// What [`UEFI_SCREEN_COMMAND`] prints, as a whole screen row.
const UEFI_SCREEN_ANSWER: &str = "typed on the wire";

/// What the screen showed, stage by stage. Each is the decoded screen at the moment that stage was
/// seen, or `None` when it never was.
#[derive(Default)]
struct ScreenReadings {
    /// The kernel's tour, with [`UEFI_SCREEN_MARKER`] on it (milestone 243).
    tour: Option<String>,
    /// The shell's prompt, a row that starts `$ `.
    prompt: Option<String>,
    /// [`UEFI_SCREEN_ANSWER`] as a whole row, after [`UEFI_SCREEN_COMMAND`] was typed.
    answer: Option<String>,
}

/// **Poll the QEMU monitor through three stages of what the screen shows** (milestone 243, then
/// the shell on the firmware screen).
///
/// Runs on its own thread beside the boot, because the kernel never exits and the runner therefore
/// does not return until its own timeout fires: by then QEMU is gone and there is nothing left to
/// photograph. The stages, in the order they must happen:
///
/// 1. **The tour's marker.** The kernel paints the boot tour until it hands the screen to the
///    userspace terminal, which clears it. So this stage has a window (from the self-test verdict
///    to the handover, a couple of seconds under TCG) and is polled faster than the others;
///    missing it is a failure, because it is milestone 243's claim.
/// 2. **The prompt.** A row starting `$ `, which the kernel's tour never prints.
/// 3. **The answer.** [`UEFI_SCREEN_COMMAND`] is written to `serial` (COM1), and a row equal to
///    [`UEFI_SCREEN_ANSWER`] must appear. The command's own echo is `$ echo ...`, a different row,
///    so this row is only there if the shell ran the command and its output reached the screen.
///
/// Stops early when the monitor stops answering after having answered once (QEMU is gone and no
/// later dump can be better), and at a deadline that bounds the whole watch. A dump that fails to
/// decode is never fatal: `screendump` writes the file asynchronously, so a read that lands
/// mid-write is short and ordinary, and the retry is the answer.
fn screen_watch(sock: &str, shot: &Path, mut serial: std::process::ChildStdin) -> ScreenReadings {
    use std::io::Write;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(180);
    let mut answered = false;
    let mut seen = ScreenReadings::default();
    let mut typed = false;
    while std::time::Instant::now() < deadline {
        if !screendump(sock, shot) {
            if answered {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(500));
            continue;
        }
        answered = true;
        // A screen that decodes but does not hold what the stage wants is deliberately NOT stored:
        // an incomplete picture must not read as a pass. The dump file itself is the artefact,
        // and the caller prints its path on failure.
        let text = std::fs::read(shot)
            .ok()
            .and_then(|bytes| board_console::screen::read(&bytes).ok());
        if let Some(text) = text {
            if seen.tour.is_none() && text.contains(UEFI_SCREEN_MARKER) {
                seen.tour = Some(text.clone());
            }
            if seen.prompt.is_none() && text.lines().any(|l| l.starts_with("$ ")) {
                seen.prompt = Some(text.clone());
            }
            if typed && text.lines().any(|l| l == UEFI_SCREEN_ANSWER) {
                seen.answer = Some(text);
                break;
            }
            // Typed once, after the prompt is on the screen: the line editor echoes a keystroke
            // the moment it arrives, so typing earlier would be typing into the boot.
            if seen.prompt.is_some() && !typed {
                typed = writeln!(serial, "{UEFI_SCREEN_COMMAND}")
                    .and_then(|()| serial.flush())
                    .is_ok();
            }
        }
        // Stage 1 has a window; the others wait on a person-speed shell.
        let pause = if seen.tour.is_none() { 50 } else { 500 };
        std::thread::sleep(std::time::Duration::from_millis(pause));
    }
    seen
}

/// **Run the kernel suite under real firmware** (milestone 195), rather than the tour
/// `uefi_boot` boots.
///
/// The difference between the two is one ELF. `#[cfg(test)]` in `kernel_main`'s `x86_64` arm runs
/// `test_main()` at the end of the same tour, so this boot prints every line `uefi-boot` asserts on
/// *and then* runs the suite, and the verdict it adds is the one thing the tour cannot say: that
/// the tests pass when the memory map, the ACPI root and the PCIe window came from firmware
/// instead of from a hypervisor. Until this existed, "it boots under real firmware" and "it passes
/// under real firmware" were different claims and only the first was made.
///
/// **The exit status is half the verdict and is checked as such.** The suite reports through
/// `isa-debug-exit`, which terminates QEMU with `(value << 1) | 1`, so a passing run is process
/// status 3 (`arch::x86_64::semihosting::EXIT_SUCCESS`) and a failing one is 1. A transcript scan
/// alone would pass a run whose harness printed its verdict and then faulted on the way out; the
/// status alone would pass a QEMU that never started the guest. Both, and neither is redundant.
///
/// # BUGS
///
/// - **It is a second firmware boot, not a replacement for the first.** `uefi-boot` still boots the
///   tour build, because the tour build is what `uefi-image` stages for the USB stick and what
///   calef carries to the bench; a regression that only the shipping image has would otherwise be
///   gated by nothing.
fn uefi_test() -> bool {
    if !initrd_x86() || !mkdisk() || !mknvmedisk() {
        return false;
    }
    let Some(kernel) = kernel_test_elf(X86_TARGET, "uefi-test") else {
        return false;
    };
    if !uefi_stage(
        &kernel,
        &uefi_test_esp_dir(),
        "the loader, the test kernel and the archive",
    ) {
        return false;
    }
    eprintln!();
    eprintln!("--- kernel tests under real firmware, x86_64 (QEMU q35 + OVMF) ---");

    let output = match Command::new("scripts/qemu-uefi-x86_64.sh")
        .arg(uefi_test_esp_dir())
        .current_dir(workspace_root())
        // **The devices the PVH runner attaches** (milestone 195), which the tour above needs none
        // of. They close the second of the three things milestone 215's BUGS listed as answerable
        // only on xenon: a PCI function whose BARs were placed by FIRMWARE rather than by this
        // kernel's own bus walk. Under `-kernel` the kernel assigns them itself, so "the driver can
        // reach an MSI-X table a bus walk found" and "the driver can reach one it put there" were
        // the same sentence.
        //
        // **One core here, two in the tour above**, and the split is a known defect rather than a
        // preference: `every_secondary_runs_scheduled_work` fails about half the time at two cores
        // on this architecture (`arch::x86_64::ap_boot`'s BUGS #3), which is why the PVH runner
        // defaults to one as well. The tour does not run that test, so it is where the second core
        // is gated.
        //
        // `NIFE_DISK` and `NIFE_NVME` are already set by `test()` for the PVH leg that ran just
        // above, so this inherits them; they are named here only for a bare `cargo xtask uefi-test`.
        .env("NIFE_DISK", disk_path())
        .env("NIFE_NVME", nvme_disk_path())
        .output()
    {
        Ok(o) => o,
        Err(e) => {
            eprintln!("uefi-test: failed to run scripts/qemu-uefi-x86_64.sh: {e}");
            return false;
        }
    };
    let transcript = String::from_utf8_lossy(&output.stdout).into_owned()
        + &String::from_utf8_lossy(&output.stderr);
    print!("{transcript}");

    let mut ok = true;
    // `(xsdt)` and the ECAM line are `uefi_boot`'s own assertions, repeated here rather than
    // assumed: this is a different binary, and a test build that took the PVH fallback path would
    // otherwise report a green suite on a machine it had discovered the wrong way.
    for wanted in ["test result: ok.", "(xsdt)", "pci         : ecam at"] {
        if !transcript.contains(wanted) {
            eprintln!("uefi-test: the boot transcript is missing {wanted:?}");
            ok = false;
        }
    }
    if transcript.contains("rsdp 0x0") {
        eprintln!("uefi-test: the kernel was handed a zero ACPI root pointer, which is PVH's tell");
        ok = false;
    }
    // 3, not 0: see the doc comment. `scripts/qemu-uefi-x86_64.sh` execs QEMU rather than
    // translating this the way the PVH runner does, because that runner is a cargo `runner` and has
    // to speak cargo's success convention while this one has only ever had one caller.
    if output.status.code() != Some(i32::from(X86_DEBUG_EXIT_SUCCESS)) {
        eprintln!(
            "uefi-test: qemu exited {:?}, not {X86_DEBUG_EXIT_SUCCESS} (the harness's own success \
             status through isa-debug-exit)",
            output.status.code()
        );
        ok = false;
    }
    if ok {
        eprintln!("uefi-test: the kernel suite passed under OVMF");
    }
    ok
}

/// The process status a passing `x86_64` suite produces, which is not zero and cannot be.
///
/// `isa-debug-exit` terminates QEMU with `(value << 1) | 1`, so every status it can report is odd.
/// The guest writes 1; QEMU exits 3. The matching half is `EXIT_SUCCESS` in
/// `kernel/src/arch/x86_64/semihosting.rs`, and `scripts/qemu-runner-x86_64.sh` translates the same
/// number for cargo's benefit. Three files naming one number is the cost of a convention QEMU owns.
const X86_DEBUG_EXIT_SUCCESS: u8 = 3;
