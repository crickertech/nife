//! **The display seam's wiring**: a confined virtio-gpu driver, the client that paints through it,
//! and the terminal that took the client's place (milestones 29, 33; notes/framebuffer-contract.md).
//!
//! # The pixels are capabilities now (milestone 108, notes/frames.md)
//!
//! Everything here used to arrive as `Spawn::maps`: the kernel wrote the DMA region into the
//! driver's address space and the surface into the client's, at addresses the kernel picked, before
//! either program's first instruction. Nothing in either capability table said the memory was there, and
//! nothing could narrow it, hand it on, or take it back.
//!
//! Each program now holds its pages as `PageFrame` capabilities and maps them itself, out of an untyped
//! it also holds.
//!
//! # One capability per run, not one per page (DECISIONS §102, milestone 142)
//!
//! `PageFrame` used to name exactly one page, so the driver's DMA region (rings and control
//! buffers, then the surface) was one capability per page: at the 128x64 scanout this milestone
//! grew from, that was 9 capabilities in 9 consecutive slots of a sixteen-slot capability table,
//! with one slot spare and no room for anything larger. §102 gave `PageFrame` a page count, so a
//! whole run is now **one capability and one `MAP` call**, however many pages it holds. That is
//! what makes the grown scanout ([`graphics_proto::WIDTH`] x [`graphics_proto::HEIGHT`],
//! [`graphics_proto::SURFACE_PAGE_FRAMES`] page frames) fit at all: that many one-page capabilities
//! never would have, whether the surface is the 900-frame 1280x720 this milestone first grew to or
//! the 311-frame 924x344 it was retargeted to on 2026-08-27. See notes/frames.md.
//!
//! What stays wired at spawn is nothing at all here: these programs have no extra stack pages, so
//! the only page the kernel still places is the one `load` gives every process.

use super::*;
use crate::cap::{
    Rights, irq_cap, memory_region_cap, page_frame_run_cap, rendezvous_cap, virtio_cap,
};
use crate::sched::RendezvousId;

/// The DMA region, in frames: one for the rings and control buffers, then the surface.
const DMA_PAGE_FRAMES: u64 = 1 + graphics_proto::SURFACE_PAGE_FRAMES as u64;

/// The driver binary's escape-attempt role; must match user/src/display.rs `ROLE_BACKING_ESCAPE`.
const ROLE_BACKING_ESCAPE: u64 = 1;

/// **The budget every program on this path draws its page tables from** (milestone 108, widened for
/// milestone 142's larger scanout, then not shrunk back when the scanout was retargeted). At 128x64
/// the driver's whole DMA region and the terminal's surface each fit one 2 MiB window (one L3
/// table); at 1280x720 the 900-page surface alone spanned two windows, and the terminal's separate
/// output page a third, so more than one L3 was briefly the ordinary case rather than the edge case
/// notes/frames.md recorded at 800x608. **At 924x344 (retargeted 2026-08-27) the surface is back
/// under 512 pages** (311, [`graphics_proto::SURFACE_PAGE_FRAMES`]), one 2 MiB window again, the
/// same shape 128x64 had. Twenty-four pages is more headroom than that now strictly needs, kept
/// rather than re-tuned down: still under one percent of the free pool, and a budget this path has
/// already exercised at the larger size is the safer one to keep than a smaller number nobody has
/// booted against.
const MAP_BUDGET_PAGES: u64 = 24;

// The driver's capability table. Must match user/src/display.rs.
const DRIVER_SLOT_REPORT: u64 = 0;
const DRIVER_SLOT_IRQ: u64 = 1;
const DRIVER_SLOT_VIRTIO: u64 = 2;
const DRIVER_SLOT_DISPLAY: u64 = 3;
const DRIVER_SLOT_BUDGET: u64 = 4;
/// **The whole DMA region, one capability** (§102). Before the widening this was
/// [`DMA_PAGE_FRAMES`] separate slots, one per page, and a `const` assertion guarded the last one
/// against the fault slot; a scanout large enough to need this milestone would have failed that
/// assertion outright. One slot names the whole run now, so the assertion (and the pressure it
/// guarded against) is retired. See notes/frames.md's BUGS.
const DRIVER_SLOT_DMA: u64 = 5;

// The painting client's capability table. Must match user/src/painter.rs.
const CLIENT_SLOT_REPORT: u64 = 0;
const CLIENT_SLOT_DISPLAY: u64 = 1;
const CLIENT_SLOT_BUDGET: u64 = 2;
/// The whole scanout, one capability (§102).
const CLIENT_SLOT_SURFACE: u64 = 3;

// The display terminal's capability table. Must match user/src/display_terminal.rs.
const TERM_SLOT_REPORT: u64 = 0;
const TERM_SLOT_DISPLAY: u64 = 1;
const TERM_SLOT_TERM: u64 = 2;
const TERM_SLOT_BUDGET: u64 = 3;
/// The whole scanout, one capability (§102).
const TERM_SLOT_SURFACE: u64 = 4;
/// The page an application writes text into. Still its own single-page capability: it is not part
/// of the scanout's contiguous run.
const TERM_SLOT_OUT: u64 = TERM_SLOT_SURFACE + 1;

/// Grant one capability naming the `count`-frame run at `base`, read/write, at `slot`. The
/// counterpart of the single `MAP` call each of these programs now makes at startup for the run
/// (§102): before this widening a run was `count` separate slots and `count` separate `MAP` calls.
fn grant_run(slot: u64, base: u64, count: u64, what: &str) {
    crate::sched::grant_at(
        slot,
        page_frame_run_cap(base, count, Rights::READ.union(Rights::WRITE)),
    )
    .unwrap_or_else(|_| panic!("{what}: slot {slot} was occupied"));
}

/// **Wire and spawn the display driver and the painting client.** Returns
/// `(driver report, client report)`, or `None` if no virtio-gpu function is on the bus.
///
/// One spawn site for two processes on purpose: they are only meaningful together (a driver with
/// no client serves nobody, a client with no driver blocks on its first CALL), and the endpoint
/// and the shared frames that join them are created here, in the one place that is allowed to
/// know both halves.
pub fn start(
    driver_image: &'static [u8],
    client_image: &'static [u8],
) -> Option<(RendezvousId, RendezvousId)> {
    let (driver_report, display_ep, surface) = wire_driver(driver_image, 0, 0)?;

    // --- the client: an endpoint and the pixels. Nothing else, which is the point. ---
    let client_report = crate::sched::create_rendezvous();
    let budget =
        crate::memory_region::create(MAP_BUDGET_PAGES).expect("no map budget for the client");
    crate::sched::spawn(move || {
        crate::sched::grant_at(
            CLIENT_SLOT_REPORT,
            rendezvous_cap(client_report, Rights::WRITE),
        )
        .expect("client slot 0 was occupied");
        crate::sched::grant_at(
            CLIENT_SLOT_DISPLAY,
            rendezvous_cap(display_ep, Rights::WRITE),
        )
        .expect("client slot 1 was occupied");
        crate::sched::grant_at(CLIENT_SLOT_BUDGET, memory_region_cap(budget))
            .expect("client slot 2 was occupied");
        grant_run(
            CLIENT_SLOT_SURFACE,
            surface,
            graphics_proto::SURFACE_PAGE_FRAMES as u64,
            "the painting client",
        );
        run(
            client_image,
            Spawn {
                arg0: 0,
                arg1: 0, // no physical address: a client has no business knowing one
                arg2: 0,
                grants: &[], // every one of them is placed above, at its own slot
                maps: &[],
            },
        )
    })
    .expect("could not spawn the painting client");

    Some((driver_report, client_report))
}

/// **Spawn a driver that attacks its own confinement** (user/src/display.rs `run_backing_escape`):
/// it asks the device to read pixels out of a frame outside its grant. Returns
/// `(report endpoint, the victim frame's physical address)`, or `None` if no GPU is on the bus.
///
/// It gets exactly the honest driver's world, no more: the same confined transport, the same
/// region, the same interrupt. That is what makes it a fair test of the barrier rather than of a
/// missing capability. No client, because it never serves one.
///
/// The **kernel** picks the victim frame and hands it over in `arg2`, the same way milestone 16b's
/// confinement test picks its own escape frame: the caller has to know the exact address to look
/// for in the IOMMU's fault queue, and a driver guessing at "the frame past my region" guesses
/// wrong (the shadow page is allocated right after it, and that frame IS in the domain). The frame
/// is deliberately never freed: it is an escape target, and handing it back to the allocator while
/// a device has been told to read it is the use-after-free-by-hardware notes/dma.md warns about.
pub fn start_backing_escape(driver_image: &'static [u8]) -> Option<(RendezvousId, u64)> {
    let victim = crate::memory::alloc()
        .expect("no victim frame for the backing-escape test")
        .addr();
    let (report, _, _) = wire_driver(driver_image, ROLE_BACKING_ESCAPE, victim)?;
    Some((report, victim))
}

/// The shared half of both spawns: find the GPU, build the DMA region, route the interrupt,
/// register the confined transport, and spawn `driver_image` at `role` with `arg2`. Returns
/// `(report endpoint, display endpoint, the surface's physical base)`.
fn wire_driver(
    driver_image: &'static [u8],
    role: u64,
    arg2: u64,
) -> Option<(RendezvousId, RendezvousId, u64)> {
    let d = crate::pci::find_gpu_device()?;

    // The DMA region: contiguous, because the surface must be one run of physical frames for the
    // device's backing to be a single memory entry and for the IOMMU domain to cover it as one
    // range. Zeroed, so neither a stale descriptor nor a stale pixel is ever visible to the
    // device or to the client.
    let dma = crate::memory::alloc_contiguous(DMA_PAGE_FRAMES as usize)
        .expect("no contiguous DMA region for the display driver")
        .addr();
    // SAFETY: a fresh contiguous run of frames, reachable through the direct map, owned by
    // nobody else.
    unsafe {
        core::ptr::write_bytes(
            mmu::phys_to_virt(dma) as *mut u8,
            0,
            (DMA_PAGE_FRAMES * FRAME_SIZE) as usize,
        );
    }
    let surface = dma + FRAME_SIZE; // page 1 onward: the frames the client also maps

    // The device's interrupt, routed to an endpoint so the driver's WAIT receives it as a
    // message (milestone 9a).
    let irq_ep = crate::sched::create_rendezvous();
    crate::sched::bind_irq(d.intid, irq_ep);
    crate::arch::irq::enable(d.intid);

    // Register the transport: the kernel keeps the registers and the two DMA-critical powers,
    // and confines the device in hardware to exactly this region plus the shadow page.
    let vid = crate::virtio::register(
        crate::virtio::Transport::pci(&d),
        dma,
        DMA_PAGE_FRAMES * FRAME_SIZE,
        Some(d.rid), // the PCIe requester id the IOMMU keys its tables on
    );

    let display_ep = crate::sched::create_rendezvous(); // client WRITE (CALL) -> driver READ
    let driver_report = crate::sched::create_rendezvous();

    // --- the driver: the confined transport, the interrupt, the whole DMA region, and the
    // display endpoint's serving half. The region is one `PageFrame` capability naming the whole
    // DMA_PAGE_FRAMES-page run (§102; see [`DRIVER_SLOT_DMA`]). ---
    let budget =
        crate::memory_region::create(MAP_BUDGET_PAGES).expect("no map budget for the driver");
    let intid = d.intid;
    crate::sched::spawn(move || {
        crate::sched::grant_at(
            DRIVER_SLOT_REPORT,
            rendezvous_cap(driver_report, Rights::WRITE),
        )
        .expect("driver slot 0 was occupied");
        // The completion IRQ.
        crate::sched::grant_at(DRIVER_SLOT_IRQ, irq_cap(intid))
            .expect("driver slot 1 was occupied");
        // The confined transport.
        crate::sched::grant_at(DRIVER_SLOT_VIRTIO, virtio_cap(vid))
            .expect("driver slot 2 was occupied");
        // Serve clients.
        crate::sched::grant_at(
            DRIVER_SLOT_DISPLAY,
            rendezvous_cap(display_ep, Rights::READ),
        )
        .expect("driver slot 3 was occupied");
        crate::sched::grant_at(DRIVER_SLOT_BUDGET, memory_region_cap(budget))
            .expect("driver slot 4 was occupied");
        grant_run(DRIVER_SLOT_DMA, dma, DMA_PAGE_FRAMES, "the display driver");
        run(
            driver_image,
            Spawn {
                arg0: role,  // 0 = the display driver; 1 = the escape attempt
                arg1: dma,   // the DMA region's PHYSICAL base: descriptors speak physical
                arg2,        // the escape role's victim frame; unused (0) by the display driver
                grants: &[], // every one of them is placed above, at its own slot
                maps: &[],
            },
        )
    })
    .expect("could not spawn the display driver");

    Some((driver_report, display_ep, surface))
}

/// **Wire and spawn the display driver alone**, with no client: `(report endpoint, display
/// endpoint, the surface's physical base)`, or `None` if no virtio-gpu is on the bus.
///
/// For rung two (milestone 33). The compositor takes `painter`'s place at this seam exactly as the
/// contract promised it would, so what it needs from rung one is a display endpoint to CALL and the
/// frames the device scans out. Nothing about the driver changes, which is the claim
/// notes/framebuffer-contract.md made when it said routing was by endpoint.
pub fn start_driver(driver_image: &'static [u8]) -> Option<(RendezvousId, RendezvousId, u64)> {
    wire_driver(driver_image, 0, 0)
}

/// What the kernel keeps after wiring a display terminal onto the scanout.
pub struct TerminalWiring {
    /// The display driver's status endpoint.
    pub driver_report: RendezvousId,
    /// The terminal's status endpoint.
    pub term_report: RendezvousId,
    /// The endpoint the terminal serves. The kernel holds WRITE, so it can play **both** classes
    /// of sender: an application (`OP_WRITE`) and an input source (`OP_BYTES`).
    pub term: RendezvousId,
    /// The application's output page, so the kernel can put the bytes of an `OP_WRITE` there.
    pub out: u64,
    /// The scanout frames, so the kernel can read the picture back through the direct map and
    /// grade it against a value it computed itself.
    pub surface: u64,
}

/// **Wire and spawn the display driver with a terminal on the whole scanout** (milestone 29's
/// remaining increment). `None` if no virtio-gpu function is on the bus.
///
/// The terminal takes `painter`'s place at the display seam with **exactly `painter`'s
/// authority**: a report endpoint, the display endpoint, and the surface frames. It holds no
/// device, no interrupt, and no physical address, and `display` cannot tell it from the client that
/// drew a test pattern. That is the answer to "did the framebuffer contract need changing to
/// carry text?", and it is an answer made of a spawn literal rather than an argument.
///
/// What it adds over `painter`'s wiring is two things, and both are the terminal contract's, not
/// the framebuffer's: an endpoint it **serves** (the terminal contract's IPC half), and a page an
/// application writes bytes into (DECISIONS §10's control-by-message, bulk-by-shared-page split).
pub fn start_terminal(
    driver_image: &'static [u8],
    term_image: &'static [u8],
) -> Option<TerminalWiring> {
    // A scanout with no room for a character has nothing to show. It does **not** have to be a
    // whole number of them: 128 is not a multiple of the font's 7-pixel cell, so the ordinary case
    // leaves a two-pixel strip on the right that the terminal paints as background on its first
    // frame (see `user/src/display_terminal.rs`) and that `Vt::pixel` answers for, which is what
    // keeps the picture a total function of the state.
    const _: () = assert!(
        graphics_proto::WIDTH >= bitmap_font::GLYPH_W
            && graphics_proto::HEIGHT >= bitmap_font::GLYPH_H,
        "the scanout is too small for one character cell",
    );
    // And the script's geometry is the scanout's, checked here rather than trusted, because the
    // script is what three independent parties predict the picture from.
    const _: () = assert!(
        graphics_proto::WIDTH / bitmap_font::GLYPH_W == video_terminal::script::COLS
            && graphics_proto::HEIGHT / bitmap_font::GLYPH_H == video_terminal::script::ROWS,
        "video_terminal::script's geometry and the scanout's have drifted apart",
    );

    let (driver_report, display_ep, surface) = wire_driver(driver_image, 0, 0)?;

    let out = crate::memory::alloc()
        .expect("no output-page frame for the display terminal")
        .addr();
    // SAFETY: a fresh frame, direct-mapped, owned by nobody yet.
    unsafe {
        core::ptr::write_bytes(mmu::phys_to_virt(out) as *mut u8, 0, FRAME_SIZE as usize);
    };

    let term_report = crate::sched::create_rendezvous();
    let term = crate::sched::create_rendezvous();
    let budget =
        crate::memory_region::create(MAP_BUDGET_PAGES).expect("no map budget for the terminal");

    crate::sched::spawn(move || {
        crate::sched::grant_at(TERM_SLOT_REPORT, rendezvous_cap(term_report, Rights::WRITE))
            .expect("terminal slot 0 was occupied");
        // CALL the driver.
        crate::sched::grant_at(TERM_SLOT_DISPLAY, rendezvous_cap(display_ep, Rights::WRITE))
            .expect("terminal slot 1 was occupied");
        // Serve the terminal.
        crate::sched::grant_at(TERM_SLOT_TERM, rendezvous_cap(term, Rights::READ))
            .expect("terminal slot 2 was occupied");
        crate::sched::grant_at(TERM_SLOT_BUDGET, memory_region_cap(budget))
            .expect("terminal slot 3 was occupied");
        grant_run(
            TERM_SLOT_SURFACE,
            surface,
            graphics_proto::SURFACE_PAGE_FRAMES as u64,
            "the display terminal",
        );
        // The page an application writes text into.
        grant_run(TERM_SLOT_OUT, out, 1, "the display terminal");
        run(
            term_image,
            Spawn {
                arg0: video_terminal::status::MODE_DISPLAY,
                arg1: 0, // no physical address: a terminal has no business knowing one
                arg2: 0,
                grants: &[], // every one of them is placed above, at its own slot
                maps: &[],
            },
        )
    })
    .expect("could not spawn the display terminal");

    Some(TerminalWiring {
        driver_report,
        term_report,
        term,
        out,
        surface,
    })
}

impl TerminalWiring {
    /// **Play the application**: put `text` in the output page and `OP_WRITE` it.
    ///
    /// Returns when the terminal has drawn it and the display driver has put it on the scanout,
    /// because that is what the terminal contract says an `OP_WRITE` reply means (the bytes are
    /// on the console's side). So a test needs no polling and no sleep between writes.
    pub fn print(&self, text: &[u8]) {
        super::term_print(self.out, self.term, text);
    }

    /// **Play the input driver**: `OP_BYTES` these keystrokes, eight to a message.
    ///
    /// Byte for byte the framing `user/src/input.rs` sends and the compositor forwards
    /// (DECISIONS §33), which is the point: the display terminal is fed by the same driver half
    /// as the serial one, so neither contract had to grow anything to carry a keystroke to a
    /// screen.
    pub fn type_bytes(&self, bytes: &[u8]) {
        for chunk in bytes.chunks(8) {
            let mut w1 = 0u64;
            for (k, &b) in chunk.iter().enumerate() {
                w1 |= (b as u64) << (8 * k);
            }
            let w0 = line_editor::proto::req(line_editor::proto::OP_BYTES, chunk.len() as u64);
            assert_eq!(
                crate::sched::ipc_call(self.term, [w0, w1])[0],
                0,
                "the terminal refused a keystroke",
            );
        }
    }

    /// A scanout pixel, read by the **kernel** through the direct map: a witness that belongs to
    /// no process in userspace.
    pub fn screen_pixel(&self, x: u32, y: u32) -> u32 {
        let at = mmu::phys_to_virt(self.surface) + (y * graphics_proto::WIDTH + x) as u64 * 4;
        // SAFETY: inside the scanout frames this kernel allocated.
        unsafe { core::ptr::read_volatile(at as *const u32) }
    }

    /// **The scanout holds exactly the picture `expect` describes.** Compared pixel for pixel
    /// rather than by digest, so a failure names a coordinate.
    pub fn assert_screen_is(&self, expect: &video_terminal::Vt, what: &str) {
        for y in 0..graphics_proto::HEIGHT {
            for x in 0..graphics_proto::WIDTH {
                let (got, want) = (self.screen_pixel(x, y), expect.pixel(x, y));
                assert_eq!(
                    got,
                    want,
                    "{what}: the framebuffer is wrong at ({x},{y}) [cell ({},{})]: {got:#010x}, \
                     expected {want:#010x}",
                    x / bitmap_font::GLYPH_W,
                    y / bitmap_font::GLYPH_H,
                );
            }
        }
    }
}
