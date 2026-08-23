use super::*;
use crate::cap::{Rights, frame_cap, rendezvous_cap, untyped_root_cap};
use crate::sched::{self, RendezvousId};

/// Where the service maps the current client's request. Must match `user/src/login.rs`.
const LOGIN_VA: u64 = 0x0000_0000_00e2_0000;
/// Where the service maps its own request to the credential service. Must match the same file.
const CRED_VA: u64 = 0x0000_0000_00e3_0000;
/// Where a `login_test_client` role stages its own request. Must match
/// `user/src/login_test_client.rs`. A different address space from the service's, so the numeric
/// value colliding with [`LOGIN_VA`] names nothing shared; both processes chose it independently.
const CLIENT_VA: u64 = 0x0000_0000_00e2_0000;

/// The physical frame [`start`] mapped at `LOGIN_VA`, remembered so [`client`] can map the exact
/// same frame at [`CLIENT_VA`] in each role it spawns. Plain atomic rather than a lock: the only
/// writer is [`start`], once, before any client exists (`credential_service.rs`'s own `FRAMES`
/// pattern, simplified to one frame because there is one request page here rather than two).
static LOGIN_FRAME: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(0);

/// Stack pages beyond the one page `run` maps. This process parses the initrd, parses an ELF, and
/// builds a child address space (`supervision_proto::build_child`), which is deeper than
/// `root_supervisor`'s own 8-page stack covers; sized against `credentialer.rs`'s own lesson (its
/// Argon2id inner loop needed 16 pages where one was not close) rather than guessed from nothing.
const LOGIN_STACK_PAGES: u64 = 16;

/// The `login_test_client` roles; must match `user/src/login_test_client.rs`.
pub const ROLE_CHRIS: u64 = 0;
pub const ROLE_CORINNE: u64 = 1;
pub const ROLE_WRONG_SECRET: u64 = 2;
/// DECISIONS §117's per-identity subtree proof; see the same file's module docs.
pub const ROLE_CHRIS_MARK: u64 = 3;
pub const ROLE_CORINNE_MARK: u64 = 4;
pub const ROLE_CHRIS_CHECK: u64 = 5;
/// A real, authenticated identity with no provisioned subtree (`login_tests.rs`'s `wired`
/// deliberately never creates one for `graeme`).
pub const ROLE_NO_SUBTREE: u64 = 6;

/// The report words `login_test_client` sends; must match the same file.
pub const RPT_OK: u64 = login_proto::OK;
pub const RPT_DENIED: u64 = login_proto::DENIED;
#[allow(dead_code)] // named for completeness with the pair above; no role exercises it today
pub const RPT_MALFORMED: u64 = login_proto::MALFORMED;

/// Bits of a successful report's second word; must match the same file.
pub const F_DIR_WORKS: u64 = 1 << 0;
pub const F_BUDGET_WORKS: u64 = 1 << 1;
pub const F_NOT_SHARED_SUBTREE: u64 = 1 << 2;
pub const F_MARKER_WRITTEN: u64 = 1 << 3;

/// A running login service and the endpoints that reach it.
pub struct Wiring {
    /// A client's login request, `WRITE`.
    pub request: RendezvousId,
    /// The verdict and, on success, three delegated capabilities, `READ`.
    pub result: RendezvousId,
    /// One [`login_proto::ATTRIBUTED`] message per successful login, `READ`.
    pub audit: RendezvousId,
}

/// **Wire and spawn the login service.** It parses the initrd for `fs_subtree_caretaker`'s own
/// bytes and then blocks on [`Wiring::request`].
///
/// `verify` is the credential service's verify endpoint (milestone 56), already sealed: login never
/// provisions it and never could. `verify_frame` is the exact physical frame that instance maps at
/// its own `VERIFY_VA` (`credential_service::Wiring::verify_frame` on the instance `verify` came
/// from). `fs_ep`/`fs_frame` are the file service's root directory capability and the page its
/// clients share with it (`fs_service::root_directory`). `construction_pages` bounds how many
/// logins this instance can serve before every further one is answered [`login_proto::DENIED`] (see
/// `user/src/login.rs`'s BUGS: nothing reclaims a caretaker's region in this slice).
///
/// **`verify_frame` is a parameter and not a lookup**, on purpose (milestone 155): a caller that
/// wired more than one credential service in the same boot (as that milestone's own suite does, for
/// a store still open to provision against) cannot ask a bare global "which one," because there is
/// no one answer. Taking the frame from the specific `Wiring` the caller already holds is correct
/// regardless of how many other instances exist or when they were wired.
pub fn start(
    image: &'static [u8],
    verify: RendezvousId,
    verify_frame: u64,
    fs_ep: RendezvousId,
    fs_frame: u64,
    construction_pages: u64,
) -> Wiring {
    let (initrd_start, initrd_len) = memory::initrd_region().expect("no initrd region");
    let initrd_pages = initrd_len.div_ceil(FRAME_SIZE);
    let elf = Elf::parse(image).expect("login is not loadable");

    let content: u64 = elf
        .segments()
        .map(|seg| {
            let (s, e) = seg.page_range(FRAME_SIZE);
            (e - s) / FRAME_SIZE
        })
        .sum::<u64>()
        + 2 // LOGIN_VA, CRED_VA
        + initrd_pages / 512
        + LOGIN_STACK_PAGES
        + 8;
    let mut space = AddressSpace::new(content).expect("no memory for login");
    map_segments(&mut space, &elf).expect("could not lay out login");
    for k in 0..LOGIN_STACK_PAGES {
        space
            .map_new(USER_STACK_VA - k * FRAME_SIZE, Flags::user_data())
            .expect("could not map login's stack");
    }
    for i in 0..initrd_pages {
        space
            .map_physical(
                INITRD_VA + i * FRAME_SIZE,
                initrd_start + i * FRAME_SIZE,
                Flags::user_rodata(),
            )
            .expect("could not map the initrd");
    }
    // **The request page must be the exact physical frame every `login_test_client` role also
    // maps**, the same way `credential_service`'s `VERIFY_VA` frame is shared with its clients:
    // this is the one page a request travels through, so a client writing into a *different* frame
    // would leave this process reading whatever was already at `LOGIN_VA` (zeros, on a fresh
    // frame), not the request that was sent. `LOGIN_FRAME` remembers which physical frame that is,
    // for [`client`] to map at its own end.
    let login_page = crate::memory::alloc()
        .expect("no frame for login's request page")
        .addr();
    LOGIN_FRAME.store(login_page, core::sync::atomic::Ordering::Release);
    // The credential-relay page is a **different** frame from the one above, `credentialer.rs`'s own
    // reason (a provisioner's page and a client's page are never the same frame): it must be the
    // exact physical frame `credential_service` wired the service's own `VERIFY_VA` to, because that
    // is the only page the credential service ever reads a request from. Taken from the caller's own
    // `Wiring` (see this function's own doc) rather than looked up.
    let cred_page = verify_frame;
    // SAFETY: `login_page` is a fresh frame the direct map reaches; zeroing it before mapping is
    // what keeps a first request from reading whatever the allocator's last owner left there.
    // `cred_page` is not zeroed here: it is `credential_service`'s own frame, already live and
    // possibly already in use by another client of the same sealed store.
    unsafe {
        core::ptr::write_bytes(
            mmu::phys_to_virt(login_page) as *mut u8,
            0,
            FRAME_SIZE as usize,
        );
    }
    space
        .map_physical(LOGIN_VA, login_page, Flags::user_data())
        .expect("could not map login's request page");
    space
        .map_physical(CRED_VA, cred_page, Flags::user_data())
        .expect("could not map login's credential-relay page");

    let aspace = readopt_user_aspace(space).expect("register the login aspace");

    let request = sched::create_rendezvous();
    let result = sched::create_rendezvous();
    let audit = sched::create_rendezvous();
    let construction = crate::untyped::create(construction_pages)
        .expect("no construction budget for the login service");

    let tcb_region = crate::untyped::create(2).expect("no tcb region for login");
    let tid = sched::create_tcb(tcb_region).expect("no tcb for login");

    // In `user/src/login.rs`'s own slot order: REQUEST, RESULT, VERIFY, FS_EP, FS_FRAME,
    // CONSTRUCTION_UT, AUDIT. Each `assert_eq!` is that file's own doc read from the other side, the
    // same discipline `authority_tests::spawn_tree` uses for `root_supervisor`.
    let grants: [(&str, crate::cap::Cap); 7] = [
        ("request", rendezvous_cap(request, Rights::READ)),
        (
            "result",
            rendezvous_cap(result, Rights::WRITE.union(Rights::GRANT)),
        ),
        ("verify", rendezvous_cap(verify, Rights::WRITE)),
        (
            "fs_ep",
            rendezvous_cap(fs_ep, Rights::WRITE.union(Rights::GRANT)),
        ),
        (
            "fs_frame",
            // GRANT as well as READ|WRITE: `user/src/login.rs` both maps this frame into every
            // caretaker it builds (`MAP_INTO`, which only checks WRITE) and delegates it directly
            // to every authenticated client (`SEND_CAP`, which needs GRANT on the capability being
            // sent). The second use is why this differs from `credential_service.rs`'s own frames,
            // which are never delegated onward.
            frame_cap(
                fs_frame,
                Rights::READ.union(Rights::WRITE).union(Rights::GRANT),
            ),
        ),
        ("construction", untyped_root_cap(construction)),
        ("audit", rendezvous_cap(audit, Rights::WRITE)),
    ];
    for (i, (name, cap)) in grants.into_iter().enumerate() {
        let slot =
            sched::tcb_insert_cap(tid, cap, None).unwrap_or_else(|_| panic!("insert {name}"));
        assert_eq!(slot, i as u64, "login's {name} must land in slot {i}");
    }

    sched::configure_tcb(tid, elf.entry(), USER_STACK_TOP, aspace).expect("configure");
    sched::start_tcb(tid, [0, initrd_len, 0]).expect("start");

    Wiring {
        request,
        result,
        audit,
    }
}

/// **Spawn a `login_test_client` role** against `w`, and return its report.
pub fn client(image: &'static [u8], w: &Wiring, role: u64) -> [u64; 5] {
    let report = sched::create_rendezvous();
    let phys = LOGIN_FRAME.load(core::sync::atomic::Ordering::Acquire);
    assert_ne!(phys, 0, "the login service was not wired before a client");
    let maps = [Mapping {
        va: CLIENT_VA,
        phys,
        flags: Flags::user_data(),
    }];
    // Copied out of `w` rather than captured by reference: the spawned closure must be `'static`,
    // and an `RendezvousId` is a plain integer with nothing left to borrow once it is in hand.
    let (request, result) = (w.request, w.result);
    sched::spawn(move || {
        run(
            image,
            Spawn {
                arg0: role,
                arg1: 0,
                arg2: 0,
                grants: &[
                    rendezvous_cap(request, Rights::WRITE),
                    rendezvous_cap(result, Rights::READ),
                    rendezvous_cap(report, Rights::WRITE),
                ],
                maps: &maps,
            },
        )
    })
    .expect("could not spawn a login_test_client");
    sched::ipc_recv(report)
}
