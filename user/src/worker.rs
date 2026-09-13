//! The worker: a whole program in one job.
//!
//! Milestone 19f.2, the first program that is its **own binary** rather than a role of `hello`.
//! init loads it out of the initrd archive by the name `"worker"` (nifefs), builds a child
//! address space and TCB at this ELF's own entry, and `START`s it with the input `n` in `x1`. The
//! worker squares `n`, `SEND`s the answer on the one endpoint init granted it (slot 0), and exits.
//! That is the entire program: no role byte to dispatch on, no capabilities beyond the single one
//! it needs. Least authority made real, because the program *is* its authority.
//!
//! It shares the `user` crate's `link.ld` (linked at `0x40_0000`, in its own address space, so the
//! shared load address is not a conflict) but not one line of hello's code: a distinct ELF with its
//! own `_start` and panic handler. The syscall runtime (`send`/`exit`) comes from the shared
//! `user_rt` crate, lifted out at 19f.6 once all the split binaries existed.
//!
//! Name: provisional, and ruled: calef ruled **`least_authority_demo`** on 2026-09-13, working the
//! unratified worklist. This block used to leave the name open because the answer depended on
//! whether this file is a fixture or the canonical minimal program; **calef ruled it the canonical
//! minimal program**, which also settles a classification three records had made by repetition
//! rather than by ruling (`notes/naming.md`, milestones 39 and 175 all list it as a fixture).
//!
//! `worker` was a **generic word** of the kind AGENTS.md flags beside `compose` and `measure`: in a
//! kernel tree it could name a thread-pool member, a scheduler entity or a job. Refused
//! `demo_square` (the header below says the squaring is arbitrary and the authority is the point, so
//! naming it after the arithmetic drops the point; `demo` is also a generic word with no precedent
//! in this tree, and `square` is a verb where the noun rule wants a thing), `squarer` (a noun and
//! honest, but it is the fixture reading this ruling rejected) and `least_authority` (the same name
//! one word shorter; calef took the longer form, which says it is a demonstration rather than a
//! claim about the program's own authority).
//!
//! **Still provisional because the rename has not happened.** Milestone 175 moves this file, and
//! into `components/` rather than `fixtures/` now that the classification is ruled. The sweep is
//! the other reason it waits: 99 files match `worker`, and `crates/board_console`'s soak-census uses
//! the English word. `README.md`'s `worker 7` is the command and does move.

#![no_std]
// Program entry points, not the crates/ library surface milestone 68's ratchet tracks
// (DECISIONS §107): each `[[bin]]` is its own crate root with one `_start`, and 58 of them
// documenting an OS-facing ABI entry point is not what the lint is for.
#![allow(missing_docs)]
#![no_main]

use user_rt::{exit, send};

/// The endpoint init grants the worker as its only capability (slot 0). Its one `SEND` goes here,
/// straight to whoever is waiting (the kernel test, or the shell behind init's spawn service).
const RESULT: u64 = 0;

/// The worker's entry. `START` (milestone 19e) hands it three registers: `x0` is unused here (a
/// standalone binary needs no role selector), the input `n` arrives in `x1`, `x2` is unused.
#[unsafe(no_mangle)]
pub extern "C" fn _start(_x0: u64, n: u64, _x2: u64) -> ! {
    let result = n.wrapping_mul(n);
    send(RESULT, result, 0, 0);
    // Exit rather than spin: this is a whole process lifecycle (spawn, run, report, exit). The
    // kernel reaps the thread and frees the address space.
    exit();
}

user_rt::panic_handler!();
