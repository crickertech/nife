//! The same linker contract as `std_exerciser`, with one change that `scripts/build-ripgrep.sh`
//! already had to make and recorded as a finding rather than a workaround.
//!
//! `crates/user_mode_runtime/link.ld` puts every program at `0x40_0000` and `kernel/src/user.rs` puts every
//! program's stack at `0x50_0000`, so an image has under 896 KiB of address space before it
//! collides with its own stack. A `rustls` provider and its RustCrypto primitives do not fit in
//! that, any more than ripgrep's 1.37 MiB of `.text` did, and the loader's refusal
//! (`Unmappable(AlreadyMapped)`) names the symptom rather than the cause. So this relinks at 16 MiB.
//!
//! The script is **derived by substitution rather than copied**, for the reason that script gives:
//! two copies of a linker script drift and nothing notices. If the base address in
//! `crates/user_mode_runtime/link.ld` ever moves, the `grep` below fails the build loudly instead of
//! silently producing a program at an address nothing agrees on.

use std::{env, fs, path::PathBuf};

fn main() {
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let shared = manifest.join("../crates/user_mode_runtime/link.ld");
    println!("cargo::rerun-if-changed=../crates/user_mode_runtime/link.ld");

    let text = fs::read_to_string(&shared).expect("cannot read the shared linker script");
    let high = text.replace("    . = 0x400000;", "    . = 0x1000000;");
    assert!(
        high.contains("0x1000000"),
        "crates/user_mode_runtime/link.ld no longer sets 0x400000 where this build script expects it; \
         see scripts/build-ripgrep.sh, which makes the same substitution"
    );

    let out = PathBuf::from(env::var("OUT_DIR").unwrap()).join("link-high.ld");
    fs::write(&out, high).expect("cannot write the relinked script");

    println!("cargo::rustc-link-arg=-T{}", out.display());
    println!("cargo::rustc-link-arg=-u_start");
    println!("cargo::rustc-link-arg=--build-id=none");
}
