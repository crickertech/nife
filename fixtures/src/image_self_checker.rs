//! **A program that checks its own image and then does nothing but exist**, milestone 7.
//!
//! This is the "a real ELF ran and verified itself" fixture. It needs no capabilities and no
//! shared memory, which is why `kernel::user::tests` can spawn it bare: everything it inspects is
//! its own memory, and the one syscall it makes is a yield, which is authority over itself that
//! nobody has to grant.
//!
//! What the kernel test reads from it is not a report. It is the *absence* of a fault plus the
//! presence of one syscall, and that pair says: `.text` executed, `.rodata` was readable, `.data`
//! was copied from the file, `.bss` was zeroed (those bytes are not in the file), and the stack
//! worked well enough to recurse eight frames. The checks themselves are
//! [`loaded_image_check`], which exists so this program and `console_test_client` share one
//! definition rather than two copies.
//!
//! It **exits** rather than spinning. A one-shot fixture with nothing left to do that never exits
//! sits on a core for the rest of the boot; `no_leaked_threads` says so in as many words, and it
//! was three such leaks that starved a later test off a four-hart machine entirely.
//!
//! Name: provisional (milestone 291). This was `hello`'s `SELF_CHECK` role, number 0, and 291 made
//! each of that binary's roles its own program. Refused `self_checker`: "self" names the caller
//! rather than what is checked, and this program checks the loaded image specifically. Refused
//! `elf_verifier`: it verifies nothing about ELF parsing, which is the `elf` crate's host tests;
//! it verifies that *this* image arrived intact in memory.

#![no_std]
// Program entry points, not the crates/ library surface milestone 68's ratchet tracks
// (DECISIONS §107): each `[[bin]]` is its own crate root with one `_start`, and 58 of them
// documenting an OS-facing ABI entry point is not what the lint is for.
#![allow(missing_docs)]
#![no_main]

#[unsafe(no_mangle)]
pub extern "C" fn _start(_arg0: u64, _arg1: u64, _arg2: u64) -> ! {
    loaded_image_check::verify(user_mode_runtime::trap);

    // One syscall that needs no capability at all, to prove we reached EL0 and can trap back in.
    user_mode_runtime::yield_now();

    user_mode_runtime::exit();
}

user_mode_runtime::panic_handler!();
