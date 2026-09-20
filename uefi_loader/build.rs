//! **Embed the kernel and the userspace archive in the loader** (milestone 87).
//!
//! One file goes on the FAT32 stick, not three, and this loader therefore speaks no
//! `SimpleFileSystem` protocol at all. The cost is honest and recorded in `src/main.rs`'s `BUGS`:
//! the loader has to be rebuilt whenever the kernel changes, which is why `cargo xtask uefi-image`
//! is the only supported way to build it.
//!
//! Both paths arrive as environment variables rather than being guessed from `target/`, because
//! this package has no idea which profile or which archive the caller meant, and a build script
//! that guesses wrong produces a stick that boots last week's kernel.

use std::path::PathBuf;
use std::{env, fs};

/// **Refuse to embed a kernel beside an archive it does not vouch for.**
///
/// The kernel compiles in the SHA-256 of each archive entry it will enter (`kernel/build.rs`'s
/// trust root) and halts at `MEASURED BOOT REFUSED` on a mismatch. That halt has reached a bench
/// twice (radon on 2026-08-15, xenon on 2026-09-17), both times because a kernel and an archive from
/// different builds were put side by side. This loader is the last place the two meet before they
/// become one file, so this is where the mismatch stops being expressible: the build fails, with the
/// entry named, instead of producing a boot file that halts.
///
/// **The comparison itself lives in `sealed_pair`**, built for milestone 223 (read a card and say whether its kernel and archive match),
/// which is where a card checker asks the same
/// question about files it did not build. It used to be written out here, and a second copy of a
/// measured-entry list is exactly the thing that goes stale without anybody noticing: a kernel that
/// grew a third boot program would have been vouched for by a check that had never heard of it.
fn refuse_an_unsealed_pair(kernel_path: &str, initrd_path: &str) {
    let kernel = fs::read(kernel_path).unwrap_or_else(|e| panic!("cannot read {kernel_path}: {e}"));
    let archive =
        fs::read(initrd_path).unwrap_or_else(|e| panic!("cannot read {initrd_path}: {e}"));
    let seal =
        sealed_pair::inspect(&kernel, &archive).unwrap_or_else(|e| panic!("{initrd_path} {e}"));
    assert!(
        seal.is_sealed(),
        "\n\n{}\nBuild the archive first and then the kernel, which is what `cargo xtask \
         uefi-image` and `cargo xtask stick` do.\n",
        seal.explain(kernel_path, initrd_path)
    );
}

fn main() {
    println!("cargo::rerun-if-env-changed=NIFE_UEFI_KERNEL");
    println!("cargo::rerun-if-env-changed=NIFE_UEFI_INITRD");

    let building_the_application = env::var_os("CARGO_FEATURE_UEFI").is_some();
    let kernel = env::var("NIFE_UEFI_KERNEL").ok().filter(|p| !p.is_empty());
    let initrd = env::var("NIFE_UEFI_INITRD").ok().filter(|p| !p.is_empty());

    let mut generated = String::new();

    match &kernel {
        Some(path) => {
            println!("cargo::rerun-if-changed={path}");
            generated.push_str(&format!(
                "/// The kernel ELF this loader places and enters.\n\
                 pub static KERNEL: &[u8] = include_bytes!(r\"{path}\");\n"
            ));
        }
        None if building_the_application => panic!(
            "NIFE_UEFI_KERNEL is unset. The UEFI application embeds the kernel it boots, so it \
             cannot be built directly; run `cargo xtask uefi-image`, which builds the kernel and \
             the archive first and passes both paths in."
        ),
        // The library half is what the host tests build, and it embeds nothing. An empty array
        // keeps the generated file valid so `cargo test -p uefi_loader` needs no environment.
        None => generated.push_str("pub static KERNEL: &[u8] = &[];\n"),
    }

    match &initrd {
        Some(path) => {
            println!("cargo::rerun-if-changed={path}");
            generated.push_str(&format!(
                "/// The userspace archive, handed over as PVH's single module.\n\
                 pub static INITRD: &[u8] = include_bytes!(r\"{path}\");\n"
            ));
        }
        // An absent archive is a legitimate build, not an error: the kernel's boot tour runs
        // without one and says so, which is the shortest possible bring-up on new hardware.
        None => generated.push_str(
            "/// No archive in this build. The kernel boots its tour and reports no module.\n\
             pub static INITRD: &[u8] = &[];\n",
        ),
    }

    if let (Some(kernel), Some(initrd)) = (&kernel, &initrd) {
        refuse_an_unsealed_pair(kernel, initrd);
    }

    let out = PathBuf::from(env::var("OUT_DIR").expect("cargo sets OUT_DIR")).join("embedded.rs");
    fs::write(&out, generated).expect("the build directory is writable");
}
