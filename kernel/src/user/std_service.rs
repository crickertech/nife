use super::*;
use crate::cap::{Rights, page_frame_cap, rendezvous_cap, untyped_cap};
use crate::sched::RendezvousId;

/// **The `std` demo's binary, or `None` because this target has no `std` for it** (milestone 161).
///
/// `std_exerciser` is not built with the rest of `user/`: it is its own workspace, compiled with
/// `-Zbuild-std` for a **custom target** (`aarch64-unknown-nife`, `riscv64-unknown-nife`) against
/// the patched `std` in the `nife-dev` toolchain. There is no `x86_64-unknown-nife` spec and no
/// x86 farm, so the program simply is not in that archive; making one is milestone 27's work.
///
/// A named accessor rather than an `.expect` at nine call sites, for [`super::fs_service::
/// fs_server_image`]'s reason: a fixture missing because of the *toolchain* rather than because of
/// the *machine* deserves to say so once, where a reader will find it.
pub fn std_exerciser_image() -> Option<&'static [u8]> {
    program("std_exerciser")
}

/// The reason a test gives when [`std_exerciser_image`] is `None`.
pub const NO_STD_EXERCISER: &str = "no std_exerciser in this archive: it is built with -Zbuild-std \
                                    for a custom target against the nife-dev toolchain, and there \
                                    is no x86_64-unknown-nife spec or farm yet (milestone 27)";

/// Where the loader maps the clock page for a std program. Must match the std PAL's
/// `rt::CLOCK_PAGE`, and the slot must match its `rt::CLOCK_SLOT`.
const CLOCK_PAGE_STD: u64 = 0x1200_0000;
const CLOCK_SLOT: u64 = 5;

/// Where the loader maps the inert-configuration page for a std program (milestone 47's
/// environment-variable fork, DECISIONS §111). Must match the std PAL's `rt::CONFIG_PAGE`, and
/// the slot must match its `rt::CONFIG_SLOT`. Clear of the clock page above and of the FS
/// contract's shared page (`fs_service::FS_CLIENT_PAGE_VA`, `0x0060_0000`, which this spawn does
/// not use).
const CONFIG_PAGE_STD: u64 = 0x1300_0000;
const CONFIG_SLOT: u64 = 7;

/// The entropy service's request endpoint (milestone 56). Must match the std PAL's
/// `rt::ENTROPY_SLOT`. **An endpoint, and no mapping**: unlike the clock, whose read authority
/// IS a page, randomness is obtained by asking, so the whole grant is one endpoint that names
/// no device.
const ENTROPY_SLOT: u64 = 6;

/// The heap high-water for the demo's Vec/String/HashMap workout plus std's own runtime
/// allocations and the heap's page tables is well under 1 MiB; 256 pages is comfortable, and
/// the initial region only needs to be contiguous at spawn, when memory is unfragmented.
pub const BUDGET_PAGES: u64 = 256;

/// std's startup, formatting machinery, and collection code use far more stack than a
/// hand-written `no_std` worker. `load` maps one stack page; map 32 more below it (128 KiB
/// total), generous so a stack-depth surprise is not what a first std bring-up debugs.
const EXTRA_STACK_PAGES: u64 = 32;

/// Wire a std program and hand back the endpoint it prints on **and the thread it runs as**.
///
/// The tid is what lets a caller wait for the program to be *gone* rather than merely quiet
/// (milestone 64). The transcript ends at `cleanup`, which runs before the process leaves, so a
/// test that stops at the last byte is still racing the exit it wants to make a claim about.
pub fn start(
    image: &'static [u8],
    clock_image: &'static [u8],
    entropy_image: &'static [u8],
) -> (RendezvousId, crate::thread::ThreadId) {
    let report = crate::sched::create_rendezvous();
    let tid = start_on(image, clock_image, entropy_image, report);
    (report, tid)
}

/// The same spawn, with **the output sink chosen by the caller** (milestone 50).
///
/// This split is the milestone's finding expressed as a function signature: everything about a
/// std program's wiring is fixed except one endpoint capability, and putting a different one in
/// slot 1 is the whole of redirection. The program is not told, cannot ask, and the two callers
/// of this function hand it an endpoint the kernel receives on and an endpoint a file sink
/// receives on. See `sink_tests`.
pub fn start_on(
    image: &'static [u8],
    clock_image: &'static [u8],
    entropy_image: &'static [u8],
    report: RendezvousId,
) -> crate::thread::ThreadId {
    let budget = crate::untyped::create(BUDGET_PAGES).expect("no untyped for std_exerciser");

    // The entropy service, wired once per boot and shared with the milestone-56 tests. Its
    // request endpoint is the whole of a std program's randomness authority: `SystemRng` is a
    // `CALL` on it, and nothing about it reaches the device (DECISIONS §44).
    let entropy = entropy_service::ensure(entropy_image, entropy_service::Bus::Mmio)
        .expect("no virtio-rng device for the std program (is NIFE_RNG set on this leg?)");
    if let Some(ready) = entropy.ready {
        let report = crate::sched::ipc_recv(ready);
        assert_eq!(
            report[0],
            entropy_proto::READY,
            "the entropy service did not come up for the std program (it reported {:#x})",
            report[0],
        );
    }

    // The clock first, and its startup report taken before the program starts, so the offset is
    // published by the time std reads the page. Waiting is not a synchronisation trick, it is
    // the honest order: a std program that started first would see `state::UNKNOWN` and be
    // correct to say so.
    let clock = clock_service::start(clock_image);
    let _ = crate::sched::ipc_recv(clock.report);

    // **The inert-configuration page** (milestone 47's environment-variable fork, DECISIONS
    // §111). Unlike the clock, nothing here runs a service: the page is assembled once, into a
    // frame nothing else can see, and only mapped read-only afterward, so there is no seqlock
    // and no readiness handshake to wait on (see `environment_proto`'s own docs for why). The values are
    // the conservative universal defaults ("no clock service configured this program's locale
    // or terminal, so tell it the least assuming thing"), the same posture `date` takes when no
    // clock service is running: an honest baseline rather than a guess. There is no shell here
    // yet to hold a *different* default and pass it explicitly (the "inheritance with
    // visibility" shape the roadmap names); that arrives with whatever program first declares it
    // wants this page through `grant_plan::Manifest`, which none does today.
    let config_bytes = environment_proto::PageBuilder::new()
        .tz("UTC")
        .expect("UTC is not a recognized environment_proto::domain::KNOWN_TZ member")
        .lang("C")
        .expect("C is not a recognized environment_proto::domain::KNOWN_LANG member")
        .term("dumb")
        .expect("dumb is not a recognized environment_proto::domain::KNOWN_TERM member")
        .build();
    let config_phys = crate::memory::alloc()
        .expect("no frame for the std program's config page")
        .addr();
    // SAFETY: fresh frame via the direct map; write the assembled page and zero the remainder of
    // the frame past `PAGE_BYTES`, so nothing left behind by a previous occupant of this physical
    // page is visible through the reserved tail (`ConfigPage` only ever reads the first
    // `PAGE_BYTES`, but a frame's contents are otherwise unspecified until written).
    unsafe {
        let dst = mmu::phys_to_virt(config_phys) as *mut u8;
        core::ptr::write_bytes(dst, 0, FRAME_SIZE as usize);
        core::ptr::copy_nonoverlapping(config_bytes.as_ptr(), dst, config_bytes.len());
    }

    // The clock page and the config page read-only, then the deep stack std needs.
    let mut maps = [Mapping {
        va: 0,
        phys: 0,
        flags: Flags::user_data(),
    }; 2 + EXTRA_STACK_PAGES as usize];
    maps[0] = Mapping {
        va: CLOCK_PAGE_STD,
        phys: clock.page_phys,
        flags: Flags::user_rodata(), // a READER, and the mapping is what says so
    };
    maps[1] = Mapping {
        va: CONFIG_PAGE_STD,
        phys: config_phys,
        flags: Flags::user_rodata(), // same shape as the clock page: a READER, never a writer
    };
    for (k, m) in maps[2..].iter_mut().enumerate() {
        let phys = crate::memory::alloc()
            .expect("no frame for std_exerciser stack")
            .addr();
        // SAFETY: fresh frame via the direct map; zero it so the new process starts clean.
        unsafe {
            core::ptr::write_bytes(mmu::phys_to_virt(phys) as *mut u8, 0, FRAME_SIZE as usize);
        }
        m.va = USER_STACK_VA - (k as u64 + 1) * FRAME_SIZE;
        m.phys = phys;
    }

    crate::sched::spawn(move || {
        // The clock, config and entropy capabilities go in at their named slots BEFORE `run`
        // grants in order, so `run`'s two grants land at 0 and 1 and slots 2 to 4 stay empty.
        // The clock and config pages are `READ` only: the whole point of each is that a reader
        // cannot write it. See `grant_at`.
        crate::sched::grant_at(CLOCK_SLOT, page_frame_cap(clock.page_phys, Rights::READ))
            .expect("the std clock slot was already occupied");
        crate::sched::grant_at(CONFIG_SLOT, page_frame_cap(config_phys, Rights::READ))
            .expect("the std config slot was already occupied");
        crate::sched::grant_at(ENTROPY_SLOT, rendezvous_cap(entropy.request, Rights::WRITE))
            .expect("the std entropy slot was already occupied");
        run(
            image,
            Spawn {
                arg0: 0,
                arg1: 0,
                arg2: 0,
                grants: &[
                    untyped_cap(budget),                   // slot 0: the heap's budget
                    rendezvous_cap(report, Rights::WRITE), // slot 1: stdout/stderr
                ],
                maps: &maps,
            },
        )
    })
    .expect("could not spawn std_exerciser")
}
