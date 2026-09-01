//! **Finding the board's serial port and putting it in the right mode**, with no dependency.
//!
//! # Why `stty` and not a serial crate
//!
//! Configuring a UART is one `tcsetattr` call, and the two ordinary ways to make it from Rust are
//! the `serialport` crate or `libc` plus the `termios` struct by hand. Both are new dependencies in
//! the shipping graph, and `DECISIONS.md` §46 makes taking one a decision rather than a
//! convenience; this is a lane, and a lane does not take that decision. `stty(1)` makes the same
//! call, is in every base system, and has been since the 1970s. The price is a process spawn per
//! session and a diagnostic that is a program's stderr rather than an `errno`, which is a fair
//! trade for a tool that runs once a boot.
//!
//! If this ever needs to *write* to the board with flow control, or set a non-standard baud, the
//! trade changes and the dependency is worth proposing. It does not need either today.
//!
//! # The ordering that is not arbitrary
//!
//! **Open the device first, then run `stty` on it.** Opening a tty that nobody currently holds
//! resets its termios to the driver's initial state, so an `stty` before the open is undone by the
//! open. Holding the descriptor open is also what keeps the settings alive: they revert when the
//! last user closes. Getting this backwards produces a session at the wrong baud that logs
//! plausible-looking garbage, which the runbook's second triage row would then send someone
//! chasing a bad adapter.
//!
//! # And why `cu.` rather than `tty.`
//!
//! On macOS `/dev/tty.*` is the dial-in side and blocks in `open` until carrier detect asserts,
//! which a three-wire console cable never does. That is the classic way a serial tool hangs before
//! printing anything at all, and it hangs in `open`, where no deadline of ours has started yet.
//! `/dev/cu.*` is the call-out side and opens immediately.

use std::ffi::OsStr;
use std::fs::{File, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The bench's baud rate: 115200 8N1, no flow control, from the VisionFive 2 Quick Start Guide by
/// way of `notes/visionfive2.md`'s serial wiring table.
pub const BAUD: u32 = 115_200;

/// The device-name prefixes worth offering, most likely first.
///
/// The rig measured on 2026-09-01 is a WCH CH343 presenting as CDC-ACM, which is why
/// `cu.usbmodem` leads. The path on that Mac is `/dev/cu.usbmodem5C7B0104661` and the digits are
/// the adapter's own serial number, so it is **not** written down here: hard-coding it would work
/// on exactly one machine with exactly one dongle.
#[cfg(target_os = "macos")]
const PREFIXES: &[&str] = &[
    "cu.usbmodem",
    "cu.usbserial",
    "cu.SLAB_USBtoUART",
    "cu.wchusbserial",
];

/// Linux names the same two classes `ttyACM*` (CDC-ACM, which is the CH343) and `ttyUSB*`.
#[cfg(not(target_os = "macos"))]
const PREFIXES: &[&str] = &["ttyACM", "ttyUSB"];

/// Every device in `/dev` that looks like a USB serial adapter, sorted.
///
/// Sorted so that two runs on an unchanged bench pick the same port, which matters when there are
/// two adapters and the caller is a script.
#[must_use]
pub fn candidates() -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir("/dev") else {
        return Vec::new();
    };
    let mut found: Vec<PathBuf> = entries
        .flatten()
        .filter(|entry| {
            entry
                .file_name()
                .to_str()
                .is_some_and(|name| PREFIXES.iter().any(|prefix| name.starts_with(prefix)))
        })
        .map(|entry| entry.path())
        .collect();
    found.sort();
    found
}

/// Pick the port to use: the caller's choice if they made one, otherwise the only candidate.
///
/// Ambiguity is an error rather than a guess. Two adapters on one desk is the normal state of a
/// bench with more than one board, and a tool that silently picked the wrong one would produce a
/// log of somebody else's boot.
///
/// # Errors
///
/// If no candidate exists, or if more than one does and the caller named none. Both messages list
/// what was found and what to pass.
pub fn choose(asked: Option<&Path>) -> io::Result<PathBuf> {
    if let Some(path) = asked {
        return Ok(path.to_path_buf());
    }
    let found = candidates();
    match found.len() {
        1 => Ok(found[0].clone()),
        0 => Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!(
                "no USB serial adapter in /dev (looked for {}). \
                 Is the adapter plugged in? Pass --port to name one.",
                PREFIXES.join("*, ")
            ),
        )),
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "more than one USB serial adapter; pass --port to say which:\n  {}",
                found
                    .iter()
                    .map(|p| p.display().to_string())
                    .collect::<Vec<_>>()
                    .join("\n  ")
            ),
        )),
    }
}

/// Open the port and put it in raw mode at [`BAUD`], in that order.
///
/// Returns the open descriptor and whatever `stty` had to say. A configuration failure is
/// **reported, not fatal**: an adapter whose driver refuses one of these flags still delivers
/// bytes, and a tool that refused to log them would be worse than one that logs them with a
/// warning attached. The read timeout is only ever belt to `watch`'s braces, so losing it costs
/// nothing but a spinning worker thread.
///
/// # Errors
///
/// If the device cannot be opened. That is the one genuinely fatal case, and its usual causes are
/// an adapter that was unplugged, a `/dev/tty.*` path (see this module's header), and another
/// program already holding the port.
pub fn open(path: &Path) -> io::Result<(File, Option<String>)> {
    warn_if_dial_in(path);
    let file = OpenOptions::new().read(true).write(true).open(path)?;
    let complaint = configure(path);
    Ok((file, complaint))
}

/// Run `stty` over an already-open device. Returns its complaint, if it had one.
fn configure(path: &Path) -> Option<String> {
    // `-f` on BSD and macOS, `-F` on GNU. Nothing else differs, which is why this is one flag and
    // not two code paths.
    let flag = if cfg!(target_os = "linux") {
        "-F"
    } else {
        "-f"
    };
    let speed = BAUD.to_string();
    let mut args: Vec<String> = vec![flag.to_string(), path.to_str()?.to_string()];
    args.extend(
        [
            // 115200 8N1, no flow control: the wiring table's line, spelled out.
            speed.as_str(),
            "cs8",
            "-cstopb",
            "-parenb",
            "-crtscts",
            // Raw. Line discipline processing on a boot log turns a firmware progress bar into
            // nonsense and, worse, `icrnl` would rewrite the bare `\r` the recogniser splits on.
            "-icanon",
            "-echo",
            "-icrnl",
            "-ixon",
            "-opost",
            // A read returns after 0.1s with whatever has arrived, including nothing. `watch` does
            // not depend on this (its worker is on its own thread precisely so it need not), but
            // with it the worker parks instead of spinning.
            "min",
            "0",
            "time",
            "1",
        ]
        .iter()
        .map(|s| (*s).to_string()),
    );

    match Command::new("stty").args(&args).output() {
        Ok(out) if out.status.success() => None,
        Ok(out) => Some(format!(
            "stty exited {}: {}",
            out.status,
            String::from_utf8_lossy(&out.stderr).trim()
        )),
        Err(e) => Some(format!("could not run stty: {e}")),
    }
}

fn warn_if_dial_in(path: &Path) {
    if cfg!(target_os = "macos")
        && path
            .file_name()
            .and_then(OsStr::to_str)
            .is_some_and(|name| name.starts_with("tty."))
    {
        eprintln!(
            "board-console: {} is the dial-in device; open() blocks on carrier detect and a \
             three-wire console cable never asserts it. Use the cu.* name instead.",
            path.display()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_explicit_port_is_taken_as_given() {
        let asked = PathBuf::from("/dev/cu.usbmodemDEADBEEF");
        assert_eq!(choose(Some(&asked)).unwrap(), asked);
    }

    /// The message a person actually hits, and the two things it has to tell them.
    #[test]
    fn no_adapter_says_what_it_looked_for_and_what_to_pass() {
        // Only meaningful on a machine with no adapter attached, which is every CI runner and was
        // this lane's own machine; on a bench with one plugged in, `choose` correctly succeeds.
        if !candidates().is_empty() {
            return;
        }
        let e = choose(None).unwrap_err();
        assert_eq!(e.kind(), io::ErrorKind::NotFound);
        assert!(e.to_string().contains("--port"));
    }

    #[test]
    fn candidates_are_sorted_so_two_runs_agree() {
        let found = candidates();
        let mut sorted = found.clone();
        sorted.sort();
        assert_eq!(found, sorted);
    }
}
