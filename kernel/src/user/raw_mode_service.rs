use super::*;
use crate::cap::{Rights, rendezvous_cap};
use crate::sched::{self, RendezvousId};

/// The VAs `user/src/line_editor.rs` hardcodes for its console and application pages. Must match
/// that file's `CONOUT_VA`, `APP_OUT_VA`, `APP_IN_VA`.
const CONOUT_VA: u64 = 0x0060_0000;
const APP_OUT_VA: u64 = 0x0080_0000;
const APP_IN_VA: u64 = 0x0090_0000;

/// A running `line_editor` and the raw physical pages a test needs to drive it directly, playing
/// both the input driver and the application on the one terminal endpoint. That double role is
/// exactly what the real terminal contract already allows: nobody on the other end of `TERM` can
/// tell who is calling (DECISIONS §21's endpoint-only naming), so a test standing in for both is
/// not a shortcut around the contract, it is the contract.
pub struct Wiring {
    /// The terminal endpoint. `CALL` here for every opcode this milestone added or reused:
    /// `OP_RAWMODE`, `OP_READRAW`, `OP_BYTES` (as the input driver would), `OP_READLINE`,
    /// `OP_WRITE`.
    pub term: RendezvousId,
    /// The console's shared page, physical. This is what `line_editor` echoes into
    /// (`user/src/line_editor.rs`'s `Con`). A test sentinel-fills it before an exchange and checks
    /// it after: that is how "no echo happened" gets proven rather than assumed, the same
    /// byte-for-byte witness-page discipline `c_seam`'s confiner tests use for a different claim.
    pub console_phys: u64,
    /// The client output page: a test writes an `OP_READLINE` prompt or an `OP_WRITE` payload
    /// here before the request that names it.
    pub app_out_phys: u64,
    /// The client input page: `line_editor` writes a completed line here for `OP_READLINE`.
    pub app_in_phys: u64,
}

/// **Spawn a fake console and a real `line_editor`**, wired exactly as the boot path wires them
/// except that the input driver and the application are both played by the caller.
///
/// The fake console speaks only the one protocol `line_editor`'s `Con::flush` needs: `SEND` a
/// count, `RECV` an ack (not a `CALL`; `user/src/line_editor.rs`'s own module doc explains why
/// that hop is safe with exactly one client). It never inspects the shared page; a test checks
/// that directly through [`Wiring::console_phys`], which is the point: a fake that graded its own
/// homework would prove nothing about echo suppression.
pub fn start() -> Wiring {
    let image = program("line_editor").expect("no line_editor program in the initrd archive");

    let term = sched::create_rendezvous();
    let conreq = sched::create_rendezvous();
    let conrep = sched::create_rendezvous();

    let console_phys = crate::memory::alloc()
        .expect("no frame for the fake console's shared page")
        .addr();
    let app_out_phys = crate::memory::alloc()
        .expect("no frame for line_editor's app-output page")
        .addr();
    let app_in_phys = crate::memory::alloc()
        .expect("no frame for line_editor's app-input page")
        .addr();
    for phys in [console_phys, app_out_phys, app_in_phys] {
        // SAFETY: each frame was just allocated, is direct-mapped, and is owned by nobody else
        // yet, so zeroing it here cannot race or leak stale RAM into the test.
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
            image,
            Spawn {
                arg0: 0,
                arg1: 0,
                arg2: 0,
                grants: &[
                    rendezvous_cap(term, Rights::READ),    // slot 0: TERM, RECV_CAP
                    rendezvous_cap(conreq, Rights::WRITE),  // slot 1: CONREQ, SEND
                    rendezvous_cap(conrep, Rights::READ),   // slot 2: CONREP, RECV
                ],
                maps: &[
                    Mapping {
                        va: CONOUT_VA,
                        phys: console_phys,
                        flags: Flags::user_data(),
                    },
                    Mapping {
                        va: APP_OUT_VA,
                        phys: app_out_phys,
                        flags: Flags::user_rodata(),
                    },
                    Mapping {
                        va: APP_IN_VA,
                        phys: app_in_phys,
                        flags: Flags::user_data(),
                    },
                ],
            },
        )
    })
    .expect("could not spawn line_editor");

    Wiring {
        term,
        console_phys,
        app_out_phys,
        app_in_phys,
    }
}
