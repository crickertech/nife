//! **The JH7110 TRNG driver** (milestone 159; roadmap `design/roadmap/159-jh7110-trng-driver.md`,
//! notes/entropy.md), an alternate backend for the entropy contract (`entropy_proto`,
//! DECISIONS §44), alongside `entropy.rs`'s virtio-rng one. Same contract, same shape ("a driver,
//! not a new protocol"), different device: no virtqueue, no DMA page, just a register window.
//!
//! # It is wired now, and it still has never run
//!
//! **Something spawns this program as of 2026-09-01**: `kernel/src/user/entropy_service.rs`'s
//! `Bus` enum grew a `Jh7110` variant, and the riscv64 boot tour asks for it. The decision the
//! previous draft of this paragraph was holding open (how a spawner locates this binary in an
//! initrd) turned out to need no new answer: `entropy_service::ensure` already takes the program's
//! bytes from its caller, exactly as it does for `entropy`, so the tour reads
//! `user::program("jh7110_trng")` and hands them over. Nothing about the interactive boot changed;
//! DECISIONS §120's stopgap question is still open and still untouched by this.
//!
//! **Nothing below has run against a real TRNG**, and wiring it did not change that: QEMU has no
//! JH7110, so on every machine this repository's CI boots the wiring resolves to a skip. The
//! register sequence follows
//! `jh7110_trng`'s citations (the Linux `jh7110-trng.c` driver); the polling bounds
//! ([`POLL_TRIES`], [`LOCKUP_RETRIES`]) are placeholders with no board measurement behind them, the
//! same honest gap `entropy.rs`'s `WAIT_WAKEUPS` once was before QEMU could prove it. See the
//! roadmap doc for what remains before this can be trusted.
//!
//! # What is proven, and where
//!
//! The register decode ([`jh7110_trng::interpret`]), the device-tree query
//! ([`jh7110_trng::discover`]) and the byte buffer ([`jh7110_trng::Pool`]) are all in the crate,
//! host-tested and Kani-reachable, because none of them needs a device to be wrong in an
//! interesting way. What is left in this file is exactly the part that cannot be tested without
//! silicon: the volatile reads and writes, and the two polling bounds below.
//!
//! # Its authority, once something grants it
//!
//! - slot 0, the **request** endpoint (RECV): clients `CALL` here, `entropy_proto`'s wire format,
//!   unchanged from `entropy.rs`'s;
//! - slot 1, a **readiness** endpoint (WRITE): exactly one message, either
//!   [`proto::READY`](entropy_proto::READY) once the device has answered a first generation with
//!   bytes that are not all zero, or a
//!   [`bringup_failure`](entropy_proto::bringup_failure) word naming which step failed;
//! - mapped: **one page**, the TRNG's register block, device-typed at [`TRNG_VA`], placed there by
//!   whoever spawns this (rule 2: a base address, passed in, nothing this driver looks up). The
//!   binding's `reg` window is `0x4000` and the spawner maps `0x1000` of it, because
//!   `jh7110_trng::regs` reaches only `0x68`;
//! - and nothing else. **No third slot, no DMA page, no IRQ**, unlike `entropy.rs`'s virtio-rng
//!   backend: this device has no virtqueue and no shared buffer to negotiate, only the eight
//!   `RAND` registers, so its authority is smaller by construction, not by omission. An earlier
//!   draft named a slot 2 for a `DeviceFrame` capability; the spawner that actually exists grants
//!   the page as a `Mapping` rather than as a capability the driver holds, so the slot was
//!   describing a thing nobody hands over.
//!
//! # BUGS
//!
//! **This driver programs no clock and deasserts no reset, and that is the likeliest way its
//! first hardware boot fails.** The binding names two clock inputs (`hclk`, `ahb`) and one reset
//! line, and Linux's `jh7110-trng.c` takes all three through the clock and reset frameworks before
//! it touches a register. Nothing in this tree drives the JH7110's clock and reset generator, so
//! this driver depends on the block being left running and out of reset by whatever ran before it
//! (U-Boot), which is an assumption nobody has checked. **The symptom is specific and worth
//! knowing in advance**: register reads come back as zeros, the first refill fails, and the
//! bring-up diagnostic in [`_start`]'s readiness message is `0x0000_0000_0000_0000`. A device
//! whose registers answer at all will show a nonzero `STAT` there instead. If the zeros are what
//! the bench sees, the next piece of work is a clock/reset driver for the `SoC`, not a change
//! here; that is a milestone of its own and is proposed rather than assumed.
//!
//! **A device that answers with zeros is condemned for the whole boot, with no way back.** An
//! all-zero first generation is treated as a bring-up failure
//! ([`proto::STEP_FIRST_ALL_ZERO`](entropy_proto::STEP_FIRST_ALL_ZERO)) and this driver then
//! answers every request `NO_ENTROPY` until it is restarted, even if the block is powered on
//! underneath it a moment later. That is deliberate (a device that answers wrongly is worse than
//! one that does not answer) and it is also the case a real clock driver would want to retry;
//! milestone 220 is where that becomes worth building, and `entropy_proto`'s own `BUGS` carries the
//! probability this refusal is wrong on a working device.
//!
//! **The polling bounds below are guesses.** [`POLL_TRIES`] and [`LOCKUP_RETRIES`] have no board
//! measurement behind them. A bound that is too small reports `NO_ENTROPY` on a working device;
//! too large, and a dead device stalls the boot tour for as long as the loop takes. Nothing has
//! measured which side of that this lands on.
//!
//! **`ISTAT` is never cleared.** How this device acknowledges a status bit (write-1-to-clear
//! against write-back) was not confirmed from the summarized Linux source, so this driver reads
//! `ISTAT` and never writes it. If `RAND_RDY` latches, the second generation will appear ready
//! before it is, and the tour's two-draw check is the thing that would catch it: two identical
//! draws.
//!
//! # Why polling, not the completion interrupt
//!
//! [binding] names `interrupts = <30>`, and the Linux driver is interrupt-driven. This program
//! polls `ISTAT` instead, for one reason worth stating rather than defaulting to: routing PLIC
//! line 30 correctly (which context, which priority) is one more fact this project cannot check
//! without the board, and a wrong interrupt binding fails silently as a hang, while a wrong poll
//! bound fails loudly as `NO_ENTROPY`. This is a reversible choice (AGENTS.md's "recommend on
//! reversible forks"): switching to the interrupt once the board confirms the binding is a
//! smaller change than getting IRQ routing wrong on the first hardware boot ever attempted.
//!
//! [binding]: https://github.com/torvalds/linux/blob/master/Documentation/devicetree/bindings/rng/starfive%2Cjh7110-trng.yaml
//!
//! Name: unrecorded, provisional (introduced 2026-08-24 by milestone 159's lane), matching the
//! crate it is the volatile shell over (`jh7110_trng`), the way `coremark` and `line_editor` share
//! a crate's name with the program built from it. calef has not ratified it.

#![no_std]
// Program entry points, not the crates/ library surface milestone 68's ratchet tracks
// (DECISIONS §107): each `[[bin]]` is its own crate root with one `_start`, and 58 of them
// documenting an OS-facing ABI entry point is not what the lint is for.
#![allow(missing_docs)]
#![no_main]

use abi::rendezvous;
use entropy_proto as proto;
use jh7110_trng::{
    CTRL_EXEC_RANDRESEED, CTRL_GENE_RANDNUM, ISTAT_SEED_DONE, Outcome, Pool, interpret,
};
use tock_registers::interfaces::{Readable, Writeable};
use tock_registers::register_structs;
use tock_registers::registers::{ReadOnly, WriteOnly};
use user_rt::{recv_cap, reply, send};

register_structs! {
    /// The JH7110 TRNG's register block, migrated onto `tock_registers` (milestone 139 round 5):
    /// offsets transcribed from `jh7110_trng::regs` (itself transcribed from the Linux driver, see
    /// `crates/jh7110_trng`'s module doc), now checked at compile time instead of asserted by a
    /// hand-written comment. Unlike the NS16550 (`kernel/src/drivers/ns16550.rs`'s own module
    /// doc), this device's layout has no runtime-variable stride or width: [binding] gives one
    /// `reg` window (`reg = <0x1600C000 0x4000>`) with no `reg-shift`/`reg-io-width` knob, so there
    /// is nothing here `register_structs!`'s compile-time-fixed layout cannot express. Only the
    /// registers this driver touches are named (`CTRL`, `STAT`, `ISTAT`, `RAND0..RAND7`); `MODE`,
    /// `SMODE`, `IE`, `AUTO_RQSTS` and `AUTO_AGE` are reserved padding here, the same "not
    /// otherwise used" status `jh7110_trng::regs`'s own doc gives several of them.
    ///
    /// [binding]: https://github.com/torvalds/linux/blob/master/Documentation/devicetree/bindings/rng/starfive%2Cjh7110-trng.yaml
    #[allow(non_snake_case)]
    RegisterBlock {
        (0x00 => CTRL: WriteOnly<u32>),
        // `STAT` is read for exactly one reason: the bring-up diagnostic in `_start`. A device
        // that never answered is otherwise indistinguishable from a device that is not there.
        (0x04 => STAT: ReadOnly<u32>),
        (0x08 => _reserved_mode_smode_ie),
        (0x14 => ISTAT: ReadOnly<u32>),
        (0x18 => _reserved_pad),
        (0x20 => RAND0: ReadOnly<u32>),
        (0x24 => RAND1: ReadOnly<u32>),
        (0x28 => RAND2: ReadOnly<u32>),
        (0x2c => RAND3: ReadOnly<u32>),
        (0x30 => RAND4: ReadOnly<u32>),
        (0x34 => RAND5: ReadOnly<u32>),
        (0x38 => RAND6: ReadOnly<u32>),
        (0x3c => RAND7: ReadOnly<u32>),
        (0x40 => @END),
    }
}

/// Capability slots, by convention with whatever kernel-side wiring eventually spawns this (see
/// the module doc: none does yet).
const REQ: u64 = 0;
const READY: u64 = 1;

/// Where the spawner maps the TRNG's register page (a device-typed mapping, the same mechanism
/// `console.rs`'s `UART_VA` uses). **Must match `kernel/src/user/entropy_service.rs`'s `TRNG_VA`.**
/// Provisional and distinct from the `0x0090_0000` DMA convention `entropy.rs` and
/// `keyboard_driver.rs` share, so the two backends cannot collide if a wiring ever needs both
/// mapped in different processes at once.
const TRNG_VA: u64 = 0x0000_0000_0094_0000;

/// How many times to poll one generation attempt before giving up on it. **Not a measured bound**:
/// see the module doc. Large enough that a real device answering in microseconds never hits it in
/// practice, small enough that a device that will never answer does not hang the driver.
const POLL_TRIES: usize = 100_000;

/// How many lockup-and-reseed cycles to absorb before telling a caller the device would not
/// answer. Bounded for the reason `entropy.rs`'s `REFILL_TRIES` is: a device that faults once is
/// worth retrying, one that never stops faulting is a fact the caller needs, not something to
/// spin on forever.
const LOCKUP_RETRIES: usize = 4;

fn regs() -> &'static RegisterBlock {
    // SAFETY: TRNG_VA is our device mapping of the JH7110 TRNG's register window, handed to us at
    // spawn (rule 2: this driver is told the address, never told to look), for the whole lifetime
    // of this process. This is the same invariant the hand-written r32/w32 calls used to assert by
    // comment; register_structs! now checks every offset above at compile time instead.
    unsafe { &*(TRNG_VA as *const RegisterBlock) }
}

fn rand_words() -> [u32; 8] {
    let r = regs();
    [
        r.RAND0.get(),
        r.RAND1.get(),
        r.RAND2.get(),
        r.RAND3.get(),
        r.RAND4.get(),
        r.RAND5.get(),
        r.RAND6.get(),
        r.RAND7.get(),
    ]
}

/// Force a reseed and wait (bounded) for `ISTAT.SEED_DONE`. Called once at bring-up, mirroring
/// `jh7110-trng.c`'s own init sequence, and again whenever [`generate`] sees
/// [`jh7110_trng::Outcome::Lockup`]. `false` on a bound-out: the caller decides what that means.
fn reseed_and_wait() -> bool {
    regs().CTRL.set(CTRL_EXEC_RANDRESEED);
    for _ in 0..POLL_TRIES {
        if regs().ISTAT.get() & ISTAT_SEED_DONE != 0 {
            return true;
        }
    }
    false
}

/// Ask the device for 32 fresh bytes, retrying a hardware-reported lockup by reseeding, bounded by
/// [`LOCKUP_RETRIES`]. `None` if the device never produced an answer inside the bound: this
/// driver's whole failure mode, reported to callers as [`proto::NO_ENTROPY`] rather than a hang.
fn generate() -> Option<[u8; 32]> {
    for _ in 0..=LOCKUP_RETRIES {
        regs().CTRL.set(CTRL_GENE_RANDNUM);
        for _ in 0..POLL_TRIES {
            match interpret(regs().ISTAT.get(), rand_words()) {
                Outcome::Ready(bytes) => return Some(bytes),
                Outcome::Lockup => {
                    reseed_and_wait();
                    break; // out of the poll loop; the outer loop's next iteration retries GENE_RANDNUM
                }
                Outcome::NotReady => continue,
            }
        }
    }
    None
}

#[unsafe(no_mangle)]
pub extern "C" fn _start(_arg0: u64, _arg1: u64, _arg2: u64) -> ! {
    reseed_and_wait();

    // Draw the first 32 bytes before reporting anything, the same discipline `entropy.rs` uses:
    // "the service is up" should mean "a client that asks will be answered", not just that the
    // handshake completed. **The bytes decide the word**, which is the fix for the defect radon
    // found on 2026-09-04: this line used to send `proto::READY` unconditionally and put the
    // truth in a second word nothing was obliged to read, so a device whose clock was gated
    // reported itself ready holding zeros.
    let first = generate();
    let report = proto::readiness(first.into_iter().flatten());
    let mut pool = Pool::new();
    if let Some(bytes) = first {
        pool.refill(&mut || Some(bytes));
    }
    // Word 2 is two different facts depending on word 0, and that is deliberate: `send` carries
    // three words and the useful third one changes with the answer. On success it is how many
    // bytes are in hand (0 would mean a refill that reported success and produced nothing). On
    // failure it is the raw `(STAT << 32) | ISTAT` snapshot, which is the only diagnostic that
    // separates the bring-up failures a bench session cannot otherwise tell apart: an all-zero
    // pair says the register window read as nothing (a gated clock, an undeasserted reset, or a
    // base address that is not the TRNG), while a nonzero `STAT` with `SEEDED` clear says the
    // device is alive and the seeding sequence is what did not finish. See the roadmap doc's
    // bench procedure, which is written to read this number.
    let diagnostic = if report == proto::READY {
        pool.remaining() as u64
    } else {
        (u64::from(regs().STAT.get()) << 32) | u64::from(regs().ISTAT.get())
    };
    send(READY, report, u64::from(first.is_some()), diagnostic);

    // **A device that answered with zeros is refused for the rest of the boot**, and that is a
    // stronger response than the report word alone. The report reaches whoever wired this service;
    // a client only ever sees a reply, so a service that reported dead and went on serving its
    // zero buffer would hand out those zeros as randomness to everyone who was not watching the
    // handshake. A device that answered with *nothing* is not refused: it has told the truth at
    // every step, `take` already answers `NO_ENTROPY` while it stays dry, and it recovers by
    // itself if it starts answering.
    serve(
        pool,
        report == proto::bringup_failure(proto::STEP_FIRST_ALL_ZERO),
    )
}

/// The serve loop: one endpoint, one wait point, forever. Identical in shape to `entropy.rs`'s,
/// because the contract (`entropy_proto`) is the thing that does not change between backends.
fn serve(mut pool: Pool, refuse: bool) -> ! {
    loop {
        let (w0, cap, _) = recv_cap(REQ);
        if cap == rendezvous::NO_CAP {
            // A plain SEND on a CALL-only contract: nothing to answer. See `entropy.rs`'s
            // identical comment.
            continue;
        }
        let (count, word) = match proto::op(w0) {
            // `refuse` is a device this driver condemned at bring-up (see `_start`). It is
            // answered exactly the way a dry device is, because `NO_ENTROPY` already means the one
            // thing a client has to know: it is not getting randomness here.
            proto::GET if !refuse => pool.take(proto::want(w0), &mut generate),
            _ => (proto::NO_ENTROPY, 0),
        };
        reply(cap, count, word);
    }
}

/// **The two 8s are the same 8.** `jh7110_trng::Pool::take` clamps to the width of the word it
/// answers with; `entropy_proto::want` clamps to the width of the word the wire carries. This file
/// is the only one that depends on both, so it is where the agreement can be checked, and a
/// compile-time assert is the rung this project reaches for when a fact can be made unrepresentable
/// rather than remembered.
const _: () = assert!(jh7110_trng::WORD_BYTES == proto::MAX_BYTES);

user_rt::panic_handler!();

// There is no capability slot for the registers, and that is worth a sentence rather than a
// silence: this driver reaches them by direct volatile access at `TRNG_VA`, the same way
// `console.rs` reaches the UART, not through a kernel-mediated `invoke` the way the virtio
// backends reach `Virtio` capabilities. The authority is the mapping the spawner installed; the
// driver holds no name for it.
