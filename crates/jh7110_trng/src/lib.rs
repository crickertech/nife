#![cfg_attr(not(test), no_std)]
//! **The `StarFive` JH7110 TRNG, as pure logic** (milestone 159; roadmap
//! `design/roadmap/159-jh7110-trng-driver.md`, notes/entropy.md).
//!
//! The register layout, the DTB query that finds the device, and the decision of whether a
//! generation attempt succeeded, failed, or is still running, with nothing an actual driver
//! touches. The controller's register file is the future driver's (`kernel/src/user/`-side or
//! `user/src/`-side, not yet written; see the roadmap doc's "What was deliberately not built"),
//! the same split `pci` and `nvme` already use (DECISIONS.md rule 7): this crate is host-testable
//! and Kani-reachable precisely because it never dereferences a pointer.
//!
//! # This has not run against real silicon
//!
//! **Nothing in this crate has been verified against the JH7110's TRNG on real hardware.** Every
//! fact below is sourced from documentation, not measurement; the roadmap doc's gate is HARDWARE
//! for exactly this reason, and this file does not change that. What is proven is proven on the
//! host, against values this crate makes up or a hand-written device-tree fixture, never against a
//! live register.
//!
//! # Sources
//!
//! - **[binding]** Linux, `Documentation/devicetree/bindings/rng/starfive,jh7110-trng.yaml`,
//!   mainline as of 2026-08-24 (fetched via
//!   `raw.githubusercontent.com/torvalds/linux/master/...`). Gives the compatible string, the
//!   `reg` window, the two clock inputs (`hclk`, `ahb`), one reset line, one interrupt, and the
//!   worked example this crate's DTB fixture is modeled on:
//!   `rng@1600C000 { reg = <0x1600C000 0x4000>; interrupts = <30>; }`.
//! - **[driver]** Linux, `drivers/char/hw_random/jh7110-trng.c`, mainline as of 2026-08-24,
//!   fetched the same way. The register offsets and bit positions below are transcribed from it;
//!   see each constant's doc for what the driver does with it. This is "the strongest grounding
//!   available without hardware" the milestone's brief asked for: a real, shipped, working Linux
//!   driver's register sequence, not a guess from the block diagram.
//! - **[ds]** `StarFive`, *JH7110 Datasheet*, v1.67 (2025-02-14),
//!   `doc-en.rvspace.org/JH7110/PDF/JH7110_DS.pdf`, §2.8.2 "TRNG", fetched and extracted with
//!   `pdftotext -layout` on 2026-08-24 (the `RVspace` TLS certificate had expired by then; a mirror
//!   at `elecrow.com/download/product/DTN63002G/JH7110_Datasheet.pdf` served the same PDF). Quoted
//!   in full because it is short and it is the compliance claim (or its absence) this crate's
//!   health-test story rests on:
//!
//!   > The TRNG module of JH7110 provides the following features.
//!   > - Ring-oscillator based entropy source
//!   > - Support LFSR based digital post process
//!   > - Support self re-seeding
//!   > - 256-bit random number generation
//!
//!   That is the entire section. **No NIST SP 800-90B, FIPS 140, or AIS-31 claim appears anywhere
//!   in the datasheet the search above could reach.** See "Health testing" below.
//!
//! # The register file [driver]
//!
//! Eight registers wide, `CTRL` through `ISTAT`, then eight 32-bit output words. All offsets and
//! bit positions are [regs] and the `*_` constants below, transcribed from `jh7110-trng.c`'s own
//! `#define`s.
//!
//! # Health testing: what the hardware gives you cheaply, and what it does not give you at all
//!
//! The brief for this milestone asked to check whether the JH7110 documents a hardware health-test
//! status register before writing any software statistical tests, because a hardware bit that is
//! merely read is nearly free and a software NIST SP 800-90B suite (repetition-count,
//! adaptive-proportion) is real design and verification work this project should not build on
//! spec.
//!
//! The answer is a partial one. **[driver] documents exactly one hardware fault signal**:
//! `ISTAT.LFSR_LOCKUP` (bit 4), which the Linux driver's interrupt handler treats as "SEU
//! [single-event upset] occurred, reseeding required" and answers by re-issuing
//! `CTRL.EXEC_RANDRESEED`. That is cheap (a status bit, already read every poll) and this crate
//! surfaces it: [`interpret`] returns [`Outcome::Lockup`] rather than folding it into "not ready
//! yet", so a caller can count it, retry it, or refuse to serve bytes past it, the same shape
//! `entropy.rs`'s `NO_ENTROPY` already gives a dry virtio-rng device.
//!
//! **What the hardware does *not* give you** is any statistical claim about the bits it is not
//! actively refusing. `LFSR_LOCKUP` catches a specific failure mode (the digital post-processing
//! stage getting stuck), not a degraded-but-still-running ring oscillator, and neither [ds] nor
//! [driver] document a built-in SP 800-90B-class test. Whether *this project* needs one before
//! trusting these bytes for anything security-shaped is a real design question the datasheet does
//! not resolve, and it is exactly the "open-ended judgment about cryptographic soundness" this
//! milestone's brief said to write up rather than guess at. **It is not decided here.** See the
//! roadmap doc and the final report for milestone 159's lane; the honest state of the question
//! belongs in `design/decisions/` as a PROPOSED entry, which is calef's call to write, not this
//! lane's (AGENTS.md: a developer never edits `design/`).
//!
//! # Examples
//!
//! ```
//! use jh7110_trng::{Outcome, interpret, ISTAT_RAND_RDY, ISTAT_LFSR_LOCKUP};
//!
//! // Not ready: neither bit set.
//! assert_eq!(interpret(0, [0; 8]), Outcome::NotReady);
//!
//! // Ready: the eight output words assemble little-endian, word 0 first.
//! let rand = [0x0403_0201, 0x0807_0605, 0, 0, 0, 0, 0, 0];
//! match interpret(ISTAT_RAND_RDY, rand) {
//!     Outcome::Ready(bytes) => assert_eq!(&bytes[..8], &[1, 2, 3, 4, 5, 6, 7, 8]),
//!     other => panic!("expected Ready, got {other:?}"),
//! }
//!
//! // A hardware fault beats a stale RAND_RDY from the same snapshot: the driver must not read a
//! // register file mid-reseed as if it were a fresh answer.
//! assert_eq!(
//!     interpret(ISTAT_RAND_RDY | ISTAT_LFSR_LOCKUP, [0; 8]),
//!     Outcome::Lockup
//! );
//! ```
//!
//! # What was deliberately not built
//!
//! No unsafe register access, no capability-holding driver program, and no wiring into
//! `entropy_service`. Rule 2's "gets what it needs passed in" is honored structurally: nothing
//! here knows a physical address at all, `discover` returns one instead of assuming it, and the
//! program that will eventually hold a `DeviceFrame` capability for that address is future work.
//! See the roadmap doc for exactly what is and is not ready for a customer to pick up.
//!
//! Name: unrecorded, provisional (introduced 2026-08-24 by milestone 159's lane). Chip-qualified
//! and unambiguous rather than generic (`trng` alone would be the "generic word that could name
//! almost anything" AGENTS.md warns off), the same reasoning `nvme` and `pci` already establish for
//! a spec-named device. calef has not ratified it; see `Cargo.toml`'s header for the same note.
//!
//! [binding]: https://github.com/torvalds/linux/blob/master/Documentation/devicetree/bindings/rng/starfive%2Cjh7110-trng.yaml
//! [driver]: https://github.com/torvalds/linux/blob/master/drivers/char/hw_random/jh7110-trng.c
//! [ds]: https://doc-en.rvspace.org/JH7110/PDF/JH7110_DS.pdf

/// Register byte offsets from the device's base, transcribed from `jh7110-trng.c`'s `#define`s
/// (\[driver\]). `RAND0..RAND7` are the eight 32-bit words a completed generation leaves behind;
/// [`assemble`] is the order they combine in.
pub mod regs {
    /// Command register: write [`super::CTRL_GENE_RANDNUM`] or [`super::CTRL_EXEC_RANDRESEED`].
    pub const CTRL: u64 = 0x00;
    /// Status register: read-only, [`super::STAT_SEEDED`] / `RAND_GENERATING` / `RAND_SEEDING`.
    pub const STAT: u64 = 0x04;
    /// Output width mode (128-bit or 256-bit PRNG output in the vendor driver; this crate only
    /// ever reads all eight `RAND` words regardless of what MODE claims, so it never needs to
    /// write this register to get a full-width answer).
    pub const MODE: u64 = 0x08;
    /// A second mode register the driver defines but whose exact bit meaning was not part of the
    /// summarized source this crate was built from. Offset only; not otherwise used here.
    pub const SMODE: u64 = 0x0C;
    /// Interrupt enable: [`super::IE_GLBL_EN`] is the one bit this crate names.
    pub const IE: u64 = 0x10;
    /// Interrupt status: [`super::ISTAT_RAND_RDY`], `SEED_DONE`, `LFSR_LOCKUP`. [`super::interpret`]
    /// reads this; a real driver must also clear it, and how (write-1-to-clear vs. write-back)
    /// was not confirmed from the summarized driver source.
    pub const ISTAT: u64 = 0x14;
    /// The eight output words. Populated once `ISTAT.RAND_RDY` is set. See [`RAND1`]..[`RAND7`].
    pub const RAND0: u64 = 0x20;
    /// See [`RAND0`].
    pub const RAND1: u64 = 0x24;
    /// See [`RAND0`].
    pub const RAND2: u64 = 0x28;
    /// See [`RAND0`].
    pub const RAND3: u64 = 0x2C;
    /// See [`RAND0`].
    pub const RAND4: u64 = 0x30;
    /// See [`RAND0`].
    pub const RAND5: u64 = 0x34;
    /// See [`RAND0`].
    pub const RAND6: u64 = 0x38;
    /// See [`RAND0`].
    pub const RAND7: u64 = 0x3C;
    /// Auto-reseed request-count threshold. The vendor driver programs this during init; the
    /// value it uses was not part of the summarized source, so this crate does not propose one.
    pub const AUTO_RQSTS: u64 = 0x60;
    /// Auto-reseed age (time) threshold. Same caveat as [`AUTO_RQSTS`].
    pub const AUTO_AGE: u64 = 0x64;
}

/// `CTRL`: ask the device for a fresh 256-bit random number.
pub const CTRL_GENE_RANDNUM: u32 = 0x1;
/// `CTRL`: force a reseed. The driver issues this once at init and again on every
/// [`Outcome::Lockup`].
pub const CTRL_EXEC_RANDRESEED: u32 = 0x2;

/// `STAT` bit 9: the device has been seeded at least once and may be asked to generate.
pub const STAT_SEEDED: u32 = 1 << 9;
/// `STAT` bit 30: a generation is in flight. The driver's `wait_idle` polls this (and
/// `RAND_SEEDING`) clear before issuing a new command, so two commands are never in flight at
/// once.
pub const STAT_RAND_GENERATING: u32 = 1 << 30;
/// `STAT` bit 31: a reseed is in flight.
pub const STAT_RAND_SEEDING: u32 = 1 << 31;

/// `IE` bit 31: the global interrupt enable. Named for completeness; this crate's own [`interpret`]
/// is poll-shaped (it takes an `ISTAT` snapshot, not an interrupt), so nothing here requires IE to
/// be set. A future driver that wants the completion interrupt rather than polling still needs
/// this bit, at PLIC line 30 per \[binding\]'s `interrupts = <30>`.
pub const IE_GLBL_EN: u32 = 1 << 31;

/// `ISTAT` bit 0: a generation completed and `RAND0..RAND7` hold a fresh answer.
pub const ISTAT_RAND_RDY: u32 = 1 << 0;
/// `ISTAT` bit 1: a reseed completed.
pub const ISTAT_SEED_DONE: u32 = 1 << 1;
/// `ISTAT` bit 4: the hardware's own fault signal, an SEU (single-event upset) in the LFSR-based
/// post-processing stage. The Linux driver treats this as "reseed and try again", never as "trust
/// the RAND registers anyway"; [`interpret`] gives it priority over `RAND_RDY` in the same
/// snapshot for the same reason (a register file that says both "ready" and "faulted" at once
/// must not be read as an answer). See the module doc's "Health testing" section: this is the one
/// hardware self-check the datasheet documents, and it is not a substitute for a statistical
/// health test over the bitstream, which the datasheet does not claim either.
pub const ISTAT_LFSR_LOCKUP: u32 = 1 << 4;

/// The device tree `compatible` string \[binding\] defines. `starfive,jh8100-trng` also lists this
/// as a fallback compatible (the JH8100 reuses the same TRNG IP), which is out of scope: this
/// crate matches the JH7110's own string only, never the fallback, so it says nothing about a `SoC`
/// this project has no board for.
pub const COMPATIBLE: &[u8] = b"starfive,jh7110-trng";

/// **The spelling radon's own firmware uses for the same device**, and the reason milestone 239
/// (radon's device tree does not describe the TRNG, so a working driver never runs) exists.
///
/// The tree nife is handed on the VisionFive 2 is the vendor U-Boot's control DTB (U-Boot 2021.10,
/// `Build: jenkins-VF2_515_Branch_SDK_Release-24`, Feb 12 2023, per the board's own banner in
/// `crates/board_console/tests/fixtures/captured/vf2-2026-09-01-manual-boot.log`). Its source is
/// StarFive's fork, not Linux's, and it spells the node \[uboot-dtsi\]:
///
/// ```text
/// trng: trng@1600C000 {
///     compatible = "starfive,trng";
///     reg = <0x0 0x1600C000 0x0 0x4000>;
///     clocks = <&clkgen JH7110_SEC_HCLK>,
///          <&clkgen JH7110_SEC_MISCAHB_CLK>;
///     clock-names = "hclk", "miscahb_clk";
///     resets = <&rstgen RSTN_U0_SEC_TOP_HRESETN>;
///     interrupts = <30>;
///     status = "disabled";
/// };
/// ```
///
/// **Same device, same window, same interrupt, different string**, and the difference is a stale
/// fork rather than different silicon: StarFive's own kernel driver for this block already matched
/// `starfive,jh7110-trng` in December 2022 \[vendor-driver\], two months before that firmware was
/// built, and its register `#define`s are byte-for-byte the offsets in [`regs`]. Nothing on
/// StarFive's side ever noticed, because Linux on this board is handed the kernel package's own
/// DTB and never sees U-Boot's.
///
/// **Accepting it is a claim about the register layout, so here is the evidence for that claim.**
/// The vendor Linux driver at the firmware's own vintage \[vendor-driver\] defines `STARFIVE_CTRL`
/// 0x00, `STAT` 0x04, `MODE` 0x08, `SMODE` 0x0C, `IE` 0x10, `ISTAT` 0x14, `RAND0`..`RAND7`
/// 0x20..0x3C, `AUTO_RQSTS` 0x60, `AUTO_AGE` 0x64, and the same `CTRL`/`STAT`/`ISTAT` bit
/// positions mainline's `jh7110-trng.c` does. Two drivers written against one IP block agree
/// completely, so a node claiming `starfive,trng` at `0x1600_C000` is claiming [`regs`].
///
/// \[uboot-dtsi\]: `arch/riscv/dts/jh7110.dtsi` in `starfive-tech/u-boot`, branch
/// `JH7110_VisionFive2_devel`, read 2026-09-03 at commit `bfbdce9b86a2` (2023-01-06, the last
/// change to that file before the flashed firmware's Feb 12 2023 build date) and unchanged at that
/// branch's head:
/// <https://github.com/starfive-tech/u-boot/blob/bfbdce9b86a2/arch/riscv/dts/jh7110.dtsi>
///
/// \[vendor-driver\]: `drivers/char/hw_random/starfive-trng.c` in `starfive-tech/linux`, commit
/// `202b558ae34c` (2022-12-14), read 2026-09-03:
/// <https://github.com/starfive-tech/linux/blob/202b558ae34c/drivers/char/hw_random/starfive-trng.c>
pub const COMPATIBLE_VENDOR: &[u8] = b"starfive,trng";

/// What [`discover`] found: the register window's physical base and size (from `reg`), and the
/// PLIC interrupt number if the tree gives one (`interrupts`, \[binding\]'s single cell).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Discovered {
    /// Physical base address of the register window (`reg`'s address cell).
    pub reg_base: u64,
    /// Size of the register window in bytes (`reg`'s size cell). \[binding\]'s example claims
    /// `0x4000`; this crate's [`regs`] only uses the low `0x68` of it.
    pub reg_size: u64,
    /// The PLIC interrupt line, if the tree names one. `None` is not an error: a tree with `reg`
    /// but no `interrupts` still names a real device, one a poll-only driver could still use.
    pub interrupt: Option<u32>,
    /// Which `compatible` string matched: [`COMPATIBLE`] or [`COMPATIBLE_VENDOR`]. Carried rather
    /// than discarded because the two spellings mean different things about the tree a bench
    /// session is looking at, and a boot tour that says which one it found tells the next reader
    /// whether they are on the vendor U-Boot's control DTB or on something fuller.
    pub compatible: &'static [u8],
    /// What the node's `status` property says, decoded to the one question a driver has: may this
    /// device be used? `true` when the property is absent (the device-tree specification's default
    /// is `"okay"`) or spells `"okay"`/`"ok"`; `false` for `"disabled"` and every other value.
    ///
    /// **[`discover`] reports this and does not act on it**, deliberately. The vendor control tree
    /// marks the TRNG `status = "disabled"` because *U-Boot* has no driver for it, not because the
    /// silicon is absent: StarFive's own Linux enables the same node from
    /// `jh7110-common.dtsi` (`&trng { status = "okay"; };`). This tree also has a recorded reason
    /// not to take that firmware's `status` at face value, in the other direction: the same DTB
    /// marks the S7 monitor core `status = "okay"` and claims it has an Sv39 MMU, and both are
    /// false (notes/visionfive2.md, "Second bench stop"). So the honest thing is to say what the
    /// tree says and let the caller decide, rather than to refuse a device that is there or trust
    /// one that is not.
    ///
    /// It is also the first thing to read when the register window comes back all zeros: a node
    /// the firmware calls disabled is a node whose clocks the firmware had no reason to ungate.
    pub status_okay: bool,
}

/// Find the JH7110 TRNG in `tree`, if it is there at all.
///
/// **`Ok(None)` is not a parse failure, it is the honest answer on every machine this repository
/// currently boots under CI**: QEMU's riscv64 `virt` board has no TRNG node under either spelling,
/// so this function must return `None` against it, and the fixture test below pins exactly that
/// against the same `.dtb` `crates/dtb`'s own tests already boot-verify against. Discovery is the
/// one piece of "does this device exist at all" this crate can prove without silicon: it is a pure
/// query over bytes, and a device tree dumped from the real board (once someone captures one) is a
/// drop-in fixture for the same test, not a new code path.
///
/// **Two spellings are tried, mainline's first** ([`COMPATIBLE`], then [`COMPATIBLE_VENDOR`]), and
/// the order is the whole of the policy: a tree that carries both is describing itself in the
/// language the binding standardised, and that is the one to believe. Trying the vendor string at
/// all is milestone 239's finding; [`COMPATIBLE_VENDOR`] carries the evidence that it names the
/// same register block.
pub fn discover(tree: &dtb::Dtb<'_>) -> Result<Option<Discovered>, dtb::Error> {
    for compatible in [COMPATIBLE, COMPATIBLE_VENDOR] {
        if let Some(found) = discover_as(tree, compatible)? {
            return Ok(Some(found));
        }
    }
    Ok(None)
}

/// One spelling's worth of [`discover`]. Split out so the two attempts cannot drift apart: every
/// property is read against the *same* string that matched the node, so a tree carrying an
/// unrelated node under the other spelling can never contribute a `reg` or an `interrupts` to this
/// answer.
fn discover_as(
    tree: &dtb::Dtb<'_>,
    compatible: &'static [u8],
) -> Result<Option<Discovered>, dtb::Error> {
    let mut regions = [dtb::Region { start: 0, size: 0 }; 1];
    let n = tree.node_reg_compatible(compatible, &mut regions)?;
    if n == 0 {
        return Ok(None);
    }
    let interrupt = tree
        .node_prop_compatible(compatible, b"interrupts")?
        .and_then(|bytes| bytes.get(0..4))
        .and_then(|b| b.try_into().ok())
        .map(u32::from_be_bytes);
    let status_okay = tree
        .node_prop_compatible(compatible, b"status")?
        .map_or(true, status_says_okay);
    Ok(Some(Discovered {
        reg_base: regions[0].start,
        reg_size: regions[0].size,
        interrupt,
        compatible,
        status_okay,
    }))
}

/// Decode a `status` property value. The device-tree specification defines `"okay"` as the only
/// value meaning "operational", with `"disabled"`, `"fail"` and `"fail-sss"` all meaning it is
/// not; `"ok"` is a widespread legacy spelling of `"okay"` that trees in the wild still carry, so
/// it is accepted here rather than read as a fourth kind of "no".
///
/// The value is a NUL-terminated string in the tree, so both the terminated and the unterminated
/// forms are matched here rather than trimming: a four-way `matches!` says exactly which byte
/// strings count, where a trim would have to decide what an empty or multi-string value means and
/// would answer for cases no tree produces.
fn status_says_okay(value: &[u8]) -> bool {
    matches!(value, b"okay\0" | b"ok\0" | b"okay" | b"ok")
}

/// The result of looking at one `ISTAT` snapshot (and the `RAND` words, if it looked ready).
///
/// Deliberately not `Result`: neither variant is this crate's idea of an error. `NotReady` is the
/// expected answer on every poll but the last; `Lockup` is a real, documented hardware condition a
/// caller is expected to handle (retry after reseed), not a bug.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// A generation completed cleanly. The 32 bytes are `RAND0..RAND7`, little-endian, word 0
    /// first: see [`assemble`].
    Ready([u8; 32]),
    /// `ISTAT.LFSR_LOCKUP` was set: an SEU, per \[driver\]'s own comment. The caller should issue
    /// `CTRL_EXEC_RANDRESEED` and retry, bounded, the same "ask again, bounded, then tell the
    /// truth" shape `entropy.rs`'s `REFILL_TRIES` already uses for a dry virtio-rng device.
    Lockup,
    /// Neither bit was set: still generating, or not yet asked. Not a failure on its own; a caller
    /// polls again, bounded by its own timeout (this crate does not impose one, because how long
    /// "too long" is is a fact about the real device's timing this crate cannot know without it).
    NotReady,
}

/// Decide what one `ISTAT` snapshot means, given the `RAND` words alongside it.
///
/// `rand` is read every call regardless of `istat`, which costs nothing (it is already in hand,
/// the same eight loads a real driver would do) and keeps this function a pure decode with no
/// hidden state: the same `(istat, rand)` pair always decides the same [`Outcome`].
///
/// **Lockup takes priority over ready**, even though the two bits live in unrelated positions and
/// nothing in \[driver\] says they cannot both be set in one snapshot: a register file the hardware
/// is actively re-seeding must not be read as if it had just answered cleanly, so a caller cannot
/// be handed 32 bytes from underneath a fault.
pub fn interpret(istat: u32, rand: [u32; 8]) -> Outcome {
    if istat & ISTAT_LFSR_LOCKUP != 0 {
        Outcome::Lockup
    } else if istat & ISTAT_RAND_RDY != 0 {
        Outcome::Ready(assemble(rand))
    } else {
        Outcome::NotReady
    }
}

/// Lay the eight `RAND` words out as 32 bytes, little-endian, word 0 first: `RAND0`'s low byte is
/// byte 0. There is no cryptographic transform here for `entropy.rs`'s own reason (DECISIONS
/// §44): with no one-way function in this tree, any reshuffling would change the bytes without
/// adding unpredictability an attacker could not undo. This is data movement, not conditioning.
pub fn assemble(rand: [u32; 8]) -> [u8; 32] {
    let mut out = [0u8; 32];
    for (i, word) in rand.iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&word.to_le_bytes());
    }
    out
}

/// How many bytes fit in the one word [`Pool::take`] answers with. The same 8 as
/// `entropy_proto::MAX_BYTES`, stated here rather than depended on: this crate is the device's
/// logic and knows nothing about the wire format, and the two agreeing is a fact the driver
/// program (which depends on both) is where a reader can check.
pub const WORD_BYTES: u64 = 8;

/// **The 32 bytes in hand, and how many are still ours to give** (milestone 159), lifted out of
/// `user/src/jh7110_trng.rs` so it can be tested somewhere a register does not have to exist.
///
/// This is the one piece of the driver that can serve a byte twice, hand back a byte it already
/// zeroed, or lose the seam between two generations, and none of that is visible in the register
/// decode above. It has no idea where its bytes come from: [`take`](Pool::take) is handed a
/// generator, which in the program is a poll of a real device and in the tests below is a counter.
///
/// **Not a pool in the "reservoir" sense.** `cursor` only ever moves forward, so no byte is served
/// twice, and each byte is zeroed as it leaves: a byte a client now holds is not also still
/// sitting in a buffer a long-lived process keeps for the rest of the boot. The same shape
/// `user/src/entropy.rs`'s virtio-rng `Pool` has, at a quarter the size, because there is no
/// device round trip here to amortize over.
///
/// # Examples
///
/// ```
/// use jh7110_trng::Pool;
///
/// // A device that answers with 32 bytes of 0xAB, forever.
/// let mut generate = || Some([0xab; 32]);
/// let mut pool = Pool::new();
/// assert_eq!(pool.take(4, &mut generate), (4, 0xabab_abab));
///
/// // A device that never answers: the caller is told, rather than spun on.
/// let mut dry = || None;
/// let mut pool = Pool::new();
/// assert_eq!(pool.take(4, &mut dry), (0, 0));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pool {
    buf: [u8; 32],
    cursor: usize,
    filled: usize,
}

impl Default for Pool {
    fn default() -> Self {
        Self::new()
    }
}

impl Pool {
    /// An empty pool: the first [`take`](Pool::take) generates.
    #[must_use]
    pub const fn new() -> Self {
        Pool {
            buf: [0; 32],
            cursor: 0,
            filled: 0,
        }
    }

    /// How many bytes are left before the next generation. Test and diagnostic support; the
    /// program reports it once, in its readiness message.
    #[must_use]
    pub const fn remaining(&self) -> usize {
        self.filled - self.cursor
    }

    /// Fill from `generate`, discarding whatever was left. `false` when the generator would not
    /// answer, which the caller reports rather than retries forever.
    pub fn refill(&mut self, generate: &mut impl FnMut() -> Option<[u8; 32]>) -> bool {
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

    /// Take `n` bytes, as a little-endian word plus a count. Gathers across a refill boundary, so
    /// a request can straddle two generations without the client ever seeing the seam, and returns
    /// a **short count** rather than zeros when the device stops answering mid-gather.
    ///
    /// **`n` is clamped to [`WORD_BYTES`]**, because the answer is one 64-bit word and there is
    /// nowhere to put a ninth byte. `entropy_proto::want` already clamps to the same 8 before the
    /// driver ever calls this, so in the program the clamp is unreachable; it is here because a
    /// public function that shifts by `8 * n` must not be callable into an overflow, and the first
    /// host test written against this API found exactly that edge.
    pub fn take(&mut self, n: u64, generate: &mut impl FnMut() -> Option<[u8; 32]>) -> (u64, u64) {
        let n = n.min(WORD_BYTES);
        let mut word = 0u64;
        let mut got = 0u64;
        while got < n {
            if self.cursor == self.filled && !self.refill(generate) {
                break;
            }
            let run = (n - got).min((self.filled - self.cursor) as u64);
            for i in 0..run {
                let at = self.cursor + i as usize;
                word |= u64::from(self.buf[at]) << (8 * (got + i));
                self.buf[at] = 0;
            }
            self.cursor += run as usize;
            got += run;
        }
        (got, word)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn not_ready_when_neither_bit_is_set() {
        assert_eq!(interpret(0, [0; 8]), Outcome::NotReady);
        assert_eq!(
            interpret(ISTAT_SEED_DONE, [0xffff_ffff; 8]),
            Outcome::NotReady
        );
    }

    #[test]
    fn ready_assembles_little_endian_word_zero_first() {
        let rand = [
            0x0403_0201,
            0x0807_0605,
            0x0c0b_0a09,
            0x100f_0e0d,
            0x1413_1211,
            0x1817_1615,
            0x1c1b_1a19,
            0x201f_1e1d,
        ];
        match interpret(ISTAT_RAND_RDY, rand) {
            Outcome::Ready(bytes) => {
                let expected: [u8; 32] = core::array::from_fn(|i| (i + 1) as u8);
                assert_eq!(bytes, expected);
            }
            other => panic!("expected Ready, got {other:?}"),
        }
    }

    #[test]
    fn lockup_beats_ready_in_the_same_snapshot() {
        assert_eq!(
            interpret(ISTAT_RAND_RDY | ISTAT_LFSR_LOCKUP, [0xffff_ffff; 8]),
            Outcome::Lockup
        );
    }

    #[test]
    fn lockup_alone_is_reported_even_with_no_rand_rdy() {
        assert_eq!(interpret(ISTAT_LFSR_LOCKUP, [0; 8]), Outcome::Lockup);
    }

    #[test]
    fn seed_done_alone_is_not_confused_with_rand_rdy() {
        // SEED_DONE (bit 1) and RAND_RDY (bit 0) are adjacent; a shift-by-one bug here would pass
        // every other test in this file and fail silently on real hardware after a reseed.
        assert_eq!(interpret(ISTAT_SEED_DONE, [0; 8]), Outcome::NotReady);
    }

    const JH7110_TRNG_PRESENT: &[u8] = include_bytes!("../tests/fixtures/jh7110-trng-present.dtb");
    const QEMU_RISCV64_VIRT: &[u8] =
        include_bytes!("../../dtb/tests/fixtures/qemu-riscv64-virt.dtb");

    #[test]
    fn discover_finds_the_device_on_a_tree_shaped_like_the_real_board() {
        let tree = dtb::Dtb::from_bytes(JH7110_TRNG_PRESENT).expect("fixture parses");
        let found = discover(&tree)
            .expect("no parse error")
            .expect("the node is there");
        assert_eq!(found.reg_base, 0x1600_C000);
        assert_eq!(found.reg_size, 0x4000);
        assert_eq!(found.interrupt, Some(30));
        assert_eq!(found.compatible, COMPATIBLE);
        // Mainline's binding example states no `status`, and the device-tree specification's
        // default for an absent `status` is "okay".
        assert!(found.status_okay);
    }

    const JH7110_TRNG_VENDOR_UBOOT: &[u8] =
        include_bytes!("../tests/fixtures/jh7110-trng-vendor-uboot.dtb");

    /// **Milestone 239's whole finding, in one assertion.** radon is handed the vendor U-Boot's
    /// control DTB, and that tree does describe the TRNG: as `trng@1600C000`, `compatible =
    /// "starfive,trng"`, `status = "disabled"`. Before this milestone `discover` matched mainline's
    /// string only and answered `None`, so the boot tour printed "this machine's tree describes no
    /// starfive,jh7110-trng" against a tree that describes the device at the address the driver
    /// wanted.
    ///
    /// The fixture is transcribed from the firmware's own source, not dumped from the board (see
    /// its header), so what this test proves is that the decoder handles that shape. **Whether the
    /// running firmware's tree really carries the node is a bench fact and is still open**;
    /// `design/roadmap/239-radons-tree-describes-less-than-the-chip-has.md` carries the two
    /// commands that settle it.
    #[test]
    fn discover_finds_the_device_under_the_vendor_uboots_own_spelling() {
        let tree = dtb::Dtb::from_bytes(JH7110_TRNG_VENDOR_UBOOT).expect("fixture parses");
        let found = discover(&tree)
            .expect("no parse error")
            .expect("the vendor tree describes the device under its own compatible");
        assert_eq!(found.reg_base, 0x1600_C000);
        assert_eq!(found.reg_size, 0x4000);
        assert_eq!(found.interrupt, Some(30));
        assert_eq!(found.compatible, COMPATIBLE_VENDOR);
        // The one property a caller must not lose: the firmware calls its own TRNG disabled.
        assert!(!found.status_okay);
    }

    /// `status` decoding, at the four values a tree can carry and one it cannot. This is a
    /// three-line function guarding a fact a bench session will read off the boot tour, and the
    /// failure it would cause is the expensive kind: a `status` misread as okay turns "the
    /// firmware never ungated this device's clocks" into an unexplained window of zeros.
    #[test]
    fn status_is_decoded_the_way_the_specification_defines_it() {
        assert!(status_says_okay(b"okay\0"));
        assert!(status_says_okay(b"ok\0"));
        assert!(!status_says_okay(b"disabled\0"));
        assert!(!status_says_okay(b"fail\0"));
        assert!(!status_says_okay(b""));
    }

    /// **The negative case this milestone can actually prove without hardware.** QEMU's riscv64
    /// `virt` machine (the board every kernel test in this repository boots against) has no
    /// `starfive,jh7110-trng` node, so `discover` must say so rather than finding a false match at
    /// some unrelated node's `reg`. This is `crates/dtb/tests/fixtures/qemu-riscv64-virt.dtb`
    /// itself, the same bytes `crates/dtb/tests/qemu_riscv64_virt.rs` already boot-verifies, so a
    /// green result here is a claim about the tree this project actually runs, not a claim about a
    /// tree nobody has looked at.
    /// **No byte is ever served twice**, across as many refills as it takes to drain several
    /// generations. The generator hands out a distinct byte per call, so a cursor that wrapped
    /// instead of refilling, or a refill that did not reset the cursor, shows up as a repeat here
    /// rather than as a security property nobody checked.
    #[test]
    fn no_byte_is_served_twice_across_refills() {
        let mut next = 1u8;
        let mut generate = || {
            let block = core::array::from_fn(|i| next.wrapping_add(i as u8));
            next = next.wrapping_add(32);
            Some(block)
        };
        let mut pool = Pool::new();
        let mut seen = [0u32; 256];
        // 40 draws of 8 bytes is 320 bytes, ten generations, so the seam is crossed nine times.
        for _ in 0..40 {
            let (got, word) = pool.take(8, &mut generate);
            assert_eq!(got, 8);
            for b in word.to_le_bytes() {
                seen[b as usize] += 1;
            }
        }
        // The generator's byte stream is 1, 2, 3, ... wrapping, so 320 bytes visits every value
        // once or twice and never three times; a repeat from the pool would push one to three.
        for (value, &count) in seen.iter().enumerate() {
            assert!(count <= 2, "byte {value} came back {count} times");
        }
    }

    /// **A request that straddles a generation gets all of its bytes**, in order, with the seam
    /// invisible: the last four of one block and the first four of the next.
    #[test]
    fn a_request_straddles_the_seam_without_losing_bytes() {
        let mut which = 0u8;
        let mut generate = || {
            which += 1;
            Some([which; 32])
        };
        let mut pool = Pool::new();
        // Five-byte draws: the seventh starts at byte 30 of a 32-byte block, so it takes two
        // bytes from block 1 and three from block 2.
        for _ in 0..6 {
            assert_eq!(pool.take(5, &mut generate).0, 5);
        }
        let (got, word) = pool.take(5, &mut generate);
        assert_eq!(got, 5);
        assert_eq!(word.to_le_bytes()[..5], [1, 1, 2, 2, 2]);
    }

    /// **A device that stops answering produces a short count, not zeros.** The distinction is the
    /// whole of `entropy_proto`'s honesty: a client that asked for eight bytes and got four must
    /// be told four, or it will treat four zeros as entropy.
    #[test]
    fn a_dry_device_shortens_the_count_rather_than_padding() {
        let mut answers = 1;
        let mut generate = || {
            if answers > 0 {
                answers -= 1;
                Some([0xff; 32])
            } else {
                None
            }
        };
        let mut pool = Pool::new();
        assert_eq!(pool.take(8, &mut generate).0, 8);
        // Drain the one block this generator will ever give: 32 bytes, so three more eights.
        for _ in 0..3 {
            assert_eq!(pool.take(8, &mut generate).0, 8);
        }
        assert_eq!(pool.take(8, &mut generate), (0, 0));
    }

    /// **A served byte is not still sitting in the buffer.** Reading the same region back after a
    /// take must not reproduce it; the pool zeroes behind its cursor.
    #[test]
    fn a_served_byte_is_zeroed_behind_the_cursor() {
        let mut generate = || Some([0xa5; 32]);
        let mut pool = Pool::new();
        assert_eq!(pool.take(8, &mut generate), (8, 0xa5a5_a5a5_a5a5_a5a5));
        assert_eq!(pool.buf[..8], [0u8; 8]);
        assert_eq!(pool.remaining(), 24);
    }

    /// **A caller asking for more than a word gets a word**, not a shift-left overflow. The wire
    /// format clamps first, so this is defence at the API rather than a live path.
    #[test]
    fn a_request_larger_than_the_word_is_clamped() {
        let mut generate = || Some([0x11; 32]);
        let mut pool = Pool::new();
        assert_eq!(pool.take(64, &mut generate), (8, 0x1111_1111_1111_1111));
    }

    #[test]
    fn discover_finds_nothing_on_qemus_virt_board() {
        let tree = dtb::Dtb::from_bytes(QEMU_RISCV64_VIRT).expect("fixture parses");
        assert_eq!(discover(&tree).expect("no parse error"), None);
    }
}

/// Machine-checked proofs (DECISIONS §14). The four `#[test]`s above sample `interpret`'s
/// classification at a handful of chosen bit patterns; `ISTAT` is a real register a fault could
/// set to anything, so what matters is the classification rule holding for **every** 32-bit value,
/// not the four this crate's author thought to write down. That is exactly the gap Kani closes
/// cheaply here: `interpret` is a few comparisons over two `u32`s (well, one `u32` and an unused
/// `[u32; 8]` for the `NotReady`/`Lockup` cases), so the whole input space is small enough to
/// exhaust rather than sample.
#[cfg(kani)]
mod verification {
    use super::*;

    /// **A hardware fault is never read as a clean answer.** For every `istat` with
    /// `LFSR_LOCKUP` set, `interpret` returns `Lockup`, regardless of what `RAND_RDY` or any other
    /// bit says and regardless of what the `RAND` words happen to hold. This is the property the
    /// module doc's "Health testing" section argues for in prose; here it holds for the full
    /// `2^32` space of `istat`, not the two combinations the unit tests above pick.
    /// Falsification: unfalsified
    #[kani::proof]
    fn a_lockup_bit_is_never_overridden() {
        let istat: u32 = kani::any();
        kani::assume(istat & ISTAT_LFSR_LOCKUP != 0);
        let rand: [u32; 8] = kani::any();
        assert_eq!(interpret(istat, rand), Outcome::Lockup);
    }

    /// **`Ready` is returned exactly when the register file says so, and carries exactly those
    /// bytes.** No `istat` value produces `Ready` unless `RAND_RDY` is set and `LFSR_LOCKUP` is
    /// clear, and when it is, the bytes are `rand`'s little-endian encoding with nothing dropped,
    /// substituted, or reordered.
    /// Falsification: replayable `crates/jh7110_trng/falsifications/verification.ready_requires_rand_rdy_and_carries_the_words_untouched.patch`
    #[kani::proof]
    fn ready_requires_rand_rdy_and_carries_the_words_untouched() {
        let istat: u32 = kani::any();
        kani::assume(istat & ISTAT_LFSR_LOCKUP == 0 && istat & ISTAT_RAND_RDY != 0);
        let rand: [u32; 8] = kani::any();
        let Outcome::Ready(bytes) = interpret(istat, rand) else {
            panic!("RAND_RDY set and LFSR_LOCKUP clear did not read as Ready");
        };
        // **The layout written out, not `assemble` compared with itself** (milestone 211).
        // `interpret` calls `assemble`, so `assert_eq!(interpret(..), Ready(assemble(rand)))`
        // is satisfied by any consistently wrong `assemble`: a byte-swap, a word reversal or a
        // silent truncation would leave both sides equal and this harness green, while the doc
        // above claims the bytes are little-endian with nothing reordered. Indexing the words
        // by hand states that claim in terms the implementation cannot pick.
        let mut i = 0;
        while i < 32 {
            assert_eq!(
                bytes[i],
                (rand[i / 4] >> (8 * (i % 4))) as u8,
                "a RAND word reached the caller reordered, truncated or substituted",
            );
            i += 1;
        }
    }

    /// **`NotReady` is the only answer when neither bit is set**, whatever the rest of `istat` or
    /// the stale `RAND` words say: a device that has not finished is not accidentally read as done
    /// or as faulted because some other bit happened to be set.
    /// Falsification: unfalsified
    #[kani::proof]
    fn neither_bit_set_is_always_not_ready() {
        let istat: u32 = kani::any();
        kani::assume(istat & ISTAT_LFSR_LOCKUP == 0 && istat & ISTAT_RAND_RDY == 0);
        let rand: [u32; 8] = kani::any();
        assert_eq!(interpret(istat, rand), Outcome::NotReady);
    }
}
