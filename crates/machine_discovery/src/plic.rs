//! **Which PLIC context is each hart's S-mode?** The `interrupts-extended` property, decoded.
//!
//! The VisionFive 2 prep (2026-08-14, notes/visionfive2.md). The kernel used to derive a hart's
//! S-mode context as `2*hart + 1`, which is QEMU `virt`'s layout: every hart contributes an M
//! context and an S context, in hart order. The JH7110 breaks the formula in the quietest way
//! available: its hart 0 is an MMU-less S7 monitor core that contributes **only an M context**, so
//! from hart 1 on every context number shifts down by one and hart h's S context is `2h`, not
//! `2h + 1`. A kernel using the formula there programs its neighbour's context: enables, claims and
//! completes all land one context off, and every external interrupt sits pending forever.
//!
//! The layout was never a formula in the first place. It is the order of the PLIC node's
//! `interrupts-extended` entries: entry *k* names, by phandle, the per-hart interrupt controller
//! (`riscv,cpu-intc`) that context *k* feeds, and the interrupt number it raises there, 11
//! (machine external) or 9 (supervisor external). So context *k* is hart *h*'s S context exactly
//! when entry *k* points at hart *h*'s intc with interrupt 9, and this module reads that instead of
//! assuming.
//!
//! Two walks stitch the answer together, because the property speaks in phandles and the kernel
//! speaks in hart ids:
//!
//! 1. `cpu@` nodes give the hart ids ([`crate::cpu_list::CpuList`]), and each hart's child
//!    `interrupt-controller` node (`riscv,cpu-intc`) gives the phandle those entries use. The two
//!    lists align by tree order: each intc is nested inside its `cpu@` node, so the *i*-th
//!    `riscv,cpu-intc` in the tree belongs to the *i*-th `cpu@` node.
//! 2. The PLIC node, found by its binding ([`COMPATIBLES`], tried in order) rather than its label,
//!    because the label differs between the machines this has to work on: QEMU spells it
//!    `plic@c000000`, the JH7110 `interrupt-controller@c000000`. The binding strings differ too:
//!    QEMU virt's PLIC node lists `sifive,plic-1.0.0` and `riscv,plic0`, but the VisionFive 2's
//!    actual U-Boot-supplied control DTB lists only `riscv,plic0` (bench, 2026-08-21), the older,
//!    generic binding. The T-Head TH1520 lists neither: its node says `thead,th1520-plic`,
//!    `thead,c900-plic` (Linux `th1520.dtsi`, read 2026-09-25 for milestone 89 (Scaleway EM-RV1:
//!    a second RISC-V implementation, rented)).
//!
//! # BUGS
//!
//! - **Entries are assumed to be two cells wide** (one phandle cell, one interrupt cell), which is
//!   correct for every `riscv,cpu-intc` (`#interrupt-cells = <1>`) and is not checked against the
//!   referenced node. A PLIC wired to an interrupt parent with a different cell count would be
//!   misread; no RISC-V tree does this, and checking would mean resolving every phandle's
//!   `#interrupt-cells` first.
//! - **A `cpu@` node without a `riscv,cpu-intc` child breaks the index alignment** of walk 1 for
//!   every hart after it. Real trees give every hart an intc (a hart without one cannot take
//!   interrupts at all); a tree that does not simply yields no mapping for the later harts, which
//!   callers treat as "the tree did not say".
//! - **A hart id at or above [`MAX_CONTEXT_HARTS`] is not recorded.** Same truncation posture as
//!   [`crate::cpu_list::MAX_CPU_NODES`], and the same sixteen.
//!
//! Name: provisional (`plic` for the module, `PlicContexts` for the record), named for the
//! controller whose property it decodes; the naming tenet's "standard terms are already right"
//! group is the intent, as with the `device_tree_blob` crate.

use device_tree_blob::{DeviceTreeBlob, Error};

use crate::cpu_list::CpuList;

/// **Every `compatible` string this tree accepts as a PLIC**, in the order they are tried. One list,
/// because two lookups read it: [`PlicContexts::from_device_tree`] here, and the kernel's
/// `memory::init`, which finds the register block. Until 2026-09-25 each carried its own list and
/// they disagreed (the kernel's lacked `riscv,plic0`), which is the drift one list removes.
///
/// - `sifive,plic-1.0.0`: QEMU `virt` and the mainline JH7110 dtsi.
/// - `riscv,plic0`: the older generic binding, and the only one radon's U-Boot control DTB states.
/// - `thead,c900-plic`: T-Head's C9xx PLIC, the TH1520's (milestone 89), with the standard
///   register layout. Two T-Head differences are firmware's or a driver's, not this parser's:
///   M-mode must set bit 0 of the control word at `0x1f_fffc` before S-mode may touch the PLIC (OpenSBI does,
///   `PLIC_FLAG_THEAD_DELEGATION`), and an edge source must be completed before it is handled or
///   the next edge is lost (Linux `PLIC_QUIRK_EDGE_INTERRUPT`).
pub const COMPATIBLES: [&[u8]; 3] = [b"sifive,plic-1.0.0", b"riscv,plic0", b"thead,c900-plic"];

/// The most harts a context map records; matches [`crate::riscv64::MAX_HARTS`].
pub const MAX_CONTEXT_HARTS: usize = crate::riscv64::MAX_HARTS;

/// The interrupt a PLIC raises into a hart's `riscv,cpu-intc` for **supervisor** external
/// interrupts: `scause` 9. Its machine twin is 11; those are the only two that appear in a PLIC's
/// `interrupts-extended`.
pub const IRQ_S_EXT: u32 = 9;

/// **Each hart's S-mode PLIC context**, as the device tree lays it out.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct PlicContexts {
    /// `s[hart]` is hart `hart`'s S-mode context number, or `None` when the tree named none
    /// (an M-only hart, like the JH7110's S7; or a tree with no PLIC at all).
    s: [Option<u16>; MAX_CONTEXT_HARTS],
}

impl Default for PlicContexts {
    fn default() -> Self {
        Self {
            s: [None; MAX_CONTEXT_HARTS],
        }
    }
}

impl PlicContexts {
    /// Hart `hart`'s S-mode context, or `None` when the tree did not say (no PLIC node, no
    /// `interrupts-extended`, an M-only hart, or a hart past [`MAX_CONTEXT_HARTS`]).
    pub fn s_context(&self, hart: usize) -> Option<usize> {
        self.s.get(hart).copied().flatten().map(usize::from)
    }

    /// How many harts have an S context recorded.
    pub fn len(&self) -> usize {
        self.s.iter().filter(|c| c.is_some()).count()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// **Read the context layout.** A tree without a PLIC (aarch64, or a malformed board file) is
    /// an empty map, not an error, for the same reason [`CpuList::from_device_tree`] treats a bare
    /// `/cpus` that way: the caller falls back to what it would otherwise have assumed, and says so.
    pub fn from_device_tree(dt: &DeviceTreeBlob<'_>) -> Result<PlicContexts, Error> {
        let mut out = PlicContexts::default();

        // Real trees do not agree on which binding string they carry; [`COMPATIBLES`] says which
        // machine taught us each one. The VisionFive 2's control DTB found no node at all here
        // until "riscv,plic0" was added (bench, 2026-08-21).
        let mut entries = None;
        for compat in COMPATIBLES {
            if let Some(found) = dt.node_prop_compatible(compat, b"interrupts-extended")? {
                entries = Some(found);
                break;
            }
        }
        let Some(entries) = entries else {
            return Ok(out);
        };

        // Walk 1: hart ids, and the phandle of each hart's own interrupt controller. The
        // `interrupt-controller` name prefix also matches the PLIC's own node on boards that spell
        // it that way (the JH7110 does), so the parallel `compatible` read is what keeps only the
        // per-hart intcs, in tree order.
        let cpus = CpuList::from_device_tree(dt)?;
        let mut phandles = [None; MAX_CONTEXT_HARTS];
        let mut compats = [None; MAX_CONTEXT_HARTS];
        dt.node_props(b"interrupt-controller", b"phandle", &mut phandles)?;
        dt.node_props(b"interrupt-controller", b"compatible", &mut compats)?;

        // hart_of[i]: the hart id whose intc carries phandle intc_phandle[i], for the i-th
        // `riscv,cpu-intc` seen. The i-th such intc belongs to the i-th `cpu@` node (see module
        // docs on why tree order aligns them).
        let mut intc_phandle = [0u32; MAX_CONTEXT_HARTS];
        let mut hart_of = [0u64; MAX_CONTEXT_HARTS];
        let mut n = 0usize;
        for (ph, comp) in phandles.iter().zip(compats.iter()) {
            let Some(comp) = comp else { continue };
            if !comp.split(|&b| b == 0).any(|s| s == b"riscv,cpu-intc") {
                continue;
            }
            let Some(ph) = ph else {
                // A cpu-intc without a phandle can be referenced by nothing, so no
                // `interrupts-extended` entry can name it; it still consumes its cpu slot.
                n += 1;
                continue;
            };
            if ph.len() >= 4
                && n < MAX_CONTEXT_HARTS
                && let Some(cpu) = cpus.cpus().get(n)
            {
                intc_phandle[n] = u32::from_be_bytes([ph[0], ph[1], ph[2], ph[3]]);
                hart_of[n] = cpu.hwid;
            }
            n += 1;
        }
        let n = n.min(MAX_CONTEXT_HARTS).min(cpus.len);

        // Walk 2's payoff: entry k is context k. Two big-endian cells per entry, phandle then
        // interrupt number; an entry whose interrupt is not 9 (the machine externals, and the
        // 0xffff_ffff "no interrupt" some SiFive trees write) defines no S context.
        for (context, entry) in entries.as_chunks::<8>().0.iter().enumerate() {
            let phandle = u32::from_be_bytes([entry[0], entry[1], entry[2], entry[3]]);
            let irq = u32::from_be_bytes([entry[4], entry[5], entry[6], entry[7]]);
            if irq != IRQ_S_EXT {
                continue;
            }
            for i in 0..n {
                if intc_phandle[i] == phandle {
                    let hart = hart_of[i] as usize;
                    if hart < MAX_CONTEXT_HARTS && context <= u16::MAX as usize {
                        out.s[hart] = Some(context as u16);
                    }
                }
            }
        }

        Ok(out)
    }
}
