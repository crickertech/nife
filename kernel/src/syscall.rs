//! The syscall boundary. **Four calls <!--count:syscalls-->.**
//!
//! DECISIONS.md §4 rule 3 said the syscall surface stays narrow and explicit, *"a boundary, not a
//! habit."* §8 said milestone 7 was a hard decision point and that hacking one in without the
//! conversation meant the plan had failed. §10 had the conversation and chose capabilities.
//!
//! This is what that buys:
//!
//! ```text
//!   exit(code)                          authority over yourself
//!   yield()                             likewise
//!   cap_delete(slot)                    likewise: your own capability table is your own
//!   invoke(cap, method, a0, a1, a2)     EVERYTHING ELSE
//! ```
//!
//! Three of the four are authority over yourself and the fourth is everything else, which is the
//! property worth remembering rather than the count. This header said "three calls" from
//! 2026-07-14 until the 2026-08-17 documentation sweep, `abi::SYS_CAP_DELETE` having arrived on
//! 2026-07-24 without it; the count is now a re-derived `<!--count:syscalls-->` claim.
//!
//! No `open`. No `read`. No `write`. No `fork`. **A process can only act on things it was
//! handed.** The ABI lives in `crates/abi`, which both the kernel and every user program depend
//! on, so the boundary is *one artifact* rather than two files that agree by luck.
//!
//! # No pointer ever crosses this boundary
//!
//! There used to be a `user_slice` here: the console `write` syscall took a `(ptr, len)` from
//! userspace and the kernel read the user's memory, which is why it needed the `AT S1E0R`
//! confused-deputy defence. Milestone 8 moved the console to a userspace server and deleted that
//! path. Today every argument is a scalar in a register (a capability slot, a method, a `va`, a
//! word), so the kernel follows no user pointer and there is no deputy to confuse. The primitive
//! that made the old check possible, `mmu::user_can_read`, is kept for the next syscall that does
//! take a user pointer.

use abi::Error;

use crate::arch::exceptions::TrapFrame;
use crate::arch::mmu;
use crate::cap::{Object, Rights};
use crate::sched;

/// Called from the `svc` arm of `exception_body` (`ecall` on riscv64, in `riscv_trap_body`). The
/// *body*, not the dispatcher, since milestone 124 split the two: a syscall arrives from user mode,
/// which is the case that deliberately does NOT move to the interrupt stack, because this path can
/// block and a blocked thread's frames must live on its own stack.
pub fn dispatch(frame: &mut TrapFrame) {
    // The syscall number and arguments come from the trap frame through arch accessors, not raw
    // register indices, because the ABI register file differs per architecture (aarch64 `svc` with
    // the number in x8 and args in x0..x5; RISC-V `ecall` with the number in a7 and args in a0..a5).
    // `TrapFrame::{syscall_nr, arg, set_arg}` hide that mapping so this dispatcher stays portable.
    // See DECISIONS §10/§17.
    let nr = frame.syscall_nr();

    // `exit` never comes back, so it is not part of the result-writing path below.
    if nr == abi::SYS_EXIT {
        sched::exit();
    }

    let result: Result<i64, Error> = match nr {
        abi::SYS_YIELD => {
            sched::yield_now();
            Ok(0)
        }
        // Drop a capability from the caller's own capability table (milestone 19d). Deleting an empty slot
        // is a no-op, not an error: a loader recycling slots should not have to track emptiness.
        abi::SYS_CAP_DELETE => {
            let _ = sched::delete_current_cap(frame.arg(0));
            Ok(0)
        }
        abi::SYS_INVOKE => invoke(
            frame,
            frame.arg(0),
            frame.arg(1),
            frame.arg(2),
            frame.arg(3),
            frame.arg(4),
        ),
        _ => Err(Error::BadSyscall),
    };

    // The return value goes back in the first argument register, which the trap-restore path pops
    // into the register the user is waiting on. Writing to the trap frame IS writing to the user's
    // registers.
    frame.set_arg(
        0,
        match result {
            Ok(v) => v as u64,
            Err(e) => (e as i64) as u64,
        },
    );
}

/// Act on a capability.
///
/// **The lookup is the security mechanism, and it is a bounds check.** `slot` is an index into
/// *this thread's* table, which lives in kernel memory. An empty slot is `NoSuchSlot`: not
/// "permission denied", but *there is nothing there*. That difference is what no-ambient-authority
/// feels like from the inside.
///
/// `pub(crate)` so kernel tests in other modules can drive **this** path rather than a
/// re-implementation of it: an authorization test that calls `sched` directly proves the helper, not
/// the boundary. See `user/reap_tests.rs`.
pub(crate) fn invoke(
    frame: &mut TrapFrame,
    slot: u64,
    method: u64,
    a0: u64,
    a1: u64,
    a2: u64,
) -> Result<i64, Error> {
    let cap = sched::current_cap(slot).map_err(|_| Error::NoSuchSlot)?;

    match cap.object {
        Object::Rendezvous(ep) => match method {
            // SEND takes WRITE, RECV takes READ. The *same* endpoint, handed out with different
            // rights, is a one-way pipe in whichever direction each holder was trusted with.
            abi::rendezvous::SEND => {
                if !cap.rights.allows(Rights::WRITE) {
                    return Err(Error::NotPermitted);
                }
                // The three words are already in registers. **Nothing is read from user memory**,
                // so there is no pointer to validate and no confused-deputy question to ask. That
                // is the fastpath, and it is why IPC carries control and not bulk data (§10).
                sched::ipc_send(ep, [a0, a1, a2]);
                // If the endpoint was revoked (stale, or reclaimed while we blocked), the send never
                // happened: report it rather than a silent success. Object revocation, notes/.
                //
                // `Gone`, not `NoSuchSlot`, since milestone 50. The slot is not empty; a real
                // capability names an object that has been destroyed, and a writer branches on the
                // difference in opposite directions (`abi::Error::Gone`, notes/sink-protocol.md).
                // This one line is what turns a dead pipe reader into something the producer can
                // act on, which is why the ABI grew a variant rather than the sink protocol growing
                // a heartbeat.
                if sched::take_ipc_aborted() {
                    return Err(Error::Gone);
                }
                Ok(0)
            }
            abi::rendezvous::RECV => {
                if !cap.rights.allows(Rights::READ) {
                    return Err(Error::NotPermitted);
                }
                let msg = sched::ipc_recv(ep);
                if sched::take_ipc_aborted() {
                    return Err(Error::Gone); // endpoint revoked; the message is a placeholder
                }
                // Word 0 goes back the way every syscall result does, in x0 (dispatch writes it
                // from our return value). Words 1..4 we place directly, because a syscall return is
                // one register and a message is up to five: ordinary IPC fills three and leaves the
                // top two zero, and a fault/exit notification (DECISIONS §26) fills all five.
                frame.set_arg(1, msg[1]);
                frame.set_arg(2, msg[2]);
                frame.set_arg(3, msg[3]);
                frame.set_arg(4, msg[4]);
                Ok(msg[0] as i64)
            }

            // Delegation. `a0` is the slot of the capability to pass on, `a1` the rights to narrow
            // it to, `a2` one data word. Two rights are in play and they are different questions:
            // WRITE on *this* endpoint (may I send here?) and GRANT on the *delegated* capability
            // (was I trusted to pass it on?). Without GRANT you may use a thing and not lend it.
            abi::rendezvous::SEND_CAP => {
                if !cap.rights.allows(Rights::WRITE) {
                    return Err(Error::NotPermitted);
                }
                let src = sched::current_cap(a0).map_err(|_| Error::NoSuchSlot)?;
                if !src.rights.allows(Rights::GRANT) {
                    return Err(Error::NotPermitted); // holder may not pass this on
                }
                let narrowed = Rights::from_bits(a1 as u32);
                if !narrowed.is_subset_of(src.rights) {
                    return Err(Error::NotPermitted); // delegation may only narrow, never widen
                }
                sched::ipc_send_cap(
                    ep,
                    a2,
                    crate::cap::Cap {
                        object: src.object,
                        rights: narrowed,
                    },
                );
                if sched::take_ipc_aborted() {
                    return Err(Error::Gone); // endpoint revoked; the delegation did not happen
                }
                Ok(0)
            }
            abi::rendezvous::RECV_CAP => {
                if !cap.rights.allows(Rights::READ) {
                    return Err(Error::NotPermitted);
                }
                let msg = sched::ipc_recv_cap(ep);
                if sched::take_ipc_aborted() {
                    return Err(Error::Gone); // endpoint revoked; the message is a placeholder
                }
                // x1 carries the slot the received capability landed in, or NO_CAP if the message
                // brought none; x2 the second data word (a CALL's, or 0). x0 returns the first word.
                frame.set_arg(1, msg[1]);
                frame.set_arg(2, msg[2]);
                Ok(msg[0] as i64)
            }

            // Call: send two words and block until replied. The kernel mints a one-shot Reply cap
            // naming us into the server (delivered by its RECV_CAP); we return here only when the
            // server invokes it. See §12 and notes/ipc-naming.md. Sending needs WRITE, like SEND.
            abi::rendezvous::CALL => {
                if !cap.rights.allows(Rights::WRITE) {
                    return Err(Error::NotPermitted);
                }
                let reply = sched::ipc_call(ep, [a0, a1]);
                if sched::take_ipc_aborted() {
                    return Err(Error::Gone); // endpoint revoked; no call, no reply
                }
                frame.set_arg(1, reply[1]); // r1; r0 returns in x0 below
                Ok(reply[0] as i64)
            }

            // **Collect a corpse this endpoint supervises** (DECISIONS §32). The method that lets a
            // supervisor reap without holding the authority to *build*: reaping used to mean
            // `MemoryRegion::DESTROY`, which needs WRITE on the region, and WRITE on a region is also what
            // retypes a thread and an address space out of it.
            //
            // READ, not WRITE: the authority to collect a death is the authority to *receive* deaths
            // here, which is what a supervisor holds and what a send-only holder (a peer that can
            // report to this supervisor) deliberately does not. `a0` is the tid the kernel stamped on
            // the death message, and it is authorized *relative to this endpoint* (sched's
            // reap_supervised), so it is a name inside a relationship rather than a global handle.
            abi::rendezvous::REAP => {
                if !cap.rights.allows(Rights::READ) {
                    return Err(Error::NotPermitted);
                }
                rendezvous_reap(ep, a0)
            }

            // **Read one entry of the domain this endpoint supervises** (milestone 126). The view
            // half of what REAP is the control half of, and scoped by the same relationship, so a
            // supervisor sees exactly the children whose deaths would arrive here.
            //
            // READ for the same reason REAP takes READ: the authority to see who may die here is
            // the authority to receive deaths here. **A send-only holder is refused rather than
            // shown an empty domain**, which is the whole point of the method; a monitor that
            // reports nothing because it could not look is the worst failure this tool has, and an
            // empty answer is reserved for a domain that really is empty.
            //
            // `a0` is the cursor: 0 to start, then whatever the last call returned, until a
            // `survey::DONE` comes back. x1 carries the tid and x2 the state code.
            abi::rendezvous::SURVEY => {
                // `ENUMERATE`, not `READ`, and the distinction is the method's whole safety
                // argument: `READ` here also unlocks `RECV` and `REAP`, so a viewer granted it
                // could reap a child. A domain names its members and does not act on them, and one
                // bit for three operations cannot say that. See `Rights::ENUMERATE`.
                if !cap.rights.allows(Rights::ENUMERATE) {
                    return Err(Error::NotPermitted);
                }
                let (next, tid, state) = sched::survey_supervised(ep, a0)?;
                frame.set_arg(1, tid);
                frame.set_arg(2, state);
                Ok(next as i64)
            }
            _ => Err(Error::BadMethod),
        },

        // A one-shot reply to a specific caller (§12). Minted by the kernel at a CALL rendezvous,
        // named by the caller's tid, consumed on use.
        Object::Reply(tid) => match method {
            abi::reply::REPLY => {
                // Minted WRITE-only, so a narrowed derivative could not answer; and minted without
                // GRANT, so it could not have been delegated here in the first place.
                if !cap.rights.allows(Rights::WRITE) {
                    return Err(Error::NotPermitted);
                }
                sched::ipc_reply(tid, [a0, a1]);
                // One-shot: consume it, so a second reply is NoSuchSlot and the caller cannot be
                // answered twice. This is the guarantee a pre-wired reply endpoint cannot make.
                let _ = sched::delete_current_cap(slot);
                Ok(0)
            }
            _ => Err(Error::BadMethod),
        },

        // Another process's memory, under construction (19b). WRITE on the address space cap is the
        // authority to shape it; the frame's own rights gate what kind of mapping, exactly as
        // frame::MAP; the va gate is the proved paging::is_user_page_va, as everywhere.
        Object::AddressSpace(name) => match method {
            abi::address_space::MAP_INTO => {
                if !cap.rights.allows(Rights::WRITE) {
                    return Err(Error::NotPermitted);
                }
                let va = a0;
                let frame = sched::current_cap(a1).map_err(|_| Error::NoSuchSlot)?;
                // The mappable object is a PageFrame (normal memory) or a DeviceFrame (a device's
                // MMIO, device-typed): the driver a userspace init builds gets its registers this
                // way (19d.2). a2 chooses the shape for a PageFrame; a DeviceFrame is always
                // device-typed read/write and needs WRITE on the cap.
                let (phys, count, flags) = match frame.object {
                    Object::DeviceFrame(phys) => {
                        if !frame.rights.allows(Rights::WRITE) {
                            return Err(Error::NotPermitted);
                        }
                        (phys, 1u64, paging::Flags::user_device())
                    }
                    // §102 (2026-08-20): `count` is the run's length. A single-page frame is
                    // `count: 1`, so this arm's behavior for every existing caller is unchanged; a
                    // run-capable frame maps the whole run in this one MAP_INTO call, exactly as
                    // `page_frame::MAP` does below.
                    Object::PageFrame(phys, count) => {
                        // 0 read-only, 1 read/write, 2 executable code (a loader's child .text).
                        // Code is W^X: user_code is RX, never writable, so it needs only READ.
                        let flags = match a2 {
                            abi::address_space::MAP_RW => {
                                if !frame.rights.allows(Rights::WRITE) {
                                    return Err(Error::NotPermitted);
                                }
                                paging::Flags::user_data()
                            }
                            abi::address_space::MAP_CODE => {
                                if !frame.rights.allows(Rights::READ) {
                                    return Err(Error::NotPermitted);
                                }
                                paging::Flags::user_code()
                            }
                            _ => {
                                if !frame.rights.allows(Rights::READ) {
                                    return Err(Error::NotPermitted);
                                }
                                paging::Flags::user_rodata()
                            }
                        };
                        (phys, count.get(), flags)
                    }
                    _ => return Err(Error::WrongObject),
                };
                // **The run's last page is checked, not only its first** (milestone 142's review,
                // MAJOR 3). This used to check `va` alone, which was right when a frame was one
                // page and wrong the moment `count` could exceed 1: a run placed near the top of
                // the low half passed the check and then walked out of it partway through the loop
                // below, refused three layers down by `Mapper::map`'s own `Half::Low` re-check
                // rather than here. Same guard `page_frame_map` uses, same reason: reject the whole
                // request before mapping any of it.
                let Some(last_va) = run_end_va(va, count) else {
                    return Err(Error::BadPointer);
                };
                if !paging::is_user_page_va::<crate::arch::mmu::Format>(va)
                    || !paging::is_user_page_va::<crate::arch::mmu::Format>(last_va)
                {
                    return Err(Error::BadPointer);
                }
                // The root the rollback below unmaps out of. Looked up once, before anything is
                // mapped: a `MAP_INTO` naming a space that does not exist must fail before it
                // spends a page table, and the loop's own `NotMapped` would otherwise report that
                // as a mapping failure with nothing to roll back.
                let Some(root) = crate::user::user_address_space_root(name) else {
                    return Err(Error::BadPointer);
                };
                for k in 0..count {
                    let (page_phys, page_va) =
                        (phys + k * paging::PAGE_SIZE, va + k * paging::PAGE_SIZE);
                    match crate::user::user_address_space_map(name, page_va, page_phys, flags) {
                        Ok(()) => {
                            // When userspace maps a frame it wrote executable (a spawner building a
                            // child's code, MAP_CODE), the instruction fetcher must be made to see
                            // the bytes the writer stored: RISC-V's `fence.i`, aarch64's
                            // dcache-clean + icache-invalidate, both behind `sync_icache`. The
                            // kernel-side ELF loader does this (user.rs map_segments); this is the
                            // userspace-built path, which a fast spawn+reap loop (bench::spawn_el0)
                            // is the first thing to stress. A child that fetches unsynced code
                            // takes an illegal-instruction fault at its entry.
                            if flags.is_user_executable() {
                                crate::arch::sync_icache(
                                    crate::arch::mmu::phys_to_virt(page_phys),
                                    paging::PAGE_SIZE as usize,
                                );
                            }
                        }
                        // **All or nothing across the run** (milestone 142's review, MAJOR 2).
                        // Whatever this loop mapped before failing is unmapped again, so a caller
                        // that gets an error never has to wonder how much of its run landed: the
                        // answer is always none of it. Before this the prefix stayed mapped and
                        // recorded with no way to ask about it, which is the pre-§102 single-page
                        // path's own rollback quietly narrowed to one page by the widening.
                        Err(e) => {
                            unmap_run_prefix(root, phys, va, k);
                            return Err(match e {
                                paging::MapError::OutOfPageFrames => Error::OutOfMemory,
                                // misaligned, already mapped, unknown space
                                _ => Error::BadPointer,
                            });
                        }
                    }
                }
                Ok(0)
            }
            // List what this address space has mapped, one entry per call, without the ability
            // to change any of it (milestone 126's `pmap`, DECISIONS §114): `Rendezvous::SURVEY`'s
            // shape one object type over, and pointedly `ENUMERATE` rather than `WRITE`, which is
            // what `MAP_INTO` above takes. See `abi::address_space::LIST` for the wire contract and
            // DECISIONS §114 for why this method's mere existence is the thing that makes
            // `ENUMERATE` live on every address-space capability minted since 2026-08-17 (the
            // `Rights::ALL`-on-creation invariant): the audit that check required is in
            // notes/process-view.md.
            abi::address_space::LIST => {
                if !cap.rights.allows(Rights::ENUMERATE) {
                    return Err(Error::NotPermitted);
                }
                address_space_list(frame, name, a0)
            }
            _ => Err(Error::BadMethod),
        },

        // A thread under construction (19c.3). WRITE on the TCB cap is the authority to shape
        // and start it. Every method refuses a thread that is not an embryo, in the scheduler.
        Object::ThreadControlBlock(tid) => match method {
            // Body extracted (milestone 156): every `ThreadControlBlock` method is process-spawn machinery a
            // loader runs once per child, never a step of the IPC round trip, so each moves out
            // of `invoke`'s own bytes. See `memory_region_map`'s doc comment for the full reasoning.
            abi::thread_control_block::CONFIGURE => {
                if !cap.rights.allows(Rights::WRITE) {
                    return Err(Error::NotPermitted);
                }
                thread_control_block_configure(tid, a0, a1, a2)
            }
            abi::thread_control_block::CAP_INSERT => {
                if !cap.rights.allows(Rights::WRITE) {
                    return Err(Error::NotPermitted);
                }
                thread_control_block_cap_insert(tid, a0, a1, a2)
            }
            abi::thread_control_block::START => {
                if !cap.rights.allows(Rights::WRITE) {
                    return Err(Error::NotPermitted);
                }
                sched::start_thread_control_block(tid, [a0, a1, a2])?; // the child's x0, x1, x2 (19d/19e)
                Ok(0)
            }
            _ => Err(Error::BadMethod),
        },

        Object::MemoryRegion(region) => match method {
            // Body extracted (milestone 156): all five `MemoryRegion` methods are memory-management
            // administration a spawner runs while building a process, never a step of the IPC
            // round trip `script/fastpath-footprint` bounds, so each stays out of `invoke`'s own
            // bytes and `#[inline(never)]` on purpose. See `address_space_list`, the pattern this copies.
            abi::memory_region::MAP => {
                if !cap.rights.allows(Rights::WRITE) {
                    return Err(Error::NotPermitted);
                }
                memory_region_map(region, a0)
            }
            abi::memory_region::RETYPE_OBJ => {
                if !cap.rights.allows(Rights::WRITE) {
                    return Err(Error::NotPermitted);
                }
                memory_region_retype_obj(region, a0)
            }
            abi::memory_region::RETYPE => {
                if !cap.rights.allows(Rights::WRITE) {
                    return Err(Error::NotPermitted);
                }
                memory_region_retype(region)
            }
            abi::memory_region::SPLIT => {
                if !cap.rights.allows(Rights::WRITE) {
                    return Err(Error::NotPermitted);
                }
                memory_region_split(cap, region, a0)
            }
            abi::memory_region::DESTROY => {
                if !cap.rights.allows(Rights::WRITE) {
                    return Err(Error::NotPermitted);
                }
                memory_region_destroy(region)
            }
            _ => Err(Error::BadMethod),
        },

        Object::PageFrame(phys, count) => match method {
            // Body extracted (milestone 156), the same reason as `MemoryRegion`'s five methods:
            // neither `MAP` nor `REVOKE` is a step of the IPC round trip, so both move out of
            // `invoke`'s own bytes. `MAP`'s rights check is data-dependent (branches on `a1`), so
            // it lives inside `page_frame_map` rather than at the call site here, unlike the fixed
            // single-right checks the other extractions keep in `invoke`.
            //
            // §102 (2026-08-20): `count` rides on the capability, not on the syscall's arguments,
            // so `MAP`'s and `REVOKE`'s wire shape is exactly what it was before the object could
            // name a run. A single-page frame (`count: 1`) runs each loop below once.
            abi::page_frame::MAP => page_frame_map(cap, phys, count, a0, a1, a2),
            abi::page_frame::REVOKE => {
                if !cap.rights.allows(Rights::GRANT) {
                    return Err(Error::NotPermitted);
                }
                page_frame_revoke(phys, count.get())
            }
            _ => Err(Error::BadMethod),
        },

        // A device's MMIO page is almost passive: it is handed to MAP_INTO as the page to map
        // (19d.2), and since milestone 23 it answers exactly one invocation, `REVOKE`.
        Object::DeviceFrame(phys) => match method {
            // **Take the registers back from everyone else** (DECISIONS §41). The step live
            // replacement needs between tearing one driver down and endowing the next, so that a
            // device never has two owners. Needs `GRANT`, the same rule `PageFrame::REVOKE` uses: you
            // were trusted to lend the device on, so you may take it back.
            //
            // Unlike a frame revoke this **spares the invoker's own** capability and mapping, and
            // it must: only the kernel mints a `DeviceFrame`, and it does so once at boot, so a
            // symmetric revoke would make the device unreachable for the rest of the machine's
            // life. `revoke::revoke_device_from_others` carries the full argument.
            abi::page_frame::REVOKE => {
                if !cap.rights.allows(Rights::GRANT) {
                    return Err(Error::NotPermitted);
                }
                crate::revoke::revoke_device_from_others(phys);
                Ok(0)
            }
            _ => Err(Error::BadMethod),
        },

        Object::Virtio(id) => {
            if !cap.rights.allows(Rights::WRITE) {
                return Err(Error::NotPermitted);
            }
            virtio_invoke(id, method, a0, a1)
        }

        Object::Irq(intid) => match method {
            // Body extracted (milestone 156). `WAIT` does block on the same `sched::ipc_recv`
            // the `Rendezvous` fastpath uses, but the caller here is a driver waiting on a device
            // interrupt, not the IPC round trip this gate bounds, so it moves out of `invoke`'s
            // own bytes with the rest. See `memory_region_map`'s doc comment for the full reasoning.
            abi::irq::WAIT => {
                if !cap.rights.allows(Rights::READ) {
                    return Err(Error::NotPermitted);
                }
                irq_wait(intid)
            }
            abi::irq::ACK => {
                if !cap.rights.allows(Rights::READ) {
                    return Err(Error::NotPermitted);
                }
                irq_ack(intid)
            }
            _ => Err(Error::BadMethod),
        },
    }
}

/// `MemoryRegion::MAP`: retype a page out of the untyped and map it, writable, at `va` in the caller's
/// own address space. Both the page and any page tables come from the untyped, so the KERNEL
/// ALLOCATES NOTHING: `mmu::map_current_user_page`'s only source of memory is the closure below,
/// which bumps the untyped's watermark.
///
/// Pulled out of [`invoke`] and marked `#[inline(never)]` (milestone 156, the pattern
/// `address_space_list` proved on milestone 126's `LIST`): `syscall_entry` is measured flat, so a rare
/// administrative arm inlined into the hot dispatcher grows every syscall's footprint for a
/// method that never runs on an IPC round trip.
#[inline(never)]
fn memory_region_map(region: u64, va: u64) -> Result<i64, Error> {
    // Reject the cheap failures BEFORE retyping a page for them: a non-page-aligned or
    // non-low-half address can never be mapped, and without this pre-check each such attempt
    // would silently spend a page of the process's own untyped (a self-inflicted budget leak the
    // audit noted). An already-mapped `va` still costs one page, which is process-local and
    // bounded by the untyped. The gate itself is proved: every address it admits is aligned and
    // in the low half (see `paging::is_user_page_va` and its harness).
    if !paging::is_user_page_va::<crate::arch::mmu::Format>(va) {
        return Err(Error::BadPointer);
    }
    match mmu::map_current_user_page(va, paging::Flags::user_data(), || {
        crate::memory_region::retype_page(region)
    }) {
        Ok(phys) => {
            // Record the mapping so it can be revoked before the region is ever reclaimed (§13).
            // MemoryRegion::MAP pages are process-private, but they still must be unmapped before
            // memory_region::destroy frees the region under them. The record is paid from the caller's
            // own address-space budget (phase C); if it cannot afford the record, it cannot keep
            // the mapping: an unrecorded mapping is invisible to revocation, the §13 hole.
            if !crate::revoke::record_mapping(phys, mmu::current_user_root(), va) {
                mmu::unmap_user_at(mmu::current_user_root(), va);
                return Err(Error::OutOfMemory);
            }
            Ok(0)
        }
        Err(paging::MapError::OutOfPageFrames) => Err(Error::OutOfMemory),
        Err(_) => Err(Error::BadPointer), // misaligned, already mapped, or wrong half
    }
}

/// `MemoryRegion::RETYPE_OBJ`: retype a page into a page-resident KERNEL OBJECT the caller now owns
/// (19a). The object lives in the carved page, the region is pinned (a live endpoint's page must
/// never be freed under a blocked thread), and the caller gets full rights on its own object,
/// delegation narrowing them as ever. `#[inline(never)]` for the reason `memory_region_map` gives.
#[inline(never)]
fn memory_region_retype_obj(region: u64, kind: u64) -> Result<i64, Error> {
    match kind {
        abi::objtype::RENDEZVOUS => {
            let ep = sched::create_rendezvous_from(region).ok_or(Error::OutOfMemory)?;
            // `Rights::ALL`, not a list. The comment above has always said the creator gets full
            // rights on its own object; spelling the set out meant "full" silently stopped being
            // full the day `ENUMERATE` was added, and the symptom was three steps away: init
            // could not narrow `deaths` to a right it did not itself hold, `CAP_INSERT` refused
            // the widen, and the spawn surfaced as `OutOfMemory` at a prompt. A rights set that
            // must be updated by hand whenever a right is added is rung four; `ALL` is the
            // invariant.
            let slot = sched::grant(crate::cap::rendezvous_cap(ep, Rights::ALL))
                .map_err(|_| Error::OutOfMemory)?;
            Ok(slot as i64)
        }
        // An address space (19b): the page becomes the L0 root, the untyped becomes the space's
        // backing region for tables and records (one budget model; see the abi doc and
        // design/init-and-granular-spawn.md).
        abi::objtype::ADDRESS_SPACE => {
            let name = crate::user::user_address_space_create(region).ok_or(Error::OutOfMemory)?;
            // `Rights::ALL` for the RENDEZVOUS arm's reason: "full rights on its own object" is the
            // invariant, and a hand-listed set stops being full the next time a right is added.
            // `AddressSpace` does not consult `ENUMERATE` today and is expected to when `pmap` is
            // built; holding a right nothing checks confers nothing, and not holding it is what
            // blocks a future grant.
            let slot = sched::grant(crate::cap::address_space_cap(name, Rights::ALL))
                .map_err(|_| Error::OutOfMemory)?;
            Ok(slot as i64)
        }
        // A thread (19c.3): the page holds an embryo TCB, born in no queue and not runnable
        // until CONFIGURE + START. The page is the creator's region's.
        abi::objtype::THREAD_CONTROL_BLOCK => {
            let tid = sched::create_thread_control_block(region).ok_or(Error::OutOfMemory)?;
            let slot = sched::grant(crate::cap::thread_control_block_cap(tid, Rights::ALL))
                .map_err(|_| Error::OutOfMemory)?;
            Ok(slot as i64)
        }
        _ => Err(Error::BadMethod), // no such object type
    }
}

/// `MemoryRegion::RETYPE`: retype a page into a `PageFrame` capability the caller now holds, instead of
/// mapping it in one shot. The caller gets full rights on its own frame (read, write, and the
/// right to pass it on); delegation is where those narrow. Nothing is mapped yet.
/// `#[inline(never)]` for the reason `memory_region_map` gives.
#[inline(never)]
fn memory_region_retype(region: u64) -> Result<i64, Error> {
    let phys = crate::memory_region::retype_page(region).ok_or(Error::OutOfMemory)?;
    // capability table full
    let slot = sched::grant(crate::cap::page_frame_cap(phys, Rights::ALL))
        .map_err(|_| Error::OutOfMemory)?;
    Ok(slot as i64)
}

/// `MemoryRegion::SPLIT`: carve a child untyped off this one (subdivision), so a spawner can give each
/// child its own reclaimable region. `count` is the child's page count. `#[inline(never)]` for
/// the reason `memory_region_map` gives.
#[inline(never)]
fn memory_region_split(cap: crate::cap::Cap, region: u64, count: u64) -> Result<i64, Error> {
    let child = crate::memory_region::split(region, count).ok_or(Error::OutOfMemory)?;
    // The child inherits THIS capability's rights, never more (milestone 31). SPLIT is a fresh
    // mint, so it must honor the derive-never-widens invariant by hand: a process holding a
    // spend-only (GRANT-less) untyped must not SPLIT itself a GRANT-bearing child over the same
    // memory and manufacture the right its capability withheld. `Cap::mint_child` is that
    // inheriting mint, and `split_never_widens_rights` (crates/capability) proves it never widens,
    // at the one mint site outside `derive` the caps proofs otherwise miss (milestone 35). Rights
    // narrow monotonically from the delegable root budget down; init holds that root with GRANT
    // and hands narrowed budgets on. See DECISIONS §16.
    let slot = sched::grant(cap.mint_child(crate::cap::Object::MemoryRegion(child)))
        .map_err(|_| Error::OutOfMemory)?; // capability table full
    Ok(slot as i64)
}

/// `MemoryRegion::DESTROY`: reclaim this region and every object retyped from it (object revocation):
/// tear the objects down and return the memory. Refused (`NotPermitted`) while a live thread still
/// occupies it, or if it has been split into children (destroy those first). Generational names
/// make every capability to the reclaimed objects stale on next use. `#[inline(never)]` for the
/// reason `memory_region_map` gives.
#[inline(never)]
fn memory_region_destroy(region: u64) -> Result<i64, Error> {
    sched::reclaim_region(region).map_err(|_| Error::NotPermitted)?;
    Ok(0)
}

/// `PageFrame::MAP`: map the run of `count` frames starting at `phys` at consecutive pages starting
/// at `va` in the caller's own address space (`a1` writable 0/1, `a2` an untyped slot the page
/// tables come from). Un-share is `page_frame_revoke`; this is the other half.
/// `#[inline(never)]` for the reason `memory_region_map` gives.
///
/// §102: `count` is fixed on the capability, not passed here, so this is one `MAP` call regardless
/// of the run's length; a single-page frame (`count: 1`) runs the loop below once, exactly the
/// pre-§102 behavior.
#[inline(never)]
fn page_frame_map(
    cap: crate::cap::Cap,
    phys: u64,
    count: core::num::NonZeroU64,
    va: u64,
    writable: u64,
    ut_slot: u64,
) -> Result<i64, Error> {
    let count = count.get();
    // Checked against the run's last page, not just its first: a `va` that only overflows partway
    // through the run must be refused before anything is mapped, the same "reject the cheap
    // failures before spending a page" discipline `memory_region_map` documents.
    let Some(last_va) = run_end_va(va, count) else {
        return Err(Error::BadPointer);
    };
    if !paging::is_user_page_va::<crate::arch::mmu::Format>(va)
        || !paging::is_user_page_va::<crate::arch::mmu::Format>(last_va)
    {
        return Err(Error::BadPointer);
    }
    // A read/write mapping needs WRITE on the frame; a read-only one needs READ. This is where a
    // delegated, narrowed frame is confined: a peer handed READ alone can map it to look, never
    // to change it. One check for the whole run: rights live on the capability, not per page.
    let flags = if writable != 0 {
        if !cap.rights.allows(Rights::WRITE) {
            return Err(Error::NotPermitted);
        }
        paging::Flags::user_data()
    } else {
        if !cap.rights.allows(Rights::READ) {
            return Err(Error::NotPermitted);
        }
        paging::Flags::user_rodata()
    };
    // Page tables come from an untyped the caller holds, so mapping a frame, like everything a
    // process spends, comes out of its own budget and not the kernel's.
    let ut = sched::current_cap(ut_slot).map_err(|_| Error::NoSuchSlot)?;
    let Object::MemoryRegion(region) = ut.object else {
        return Err(Error::WrongObject);
    };
    if !ut.rights.allows(Rights::WRITE) {
        return Err(Error::NotPermitted);
    }
    let root = mmu::current_user_root();
    for k in 0..count {
        let (page_phys, page_va) = (phys + k * paging::PAGE_SIZE, va + k * paging::PAGE_SIZE);
        match mmu::map_current_user_page_frame(page_va, page_phys, flags, || {
            crate::memory_region::retype_page(region)
        }) {
            Ok(()) => {
                // Record the mapping so a later REVOKE (or memory_region::destroy) can pull this
                // page out of every holder before it is reused (§13). Unrecordable means
                // unmappable, at the mapper's own expense (phase C): see MemoryRegion::MAP.
                if !crate::revoke::record_mapping(page_phys, root, page_va) {
                    mmu::unmap_user_at(root, page_va);
                    unmap_run_prefix(root, phys, va, k);
                    return Err(Error::OutOfMemory);
                }
            }
            // **All or nothing across the run** (milestone 142's review, MAJOR 2): see the same
            // rollback at `MAP_INTO`, which fails the same way for the same reasons.
            Err(e) => {
                unmap_run_prefix(root, phys, va, k);
                return Err(match e {
                    paging::MapError::OutOfPageFrames => Error::OutOfMemory,
                    // misaligned, already mapped, or wrong half
                    _ => Error::BadPointer,
                });
            }
        }
    }
    Ok(0)
}

/// The **last** virtual address a `count`-page run starting at `va` covers, or `None` if the run
/// does not fit in a `u64`.
///
/// Both mapping paths guard on this, and both used to compute it inline as
/// `va.checked_add((count - 1) * paging::PAGE_SIZE)`, where only the *addition* was checked: the
/// multiply was not, so a `count` large enough to wrap `(count - 1) * 4096` back around to a small
/// number produced a `last_va` near `va` and the guard cheerfully passed a run that spans the
/// address space (milestone 142's review, MAJOR 4). `count` is a `NonZeroU64` on the capability
/// now, so the subtraction cannot underflow; this closes the other half.
fn run_end_va(va: u64, count: u64) -> Option<u64> {
    va.checked_add(count.checked_sub(1)?.checked_mul(paging::PAGE_SIZE)?)
}

/// **Undo the `mapped` pages a failed run-map had already established**, in the space rooted at
/// `root`: unmap each page and tombstone its revocation record, leaving the space exactly as the
/// call found it.
///
/// The rollback the pre-§102 code got for free by mapping one page. A partially mapped run is
/// worse than a failed one: the caller is told `OutOfMemory` or `BadPointer` with no way to ask
/// how much of its request survived, and a shared surface half-mapped is a peer reading pixels
/// that are not there.
///
/// # BUGS
///
/// **The page tables the mapped prefix retyped are not returned**, so a failed multi-page map
/// still costs the caller whatever L3s (and their parents) the prefix needed, permanently. A
/// region is spend-only (`MemoryRegion::RETYPE` never un-retypes), so giving them back is not a
/// matter of calling something; it is the reverse of the model. The mapping is undone, the budget
/// is not. Recorded in notes/frames.md.
fn unmap_run_prefix(root: u64, phys: u64, va: u64, mapped: u64) {
    for k in 0..mapped {
        let (page_phys, page_va) = (phys + k * paging::PAGE_SIZE, va + k * paging::PAGE_SIZE);
        mmu::unmap_user_at(root, page_va);
        crate::revoke::forget_mapping(page_phys, root, page_va);
    }
}

/// `PageFrame::REVOKE`: un-share the run of `count` frames starting at `phys` from every holder and
/// delete every capability naming the run, including the caller's own. Does not reclaim the pages
/// (untyped is spend-only); that is `MemoryRegion::DESTROY`. §13, §102.
/// `#[inline(never)]` for the reason `memory_region_map` gives.
#[inline(never)]
fn page_frame_revoke(phys: u64, count: u64) -> Result<i64, Error> {
    crate::revoke::revoke_page_frame_run(phys, count);
    Ok(0)
}

/// `ThreadControlBlock::CONFIGURE`: bind an address space to an embryo thread and set its entry point and stack
/// (`a0` entry, `a1` stack, `a2` the address space cap slot, consumed). `#[inline(never)]` for the
/// reason `memory_region_map` gives.
#[inline(never)]
fn thread_control_block_configure(
    tid: crate::thread::ThreadId,
    entry: u64,
    stack: u64,
    aspace_slot: u64,
) -> Result<i64, Error> {
    // aspace_slot must name a WRITE AddressSpace cap, and it is consumed.
    let aspace = sched::current_cap(aspace_slot).map_err(|_| Error::NoSuchSlot)?;
    let Object::AddressSpace(aspace_name) = aspace.object else {
        return Err(Error::WrongObject);
    };
    if !aspace.rights.allows(Rights::WRITE) {
        return Err(Error::NotPermitted);
    }
    sched::configure_thread_control_block(tid, entry, stack, aspace_name)?;
    // Consume the address space cap: it is the thread's now, and a second bind must not find it. (The
    // space already left the registry, so the cap is inert regardless; this keeps the caller's
    // capability table honest.)
    let _ = sched::delete_current_cap(aspace_slot);
    Ok(0)
}

/// `ThreadControlBlock::CAP_INSERT`: endow the embryo with a capability (`a0` the cap to give, `a1` the rights
/// to narrow it to, `a2` the target slot: 0 is first-free, n is slot n - 1, a supervisor placing
/// a fault endpoint in the reserved slot, milestone 22). GRANT-gated and narrowing-only, exactly
/// as `SEND_CAP`: you may endow a child only with authority you were trusted to pass on, and only
/// narrowed. `#[inline(never)]` for the reason `memory_region_map` gives.
#[inline(never)]
fn thread_control_block_cap_insert(
    tid: crate::thread::ThreadId,
    src_slot: u64,
    rights: u64,
    target: u64,
) -> Result<i64, Error> {
    let src = sched::current_cap(src_slot).map_err(|_| Error::NoSuchSlot)?;
    if !src.rights.allows(Rights::GRANT) {
        return Err(Error::NotPermitted);
    }
    let narrowed = Rights::from_bits(rights as u32);
    if !narrowed.is_subset_of(src.rights) {
        return Err(Error::NotPermitted);
    }
    let target = (target != 0).then(|| target - 1);
    let child_slot = sched::thread_control_block_insert_cap(
        tid,
        crate::cap::Cap {
            object: src.object,
            rights: narrowed,
        },
        target,
    )?;
    Ok(child_slot as i64)
}

/// `Irq::WAIT`: block on the endpoint the kernel routed this interrupt to. The interrupt arrives
/// as a message (`sched::irq_notify`), exactly like any other. `#[inline(never)]` for the reason
/// `memory_region_map` gives.
#[inline(never)]
fn irq_wait(intid: u32) -> Result<i64, Error> {
    let ep = sched::irq_route(intid).ok_or(Error::WrongObject)?;
    let m = sched::ipc_recv(ep);
    Ok(m[0] as i64)
}

/// `Irq::ACK`: re-enable the interrupt at the controller. The kernel masked it when it fired; now
/// that the driver has serviced the device, it is safe to let it fire again. This names
/// `arch::irq`, not a specific controller: the GIC on aarch64, the PLIC on RISC-V.
/// `#[inline(never)]` for the reason `memory_region_map` gives.
#[inline(never)]
fn irq_ack(intid: u32) -> Result<i64, Error> {
    crate::arch::irq::enable(intid);
    Ok(0)
}

/// `Virtio`'s four register-level methods (`READ_REG`, `WRITE_REG`, `SETUP_QUEUE`, `NOTIFY`): a
/// driver's transport-level plumbing, not a step of the IPC round trip. `#[inline(never)]` for
/// the reason `memory_region_map` gives.
#[inline(never)]
fn virtio_invoke(id: usize, method: u64, a0: u64, a1: u64) -> Result<i64, Error> {
    use crate::virtio::TransportError;
    let map = |e: TransportError| match e {
        TransportError::DmaEscape => Error::DeviceRefused,
        _ => Error::WrongObject,
    };
    match method {
        abi::virtio::READ_REG => crate::virtio::read_register(id, a0)
            .map(|v| v as i64)
            .ok_or(Error::WrongObject),
        abi::virtio::WRITE_REG => crate::virtio::write_register(id, a0, a1 as u32)
            .map(|_| 0)
            .map_err(map),
        abi::virtio::SETUP_QUEUE => crate::virtio::setup_queue(id, a0 as u16, a1 as u16)
            .map(|_| 0)
            .map_err(map),
        abi::virtio::NOTIFY => crate::virtio::notify(id, a0 as u16).map(|_| 0).map_err(map),
        _ => Err(Error::BadMethod),
    }
}

/// `Rendezvous::REAP`: collect a corpse this rendezvous supervises (DECISIONS §32), the control
/// half `rendezvous::SURVEY` is the view half of. Administrative, not the IPC round trip: rare
/// enough (one call per child death, not per message) that it moves out of `invoke`'s own bytes
/// with the rest of this arm's non-fastpath methods. `#[inline(never)]` for the reason
/// `memory_region_map` gives.
#[inline(never)]
fn rendezvous_reap(ep: crate::sched::RendezvousId, tid: u64) -> Result<i64, Error> {
    sched::reap_supervised(ep, tid)?;
    Ok(0)
}

/// The body of `abi::address_space::LIST` (milestone 126's `pmap`, DECISIONS §114), pulled out of
/// [`invoke`] and marked `#[inline(never)]` on purpose: `syscall_entry` is measured flat
/// (`script/fastpath-footprint`), so a rare administrative loop inlined into the hot dispatcher
/// grows every syscall's instruction footprint for a method almost nothing calls. One call-site's
/// worth of bytes in `invoke` costs far less than this loop's own bytes would.
#[inline(never)]
fn address_space_list(frame: &mut TrapFrame, name: u64, cursor: u64) -> Result<i64, Error> {
    // The capability names a registry entry by generation; once `ThreadControlBlock::CONFIGURE` binds this space
    // to a thread, `take_user_address_space` removes it, and `root` is `None` from here on for every
    // capability that pointed at it. That is not a refusal (the capability is real and was never
    // widened past what it always held): it reads as an empty listing, symmetric to `SURVEY`'s
    // "before the scheduler exists there is no domain to report."
    let Some(root) = crate::user::user_address_space_root(name) else {
        frame.set_arg(1, 0);
        frame.set_arg(2, 0);
        return Ok(abi::survey::DONE as i64);
    };
    // Skip an entry whose `va` no longer translates (a race with revocation of a shared page this
    // space had mapped) rather than report a fabricated `kind` for it; bounded by the log's own
    // finite length, so this cannot loop forever.
    let mut cursor = cursor;
    loop {
        let (next, va) = crate::revoke::list_mapping(root, cursor);
        if next == abi::survey::DONE {
            frame.set_arg(1, 0);
            frame.set_arg(2, 0);
            return Ok(abi::survey::DONE as i64);
        }
        if let Some((_, flags)) = crate::arch::mmu::translate_at(root, va) {
            let kind = if flags.is_user_executable() {
                abi::address_space::MAP_CODE
            } else if flags.is_writable() {
                abi::address_space::MAP_RW
            } else {
                abi::address_space::MAP_RO
            };
            frame.set_arg(1, va);
            frame.set_arg(2, kind);
            return Ok(next as i64);
        }
        cursor = next;
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    /// **A run that fails partway through leaves nothing mapped** (milestone 142's review, MAJOR
    /// 2), driven through the real `MAP_INTO` handler.
    ///
    /// The regression: the pre-§102 single-page path rolled back on failure, and the widening
    /// narrowed that rollback to the one page that failed. So a three-page run whose second page
    /// could not be mapped returned `BadPointer` with its first page mapped and recorded, and the
    /// caller had no method to ask which. Silent partial state is worse than a refusal, and this is
    /// the assertion that says so: after the error, page 0 is not mapped.
    ///
    /// The failure is injected by occupying the run's middle virtual address first, so the loop's
    /// second iteration takes `AlreadyMapped`. That is the cheapest deterministic mid-run failure
    /// available; an exhausted page-table budget would do it too, and would depend on arithmetic
    /// about region sizes that has drifted before.
    #[test_case]
    fn a_partly_mapped_run_is_rolled_all_the_way_back() {
        let mut trap = TrapFrame::for_user_entry(0, 0, [0, 0, 0]);

        let space_region = crate::memory_region::create(16).expect("no space region");
        let name = crate::user::user_address_space_create(space_region).expect("no address space");
        let root = crate::user::user_address_space_root(name).expect("no root");
        let space_slot = sched::grant(crate::cap::address_space_cap(name, Rights::ALL))
            .expect("grant the address space");

        let frame_region = crate::memory_region::create(8).expect("no frame region");
        let base = crate::memory_region::retype_page(frame_region).expect("retype 0");
        let second = crate::memory_region::retype_page(frame_region).expect("retype 1");
        let third = crate::memory_region::retype_page(frame_region).expect("retype 2");
        assert_eq!(second, base + paging::PAGE_SIZE, "run must be contiguous");
        assert_eq!(third, second + paging::PAGE_SIZE, "run must be contiguous");
        let frame_slot = sched::grant(crate::cap::page_frame_run_cap(
            base,
            crate::cap::page_frame_run_len(3),
            Rights::ALL,
        ))
        .expect("grant the run");

        // Occupy the middle of where the run wants to go, out of a page the run does not name.
        let va = 0x40_0000u64;
        let squatter = crate::memory_region::retype_page(frame_region).expect("retype squatter");
        crate::user::user_address_space_map(
            name,
            va + paging::PAGE_SIZE,
            squatter,
            paging::Flags::user_data(),
        )
        .expect("the squatter maps");

        let outcome = invoke(
            &mut trap,
            space_slot,
            abi::address_space::MAP_INTO,
            va,
            frame_slot,
            abi::address_space::MAP_RW,
        );
        assert_eq!(
            outcome,
            Err(Error::BadPointer),
            "MAP_INTO over an occupied page must refuse",
        );
        assert!(
            mmu::translate_at(root, va).is_none(),
            "a failed MAP_INTO left the run's first page mapped: silent partial state",
        );
        assert_eq!(
            mmu::translate_at(root, va + paging::PAGE_SIZE).map(|(phys, _)| phys),
            Some(squatter),
            "the rollback unmapped a page the failed call never mapped",
        );

        // Give everything back, so the free-frame baseline later tests measure is undisturbed. The
        // space's region is pinned by its root page, so it comes back through `reclaim_region`
        // (which reaps the space itself) rather than through `destroy`.
        let _ = sched::delete_current_cap(space_slot);
        let _ = sched::delete_current_cap(frame_slot);
        let _ = sched::reclaim_region(space_region);
        crate::memory_region::destroy(frame_region);
    }

    /// **`SPLIT` never widens rights: a spend-only untyped splits into spend-only children.** SPLIT
    /// gates only on `WRITE`, and mints a fresh capability to the child budget, so it must honor the
    /// derive-never-widens invariant by hand or a process could manufacture authority it was denied:
    /// hold a deliberately `GRANT`-less untyped, `SPLIT` it, and receive a `GRANT`-bearing child over
    /// the same memory, then delegate what its own capability could not. This drives the real syscall
    /// path (not the region-level `memory_region::split`, which carries no rights) and pins the child's
    /// rights to the parent capability's, at the mint site the Kani proofs do not cover. The contrast
    /// arm shows the delegable root does pass `GRANT` down, so the inheritance is real, not a blanket
    /// deny. Milestone 31; see DECISIONS §16 amendment.
    #[test_case]
    fn split_inherits_the_parent_capabilitys_rights_never_widening() {
        // `for_user_entry` is the portable frame constructor (both ISAs); SPLIT does not read it.
        let mut frame = TrapFrame::for_user_entry(0, 0, [0, 0, 0]);

        // A spend-only (WRITE, no GRANT) untyped, exactly what a leaf child is handed.
        let spend_only_region = crate::memory_region::create(8).expect("a region to split");
        let parent =
            sched::grant(crate::cap::memory_region_cap(spend_only_region)).expect("grant parent");
        assert!(
            !sched::current_cap(parent)
                .unwrap()
                .rights
                .allows(Rights::GRANT),
            "the parent capability must lack GRANT for this test to mean anything",
        );

        // SPLIT through the real handler. WRITE permits it; the child must inherit WRITE only.
        let child_slot = invoke(&mut frame, parent, abi::memory_region::SPLIT, 2, 0, 0)
            .expect("split succeeds: the parent holds WRITE") as u64;
        let child = sched::current_cap(child_slot).expect("the child capability exists");
        assert!(
            !child.rights.allows(Rights::GRANT),
            "escalation: a GRANT-less untyped split into a GRANT-bearing child",
        );
        // Because it lacks GRANT it cannot be delegated: SEND_CAP and CAP_INSERT both refuse without
        // it (the exact gate at syscall.rs lines ~143 and ~319), so the child is non-transferable.
        assert!(
            !child.rights.allows(Rights::GRANT),
            "a spend-only child must be un-delegatable",
        );
        let Object::MemoryRegion(child_region) = child.object else {
            panic!("SPLIT must mint an untyped");
        };

        // Contrast: the delegable root (READ|WRITE|GRANT, what init holds) passes GRANT to its
        // children, so a spawner can hand a budget on. Inheritance, not a blanket deny.
        let root_region = crate::memory_region::create(8).expect("a root region");
        let root =
            sched::grant(crate::cap::memory_region_root_cap(root_region)).expect("grant root");
        let root_child_slot = invoke(&mut frame, root, abi::memory_region::SPLIT, 2, 0, 0)
            .expect("split the root") as u64;
        let root_child = sched::current_cap(root_child_slot).expect("root child exists");
        assert!(
            root_child.rights.allows(Rights::GRANT),
            "a delegable root must split into delegable children",
        );
        let Object::MemoryRegion(root_child_region) = root_child.object else {
            panic!("SPLIT must mint an untyped");
        };

        // Clean up: reclaim children (LIFO top of each parent) then parents, and drop the cap slots,
        // so the test returns every frame it borrowed and leaves the test thread's capability table as it found
        // it (the free-frame baseline that later tests measure against).
        sched::reclaim_region(child_region).expect("reclaim the spend-only child");
        sched::reclaim_region(spend_only_region).expect("reclaim the spend-only parent");
        sched::reclaim_region(root_child_region).expect("reclaim the root child");
        sched::reclaim_region(root_region).expect("reclaim the root parent");
        for slot in [parent, child_slot, root, root_child_slot] {
            let _ = sched::delete_current_cap(slot);
        }
    }
}
