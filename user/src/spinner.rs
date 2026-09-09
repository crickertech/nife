//! The spinner: a runaway that ignores the interrupt entirely (DECISIONS §24).
//!
//! A tight loop that touches nothing and checks nothing. It is the case the cooperative tier cannot
//! reach: the shell can set the interrupt flag all it likes, and the spinner never reads it. Only
//! the forcible tier ends it, the shell tearing the spinner's region down with object revocation
//! (DECISIONS §24, §16). That is the whole reason the second `^C` exists.
//!
//! It is deliberately a pure `loop`, accessing no memory the shell holds, so it is the honest worst
//! case: it cannot be killed by revoking a frame it depends on (it depends on none), only by the
//! region owner's `DESTROY` force-killing the resident thread. That kernel behavior is the §16
//! amendment landing alongside this milestone; until it merges, the shell's teardown of a spinner is
//! refused and the prompt returns having said so.
//!
//! It holds nothing: no capabilities, and it does not even map the shared job frame it was granted.
//!
//! Name: provisional. Introduced 2026-07-28 as the fixture the cooperative interrupt tier cannot
//! reach: a tight loop that touches nothing and checks nothing, so only the forcible tier of
//! DECISIONS §24 ends it. Nothing records the choice. The case. It is an agent noun in milestone
//! 63's family, it is the field's ordinary word for a thread that burns cycles rather than
//! blocking, and it is `heeder`'s counterpart, which is why the two want names of the same shape.
//! What the name does not say is that it holds no memory the shell granted it, which is the
//! property that makes it the honest worst case; that is in the header rather than the name, and
//! no one-word name would carry it.

#![no_std]
// Program entry points, not the crates/ library surface milestone 68's ratchet tracks
// (DECISIONS §107): each `[[bin]]` is its own crate root with one `_start`, and 58 of them
// documenting an OS-facing ABI entry point is not what the lint is for.
#![allow(missing_docs)]
#![no_main]

#[unsafe(no_mangle)]
pub extern "C" fn _start(_x0: u64, _x1: u64, _x2: u64) -> ! {
    loop {
        core::hint::spin_loop();
    }
}

user_rt::panic_handler!();
