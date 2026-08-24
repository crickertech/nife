//! **The JH7110 TRNG driver** (milestone 159; roadmap `design/roadmap/159-jh7110-trng-driver.md`,
//! notes/entropy.md), an alternate backend for the entropy contract (`entropy_proto`,
//! DECISIONS §44), alongside `entropy.rs`'s virtio-rng one. Same contract, same shape ("a driver,
//! not a new protocol"), different device: no virtqueue, no DMA page, just a register window.
//!
//! # This is not wired to anything, and has never run
//!
//! **No kernel-side wiring spawns this program.** `kernel/src/user/entropy_service.rs`'s `Bus`
//! enum has `Mmio` and `Pci`; there is no `Bus::Jh7110`, deliberately: adding one needs a real
//! decision about how the service that wires it locates this binary in an initrd, and that is
//! design surface a lane should not settle alone (AGENTS.md's "recommend on reversible forks, give
//! options on irreversible ones" and the milestone's own explicit "do not wire this into the
//! interactive boot"). What exists here is the driver itself, buildable and type-checked against
//! `entropy_proto` and `user_rt` today, so wiring it in later is choosing capability slots for an
//! already-working program rather than writing one under time pressure on the day the board is on
//! the bench.
//!
//! **Nothing below has run against a real TRNG.** The register sequence follows
//! `jh7110_trng`'s citations (the Linux `jh7110-trng.c` driver); the polling bounds
//! ([`POLL_TRIES`], [`LOCKUP_RETRIES`]) are placeholders with no board measurement behind them, the
//! same honest gap `entropy.rs`'s `WAIT_WAKEUPS` once was before QEMU could prove it. See the
//! roadmap doc for what remains before this can be trusted.
//!
//! # Its authority, once something grants it
//!
//! - slot 0, the **request** endpoint (RECV): clients `CALL` here, `entropy_proto`'s wire format,
//!   unchanged from `entropy.rs`'s;
//! - slot 1, a **readiness** endpoint (WRITE): one message once the device answers a first
//!   generation, or never, if bring-up fails;
//! - slot 2, a **`DeviceFrame`** capability for the TRNG's register window, mapped at [`TRNG_VA`]
//!   by whoever spawns this (rule 2: a base address, passed in, nothing this driver looks up);
//! - mapped: nothing else. **No DMA page and no IRQ slot**, unlike `entropy.rs`'s virtio-rng
//!   backend: this device has no virtqueue and no shared buffer to negotiate, only the eight
//!   `RAND` registers, so its authority is smaller by construction, not by omission.
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
    CTRL_EXEC_RANDRESEED, CTRL_GENE_RANDNUM, ISTAT_SEED_DONE, Outcome, interpret, regs,
};
use user_rt::{recv_cap, reply, send};

/// Capability slots, by convention with whatever kernel-side wiring eventually spawns this (see
/// the module doc: none does yet).
const REQ: u64 = 0;
const READY: u64 = 1;
const REG: u64 = 2;

/// Where a future spawner maps the TRNG's register window (a `DeviceFrame` mapping, the same
/// mechanism `console.rs`'s `UART_VA` uses). Provisional and distinct from the `0x0090_0000` DMA
/// convention `entropy.rs` and `kbd.rs` share, so the two backends cannot collide if a future
/// wiring ever needs both mapped in different processes at once.
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

fn r32(off: u64) -> u32 {
    // SAFETY: TRNG_VA is our device mapping of the JH7110 TRNG's register window, handed to us at
    // spawn (rule 2: this driver is told the address, never told to look), for the whole lifetime
    // of this process. `off` is one of `jh7110_trng::regs`' offsets, all inside the 0x4000-byte
    // window `jh7110_trng`'s DTB fixture pins.
    unsafe { core::ptr::read_volatile((TRNG_VA + off) as *const u32) }
}

fn w32(off: u64, v: u32) {
    // SAFETY: as above.
    unsafe { core::ptr::write_volatile((TRNG_VA + off) as *mut u32, v) }
}

fn rand_words() -> [u32; 8] {
    [
        r32(regs::RAND0),
        r32(regs::RAND1),
        r32(regs::RAND2),
        r32(regs::RAND3),
        r32(regs::RAND4),
        r32(regs::RAND5),
        r32(regs::RAND6),
        r32(regs::RAND7),
    ]
}

/// Force a reseed and wait (bounded) for `ISTAT.SEED_DONE`. Called once at bring-up, mirroring
/// `jh7110-trng.c`'s own init sequence, and again whenever [`generate`] sees
/// [`jh7110_trng::Outcome::Lockup`]. `false` on a bound-out: the caller decides what that means.
fn reseed_and_wait() -> bool {
    w32(regs::CTRL, CTRL_EXEC_RANDRESEED);
    for _ in 0..POLL_TRIES {
        if r32(regs::ISTAT) & ISTAT_SEED_DONE != 0 {
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
        w32(regs::CTRL, CTRL_GENE_RANDNUM);
        for _ in 0..POLL_TRIES {
            match interpret(r32(regs::ISTAT), rand_words()) {
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

/// The service's running state: 32 bytes from the last [`generate`] and how many are still ours
/// to give. Not a pool: `cursor` only ever moves forward, so no byte is served twice. The same
/// shape `entropy.rs`'s `Pool` is, at a quarter the size (32 bytes here vs. `entropy.rs`'s 256:
/// there is no round-trip cost to amortize over here, since there is no device round trip at all,
/// only a register poll).
struct Pool {
    buf: [u8; 32],
    cursor: usize,
    filled: usize,
}

impl Pool {
    fn refill(&mut self) -> bool {
        match generate() {
            Some(bytes) => {
                self.buf = bytes;
                self.cursor = 0;
                self.filled = 32;
                true
            }
            None => false,
        }
    }

    /// Take `n` bytes, as a little-endian word plus a count. Gathers across a refill boundary the
    /// same way `entropy.rs`'s `Pool::take` does, so a client's request can straddle two
    /// generations without the client ever seeing the seam.
    fn take(&mut self, n: u64) -> (u64, u64) {
        let mut word = 0u64;
        let mut got = 0u64;
        while got < n {
            if self.cursor == self.filled && !self.refill() {
                break;
            }
            let run = (n - got).min((self.filled - self.cursor) as u64);
            for i in 0..run {
                let at = self.cursor + i as usize;
                word |= (self.buf[at] as u64) << (8 * (got + i));
                // Zero behind the cursor: the same hygiene `entropy.rs`'s `Pool::take` keeps, so a
                // byte a client now holds is not also still sitting in a buffer this long-lived
                // process keeps for the rest of the boot.
                self.buf[at] = 0;
            }
            self.cursor += run as usize;
            got += run;
        }
        (got, word)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn _start(_arg0: u64, _arg1: u64, _arg2: u64) -> ! {
    reseed_and_wait();

    let mut pool = Pool {
        buf: [0; 32],
        cursor: 0,
        filled: 0,
    };
    // Fetch the first 32 bytes before reporting ready, the same discipline `entropy.rs` uses:
    // "the service is up" should mean "a client that asks will be answered", not just that the
    // handshake completed.
    let first = pool.refill();
    send(READY, proto::READY, u64::from(first), pool.filled as u64);

    serve(pool)
}

/// The serve loop: one endpoint, one wait point, forever. Identical in shape to `entropy.rs`'s,
/// because the contract (`entropy_proto`) is the thing that does not change between backends.
fn serve(mut pool: Pool) -> ! {
    loop {
        let (w0, cap, _) = recv_cap(REQ);
        if cap == rendezvous::NO_CAP {
            // A plain SEND on a CALL-only contract: nothing to answer. See `entropy.rs`'s
            // identical comment.
            continue;
        }
        let (count, word) = match proto::op(w0) {
            proto::GET => pool.take(proto::want(w0)),
            _ => (proto::NO_ENTROPY, 0),
        };
        reply(cap, count, word);
    }
}

user_rt::panic_handler!();

// `REG` (slot 2) is named for the reader who wonders why a device mapping is granted but no
// capability invoke touches it: this driver reaches its registers by direct volatile access at
// `TRNG_VA`, the same as `console.rs`'s UART, not through a kernel-mediated `invoke` the way the
// virtio backends reach `Virtio` capabilities. The slot's only job is to be the authority a
// spawner's `Mapping` stands behind; nothing in this file reads the capability itself.
const _: u64 = REG;
