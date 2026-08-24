//! **ACPI: what x86 has instead of a device tree.**
//!
//! Milestone 161. This module sits beside the arch records rather than inside the `x86_64` one, for
//! the reason [`cpu_list`](crate::cpu_list) does: ACPI is not an x86 standard. Every aarch64 server
//! that is not a device-tree board describes itself with exactly these tables, and milestone 20's
//! own roadmap text says the machine after the VisionFive 2 "should probably be a UEFI/ACPI machine
//! rather than another Device Tree board". x86 is simply its first consumer here.
//!
//! # The shape, and how it differs from a device tree
//!
//! A device tree is one blob with one root and a tree of nodes, and `crates/dtb` walks it. ACPI is a
//! **linked structure of independent tables**, each with its own signature and checksum, reached
//! from a root pointer that is not itself a table:
//!
//! ```text
//!   RSDP  ("RSD PTR ")            found by scanning low memory, or handed over by the loader
//!    |
//!    +-- RSDT (32-bit pointers)   ACPI 1.0
//!    +-- XSDT (64-bit pointers)   ACPI 2.0+, and what to prefer when both exist
//!         |
//!         +-- APIC ("MADT")       the local APICs, the IO APICs, and the interrupt rewiring
//!         +-- MCFG                where the PCIe ECAM window is
//!         +-- DMAR                where the IOMMU is
//!         +-- FACP, HPET, WAET, ...
//! ```
//!
//! **Every table is checksummed and this module checks**, which a device tree has no equivalent of.
//! That is not politeness: the RSDP is found by *scanning memory for a string*, so without the
//! checksum any sixteen bytes that happen to spell `RSD PTR ` would be believed.
//!
//! # What is here and what is deliberately not
//!
//! Here: the RSDP, the root table walk, the SDT header, and the two tables the kernel needs first
//! (MADT and MCFG). Not here, and it is the big one: **AML**, the bytecode in the DSDT that
//! describes everything ACPI does not have a fixed table for, including PCI interrupt routing
//! (`_PRT`) and every power-management method. AML needs an interpreter, which is a project rather
//! than a parser, and nothing in this kernel needs it yet.
//!
//! # BUGS
//!
//! - **Nothing validates that two tables do not overlap**, or that a table's length is sane
//!   relative to where it sits in memory. The caller supplies the bytes, so the caller is where a
//!   physical-address sanity check belongs.
//! - **The MADT's `flags` bit 0 (`PCAT_COMPAT`) is reported and not acted on.** It means the machine
//!   also has 8259 PICs that must be masked before the APICs are used. Whoever brings the APIC up
//!   has to mask them; this only says whether they are there.

/// The eight bytes that begin an RSDP. Note the trailing space; it is part of the signature.
pub const RSDP_SIGNATURE: &[u8; 8] = b"RSD PTR ";

/// The length of the ACPI 1.0 RSDP, which is what its checksum covers.
const RSDP_V1_LEN: usize = 20;
/// The length of the ACPI 2.0 RSDP, which carries a second checksum over the whole thing.
const RSDP_V2_LEN: usize = 36;

/// Every system descriptor table begins with this many bytes of common header.
pub const SDT_HEADER_LEN: usize = 36;

/// Why an ACPI structure could not be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcpiError {
    /// The signature is not the one expected.
    BadSignature,
    /// The bytes end before the structure does.
    Truncated,
    /// The bytes sum to something other than zero. **The most important error here**, because the
    /// RSDP is found by scanning for a string and this is the only thing that separates a real one
    /// from a coincidence.
    BadChecksum,
    /// A length field says something impossible (shorter than its own header).
    BadLength(u32),
}

/// Do these bytes sum to zero in eight-bit arithmetic? That is ACPI's checksum for every structure
/// it defines.
pub fn checksum_ok(bytes: &[u8]) -> bool {
    bytes.iter().fold(0u8, |acc, b| acc.wrapping_add(*b)) == 0
}

/// The root pointer, decoded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rsdp {
    /// 0 for ACPI 1.0 (RSDT only), 2 or more for ACPI 2.0+ (XSDT present).
    pub revision: u8,
    /// The 32-bit root table's physical address. Always present.
    pub rsdt: u32,
    /// The 64-bit root table's physical address, or 0 on a revision-0 RSDP.
    pub xsdt: u64,
}

impl Rsdp {
    /// **Which root table to walk.** The XSDT when there is one, because a machine with memory
    /// above 4 GiB can have tables the RSDT's 32-bit pointers cannot name, and a firmware that
    /// publishes both is not required to list the same tables in each. Returns
    /// `(physical_address, entries_are_64_bit)`.
    pub const fn root_table(&self) -> (u64, bool) {
        if self.revision >= 2 && self.xsdt != 0 {
            (self.xsdt, true)
        } else {
            (self.rsdt as u64, false)
        }
    }
}

/// Decode an RSDP from bytes beginning at its signature.
///
/// Both checksums are checked when the revision says there are two. The first covers the first 20
/// bytes (which is the whole ACPI 1.0 structure, and is why the field order was never allowed to
/// change); the second covers all 36.
pub fn parse_rsdp(bytes: &[u8]) -> Result<Rsdp, AcpiError> {
    if bytes.len() < RSDP_V1_LEN {
        return Err(AcpiError::Truncated);
    }
    if &bytes[0..8] != RSDP_SIGNATURE {
        return Err(AcpiError::BadSignature);
    }
    if !checksum_ok(&bytes[..RSDP_V1_LEN]) {
        return Err(AcpiError::BadChecksum);
    }
    let revision = bytes[15];
    let rsdt = u32(bytes, 16);

    if revision < 2 {
        return Ok(Rsdp {
            revision,
            rsdt,
            xsdt: 0,
        });
    }

    if bytes.len() < RSDP_V2_LEN {
        return Err(AcpiError::Truncated);
    }
    // The extended structure's own length field, at offset 20. Trusted only as far as the second
    // checksum, which is computed over exactly what it claims.
    let length = u32(bytes, 20) as usize;
    if !(RSDP_V2_LEN..=bytes.len()).contains(&length) {
        return Err(AcpiError::BadLength(length as u32));
    }
    if !checksum_ok(&bytes[..length]) {
        return Err(AcpiError::BadChecksum);
    }
    Ok(Rsdp {
        revision,
        rsdt,
        xsdt: u64(bytes, 24),
    })
}

/// The header every system descriptor table begins with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SdtHeader {
    /// Four ASCII characters: `APIC`, `MCFG`, `DMAR`, `FACP`.
    pub signature: [u8; 4],
    /// The table's total length **including** this header.
    pub length: u32,
    pub revision: u8,
    /// Six ASCII characters naming the firmware vendor. Read only for the boot print, and worth
    /// printing: it is the one place a firmware identifies itself before anything else runs.
    pub oem_id: [u8; 6],
}

impl SdtHeader {
    /// The signature as a string, for printing. `None` if it is not ASCII, which would mean the
    /// bytes are not a table.
    pub fn signature_str(&self) -> Option<&str> {
        core::str::from_utf8(&self.signature).ok()
    }

    /// How many bytes follow this header in the table.
    pub const fn body_len(&self) -> usize {
        self.length as usize - SDT_HEADER_LEN
    }
}

/// Decode a table header, without checking the checksum (which needs the whole table, and the
/// caller has to read `length` bytes before it can have them).
pub fn parse_sdt_header(bytes: &[u8]) -> Result<SdtHeader, AcpiError> {
    if bytes.len() < SDT_HEADER_LEN {
        return Err(AcpiError::Truncated);
    }
    let length = u32(bytes, 4);
    if (length as usize) < SDT_HEADER_LEN {
        return Err(AcpiError::BadLength(length));
    }
    Ok(SdtHeader {
        signature: [bytes[0], bytes[1], bytes[2], bytes[3]],
        length,
        revision: bytes[8],
        // Offset 10, not 9: byte 8 is the revision and byte 9 is the table's own checksum. Getting
        // this off by one reads the checksum as the first letter of the vendor's name, which prints
        // as garbage and is the kind of mistake that survives review because nothing depends on it.
        oem_id: [
            bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15],
        ],
    })
}

/// How many table pointers a root table of `length` bytes holds.
pub const fn root_entry_count(length: u32, entries_are_64_bit: bool) -> usize {
    let body = (length as usize).saturating_sub(SDT_HEADER_LEN);
    if entries_are_64_bit {
        body / 8
    } else {
        body / 4
    }
}

/// The physical address of root-table entry `index`. `body` begins **after** the SDT header.
///
/// Split from the header this way because the kernel reads the header first to learn the length,
/// then reads the body; handing this the whole table would mean the caller doing the same offset
/// arithmetic twice, in two places, with one chance to get it wrong.
pub fn root_entry(body: &[u8], index: usize, entries_are_64_bit: bool) -> Option<u64> {
    let width = if entries_are_64_bit { 8 } else { 4 };
    let at = index.checked_mul(width)?;
    if body.len() < at.checked_add(width)? {
        return None;
    }
    Some(if entries_are_64_bit {
        u64(body, at)
    } else {
        u32(body, at) as u64
    })
}

// ---------------------------------------------------------------------------------------------
// The MADT (signature "APIC"): where the interrupt controllers are.
// ---------------------------------------------------------------------------------------------

/// The MADT's fixed part, which precedes its variable-length entry list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Madt {
    /// The local APIC's physical address as the 32-bit field states it. A
    /// [`MadtEntry::LocalApicAddressOverride`] later in the list replaces it, which is why this is
    /// not the final answer on every machine.
    pub local_apic: u32,
    /// Bit 0 (`PCAT_COMPAT`) means the machine also has 8259 PICs. See this module's BUGS.
    pub flags: u32,
}

/// True when the machine has legacy 8259 PICs that must be masked before the APICs are used.
pub const MADT_PCAT_COMPAT: u32 = 1 << 0;

/// The eight bytes of the MADT that precede its entries.
const MADT_FIXED_LEN: usize = 8;

/// Decode the MADT's fixed part. `body` begins after the SDT header.
pub fn parse_madt(body: &[u8]) -> Result<Madt, AcpiError> {
    if body.len() < MADT_FIXED_LEN {
        return Err(AcpiError::Truncated);
    }
    Ok(Madt {
        local_apic: u32(body, 0),
        flags: u32(body, 4),
    })
}

/// One entry of the MADT's list. Only the four kinds this kernel will act on are decoded; the rest
/// keep their type byte so a boot print can say what it skipped rather than pretending the list was
/// shorter than it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MadtEntry {
    /// A CPU, named by its local APIC id. **`enabled` is what decides whether it can be started**:
    /// a processor entry exists for sockets that are empty and for cores the firmware disabled.
    LocalApic {
        processor_id: u8,
        apic_id: u8,
        /// The CPU is usable now.
        enabled: bool,
        /// The CPU is not enabled but could be brought online later (hot-plug). Not the same as
        /// `enabled`, and starting one that is merely online-capable is a different operation.
        online_capable: bool,
    },
    /// An IO APIC: where device interrupts arrive. `gsi_base` is the first global system interrupt
    /// this one owns, which is how a machine with several of them divides the space.
    IoApic { id: u8, address: u32, gsi_base: u32 },
    /// **A legacy IRQ has been rewired.** The ISA interrupt `source` actually arrives as global
    /// system interrupt `gsi`, with `flags` giving polarity and trigger mode. This is the entry that
    /// makes "COM1 is IRQ 4" not mean "COM1 is IO APIC input 4": on almost every machine the timer's
    /// IRQ 0 is remapped, and reading this wrong arms the wrong line.
    InterruptSourceOverride {
        bus: u8,
        source: u8,
        gsi: u32,
        flags: u16,
    },
    /// The local APIC is not at the 32-bit address the fixed part gave; it is here.
    LocalApicAddressOverride(u64),
    /// A kind this decoder does not act on, with its type byte.
    Other(u8),
}

/// Walks a MADT's entry list. Each entry is `[type, length, ...]`, so the list is self-describing
/// and a length of zero would loop forever; that is refused by ending the walk.
pub struct MadtEntries<'a> {
    body: &'a [u8],
    at: usize,
}

/// Iterate the MADT's entries. `body` begins after the SDT header.
pub fn madt_entries(body: &[u8]) -> MadtEntries<'_> {
    MadtEntries {
        body,
        at: MADT_FIXED_LEN,
    }
}

impl Iterator for MadtEntries<'_> {
    type Item = MadtEntry;

    fn next(&mut self) -> Option<MadtEntry> {
        if self.at + 2 > self.body.len() {
            return None;
        }
        let kind = self.body[self.at];
        let len = self.body[self.at + 1] as usize;
        // A zero (or absurdly short) length is a malformed table, and continuing would either loop
        // forever or read the next entry from the middle of this one. Stop, which reports the
        // entries read so far rather than inventing more.
        if len < 2 || self.at + len > self.body.len() {
            return None;
        }
        let e = &self.body[self.at..self.at + len];
        self.at += len;

        Some(match kind {
            0 if len >= 8 => {
                let flags = u32(e, 4);
                MadtEntry::LocalApic {
                    processor_id: e[2],
                    apic_id: e[3],
                    enabled: flags & 1 != 0,
                    online_capable: flags & 2 != 0,
                }
            }
            1 if len >= 12 => MadtEntry::IoApic {
                id: e[2],
                address: u32(e, 4),
                gsi_base: u32(e, 8),
            },
            2 if len >= 10 => MadtEntry::InterruptSourceOverride {
                bus: e[2],
                source: e[3],
                gsi: u32(e, 4),
                flags: u16(e, 8),
            },
            5 if len >= 12 => MadtEntry::LocalApicAddressOverride(u64(e, 4)),
            other => MadtEntry::Other(other),
        })
    }
}

// ---------------------------------------------------------------------------------------------
// Legacy IRQ numbers, and the overrides that make them a lie.
// ---------------------------------------------------------------------------------------------

/// **The polarity field of an MPS INTI flags word**, bits 1:0. `00` means "whatever this bus
/// specifies", `01` active high, `11` active low. `10` is reserved.
pub const INTI_POLARITY_MASK: u16 = 0b11;
/// Polarity `11`: the line is asserted low.
const INTI_POLARITY_ACTIVE_LOW: u16 = 0b11;

/// **The trigger-mode field**, bits 3:2. `00` means "whatever this bus specifies", `01` edge, `11`
/// level. `10` is reserved.
pub const INTI_TRIGGER_MASK: u16 = 0b11 << 2;
/// Trigger `11`: the line stays asserted until the device is serviced.
const INTI_TRIGGER_LEVEL: u16 = 0b11 << 2;

/// The bus number an [`MadtEntry::InterruptSourceOverride`] uses for the ISA bus. It is the only
/// value ACPI defines for that field, which is why the overrides are exactly the legacy IRQs.
pub const ISA_BUS: u8 = 0;

/// How many legacy ISA IRQs there are: two cascaded 8259s, eight lines each.
pub const ISA_IRQ_COUNT: usize = 16;

/// **How one ISA interrupt actually reaches an IO APIC.**
///
/// The whole point of this type is that `gsi` is very often *not* the IRQ number it was looked up
/// by. On essentially every PC the timer's IRQ 0 arrives as global system interrupt 2, because the
/// PIT is wired to the IO APIC's pin 2 while pin 0 carries the 8259 cascade. A kernel that armed
/// redirection entry 0 for "the timer" would arm a line nothing drives, and would see no
/// interrupts and no error.
///
/// **Provisional name** (milestone 161), along with the two fields and [`isa_irq_table`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IsaIrqRouting {
    /// The global system interrupt this IRQ arrives on.
    pub gsi: u32,
    /// The line is asserted low rather than high.
    pub active_low: bool,
    /// The line is level triggered rather than edge triggered.
    pub level_triggered: bool,
}

impl IsaIrqRouting {
    /// **What an ISA IRQ is when nothing overrides it**: identity-mapped onto the global interrupt
    /// space, active high, edge triggered. That is the ISA bus's own convention, which is what the
    /// `00` ("conforms to the specifications of the bus") encoding in an override's flags means.
    pub const fn isa_default(irq: u8) -> Self {
        Self {
            gsi: irq as u32,
            active_low: false,
            level_triggered: false,
        }
    }

    /// Apply an override's MPS INTI flags word. `00` in either field means "conforms to the bus",
    /// which for the ISA bus is what [`isa_default`](Self::isa_default) already set, so those bits
    /// deliberately change nothing.
    const fn with_flags(mut self, flags: u16) -> Self {
        if flags & INTI_POLARITY_MASK == INTI_POLARITY_ACTIVE_LOW {
            self.active_low = true;
        }
        if flags & INTI_TRIGGER_MASK == INTI_TRIGGER_LEVEL {
            self.level_triggered = true;
        }
        self
    }
}

/// **Resolve all sixteen legacy ISA IRQs through the MADT's interrupt source overrides.** `body`
/// begins after the SDT header.
///
/// Returned as a whole table rather than one lookup at a time because the overrides are a list that
/// has to be walked to answer any single question, and the caller (an interrupt controller being
/// brought up) wants the answers to outlive the table's bytes.
///
/// An override naming a source outside 0..16 is ignored: the field is a legacy IRQ number and there
/// are sixteen of those, so a larger one is a malformed table rather than a seventeenth IRQ.
pub fn isa_irq_table(body: &[u8]) -> [IsaIrqRouting; ISA_IRQ_COUNT] {
    let mut table = core::array::from_fn(|irq| IsaIrqRouting::isa_default(irq as u8));
    for entry in madt_entries(body) {
        if let MadtEntry::InterruptSourceOverride {
            bus,
            source,
            gsi,
            flags,
        } = entry
            && bus == ISA_BUS
            && (source as usize) < ISA_IRQ_COUNT
        {
            table[source as usize] = IsaIrqRouting {
                gsi,
                active_low: false,
                level_triggered: false,
            }
            .with_flags(flags);
        }
    }
    table
}

// ---------------------------------------------------------------------------------------------
// The MCFG: where the PCIe ECAM window is.
// ---------------------------------------------------------------------------------------------

/// One ECAM window: a range of buses on one PCI segment, and the physical address their
/// configuration space is memory-mapped at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct McfgEntry {
    pub base: u64,
    pub segment: u16,
    pub start_bus: u8,
    pub end_bus: u8,
}

impl McfgEntry {
    /// How many bytes of configuration space this window covers. One bus is 1 MiB (32 devices x 8
    /// functions x 4 KiB), and the range is inclusive at both ends.
    pub const fn size(&self) -> u64 {
        (self.end_bus as u64 - self.start_bus as u64 + 1) * 0x10_0000
    }
}

/// The MCFG's eight reserved bytes before its entry list.
const MCFG_FIXED_LEN: usize = 8;
/// The size of one MCFG allocation entry.
const MCFG_ENTRY_LEN: usize = 16;

/// Decode MCFG entry `index`. `body` begins after the SDT header.
pub fn mcfg_entry(body: &[u8], index: usize) -> Option<McfgEntry> {
    let at = MCFG_FIXED_LEN + index.checked_mul(MCFG_ENTRY_LEN)?;
    if body.len() < at.checked_add(MCFG_ENTRY_LEN)? {
        return None;
    }
    Some(McfgEntry {
        base: u64(body, at),
        segment: u16(body, at + 8),
        start_bus: body[at + 10],
        end_bus: body[at + 11],
    })
}

fn u16(bytes: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([bytes[at], bytes[at + 1]])
}

fn u32(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
}

fn u64(bytes: &[u8], at: usize) -> u64 {
    let mut w = [0u8; 8];
    w.copy_from_slice(&bytes[at..at + 8]);
    u64::from_le_bytes(w)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Set the checksum byte at `at` so `bytes[..len]` sums to zero, the way firmware does when it
    /// writes a table.
    fn seal(bytes: &mut [u8], at: usize, len: usize) {
        bytes[at] = 0;
        let sum = bytes[..len].iter().fold(0u8, |a, b| a.wrapping_add(*b));
        bytes[at] = sum.wrapping_neg();
    }

    fn rsdp_v2() -> [u8; RSDP_V2_LEN] {
        let mut b = [0u8; RSDP_V2_LEN];
        b[0..8].copy_from_slice(RSDP_SIGNATURE);
        b[9..15].copy_from_slice(b"BOCHS ");
        b[15] = 2; // revision
        b[16..20].copy_from_slice(&0x7ffe_1a40u32.to_le_bytes()); // rsdt
        b[20..24].copy_from_slice(&(RSDP_V2_LEN as u32).to_le_bytes());
        b[24..32].copy_from_slice(&0x7ffe_1b00u64.to_le_bytes()); // xsdt
        seal(&mut b, 8, RSDP_V1_LEN);
        seal(&mut b, 32, RSDP_V2_LEN);
        b
    }

    /// **The checksum is the whole defence.** The RSDP is found by scanning memory for an
    /// eight-byte string, so without this any sixteen bytes spelling `RSD PTR ` would be believed
    /// and the kernel would follow a pointer out of somebody's data.
    #[test]
    fn a_string_that_spells_the_signature_but_does_not_checksum_is_refused() {
        let mut b = rsdp_v2();
        b[17] ^= 0xff; // corrupt the RSDT pointer without fixing the checksum
        assert_eq!(parse_rsdp(&b), Err(AcpiError::BadChecksum));
    }

    /// A revision-2 RSDP has a second checksum over all 36 bytes, and it is checked too.
    #[test]
    fn the_extended_checksum_is_checked_as_well_as_the_short_one() {
        let mut b = rsdp_v2();
        b[30] ^= 0xff; // inside the extended part only; the 20-byte checksum still passes
        assert!(checksum_ok(&b[..RSDP_V1_LEN]), "the short one still passes");
        assert_eq!(parse_rsdp(&b), Err(AcpiError::BadChecksum));
    }

    /// A wrong signature is refused before anything else is read.
    #[test]
    fn a_wrong_signature_is_refused() {
        let mut b = rsdp_v2();
        b[7] = b'!';
        assert_eq!(parse_rsdp(&b), Err(AcpiError::BadSignature));
    }

    /// **The XSDT wins when there is one.** A machine with memory above 4 GiB can have tables the
    /// RSDT's 32-bit pointers cannot name, and firmware is not required to list the same tables in
    /// both.
    #[test]
    fn the_xsdt_is_preferred_and_its_entries_are_64_bit() {
        let r = parse_rsdp(&rsdp_v2()).expect("well-formed");
        assert_eq!(r.root_table(), (0x7ffe_1b00, true));
    }

    /// A revision-0 RSDP has no XSDT, so the RSDT is the only answer and its entries are 32-bit.
    /// The extended fields are not read at all, because on a 20-byte structure they do not exist.
    #[test]
    fn a_revision_0_rsdp_uses_the_rsdt_and_never_reads_the_extended_fields() {
        let mut b = rsdp_v2();
        b[15] = 0;
        // Leave garbage where the XSDT would be, to prove it is not read.
        b[24..32].copy_from_slice(&0xdead_beef_dead_beefu64.to_le_bytes());
        seal(&mut b, 8, RSDP_V1_LEN);
        let r = parse_rsdp(&b).expect("a 20-byte RSDP is still an RSDP");
        assert_eq!(r.xsdt, 0);
        assert_eq!(r.root_table(), (0x7ffe_1a40, false));
    }

    /// Build a well-formed table: signature, length, revision, a sealed checksum at offset 9, and
    /// a vendor name at offset 10.
    fn sdt(signature: &[u8; 4], body: &[u8]) -> [u8; 128] {
        let mut t = [0u8; 128];
        let len = SDT_HEADER_LEN + body.len();
        assert!(len <= 128);
        t[0..4].copy_from_slice(signature);
        t[4..8].copy_from_slice(&(len as u32).to_le_bytes());
        t[8] = 1;
        t[10..16].copy_from_slice(b"BOCHS ");
        t[SDT_HEADER_LEN..len].copy_from_slice(body);
        seal(&mut t, 9, len);
        t
    }

    /// A table header's length must at least cover its own header, or the body length underflows.
    #[test]
    fn a_length_shorter_than_the_header_is_refused() {
        let mut t = sdt(b"APIC", &[]);
        t[4..8].copy_from_slice(&10u32.to_le_bytes());
        assert_eq!(parse_sdt_header(&t), Err(AcpiError::BadLength(10)));
    }

    /// The header's fields land where the specification puts them.
    #[test]
    fn a_table_header_decodes() {
        let t = sdt(b"MCFG", &[0u8; 24]);
        let h = parse_sdt_header(&t).expect("well-formed");
        assert_eq!(h.signature_str(), Some("MCFG"));
        assert_eq!(h.length as usize, SDT_HEADER_LEN + 24);
        assert_eq!(h.body_len(), 24);
        assert_eq!(
            &h.oem_id, b"BOCHS ",
            "the vendor name starts at offset 10, after the checksum byte"
        );
        assert!(
            checksum_ok(&t[..h.length as usize]),
            "a sealed table sums to zero"
        );
    }

    /// Root-table entries are 4 bytes in an RSDT and 8 in an XSDT, and running off the end is
    /// `None` rather than a garbage pointer.
    #[test]
    fn root_entries_are_four_or_eight_bytes_and_the_end_is_none() {
        let body32 = [0x40u8, 0x1a, 0xfe, 0x7f, 0x00, 0x1b, 0xfe, 0x7f];
        assert_eq!(root_entry(&body32, 0, false), Some(0x7ffe_1a40));
        assert_eq!(root_entry(&body32, 1, false), Some(0x7ffe_1b00));
        assert_eq!(root_entry(&body32, 2, false), None);
        assert_eq!(root_entry_count(SDT_HEADER_LEN as u32 + 8, false), 2);
        assert_eq!(root_entry_count(SDT_HEADER_LEN as u32 + 8, true), 1);
    }

    /// The MADT QEMU's `q35` produces for one CPU: a processor, an IO APIC, and the two interrupt
    /// source overrides every PC has.
    fn q35_madt_body() -> [u8; 8 + 8 + 12 + 10 + 10] {
        let mut b = [0u8; 8 + 8 + 12 + 10 + 10];
        b[0..4].copy_from_slice(&0xfee0_0000u32.to_le_bytes());
        b[4..8].copy_from_slice(&MADT_PCAT_COMPAT.to_le_bytes());
        // Local APIC: processor 0, apic id 0, enabled.
        b[8] = 0;
        b[9] = 8;
        b[10] = 0;
        b[11] = 0;
        b[12..16].copy_from_slice(&1u32.to_le_bytes());
        // IO APIC: id 0 at 0xfec00000, gsi base 0.
        b[16] = 1;
        b[17] = 12;
        b[18] = 0;
        b[20..24].copy_from_slice(&0xfec0_0000u32.to_le_bytes());
        b[24..28].copy_from_slice(&0u32.to_le_bytes());
        // Interrupt source override: ISA IRQ 0 arrives as GSI 2.
        b[28] = 2;
        b[29] = 10;
        b[30] = 0; // bus 0 = ISA
        b[31] = 0; // source: IRQ 0
        b[32..36].copy_from_slice(&2u32.to_le_bytes());
        b[36..38].copy_from_slice(&0u16.to_le_bytes());
        // Interrupt source override: ISA IRQ 5, active high, level triggered.
        b[38] = 2;
        b[39] = 10;
        b[40] = 0;
        b[41] = 5;
        b[42..46].copy_from_slice(&5u32.to_le_bytes());
        b[46..48].copy_from_slice(&0x000du16.to_le_bytes());
        b
    }

    /// The fixed part, and the PICs the machine still has.
    #[test]
    fn the_madt_reports_the_local_apic_and_whether_the_8259s_are_there() {
        let m = parse_madt(&q35_madt_body()).expect("well-formed");
        assert_eq!(m.local_apic, 0xfee0_0000);
        assert_ne!(
            m.flags & MADT_PCAT_COMPAT,
            0,
            "a PC still has 8259s and they must be masked before the APICs are used"
        );
    }

    /// **The entry list decodes, including the overrides that make legacy IRQ numbers a lie.** On
    /// almost every machine the timer's IRQ 0 arrives as GSI 2, and a kernel that armed input 0
    /// would arm nothing.
    #[test]
    fn the_madt_entries_decode_and_the_overrides_rewire_legacy_irqs() {
        let body = q35_madt_body();
        let mut it = madt_entries(&body);
        assert_eq!(
            it.next(),
            Some(MadtEntry::LocalApic {
                processor_id: 0,
                apic_id: 0,
                enabled: true,
                online_capable: false,
            })
        );
        assert_eq!(
            it.next(),
            Some(MadtEntry::IoApic {
                id: 0,
                address: 0xfec0_0000,
                gsi_base: 0,
            })
        );
        assert_eq!(
            it.next(),
            Some(MadtEntry::InterruptSourceOverride {
                bus: 0,
                source: 0,
                gsi: 2,
                flags: 0,
            }),
            "IRQ 0 is remapped on essentially every PC"
        );
        assert_eq!(
            it.next(),
            Some(MadtEntry::InterruptSourceOverride {
                bus: 0,
                source: 5,
                gsi: 5,
                flags: 0x000d,
            })
        );
        assert_eq!(it.next(), None);
    }

    /// **A malformed entry length ends the walk rather than looping forever.** A length of zero
    /// would never advance, and a length past the end would read the next entry from the middle of
    /// this one.
    #[test]
    fn a_zero_or_overlong_entry_length_ends_the_walk() {
        let mut body = q35_madt_body();
        body[9] = 0; // the first entry claims zero length
        assert_eq!(madt_entries(&body).count(), 0);

        let mut body = q35_madt_body();
        body[17] = 200; // the second entry claims to run past the table
        assert_eq!(
            madt_entries(&body).count(),
            1,
            "the entries before the bad one are still reported"
        );
    }

    /// A CPU that is present but not enabled is not startable, and online-capable is a third state
    /// rather than a synonym for enabled.
    #[test]
    fn a_disabled_processor_is_reported_as_disabled() {
        let mut body = q35_madt_body();
        body[12..16].copy_from_slice(&2u32.to_le_bytes()); // online-capable, not enabled
        let first = madt_entries(&body).next().expect("an entry");
        assert_eq!(
            first,
            MadtEntry::LocalApic {
                processor_id: 0,
                apic_id: 0,
                enabled: false,
                online_capable: true,
            }
        );
    }

    /// **The trap, resolved.** IRQ 0 is the timer and it is not IO APIC input 0; the override says
    /// it arrives as GSI 2. A kernel that armed redirection entry 0 would arm the 8259 cascade and
    /// see nothing at all, with no error anywhere to say why.
    #[test]
    fn the_timers_irq_0_resolves_to_gsi_2() {
        let table = isa_irq_table(&q35_madt_body());
        assert_eq!(
            table[0],
            IsaIrqRouting {
                gsi: 2,
                active_low: false,
                level_triggered: false,
            },
            "IRQ 0 is remapped on essentially every PC"
        );
    }

    /// An IRQ with no override keeps its number and the ISA bus's own polarity and trigger mode.
    #[test]
    fn an_irq_with_no_override_is_identity_mapped_edge_triggered_and_active_high() {
        let table = isa_irq_table(&q35_madt_body());
        for irq in [1usize, 4, 8, 15] {
            assert_eq!(
                table[irq],
                IsaIrqRouting::isa_default(irq as u8),
                "IRQ {irq} has no override in this MADT"
            );
        }
        assert_eq!(table[4].gsi, 4, "COM1 really is GSI 4 on a PC");
    }

    /// **The flags word is two two-bit fields, and reading it as two one-bit flags gets both
    /// wrong.** `0x000d` is `0b1101`: bits 1:0 are `01`, active *high*, and bits 3:2 are `11`,
    /// level triggered. A decoder that tested bit 1 for polarity and bit 3 for trigger would answer
    /// "active low, level" here, which is a redirection entry that never fires.
    #[test]
    fn the_inti_flags_are_two_two_bit_fields_not_two_bits() {
        let table = isa_irq_table(&q35_madt_body());
        assert_eq!(
            table[5],
            IsaIrqRouting {
                gsi: 5,
                active_low: false,
                level_triggered: true,
            },
        );
    }

    /// Active low is `11` in bits 1:0, and it is the encoding a PCI line uses.
    #[test]
    fn active_low_is_the_11_encoding() {
        let mut body = q35_madt_body();
        body[46..48].copy_from_slice(&0b1111u16.to_le_bytes());
        let table = isa_irq_table(&body);
        assert!(table[5].active_low);
        assert!(table[5].level_triggered);
    }

    /// **A `00` field means "conforms to the bus", not "active high, edge" by accident.** The two
    /// happen to coincide on the ISA bus, and the test exists so that a future reader changing the
    /// default cannot do it in only one of the two places.
    #[test]
    fn a_conforms_to_the_bus_override_keeps_the_isa_defaults_but_takes_the_new_gsi() {
        let mut body = q35_madt_body();
        // Rewrite the second override: IRQ 9 -> GSI 9, flags 0 (conforms).
        body[41] = 9;
        body[42..46].copy_from_slice(&9u32.to_le_bytes());
        body[46..48].copy_from_slice(&0u16.to_le_bytes());
        let table = isa_irq_table(&body);
        assert_eq!(table[9], IsaIrqRouting::isa_default(9));
        assert_eq!(
            table[5],
            IsaIrqRouting::isa_default(5),
            "IRQ 5 is back to itself"
        );
    }

    /// An override naming a source outside the sixteen legacy IRQs is a malformed table, not a
    /// seventeenth IRQ, and must not index past the end of the table.
    #[test]
    fn an_override_for_a_source_above_15_is_ignored() {
        let mut body = q35_madt_body();
        body[41] = 200;
        let table = isa_irq_table(&body);
        for irq in 1..ISA_IRQ_COUNT {
            assert_eq!(table[irq], IsaIrqRouting::isa_default(irq as u8));
        }
        assert_eq!(table[0].gsi, 2, "the well-formed override still applied");
    }

    /// The MCFG entry q35 produces: bus 0 through 255 at 0xb0000000.
    #[test]
    fn the_mcfg_gives_the_ecam_window_and_its_size() {
        let mut body = [0u8; MCFG_FIXED_LEN + MCFG_ENTRY_LEN];
        body[8..16].copy_from_slice(&0xb000_0000u64.to_le_bytes());
        body[16..18].copy_from_slice(&0u16.to_le_bytes());
        body[18] = 0;
        body[19] = 255;
        let e = mcfg_entry(&body, 0).expect("one window");
        assert_eq!(e.base, 0xb000_0000);
        assert_eq!(e.start_bus, 0);
        assert_eq!(e.end_bus, 255);
        assert_eq!(e.size(), 256 * 0x10_0000, "256 buses of 1 MiB each");
        assert_eq!(mcfg_entry(&body, 1), None);
    }
}
