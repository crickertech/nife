//! **`pmap`: what is mapped, and who is allowed to ask** (milestone 126, notes/process-view.md,
//! DECISIONS §114).
//!
//! The whole program is: walk one address space with `abi::aspace::LIST`, then say what it found on
//! one stream and what went wrong on the other. `ps`'s shape, one object type over. The listing
//! itself is `crates/pmap`, which runs on the host in milliseconds; what lives here is the syscall
//! and the two sinks.
//!
//! Name: recorded (milestone 126, and notes/naming.md). `pmap` is the name every reader already
//! knows from outside this project. The crate beside it shares the name deliberately, the same
//! crate-and-program pair `ps`, `coremark`, `line_editor` and `compositor` already are.
//!
//! # `ENUMERATE`, not `WRITE`, is the whole demonstration
//!
//! `abi::aspace::MAP_INTO` needs `WRITE` on the address-space capability: the authority to shape
//! it. `abi::aspace::LIST` needs `ENUMERATE`: the authority to look. A `pmap` granted `ENUMERATE`
//! alone can list every mapping and cannot add, remove or change one, the identical split `ps`
//! demonstrates between naming a domain's members (`ENUMERATE`) and reaping one (`READ`).
//!
//! # Capability contract
//!
//! | slot | what | why |
//! |---|---|---|
//! | 0 | the output sink, `WRITE` | where the table goes |
//! | [`ASPACE_SLOT`] | the address space, `ENUMERATE` | the space whose mappings it may **name** |
//! | [`DIAG_SLOT`](grant_plan::DIAGNOSTICS_SLOT) | the diagnostics sink, `WRITE` | where a refusal goes |
//!
//! Two capabilities beyond the output sink, and **neither of them can map or unmap a page**.
//!
//! # There is nowhere this program can be run from the interactive prompt today
//!
//! **This is a real, load-bearing limitation, not a caveat to skim.** `ps` reaches the shell
//! because `Manifest::domain` tells `system_initializer` which live supervision endpoint (`deaths`)
//! to place in its capability table. Nothing plays that role for an address space: every `Object::Aspace`
//! capability in this tree is minted and consumed **within the thread that built it**
//! (`RETYPE_OBJ(ASPACE)` -> `MAP_INTO`* -> `Tcb::CONFIGURE`, which removes the space from the
//! registry the instant it binds to a thread), and nothing shipped here ever delegates one to a
//! different program (checked: `user/src/builder.rs`, `crates/supervision_proto`, `user/src/hello.rs`,
//! `user/src/os_primitives_benchmarker.rs`, the only sites that mint an `Object::Aspace` at all --
//! DECISIONS §114's required audit). So there is no manifest field for this program to declare and
//! no wiring for `system_initializer` to add: there is nothing alive anywhere in the system to hand
//! it.
//!
//! What is real: the kernel method (`abi::aspace::LIST`, gated by `Rights::ENUMERATE`, refusing
//! `MAP_INTO` to the same capability) and this program, proven end to end against a genuine
//! `Object::Aspace` built the same way a spawner builds one, in `kernel::user::pmap_tests`. What is
//! missing is a design for a builder to hand a narrowed, still-live view of a space it is
//! constructing to a third party *before* `CONFIGURE` consumes its own copy -- a real gap this
//! milestone's own execution found, named here rather than designed around, per `crates/pmap`'s
//! `BUGS`.
//!
//! # EXAMPLES
//!
//! From `kernel::user::pmap_tests`, which is the only place this program runs today:
//!
//! ```text
//! $ pmap
//!               VA  PERM
//!       0x00400000  r-x
//!       0x00500000  rw-
//!
//! $ pmap        pmap: this address space may be mapped into, but not looked at: no ENUMERATE
//! ```

#![no_std]
// Program entry points, not the crates/ library surface milestone 68's ratchet tracks
// (DECISIONS §107): each `[[bin]]` is its own crate root with one `_start`, and this is one more
// of the 58 that document an OS-facing ABI entry point rather than a library the lint is for.
#![allow(missing_docs)]
#![no_main]

use core::sync::atomic::{AtomicBool, Ordering};

use user_rt::{exit, invoke, list, send};

/// The output sink: where the table goes. Slot 0 is where every spawned program's output lands.
const REPORT: u64 = 0;

/// **The address space under view**: an `Aspace` capability, `ENUMERATE`. Everything this program
/// can see comes through here and nothing else in the capability table names a space.
///
/// Not a `grant_plan` constant like `ps`'s [`grant_plan::DOMAIN_SLOT`], on purpose: `grant_plan` is
/// the shell's command-line planning surface, and there is no command line that can grant this
/// program anything (see the module docs). The number is chosen to sit in the same reserved-low-slot
/// range `DOMAIN_SLOT` (7) and `DIAGNOSTICS_SLOT` (8) already use, so a caller that does eventually
/// wire this up is not the first program to want a fixed low slot for "the thing under view."
const ASPACE_SLOT: u64 = 7;

/// The declared second stream (DECISIONS §67): complaints about the run, never about the listing.
/// Reused from `grant_plan` rather than a second constant minted for the same number: every program
/// with a diagnostics stream in this tree uses slot 8, whether or not the shell can reach it.
const DIAG_SLOT: u64 = grant_plan::DIAGNOSTICS_SLOT;

/// Whether this process was granted a second stream, decided once in [`_start`].
static HAS_DIAG: AtomicBool = AtomicBool::new(false);

#[unsafe(no_mangle)]
pub extern "C" fn _start(_a0: u64, _a1: u64, _a2: u64) -> ! {
    HAS_DIAG.store(granted(DIAG_SLOT), Ordering::Relaxed);

    // Collect first, complain second, print third (DECISIONS §67), `ps`'s shape verbatim: a
    // listing cannot know its complaints up front (the space may be refused on the first call or
    // vanish on a later one), so the whole thing goes into a buffer before either stream is
    // touched.
    let mut rows = [pmap::Row::default(); pmap::MAX_ROWS];
    let found = pmap::collect(&mut rows, &mut |cursor| list(ASPACE_SLOT, cursor));

    found.write_diagnostics(&mut |bytes| write_on(diag_slot(), bytes));
    diag_end();

    found.write_report(&mut |bytes| write_on(REPORT, bytes));
    send(REPORT, byte_sink_proto::eof(), 0, 0);
    exit();
}

/// Which endpoint a complaint goes to: the declared second stream when there is one, and the output
/// otherwise. `ps`'s fallback, verbatim: honest rather than a silent drop, and what a test harness
/// that wires no second sink gets.
fn diag_slot() -> u64 {
    if HAS_DIAG.load(Ordering::Relaxed) {
        DIAG_SLOT
    } else {
        REPORT
    }
}

/// **Close the second stream**, which is not tidiness: its reader waits for the end before it reads
/// the first, so a `pmap` that exited without this would leave the prompt blocked.
fn diag_end() {
    if HAS_DIAG.load(Ordering::Relaxed) {
        send(DIAG_SLOT, byte_sink_proto::eof(), 0, 0);
    }
}

/// Write bytes to an endpoint under the sink contract, sixteen at a time.
fn write_on(slot: u64, bytes: &[u8]) {
    let mut rest = bytes;
    while !rest.is_empty() {
        let (w0, w1, w2, n) = byte_sink_proto::pack(rest);
        send(slot, w0, w1, w2);
        rest = &rest[n..];
    }
}

/// Whether a capability is in `slot`, without touching whatever it names. `ps`'s probe, verbatim:
/// invoke a method number no object type defines, so the call can only be refused, and read which
/// refusal came back.
fn granted(slot: u64) -> bool {
    /// A method number no object type defines, so the invocation can only ever be refused.
    const NO_SUCH_METHOD: u64 = 0xffff;
    // SAFETY: a syscall that cannot succeed; the kernel validates the slot before the method.
    let r = unsafe { invoke(slot, NO_SUCH_METHOD, 0, 0, 0) };
    r != abi::Error::NoSuchSlot as i64
}

user_rt::panic_handler!();
