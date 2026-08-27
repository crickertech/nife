use super::*;
use crate::cap::{Rights, rendezvous_cap};
use crate::sched::{self, RendezvousId};

/// The VAs `user/src/kilo.rs` hardcodes. Must match that file.
const TERM_OUT_VA: u64 = 0x0000_0000_0080_0000;
const FS_VA: u64 = 0x0000_0000_0060_0000;

/// Stack pages beyond the one `run` maps. `kilo`'s own `Editor` is a few kilobytes (32 rows of
/// 100 bytes plus bookkeeping) and this program has no allocator, so it all lives on the stack;
/// sized with the same "extra pages rather than exactly enough" margin `swish`'s own wiring uses.
const KILO_STACK_PAGES: usize = 4;

/// The file channel's width, straight from the contract `kilo`'s FS calls speak, so this wiring
/// cannot disagree with the program on the other end. `fs_service::FILE_PAGES` is the same value
/// but private to that module; this is the source it is itself defined from.
const FILE_PAGES: usize = filesystem_proto::fs::TRANSFER_PAGES;

/// A fresh, zeroed frame, for `kilo`'s own extra stack pages. `fs_service`'s own `page_frame` does
/// the same thing and is private to that module; this is a second copy of three lines rather than
/// a visibility change to a helper whose whole point is to stay unexported.
fn page_frame() -> u64 {
    let p = crate::memory::alloc()
        .expect("no frame for kilo's stack")
        .addr();
    // SAFETY: fresh frame, reachable through the direct map.
    unsafe { core::ptr::write_bytes(mmu::phys_to_virt(p) as *mut u8, 0, FRAME_SIZE as usize) };
    p
}

/// A running `kilo`, wired with a real terminal (`line_editor` and a fake console, exactly
/// [`raw_mode_service::start`]'s wiring) and a real filesystem (a directory narrowed to `dir_name`
/// under `fs_subtree_caretaker`, exactly [`fs_service::narrow_dir`]'s wiring). Composing the two is
/// the only thing this module adds: `kilo` is the one program in this tree that needs both a
/// terminal and a file at once, the same shape `swish`'s own wiring already has (milestone 50's
/// redirection witness), so nothing here is a new pattern.
pub struct Wiring {
    /// The terminal endpoint, played by the fake console + real `line_editor` underneath. A test
    /// wanting to observe the screen reads [`Wiring::term_out_phys`] after driving keystrokes
    /// through the same input-driver role [`raw_mode_service`]'s own tests use.
    pub term: RendezvousId,
    pub term_out_phys: u64,
    /// The narrowed directory `kilo` holds. A test may also call this directly (after `kilo` has
    /// exited and stopped using its shared page) to read the file back as an independent witness,
    /// exactly the two-witness discipline `c_seam`'s confiner tests use elsewhere.
    pub dir: RendezvousId,
    pub file_shared: u64,
    /// `kilo`'s one closing report: `(STATUS_QUIT | STATUS_OPEN_FAILED, dirty-or-errno, 0)`.
    pub report: RendezvousId,
}

/// Spawn `kilo` against `dir_name` (a directory already in the test fixture tree, granted with
/// [`filesystem_proto::dir::ALL`]) and `file_name` (the file inside it `kilo` edits, created if
/// absent). `None` if there is no RedoxFS disk attached to this run.
pub fn start(dir_name: &'static str, file_name: &str) -> Option<Wiring> {
    let image = program("kilo").expect("no kilo program in the initrd archive");

    // The terminal half: identical to raw_mode_service::start, duplicated rather than reused
    // because that function's Wiring hands back a line_editor already wired to nobody; composing a
    // *third* process's grants into a spawn that function does not perform would need it split the
    // way fs_service::narrow_dir already is split from fs_service::start_granted_dir, and one
    // wiring function is enough plumbing for a milestone that is not itself about wiring.
    let term_img = program("line_editor").expect("no line_editor program in the initrd archive");
    let term = sched::create_rendezvous();
    let conreq = sched::create_rendezvous();
    let conrep = sched::create_rendezvous();
    let console_phys = crate::memory::alloc()
        .expect("no frame for the fake console")
        .addr();
    let term_out_phys = crate::memory::alloc()
        .expect("no frame for kilo's terminal-output page")
        .addr();
    let term_app_in_phys = crate::memory::alloc()
        .expect("no frame for line_editor's unused app-input page")
        .addr();
    for phys in [console_phys, term_out_phys, term_app_in_phys] {
        // SAFETY: each just allocated, direct-mapped, owned by nobody else yet.
        unsafe {
            core::ptr::write_bytes(mmu::phys_to_virt(phys) as *mut u8, 0, FRAME_SIZE as usize);
        }
    }
    sched::spawn(move || {
        loop {
            sched::ipc_recv(conreq);
            sched::ipc_send(conrep, [0, 0, 0]);
        }
    })
    .expect("could not spawn the fake console");
    sched::spawn(move || {
        run(
            term_img,
            Spawn {
                arg0: 0,
                arg1: 0,
                arg2: 0,
                grants: &[
                    rendezvous_cap(term, Rights::READ),
                    rendezvous_cap(conreq, Rights::WRITE),
                    rendezvous_cap(conrep, Rights::READ),
                ],
                maps: &[
                    Mapping {
                        va: 0x0060_0000, // line_editor's own CONOUT_VA
                        phys: console_phys,
                        flags: Flags::user_data(),
                    },
                    Mapping {
                        va: 0x0080_0000, // line_editor's own APP_OUT_VA
                        phys: term_out_phys,
                        flags: Flags::user_rodata(),
                    },
                    Mapping {
                        va: 0x0090_0000, // line_editor's own APP_IN_VA
                        phys: term_app_in_phys,
                        flags: Flags::user_data(),
                    },
                ],
            },
        )
    })
    .expect("could not spawn line_editor");

    // The filesystem half: fs_service's own split, built for exactly this (a client that needs a
    // directory *and* something else, milestone 50's own redirection witness).
    let (dir_ep, file_shared) = fs_service::narrow_dir(
        fs_service::blk_server_image(),
        fs_service::fs_server_image()?,
        program("fs_subtree_caretaker").expect("no fs_subtree_caretaker program in the initrd"),
        dir_name,
        filesystem_proto::dir::ALL,
    )?;

    assert!(
        filesystem_proto::grant::fits(file_name.as_bytes()),
        "kilo's target name rides in two argument words; this one does not fit",
    );
    let (lo, hi) = filesystem_proto::grant::pack_name(file_name.as_bytes());
    let spec = filesystem_proto::grant::spec(file_name.len(), 0);

    let report = sched::create_rendezvous();
    sched::spawn(move || {
        let mut maps = [Mapping {
            va: 0,
            phys: 0,
            flags: Flags::user_data(),
        }; FILE_PAGES + KILO_STACK_PAGES + 1];
        maps[0] = Mapping {
            va: TERM_OUT_VA,
            phys: term_out_phys,
            flags: Flags::user_data(),
        };
        let n = 1 + fs_service::map_channel(&mut maps[1..], FS_VA, file_shared, FILE_PAGES);
        for (k, m) in maps[n..n + KILO_STACK_PAGES].iter_mut().enumerate() {
            m.va = USER_STACK_VA - (k as u64 + 1) * FRAME_SIZE;
            m.phys = page_frame();
        }
        run(
            image,
            Spawn {
                arg0: spec,
                arg1: lo,
                arg2: hi,
                grants: &[
                    rendezvous_cap(term, Rights::WRITE),   // slot 0: the terminal
                    rendezvous_cap(dir_ep, Rights::WRITE), // slot 1: the directory
                    rendezvous_cap(report, Rights::WRITE), // slot 2: the closing report
                ],
                maps: &maps[..n + KILO_STACK_PAGES],
            },
        )
    })
    .expect("could not spawn kilo");

    Some(Wiring {
        term,
        term_out_phys,
        dir: dir_ep,
        file_shared,
        report,
    })
}
