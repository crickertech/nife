//! **The system builder: userspace init for the interactive shell** (parity D), on riscv64.
//!
//! The portable counterpart of hello's `init_boot` role (which is aarch64-wired only by living
//! inside the PL011-tied `hello`). The kernel loads this as the boot process, maps the initrd, and
//! grants it the capabilities [`GRANTS`] names below. From those, and nothing else, it builds the
//! whole interactive system out of its own budget: the console server, the input driver, the line
//! discipline, the shell, the terminal's sink adapter and the job undertaker, wired together with
//! endpoints and shared pages it creates. Then it stays alive as the spawn service.
//!
//! **All of that is `crates/system_initializer` now** (milestone 96), and so is the reasoning: what init
//! gives away once the system is up, why the job pool is bounded, and the honest limits (LIFO
//! recovery, the retained scratch mappings, the sixteen-slot capability table). This file is the boot entry
//! and the one thing the two boards genuinely disagree about, which is the order their kernels grant
//! capabilities in.
//!
//! The duplication that ended here was expensive rather than untidy. `user::initrd()` loads the
//! archive entry `init`, which is `user/src/hello.rs`'s `init_boot` role on aarch64 and this program
//! on riscv64, and the construction and the spawn service were written once in each: a fix that
//! landed in one and not the other presented as a boot that reached userspace and printed nothing at
//! all, with no fault and no message. `script/shell-check` runs both, which is what makes it the
//! gate for this file.
//!
//! Name: ratified 2026-08-01 (calef, milestone 63), replacing `sysinit`. Refused `sysinit`
//! (squished), `system_builder` (`user/src/builder.rs` already calls itself "the system builder",
//! so two programs would claim one phrase), `session_initializer` (squats milestone 49's
//! vocabulary, since sessions arrive with users and login and this program manages neither) and
//! `shell_init` (refused on evidence: it looks the shell up **by name in the initrd**, so it brings
//! up whatever is packed as `shell` rather than `swish` specifically, and it stays alive as the
//! spawn service, which is not a shell concern at all).

#![no_std]
// Program entry points, not the crates/ library surface milestone 68's ratchet tracks
// (DECISIONS §107): each `[[bin]]` is its own crate root with one `_start`, and 58 of them
// documenting an OS-facing ABI entry point is not what the lint is for.
#![allow(missing_docs)]
#![no_main]

use system_initializer::BootEndowment;

/// **What `kernel::user::riscv_shell_boot` grants, in order.** The kernel inserts these into this
/// process's capability table before it starts, and the numbers below are that call's `assert_eq!`s read from
/// the other side. Slots 5 and 6 hold nothing when this boot attached no RedoxFS disk, which is what
/// `a2` (the endpoint's `filesystem_proto::dir` rights, 0 for no disk) says.
///
/// The clock and the inert-configuration page are both granted ahead of the filesystem pair on
/// purpose, so their slots are the same on every boot whether or not a disk was attached.
const GRANTS: BootEndowment = BootEndowment {
    untyped: 0,
    uart_dev: 1,
    uart_irq: 2,
    clock_page: 3,
    config_page: 4,
    fs_ep: 5,
    fs_page: 6,
    // Always these three (`kernel::user::riscv_shell_boot` grants them at explicit slots, not
    // `thread_control_block_insert_cap`'s first-free `None`, for exactly this reason): a boot with
    // no virtio-rng device leaves them empty, and `system_initializer::boot`'s own probe is what
    // tells it apart from a granted one. Fixed past the filesystem pair's own max reach (slot 6),
    // not slot 5, because the inert-configuration page (slot 4) shifted that pair down by one.
    virtio_rng: 7,
    virtio_rng_irq: 8,
    virtio_rng_dma: 9,
    // The graphical terminal stack (milestone 177, option A), fixed past the virtio-rng trio's
    // own floor (slot 9) for its own reason: a boot with no GPU or no keyboard attached leaves
    // all three empty, and `system_initializer::boot`'s own probe is what tells it apart from a
    // granted one.
    disp_term_ep: 10,
    disp_term_page: 11,
    kbd_ep: 12,
    // Nothing. Unlike aarch64's, this boot path is not shared with milestone 19d's test roles, so
    // the kernel grants exactly what the interactive system uses.
    for_test_roles: &[],
};

#[unsafe(no_mangle)]
pub extern "C" fn _start(_a0: u64, initrd_len: u64, fs_rights: u64) -> ! {
    // `None`: milestone 154's second directory grant is a real mechanism
    // (`system_initializer::boot`'s `second_dir` parameter), but what the second subtree should
    // *be* is a boot-time policy decision DECISIONS §126 reserves for calef, so this real boot
    // does not enable it.
    system_initializer::boot(&GRANTS, initrd_len, fs_rights, None)
}

user_rt::panic_handler!();
