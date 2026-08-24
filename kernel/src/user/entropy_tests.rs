use entropy_service::Bus;

use super::*;

/// Reach the service on `bus`, wiring it if this is the first test to ask, and check its
/// startup report when this call is the one that wired it.
///
/// Returns `None` if the bus has no virtio-rng device (milestone 145: correct on a bare board
/// boot, which has no equivalent of the runner's `NIFE_RNG`-gated `-device virtio-rng` line);
/// callers must `skip!()` at their own call site rather than have this helper do it, because
/// `skip!()` returns from its immediate caller and a helper is not the test.
fn start(bus: Bus) -> Option<entropy_service::Wiring> {
    let image = program("entropy").expect("no entropy program in the initrd archive");
    let w = entropy_service::ensure(image, bus)?;
    if let Some(report) = w.wait_for_ready() {
        assert_eq!(
            report[0],
            entropy_proto::READY,
            "the entropy service did not come up on {bus:?} (it reported {:#x}; a 0xDEAD_.. \
             word's low byte names the step, see user/src/entropy.rs)",
            report[0],
        );
        assert_eq!(
            report[1], 1,
            "the entropy service came up on {bus:?} but the device gave it no bytes at all",
        );
    }
    Some(w)
}

/// Draw `WORDS` eight-byte words through the request endpoint, asserting every draw is full.
/// Deliberately more than one bufferful (the service fetches 256 bytes per device request), so
/// this crosses the refill boundary and a cursor that wrapped instead of refilling shows up as
/// a repeat below.
const WORDS: usize = 64;

fn draw(w: &entropy_service::Wiring) -> [u64; WORDS] {
    let mut words = [0u64; WORDS];
    for (i, slot) in words.iter_mut().enumerate() {
        let mut buf = [0u8; 8];
        let n = w.get(8, &mut buf);
        assert_eq!(
            n, 8,
            "draw {i} of {WORDS} on {:?} returned {n} bytes, not 8: the device ran dry, or the \
             service failed to refill",
            w.bus,
        );
        *slot = u64::from_le_bytes(buf);
    }
    words
}

/// Every word distinct, and none of them zero. With a real source a collision among 64 draws is
/// a 2^-58 event, so a failure here is a bug rather than bad luck: a stuck device, a buffer
/// served twice, or a used ring the driver never re-read all present exactly this way.
fn assert_unpredictable(words: &[u64; WORDS], what: &str) {
    for (i, &a) in words.iter().enumerate() {
        assert_ne!(
            a, 0,
            "{what}: draw {i} is all zeros, which is the DMA page unwritten"
        );
        for (j, &b) in words.iter().enumerate().skip(i + 1) {
            assert_ne!(a, b, "{what}: draws {i} and {j} are identical ({a:#018x})");
        }
    }
}

/// **The headline, over virtio-mmio.** A client that holds one endpoint and no device gets
/// bytes off a real random-number generator, 512 of them, across a refill, all different.
#[test_case]
fn a_client_obtains_unpredictable_bytes_from_a_virtio_rng_over_mmio() {
    let Some(w) = start(Bus::Mmio) else {
        crate::testing::skip!("no virtio-rng device on the mmio bus (NIFE_RNG not set?)");
    };
    let words = draw(&w);
    assert_unpredictable(&words, "mmio");
}

/// **The same service, the same binary, over PCIe** (DECISIONS §18), and behind the IOMMU while
/// it is there. An entropy source's buffer is the one page in memory whose contents must not be
/// guessable, so an unconfined device writing it is worth asserting against rather than hoping.
#[test_case]
fn a_client_obtains_unpredictable_bytes_from_a_virtio_rng_over_pcie() {
    let Some(w) = start(Bus::Pci) else {
        crate::testing::skip!("no virtio-rng device on the pcie bus (NIFE_RNG not set?)");
    };
    assert!(
        w.confined_by_iommu,
        "the PCIe RNG is present but not behind the IOMMU: the buffer the device writes the \
         system's key material into is unconfined (is iommu_platform=on missing from the \
         runner's virtio-rng-pci line?)",
    );
    let words = draw(&w);
    assert_unpredictable(&words, "pcie");
}

/// **Two independent sources do not agree**, which is what says the bytes came from the devices
/// rather than from anything shared underneath them (a fixed seed, a counter, the DMA page's
/// previous contents). Also the cheapest proof that two services can hold two devices at once.
#[test_case]
fn two_entropy_services_on_two_devices_do_not_produce_the_same_bytes() {
    let Some(mmio) = start(Bus::Mmio) else {
        crate::testing::skip!("no virtio-rng device on the mmio bus (NIFE_RNG not set?)");
    };
    let Some(pci) = start(Bus::Pci) else {
        crate::testing::skip!("no virtio-rng device on the pcie bus (NIFE_RNG not set?)");
    };
    let a = draw(&mmio);
    let b = draw(&pci);
    for (i, (&x, &y)) in a.iter().zip(b.iter()).enumerate() {
        assert_ne!(
            x, y,
            "draw {i} is identical on both devices ({x:#018x}): these are not two sources",
        );
    }
}

/// **The count in a reply is the truth about the reply.** A short request gets exactly that
/// many bytes and leaves the rest of the caller's buffer alone, so a caller cannot be handed
/// padding it mistakes for entropy; an oversized one is clamped and answered rather than
/// refused; and an opcode the service does not implement is answered with nothing rather than
/// killing the service, which the draw afterwards proves.
#[test_case]
fn a_reply_never_delivers_more_bytes_than_it_says() {
    let Some(w) = start(Bus::Mmio) else {
        crate::testing::skip!("no virtio-rng device on the mmio bus (NIFE_RNG not set?)");
    };

    let mut buf = [0xAAu8; 8];
    assert_eq!(w.get(3, &mut buf), 3, "asked for three bytes");
    assert_eq!(
        &buf[3..],
        &[0xAA; 5],
        "the service wrote past the count it reported",
    );

    let mut big = [0u8; 8];
    assert_eq!(
        w.get(200, &mut big),
        entropy_proto::MAX_BYTES as usize,
        "an oversized request should be clamped and answered, not refused",
    );

    let r = crate::sched::ipc_call(w.request, [entropy_proto::req(0xff, 8), 0]);
    assert_eq!(
        r[0],
        entropy_proto::NO_ENTROPY,
        "an unknown opcode should be answered with no bytes",
    );

    let mut after = [0u8; 8];
    assert_eq!(
        w.get(8, &mut after),
        8,
        "the service stopped serving after an unknown opcode",
    );
}

/// **The instruction backend, milestone 162: RNDRRS on aarch64, no virtio device at all.**
///
/// `Bus::Instruction` needs `ID_AA64ISAR0_EL1.RNDR` set, which the suite's default CPU
/// (`cortex-a72`, an ARMv8.0-A part) does not implement: `FEAT_RNG` is ARMv8.5. This is the same
/// shape as the virtio tests above skipping when `NIFE_RNG` is unset, one level down the stack:
/// a real hardware precondition this suite cannot fake, named rather than assumed. Run this test
/// for real with `script/test --arch aarch64 --cpu neoverse-n2` (verified 2026-08-24 against QEMU
/// 11.0.2). **Not `--cpu max`**: QEMU's `max` model does carry `FEAT_RNG`, but this kernel refuses
/// to boot on it at all ("no 4 KiB stage-1 granule (`ID_AA64MMFR0_EL1.TGran4`)"), a QEMU-model quirk
/// unrelated to entropy; `neoverse-n2` (Armv9.0-A) has both. On `x86_64`, `RDSEED` has no such gap
/// because ring 3 does not exist yet on that port at all (see
/// `design/roadmap/162-cpu-instruction-entropy.md`), so there is no service to test end to end
/// there yet, only the kernel-side probe the boot tour already proves. This test also compiles and
/// runs (and skips) on riscv64, which has neither instruction: `instruction_backend_available`
/// there is unconditionally `false`, the JH7110's real hardware source (milestone 159) being a
/// separate driver entirely, so the skip is correct there too, just for a different reason.
#[test_case]
fn a_client_obtains_unpredictable_bytes_from_rndrrs_with_no_device_at_all() {
    let Some(w) = start(Bus::Instruction) else {
        crate::testing::skip!(
            "no instruction-mode entropy source on this build (aarch64 needs FEAT_RNG, \
             ID_AA64ISAR0_EL1.RNDR clear here; try --cpu neoverse-n2. riscv64 has none.)"
        );
    };
    assert!(
        !w.confined_by_iommu,
        "an instruction has no device for an IOMMU to confine",
    );
    let words = draw(&w);
    assert_unpredictable(&words, "rndrrs");
}
