//! **A confined EL0 process drives the machine's NVMe disk** (milestone 261; DECISIONS §86's
//! option 2a, notes/nvme.md).
//!
//! This is the test the kernel-resident driver's own end-to-end test became. The sequence is the
//! same one milestone 53 proved (find the controller over the §18 PCIe transport, confine it
//! behind the machine's IOMMU, bring it from reset through identify to an I/O queue pair, then
//! serve the blk verbs), and exactly one thing about it has changed: **the half that touches the
//! queue runs at EL0 now**, in a process holding two endpoints, one page of BAR0 and a run of DMA
//! pages, with no capability naming any of it.
//!
//! **This is not fatal risk 6's experiment.** Milestone 16b already proved IOMMU-backed isolation
//! against emulated silicon; risk 6's open clause is a *real* device at *real* speed, on xenon,
//! photographed. What runs here is that experiment's prerequisite, and any number measured under
//! QEMU is a number about QEMU.

use super::*;

/// Wire the server, or `None` when this machine has no NVMe controller (a bare board boot, or a
/// leg the runner did not attach one on). Callers `skip!()` at their own call site rather than
/// have this helper do it, because `skip!()` returns from its immediate caller and a helper is
/// not the test.
fn start() -> Option<nvme_service::Wiring> {
    let image = program("nvme_server").expect("no nvme_server program in the initrd archive");
    let w = nvme_service::ensure(image)?;
    if let Some(report) = w.wait_for_ready() {
        assert_eq!(
            report[0],
            filesystem_protocol::fixture::READY,
            "the NVMe server did not come up (it reported {:#x}, first-read status {:#x}; see \
             components/src/nvme_server.rs for the step words)",
            report[0],
            report[2],
        );
        assert_eq!(
            report[1],
            8 * 1024 * 1024,
            "the namespace size did not survive the spawn handoff into ring 3",
        );
    }
    Some(w)
}

/// **The headline.** An unprivileged process drives a real NVMe controller: it answers the disk's
/// size, persists a block, brings it back byte for byte, and leaves the blocks it was not asked
/// about alone. Every one of those answers crossed a rendezvous from a client that holds no
/// device, no doorbell and no DMA page.
///
/// One test on purpose, for the reason its kernel-resident ancestor gave: bring-up is not
/// idempotent state to share between cases (a second wiring would reset the controller and
/// recreate the queues under the first), so the sequence lives in one place with the ordering
/// visible.
#[test_case]
fn a_confined_el0_process_serves_the_block_interface_end_to_end() {
    let Some(disk) = start() else {
        crate::testing::skip!("no NVMe controller came up (NIFE_NVME not set on this leg?)");
    };

    // On every test boot an IOMMU fronts the PCIe bus on all three ISAs, so what this run proved
    // is the confined configuration; say so if that ever silently stops being true. It matters
    // more here than it did for the kernel-resident driver: with the driver at EL0 the IOMMU is
    // the *whole* of what stops a compromised server reaching memory it was not given, where
    // before it was a second line behind the kernel's own arithmetic.
    assert!(
        disk.confined_by_iommu,
        "the NVMe server was wired without an IOMMU; the confinement claim is untested"
    );

    // SIZE: the runner's image is 8 MiB (xtask's mknvmedisk), and the server was told the
    // geometry rather than allowed to ask, since asking means IDENTIFY and IDENTIFY is admin.
    assert_eq!(
        disk.blk(filesystem_protocol::blk::SIZE, 0),
        8 * 1024 * 1024,
        "the server answered the wrong size"
    );

    // WRITE a block whose bytes are a function of their offset, far enough in that a server
    // confusing block and LBA units (the classic factor-of-8) would land visibly elsewhere.
    const BLOCK: u64 = 37;
    // SAFETY: the blk contract's turn-taking. A request is a CALL, so this thread holds the
    // buffer exactly while the server is not running, and vice versa.
    for (i, b) in unsafe { disk.transfer_block() }.iter_mut().enumerate() {
        *b = (i as u64 % 251) as u8; // 251 is prime to 4096, so no page-periodic alias
    }
    assert_eq!(
        disk.blk(filesystem_protocol::blk::WRITE, BLOCK),
        0,
        "the server refused the write"
    );

    // Clobber the buffer, READ the block back, and every byte must be the function again.
    // SAFETY: as above.
    unsafe { disk.transfer_block() }.fill(0xaa);
    assert_eq!(
        disk.blk(filesystem_protocol::blk::READ, BLOCK),
        0,
        "the server refused the read"
    );
    // SAFETY: as above.
    for (i, b) in unsafe { disk.transfer_block() }.iter().enumerate() {
        assert_eq!(
            *b,
            (i as u64 % 251) as u8,
            "byte {i} of the block came back wrong"
        );
    }

    // A block this test never wrote is still the image's zeros: the write landed where it said,
    // not everywhere.
    assert_eq!(disk.blk(filesystem_protocol::blk::READ, BLOCK + 1), 0);
    // SAFETY: as above.
    assert!(unsafe { disk.transfer_block() }.iter().all(|b| *b == 0));

    // **A block outside the namespace is refused rather than asked for.** `nvme::Handoff`'s range
    // check is what does it (`crates/nvme`, Kani-proved), and the point is that the refusal
    // happens in the driver's own arithmetic rather than arriving as a controller status nobody
    // can attribute. 8 MiB is 2048 filesystem blocks, so 2048 is one past the end.
    assert!(
        disk.blk(filesystem_protocol::blk::READ, 8 * 1024 * 1024 / 4096) < 0,
        "the server read a block the namespace does not have"
    );

    // **FLUSH answers a count, and the count moves.** The contract's own falsifiability argument:
    // two syncs returning the same number would mean the second never reached the device. Unlike
    // the virtio server this one never answers EOPNOTSUPP, because NVMe 1.4 §6.8 makes Flush
    // mandatory.
    let first = disk.blk(filesystem_protocol::blk::FLUSH, 0);
    assert!(first >= 1, "the server refused a flush: {first}");
    assert_eq!(
        disk.blk(filesystem_protocol::blk::FLUSH, 0),
        first + 1,
        "a second flush returned the same count, so it never reached the controller"
    );

    // **An opcode the server has no verb for is refused, not guessed at.**
    assert!(disk.blk(0xfe, 0) < 0, "the server answered an unknown verb");
}
