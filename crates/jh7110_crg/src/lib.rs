#![cfg_attr(not(test), no_std)]
//! **The `StarFive` JH7110's clock and reset generator, as pure logic** (milestone 220; roadmap
//! `design/roadmap/220-jh7110-clock-and-reset.md`).
//!
//! Register offsets, bit positions, the bring-up plan the TRNG needs, and the device-tree query
//! that finds the controller, with nothing an actual driver touches. The volatile shell is
//! `kernel/src/drivers/jh7110_crg.rs`; this crate never dereferences a pointer, which is what
//! makes it host-testable and Kani-reachable, the same rule-7 split `jh7110_trng` and `pci`
//! already use.
//!
//! # Why this exists at all, and it is a measurement rather than an inference
//!
//! Every device nife has driven came up already running: QEMU's virtio devices, the PL011 and
//! NS16550 consoles, the PLIC. Real `SoC` peripherals do not. On **2026-09-04** radon (the
//! `StarFive` VisionFive 2) booted milestone 159's confined userspace TRNG driver twice,
//! byte-identically, and the register window read as nothing:
//!
//! ```text
//! hw entropy  : FAILED: JH7110 TRNG at 0x1600c000 (tree says starfive,trng, status disabled):
//!               report 0x524e475550, bring-up diagnostic 0x0000000000000000,
//!               draws 0/0 bytes, first-all-zero true, draws-differ false
//! ```
//!
//! Transcript: `target/board/radon-2026-09-04-trng-bringup.log`. The all-zero diagnostic is the
//! raw `(STAT << 32) | ISTAT`, so the whole register file read back as zeros; the device tree
//! independently marks the node `status disabled`. Two signals, and they agree.
//!
//! # Sources, fetched rather than recalled
//!
//! Every number below appears in **two independent published trees** that describe the same
//! silicon, and they agree. That agreement is the reason this crate is willing to carry a
//! constant at all; see [`STG`]'s own note.
//!
//! - **[mainline-dts]** Linux, `arch/riscv/boot/dts/starfive/jh7110.dtsi`, mainline, fetched
//!   2026-09-04 via `raw.githubusercontent.com/torvalds/linux/master/...`:
//!
//!   ```text
//!   rng: rng@1600c000 {
//!       compatible = "starfive,jh7110-trng";
//!       reg = <0x0 0x1600C000 0x0 0x4000>;
//!       clocks = <&stgcrg JH7110_STGCLK_SEC_AHB>,
//!                <&stgcrg JH7110_STGCLK_SEC_MISC_AHB>;
//!       clock-names = "hclk", "ahb";
//!       resets = <&stgcrg JH7110_STGRST_SEC_AHB>;
//!       interrupts = <30>;
//!   };
//!   ```
//!
//!   with `stgcrg` being `compatible = "starfive,jh7110-stgcrg"` at
//!   `reg = <0x0 0x10230000 0x0 0x10000>`.
//! - **[mainline-trng]** Linux, `drivers/char/hw_random/jh7110-trng.c`, fetched 2026-09-04. The
//!   probe order this crate's [`TRNG_BRING_UP`] reproduces:
//!
//!   ```c
//!   trng->hclk = devm_clk_get(&pdev->dev, "hclk");
//!   trng->ahb  = devm_clk_get(&pdev->dev, "ahb");
//!   trng->rst  = devm_reset_control_get_shared(&pdev->dev, NULL);
//!   clk_prepare_enable(trng->hclk);
//!   clk_prepare_enable(trng->ahb);
//!   reset_control_deassert(trng->rst);
//!   ```
//!
//! - **[mainline-ids]** Linux, `include/dt-bindings/clock/starfive,jh7110-crg.h` and
//!   `include/dt-bindings/reset/starfive,jh7110-crg.h`, fetched 2026-09-04:
//!   `JH7110_STGCLK_SEC_AHB 15`, `JH7110_STGCLK_SEC_MISC_AHB 16`, `JH7110_STGRST_SEC_AHB 3`,
//!   `JH7110_STGRST_END 23`.
//! - **[mainline-clk]** Linux, `drivers/clk/starfive/clk-starfive-jh71x0.c` and its header,
//!   fetched 2026-09-04. One 32-bit register per clock, `void __iomem *reg = priv->base + 4 *
//!   clk->idx;`, and `#define JH71X0_CLK_ENABLE BIT(31)`.
//! - **[mainline-rst]** Linux, `drivers/reset/starfive/reset-starfive-jh7110.c` and
//!   `drivers/reset/starfive/reset-starfive-jh71x0.c`, fetched 2026-09-04:
//!
//!   ```c
//!   static const struct jh7110_reset_info jh7110_stg_info = {
//!       .nr_resets = JH7110_STGRST_END,
//!       .assert_offset = 0x74,
//!       .status_offset = 0x78,
//!   };
//!   ```
//!
//!   and the update rule, which is where [`deasserted`]'s inverted sense comes from:
//!
//!   ```c
//!   u32 done = data->asserted ? data->asserted[offset] & mask : 0;
//!   if (!assert)
//!           done ^= mask;
//!   ...
//!   ret = readl_poll_timeout_atomic(reg_status, value, (value & mask) == done, 0, 1000);
//!   ```
//!
//!   The JH7110 passes `asserted = NULL`, so `done` is `0` for an assert and `mask` for a
//!   deassert: **a set status bit means the line is out of reset.**
//! - **[vendor-dts]** `starfive-tech/u-boot`, branch `JH7110_VisionFive2_devel`,
//!   `arch/riscv/dts/jh7110.dtsi`, fetched 2026-09-04. **This is the firmware radon actually
//!   runs**, and it spells the same wiring differently:
//!
//!   ```text
//!   trng: trng@1600C000 {
//!       compatible = "starfive,trng";
//!       clocks = <&clkgen JH7110_SEC_HCLK>, <&clkgen JH7110_SEC_MISCAHB_CLK>;
//!       clock-names = "hclk", "miscahb_clk";
//!       resets = <&rstgen RSTN_U0_SEC_TOP_HRESETN>;
//!       status = "disabled";
//!   };
//!
//!   clkgen: clock-controller {
//!       compatible = "starfive,jh7110-clkgen";
//!       reg = <0x0 0x13020000 0x0 0x10000>,
//!             <0x0 0x10230000 0x0 0x10000>,
//!             <0x0 0x17000000 0x0 0x10000>;
//!       reg-names = "sys", "stg", "aon";
//!   };
//!
//!   rstgen: reset-controller {
//!       compatible = "starfive,jh7110-reset";
//!       reg-names = "syscrg", "stgcrg", "aoncrg", "ispcrg", "voutcrg";
//!   };
//!   ```
//!
//! - **[vendor-ids]** the same branch's `include/dt-bindings/clock/starfive-jh7110-clkgen.h` and
//!   `include/dt-bindings/reset/starfive-jh7110.h`, fetched 2026-09-04. The vendor numbers are
//!   **flat across all the domains**, so they must be rebased before they mean anything:
//!   `JH7110_SEC_HCLK 205` and `JH7110_SEC_MISCAHB_CLK 206` against a stg group starting at
//!   `JH7110_HIFI4_CLK_CORE 190`, giving **15** and **16**; `RSTN_U0_SEC_TOP_HRESETN 131`
//!   against a stg group starting at `RSTN_U0_STG_SYSCON_PRESETN 128`, giving **3**.
//!
//! **So the two trees converge**: `15`, `16`, `3`, in the STG domain at `0x1023_0000`. That is
//! worth stating plainly because the vendor spellings look nothing like mainline's, and a reader
//! who only checked one would reasonably fear the driver was written against the wrong chip.
//!
//! # This has not run against real silicon
//!
//! **Nothing here has been verified against a JH7110.** QEMU's riscv64 `virt` machine has no
//! clock or reset controller of any kind, so an emulator cannot validate the sequence end to end:
//! what CI exercises is the absence path and the arithmetic, never a device answering. The bench
//! procedure that would settle it, with a table mapping each observable outcome to what it means,
//! is `notes/jh7110-clock-and-reset.md`.
//!
//! Name: unrecorded, provisional (introduced 2026-09-04 by milestone 220's lane). `crg` is not an
//! abbreviation this project coined: both device trees that describe this chip spell the blocks
//! `syscrg`, `stgcrg` and `aoncrg` (mainline's `starfive,jh7110-stgcrg`, the vendor's
//! `reg-names = "syscrg", "stgcrg", ...`), which puts it in the protected class of names a reader
//! already knows from outside, beside `elf`, `pci` and `dtb`. Chip-qualified for `jh7110_trng`'s
//! reason: a bare `crg` would name almost anything, and this tree has one `SoC` today and intends
//! more. Refused `jh7110_clock` (this controller also owns resets, and a name that says only
//! clock would make the reset half read as a surprise) and `jh7110_clkgen` (the vendor's own label,
//! but it names one of the two published spellings and this crate reads both). calef has not
//! ratified it; see `Cargo.toml`'s header for the same note and `script/names`.
//!
//! # Examples
//!
//! ```
//! use jh7110_crg::{STG, Step, TRNG_BRING_UP, CLOCK_ENABLE, deasserted};
//!
//! // The TRNG's plan is two clocks then one reset, in that order, which is the order
//! // Linux's own probe takes: a reset deassert against a gated clock can hang forever.
//! assert_eq!(
//!     TRNG_BRING_UP,
//!     &[Step::EnableClock(15), Step::EnableClock(16), Step::DeassertReset(3)]
//! );
//!
//! // Clock 15 lives one word per clock from the domain's base.
//! assert_eq!(STG.clock_offset(15), Some(0x3c));
//! assert_eq!(CLOCK_ENABLE, 1 << 31);
//!
//! // Reset 3 is bit 3 of the word at 0x74, watched at 0x78. A SET status bit means
//! // "out of reset", which is the opposite of what the register name suggests.
//! let r = STG.reset_bit(3).unwrap();
//! assert_eq!((r.assert_offset, r.status_offset, r.mask), (0x74, 0x78, 1 << 3));
//! assert!(deasserted(0b1000, r.mask));
//! assert!(!deasserted(0b0111, r.mask));
//! ```
//!
//! [mainline-dts]: https://github.com/torvalds/linux/blob/master/arch/riscv/boot/dts/starfive/jh7110.dtsi
//! [mainline-trng]: https://github.com/torvalds/linux/blob/master/drivers/char/hw_random/jh7110-trng.c
//! [mainline-ids]: https://github.com/torvalds/linux/blob/master/include/dt-bindings/reset/starfive%2Cjh7110-crg.h
//! [mainline-clk]: https://github.com/torvalds/linux/blob/master/drivers/clk/starfive/clk-starfive-jh71x0.c
//! [mainline-rst]: https://github.com/torvalds/linux/blob/master/drivers/reset/starfive/reset-starfive-jh7110.c
//! [vendor-dts]: https://github.com/starfive-tech/u-boot/blob/JH7110_VisionFive2_devel/arch/riscv/dts/jh7110.dtsi
//! [vendor-ids]: https://github.com/starfive-tech/u-boot/blob/JH7110_VisionFive2_devel/include/dt-bindings/clock/starfive-jh7110-clkgen.h

/// The bit that turns a clock on, in every one of this controller's per-clock words
/// (\[mainline-clk\], `#define JH71X0_CLK_ENABLE BIT(31)`).
pub const CLOCK_ENABLE: u32 = 1 << 31;

/// A single step of a device's bring-up sequence, in the order it must be taken.
///
/// Deliberately a plan rather than a pair of methods on a driver: the order is the load-bearing
/// part (\[mainline-trng\] enables both clocks *before* deasserting, and \[mainline-rst\]'s own
/// comment says why: *"if the associated clock is gated, deasserting might otherwise hang
/// forever"*), and a plan is the only shape that lets a host test assert on the order without a
/// device. The kernel driver walks this slice; it does not know which device it is bringing up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// Set [`CLOCK_ENABLE`] in the domain's word for this clock index.
    EnableClock(u32),
    /// Clear this reset's bit in the domain's assert word, then wait for its status bit to read
    /// as [`deasserted`].
    DeassertReset(u32),
}

/// **What the JH7110's TRNG needs before its registers answer**, transcribed from
/// \[mainline-trng\]'s probe and cross-checked against \[vendor-dts\] (see the module header:
/// both trees name clocks 15 and 16 and reset 3 of the STG domain, in different spellings).
///
/// All three identifiers are STG-domain, so this slice is meaningless without [`STG`]; that
/// coupling is why there is no `Domain` field on `Step`. A second device in a second domain would
/// carry its own plan and its own domain beside it, and the day that happens is the day to decide
/// whether the pair wants a type.
pub const TRNG_BRING_UP: &[Step] = &[
    Step::EnableClock(STGCLK_SEC_AHB),
    Step::EnableClock(STGCLK_SEC_MISC_AHB),
    Step::DeassertReset(STGRST_SEC_AHB),
];

/// `JH7110_STGCLK_SEC_AHB` \[mainline-ids\]; the vendor tree's `JH7110_SEC_HCLK` (205) rebased on
/// its stg group's start (190). The TRNG's `hclk`.
pub const STGCLK_SEC_AHB: u32 = 15;

/// `JH7110_STGCLK_SEC_MISC_AHB` \[mainline-ids\]; the vendor tree's `JH7110_SEC_MISCAHB_CLK` (206)
/// rebased the same way. The TRNG's second clock, `ahb` to mainline and `miscahb_clk` to the
/// vendor, which is the same wire under two names.
pub const STGCLK_SEC_MISC_AHB: u32 = 16;

/// `JH7110_STGRST_SEC_AHB` \[mainline-ids\]; the vendor tree's `RSTN_U0_SEC_TOP_HRESETN` (131)
/// rebased on its stg group's start (128).
///
/// **It is a *shared* reset** (\[mainline-trng\] takes it with `devm_reset_control_get_shared`),
/// which is a fact about the silicon rather than about Linux's API: the same line resets the whole
/// security top block, the PL080 DMA at `0x1600_8000` included. Deasserting it is safe; asserting
/// it would reset a neighbour, which is why nothing in this crate offers an assert.
pub const STGRST_SEC_AHB: u32 = 3;

/// Where one reset lives: which word to write, which word to watch, and which bit in both.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResetBit {
    /// Byte offset from the domain's base of the word whose bit is written to assert or deassert.
    pub assert_offset: u64,
    /// Byte offset from the domain's base of the word whose bit reports what the hardware did.
    pub status_offset: u64,
    /// The bit, in both words.
    pub mask: u32,
}

/// One clock-and-reset domain of the JH7110: a register window, and where the resets sit in it.
///
/// Three exist ([vendor-dts]'s `reg-names = "syscrg", "stgcrg", "aoncrg", "ispcrg", "voutcrg"`
/// names five), and only [`STG`] is described here, deliberately. The milestone's own `BUGS`
/// section named unbounded scope as its main risk: "a clock and reset driver for the JH7110" could
/// mean the one clock the TRNG needs or the whole controller, and those differ by an order of
/// magnitude. This is the first, with the arithmetic general enough that the second is a table
/// rather than a rewrite.
///
/// [vendor-dts]: https://github.com/starfive-tech/u-boot/blob/JH7110_VisionFive2_devel/arch/riscv/dts/jh7110.dtsi
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Domain {
    /// Byte offset from the base of the first reset-assert word.
    pub reset_assert: u64,
    /// Byte offset from the base of the first reset-status word.
    pub reset_status: u64,
    /// How many resets this domain has. Bounds [`Domain::reset_bit`], so an out-of-range id
    /// cannot become a write to whatever word the arithmetic lands on.
    pub resets: u32,
    /// How many clocks this domain has, bounding [`Domain::clock_offset`] for the same reason.
    pub clocks: u32,
}

/// **The STG (system-transport-group) domain**, which is where the TRNG's clocks and reset live.
///
/// Offsets from \[mainline-rst\]'s `jh7110_stg_info`; the counts from \[mainline-ids\]'s
/// `JH7110_STGRST_END` (23) and `JH7110_STGCLK_END` (29).
pub const STG: Domain = Domain {
    reset_assert: 0x74,
    reset_status: 0x78,
    resets: 23,
    clocks: 29,
};

/// **The STG domain's register window, as both published trees give it.**
///
/// A constant address in a tree whose whole habit is to read addresses out of the device tree, so
/// it owes a reason. [`discover`] reads the tree first and falls back to this only when the tree
/// names no controller, and [`Found::from_tree`] says which happened, so nothing can quietly
/// mistake one for the other. It is here because the alternative is worse: radon's firmware tree
/// is already known to omit and misdescribe things (milestone 239; the same tree calls the S7 core
/// `okay` and gives it an MMU it does not have), and a bench session that comes back with "no
/// controller node, nothing attempted" has spent a trip to the machine and learned nothing. Two
/// independently published trees agree on this number, which is a stronger warrant than most
/// device-tree reads get.
pub const STG_BASE: u64 = 0x1023_0000;

/// The STG window's size, `0x10000` in both trees.
pub const STG_SIZE: u64 = 0x1_0000;

impl Domain {
    /// The byte offset of `index`'s clock word, or `None` if this domain has no such clock.
    ///
    /// One 32-bit word per clock, in index order, from \[mainline-clk\]: `priv->base + 4 *
    /// clk->idx`. The `Option` is the bound: a caller holding an identifier from the wrong domain
    /// gets nothing rather than a plausible-looking offset into somebody else's register.
    #[must_use]
    pub const fn clock_offset(&self, index: u32) -> Option<u64> {
        if index >= self.clocks {
            return None;
        }
        Some(4 * index as u64)
    }

    /// Where reset `id` is written and watched, or `None` if this domain has no such reset.
    ///
    /// 32 resets to a word, from \[mainline-rst\]'s `offset = id / 32; mask = BIT(id % 32)`. The
    /// STG domain has 23, so every one of them is in word zero; the arithmetic is written out
    /// anyway because the SYS domain has 126 and would otherwise be a second implementation of
    /// the same rule.
    #[must_use]
    pub const fn reset_bit(&self, id: u32) -> Option<ResetBit> {
        if id >= self.resets {
            return None;
        }
        let word = (id / 32) as u64 * 4;
        Some(ResetBit {
            assert_offset: self.reset_assert + word,
            status_offset: self.reset_status + word,
            mask: 1 << (id % 32),
        })
    }
}

/// **Is this reset released?** Given a status word and a reset's mask.
///
/// The sense is inverted from what the name `status` suggests and the inversion is not this
/// crate's invention. \[mainline-rst\]'s `jh71x0_reset_update` computes `done = 0` for an assert
/// and `done = mask` for a deassert (the JH7110 passes `asserted = NULL`), then polls until
/// `(value & mask) == done`. So a **set** bit is a line that is out of reset. Getting this
/// backwards would produce a driver that waits forever on a device that came up correctly, which
/// is the failure this function exists to have exactly one copy of.
#[must_use]
pub const fn deasserted(status_word: u32, mask: u32) -> bool {
    status_word & mask != 0
}

/// **Is this clock running?** Given the clock's own word.
#[must_use]
pub const fn clock_enabled(clock_word: u32) -> bool {
    clock_word & CLOCK_ENABLE != 0
}

/// How many `EnableClock` steps a report keeps words for. Two is what the TRNG needs; four is
/// slack for the next device, and a plan with more simply stops recording rather than growing the
/// report, which is what [`Report::truncated`] says out loud.
pub const MAX_RECORDED_CLOCKS: usize = 4;

/// What the hardware said, in enough detail that a bench transcript is diagnosable without a
/// second trip to the machine.
///
/// **The `before` words are the load-bearing ones.** If the clocks read back already enabled and
/// the reset already released, then this milestone's premise was wrong for radon and the TRNG's
/// all-zero register window on 2026-09-04 has some other cause. That is the outcome a bench
/// session most needs to be able to tell apart, and only a before-and-after can tell it.
#[derive(Clone, Copy, Debug, Default)]
pub struct Report {
    /// Each `EnableClock` step's word as it read *before* anything was written, in plan order.
    pub clock_before: [u32; MAX_RECORDED_CLOCKS],
    /// The same words read back *after* the enable bit was stored.
    pub clock_after: [u32; MAX_RECORDED_CLOCKS],
    /// How many entries of the two arrays above are meaningful.
    pub clocks: usize,
    /// True when the plan had more clock steps than [`MAX_RECORDED_CLOCKS`]. They were still
    /// performed; only the recording stopped.
    pub truncated: bool,
    /// The reset-assert word as it read *before* the plan's last `DeassertReset` step wrote it.
    /// Zero when the plan had none, which [`Report::had_reset`] distinguishes from a real zero.
    pub reset_assert_before: u32,
    /// The same word read back after the step's bit was cleared.
    pub reset_assert_after: u32,
    /// The status word as the poll last saw it. A set bit means out of reset; see [`deasserted`],
    /// whose doc records why that sense is the opposite of what the name suggests.
    pub reset_status_after: u32,
    /// True when the plan contained a `DeassertReset` at all, so a reader can tell "no reset in
    /// this plan" from "a reset whose registers all read zero".
    pub had_reset: bool,
    /// True when the status word said the line was out of reset before the poll gave up.
    pub released: bool,
    /// How many status reads the deassert took. The driver's own `POLL_LIMIT` here means it never
    /// came out.
    pub polls: u32,
    /// Steps whose identifier this domain rejected, which would mean the plan and the domain
    /// disagree: a programming error, not a hardware condition. Nonzero here invalidates the rest.
    pub rejected: usize,
}

impl Report {
    /// **Did every clock this plan named read its enable bit back?** A clock that does not is a
    /// window with nothing behind it, or a base address that is not this controller.
    #[must_use]
    pub fn clocks_running(&self) -> bool {
        self.clocks > 0
            && self.clock_after[..self.clocks]
                .iter()
                .all(|&w| clock_enabled(w))
    }

    /// **Was the device already up before this ran?** True when every recorded clock was enabled
    /// and, if the plan had one, the reset was already released. See the type's own doc for why
    /// this is the question a bench session asks first.
    #[must_use]
    pub fn was_already_up(&self) -> bool {
        let clocks = self.clocks > 0
            && self.clock_before[..self.clocks]
                .iter()
                .all(|&w| clock_enabled(w));
        clocks && (!self.had_reset || self.reset_assert_before & self.reset_mask() == 0)
    }

    /// The bit the reset step used, recoverable from the two assert words. Zero when there was no
    /// reset step or when the bit was already clear, which is why `was_already_up` reads the
    /// before-word rather than trusting this alone.
    const fn reset_mask(&self) -> u32 {
        self.reset_assert_before ^ self.reset_assert_after
    }
}

/// What [`discover`] concluded about where the STG domain's registers are.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Found {
    /// Physical base of the STG register window.
    pub base: u64,
    /// Its size in bytes.
    pub size: u64,
    /// **True when the device tree said so, false when this is [`STG_BASE`].** The one field a
    /// bench transcript must carry: a base that came from a constant is a base nobody on that
    /// machine confirmed, and a reader three months later cannot re-derive which it was.
    pub from_tree: bool,
    /// Which `compatible` string matched, or `None` when nothing did and the constant was used.
    pub compatible: Option<&'static [u8]>,
}

/// Mainline's dedicated STG clock-and-reset controller node \[mainline-dts\].
pub const COMPATIBLE_STGCRG: &[u8] = b"starfive,jh7110-stgcrg";

/// The vendor U-Boot's single clock controller covering sys, stg and aon \[vendor-dts\]. This is
/// the one radon's firmware actually serves, if it serves either.
pub const COMPATIBLE_VENDOR_CLKGEN: &[u8] = b"starfive,jh7110-clkgen";

/// The vendor U-Boot's separate reset controller, whose `reg` list covers the same windows
/// \[vendor-dts\]. Tried last: it names the same address as the clkgen node, so it is only
/// reached by a tree that carries one and not the other.
pub const COMPATIBLE_VENDOR_RSTGEN: &[u8] = b"starfive,jh7110-reset";

/// The `reg-names` entry naming the STG window, in each vendor node's own spelling
/// (`"stg"` in `clkgen`, `"stgcrg"` in `rstgen`; \[vendor-dts\]).
const VENDOR_CLKGEN_STG_NAME: &[u8] = b"stg";
const VENDOR_RSTGEN_STG_NAME: &[u8] = b"stgcrg";

/// **Find the STG clock and reset window in `tree`.**
///
/// Never returns `None` and never fails to produce an address: a tree that names no controller
/// gets [`STG_BASE`] with `from_tree: false`, for the reason [`STG_BASE`] records. What it can
/// return is an error, if the blob itself does not parse.
///
/// **Three spellings are tried, mainline's first**, the same order and the same reasoning
/// `jh7110_trng::discover` uses: a tree that carries the standardised binding is describing itself
/// in the language the binding standardised, and that is the one to believe. The two vendor nodes
/// carry several windows and are indexed by `reg-names` rather than by position, because a
/// position that happens to be right today is a fact nobody wrote down.
///
/// # Errors
///
/// Propagates [`dtb::Error`] if the blob is malformed.
pub fn discover(tree: &dtb::Dtb<'_>) -> Result<Found, dtb::Error> {
    for (compatible, name) in [
        (COMPATIBLE_STGCRG, None),
        (COMPATIBLE_VENDOR_CLKGEN, Some(VENDOR_CLKGEN_STG_NAME)),
        (COMPATIBLE_VENDOR_RSTGEN, Some(VENDOR_RSTGEN_STG_NAME)),
    ] {
        if let Some(found) = discover_as(tree, compatible, name)? {
            return Ok(found);
        }
    }
    Ok(Found {
        base: STG_BASE,
        size: STG_SIZE,
        from_tree: false,
        compatible: None,
    })
}

/// One spelling's worth of [`discover`]. `window` is the `reg-names` entry to select, or `None`
/// for a node whose single `reg` is the answer.
fn discover_as(
    tree: &dtb::Dtb<'_>,
    compatible: &'static [u8],
    window: Option<&[u8]>,
) -> Result<Option<Found>, dtb::Error> {
    // Five, because the vendor `rstgen` node names five windows and a short buffer would silently
    // truncate the list `reg-names` is indexing into.
    let mut regions = [dtb::Region { start: 0, size: 0 }; 5];
    let n = tree.node_reg_compatible(compatible, &mut regions)?;
    if n == 0 {
        return Ok(None);
    }
    let index = match window {
        None => 0,
        Some(want) => {
            let names = tree.node_prop_compatible(compatible, b"reg-names")?;
            // A multi-window node with no `reg-names` is not something this crate will guess at:
            // picking window 1 because the tree we read said so is exactly the "a fact nobody
            // wrote down" case, and the constant fallback is the honest answer instead.
            match names.and_then(|bytes| name_index(bytes, want)) {
                Some(i) if i < n => i,
                _ => return Ok(None),
            }
        }
    };
    Ok(Some(Found {
        base: regions[index].start,
        size: regions[index].size,
        from_tree: true,
        compatible: Some(compatible),
    }))
}

/// The position of `want` in a device-tree string list (NUL-separated, NUL-terminated).
///
/// Split rather than trimmed, and an empty trailing element is not counted: `"sys\0stg\0aon\0"`
/// has three entries, not four, and `stg` is index 1. Getting that wrong would shift every window
/// after the first.
fn name_index(list: &[u8], want: &[u8]) -> Option<usize> {
    list.split(|&b| b == 0)
        .filter(|s| !s.is_empty())
        .position(|s| s == want)
}

#[cfg(test)]
mod tests {
    use super::*;

    const STGCRG_MAINLINE: &[u8] = include_bytes!("../tests/fixtures/jh7110-stgcrg-mainline.dtb");
    const CLKGEN_VENDOR: &[u8] = include_bytes!("../tests/fixtures/jh7110-clkgen-vendor.dtb");
    const CLKGEN_VENDOR_UNNAMED: &[u8] =
        include_bytes!("../tests/fixtures/jh7110-clkgen-vendor-unnamed.dtb");
    /// The blob `crates/dtb`'s own tests boot-verify against, so a change to QEMU's `virt` board
    /// is caught here rather than surfacing as a mystery at the bench.
    const QEMU_RISCV64_VIRT: &[u8] =
        include_bytes!("../../dtb/tests/fixtures/qemu-riscv64-virt.dtb");

    #[test]
    fn mainline_stgcrg_is_read_from_its_single_reg() {
        let tree = dtb::Dtb::from_bytes(STGCRG_MAINLINE).unwrap();
        let found = discover(&tree).unwrap();
        assert_eq!(found.base, 0x1023_0000);
        assert_eq!(found.size, 0x1_0000);
        assert!(found.from_tree);
        assert_eq!(found.compatible, Some(COMPATIBLE_STGCRG));
    }

    #[test]
    fn the_vendor_clkgens_stg_window_is_found_by_name() {
        // The whole point of the vendor arm: `reg[0]` is the SYS domain at 0x13020000, and a
        // driver that took it would enable clock 15 of the wrong controller.
        let tree = dtb::Dtb::from_bytes(CLKGEN_VENDOR).unwrap();
        let found = discover(&tree).unwrap();
        assert_eq!(found.base, STG_BASE, "the second window, not the first");
        assert_eq!(found.size, 0x1_0000);
        assert!(found.from_tree);
        assert_eq!(found.compatible, Some(COMPATIBLE_VENDOR_CLKGEN));
    }

    #[test]
    fn a_multi_window_node_without_reg_names_falls_back_rather_than_guessing() {
        let tree = dtb::Dtb::from_bytes(CLKGEN_VENDOR_UNNAMED).unwrap();
        let found = discover(&tree).unwrap();
        assert_eq!(found.base, STG_BASE);
        assert!(
            !found.from_tree,
            "the address is right and nothing in the tree said so; the transcript must say which"
        );
        assert_eq!(found.compatible, None);
    }

    #[test]
    fn qemus_virt_board_has_no_clock_controller_at_all() {
        // This is the path every machine this repository's CI boots takes, and it is why this
        // milestone cannot be gated in an emulator: `virt` has no clock or reset controller to
        // program, so what runs here is the fallback and the arithmetic, never a device.
        let tree = dtb::Dtb::from_bytes(QEMU_RISCV64_VIRT).unwrap();
        let found = discover(&tree).unwrap();
        assert!(!found.from_tree);
        assert_eq!(found.compatible, None);
        assert_eq!(found.base, STG_BASE);
    }

    #[test]
    fn a_report_of_zeros_is_not_a_report_of_success() {
        // This is radon's 2026-09-04 shape, one level up: a window with nothing behind it accepts
        // every store and reads back zero, so `clocks_running` must be false on the *after*
        // words rather than on the fact that the writes returned.
        let dead = Report {
            clocks: 2,
            had_reset: true,
            ..Report::default()
        };
        assert!(!dead.clocks_running());
        assert!(!dead.was_already_up());
        assert!(!dead.released);
    }

    #[test]
    fn a_report_with_no_clock_steps_claims_nothing() {
        // `clocks: 0` must not read as "all zero of my clocks are running", which is what a bare
        // `.iter().all()` would say. A plan that enabled nothing has proven nothing.
        assert!(!Report::default().clocks_running());
        assert!(!Report::default().was_already_up());
    }

    #[test]
    fn the_firmware_having_already_done_it_is_a_distinguishable_outcome() {
        // The one result that would refute this milestone's premise: the clocks were already on
        // and the reset already released before anything here ran, so the TRNG's zeros have some
        // other cause. A bench transcript has to be able to say this, which is why the report
        // keeps `before` words at all.
        let up = Report {
            clock_before: [CLOCK_ENABLE | 4, CLOCK_ENABLE | 4, 0, 0],
            clock_after: [CLOCK_ENABLE | 4, CLOCK_ENABLE | 4, 0, 0],
            clocks: 2,
            had_reset: true,
            reset_assert_before: 0,
            reset_assert_after: 0,
            reset_status_after: 1 << STGRST_SEC_AHB,
            released: true,
            polls: 1,
            ..Report::default()
        };
        assert!(up.clocks_running());
        assert!(up.was_already_up());
    }

    #[test]
    fn a_device_this_run_actually_started_is_not_reported_as_already_up() {
        let started = Report {
            clock_before: [0, 0, 0, 0],
            clock_after: [CLOCK_ENABLE, CLOCK_ENABLE, 0, 0],
            clocks: 2,
            had_reset: true,
            reset_assert_before: 1 << STGRST_SEC_AHB,
            reset_assert_after: 0,
            reset_status_after: 1 << STGRST_SEC_AHB,
            released: true,
            polls: 3,
            ..Report::default()
        };
        assert!(started.clocks_running());
        assert!(!started.was_already_up(), "this run is what turned it on");
    }

    #[test]
    fn the_trng_plan_enables_both_clocks_before_it_touches_the_reset() {
        // [mainline-rst]'s own comment is the reason this order is a test rather than a comment:
        // "if the associated clock is gated, deasserting might otherwise hang forever". A plan
        // that deasserted first would wedge the boot on hardware and pass every host test.
        let reset_at = TRNG_BRING_UP
            .iter()
            .position(|s| matches!(s, Step::DeassertReset(_)))
            .expect("the plan must deassert something");
        let clocks = TRNG_BRING_UP
            .iter()
            .filter(|s| matches!(s, Step::EnableClock(_)))
            .count();
        assert_eq!(clocks, 2, "hclk and ahb, per [mainline-trng]'s probe");
        assert_eq!(reset_at, 2, "the reset comes last");
    }

    #[test]
    fn the_two_trees_agree_on_the_identifiers() {
        // The rebase arithmetic from [vendor-ids], written out so a reader can check it rather
        // than take the module header's word for it. If StarFive ever renumbers a group, this is
        // the test that fails.
        const VENDOR_STG_CLK_BASE: u32 = 190; // JH7110_HIFI4_CLK_CORE
        const VENDOR_SEC_HCLK: u32 = 205;
        const VENDOR_SEC_MISCAHB_CLK: u32 = 206;
        const VENDOR_STG_RST_BASE: u32 = 128; // RSTN_U0_STG_SYSCON_PRESETN
        const VENDOR_SEC_TOP_HRESETN: u32 = 131;

        assert_eq!(VENDOR_SEC_HCLK - VENDOR_STG_CLK_BASE, STGCLK_SEC_AHB);
        assert_eq!(
            VENDOR_SEC_MISCAHB_CLK - VENDOR_STG_CLK_BASE,
            STGCLK_SEC_MISC_AHB
        );
        assert_eq!(VENDOR_SEC_TOP_HRESETN - VENDOR_STG_RST_BASE, STGRST_SEC_AHB);
    }

    #[test]
    fn clock_words_are_one_per_index_and_bounded() {
        assert_eq!(STG.clock_offset(0), Some(0x00));
        assert_eq!(STG.clock_offset(STGCLK_SEC_AHB), Some(0x3c));
        assert_eq!(STG.clock_offset(STGCLK_SEC_MISC_AHB), Some(0x40));
        // 29 clocks, so 28 is the last. An identifier from another domain gets nothing rather
        // than an offset that lands somewhere plausible.
        assert_eq!(STG.clock_offset(28), Some(0x70));
        assert_eq!(STG.clock_offset(29), None);
        assert_eq!(STG.clock_offset(205), None); // the vendor number, un-rebased
    }

    #[test]
    fn the_last_clock_word_does_not_reach_the_reset_words() {
        // 0x70 is the last clock word and 0x74 is the assert register. They abut, which means an
        // off-by-one in `clocks` would write an enable bit into a reset word: a stuck-on clock
        // would be the *good* outcome, and resetting the STG matrix mid-boot the bad one.
        let last = STG.clock_offset(STG.clocks - 1).unwrap();
        assert!(last + 4 <= STG.reset_assert);
    }

    #[test]
    fn resets_are_thirty_two_to_a_word_and_bounded() {
        let r = STG.reset_bit(STGRST_SEC_AHB).unwrap();
        assert_eq!(r.assert_offset, 0x74);
        assert_eq!(r.status_offset, 0x78);
        assert_eq!(r.mask, 1 << 3);
        assert_eq!(STG.reset_bit(22).unwrap().mask, 1 << 22);
        assert_eq!(STG.reset_bit(23), None);
        assert_eq!(STG.reset_bit(131), None); // the vendor number, un-rebased
    }

    #[test]
    fn a_set_status_bit_means_out_of_reset() {
        let mask = STG.reset_bit(STGRST_SEC_AHB).unwrap().mask;
        assert!(deasserted(u32::MAX, mask));
        assert!(!deasserted(0, mask));
        assert!(!deasserted(!mask, mask), "every other bit must not count");
    }

    #[test]
    fn an_enabled_clock_reads_its_top_bit_back() {
        assert!(clock_enabled(CLOCK_ENABLE));
        assert!(clock_enabled(CLOCK_ENABLE | 0x1234));
        assert!(!clock_enabled(0x7fff_ffff));
        // Zero is what radon's TRNG window read on 2026-09-04, and it is what a gated clock
        // reads: the same value a window with nothing behind it reads, which is why the tour
        // reports `from_tree` and the before/after words rather than a verdict.
        assert!(!clock_enabled(0));
    }

    #[test]
    fn reg_names_index_by_name_not_by_position() {
        assert_eq!(name_index(b"sys\0stg\0aon\0", b"stg"), Some(1));
        assert_eq!(
            name_index(b"syscrg\0stgcrg\0aoncrg\0ispcrg\0voutcrg\0", b"stgcrg"),
            Some(1)
        );
        assert_eq!(name_index(b"sys\0aon\0", b"stg"), None);
        // A trailing NUL must not invent an empty fourth entry.
        assert_eq!(name_index(b"sys\0stg\0aon\0", b"aon"), Some(2));
        assert_eq!(name_index(b"", b"stg"), None);
    }
}
