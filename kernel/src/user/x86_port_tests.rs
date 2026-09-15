//! **The two load-bearing tests of the x86 port-range capability** (milestone 299, DECISIONS §121
//! reversed 2026-09-15). A `PortRange` capability is enforced entirely at the context switch, by the
//! TSS I/O permission bitmap, with nothing on the syscall path to assert on; the only honest test is
//! a ring-3 program that executes `out` and observes whether the CPU allowed it. So these build
//! hand-assembled `x86_64` children (`super::x86_programs`) exactly as `supervision_tests` does, and
//! read the outcome from the child's supervision endpoint: a word arrives if the `out` was allowed,
//! an `EVENT_FAULT` if it was not.
//!
//! Two properties, both named load-bearing in the milestone's own BUGS:
//!
//! 1. **A non-holder cannot touch the port, and a holder's grant does not leak across a switch.**
//!    [`port_holder_transmits_then_a_non_holder_faults`] runs a holder (which transmits) and then a
//!    child with no port capability (which faults on the same `out`). The second running *after* the
//!    first is the hand-off test: if the switch away from the holder had left the TSS permitting the
//!    port, the non-holder would inherit it and not fault.
//! 2. **A revoked holder faults on its next `in`/`out`.**
//!    [`a_revoked_holder_faults_on_its_next_port_write`] parks a holder in `RECV`, revokes its port
//!    range while it is parked, wakes it, and confirms the `out` it then executes faults.
//!
//! `x86_64` only: there is no port space, and no TSS I/O bitmap, on the other two architectures.

use abi::fault::{EVENT_EXIT, EVENT_FAULT, FAULT_EP_SLOT};

use super::*;
use crate::sched;

/// Where the child's code and stack go. Any two low-half pages; distinct from `supervision_tests`'
/// so a stray global could not make one test's leftovers look like another's.
const CODE_VA: u64 = 0x0060_0000;
const STACK_VA: u64 = 0x0070_0000;

/// The port the children write to: COM1's **scratch register** (`0x3FF`, the eighth of the eight
/// ports the range names). It is inside the granted `(0x3F8, 8)` range, so a holder may write it, and
/// it has no device effect, so a permitted write does not scribble on the serial console the kernel
/// itself is printing the test transcript to. The value is arbitrary.
const SCRATCH_PORT: u16 = 0x3FF;
const SCRATCH_VAL: u8 = 0x5A;

/// COM1's port range, the object a `PortRange` capability over the console names. The children hold a
/// capability to this exact `(base, count)`, and revocation names the same pair.
const COM1_BASE: u16 = super::X86_COM1_PORT_BASE;
const COM1_COUNT: u16 = super::X86_COM1_PORT_COUNT;

/// The word a holder SENDs once its `out` is allowed, so a test can tell "it transmitted" from "it
/// faulted". Distinctive.
const REPORTED: u64 = 0xC0DE;

/// Build a ring-3 child from `stub` with its whole world in one region (address space, code, stack,
/// TCB), so a single reclaim frees it. `report` lands in slot 0 (what the stub SENDs on), `wake` in
/// slot 1 if given (what a `recv_then_port_out` child parks on), the `PortRange` capability next if
/// `port` is given (held, never invoked by slot), and `fault_ep` in the reserved fault slot. Returns
/// `(child_tid, region)`.
fn build_child(
    stub: &[u32],
    report: sched::RendezvousId,
    wake: Option<sched::RendezvousId>,
    port: bool,
    fault_ep: sched::RendezvousId,
) -> (u64, u64) {
    let region = crate::memory_region::create(16).expect("no region for the child");
    let aspace = user_address_space_create(region).expect("no aspace");

    let code_phys = crate::memory_region::retype_page(region).expect("no code frame");
    // SAFETY: a fresh frame we own, direct-mapped; write the stub and make it fetchable.
    unsafe {
        let dst = mmu::phys_to_virt(code_phys) as *mut u32;
        for (i, &insn) in stub.iter().enumerate() {
            dst.add(i).write(insn);
        }
    }
    sync_icache(mmu::phys_to_virt(code_phys), core::mem::size_of_val(stub));
    user_address_space_map(
        aspace,
        CODE_VA,
        code_phys,
        Flags::user_code(),
        crate::revoke::PageMapSource::NoCapability,
    )
    .expect("map code");

    let stack_phys = crate::memory_region::retype_page(region).expect("no stack frame");
    user_address_space_map(
        aspace,
        STACK_VA,
        stack_phys,
        Flags::user_data(),
        crate::revoke::PageMapSource::NoCapability,
    )
    .expect("map stack");

    let tid = sched::create_thread_control_block(region).expect("no tcb");

    // Slot 0: the report endpoint the stub SENDs on.
    let slot = sched::thread_control_block_insert_cap(
        tid,
        crate::cap::rendezvous_cap(report, crate::cap::Rights::WRITE),
        None,
    )
    .expect("insert report");
    assert_eq!(slot, 0, "the report cap must land in slot 0");

    // Slot 1: the wake endpoint a `recv_then_port_out` child parks on, when this build has one.
    if let Some(w) = wake {
        let slot = sched::thread_control_block_insert_cap(
            tid,
            crate::cap::rendezvous_cap(w, crate::cap::Rights::READ),
            None,
        )
        .expect("insert wake");
        assert_eq!(slot, 1, "the wake cap must land in slot 1");
    }

    // The port range, held so the TSS bitmap grants it: the whole point of the test. `WRITE`, the
    // rights a driver gets; never `GRANT`, so the child cannot re-delegate or revoke it. It is not
    // invoked by slot number (the child executes `out` directly), so its slot does not matter.
    if port {
        sched::thread_control_block_insert_cap(
            tid,
            crate::cap::port_range_cap(COM1_BASE, COM1_COUNT, crate::cap::Rights::WRITE),
            None,
        )
        .expect("insert the port range");
    }

    // The reserved fault slot: born supervised, so a #GP on the `out` arrives as a message rather
    // than panicking the kernel (there would otherwise be no thread to kill).
    sched::thread_control_block_insert_cap(
        tid,
        crate::cap::rendezvous_cap(fault_ep, crate::cap::Rights::READ),
        Some(FAULT_EP_SLOT),
    )
    .expect("insert fault ep");

    sched::configure_thread_control_block(tid, CODE_VA, STACK_VA + page_frames::FRAME_SIZE, aspace)
        .expect("configure");
    sched::start_thread_control_block(tid, [0; 3]).expect("start");
    (tid, region)
}

/// Reclaim a child's region once it is a corpse, retried for the window a just-delivered death
/// message opens (the corpse is still on its own kernel stack for a few hundred instructions; a
/// stack must not be unmapped under a core standing on it). Same shape and same reason as
/// `supervision_tests`' reap.
fn reap(region: u64) {
    assert!(
        super::wait_for(|| sched::reclaim_region(region).is_ok()),
        "reaping the child's region failed",
    );
}

/// **A holder transmits; the non-holder that runs next faults.** The non-holder test and the
/// hand-off test in one, because the order is the hand-off: the non-holder is scheduled after the
/// holder installed and then vacated the TSS bitmap, so its fault is proof the switch left the port
/// denied rather than inheriting the holder's grant.
#[test_case]
fn port_holder_transmits_then_a_non_holder_faults() {
    // The holder: it executes `out` to a port its capability names, so the CPU permits it, and the
    // word arrives.
    let report = sched::create_rendezvous();
    let sup = sched::create_rendezvous();
    let (_holder, holder_region) = build_child(
        &super::x86_programs::port_out(SCRATCH_PORT, SCRATCH_VAL, REPORTED as u32),
        report,
        None,
        true,
        sup,
    );
    assert_eq!(
        sched::ipc_recv(report)[0],
        REPORTED,
        "the port holder's `out` should have been permitted, and its report should have arrived",
    );
    assert_eq!(
        sched::ipc_recv(sup)[0],
        EVENT_EXIT,
        "the holder should have exited cleanly after transmitting",
    );
    reap(holder_region);

    // The non-holder: the *same* program, granted no port capability, run right after the holder. Its
    // `out` faults, which is both "a non-holder cannot touch the port" and "the hand-off away from
    // the holder left the TSS denying".
    let report2 = sched::create_rendezvous();
    let sup2 = sched::create_rendezvous();
    let (non_holder, nh_region) = build_child(
        &super::x86_programs::port_out(SCRATCH_PORT, SCRATCH_VAL, REPORTED as u32),
        report2,
        None,
        false,
        sup2,
    );
    let msg = sched::ipc_recv(sup2);
    assert_eq!(
        msg[0], EVENT_FAULT,
        "a non-holder's `out` must fault, not be permitted",
    );
    assert_eq!(msg[1], non_holder, "the fault named the wrong thread");
    assert_eq!(
        msg[2],
        CODE_VA + super::x86_programs::PORT_OUT_PC_OFFSET,
        "the faulting pc was not the `out` instruction",
    );
    reap(nh_region);
}

/// **A revoked holder faults on its next `out`.** The child holds the port range and parks in
/// `RECV`; while it is parked the test revokes the range (deleting its capability and clearing the
/// cached grant the switch installs), then wakes it. The `out` it executes on waking faults, which
/// is the whole claim: a capability that was real became unusable the instant it was revoked.
#[test_case]
fn a_revoked_holder_faults_on_its_next_port_write() {
    let report = sched::create_rendezvous();
    let wake = sched::create_rendezvous();
    let sup = sched::create_rendezvous();
    let (holder, region) = build_child(
        &super::x86_programs::recv_then_port_out(SCRATCH_PORT, SCRATCH_VAL, REPORTED as u32),
        report,
        Some(wake),
        true,
        sup,
    );

    // Revoke the port range while the child is still parked in RECV (it cannot reach its `out`
    // before the wake below, so the revoke provably precedes the port access). This deletes the
    // child's `PortRange` capability and clears the grant the context switch would install.
    crate::revoke::revoke_port_range(COM1_BASE, COM1_COUNT);

    // Wake it. It leaves RECV, executes `out`, and faults, because the port it once held is gone.
    sched::ipc_send(wake, [0, 0, 0]);

    let msg = sched::ipc_recv(sup);
    assert_eq!(
        msg[0], EVENT_FAULT,
        "a revoked holder's next `out` must fault; the word must never arrive",
    );
    assert_eq!(msg[1], holder, "the fault named the wrong thread");
    reap(region);
}
