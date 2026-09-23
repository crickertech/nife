//! **A device tree for a machine that has none**: the ACPI tables an aarch64 server describes
//! itself with, written out as the flattened tree this kernel already reads.
//!
//! # Why this shape, and not the other one
//!
//! The x86_64 half of this loader hands the kernel an ACPI root pointer and the kernel reads the
//! tables itself. Mirroring that here would mean a second discovery path through
//! `kernel/src/memory.rs`, `smp.rs`, `console.rs` and `arch/aarch64/`, every one of which asks the
//! device tree today, **and a change to what the loader and the kernel agree on across the
//! handoff**: `x0` is the physical address of a device tree blob (Linux's arm64 boot protocol, which
//! `boot.s` cites and U-Boot's `booti` meets). That is a wire format, and wire formats are decided
//! by the architect rather than by a lane.
//!
//! So this takes the other road, which costs one module and changes nothing anybody agreed on: the
//! loader reads the tables while the firmware is still alive and **writes the tree the kernel would
//! have been handed**. The kernel boots on an ACPI machine without knowing that ACPI exists.
//!
//! **This is a translation and it loses information**, which is the honest reason the firmware's own
//! device tree stays first in [`super::arch`]'s order when a machine offers both. See BUGS.
//!
//! # Tested by the reader, not by inspection
//!
//! Every test below decodes what this writes with `crates/device_tree_blob` and the
//! `machine_discovery` decoders the **kernel** uses (`gic::discover`, `cpu_list::CpuList`,
//! `aarch64::Psci`, `interrupt_id::of_node`, `DeviceTreeBlob::memory_regions`), so the writer and
//! the readers cannot drift apart without a host test failing in milliseconds. That is the rule
//! `handoff.rs` and `device_tree_patch.rs` already follow.
//!
//! # BUGS
//!
//! - **No PCI.** An ECAM window is in the MCFG, but a PCI node also needs `ranges` (the 32-bit and
//!   64-bit BAR windows) and an `interrupt-map`, and on an ACPI machine both of those live in the
//!   DSDT's `_CRS` and `_PRT`, which are AML. So no `pci-host-ecam-generic` node is written at all,
//!   `memory::pci_regions()` answers `None`, and the boot takes the same path it takes on the
//!   JH7110, which has no such node either. A bus that is there is invisible.
//! - **No virtio-mmio, no `fw_cfg`, no ITS.** Same reason for the first two (QEMU describes them in
//!   AML); the third is a node `machine_discovery::gic` does not read yet (milestone 317).
//! - **The memory map is the UEFI one, merged.** Runtime-services and ACPI regions are excluded, so
//!   the kernel will not hand out the tables it was described by, and conventional plus
//!   boot-services plus loader memory is offered as RAM. That is the same set the firmware's own
//!   device tree offers, which is why it is not narrower.
//! - **Register block sizes are architectural, not stated.** The MADT gives a GIC distributor's
//!   address and never its length, because the GIC architecture fixes the frame at 64 KiB. The same
//!   goes for the GICv2 CPU interface and the PL011's page. A machine whose frames are elsewhere
//!   would need this told rather than assumed, and nothing here would notice.
//! - **One `/memory` node per merged run, capped at [`MAX_RAM`].** A machine with more runs than
//!   that is described short, and nothing says so.
//!
//! Names in this module are **provisional** (2026-09-23): the module, [`Machine`], [`Cpu`],
//! [`Uart`], [`build`] and the root node's `compatible` string all name things a reader meets, and
//! naming is calef's call.

use machine_discovery::acpi::{GTDT_ACTIVE_LOW, GTDT_EDGE_TRIGGERED, Gtdt};
use machine_discovery::gic::Gic;

use crate::device_tree_patch::{BEGIN_NODE, END, END_NODE, Error, HEADER_LEN, MAGIC, Out, PROP};

/// The most cores this writes a `cpu@` node for. Sixteen, matching
/// `machine_discovery::cpu_list::MAX_CPU_NODES`, which is what would read them back.
pub const MAX_CPUS: usize = 16;

/// The most `/memory` nodes this writes. Sixteen, matching `kernel::memory`'s own `MAX_REGIONS`.
pub const MAX_RAM: usize = 16;

/// The GIC distributor's register frame, which the architecture fixes at 64 KiB and the MADT never
/// states. See this module's BUGS.
pub const GICD_LEN: u64 = 0x1_0000;

/// A GICv2 CPU interface's frame. 8 KiB, which is the architected `GICC` block plus its alias page.
pub const GICC_LEN: u64 = 0x2000;

/// A PL011's register page. The SPCR gives an address and no length, and every PL011 occupies one
/// 4 KiB page.
const PL011_LEN: u64 = 0x1000;

/// The phandle the interrupt controller gets, and the root's `interrupt-parent`. Any nonzero value
/// would do; this tree has exactly one controller, so it gets the first.
const INTC_PHANDLE: u32 = 1;

/// The GIC's SPI bank base: an absolute INTID of `32 + n` is `interrupts = <0 n ...>`.
const GIC_SPI_BASE: u32 = 32;
/// The GIC's PPI bank base: an absolute INTID of `16 + n` is `interrupts = <1 n ...>`.
const GIC_PPI_BASE: u32 = 16;

/// A device tree interrupt flag: the line is level-triggered and asserted high.
const IRQ_TYPE_LEVEL_HIGH: u32 = 4;
/// Level-triggered, asserted low.
const IRQ_TYPE_LEVEL_LOW: u32 = 8;
/// Edge-triggered, rising.
const IRQ_TYPE_EDGE_RISING: u32 = 1;
/// Edge-triggered, falling.
const IRQ_TYPE_EDGE_FALLING: u32 = 2;

/// One core, as the MADT's GIC CPU interface entry describes it.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct Cpu {
    /// `MPIDR_EL1[39:0]`, which becomes the node's `reg` and is the id `CPU_ON` takes.
    pub mpidr: u64,
    /// Firmware will start this one. A disabled entry still gets a node, marked `status =
    /// "disabled"`, because a tree that omitted it would be claiming a smaller machine.
    pub enabled: bool,
}

/// The firmware's console UART, from the SPCR.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Uart {
    /// Its register block, physical.
    pub address: u64,
    /// The absolute INTID it raises, when the table states one.
    pub interrupt: Option<u32>,
}

/// A span of physical memory.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct Ram {
    /// First byte.
    pub start: u64,
    /// How many.
    pub len: u64,
}

/// **Everything read out of ACPI and the firmware's memory map**, which is exactly what [`build`]
/// turns into a tree. Assembled by the caller, because reading physical memory is the caller's
/// unsafety and this module is pure.
#[derive(Copy, Clone, Debug)]
pub struct Machine {
    /// The interrupt controller, in the same record `machine_discovery::gic::discover` hands the
    /// kernel back. Writing the node this type reads, and reading it back in a test, is what keeps
    /// the two halves honest.
    pub gic: Gic,
    /// The cores, in MADT order.
    pub cpus: [Cpu; MAX_CPUS],
    /// How many of [`Machine::cpus`] are real.
    pub cpu_count: usize,
    /// `Some(true)` for PSCI over `hvc`, `Some(false)` for `smc`, `None` for a machine whose FADT
    /// does not claim PSCI. The kernel already treats a missing `/psci` as "one core", so `None`
    /// costs the secondaries and not the boot.
    pub psci_hvc: Option<bool>,
    /// The architected timer's interrupts, when there is a GTDT.
    pub timer: Option<Gtdt>,
    /// The console UART, when there is an SPCR naming one.
    pub uart: Option<Uart>,
    /// RAM, merged into runs.
    pub ram: [Ram; MAX_RAM],
    /// How many of [`Machine::ram`] are real.
    pub ram_count: usize,
}

/// The property names this writer uses, as one NUL-separated block. Every `FDT_PROP` token carries
/// an offset into it, and the block is emitted verbatim: the tree is small and fixed enough that a
/// table beats an interning pass.
const STRINGS: &[u8] = b"#address-cells\0#size-cells\0#interrupt-cells\0compatible\0reg\0device_type\0interrupt-controller\0interrupt-parent\0interrupts\0phandle\0method\0enable-method\0status\0";

/// Offset of `name` in [`STRINGS`].
///
/// # Panics
///
/// If `name` is not in the table, which is a bug in this file rather than anything a machine can
/// cause: every caller passes a literal that is written above.
fn string_offset(name: &[u8]) -> u32 {
    let mut at = 0usize;
    while at < STRINGS.len() {
        let end = at
            + STRINGS[at..]
                .iter()
                .position(|&b| b == 0)
                .expect("the strings block is NUL-terminated throughout");
        if &STRINGS[at..end] == name {
            return at as u32;
        }
        at = end + 1;
    }
    panic!("a property name this writer emits is missing from its own strings block")
}

/// The tree-writing verbs, on top of the byte-level [`Out`] the patcher already had.
trait WriteNode {
    fn begin(&mut self, name: &[u8]) -> Result<(), Error>;
    fn end_node(&mut self) -> Result<(), Error>;
    fn prop(&mut self, name: &[u8], value: &[u8]) -> Result<(), Error>;
    fn prop_cells(&mut self, name: &[u8], cells: &[u32]) -> Result<(), Error>;
    fn prop_empty(&mut self, name: &[u8]) -> Result<(), Error>;
}

impl WriteNode for Out<'_> {
    fn begin(&mut self, name: &[u8]) -> Result<(), Error> {
        self.u32(BEGIN_NODE)?;
        self.bytes(name)?;
        self.bytes(&[0])?;
        self.pad_to(4)
    }

    fn end_node(&mut self) -> Result<(), Error> {
        self.u32(END_NODE)
    }

    fn prop(&mut self, name: &[u8], value: &[u8]) -> Result<(), Error> {
        self.u32(PROP)?;
        self.u32(value.len() as u32)?;
        self.u32(string_offset(name))?;
        self.bytes(value)?;
        self.pad_to(4)
    }

    fn prop_cells(&mut self, name: &[u8], cells: &[u32]) -> Result<(), Error> {
        self.u32(PROP)?;
        self.u32((cells.len() * 4) as u32)?;
        self.u32(string_offset(name))?;
        for cell in cells {
            self.u32(*cell)?;
        }
        Ok(())
    }

    fn prop_empty(&mut self, name: &[u8]) -> Result<(), Error> {
        self.prop(name, &[])
    }
}

/// A node name with a unit address, written into `buf` and returned as the slice that was used.
///
/// Device tree unit addresses are lower-case hex with no `0x`, and the node name is what the kernel
/// matches on (`console::UART_NODE` is the literal `pl011@9000000`), so the spelling is load-bearing
/// rather than cosmetic.
fn unit_name<'a>(prefix: &[u8], address: u64, buf: &'a mut [u8; 40]) -> &'a [u8] {
    buf[..prefix.len()].copy_from_slice(prefix);
    let mut at = prefix.len();
    buf[at] = b'@';
    at += 1;
    let mut shift = 60i32;
    let mut started = false;
    while shift >= 0 {
        let nibble = ((address >> shift) & 0xf) as u8;
        shift -= 4;
        if nibble == 0 && !started && shift >= 0 {
            continue;
        }
        started = true;
        buf[at] = if nibble < 10 {
            b'0' + nibble
        } else {
            b'a' + nibble - 10
        };
        at += 1;
    }
    &buf[..at]
}

/// The two cells of a 64-bit value, high first, which is how a `#address-cells = <2>` tree spells
/// an address.
const fn cells64(v: u64) -> [u32; 2] {
    [(v >> 32) as u32, v as u32]
}

/// A GTDT timer's `(GSIV, flags)` as the three cells a GIC's `interrupts` entry wants.
///
/// The architected timers are all PPIs, so a GSIV below [`GIC_PPI_BASE`] is a table this writer does
/// not understand rather than a number to subtract into the negatives; such a timer is written as
/// PPI 0, which arms nothing, and the kernel's own `TIMER_INTID` constant is what it actually uses.
/// Nothing here can make it arm the wrong line.
fn timer_cells(timer: (u32, u32)) -> [u32; 3] {
    let (gsiv, flags) = timer;
    let ppi = gsiv.saturating_sub(GIC_PPI_BASE);
    [1, ppi, interrupt_flags(flags)]
}

/// GTDT interrupt flags as a device tree interrupt type.
const fn interrupt_flags(flags: u32) -> u32 {
    match (
        flags & GTDT_EDGE_TRIGGERED != 0,
        flags & GTDT_ACTIVE_LOW != 0,
    ) {
        (false, false) => IRQ_TYPE_LEVEL_HIGH,
        (false, true) => IRQ_TYPE_LEVEL_LOW,
        (true, false) => IRQ_TYPE_EDGE_RISING,
        (true, true) => IRQ_TYPE_EDGE_FALLING,
    }
}

/// **An upper bound on the tree [`build`] will write**, which is what the caller allocates.
///
/// Deliberately generous and deliberately not exact: every node here is a few dozen bytes, the
/// whole tree is under four kilobytes on any machine this can describe, and an allocation sized by
/// a second copy of the layout is a second place to be wrong.
pub const fn output_len() -> usize {
    // Header, reservation block, a fixed part (root, psci, cpus, intc, timer, uart), the per-core
    // and per-region nodes, and the strings block.
    HEADER_LEN + 16 + 1024 + (MAX_CPUS + MAX_RAM) * 128 + STRINGS.len() + 64
}

/// **Write `machine` as a flattened device tree into `out`**, returning its length.
///
/// `out` must be at least [`output_len`] bytes. The result has no `/chosen`;
/// `device_tree_patch::with_initrd` adds one, which is the same call the firmware's own tree goes
/// through, so this module has one job and the archive handoff keeps its single writer.
pub fn build(machine: &Machine, out: &mut [u8]) -> Result<usize, Error> {
    let mut o = Out { buf: out, at: 0 };
    o.bytes(&[0; HEADER_LEN])?; // filled in last, once the offsets are known
    o.pad_to(8)?;
    let off_rsvmap = o.at;
    // The reservation block's terminator, and nothing before it: what this tree reserves is said
    // with nodes, and the firmware's own reservations went away with `ExitBootServices`.
    o.bytes(&[0; 16])?;
    let off_struct = o.at;

    o.begin(b"")?;
    o.prop_cells(b"#address-cells", &[2])?;
    o.prop_cells(b"#size-cells", &[2])?;
    // Provisional (2026-09-23). A root `compatible` is required by the specification and read by
    // nothing in this kernel; what it is for is a person reading a dump and asking where this tree
    // came from, so it says.
    o.prop(b"compatible", b"nife,machine-from-acpi\0")?;
    o.prop_cells(b"interrupt-parent", &[INTC_PHANDLE])?;

    write_psci(&mut o, machine)?;
    write_cpus(&mut o, machine)?;
    write_intc(&mut o, machine)?;
    write_timer(&mut o, machine)?;
    write_uart(&mut o, machine)?;
    write_memory(&mut o, machine)?;

    o.end_node()?;
    o.u32(END)?;

    let size_struct = o.at - off_struct;
    let off_strings = o.at;
    o.bytes(STRINGS)?;
    let size_strings = o.at - off_strings;
    let total = o.at;

    let header: [u32; 10] = [
        MAGIC,
        total as u32,
        off_struct as u32,
        off_strings as u32,
        off_rsvmap as u32,
        17, // version
        16, // last compatible version
        0,  // boot_cpuid_phys: this writer does not know which core is running it
        size_strings as u32,
        size_struct as u32,
    ];
    for (i, word) in header.iter().enumerate() {
        o.buf[i * 4..i * 4 + 4].copy_from_slice(&word.to_be_bytes());
    }
    Ok(total)
}

/// `/psci`, when the FADT claims PSCI. A machine that does not gets no node, which
/// `machine_discovery::aarch64::Psci::from_device_tree` reads as `None` and the kernel reports as
/// one core rather than as a failure.
fn write_psci(o: &mut Out<'_>, machine: &Machine) -> Result<(), Error> {
    let Some(hvc) = machine.psci_hvc else {
        return Ok(());
    };
    o.begin(b"psci")?;
    // `arm,psci-1.0` first, which is what the FADT's flag means as of ACPI 6: a machine claiming
    // the bit implements at least 0.2, and the 1.0 string is what QEMU's own tree states. Both are
    // listed because both are accepted, and the decoder takes the standard `CPU_ON` id from either.
    o.prop(b"compatible", b"arm,psci-1.0\0arm,psci-0.2\0")?;
    o.prop(b"method", if hvc { b"hvc\0" } else { b"smc\0" })?;
    o.end_node()
}

/// `/cpus`, one `cpu@<mpidr>` per MADT GIC CPU interface entry.
fn write_cpus(o: &mut Out<'_>, machine: &Machine) -> Result<(), Error> {
    o.begin(b"cpus")?;
    // Two address cells, because an `MPIDR_EL1` affinity value is 40 bits and the ARM CPU binding
    // requires two the moment `Aff3` is nonzero. One would be right on QEMU and wrong on a machine
    // with clusters, and the decoder reads the declaration rather than assuming either.
    o.prop_cells(b"#address-cells", &[2])?;
    o.prop_cells(b"#size-cells", &[0])?;
    for cpu in &machine.cpus[..machine.cpu_count.min(MAX_CPUS)] {
        let mut buf = [0u8; 40];
        o.begin(unit_name(b"cpu", cpu.mpidr, &mut buf))?;
        o.prop(b"device_type", b"cpu\0")?;
        o.prop(b"compatible", b"arm,armv8\0")?;
        o.prop_cells(b"reg", &cells64(cpu.mpidr))?;
        o.prop(b"enable-method", b"psci\0")?;
        // A disabled MADT entry is firmware saying "a socket is here and I will not start it",
        // which is exactly what `status = "disabled"` says to `cpu_list::Cpu::startable`.
        if !cpu.enabled {
            o.prop(b"status", b"disabled\0")?;
        }
        o.end_node()?;
    }
    o.end_node()
}

/// The interrupt controller, in the binding `machine_discovery::gic::discover` reads back.
fn write_intc(o: &mut Out<'_>, machine: &Machine) -> Result<(), Error> {
    let (compatible, distributor, second) = match machine.gic {
        Gic::V2 {
            distributor,
            cpu_interface,
        } => (&b"arm,cortex-a15-gic\0"[..], distributor, cpu_interface),
        Gic::V3 {
            distributor,
            redistributors,
        } => (&b"arm,gic-v3\0"[..], distributor, redistributors),
    };
    let mut buf = [0u8; 40];
    o.begin(unit_name(b"intc", distributor.start, &mut buf))?;
    o.prop(b"compatible", compatible)?;
    o.prop_empty(b"interrupt-controller")?;
    // Three cells is the GIC binding: <type number flags>. `machine_discovery::interrupt_id` reads
    // this through the root's `interrupt-parent` phandle to decode the UART's line, so a wrong
    // count here would not be a wrong answer, it would be `None` and the documented fallback.
    o.prop_cells(b"#interrupt-cells", &[3])?;
    o.prop_cells(b"#address-cells", &[2])?;
    o.prop_cells(b"#size-cells", &[2])?;
    o.prop_cells(b"phandle", &[INTC_PHANDLE])?;
    let mut reg = [0u32; 8];
    reg[..2].copy_from_slice(&cells64(distributor.start));
    reg[2..4].copy_from_slice(&cells64(distributor.size));
    reg[4..6].copy_from_slice(&cells64(second.start));
    reg[6..8].copy_from_slice(&cells64(second.size));
    o.prop_cells(b"reg", &reg)?;
    o.end_node()
}

/// The architected timer, from the GTDT. Nothing on the interrupt path reads it (`TIMER_INTID` is
/// a constant, and PPI 11 is architected), but `arch::aarch64::timer` cross-checks `CNTFRQ_EL0`
/// against this node's `clock-frequency` when one is stated, and a dump that names the machine's
/// timer is worth the forty bytes.
fn write_timer(o: &mut Out<'_>, machine: &Machine) -> Result<(), Error> {
    let Some(gtdt) = machine.timer else {
        return Ok(());
    };
    o.begin(b"timer")?;
    o.prop(b"compatible", b"arm,armv8-timer\0")?;
    // The binding's order: secure EL1, non-secure EL1, virtual, and the EL2 physical timer. The
    // same four the GTDT states, in the same order, which is why this is a copy and not a decision.
    let mut cells = [0u32; 12];
    cells[0..3].copy_from_slice(&timer_cells(gtdt.secure_el1));
    cells[3..6].copy_from_slice(&timer_cells(gtdt.non_secure_el1));
    cells[6..9].copy_from_slice(&timer_cells(gtdt.virtual_el1));
    cells[9..12].copy_from_slice(&timer_cells(gtdt.el2));
    o.prop_cells(b"interrupts", &cells)?;
    // **No `clock-frequency`.** The GTDT has no such field; the rate is `CNTFRQ_EL0`, which the
    // kernel reads directly. Writing the register's own value back as a property would turn the
    // kernel's cross-check into a comparison of a number with itself.
    o.end_node()
}

/// The console UART, from the SPCR, named so that `console::UART_NODE`'s prefix match finds it.
fn write_uart(o: &mut Out<'_>, machine: &Machine) -> Result<(), Error> {
    let Some(uart) = machine.uart else {
        return Ok(());
    };
    let mut buf = [0u8; 40];
    o.begin(unit_name(b"pl011", uart.address, &mut buf))?;
    o.prop(b"compatible", b"arm,pl011\0arm,primecell\0")?;
    let mut reg = [0u32; 4];
    reg[..2].copy_from_slice(&cells64(uart.address));
    reg[2..4].copy_from_slice(&cells64(PL011_LEN));
    o.prop_cells(b"reg", &reg)?;
    if let Some(gsiv) = uart.interrupt {
        // A UART raises an SPI. A GSIV below the SPI base would be a PPI or an SGI, which no UART
        // raises and this writer will not translate into a number that means something else; the
        // property is omitted and the kernel says it fell back, which is the honest report.
        if let Some(spi) = gsiv.checked_sub(GIC_SPI_BASE) {
            // Level-high is what every PL011 binding states and what QEMU's own tree says (`<0 1
            // 4>`). The SPCR has no polarity field to read, so this is the binding's value rather
            // than the machine's claim, and it is the one place in this file that is neither.
            o.prop_cells(b"interrupts", &[0, spi, IRQ_TYPE_LEVEL_HIGH])?;
        }
    }
    o.end_node()
}

/// One `/memory` node per merged run of RAM.
fn write_memory(o: &mut Out<'_>, machine: &Machine) -> Result<(), Error> {
    for run in &machine.ram[..machine.ram_count.min(MAX_RAM)] {
        let mut buf = [0u8; 40];
        o.begin(unit_name(b"memory", run.start, &mut buf))?;
        o.prop(b"device_type", b"memory\0")?;
        let mut reg = [0u32; 4];
        reg[..2].copy_from_slice(&cells64(run.start));
        reg[2..4].copy_from_slice(&cells64(run.len));
        o.prop_cells(b"reg", &reg)?;
        o.end_node()?;
    }
    Ok(())
}
