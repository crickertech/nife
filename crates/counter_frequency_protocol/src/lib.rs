//! **The `x86_64` timebase page** (milestone 161's `cntfrq` follow-up).
//!
//! aarch64 answers "how many [`user_mode_runtime::now`](../user_mode_runtime/fn.now.html) ticks make a second" by
//! reading `CNTFRQ_EL0`, a register the machine states. `x86_64` has no such register: the TSC's
//! rate is either reported by `CPUID` leaf `0x15` (on parts that implement it) or has to be
//! measured against a timer the kernel already trusts, and either way the number is known **once,
//! at boot, in ring 0**, then never again. This crate is the one definition of the page that
//! carries that number to every process, so the kernel (the one writer) and `user_mode_runtime::cntfrq`
//! (every reader) cannot drift on its layout or its fixed address. The same split `clock_protocol`
//! makes for the wall clock and `environment_protocol` makes for `TZ`/`LANG`/`TERM`.
//!
//! # Why a page, and why unconditional
//!
//! `clock_protocol`'s page is capability-gated: a process either holds a mapping or it does not, and
//! not knowing the wall clock is a real, representable state. The timebase page is not like that.
//! Every `x86_64` process that calls [`now`](../user_mode_runtime/fn.now.html) needs a rate to turn ticks
//! into seconds, ambiently, with no capability to ask for (aarch64 and RISC-V both give this for
//! free: a register read and a build-time constant respectively, neither gated on anything). So
//! `kernel::user::load` (the arch-neutral function every ordinary ELF-loaded test fixture passes
//! through) and every kernel-side function that builds a top-level process's space by hand instead
//! (`spawn_init` and the handful of `spawn_<program>`-shaped test harnesses; see
//! `kernel::user::map_x86_timebase_page`, the one place that logic lives) map this page
//! **unconditionally** into the process they build, the same "grant is unconditional, a zeroed page
//! reads as unknown" shape `kernel::user::boot_clock_page` already uses for init's clock page: a
//! boot on which calibration somehow has not run yet (never observed in practice; the kernel
//! measures the TSC well before the first process is loaded) still hands out a page, and
//! [`TimebasePage::hz`] reads that as `None` rather than a fabricated number.
//!
//! **A process built by the userspace ELF loader instead** (`supervision_protocol::build_child_space`,
//! which every `root_supervisor`/`spawner`/`system_initializer`/`hello`-role child, `coremark`
//! included, actually goes through) cannot reach the kernel's real page at all: nothing hands that
//! crate a capability naming the kernel's specific physical frame, so it maps a *freshly retyped,
//! zeroed* placeholder from the child's own budget instead, exactly the amount of forwarding a
//! userspace crate can do without one. See `user_mode_runtime::cntfrq`'s own `BUGS` section for what that
//! means for the number such a process reads back.
//!
//! # One writer, then read-only, no seqlock
//!
//! Same shape as `environment_protocol::ConfigPage`: the kernel writes this page's bytes once, before
//! mapping it read-only into any process, so there is nothing to race and no seqlock is needed.
//! Unlike the clock page, nothing ever updates it again after boot: the TSC's rate does not
//! change while the machine is running (see the crate's own `BUGS` section for the one caveat
//! that survives from `arch::x86_64::timer`'s own docs).
//!
//! # Examples
//!
//! ```
//! use counter_frequency_protocol::{TimebasePage, build_page};
//!
//! let bytes = build_page(1_000_000_000);
//! // SAFETY: `bytes` is a live, aligned buffer of exactly `PAGE_BYTES` for this block.
//! let page = unsafe { TimebasePage::new(bytes.as_ptr() as u64) };
//! assert_eq!(page.hz(), Some(1_000_000_000));
//! ```
//!
//! A zeroed frame (a page nobody has written into) reads as "unknown", never as a fabricated
//! rate:
//!
//! ```
//! use counter_frequency_protocol::TimebasePage;
//!
//! let zeroed = [0u8; counter_frequency_protocol::PAGE_BYTES];
//! // SAFETY: as above.
//! let page = unsafe { TimebasePage::new(zeroed.as_ptr() as u64) };
//! assert_eq!(page.hz(), None);
//! ```
//!
//! # BUGS
//!
//! - **The TSC is assumed invariant, the same assumption `arch::x86_64::timer`'s own docs name.**
//!   `CPUID.80000007H:EDX[8]` says whether the part keeps a constant TSC rate across frequency and
//!   idle-state changes; this page is written once from whatever the kernel measured or read at
//!   boot and never revisited, so a part that does not hold that bit and later changes its TSC
//!   rate would leave every process reading a stale number. QEMU's TSC is invariant. Checking the
//!   bit is milestone 87's, the same note `arch::x86_64::timer`'s own `BUGS` section carries.
//! - **A process built by `supervision_protocol::build_child_space` reads a placeholder, not the
//!   kernel's real number**, for the reason the crate docs above state: no capability carries the
//!   real frame down into that crate. This is the honest residue of this milestone's scope, not an
//!   oversight; closing it needs a capability handed from whoever built the calling process,
//!   forwarded through every generation of the supervision tree, which is real plumbing across
//!   `spawn_init` and every builder role this milestone did not reach.
//! - **A process spawned any way other than those two paths** (there is currently exactly one on
//!   this architecture: `kernel::user::x86_userspace_demo`'s hand-assembled children, which run raw
//!   machine code and never call `user_mode_runtime::cntfrq`) does not get this page mapped at all, and a
//!   call to [`user_mode_runtime::cntfrq`](../user_mode_runtime/fn.cntfrq.html) from such a process would fault on the
//!   unmapped read. A future spawn path that does not go through `kernel::user::load`,
//!   `kernel::user::map_x86_timebase_page`, or `supervision_protocol::build_child_space` must map this
//!   page too (real or placeholder), or must not link anything that calls `cntfrq`.
//!
//! Name: provisional, and ruled: calef ruled **`counter_frequency_proto`** on 2026-09-13, working
//! the unratified worklist. Minted by milestone 161's `cntfrq` follow-up lane as `timebase_proto`.
//!
//! **Performed at milestone 265 on 2026-09-14, and the name landed is `counter_frequency_protocol`**,
//! not `counter_frequency_proto`: the stem is calef's 2026-09-13 ruling and the suffix is 265's
//! 2026-09-05 one, and applying them separately would have renamed this crate twice for no reason.
//! That is the same treatment `mdns_proto` and `ntp_proto` got, and **265's own block did not list
//! this crate among them**: it named four stems ruled that day and there were five. The block now
//! lists five. The name stays `provisional` because calef ruled the stem and has not been shown the
//! whole name.
//!
//! **What `Refused` above says, exactly**: `timebase_proto`, the stem and suffix together. Applying
//! 265's suffix rule to that name mechanically gives `timebase_protocol`, which is **not** on record
//! as refused, so `script/names --check` would not have caught it: that check matches a refused name
//! against a live one, and these differ by three letters. The refusal is of the stem. Nothing
//! mechanical stood between this crate and a refused stem; reading this block did.
//!
//! **The block this replaces was the thinnest in the tree**, and that is worth recording rather
//! than quietly improving: it read *"Name: provisional (this lane, milestone 161's `cntfrq`
//! follow-up). calef names crates."* One line, no argument, no refusals. Nothing had ever been
//! weighed, so there was no losing case for a reader to weigh against, which is the state
//! milestone 115's provenance convention exists to prevent.
//!
//! **The ratified name mirrors the register this page substitutes for.** aarch64 answers "how many
//! ticks make a second" from `CNTFRQ_EL0`, which is literally the *counter frequency*; `x86_64` has
//! no such register, so the kernel learns the number once and publishes it here. A reader who knows
//! why the crate exists meets a name that says so, and one who does not is told. The page carries
//! exactly that number and a magic, which is the other half of the argument: `timebase` named the
//! oscillator where the contents are the frequency.
//!
//! Refused `timebase_proto`, above. Refused `tick_rate_proto`, accurate and plainer but dropping
//! the connection to `CNTFRQ_EL0` that explains the crate's existence. Refused `tsc_frequency_proto`
//! for naming `x86_64`'s counter specifically, when the contract is architecture-neutral even
//! though today's only consumer is not, and `TSC` would want expanding besides. Refused
//! `x86_64_timebase_proto` on the same ground: `jh7110_` qualifies by chip because this tree will
//! have a second system on a chip, but this page is the general answer to "the counter frequency is
//! not in a register", and qualifying it by today's only caller would go stale the first time
//! another architecture needed it.
//!
//! **The `_proto` suffix was not reopened here and was reopened three weeks later**, which is
//! worth leaving standing rather than editing away. This block said the suffix was settled because
//! `clock_proto` had been ratified with it on 2026-08-23 and this page uses the same `MAGIC` shape,
//! so it was an established family rather than an open question. calef reopened it himself on
//! 2026-09-05, on being shown this very crate: *"I think `_proto` was lazy on my part. It should
//! have been `_protocol` globally to differentiate from prototype."* An established family is
//! evidence about consistency and not about whether the thing being copied was right.

#![cfg_attr(not(test), no_std)]

/// The page's first eight bytes: an assembled page from an unrecognized or zeroed frame. ASCII,
/// unpadded, the same shape `clock_protocol::MAGIC` and `environment_protocol::MAGIC` use.
pub const MAGIC: [u8; 8] = *b"TIMEBAS1";

/// **The slowest counter this tree will believe**, 100 kHz.
///
/// Every rate check in this tree used to validate against *zero* and nothing else: aarch64 asserted
/// `freq > 0`, riscv64 asserted `hz > 0`, and `x86_64`'s PIT calibration had no band at all. A
/// firmware that reports 1 Hz, or a calibration that returns garbage because the host descheduled
/// the vCPU mid-measurement, passes all three and then poisons every number derived from it,
/// silently, which is the failure this whole file exists to prevent one layer up.
///
/// **The bound is deliberately far below anything real**, because a false refusal on somebody's
/// silicon is worse than the failure it prevents: it turns a machine that would have run into a
/// machine that will not boot. The slowest counter in this tree's world is radon's 4 MHz `time`
/// CSR; the 32.768 kHz watch crystal is the slowest thing anyone drives a timer from anywhere, and
/// 100 kHz sits above it on purpose, because nothing here is a watch. Anything under this is a
/// measurement that failed, not a part that is slow.
pub const MIN_PLAUSIBLE_HZ: u64 = 100_000;

/// **The fastest counter this tree will believe**, 100 GHz, and the same reasoning as
/// [`MIN_PLAUSIBLE_HZ`] read from the other end. No part ticks a counter at 100 GHz and none will
/// for a long time (the fastest here is a few GHz of TSC), so a number above this came from a
/// division that went the wrong way or a register that was never written. Generous by two orders of
/// magnitude on purpose: this band exists to catch nonsense, not to police hardware.
pub const MAX_PLAUSIBLE_HZ: u64 = 100_000_000_000;

/// Is `hz` a rate a real machine could plausibly have? See [`MIN_PLAUSIBLE_HZ`] and
/// [`MAX_PLAUSIBLE_HZ`] for where the bounds come from and why they are so wide.
///
/// One definition rather than three, because a band copied into each architecture's timer is a band
/// that drifts: the kernel's aarch64, riscv64 and `x86_64` boot paths all call this, at the point
/// each first learns its number.
///
/// # Examples
///
/// ```
/// use counter_frequency_protocol::is_plausible;
///
/// assert!(is_plausible(4_000_000)); // radon's `time` CSR
/// assert!(is_plausible(62_500_000)); // QEMU's aarch64 virtual counter under TCG
/// assert!(!is_plausible(0)); // firmware never wrote it
/// assert!(!is_plausible(1)); // firmware wrote nonsense, and `> 0` would have believed it
/// ```
pub fn is_plausible(hz: u64) -> bool {
    (MIN_PLAUSIBLE_HZ..=MAX_PLAUSIBLE_HZ).contains(&hz)
}

const OFF_MAGIC: usize = 0;
const OFF_HZ: usize = OFF_MAGIC + 8;

/// The whole page's size in bytes: the magic, then one little-endian `u64`. Far under one frame
/// (4096 bytes), which is the unit this is mapped as.
pub const PAGE_BYTES: usize = OFF_HZ + 8;

/// **The fixed virtual address `kernel::user::load` maps this page at**, in every `x86_64`
/// process. Both sides of the page (the kernel's writer, `user_mode_runtime::cntfrq`'s reader) agree on
/// this number through this crate rather than through two copies of a magic constant (CLAUDE.md
/// rule 7).
///
/// **Deliberately far above every other low-half address this tree hands out**, rather than
/// beside the boot tour's own small demo addresses (`kernel::user::X86_DEMO_CODE_VA` = `0x40_0000`,
/// `USER_STACK_VA` = `0x50_0000`). Every program's ELF loads at `0x40_0000` (`crates/user_mode_runtime/link.ld`) and
/// individual test fixtures map their own extra windows in the low few megabytes above it (a
/// first attempt at `0x60_0000` collided with `fixtures/src/window.rs`'s own `CTL_VA`, which a
/// full-suite run under `script/test --arch x86_64` caught as `AlreadyMapped`). Because this page
/// is mapped **unconditionally into every `x86_64` process** rather than opted into by one
/// program's own wiring, it cannot share that convention's address space at all: any low-MiB
/// address might be exactly what some future program's own segments or some future fixture's own
/// window wants. `0x0000_7000_0000_0000` sits at seven-eighths of the low half's own ceiling
/// (`x86_64`'s `SPLIT_SHIFT` is 47, so the low half is every address under `0x0000_8000_0000_0000`;
/// see `crates/paging/src/x86_64.rs`), leaving roughly 26 TiB of headroom below the boundary and
/// none of this tree's other conventions anywhere near it.
pub const PAGE_VA: u64 = 0x0000_7000_0000_0000;

/// Build a timebase page's bytes for `hz`, the calibrated (or `CPUID`-reported) TSC rate in
/// hertz. The kernel writes the result into a fresh frame before mapping it read-only into any
/// process; nothing else ever constructs one of these.
pub fn build_page(hz: u64) -> [u8; PAGE_BYTES] {
    let mut page = [0u8; PAGE_BYTES];
    page[OFF_MAGIC..OFF_MAGIC + 8].copy_from_slice(&MAGIC);
    page[OFF_HZ..OFF_HZ + 8].copy_from_slice(&hz.to_le_bytes());
    page
}

/// **The timebase page**, as seen through one process's read-only mapping of it at [`PAGE_VA`].
#[derive(Debug, Clone, Copy)]
pub struct TimebasePage {
    base: *const u8,
}

// SAFETY: the page has one writer (the kernel, before the page is ever mapped into a second
// address space) and every reader after that sees only immutable bytes, so there is no mutable
// aliasing across the `Send`/`Sync` boundary to protect against. Same reasoning as
// `environment_protocol::ConfigPage`.
unsafe impl Send for TimebasePage {}
// SAFETY: as `Send` above.
unsafe impl Sync for TimebasePage {}

impl TimebasePage {
    /// Name the timebase page mapped at `va`.
    ///
    /// # Safety
    ///
    /// `va` must be a mapped, byte-aligned buffer of at least [`PAGE_BYTES`] bytes (either a page
    /// [`build_page`] wrote, or a zeroed frame, which reads as "unknown"), and it must stay
    /// mapped and unwritten by this process for as long as this value is used. `kernel::user::load`
    /// maps exactly such a page, read-only, at [`PAGE_VA`], into every `x86_64` process; see the
    /// crate's own `BUGS` section for the one spawn path that does not.
    pub const unsafe fn new(va: u64) -> Self {
        TimebasePage {
            base: va as *const u8,
        }
    }

    /// # Safety
    ///
    /// Upheld by [`new`](Self::new)'s contract: `base` names at least `PAGE_BYTES` mapped,
    /// stable bytes.
    fn bytes(&self) -> &[u8] {
        // SAFETY: `new`'s contract.
        unsafe { core::slice::from_raw_parts(self.base, PAGE_BYTES) }
    }

    /// The calibrated TSC rate in hertz, or `None` if this page is unrecognized (a zeroed frame,
    /// the state before calibration has run, or a page belonging to something else). Never a
    /// fabricated default: see the crate's own docs for why the zeroed case is representable
    /// rather than papered over.
    pub fn hz(&self) -> Option<u64> {
        let bytes = self.bytes();
        if bytes[OFF_MAGIC..OFF_MAGIC + 8] != MAGIC {
            return None;
        }
        let mut raw = [0u8; 8];
        raw.copy_from_slice(&bytes[OFF_HZ..OFF_HZ + 8]);
        Some(u64::from_le_bytes(raw))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The round trip: build a page for a real-looking rate, read it back through a raw pointer
    /// the way a mapped frame would be named.
    #[test]
    fn a_built_page_round_trips_its_rate() {
        let bytes = build_page(2_400_000_000);
        // SAFETY: `bytes` is a live, aligned buffer of exactly PAGE_BYTES for this call's scope.
        let page = unsafe { TimebasePage::new(bytes.as_ptr() as u64) };
        assert_eq!(page.hz(), Some(2_400_000_000));
    }

    /// A zeroed frame (nothing has ever written a page into it, or calibration has not run) reads
    /// as "unknown", not as a fabricated 0 Hz or a stale value.
    #[test]
    fn a_zeroed_frame_reads_as_unknown() {
        let zeroed = [0u8; PAGE_BYTES];
        // SAFETY: as above.
        let page = unsafe { TimebasePage::new(zeroed.as_ptr() as u64) };
        assert_eq!(page.hz(), None);
    }

    /// An unrecognized magic refuses the rate even when the bytes after it look plausible: the
    /// magic check runs first, the same discipline `environment_protocol`'s equivalent test pins.
    #[test]
    fn an_unrecognized_magic_reads_as_unknown() {
        let mut bytes = build_page(1_000_000_000);
        bytes[OFF_MAGIC] ^= 0xff;
        // SAFETY: as above.
        let page = unsafe { TimebasePage::new(bytes.as_ptr() as u64) };
        assert_eq!(page.hz(), None);
    }

    /// The band accepts every rate this tree has actually met and refuses the two shapes the old
    /// `> 0` assertions let through: a register firmware never wrote, and a register firmware wrote
    /// nonsense into. The real rates are pinned by value so that a later reader narrowing the band
    /// for tidiness fails here rather than on somebody's board.
    #[test]
    fn the_band_accepts_real_rates_and_refuses_nonsense() {
        for hz in [
            4_000_000,     // radon, the VisionFive 2's `time` CSR
            10_000_000,    // QEMU virt, riscv64
            62_500_000,    // QEMU virt, aarch64 under TCG
            24_000_000,    // a common ARM board crystal
            3_000_000_000, // a TSC
        ] {
            assert!(is_plausible(hz), "{hz} Hz is a rate a real machine has");
        }
        for hz in [0, 1, 1_000, u64::MAX] {
            assert!(!is_plausible(hz), "{hz} Hz is a measurement that failed");
        }
    }

    /// The layout constants do not overlap and the page fits in one frame, pinned so a mutant
    /// swapping an offset is caught here rather than by a torn read on real hardware. The same
    /// shape `environment_protocol::the_layout_offsets_do_not_overlap` and
    /// `clock_protocol`'s own layout test use.
    #[test]
    #[allow(clippy::assertions_on_constants)]
    fn the_layout_offsets_do_not_overlap() {
        assert_eq!(OFF_HZ, 8);
        assert_eq!(PAGE_BYTES, 16);
        assert!(PAGE_BYTES < 4096, "the page must fit in one frame");
    }
}
