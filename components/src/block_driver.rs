//! The virtio-blk driver as its own binary (parity C).
//!
//! The 19f pattern: the whole driver logic is the shared `virtio` module, and this file is only the
//! skeleton a dedicated binary needs (an `_start` that dispatches the role, a panic handler, the
//! runtime re-exports).
//!
//! **This was the portable analog of seven roles of `hello` until milestone 291, and those seven
//! were this file's code.** aarch64 reached the identical `crates/virtio` through the `hello`
//! multiplexer, because `initrd_aarch64`'s table never packed this binary; 291 packed it and
//! deleted the roles. Every architecture spawns this program now, with the same role numbers and
//! the same capability slots, so the kernel side was byte-identical across the two shapes and is
//! now simply one shape.
//!
//! The roles are the honest driver, the block server, the net driver, the write path, and the two
//! attackers the DMA-confinement tests spawn (a descriptor aimed at kernel memory, and the
//! indirect-descriptor escape). The attackers ride in the same binary deliberately: they differ
//! from the honest driver by one descriptor, and sharing the setup code is what makes the attack a
//! fair test rather than a straw man.
//!
//! Name: ratified 2026-08-27 (calef). Renamed from `blk` to `block_driver`, matching the
//! `<device>_driver` shape given to `gpu_driver` and `keyboard_driver` in the same pass. Not
//! `disk_driver`: virtio-blk operates on the block-device abstraction, which is medium-agnostic
//! (RAM-backed, network-backed and virtual block devices are all real, not just physical disks),
//! so `block` is the accurate term and `disk` would be a narrower, slightly false claim. `blk`
//! itself was ratified 2026-07-30 (DECISIONS §39) as an abbreviation that was the ordinary name
//! of the thing; this supersedes that entry with the fuller, consistent name.

#![no_std]
// Program entry points, not the crates/ library surface milestone 68's ratchet tracks
// (DECISIONS §107): each `[[bin]]` is its own crate root with one `_start`, and 58 of them
// documenting an OS-facing ABI entry point is not what the lint is for.
#![allow(missing_docs)]
#![no_main]

// The virtio module names `crate::{check, invoke, send}`; here they are `user_mode_runtime`'s (same
// signatures) plus the local `check`.
pub use user_mode_runtime::{invoke, send};

/// Role numbers, matching `kernel/src/user/virtio_service.rs`. This binary is the only thing that
/// dispatches on them since milestone 291.
const VIRTIO_BLK: u64 = 3;
const VIRTIO_ATTACK: u64 = 8;
const VIRTIO_ATTACK_INDIRECT: u64 = 13;
const VIRTIO_BLK_WRITE: u64 = 30;
const VIRTIO_BLK_WRITE_ABANDON: u64 = 31;
const VIRTIO_NET: u64 = 40;
/// The block server (milestone 32 phase 2): serves blocks over blk IPC to the FS server.
const VIRTIO_BLK_SERVER: u64 = 32;

/// A failed sanity check is a fault, not a wrong answer: panic, and the handler traps.
pub fn check(ok: bool) {
    if !ok {
        panic!();
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn _start(role: u64, direct_memory_access_phys: u64, _arg2: u64) -> ! {
    match role {
        VIRTIO_BLK => virtio::run(direct_memory_access_phys),
        VIRTIO_ATTACK => virtio::run_attack(direct_memory_access_phys),
        VIRTIO_ATTACK_INDIRECT => virtio::run_attack_indirect(direct_memory_access_phys),
        VIRTIO_BLK_WRITE => virtio::run_write(direct_memory_access_phys),
        VIRTIO_BLK_WRITE_ABANDON => virtio::run_write_abandon(direct_memory_access_phys),
        VIRTIO_NET => virtio::run_net(direct_memory_access_phys),
        VIRTIO_BLK_SERVER => virtio::run_blk_server(direct_memory_access_phys),
        _ => panic!(),
    }
}

user_mode_runtime::panic_handler!();
