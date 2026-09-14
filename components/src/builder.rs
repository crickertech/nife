//! **A minimal first process: the system builder** (milestone 20, the richer-initrd step).
//!
//! This is the RISC-V counterpart of the init role in `hello` (which is aarch64-wired: PL011,
//! `svc`, the shell/console/input system). It is deliberately small and fully portable: it does the
//! one thing that matters for the demonstrator's thesis on a second ISA, which is that **userspace,
//! not the kernel, composes the system.**
//!
//! The kernel loads this program from the initrd's `builder` entry, maps the whole nifefs archive
//! read-only, and grants it two capabilities: a large untyped budget (slot 0) and a report endpoint
//! (slot 1). From those, and nothing else, this program:
//!
//! 1. parses the archive (nifefs) and reads the `least_authority_demo` program out of it by name,
//! 2. parses that ELF (the `elf` crate, linked into userspace),
//! 3. builds a child process out of its own budget through the granular capability verbs (retype an
//!    address space, copy each segment into retyped frames and map them, retype a TCB, endow it,
//!    configure, start), wiring the child's one endpoint to the report line, and
//! 4. starts the child with an input.
//!
//! The child (the `least_authority_demo`) squares the input and SENDs the answer straight to the report endpoint,
//! which the kernel is waiting on. The kernel never touches the `least_authority_demo`'s bytes: this program loaded
//! it, built its address space, and started it. That is the userspace-as-system-builder model, proven on
//! RISC-V. It shares the `user` crate's `link.ld` and the `user_mode_runtime` syscall runtime; every syscall
//! it makes (retype, map, configure, start) crosses the same `ecall` ABI the `least_authority_demo` uses.
//!
//! **Why this is still here in a world that boots to `swish`** (milestone 289, which was sent to
//! find out whether it was vestigial and found that it is not). The suspicion was fair: on aarch64
//! the machine hands itself to `progenitor`, and on RISC-V `riscv_shell_boot` does the same thing
//! harder, building the console server, the input driver, the line discipline, the shell, the
//! terminal sink and the job undertaker out of one budget. If that ran where this runs, this file
//! would be redundant and milestone 289 would have deleted it.
//!
//! It does not run where this runs. `riscv_shell_boot` is `#[cfg(feature = "shell")]`, and the only
//! thing that builds that kernel is `script/shell-check`, in QEMU. The **default** riscv64 build is
//! the boot tour, which is what `script/board-image` writes to a card, what `script/soak --arch
//! riscv64` and `script/job-mix --arch riscv64` boot before their workloads, and the only riscv64
//! build anything here produces a board payload from. So this program is the only demonstration on
//! that ISA, outside a QEMU gate, that userspace and not the kernel composes the system.
//!
//! **And the trimming is why it can be.** `riscv_shell_boot` needs the PLIC initialised, the
//! NS16550's registers delegated as a device frame, and the UART's interrupt routed, and that last
//! one is board-specific: source 10 on QEMU `virt`, 32 on the JH7110 (notes/visionfive2.md, BUGS).
//! This program takes a budget and a report endpoint. On a board that separates "the capability core
//! can compose a process" from "this machine's interrupt wiring is right", and the tour's own
//! history is that the second is the half that breaks.
//!
//! **One printed line is a machine-readable signal.** The kernel's `init/build` line
//! (kernel/src/main.rs) is matched by substring in `crates/board_console`'s
//! `Progress::userspace_ran`, asserted by four host tests, and present in three captured transcripts,
//! one of them taken off the VisionFive 2 itself (`vf2-2026-09-01-userspace.log`, named for the fact
//! that this program ran). notes/board-console.md calls it the only difference between the two
//! successful board captures.
//!
//! **And it was the evidence that resolved a false alarm.** Three of the five identifications that overturned the VisionFive 2 "hang" (notes/visionfive2.md,
//! fifth stop) are facts about this file: its exact syscall count (14 of that boot's 20 ecalls), the
//! object kinds it retypes (ASPACE, FRAME and TCB, never ENDPOINT), and that it issues no receive of
//! any kind.
//!
//! **Its category is unsettled and is calef's.** Milestones 39 and 175 both listed it as a
//! `fixtures/` example, and 175 is explicit that those lists classified it "by repetition rather
//! than by ruling"; it sits in `components/` today. The recommendation from 289 is that it stays
//! there, on the ground that the kernel measures it against the trust root and enters it on the
//! **default** riscv64 boot (`kernel/src/user.rs`, `trust::require("builder", ...)`), which is what
//! it does for the first process and for nothing else. The objection to that argument is worth
//! knowing: `boot_programs()` also names `hello`, which lives in `fixtures/`. `xtask`'s doc says
//! why, and the reason is the distinction: `hello` is measured because `spawn_progenitor` enters it
//! for milestone 19d's **test roles** and `trust::require` refuses an unnamed entry, so the trust
//! root grew rather than the check shrinking. This program is entered on an ordinary boot.
//! `notes/trusted-init.md` groups it with the demo loaders, correctly, about a different question:
//! which loaders extend the measurement chain.
//!
//! # BUGS
//!
//! **Nothing that runs on a pull request executes this program.** `script/test`'s riscv64 leg and
//! `script/cpu-matrix` both boot the `#[cfg(test)]` kernel, which runs its suite and exits by
//! semihosting before the tour block; `script/shell-check` boots `--features shell`;
//! `script/bench --riscv --check` and `script/icount` each park before the tour. The callers that do
//! reach it are `script/soak --arch riscv64`, `script/job-mix --arch riscv64` and a board, and none
//! of those runs on a branch. So this step is not unused, it is **unasserted**, and from a grep the
//! two look the same: that is what got milestone 289 minted as a retirement. Proposed as
//! design/roadmap/proposals/nothing-in-ci-boots-the-riscv-tour.md.
//!
//! **The child is loaded unmeasured.** The kernel measures this program against the trust root
//! (kernel/src/trust.rs) and then this program reads `least_authority_demo` out of the archive and
//! builds it without consulting `measured_boot::PROGRAM_MEASUREMENTS`, which the archive already
//! carries and which `progenitor` does consult. notes/trusted-init.md's "Still not covered" lists
//! this as one of three such loaders and prices the remaining work as the call rather than the data;
//! what that note does not say, and what is worth knowing here, is that on the board path this
//! program is the first process, so the gap is on the shipped boot for that ISA and not only on a
//! demo.
//!
//! **It builds exactly one child, of one hardcoded name, with one hardcoded input.** There is no
//! argument surface and there is not meant to be: the proof is that the verbs compose a process, not
//! that this program is configurable. A second child, a different demo, or a failure injected partway
//! through the build are all things it cannot express, and a reader looking for a general userspace
//! loader wants `supervision_protocol::build_child` and `progenitor`.
//!
//! Name: recorded (crate `system_initializer`, ratified 2026-08-04 by calef, and milestone 63's
//! name table before it). Never argued for directly and argued around twice, which is stronger
//! than it sounds. `builder.rs`'s own first line called it "a minimal init: the system builder"
//! when those refusals were recorded, and that phrase is why `system_builder` was turned down for a
//! crate on 2026-08-01 and again on 2026-08-04, both times to stop two programs claiming one
//! phrase. A name the tree has twice declined to give away is a name the tree has reasoned about.
//! Its archive entry is `builder` since milestone 266, so it is no longer the exception to "the
//! binary, the source file and the archive entry are the same string" (notes/naming.md) that it was
//! while the kernel loaded it under the entry `init`. calef has not ratified it.
//! Proposed provisionally by milestone 289's lane (2026-09-14), since the category above has to be
//! settled first and this is the half a reader meets: `process_builder`, a noun for the thing it
//! makes, which is one process and not a system. That is what the program actually does, and it is
//! the claim `system_builder` overstated in both refusals: the system on this ISA is `progenitor`'s
//! to build, and this builds one child out of one budget. Refused: `least_authority_builder` (it
//! names the child rather than the act, and the child already carries that name);
//! `riscv_process_builder` (the program is architecture-neutral and compiles for aarch64 too, so an
//! ISA in the name would record where it is used rather than what it is); `boot_builder` (generic on
//! the second word and wrong on the first, since this is not the boot program on two of three
//! architectures). `builder` itself is the failure mode AGENTS.md names first, a generic word that
//! could label almost anything in an operating system, and `NAME_LEN = 32` leaves room.

#![no_std]
// Program entry points, not the crates/ library surface milestone 68's ratchet tracks
// (DECISIONS §107): each `[[bin]]` is its own crate root with one `_start`, and 58 of them
// documenting an OS-facing ABI entry point is not what the lint is for.
#![allow(missing_docs)]
#![no_main]

use user_mode_runtime::{
    cap_delete, exit, map_into, map_page_frame, send, tcb_cap_insert, tcb_configure, tcb_start,
};

/// The capabilities the kernel grants this program before it runs.
const MEMORY_REGION: u64 = 0; // a budget to retype the child's address space, frames, and TCB from
const REPORT: u64 = 1; // the report endpoint; we hand the child a narrowed WRITE view of it

/// The input we hand the `least_authority_demo`. It returns `n * n` on the report endpoint.
const INPUT: u64 = 9;

/// The child's layout, in its own address space (it links at `0x40_0000`, like every user program).
const PAGE: u64 = 4096;
const CHILD_STACK_TOP: u64 = 0x0050_0000; // one page of stack is plenty for the least_authority_demo

#[unsafe(no_mangle)]
pub extern "C" fn _start(_x0: u64, initrd_len: u64, _x2: u64) -> ! {
    // The archive the kernel mapped read-only at INITRD_VA; its length arrived in a1.
    // SAFETY: forwarded from user_mode_runtime::initrd::initrd_bytes's own contract.
    let archive = unsafe { user_mode_runtime::initrd::initrd_bytes(initrd_len) };

    let Ok(fs) = nifefs::Fs::parse(archive) else {
        fail(0xE1);
    };
    let Some(demo_bytes) = fs.read("least_authority_demo") else {
        fail(0xE2);
    };
    let Ok(elf) = elf::Elf::parse(demo_bytes) else {
        fail(0xE3);
    };

    // Build the least_authority_demo and start it with INPUT. It SENDs its answer straight to REPORT (which we
    // grant it, narrowed to WRITE, as its slot 0), and the kernel receives it. We built the pipe.
    if build_and_start(&elf, INPUT).is_err() {
        fail(0xE4);
    }

    // Our job is done: the least_authority_demo runs on its own. Leave, and the kernel reaps us.
    exit();
}

/// Build a child process from `elf` out of our untyped budget, grant it the report endpoint as its
/// slot 0, and start it with `n` in its second argument register. Returns `Err(())` on any failure.
///
/// This is a userspace ELF loader, the same shape as the kernel's `map_segments` but driven entirely
/// through the capability verbs: nothing here is privileged, it is all `invoke` on capabilities we
/// hold. It mirrors `hello`'s `build_child`, trimmed to exactly what this one child needs.
fn build_and_start(elf: &elf::Elf, n: u64) -> Result<(), ()> {
    // A fresh address space, retyped from our budget.
    let aspace = retype_obj(abi::objtype::ADDRESS_SPACE)?;

    // Lay each loadable segment into the child at the VA it names, with its own permissions. Each
    // page: retype a frame, map it in our OWN space to fill it, copy this page's slice of the
    // segment, then map it into the child. SCRATCH advances so we never reuse a filled VA.
    let mut scratch = 0x1000_0000u64;
    for seg in elf.segments() {
        let mode = if seg.is_executable() {
            abi::address_space::MAP_CODE
        } else if seg.is_writable() {
            abi::address_space::MAP_RW
        } else {
            abi::address_space::MAP_RO
        };
        let (start, end) = seg.page_range(PAGE);
        let mut va = start;
        while va < end {
            let frame = retype_page_frame()?;
            // Map the frame writable in our own space at `scratch`, fill it, then hand it over.
            if !map_page_frame(frame, scratch, true, MEMORY_REGION) {
                return Err(());
            }
            // SAFETY: `scratch` is a page we just mapped read/write in our own space.
            let dst = unsafe { core::slice::from_raw_parts_mut(scratch as *mut u8, PAGE as usize) };
            dst.fill(0); // zero first, so a segment's .bss tail is clean
            let file_lo = seg.vaddr;
            let file_hi = seg.vaddr + seg.data.len() as u64;
            let lo = va.max(file_lo);
            let hi = (va + PAGE).min(file_hi);
            if lo < hi {
                let d = (lo - va) as usize;
                let s = (lo - file_lo) as usize;
                let len = (hi - lo) as usize;
                dst[d..d + len].copy_from_slice(&seg.data[s..s + len]);
            }
            // Into the child at the segment's own VA, with the segment's permissions.
            if map_into(aspace, va, frame, mode) != 0 {
                return Err(());
            }
            cap_delete(frame); // done with this frame's cap; free the slot for reuse
            scratch += PAGE;
            va += PAGE;
        }
    }

    // A one-page stack for the child, mapped just below CHILD_STACK_TOP.
    let stack_frame = retype_page_frame()?;
    if map_into(
        aspace,
        CHILD_STACK_TOP - PAGE,
        stack_frame,
        abi::address_space::MAP_RW,
    ) != 0
    {
        return Err(());
    }
    cap_delete(stack_frame);

    // The thread, its one capability (the report endpoint, narrowed to WRITE, lands as slot 0),
    // configured at the ELF's entry with the stack top, then started with `n` in its second arg.
    let tcb = retype_obj(abi::objtype::THREAD_CONTROL_BLOCK)?;
    if tcb_cap_insert(tcb, REPORT, abi::rights::WRITE, 0) < 0 {
        return Err(());
    }
    if tcb_configure(tcb, elf.entry(), CHILD_STACK_TOP, aspace) != 0 {
        return Err(());
    }
    // START's arguments become the child's a0/a1/a2; the least_authority_demo reads its input from a1.
    if tcb_start(tcb, 0, n, 0) != 0 {
        return Err(());
    }
    cap_delete(tcb); // our TCB cap; the least_authority_demo keeps running until it exits
    Ok(())
}

/// Retype a kernel object out of our untyped budget; returns the slot its capability landed in.
fn retype_obj(objtype: u64) -> Result<u64, ()> {
    let r = user_mode_runtime::retype_object(MEMORY_REGION, objtype);
    if r < 0 { Err(()) } else { Ok(r as u64) }
}

/// Retype a page of our budget into a `PageFrame` capability; returns its cap slot.
fn retype_page_frame() -> Result<u64, ()> {
    let r = user_mode_runtime::retype_page_frame(MEMORY_REGION);
    if r < 0 { Err(()) } else { Ok(r as u64) }
}

/// Report a build failure to the kernel (a nonzero code the `least_authority_demo`'s real answer can never be, since
/// the `least_authority_demo` only ever sends a perfect square) and exit. The kernel's `recv` sees it and says so.
fn fail(code: u64) -> ! {
    let _ = send(REPORT, code, 0, 0);
    exit();
}

user_mode_runtime::panic_handler!();
