use super::*;
use crate::cap::{Rights, memory_region_root_cap, page_frame_cap, rendezvous_cap};
use crate::sched::{self, RendezvousId};

/// Where the service maps its own request to the credential service. Must match `user/src/login.rs`.
const CRED_VA: u64 = 0x0000_0000_00e3_0000;

/// How many pages a spawned `login_test_client` role's own `memory_region_cap` (slot 3) holds:
/// enough for `page_frame::MAP`'s own page-table cost when it self-maps the frame `login`'s
/// `CONNECT` step delegates (milestone 49's channel-per-client update; `user/src/login_test_client.rs`
/// mirrors this program's own post-auth `map_page_frame(fs_page_frame, FS_VA, true, budget)`, but a
/// role holds no budget yet at the point it must map its own connect channel). Margin over the one
/// page a fresh mapping ever strictly needs, on this file's own existing style for every other
/// region here.
///
/// # BUGS
///
/// **Nothing reclaims one of these when its role exits**, so a full aarch64 suite leaves thirty of
/// them (120 frames) held for the rest of the boot (milestone 49's terminal update added four more:
/// `login_hands_out_the_terminal_once_and_denies_a_concurrent_second_login_until_logout`'s own
/// `ROLE_TERM_FIRST` x2, `ROLE_TERM_SECOND`, `ROLE_TERM_LOGOUT`), which is a measured line item in
/// `kernel::testing::SUITE_PAGE_FRAME_BUDGET`'s own account. This is scaffolding rather than a
/// property under test, and `kernel::user::holding::Holding` is the mechanism that would give it
/// back; what stops it being a two-line change is that a role's scratch pays for **page tables** in
/// that role's own address space rather than for anything the role holds a capability to, so
/// destroying the region frees tables the dying process is still walking. Doing this properly means
/// reclaiming the role's whole address space first (`Holding::add_region_after_death`), which needs
/// [`spawn_client`] to hand its caller the thread id it currently drops.
const CLIENT_SCRATCH_UT_PAGES: u64 = 4;

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
/// Logs in, then tears the session down with the fourth delegated capability and proves the
/// directory came down with it. See the same file's module docs.
pub const ROLE_LOGOUT: u64 = 7;
/// Milestone 49's terminal update: logs in, proves the fifth delegated capability (the terminal)
/// works, tears the session down without freeing the terminal. See the same file's module docs.
pub const ROLE_TERM_FIRST: u64 = 8;
/// A real credential presented while [`ROLE_TERM_FIRST`]'s terminal loan is outstanding.
pub const ROLE_TERM_SECOND: u64 = 9;
/// Sends `login_proto::logout_word` on the front door directly.
pub const ROLE_TERM_LOGOUT: u64 = 10;

/// The report words `login_test_client` sends; must match the same file.
pub const RPT_OK: u64 = login_proto::OK;
pub const RPT_DENIED: u64 = login_proto::DENIED;
#[allow(dead_code)] // named for completeness with the pair above; no role exercises it today
pub const RPT_MALFORMED: u64 = login_proto::MALFORMED;
/// Milestone 49's terminal update: the terminal was already on loan.
pub const RPT_NO_TERMINAL: u64 = login_proto::NO_TERMINAL;
/// [`ROLE_TERM_LOGOUT`]'s own answer.
pub const RPT_LOGGED_OUT: u64 = login_proto::LOGGED_OUT;

/// [`ROLE_TERM_FIRST`]'s proof-of-life word for the delegated terminal; must match the same file's
/// `TERM_MAGIC`.
pub const TERM_MAGIC: u64 = 0x_7e12_0000_0000_0001;

/// Bits of a successful report's second word; must match the same file.
pub const F_DIR_WORKS: u64 = 1 << 0;
pub const F_BUDGET_WORKS: u64 = 1 << 1;
pub const F_NOT_SHARED_SUBTREE: u64 = 1 << 2;
pub const F_MARKER_WRITTEN: u64 = 1 << 3;
pub const F_TEARDOWN_OK: u64 = 1 << 4;
pub const F_DEAD_AFTER_TEARDOWN: u64 = 1 << 5;
pub const F_BUDGET_TEARDOWN_OK: u64 = 1 << 6;
pub const F_BUDGET_DEAD_AFTER_TEARDOWN: u64 = 1 << 7;
/// Milestone 49's terminal update: the fifth delegated capability delivered [`TERM_MAGIC`] to a
/// real receiver. Set only by [`ROLE_TERM_FIRST`].
pub const F_TERM_WORKS: u64 = 1 << 8;

/// A running login service and the endpoints that reach it.
pub struct Wiring {
    /// A client's login request, `WRITE`.
    pub request: RendezvousId,
    /// The verdict and, on success, five delegated capabilities, `READ`.
    pub result: RendezvousId,
    /// One [`login_proto::ATTRIBUTED`] message per successful login, `READ`.
    pub audit: RendezvousId,
    /// **The stand-in terminal** (milestone 49's terminal update): this test harness holds no real
    /// terminal to grant, so it wires a bare rendezvous in its place, `READ`. A test can `ipc_recv`
    /// here to confirm a delegated `TERM_EP` copy actually names this object (real communication,
    /// not merely "a capability arrived"), the same "prove it works, not merely that it arrived"
    /// standard this file's own module doc already sets for the directory and the budget.
    pub term_ep: RendezvousId,
}

/// **Wire and spawn the login service.** It parses the initrd for `fs_subtree_caretaker`'s own
/// bytes and then blocks on [`Wiring::request`].
///
/// `verify` is the credential service's verify endpoint (milestone 56), already sealed: login never
/// provisions it and never could. `verify_page_frame` is the exact physical frame that instance maps at
/// its own `VERIFY_VA` (`credential_service::Wiring::verify_page_frame` on the instance `verify` came
/// from). `fs_ep`/`fs_page_frame` are the file service's root directory capability and the page its
/// clients share with it (`fs_service::root_directory`). `construction_pages` bounds how many
/// logins this instance can serve before every further one is answered [`login_proto::DENIED`] (see
/// `user/src/login.rs`'s BUGS: nothing reclaims a caretaker's region in this slice).
///
/// **`verify_page_frame` is a parameter and not a lookup**, on purpose (milestone 155): a caller that
/// wired more than one credential service in the same boot (as that milestone's own suite does, for
/// a store still open to provision against) cannot ask a bare global "which one," because there is
/// no one answer. Taking the frame from the specific `Wiring` the caller already holds is correct
/// regardless of how many other instances exist or when they were wired.
pub fn start(
    image: &'static [u8],
    verify: RendezvousId,
    verify_page_frame: u64,
    fs_ep: RendezvousId,
    fs_page_frame: u64,
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
        + 1 // CRED_VA
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
    #[cfg(target_arch = "x86_64")]
    map_x86_timebase_page(&mut space).expect("could not map login's timebase page");
    for i in 0..initrd_pages {
        space
            .map_physical(
                INITRD_VA + i * FRAME_SIZE,
                initrd_start + i * FRAME_SIZE,
                Flags::user_rodata(),
            )
            .expect("could not map the initrd");
    }
    // Milestone 49's channel-per-client update removed the front door's own shared staging page:
    // `CONNECT` (the only word the front door accepts) carries no page at all, and every actual
    // login's identity and secret now travel on a page `login`'s own `connect()` mints and maps at
    // runtime, private to the one client it was minted for. Only the credential-relay page below is
    // still wired here, statically, because it must be the exact frame `credential_service` itself
    // reads from.
    //
    // The credential-relay page: it must be the exact physical frame `credential_service` wired the
    // service's own `VERIFY_VA` to, because that is the only page the credential service ever reads
    // a request from. Taken from the caller's own `Wiring` (see this function's own doc) rather than
    // looked up.
    let cred_page = verify_page_frame;
    space
        .map_physical(CRED_VA, cred_page, Flags::user_data())
        .expect("could not map login's credential-relay page");

    let aspace = readopt_user_address_space(space).expect("register the login aspace");

    let request = sched::create_rendezvous();
    let result = sched::create_rendezvous();
    let audit = sched::create_rendezvous();
    // The stand-in terminal (milestone 49's terminal update); see `Wiring::term_ep`'s own doc.
    let term_ep = sched::create_rendezvous();
    let construction = crate::memory_region::create(construction_pages)
        .expect("no construction budget for the login service");

    let thread_control_block_region =
        crate::memory_region::create(2).expect("no tcb region for login");
    let tid =
        sched::create_thread_control_block(thread_control_block_region).expect("no tcb for login");

    // In `user/src/login.rs`'s own slot order: REQUEST, RESULT, VERIFY, FS_EP, FS_PAGE_FRAME,
    // CONSTRUCTION_UT, AUDIT, TERM_EP. Each `assert_eq!` inside `grant_in_order` is that file's
    // own doc read from the other side, the same discipline `authority_tests::spawn_tree` uses for
    // `root_supervisor`.
    //
    // Granted one at a time rather than collected into a `[(&str, Cap); 8]` first: milestone 49's
    // terminal update took that array to eight entries and `start`'s frame to 4288 bytes, over the
    // 4096-byte guard page `script/stack-frame-check` gates against, because an unoptimised build
    // materialises the whole table plus a temporary per element before the first insert ever runs.
    // The array only ever existed to pair each name with its slot index, and the counter below
    // pairs them just as tightly while nothing but one capability is ever live at once: 1632 bytes
    // on aarch64, 1648 on riscv64, measured the same way the gate measures.
    let mut next_slot = 0u64;
    let mut grant_in_order = |name: &str, cap: crate::cap::Cap| {
        let slot = sched::thread_control_block_insert_cap(tid, cap, None)
            .unwrap_or_else(|_| panic!("insert {name}"));
        assert_eq!(
            slot, next_slot,
            "login's {name} must land in slot {next_slot}"
        );
        next_slot += 1;
    };
    grant_in_order("request", rendezvous_cap(request, Rights::READ));
    grant_in_order(
        "result",
        rendezvous_cap(result, Rights::WRITE.union(Rights::GRANT)),
    );
    grant_in_order("verify", rendezvous_cap(verify, Rights::WRITE));
    grant_in_order(
        "fs_ep",
        rendezvous_cap(fs_ep, Rights::WRITE.union(Rights::GRANT)),
    );
    grant_in_order(
        "fs_page_frame",
        // GRANT as well as READ|WRITE: `user/src/login.rs` both maps this frame into every
        // caretaker it builds (`MAP_INTO`, which only checks WRITE) and delegates it directly
        // to every authenticated client (`SEND_CAP`, which needs GRANT on the capability being
        // sent). The second use is why this differs from `credential_service.rs`'s own frames,
        // which are never delegated onward.
        page_frame_cap(
            fs_page_frame,
            Rights::READ.union(Rights::WRITE).union(Rights::GRANT),
        ),
    );
    grant_in_order("construction", memory_region_root_cap(construction));
    grant_in_order("audit", rendezvous_cap(audit, Rights::WRITE));
    grant_in_order(
        "term_ep",
        rendezvous_cap(term_ep, Rights::WRITE.union(Rights::GRANT)),
    );
    assert_eq!(next_slot, 8, "login must hold exactly eight capabilities");

    sched::configure_thread_control_block(tid, elf.entry(), USER_STACK_TOP, aspace)
        .expect("configure");
    sched::start_thread_control_block(tid, [0, initrd_len, 0]).expect("start");

    Wiring {
        request,
        result,
        audit,
        term_ep,
    }
}

/// **Spawn a `login_test_client` role** against `w`, and return its report. Waits for the role to
/// finish before returning: `spawn_client` followed by `wait_client` is the same pair, split, for a
/// caller that wants two (or more) roles genuinely in flight together (see those two functions'
/// own docs, and `kernel::user::login_tests` for the isolation proof that needs it).
pub fn client(image: &'static [u8], w: &Wiring, role: u64) -> [u64; 5] {
    wait_client(spawn_client(image, w, role))
}

/// **Spawn a `login_test_client` role and return its report endpoint immediately**, without
/// waiting for it to run at all. Milestone 49's channel-per-client update is what makes this worth
/// having separately from [`client`]: two roles spawned this way before either is waited on reach
/// the front door on their own schedule, which is genuine concurrency at the front door rather than
/// the artificial kind a single call that spawns-then-waits could ever produce. Pair with
/// [`wait_client`].
pub fn spawn_client(image: &'static [u8], w: &Wiring, role: u64) -> RendezvousId {
    let report = sched::create_rendezvous();
    // A small, private scratch budget for this one role: milestone 49's channel-per-client update
    // means a role must map the page `login`'s `CONNECT` step delegates before it holds anything
    // else of its own (unlike the post-auth `budget`, `map_page_frame`'s own page-table cost has
    // nowhere else to come from at that point). Independent per role, the same reason
    // `login`'s own `CONSTRUCTION_UT` is never shared with a client: two roles racing to map their
    // own, unrelated pages must never be able to exhaust or interfere with each other's page tables.
    let scratch =
        crate::memory_region::create(CLIENT_SCRATCH_UT_PAGES).expect("no scratch region for role");
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
                    memory_region_root_cap(scratch),
                ],
                maps: &[],
            },
        )
    })
    .expect("could not spawn a login_test_client");
    report
}

/// **Block for one role's report**, the other half of [`spawn_client`].
pub fn wait_client(report: RendezvousId) -> [u64; 5] {
    sched::ipc_recv(report)
}
