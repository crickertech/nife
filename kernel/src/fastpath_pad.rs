//! E3's footprint-perturbation experiment (milestone 134,
//! design/roadmap/134-the-measurements-that-decide.md): pad the IPC fastpath's call graph with a
//! large, reachable-but-never-taken function, so the fastpath's *measured* footprint
//! (`script/fastpath-footprint`) roughly doubles while the *executed* work on a benchmarked round
//! trip stays exactly what it was. Liedtke's claim is that footprint costs cache, not that any
//! particular byte is slow; padding tests that without a cache counter, by making the tool that
//! already measures footprint (milestone 132) disagree or agree with a real latency change.
//!
//! Gated on the `fastpath_pad` Cargo feature (kernel/Cargo.toml): this module, the call into it,
//! and the per-arch nop sleds it calls do not exist in an ordinary build, so nothing here costs a
//! normal boot anything. `crate::sched::ipc_send` calls [`maybe_pad`] unconditionally when the
//! feature is on; whether that call actually reaches the padding is decided at runtime by
//! [`core::hint::black_box`], not by a flag anyone sets, which is the property that makes the
//! padding *resident* (present in `.text`, walked by `script/fastpath-footprint`'s static closure)
//! while staying *dead* (never executed, so a benchmark run with the feature on does exactly the
//! IPC work a run without it does, plus this one guard).
//!
//! The guard itself is not free: `black_box(false)` still costs a real compare-and-branch on every
//! `ipc_send`, in both the padded and the un-padded-by-flag sense, because the feature being *on*
//! is what adds it, whether or not the branch is ever taken. That is a real, if small, confound on
//! E3's comparison and it is named in this milestone's roadmap doc rather than hidden: a purer
//! design would need the compiler to see "always false" at compile time, which is exactly the
//! condition LLVM is licensed to delete, undoing the whole point. `black_box` is the standard way
//! to keep a dead branch reachable without also making it free; the residual cost is on the order
//! of a nanosecond, which the observed IPC round trip (low microseconds under HVF) comfortably
//! dwarfs. See this file's callers for where the guard sits.
//!
//! # The layout control (2026-09-19)
//!
//! **Why a boolean was not enough.** On radon on 2026-09-04 the padded kernel was 1.49% slower on
//! `call_reply` and 3.01% *faster* on `ipc_rtt_el0`. The sled is never executed, so nothing it adds
//! can make a round trip faster; what it does is move every symbol linked after it (on riscv64 the
//! whole trap path, `trap_entry` onward, sits after it, and the `sched` IPC functions sit before
//! it). A shift changes which L1i sets each hot line lands in, where 64-byte line boundaries fall,
//! and whether a branch target is 8-byte aligned, which the U74's BTB needs for a zero-bubble
//! prediction (`SiFive`'s U74-MC Core Complex Manual 21G3.02.00, §4.2.6). One padded kernel against
//! one un-padded kernel therefore reports footprint and layout summed.
//!
//! **Two numbers, one mechanism, both read at build time** by kernel/build.rs:
//!
//! - `NIFE_FASTPATH_PAD=<units>`, [`PAD_UNITS`]: the sled's length in units of what this feature
//!   has always linked. Unset is 1, the sled the 2026-09-04 session booted. 0 is the
//!   dose-response's zero: the guard and a sled that is only `ret`, so every rung executes the same
//!   instructions and the guard stops being a difference between conditions.
//! - `NIFE_FASTPATH_SHIFT=<bytes>`, [`SHIFT_BYTES`]: zero bytes appended after the sled's `ret`,
//!   under their own symbol (`fastpath_layout_shift`) that nothing references. Being in the sled's
//!   input section places them exactly where the pad goes and keeps them past `--gc-sections` with
//!   no retain flag, since the section survives because the sled is referenced. Zeros rather than
//!   `nop`s because both ISAs decode zero as an illegal instruction: a block that were ever
//!   executed would trap rather than run.
//!
//! A layout variant is `PAD=0` with a non-zero `SHIFT`: an un-padded kernel whose trap path is
//! moved and which executes nothing new. notes/footprint-perturbation.md has the images E3 builds,
//! the sizes and why, and `script/fastpath-footprint --layout` is the proof that every one of them
//! executes the same instructions (a hash of the hot path with addresses normalised away).
//!
//! **Why environment variables and not a feature per size, which was built first and measured.**
//! A feature's name enters cargo's `-C metadata` hash, which renames every symbol and reorders
//! codegen units; seven sibling features moved code linked *before* the sled by up to 11 KB on the
//! radon card build, so each image was an uncontrolled layout draw and the designed sizes were
//! noise on top of it. The environment does not enter that hash.

/// The sled's length, in units of `UNIT_NOPS` (each arch's `fastpath_pad.rs`).
pub const PAD_UNITS: usize = parse(env!("NIFE_FASTPATH_PAD"));

/// Unreferenced zero bytes after the sled; non-zero only in a layout variant.
pub const SHIFT_BYTES: usize = parse(env!("NIFE_FASTPATH_SHIFT"));

/// The two numbers as data, for the bench boot to print (`bench-probe: fastpath_pad ...`), so a
/// card can say which E3 image it is. Data rather than immediates on purpose: on riscv64 `li a0, 0`
/// is two bytes and `li a0, 3280` is eight, so printing constants would change the length of the
/// printing function and move everything linked after it, which is the variable being controlled.
#[cfg(feature = "bench")]
pub static BUILD: [usize; 2] = [PAD_UNITS, SHIFT_BYTES];

/// Decimal, checked by kernel/build.rs before it gets here; a const fn because `str::parse` is not.
const fn parse(s: &str) -> usize {
    let b = s.as_bytes();
    let mut i = 0;
    let mut n = 0;
    while i < b.len() {
        n = n * 10 + (b[i] - b'0') as usize;
        i += 1;
    }
    n
}

/// Call from a root the fastpath-footprint closure walks (today: [`crate::sched::ipc_send`]).
/// Branches into the arch's nop sled only if [`core::hint::black_box`] fails to prove its input
/// false, which it never does, so this costs one untaken branch and nothing else at runtime.
#[inline(never)]
pub fn maybe_pad() {
    if core::hint::black_box(false) {
        // SAFETY: the arch nop sled is a leaf function (no memory access, no stack frame beyond
        // its own `ret`) reached with the platform's ordinary call ABI; it touches nothing this
        // caller owns. It is also never actually called (see the module doc), so its body has no
        // runtime effect to be unsound about; this `unsafe` covers the FFI call shape only.
        unsafe { crate::arch::fastpath_pad_body() };
    }
}
