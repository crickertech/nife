//! **The system builder: userspace init for the interactive shell**, on all three architectures.
//!
//! The kernel loads this as the boot process, maps the initrd, and
//! grants it the capabilities `GRANTS` names below. From those, and nothing else, it builds the
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
//! **One program, three architectures.** Until this change aarch64's first process was a *role* of
//! the `hello` demo catalogue (`init_boot`, role 27) while riscv64 and `x86_64` got this purpose-built
//! one, so the archive entry the kernel loads meant a different binary depending on the board. That
//! is [DECISIONS §19](../../design/decisions/19-architectural-parity.md)'s own failure mode, and the
//! bill was paid once already: a fix that landed in one copy and not the other presented as a boot
//! that reached userspace and printed nothing at all, with no fault and no message.
//! `script/shell-check` runs both legs, which is what makes it the gate for this file.
//!
//! **The one `cfg` is the one genuine difference**, and it is not a portability seam: the two
//! kernels grant the boot capabilities *in a different order*, so the slot numbers differ. Nothing
//! else about building the system does, and there is no third thing to abstract over, so the
//! difference is data under a `cfg` rather than a trait or a runtime probe.
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

/// **What `kernel::user::spawn_init` grants on aarch64, in order.** The kernel inserts these into
/// this process's capability table before it starts, and the numbers below are that function's own
/// grant sequence read from the other side.
///
/// **It is not riscv64's numbering, and the reason is history rather than design.** This path is
/// shared with milestone 19d's test roles, which enter `hello` with the same endowment and whose
/// slot numbering must not move, so it carries two capabilities the interactive system has no use
/// for: the kernel's report endpoint (slot 1) and the 19d.2b test interrupt (slot 3).
/// [`boot`](system_initializer::boot) deletes them with the device authority once the drivers exist.
///
/// The clock and the inert-configuration page are both granted ahead of the filesystem pair on
/// purpose, so their slots are the same on every boot whether or not a disk was attached. Slots
/// 7 and 8 hold nothing when this boot attached no RedoxFS disk, which is what `fs_rights` (0
/// for no disk) says.
#[cfg(target_arch = "aarch64")]
const GRANTS: BootEndowment = BootEndowment {
    untyped: 0,
    uart_dev: 2,
    uart_irq: 4,
    clock_page: 5,
    config_page: 6,
    fs_ep: 7,
    fs_page: 8,
    // Always these three (`kernel::user::spawn_init` grants them with `grant_at`, not `grant`'s
    // first-free numbering, for exactly this reason): a boot with no virtio-rng device leaves them
    // empty, and `system_initializer::boot`'s own probe is what tells it apart from a granted one.
    // Fixed past the filesystem pair's own max reach (slot 8), not past slot 7, because milestone
    // 47's config_page (slot 6) shifted that pair down by one.
    virtio_rng: 9,
    virtio_rng_irq: 10,
    virtio_rng_dma: 11,
    // The graphical terminal stack (milestone 177, option A), fixed past the virtio-rng trio's own
    // floor (slot 11) for its own reason: a boot with no GPU or no keyboard attached leaves all
    // three empty, and `system_initializer::boot`'s own probe is what tells it apart from a granted
    // one.
    disp_term_ep: 12,
    disp_term_page: 13,
    kbd_ep: 14,
    // The kernel's report endpoint (slot 1) and the milestone-19d.2b test interrupt (slot 3): the
    // two this boot path carries only because the 19d test roles share it. Nothing interactive
    // receives on either.
    for_test_roles: &[1, 3],
};

/// **What `kernel::user::riscv_shell_boot` grants, in order.** The kernel inserts these into this
/// process's capability table before it starts, and the numbers below are that call's `assert_eq!`s read from
/// the other side. Slots 5 and 6 hold nothing when this boot attached no RedoxFS disk, which is what
/// `a2` (the endpoint's `filesystem_proto::dir` rights, 0 for no disk) says.
///
/// The clock and the inert-configuration page are both granted ahead of the filesystem pair on
/// purpose, so their slots are the same on every boot whether or not a disk was attached.
#[cfg(not(target_arch = "aarch64"))]
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
    // `_a0` is the role the kernel entered this program at. This program has exactly one role, so it
    // does not read it; aarch64's `spawn_init` still passes one because the same function enters
    // `hello` at milestone 19d's test roles.
    //
    // `None`: milestone 154's second directory grant is a real mechanism
    // (`system_initializer::boot`'s `second_dir` parameter), but what the second subtree should
    // *be* is a boot-time policy decision DECISIONS §126 reserves for calef, so this real boot
    // does not enable it.
    system_initializer::boot(&GRANTS, initrd_len, fs_rights, None)
}

user_rt::panic_handler!();
