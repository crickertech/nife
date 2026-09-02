//! **The serial keystroke source, spawned kernel-side** (milestone 192, option A).
//!
//! `keyboard_service::start_direct`'s twin, one device over: it spawns `user/src/input.rs`, the
//! plain UART receive driver, wired to a fixed endpoint it will `CALL` with
//! `line_editor::proto::OP_BYTES`. Same program, same authority and same framing the interactive
//! boot's `input` has always had; the only thing that changed is **who spawns it**, and that is
//! the whole of this module's reason to exist.
//!
//! # Why the kernel spawns it here rather than init
//!
//! On a graphical boot the endpoint a keystroke source must reach is `line_editor`'s own served
//! endpoint, which the kernel creates before init exists
//! (`kernel::user::boot_graphical_terminal`, and the reason is recorded in full there: a driver
//! init spawns can only be wired to capabilities init itself already holds). The virtio keyboard
//! is already spawned there for exactly that reason. A serial source needs the same treatment for
//! the same reason, and doing it here means `crates/system_initializer` needs no line changed and
//! cannot tell which source it got.
//!
//! # What it holds, and what it does not
//!
//! Two capabilities and one mapping, which is `user/src/input.rs`'s own documented authority
//! unchanged:
//!
//! - slot 0, the **terminal endpoint**, `WRITE` only: it may `CALL` exactly one destination,
//!   fixed here at spawn, and can name no other;
//! - slot 1, the **UART receive `Irq`**, `READ` only (WAIT and ACK, not the authority to hand it
//!   on);
//! - mapped: one page of the UART's registers, device-typed, at [`IN_UART_VA`].
//!
//! No DMA page, no `Virtio` transport, no budget, no report endpoint, and no capability naming
//! any other process. It cannot print, cannot spawn, and cannot read what anyone else typed.
//!
//! Name: **provisional** (milestone 192's lane). `input_service` is `keyboard_service`'s shape
//! applied to the program it spawns, the way `console_service` is named for `console`.

use super::*;
use crate::cap::{Rights, irq_cap_rights, rendezvous_cap};
use crate::sched::RendezvousId;

/// Where the driver maps the UART's registers. **Must match `user/src/input.rs`'s `UART_VA`**, and
/// it is the same address `crates/system_initializer`'s `IN_UART_VA` maps it at on a plain boot,
/// for the same reason: it is the wiring's fact, agreed between the two sides.
const IN_UART_VA: u64 = 0x0000_0000_00a0_0000;

/// **Wire and spawn the UART receive driver against a fixed target.**
///
/// `target` is the endpoint the driver will `CALL` with `line_editor::proto::OP_BYTES`, granted
/// here with `WRITE` and nothing else. `uart_rx_intid` is the receive interrupt the caller has
/// already routed with `crate::sched::bind_irq` and enabled; this only grants the `Irq`
/// capability, which is a per-thread act.
///
/// `None` on a machine whose console UART has no page for a device capability to be a mapping of
/// (`x86_64`, DECISIONS §121: the console is permanently kernel-resident, so there is no
/// userspace serial source to spawn at all). The caller treats that as "this boot has no serial
/// keystroke source", the same absence-rather-than-failure shape the GPU and the keyboard already
/// get.
pub fn start_direct(
    image: &'static [u8],
    target: RendezvousId,
    uart_rx_intid: u32,
) -> Option<RendezvousId> {
    if machine_has_no_device_page_for_the_console() {
        return None;
    }

    let maps = [Mapping {
        va: IN_UART_VA,
        phys: UART_PHYS,
        flags: Flags::user_device(),
    }];
    crate::sched::spawn(move || {
        run(
            image,
            Spawn {
                arg0: 0, // no role selector: `input` is its own binary and has one mode
                arg1: 0,
                arg2: 0,
                grants: &[
                    rendezvous_cap(target, Rights::WRITE), // slot 0: line_editor, directly
                    irq_cap_rights(uart_rx_intid, Rights::READ), // slot 1: WAIT / ACK
                ],
                maps: &maps,
            },
        )
    })
    .expect("could not spawn the input driver");

    Some(target)
}
