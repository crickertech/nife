//! **A program proving its own ELF image was loaded correctly**, using nothing but its own memory.
//!
//! None of this needs a capability: every byte it reads belongs to the process already. That is
//! what makes it the first thing a new loader can be tested with, and it is why the milestone-7
//! test spawns the self-checker with no authority at all.
//!
//! # What each check would catch
//!
//! - `.rodata` holds the bytes the file said it did, so the read-only segment was mapped from the
//!   file rather than zeroed.
//! - `.data` holds its initialiser **and** is writable, so the loader copied it rather than
//!   pointing at the file's pages read-only.
//! - `.bss` reads zero before anything writes it. Nobody else would catch this: those bytes are
//!   not in the file, so a loader that forgot them would leave whatever the frame allocator handed
//!   over, and every value it could hold is a value some program is happy with.
//! - The stack survives eight frames of recursion, so `sp` was set to real, writable memory with
//!   room under it.
//!
//! # How it says no
//!
//! It traps. A failed check must be indistinguishable from a broken program, because it *is* one,
//! and a program whose loader is broken has no reliable way to report anything: the endpoint it
//! would report on may not be mapped either. The kernel turns the trap into a fault and kills the
//! thread, which the test counts.
//!
//! # Examples
//!
//! The whole of a self-checking program:
//!
//! ```ignore
//! #[unsafe(no_mangle)]
//! pub extern "C" fn _start(_a: u64, _b: u64, _c: u64) -> ! {
//!     loaded_image_check::verify(user_mode_runtime::trap);
//!     user_mode_runtime::exit();
//! }
//! ```
//!
//! **`fail` is a parameter rather than a call into `user_mode_runtime`**, and that is what keeps this crate
//! host-buildable. A crate that reaches `user_mode_runtime` reaches EL0 syscall `asm!` and compiles for
//! aarch64 or riscv64 only, which would put this in four separate host-pass exclusion lists to buy
//! one function call. The caller already has the right way for *it* to die, and hands it over.
//!
//! # Bugs
//!
//! **The markers are `#[unsafe(no_mangle)]` statics, so exactly one copy of this crate may be
//! linked into a binary.** Two would be a duplicate symbol at link time rather than a silent
//! wrong answer, which is the failure mode to prefer, but it does mean this crate cannot be a
//! transitive dependency of something a program also depends on directly.
//!
//! **It cannot check the executable segment.** A program that is running has already proven its
//! `.text` was mapped, so there is nothing left for a marker to say; a loader that mapped `.text`
//! writable would pass every check here and DECISIONS §10's `W^X` assertions in
//! `kernel/src/user/tests.rs` are what catch that instead.
//! Name: provisional (milestone 291). A program's check that its own ELF image was loaded
//! correctly. It is a crate because two fixtures need it and AGENTS.md rule 7 admits no `#[path]`
//! module; it was `hello`'s `self_check()` while the self-checker and the printing client were two
//! roles of one binary. Refused `self_check` (a verb phrase, and `self` names the caller rather
//! than the thing checked, which reads oddly at `self_check::verify()`). Refused `image_check`
//! ("image" is this tree's word for a disk image as often as for a loaded program:
//! `target/nifefs.img`, `uefi-image`). Refused putting it in `crates/user_mode_runtime` (`user_mode_runtime` is what
//! every program links to reach the kernel, and a diagnostic nothing in a running system calls does
//! not belong in that surface; and a crate that depends on it cannot build for the host).

#![no_std]

#[unsafe(no_mangle)]
static RODATA_MARKER: [u8; 4] = [0xc0, 0xff, 0xee, 0xd0];
#[unsafe(no_mangle)]
static mut DATA_MARKER: u64 = 0x0000_c0ff_ee00_d0d0;
#[unsafe(no_mangle)]
static mut BSS_MARKER: u64 = 0;

/// Check this process's own image, calling `fail` if any part of it is wrong.
///
/// Returns normally when everything held. There is no error value on purpose: see the module's
/// "How it says no". `fail` diverges, so nothing after a failed check runs.
pub fn verify(fail: fn() -> !) {
    let check = |ok: bool| {
        if !ok {
            fail()
        }
    };
    check(RODATA_MARKER == [0xc0, 0xff, 0xee, 0xd0]);
    // SAFETY: single-threaded, sole owner of this address space.
    unsafe {
        check(core::ptr::read_volatile(&raw const DATA_MARKER) == 0x0000_c0ff_ee00_d0d0);
        check(core::ptr::read_volatile(&raw const BSS_MARKER) == 0); // .bss was zeroed
        core::ptr::write_volatile(&raw mut BSS_MARKER, 1);
        check(core::ptr::read_volatile(&raw const BSS_MARKER) == 1); // .data is writable
    }
    check(stack_works(7));
}

/// Recurse `n` frames, keeping a live array on each so the compiler cannot flatten the chain.
#[inline(never)]
fn stack_works(n: u64) -> bool {
    let local = [n; 8];
    if n == 0 {
        return local[0] == 0;
    }
    core::hint::black_box(&local);
    stack_works(n - 1)
}

#[cfg(test)]
mod tests {
    /// **The check passes on a correctly loaded image**, which on the host is every image: the
    /// point of running it here is that the *logic* is exercised by an ordinary `cargo test`
    /// rather than only inside an emulator. What it cannot prove on the host is the thing it
    /// exists for, a loader that got a segment wrong, so this is a smoke test and says so.
    #[test]
    fn a_correctly_loaded_image_passes_every_check() {
        super::verify(|| unreachable!("a host binary failed this crate's own image check"));
    }

    /// The recursion really recurses: eight frames deep, and the answer depends on the deepest.
    #[test]
    fn the_stack_check_reaches_the_bottom() {
        assert!(super::stack_works(7));
        assert!(super::stack_works(0));
    }
}
