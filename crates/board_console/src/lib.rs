//! **A serial console for a board this repository cannot see** (milestone 216).
//!
//! `script/console` opens a shell in QEMU. `script/board-image` builds the payload a VisionFive 2
//! boots and prints the `dd` commands for a card. Between those two there was nothing, so every
//! milestone gated on real hardware waited on a person at a terminal emulator, reading with their
//! eyes. This crate is the missing middle: **open the port, log every byte, recognise how far the
//! boot got, and stop on a deadline.**
//!
//! The three parts, in the order a session uses them:
//!
//! - [`port`] finds the adapter and puts it in raw mode at 115200 8N1.
//! - [`watch`] reads until the policy says stop, teeing every byte to a log file.
//! - [`progress`] decides, from the text alone, how far the boot got.
//!
//! # Examples
//!
//! Watch a board boot, with a two-minute cap:
//!
//! ```no_run
//! use std::fs::File;
//! use board_console::{port, watch::{self, Policy}};
//!
//! let path = port::choose(None)?;
//! let (device, complaint) = port::open(&path)?;
//! if let Some(complaint) = complaint {
//!     eprintln!("board-console: {complaint}");
//! }
//! let mut log = File::create("target/board-console.log")?;
//! let session = watch::watch(device, &mut log, &Policy::default(), true)?;
//! println!("{}", session.summary());
//! # Ok::<(), std::io::Error>(())
//! ```
//!
//! Replay a log that was already captured, which is how a session is re-read after the fact and
//! how the recogniser is exercised with no board present:
//!
//! ```no_run
//! use std::fs::File;
//! use board_console::watch::{self, Policy};
//!
//! let captured = File::open("target/board-console.log")?;
//! let mut nowhere = std::io::sink();
//! // `false`: end of file means the log is over, not that the board went quiet.
//! let session = watch::watch(captured, &mut nowhere, &Policy::default(), false)?;
//! println!("{}", session.summary());
//! # Ok::<(), std::io::Error>(())
//! ```
//!
//! # BUGS
//!
//! **The markers are checked against one board, on one day, in two states.** calef captured a
//! successful boot and an extlinux failure on 2026-09-01; both are in
//! `tests/fixtures/captured/` and both are asserted on, which is what turned "quoted from
//! documentation" into "printed by a machine". What that does not cover is every other way this
//! board can behave and every other board: the two synthetic fixtures are cases nobody has yet
//! seen at a bench, a different vendor firmware build could word its banners differently, and the
//! aarch64 and `x86_64` boards have not been looked at at all. A marker whose real text differs by
//! a word is missed, and a missed marker reports a healthy board as having got less far than it
//! did.
//!
//! **A missed marker fails toward pessimism, which is the safe direction, and a *matched* one does
//! not.** The recogniser matches substrings anywhere in a line, so a board that echoes the word
//! `Starting kernel ...` back at a U-Boot prompt would be read as having handed over. Nothing
//! guards against a console that repeats its input, and this tool never writes to the port, so
//! today the only way that happens is a person typing into the same session.
//!
//! **It does not recognise an OpenSBI trap dump**, which the failure-triage ladder lists as a real
//! and specific outcome (the kernel started the S7 and the vendor firmware died in its own
//! handler). The dump's exact first line is not written down anywhere in this tree, and guessing
//! it would put text in a recogniser that no machine has ever printed. Such a boot is caught as
//! silence or as time running out, with the dump in the log, which is one step worse than being
//! named.
//!
//! **A captured log is not a test result**, and the roadmap block says so first. This crate
//! reports how far a boot got; deciding that a milestone passed on the strength of a vendor's boot
//! message is a line nobody has yet agreed to cross, and `Outcome::Reached` is deliberately named
//! for what it observed rather than for a verdict.
//!
//! **It reads and never writes, and on this board that is not enough to boot.** The capture proves
//! it: the extlinux path ends at `### ERROR ### Please RESET the board ###`, so reaching nife
//! requires interrupting autoboot and typing the four `StarFive #` commands
//! `script/board-image` prints. A console that only reads therefore cannot, on its own, get this
//! board to the state the hardware-gated milestones need. Driving U-Boot is proven to work (calef
//! did it on 2026-09-01) and is deliberately not here, because whether that is this tool or a
//! second one is a scope decision that belongs to calef; see the roadmap block.
//!
//! **It does not touch power.** The board is powered by a Kasa strip that was not reachable from
//! either machine when milestone 216 was written, and the roadmap block declines to decide whether
//! this tool should ever drive it. A tool that power-cycles is a different and more dangerous
//! object than one that reads.
//!
//! **One board's vocabulary.** The stages are the VisionFive 2's boot chain. An aarch64 or `x86_64`
//! board would want the same shape with different banners, and whether that is one tool with a
//! profile or three tools is an open design question the roadmap block names and does not answer.

//! # Name
//!
//! Name: provisional, this lane's coinage (2026-09-01, milestone 216). Two nouns for what it is,
//! a console for a board, in the `snake_case` the crate rule asks for. `serial` was refused as a
//! generic word that names a wire rather than this thing, and because the tree already spends
//! `console` on the kernel's own (`kernel/src/console.rs`) and on `script/console`, which makes
//! the qualifier the load-bearing half. `board_serial` was refused for the same reason in
//! reverse: it names the transport, and the transport is the least interesting part. `bench` was
//! refused as a place rather than a thing.
//!
//! **The open question is not the word, it is the scope**, and milestone 216's block names it:
//! whether this stays one crate that learns a board profile, or becomes one per board. A name
//! chosen before that is answered is a name that may be answering it by accident. Not put to
//! calef.

pub mod port;
pub mod progress;
pub mod watch;
