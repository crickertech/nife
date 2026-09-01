use super::*;
use crate::cap::{Rights, irq_cap, rendezvous_cap, virtio_cap};
use crate::sched::RendezvousId;

/// Where the service maps its DMA page. Must match user/src/entropy.rs.
const DMA_VA: u64 = 0x0000_0000_0090_0000;

/// One page, and no more. The rings take 0x16e of it and the buffer 0x100; a device whose whole
/// job is to write 256 bytes at a time has no business holding a larger grant, and "a device
/// gets the grant it needs and no more" is the standing rule in both directions.
const DMA_PAGE_FRAMES: u64 = 1;

/// What `Spawn::arg0` means. Must match `user/src/entropy.rs`; the kernel's own spawn-argument
/// convention with the one program it spawns (not a wire contract between two user programs, so
/// rule 7 does not apply), the same footing `DMA_VA` above is already on.
const MODE_VIRTIO: u64 = 0;
const MODE_INSTRUCTION: u64 = 1;

/// Which bus to take the RNG from. Both `virt` machines offer both, and the milestone-56 test
/// runs the same binary over each in turn, because a driver that works on one transport and
/// silently not the other is exactly what DECISIONS §18's seam exists to prevent.
///
/// **`Instruction` is not a bus** (milestone 162): `RDSEED`/`RNDRRS` need no MMIO, no PCIe function,
/// no transport at all. It is named here anyway rather than given a parallel enum, because
/// everything this type already exists to do (pick which source `ensure` wires, index the
/// once-per-boot state below, name the source a failure came from) is exactly what an instruction
/// source also needs, and "no bus" is itself the fact worth a reader seeing in the same place the
/// other two sources are. Provisional; flagged for calef the same as the other new names here.
///
/// **`Jh7110` is not a bus either** (milestone 159), and it is here for `Instruction`'s reason
/// rather than by analogy to it: the JH7110's TRNG is a plain MMIO register block on the `SoC`'s
/// own fabric, with no transport to negotiate, no virtqueue, and no PCIe function. Naming it a
/// `Bus` variant buys the same four things the variant above buys (pick which source `ensure`
/// wires, index the once-per-boot state, name the source a failure came from, and reuse
/// [`Wiring`]'s client half unchanged), and the alternative was a parallel enum with a parallel
/// registry saying the same thing twice. Provisional; flagged for calef with the rest.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Bus {
    Mmio,
    Pci,
    Instruction,
    Jh7110,
}

pub struct Wiring {
    /// The service's readiness endpoint, and **only on the call that did the wiring**: the
    /// report is sent once and whoever asked first has taken it.
    pub ready: Option<RendezvousId>,
    /// The request endpoint. **This is the capability a client is given**, with WRITE; the
    /// service holds READ. Nothing about it names the device.
    pub request: RendezvousId,
    /// Which bus this instance took its device from, so a failure names it.
    pub bus: Bus,
    /// True when the device sat behind an IOMMU, which on this machine means the PCIe wiring.
    pub confined_by_iommu: bool,
}

/// **One entropy service per device per boot**, for the same reason the FS service is wired
/// once: a second service on the same device would reset it and reprogram its queue out from
/// under the first, and the first would then wait forever for a completion the device was
/// never told to make. Whoever asks first pays for the wiring and receives the readiness
/// endpoint; later callers get the same request endpoint and `None` for it.
///
/// Plain atomics rather than a lock: the only writer is the boot/test thread that calls this.
static WIRED: [core::sync::atomic::AtomicBool; 4] = [
    core::sync::atomic::AtomicBool::new(false),
    core::sync::atomic::AtomicBool::new(false),
    core::sync::atomic::AtomicBool::new(false),
    core::sync::atomic::AtomicBool::new(false),
];
static REQUEST: [core::sync::atomic::AtomicU64; 4] = [
    core::sync::atomic::AtomicU64::new(0),
    core::sync::atomic::AtomicU64::new(0),
    core::sync::atomic::AtomicU64::new(0),
    core::sync::atomic::AtomicU64::new(0),
];
static CONFINED: [core::sync::atomic::AtomicBool; 4] = [
    core::sync::atomic::AtomicBool::new(false),
    core::sync::atomic::AtomicBool::new(false),
    core::sync::atomic::AtomicBool::new(false),
    core::sync::atomic::AtomicBool::new(false),
];

impl Bus {
    const fn index(self) -> usize {
        match self {
            Bus::Mmio => 0,
            Bus::Pci => 1,
            Bus::Instruction => 2,
            Bus::Jh7110 => 3,
        }
    }
}

/// Wire the entropy service on `bus` if this boot has not already, else hand back what is
/// already running. `None` means there is no virtio-rng function on that bus.
pub fn ensure(image: &'static [u8], bus: Bus) -> Option<Wiring> {
    use core::sync::atomic::Ordering;

    let i = bus.index();
    if WIRED[i].load(Ordering::Acquire) {
        return Some(Wiring {
            ready: None,
            request: REQUEST[i].load(Ordering::Relaxed),
            bus,
            confined_by_iommu: CONFINED[i].load(Ordering::Relaxed),
        });
    }
    let w = start(image, bus)?;
    REQUEST[i].store(w.request, Ordering::Relaxed);
    CONFINED[i].store(w.confined_by_iommu, Ordering::Relaxed);
    WIRED[i].store(true, Ordering::Release);
    Some(w)
}

/// **Wire and spawn the entropy service.** `None` if `bus` has no virtio-rng function on it (or,
/// for [`Bus::Instruction`], if this machine does not implement the instruction).
fn start(image: &'static [u8], bus: Bus) -> Option<Wiring> {
    if bus == Bus::Instruction {
        return start_instruction(image);
    }
    if bus == Bus::Jh7110 {
        return start_jh7110(image);
    }
    // The two buses differ in exactly two things: where the registers are, and whether there is
    // a requester id for the IOMMU to confine. Everything below is shared, which is the §18
    // seam doing its job.
    let (transport, intid, rid) = match bus {
        Bus::Instruction | Bus::Jh7110 => {
            unreachable!("the two early returns above handle these; see start()'s first lines")
        }
        Bus::Mmio => {
            let d = crate::virtio::find_entropy_device()?;
            (
                crate::virtio::Transport::Mmio {
                    mmio_phys: d.mmio_phys,
                },
                d.intid,
                None,
            )
        }
        Bus::Pci => {
            let d = crate::pci::find_rng_device()?;
            (crate::virtio::Transport::pci(&d), d.intid, Some(d.rid))
        }
    };

    // Zeroed so no stale descriptor is visible to the device, and so a buffer the service has not
    // filled yet reads as zeros rather than as somebody's old page contents pretending to be
    // entropy.
    let dma = crate::memory::alloc_contiguous_zeroed(DMA_PAGE_FRAMES as usize)
        .expect("no DMA region for the entropy service")
        .addr();

    let irq_ep = crate::sched::create_rendezvous();
    crate::sched::bind_irq(intid, irq_ep);
    crate::arch::irq::enable(intid);

    let confined_by_iommu = rid.is_some() && crate::iommu::active();
    let vid = crate::virtio::register(transport, dma, DMA_PAGE_FRAMES * FRAME_SIZE, rid);

    let ready = crate::sched::create_rendezvous();
    let request = crate::sched::create_rendezvous();

    let maps = [Mapping {
        va: DMA_VA,
        phys: dma,
        flags: Flags::user_data(),
    }];
    crate::sched::spawn(move || {
        run(
            image,
            Spawn {
                arg0: MODE_VIRTIO,
                arg1: dma, // the DMA region's PHYSICAL base: descriptors speak physical
                arg2: 0,
                grants: &[
                    rendezvous_cap(request, Rights::READ), // slot 0: RECV client requests
                    irq_cap(intid),                        // slot 1: the completion interrupt
                    virtio_cap(vid),                       // slot 2: the confined transport
                    rendezvous_cap(ready, Rights::WRITE),  // slot 3: signal readiness once
                ],
                maps: &maps,
            },
        )
    })
    .expect("could not spawn the entropy service");

    Some(Wiring {
        ready: Some(ready),
        request,
        bus,
        confined_by_iommu,
    })
}

/// **Does this machine implement an instruction source `entropy` can use?** The kernel's call to
/// make, not the process's: `ID_AA64ISAR0_EL1` (aarch64) is not readable from EL0, so `entropy.rs`
/// cannot check `FEAT_RNG` itself, the same way it cannot check whether a `Virtio` capability really
/// names a device; it trusts what it was spawned with.
#[cfg(target_arch = "aarch64")]
fn instruction_backend_available() -> bool {
    crate::arch::isa::get().rndr
}

/// `x86_64`: ring 3 (milestone 161, item 3) and a real userspace on this port both landed on
/// `main` after this function's `x86_64` arm was first written unconditionally `false`; that gap is
/// closed. Same shape as the aarch64 arm above: `CPUID` leaf 7's `EBX` bit 18 is read once at boot
/// (`kernel/src/arch/x86_64/isa.rs::init`) and cached in `Isa`, because `entropy.rs` in userspace
/// has no way to execute `CPUID` and trust the answer the way ring 0 does; it trusts what it was
/// spawned with, same as aarch64's `entropy.rs` trusts `FEAT_RNG`.
#[cfg(target_arch = "x86_64")]
fn instruction_backend_available() -> bool {
    crate::arch::isa::get().rdseed
}

/// riscv64: neither `RDSEED` nor `RNDR`/`RNDRRS` exists on this ISA. Milestone 159's JH7110 TRNG is
/// the real hardware source there, wired through the JH7110 driver rather than through this file.
#[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
fn instruction_backend_available() -> bool {
    false
}

/// **Wire and spawn the entropy service in instruction mode**: no virtio device, no DMA page, no
/// IRQ. `None` if this machine does not implement the instruction its architecture would use
/// (see [`instruction_backend_available`]), the same "no source, no service" answer the virtio path
/// gives for a missing device.
fn start_instruction(image: &'static [u8]) -> Option<Wiring> {
    if !instruction_backend_available() {
        return None;
    }

    let ready = crate::sched::create_rendezvous();
    let request = crate::sched::create_rendezvous();

    crate::sched::spawn(move || {
        run(
            image,
            Spawn {
                arg0: MODE_INSTRUCTION,
                arg1: 0,
                arg2: 0,
                grants: &[
                    rendezvous_cap(request, Rights::READ), // slot 0: RECV client requests
                    rendezvous_cap(ready, Rights::WRITE),  // slot 1: signal readiness once
                ],
                maps: &[],
            },
        )
    })
    .expect("could not spawn the entropy service");

    Some(Wiring {
        ready: Some(ready),
        request,
        bus: Bus::Instruction,
        // Nothing here is a device, so there is nothing an IOMMU could confine. `false` states
        // that rather than leaving the virtio path's field to be misread as "not confined" in the
        // sense that matters for a real device.
        confined_by_iommu: false,
    })
}

/// Where the service maps the TRNG's register page. **Must match `user/src/jh7110_trng.rs`'s
/// `TRNG_VA`**, and deliberately distinct from [`DMA_VA`] so the two entropy backends could be
/// mapped into different processes at once without either constant meaning two things.
const TRNG_VA: u64 = 0x0000_0000_0094_0000;

/// **Does this machine have a JH7110 TRNG?** The device tree's answer, decoded by
/// `jh7110_trng::discover` (the crate that owns the `compatible` string and the `reg` decode, so
/// this file keeps no second copy of either).
///
/// riscv64 only, and that is a statement rather than a shortcut: the JH7110 is a RISC-V `SoC`, so
/// on the other two architectures the honest answer is "no" without reading anything. On riscv64
/// it is "no" too under QEMU's `virt` board, which carries no such node; that is the skip path
/// this tree's whole CI runs through, and `crates/jh7110_trng`'s own
/// `discover_finds_nothing_on_qemus_virt_board` test pins it against the same blob.
#[cfg(target_arch = "riscv64")]
pub fn jh7110_trng_device() -> Option<jh7110_trng::Discovered> {
    jh7110_trng::discover(&crate::device_tree().ok()?)
        .ok()
        .flatten()
}

/// See the riscv64 arm: no JH7110 anywhere but a JH7110.
#[cfg(not(target_arch = "riscv64"))]
pub fn jh7110_trng_device() -> Option<jh7110_trng::Discovered> {
    None
}

/// **Wire and spawn the entropy service against the JH7110's TRNG** (milestone 159): no virtio
/// device, no DMA page, no IRQ, and no transport. `None` when this machine's device tree does not
/// describe the device, which is every machine this repository's CI boots.
///
/// **The authority here is the point, and it is smaller than any other entropy backend's.** The
/// driver is granted two rendezvous capabilities (a request endpoint it RECVs on, a readiness
/// endpoint it SENDs once) and **one page of device memory**: the TRNG's register block, mapped
/// user-device-typed at [`TRNG_VA`]. Not a DMA page, because the device writes nothing to memory;
/// not an `Irq` capability, because the driver polls (`user/src/jh7110_trng.rs` records why);
/// not a `Virtio` capability, because there is no transport. The binding's `reg` window is
/// `0x4000` and this maps `0x1000` of it, since `jh7110_trng::regs` reaches only `0x68`: a driver
/// that cannot name a register cannot touch it.
///
/// **Fatal risk 6's experiment is exactly this shape** (`design/fatal-risks.md`): an unprivileged
/// userspace process, holding one device page and two endpoints, driving a real non-virtio device.
/// Whether it does so at real speed is the part the bench measures, not the part this wires.
fn start_jh7110(image: &'static [u8]) -> Option<Wiring> {
    let device = jh7110_trng_device()?;

    let ready = crate::sched::create_rendezvous();
    let request = crate::sched::create_rendezvous();

    // One page of the register window, device-typed. `map_physical` maps exactly one frame, which
    // is the whole grant: see this function's doc for why 0x1000 of a 0x4000 window is not a
    // rounding error.
    let maps = [Mapping {
        va: TRNG_VA,
        phys: device.reg_base,
        flags: Flags::user_device(),
    }];
    crate::sched::spawn(move || {
        run(
            image,
            Spawn {
                // No role to choose and no address to pass: this program serves one device whose
                // registers it reaches through its own mapping, and it is a different binary from
                // `entropy`, so there is nothing for `arg0`'s mode convention to select between.
                arg0: 0,
                arg1: 0,
                arg2: 0,
                grants: &[
                    rendezvous_cap(request, Rights::READ), // slot 0: RECV client requests
                    rendezvous_cap(ready, Rights::WRITE),  // slot 1: signal readiness once
                ],
                maps: &maps,
            },
        )
    })
    .expect("could not spawn the JH7110 TRNG entropy service");

    Some(Wiring {
        ready: Some(ready),
        request,
        bus: Bus::Jh7110,
        // No PCIe requester id, so nothing an IOMMU could confine, and no DMA for it to confine
        // *of*: this device never touches memory. Same `false` as the instruction path's, for a
        // related reason, and it is not a weaker confinement claim than the virtio path's; it is a
        // narrower attack surface than the one an IOMMU exists to close.
        confined_by_iommu: false,
    })
}

impl Wiring {
    /// Take the startup report: `[READY, first_refill_ok, bytes_in_hand]`, or a `0xDEAD_..`
    /// word whose low byte names the bring-up step that failed. `None` when this caller was not
    /// the one that wired the service, since the report is sent once.
    pub fn wait_for_ready(&self) -> Option<[u64; 5]> {
        self.ready.map(crate::sched::ipc_recv)
    }

    /// **Play a client**: ask for `n` random bytes over the request endpoint and copy out what
    /// arrives. Returns how many landed in `out`. This is the whole of a client's power, and
    /// the kernel deliberately exercises it through the same endpoint a userspace client would
    /// hold rather than reaching into the service.
    pub fn get(&self, n: u64, out: &mut [u8]) -> usize {
        let r =
            crate::sched::ipc_call(self.request, [entropy_proto::req(entropy_proto::GET, n), 0]);
        match entropy_proto::delivered(r[0]) {
            Some(count) => entropy_proto::take(count, r[1], out),
            None => 0,
        }
    }
}
