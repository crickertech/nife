//! **`printenv`**: print the inert-configuration page (milestone 47's environment-variable fork,
//! DECISIONS §111, notes/env-config.md).
//!
//! The whole program is: read a page, print what it carries. It holds two capabilities and
//! neither of them can change anything, exactly `date`'s shape one manifest field over
//! ([`grant_plan::Manifest::config`] rather than [`grant_plan::Manifest::clock`]).
//!
//! # It reads. There is no way to set anything from here.
//!
//! The page this program maps is `READ`-only, and that rung is the whole of why there is no
//! `printenv -s TZ=...` here: the authority this process lacks is a page permission rather than a
//! check somewhere in this file. Nothing about what a shell hands its own children can be changed
//! by a program spawned with a config grant, which is the same shape `date` already demonstrates
//! for the clock.
//!
//! # Three keys, three closed domains, and a state the type carries honestly
//!
//! `environment_proto::ConfigPage` answers `Option<&str>` per key: `Some(value)` when the page is
//! valid and carries that key, `None` when the page is valid but that key was never set, and
//! (indistinguishably from the outside, by design) `None` when nobody assembled a page into this
//! frame at all. This program tells the second case apart from the first two the same way `date`
//! tells "no clock service" from "no clock capability": by checking whether it holds the
//! capability at all before it prints anything about what the page says.
//!
//! # Arguments: none
//!
//! There is no argv on this ABI (notes/abi.md), and this program needs none: `printenv` with no
//! arguments is the only spelling, matching Unix's own `printenv(1)`/`env(1)` with nothing to
//! filter by.
//!
//! # EXAMPLES
//!
//! ```text
//! printenv                  TZ=UTC
//!                            LANG=C
//!                            TERM=dumb
//! printenv  (a key unset)   TZ=UTC
//!                            LANG (unset)
//!                            TERM=dumb
//! printenv  (no grant)      printenv: no configuration was granted
//! ```
//!
//! # BUGS
//!
//! - **No filtering.** Unix's `printenv NAME` prints one variable; this always prints all three,
//!   because `ArgSpec` has no position yet (the same gap `date`'s module docs name for a format
//!   selector) and three lines cost nothing to read in full.
//! - **The three keys are hardcoded.** Nothing here enumerates the page; a fourth validated domain
//!   would need a fourth line written by hand. `environment_proto::PAGE_BYTES` is small and fixed,
//!   so this is a property of the contract rather than an oversight, but a reader expecting a
//!   generic env dump should know the shape is closed.
//!
//! Name: provisional, introduced 2026-08-26 alongside `grant_plan::Manifest::config`. Unix's
//! own name for exactly this (`printenv(1)`), a term of art already right per this tree's own
//! naming convention for standard terms; unrated by calef.

#![no_std]
// Program entry points, not the crates/ library surface milestone 68's ratchet tracks
// (DECISIONS §107): each `[[bin]]` is its own crate root with one `_start`, and documenting an
// OS-facing ABI entry point is not what the lint is for.
#![allow(missing_docs)]
#![no_main]

use environment_proto::ConfigPage;
use user_rt::{exit, invoke, send};

/// Slot 0: where the output goes. An endpoint with `WRITE`, and the same 16-bytes-per-message
/// framing the std PAL's stdout uses (`w0` = the byte count, `w1`|`w2` = the bytes, little-endian).
const REPORT: u64 = 0;

/// Slot 1: the config page's `PageFrame` capability, with `READ` and nothing else. Its *presence*
/// is what [`config_page`] probes for; its rights are what stop this program setting anything.
const CONFIG_SLOT: u64 = 1;

/// Where the wiring maps the config page, read-only. Must match
/// `crates/system_initializer`'s `CHILD_CONFIG_VA`.
const CONFIG_VA: u64 = 0x00e0_0000;

#[unsafe(no_mangle)]
pub extern "C" fn _start(_a0: u64, _a1: u64, _a2: u64) -> ! {
    let Some(page) = config_page() else {
        line(b"printenv: no configuration was granted");
        send(REPORT, byte_sink_proto::eof(), 0, 0);
        exit();
    };

    key_line(&page, b"TZ", page.tz());
    key_line(&page, b"LANG", page.lang());
    key_line(&page, b"TERM", page.term());

    send(REPORT, byte_sink_proto::eof(), 0, 0);
    exit();
}

/// One `KEY=value` line when the domain holds a value, `KEY (unset)` when the page is valid and
/// simply does not carry it. `_page` is unused directly but keeps this a method-shaped call at
/// every site, so a reader sees the three lines are the same operation three times rather than
/// three hand-written formats that happen to agree today.
fn key_line(_page: &ConfigPage, key: &[u8], value: Option<&str>) {
    let mut buf = [0u8; 96];
    let mut n = 0;
    push(&mut buf, &mut n, key);
    match value {
        Some(v) => {
            push(&mut buf, &mut n, b"=");
            push(&mut buf, &mut n, v.as_bytes());
        }
        None => push(&mut buf, &mut n, b" (unset)"),
    }
    line(&buf[..n]);
}

fn push(buf: &mut [u8; 96], n: &mut usize, bytes: &[u8]) {
    for &b in bytes {
        if *n < buf.len() {
            buf[*n] = b;
            *n += 1;
        }
    }
}

/// Whether a capability is in `slot`, without touching whatever it names.
///
/// [`config_page`]'s probe, `date`'s `granted()` lifted verbatim: invoke a method number no object
/// type defines, so the call can only be refused, and read *which* refusal came back. An empty
/// slot answers `NoSuchSlot`; a real object answers `BadMethod`, which is a refusal from something
/// that exists.
fn granted(slot: u64) -> bool {
    /// A method number no object type defines, so the invocation can only ever be refused.
    const NO_SUCH_METHOD: u64 = 0xffff;
    // SAFETY: a syscall that cannot succeed; the kernel validates the slot before the method.
    let r = unsafe { invoke(slot, NO_SUCH_METHOD, 0, 0, 0) };
    r != abi::Error::NoSuchSlot as i64
}

/// The config page, or `None` when this process was granted no configuration capability at all.
///
/// The probe has to answer **without touching the page**, for `date`'s reason (a different binary,
/// the same shape): a process granted nothing has nothing mapped at [`CONFIG_VA`], and a read
/// there would fault instead of answering.
fn config_page() -> Option<ConfigPage> {
    if !granted(CONFIG_SLOT) {
        return None;
    }
    // SAFETY: the wiring maps the config page read-only at CONFIG_VA alongside the capability the
    // probe just found, and nothing unmaps it. Without the capability we never build the pointer.
    Some(unsafe { ConfigPage::new(CONFIG_VA) })
}

/// Send `bytes` and a newline to the output endpoint, 16 bytes per message. `date`'s `line`/
/// `line_on` collapsed into one, because this program has only the one destination.
fn line(bytes: &[u8]) {
    let mut out = [0u8; 96];
    let n = bytes.len().min(out.len() - 1);
    out[..n].copy_from_slice(&bytes[..n]);
    out[n] = b'\n';
    for chunk in out[..n + 1].chunks(16) {
        let mut w1 = 0u64;
        let mut w2 = 0u64;
        for (i, &b) in chunk.iter().enumerate() {
            if i < 8 {
                w1 |= (b as u64) << (8 * i);
            } else {
                w2 |= (b as u64) << (8 * (i - 8));
            }
        }
        send(REPORT, chunk.len() as u64, w1, w2);
    }
}

user_rt::panic_handler!();
