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
//! And one part that reads a *screen* rather than a wire (milestone 243), because six of the
//! machines this project wants to boot have no serial port at all:
//!
//! - [`screen`] turns a screendump of nife's framebuffer console back into the text that was drawn
//!   into it, so that [`progress`] judges a monitor exactly as it judges a cable.
//!
//! And one part that reads a capture after the fact rather than a board in front of it:
//!
//! - [`lottery`] takes a log of *many* boots and reports what the thread-placement lottery drew
//!   each time (milestone 249), which is the question a self-rebooting soak exists to answer and
//!   the one nothing that reads a single boot can be asked.
//!
//! # This crate is not run under Miri
//!
//! `script/undefined-behavior-check` excludes it, the way it already excludes `xtask`, and for cost
//! rather than principle (milestone 238). Measured 2026-09-03: **3,307 seconds, 55 minutes, for the
//! lib tests alone** under the interpreter, against roughly four minutes for the whole rest of the
//! workspace. There is no `unsafe` and no dependency anywhere in here, so the rules Miri enforces
//! (aliasing, provenance, uninitialized reads) have nothing in this call graph to be broken by.
//! (Milestone 243 gave it one dependency, `bitmap_font`, which is ours, has none of its own, and
//! contains no `unsafe` either.)
//!
//! Ten of the tests could not run there in any case, and both families are worth knowing before
//! anyone points Miri at this crate by hand. Five in [`port`] reach the host filesystem (`open`,
//! reading `/dev`, the temp directory) and Miri's isolation refuses them. Five in [`watch`] are
//! **wall-clock driven**: the policy's 15-second quiet timeout and 120-second budget are measured
//! with `Instant`, so under an interpreter they expire while the replay is still feeding bytes and
//! the watcher reports `Reached(Banner)` where a real run reaches `Reached(Tour)`. That is Miri
//! being slow, not this crate being wrong, and it is the same category as `cred`'s timing test.
//! See notes/undefined-behavior.md and the exclusion list in `xtask`.
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
//! **The markers are checked against one board, on one day, in four states.** calef captured two
//! successful boots, an extlinux refusal and a measured-boot refusal on 2026-09-01; all four are
//! in `tests/fixtures/captured/` and all four are asserted on, which is what turned "quoted from
//! documentation" into "printed by a machine". What that does not cover is every other way this
//! board can behave and every other board: a different vendor firmware build could word its
//! banners differently, and the aarch64 and `x86_64` boards have not been looked at at all. A
//! marker whose real text differs by a word is missed, and a missed marker reports a healthy board
//! as having got less far than it did.
//!
//! **There is no real sample of a hang**, which is the outcome this tool exists for, since a hang
//! is what a multicore defect looks like from the far end of a serial cable. The synthetic fixture
//! is a real capture truncated before the tour. A genuine one, if risk 5 ever produces one, would
//! be worth more than every other fixture here.
//!
//! **The settle window is two seconds, and two seconds is a guess.** It is what stops a failure
//! printed just after the awaited stage being reported as a success, which the captured
//! measured-boot refusal is: it prints the banner and most of a tour before halting. A failure
//! announced three seconds later would still slip through. `--until none` has no early exit at all
//! and is the answer when being right matters more than being quick.
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
//! **No test opens a real serial device.** A pseudo-terminal pair would be the honest stand-in,
//! and making one needs `posix_openpt` and its `ioctl`s, which is `libc`, which is §46's decision
//! (thin primitives or whole subsystems; we write everything in between) and not a lane's. So the
//! port layer is tested as pure logic plus a regular file, and the single claim that leaves
//! unproven is that `tcsetattr` actually took. That was checked by hand against the CH343 and is
//! gated at runtime by the speed read-back, which runs on every real session rather than only when
//! somebody runs the tests.
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

pub mod lottery;
pub mod port;
pub mod progress;
pub mod screen;
pub mod watch;
