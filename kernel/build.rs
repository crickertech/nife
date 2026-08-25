//! Tell the linker to use our layout instead of the default one.
//!
//! `cargo:rustc-link-arg` applies to binaries *and* test binaries, which is what we
//! want: the test build has to boot in QEMU too, so it needs the same layout.

fn main() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap();

    // Each architecture has its own memory map (aarch64 links high with an AT()-relocated load
    // address; riscv links at the OpenSBI payload address). The linker script is arch-specific, so
    // it is selected here rather than hard-coded. See notes/riscv-port.md.
    let (link_script, boot_asm) = match arch.as_str() {
        "aarch64" => ("link-aarch64.ld", "src/arch/aarch64/boot.s"),
        "riscv64" => ("link-riscv64.ld", "src/arch/riscv64/boot.s"),
        // x86_64 (milestone 161). Its script is the only one of the three with two worlds in it: a
        // low 32-bit `.boot` for the trampoline and a high 64-bit everything-else. See
        // link-x86_64.ld's header for why that is forced rather than chosen.
        "x86_64" => ("link-x86_64.ld", "src/arch/x86_64/boot.s"),
        other => panic!("nife has no linker script for target arch {other}"),
    };

    println!("cargo::rerun-if-changed={link_script}");
    println!("cargo::rerun-if-changed={boot_asm}");
    println!("cargo::rustc-link-arg=-T{manifest_dir}/{link_script}");

    declare_initrd_cfg(&arch);
    generate_trust_root(&manifest_dir, &arch);
}

/// **`cfg(initrd)`: does this target have user programs to pack into one?** (milestone 161, roadmap
/// item 4.)
///
/// A great many `#[test_case]`s in `kernel/src/user/` are portable in every respect except that
/// their fixture is a **real ELF binary read out of the initrd archive**, and `x86_64-unknown-none`
/// has none: `crates/user_rt` has no arms for that ISA and `user/build.rs` cannot compile its C
/// components for it, so nothing in `user/` builds and `xtask` packs no archive. See
/// notes/x86-port.md.
///
/// **The cfg names the reason rather than the architecture**, which is the whole point of spending a
/// build script on it. `#[cfg(all(test, initrd))]` on a module reads "this needs an initrd", and
/// stays true; `#[cfg(not(target_arch = "x86_64"))]` would have said "not on x86" twenty-six times
/// over and would be wrong the day x86 gets user programs, in twenty-six places nobody would think
/// to look. The day this port can build them, one arm of the match below changes and every one of
/// those modules comes back at once.
///
/// **Name provisional** (milestone 161): calef names things, and a `cfg` a reader meets in front of
/// a module is as reader-facing as a crate.
fn declare_initrd_cfg(arch: &str) {
    // Declare it whatever the answer, so `unexpected_cfgs` stays a useful lint rather than being
    // silenced: a typo'd `#[cfg(initd)]` should still be caught.
    println!("cargo::rustc-check-cfg=cfg(initrd)");
    match arch {
        "aarch64" | "riscv64" => println!("cargo::rustc-cfg=initrd"),
        // x86_64, and anything a fourth port adds before it can build userspace.
        _ => {}
    }
}

/// **Compile the boot program's measurement into the kernel image** (milestone 22 phase B.1).
///
/// The build packs the initrd archive first (`xtask::mkinitrd` / `initrd_riscv`), hashes the entries
/// the kernel may enter as init, and writes `target/init-measure-<arch>.txt`. Here we turn that
/// manifest into `TRUST_ROOT`, a `&[measured_boot::Measurement]` in the kernel's own `.rodata`. That is
/// what makes the check mean "this kernel image runs exactly this init" with no key management: the
/// expected digest is part of the thing doing the checking.
///
/// **The ordering, and why it is not circular.** The kernel image contains the hash of a
/// separately-built initrd, so the initrd must exist first: userspace builds, the archive is packed,
/// the manifest is written, *then* the kernel compiles. Every xtask path already had that order
/// (`user()` before the kernel build, because the kernel boots with the archive as `-initrd`), so
/// nothing needed resequencing. The hash never feeds back into the initrd, so there is no
/// chicken-and-egg: it is a one-way dependency the build already had, now made explicit.
///
/// A **missing** manifest yields an empty trust root rather than a build failure, so a bare
/// `cargo clippy -p kernel` (script/lint) and a bare `cargo build` still work. The kernel refuses to
/// boot on an empty root (`measured_boot::VerifyError::Unmeasured`), which is the right place for that
/// failure: loud at boot, not confusing at lint time. A *malformed* manifest line is a hard error,
/// because that means the build wrote something we cannot read and silently measuring nothing would
/// be the one outcome worse than stopping.
fn generate_trust_root(manifest_dir: &str, arch: &str) {
    let manifest = std::path::Path::new(manifest_dir)
        .parent()
        .expect("the kernel manifest dir has no parent")
        .join(format!("target/init-measure-{arch}.txt"));

    // Rebuild the kernel whenever the measurement changes, which is to say whenever userspace
    // changes. That relink is the honest cost of "this kernel runs exactly this init."
    println!("cargo::rerun-if-changed={}", manifest.display());

    let text = std::fs::read_to_string(&manifest).unwrap_or_default();
    let mut entries = String::new();
    // The format has one definition (`measured_boot::manifest_entries`), shared with init, which
    // reads the same shape out of the archive at boot (milestone 104). This side turns an
    // unparseable line into a hard error where init treats it as a refusal, and the asymmetry is
    // deliberate: at build time we can still stop, and a manifest we cannot read means the build
    // wrote something we cannot read.
    for entry in measured_boot::manifest_entries(&text) {
        let (name, digest) = entry.unwrap_or_else(|line| {
            panic!(
                "malformed measurement line in {}: {line:?}",
                manifest.display()
            )
        });
        let bytes: Vec<String> = digest.iter().map(|b| format!("{b:#04x}")).collect();
        entries.push_str(&format!(
            "    measured_boot::Measurement {{ name: {name:?}, digest: [{}] }},\n",
            bytes.join(", ")
        ));
    }

    let generated = format!(
        "// Generated by kernel/build.rs from {}. Do not edit.\n\
         pub static TRUST_ROOT: &[measured_boot::Measurement] = &[\n{entries}];\n",
        manifest.display()
    );
    let out = std::path::Path::new(&std::env::var("OUT_DIR").unwrap()).join("trust_root.rs");
    std::fs::write(&out, generated).expect("cannot write the generated trust root");
}
