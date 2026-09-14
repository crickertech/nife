//! **Thin safe wrappers over the four `Virtio` capability methods** (milestone 139 round 7).
//!
//! Four drivers (`gpu_driver.rs`, `entropy.rs`, `keyboard_driver.rs`, `net_transport.rs`) each called
//! `invoke(VIRTIO, abi::virtio::READ_REG/WRITE_REG/SETUP_QUEUE/NOTIFY, ...)` directly, eighteen
//! call sites in all, every one carrying the identical `// SAFETY:` comment: "the kernel validates
//! the capability and the method before acting." That comment is `invoke`'s own contract, not a
//! per-call obligation any of these four methods add: `abi::virtio`'s own doc already states what
//! the kernel checks (`WRITE_REG` refuses the queue-address and notify registers; `SETUP_QUEUE`
//! and `NOTIFY` are bounds-checked against the driver's DMA region and its own high-water mark),
//! so the raw `invoke` site could not add anything a wrapper cannot add just as well.
//!
//! Scoped to a module, not lifted into the crate root, because unlike [`super::send`] or
//! [`super::reply`] these four methods are meaningful to exactly the programs that own a `Virtio`
//! capability: the same "opt in by importing the module" shape [`super::heap`],
//! [`super::initrd`] and [`super::mapped_window`] already use for functionality most programs
//! never touch.

/// `READ_REG` at `off` on the `Virtio` capability in `virtio_slot`. Reads are DMA-safe, so any
/// register. Returns the register value, or a negative `abi::Error`.
pub fn virtio_read_reg(virtio_slot: u64, off: u64) -> i64 {
    // SAFETY: `svc`/`ecall`. The kernel validates the Virtio capability before it reads the
    // register; every register is DMA-safe to read.
    unsafe { super::invoke(virtio_slot, abi::virtio::READ_REG, off, 0, 0) }
}

/// `WRITE_REG` `val` at `off` on the `Virtio` capability in `virtio_slot`. `0` on success; a
/// negative `abi::Error` (`NotPermitted` for a queue-address or notify register, which only
/// [`virtio_setup_queue`]/[`virtio_notify`] may touch).
pub fn virtio_write_reg(virtio_slot: u64, off: u64, val: u64) -> i64 {
    // SAFETY: `svc`/`ecall`. The kernel validates the Virtio capability and refuses any register
    // this call is not DMA-safe to write.
    unsafe { super::invoke(virtio_slot, abi::virtio::WRITE_REG, off, val, 0) }
}

/// `SETUP_QUEUE` virtqueue `queue` (0-indexed; a virtio-net device uses 0 = receive, 1 = transmit)
/// with `num` descriptors, on the `Virtio` capability in `virtio_slot`. The kernel programs the
/// ring addresses to the fixed offsets of that queue's ring block in the driver's own DMA region;
/// the driver never chooses them. `0` on success; a negative `abi::Error` (`BadQueue`/`WrongObject`
/// if `queue` is out of range or the block does not fit the region).
pub fn virtio_setup_queue(virtio_slot: u64, num: u64, queue: u64) -> i64 {
    // SAFETY: `svc`/`ecall`. The kernel validates the Virtio capability, `queue`, and that the
    // ring block fits the driver's DMA region before it programs anything.
    unsafe { super::invoke(virtio_slot, abi::virtio::SETUP_QUEUE, num, queue, 0) }
}

/// `NOTIFY` the device that virtqueue `queue` has new descriptors, on the `Virtio` capability in
/// `virtio_slot`. `0` on success; a negative `abi::Error` (`DeviceRefused` if a newly-published
/// descriptor points outside the driver's DMA region, in which case the device is not told to go).
pub fn virtio_notify(virtio_slot: u64, queue: u64) -> i64 {
    // SAFETY: `svc`/`ecall`. The kernel validates every published descriptor on `queue` against the
    // driver's own high-water mark before it lets the device see them.
    unsafe { super::invoke(virtio_slot, abi::virtio::NOTIFY, queue, 0, 0) }
}
