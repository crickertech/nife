use super::*;
use crate::cap::{Rights, irq_cap, rendezvous_cap, virtio_cap};
use crate::sched::RendezvousId;

/// Where the driver maps its DMA page and the input ring. Must match `user/src/keyboard_driver.rs`.
const DMA_VA: u64 = 0x0000_0000_0090_0000;
const RING_VA: u64 = 0x0000_0000_0082_0000;

/// One page, like every other driver here except the GPU driver's. A keyboard's event queue is
/// eight eight-byte records; there is nothing bulk about it, so the standing rule holds in the
/// other direction too: **a device gets the grant it needs and no more.**
const DMA_PAGE_FRAMES: u64 = 1;

pub struct Wiring {
    /// The driver's status endpoint.
    pub report: RendezvousId,
    /// The doorbell the driver rings. The kernel holds READ here, playing the compositor.
    pub doorbell: RendezvousId,
    /// The input ring's frame, so the kernel can read what was typed.
    pub ring: u64,
    head: u32,
}

/// **Wire and spawn the keyboard driver.** `None` if no virtio-input function is on the bus.
///
/// The kernel keeps the doorbell's receiving half and the ring, so it can stand in for the
/// compositor; a real system hands both to `compositor` instead and nothing about this driver
/// changes, which is the same swap rung two made at the display seam.
pub fn start(image: &'static [u8]) -> Option<Wiring> {
    let d = crate::pci::find_input_device()?;

    let dma = crate::memory::alloc_contiguous(DMA_PAGE_FRAMES as usize)
        .expect("no DMA region for the keyboard driver")
        .addr();
    // SAFETY: a fresh frame, direct-mapped, owned by nobody else. Zeroed so no stale descriptor
    // and no stale event is ever visible to the device or to us.
    unsafe {
        core::ptr::write_bytes(
            mmu::phys_to_virt(dma) as *mut u8,
            0,
            (DMA_PAGE_FRAMES * FRAME_SIZE) as usize,
        );
    }
    let ring = crate::memory::alloc()
        .expect("no frame for the input ring")
        .addr();
    // SAFETY: as above.
    unsafe {
        core::ptr::write_bytes(mmu::phys_to_virt(ring) as *mut u8, 0, FRAME_SIZE as usize);
    };

    let irq_ep = crate::sched::create_rendezvous();
    crate::sched::bind_irq(d.intid, irq_ep);
    crate::arch::irq::enable(d.intid);

    let vid = crate::virtio::register(
        crate::virtio::Transport::pci(&d),
        dma,
        DMA_PAGE_FRAMES * FRAME_SIZE,
        Some(d.rid),
    );

    let report = crate::sched::create_rendezvous();
    let doorbell = crate::sched::create_rendezvous();

    let maps = [
        Mapping {
            va: DMA_VA,
            phys: dma,
            flags: Flags::user_data(),
        },
        Mapping {
            va: RING_VA,
            phys: ring,
            flags: Flags::user_data(),
        },
    ];
    crate::sched::spawn(move || {
        run(
            image,
            Spawn {
                arg0: 0,
                arg1: dma, // the DMA region's PHYSICAL base: descriptors speak physical
                arg2: 0,
                grants: &[
                    rendezvous_cap(report, Rights::WRITE),   // slot 0: status
                    irq_cap(d.intid),                        // slot 1: the event interrupt
                    virtio_cap(vid),                         // slot 2: the confined transport
                    rendezvous_cap(doorbell, Rights::WRITE), // slot 3: ring the compositor
                ],
                maps: &maps,
            },
        )
    })
    .expect("could not spawn the keyboard driver");

    Some(Wiring {
        report,
        doorbell,
        ring,
        head: 0,
    })
}

/// `arg0`'s direct-wiring value. Must match `user/src/keyboard_driver.rs` `MODE_DIRECT`.
const MODE_DIRECT: u64 = 1;

/// **Wire and spawn the keyboard driver in `MODE_DIRECT`** (milestone 177, option A): a fixed
/// `CALL` target instead of the compositor's ring and doorbell, for a boot with exactly one
/// terminal and no compositor in the input path at all
/// (design/roadmap/177-graphical-interactive-boot.md's own reasoning: the compositor's focus
/// arbitration answers a multi-client question a single-terminal boot does not have).
///
/// `target` is the endpoint the driver will `CALL` with `line_editor::proto::OP_BYTES`, granted
/// here with `WRITE` and nothing else, so this driver can name exactly one destination and no
/// other. Ordinarily `line_editor`'s own served endpoint (its slot 0), the same endpoint
/// `user/src/input.rs`'s UART driver already holds `WRITE` on for the plain-console boot: a
/// keyboard and a serial line are both "one input source" to the line discipline.
///
/// Returns the driver's report endpoint, or `None` if no virtio-input function is on the bus. No
/// input ring and no doorbell exist in this wiring: nothing here plays the compositor, because
/// there is no compositor in this path.
pub fn start_direct(image: &'static [u8], target: RendezvousId) -> Option<RendezvousId> {
    let d = crate::pci::find_input_device()?;

    let dma = crate::memory::alloc_contiguous(DMA_PAGE_FRAMES as usize)
        .expect("no DMA region for the keyboard driver")
        .addr();
    // SAFETY: a fresh frame, direct-mapped, owned by nobody else. Zeroed so no stale descriptor
    // and no stale event is ever visible to the device.
    unsafe {
        core::ptr::write_bytes(
            mmu::phys_to_virt(dma) as *mut u8,
            0,
            (DMA_PAGE_FRAMES * FRAME_SIZE) as usize,
        );
    }

    let irq_ep = crate::sched::create_rendezvous();
    crate::sched::bind_irq(d.intid, irq_ep);
    crate::arch::irq::enable(d.intid);

    let vid = crate::virtio::register(
        crate::virtio::Transport::pci(&d),
        dma,
        DMA_PAGE_FRAMES * FRAME_SIZE,
        Some(d.rid),
    );

    let report = crate::sched::create_rendezvous();

    // Only the DMA page: MODE_DIRECT has no input ring to map, because it has no compositor to
    // share one with.
    let maps = [Mapping {
        va: DMA_VA,
        phys: dma,
        flags: Flags::user_data(),
    }];
    crate::sched::spawn(move || {
        run(
            image,
            Spawn {
                arg0: MODE_DIRECT,
                arg1: dma, // the DMA region's PHYSICAL base: descriptors speak physical
                arg2: 0,
                grants: &[
                    rendezvous_cap(report, Rights::WRITE), // slot 0: status
                    irq_cap(d.intid),                      // slot 1: the event interrupt
                    virtio_cap(vid),                       // slot 2: the confined transport
                    rendezvous_cap(target, Rights::WRITE), // slot 3: line_editor, directly
                ],
                maps: &maps,
            },
        )
    })
    .expect("could not spawn the keyboard driver");

    Some(report)
}

impl Wiring {
    /// **Take what the driver has typed into the ring**, advancing the head the way a compositor
    /// does. Returns how many bytes landed in `out`.
    pub fn take_typed(&mut self, out: &mut [u8]) -> usize {
        use compositor::proto::ring;
        let base = mmu::phys_to_virt(self.ring);
        // SAFETY: inside the ring frame this kernel allocated and shares with the driver.
        let tail = unsafe { core::ptr::read_volatile((base + ring::TAIL) as *const u32) };
        // The tail is published after the bytes it advertises; read it before them.
        //
        // PAIR: `ring_publish`'s `fence(SeqCst)` in user/src/keyboard_driver.rs. Milestone 43's audit named this
        // reader as the one that gets it right, against `user/src/compositor.rs`'s `drain_input`,
        // which reads the same contract and had no fence at all.
        core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);
        let mut n = 0;
        while self.head != tail && n < out.len() {
            let at = base + ring::BYTES + (self.head % ring::CAPACITY) as u64;
            // SAFETY: inside the ring frame.
            out[n] = unsafe { core::ptr::read_volatile(at as *const u8) };
            self.head = self.head.wrapping_add(1);
            n += 1;
        }
        // SAFETY: inside the ring frame; the head is ours to advance.
        unsafe { core::ptr::write_volatile((base + ring::HEAD) as *mut u32, self.head) };
        n
    }

    /// Answer the driver's `COMMIT`, the way a compositor would after compositing.
    pub fn answer_doorbell(&self) {
        let m = crate::sched::ipc_recv_cap(self.doorbell);
        let crate::cap::Object::Reply(caller) = crate::sched::current_cap(m[1])
            .expect("the keyboard driver's ring was not a CALL")
            .object
        else {
            panic!("the keyboard driver rang without a reply capability");
        };
        assert_eq!(
            compositor::proto::op(m[0]),
            compositor::proto::COMMIT,
            "the keyboard driver rang with something other than COMMIT",
        );
        crate::sched::ipc_reply(caller, [0, 0]);
        crate::sched::delete_current_cap(m[1]).expect("consume the one-shot reply");
    }
}
