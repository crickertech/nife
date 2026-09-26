//! **A program no boot image carries** (milestone 198 (a package manager) rung 3a).
//!
//! It prints one line and exits. What makes it worth a file is where it comes from: `fixtures`
//! lists it in `packaged_only`, so no archive packs it (`script/swish-check` reads the archive to
//! make sure), and the only way onto a machine is the
//! package `packages/greeting.recipe` builds. So when it runs at the prompt, the image's catalogue,
//! the fetch, the digest check and the activation set are the only things that could have put it
//! there. Every earlier install line ran a copy of the image's own `uptime`, which proved the path
//! and not novelty.
//!
//! ```text
//! $ package install greeting
//!   fetched and installed; generation 2 is live
//! $ packages/greeting/0.1.0/greeting
//! hello from a package this image never carried
//! ```
//!
//! # What this program holds
//!
//! Slot 0, its output: the sink contract (`crates/byte_sink_protocol`). An installed program is
//! endowed `grant_plan::INSTALLED_MANIFEST_OF`, which is `uptime`'s manifest, and this one needs
//! nothing more.
//!
//! # BUGS
//!
//! It proves the path and nothing about the program: a line of text is the least a program can do.
//! A real tool the image lacks, with its own manifest, waits on DECISIONS §197 (a package is one
//! archive file) saying where a manifest travels.
//!
//! Name: provisional (2026-09-26). Says what it prints; GNU `hello`, which exists to demonstrate
//! packaging, would be the prior art, and `hello` is already a fixture here.

#![no_std]
#![allow(missing_docs)]
#![no_main]

use user_mode_runtime::{exit, send};

/// Slot 0: where the line goes.
const OUT: u64 = 0;

/// The line. The gate asserts it, so a copy of any program the image carries could not print it.
const LINE: &[u8] = b"hello from a package this image never carried\n";

#[unsafe(no_mangle)]
pub extern "C" fn _start(_a0: u64, _a1: u64, _a2: u64) -> ! {
    let mut rest = LINE;
    while !rest.is_empty() {
        let (w0, w1, w2, n) = byte_sink_protocol::pack(rest);
        send(OUT, w0, w1, w2);
        rest = &rest[n..];
    }
    send(OUT, byte_sink_protocol::eof(), 0, 0);
    exit();
}

user_mode_runtime::panic_handler!();
