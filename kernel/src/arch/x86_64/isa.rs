//! **What CPU is this**, x86_64. The third implementation of the arch contract aarch64 answers from
//! `MIDR_EL1` and RISC-V from the device tree plus `mvendorid`.
//!
//! x86 has had one answer since 1993 and it is a good one: `CPUID`, an instruction that returns
//! structured data about the part, including a 48-character brand string the vendor wrote. There is
//! no device tree to consult and no firmware call to make, which makes this the *easiest* of the
//! three rather than the hardest.
//!
//! # BUGS
//!
//! - **Nothing is recorded or checked yet.** The other two implementations gate the boot on the
//!   features the kernel actually uses (RISC-V refuses a firmware without the SBI extensions it
//!   calls). The x86 equivalents worth gating on are NX, SYSCALL, and the invariant TSC; this
//!   reports and does not refuse. See design/roadmap/161-x86-64-kernel-port.md.

use core::arch::x86_64::__cpuid;

/// What this machine is, as far as this port has learned to ask.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Isa {
    /// The 12-character vendor string CPUID leaf 0 returns: "GenuineIntel", "AuthenticAMD", or on
    /// QEMU whatever `-cpu` was asked for.
    pub vendor: [u8; 12],
    /// The maximum standard CPUID leaf this part answers.
    pub max_leaf: u32,
}

static mut ISA: Isa = Isa {
    vendor: [0; 12],
    max_leaf: 0,
};

/// Read what this machine is. `dtb_ptr` is ignored: the argument is the portable arch contract's
/// (both other architectures are handed a device tree), and x86 answers from the instruction set
/// itself. Named rather than dropped so the seam stays one shape across three architectures.
pub fn init(dtb_ptr: usize) {
    let _ = dtb_ptr;
    // `__cpuid` is a safe function in `core::arch::x86_64` (the instruction has no precondition on
    // a 64-bit part), so there is no `unsafe` block here and none is needed. Leaf 0 in particular
    // needs no maximum-leaf check first, because leaf 0 is what reports the maximum.
    let leaf0 = __cpuid(0);
    let mut vendor = [0u8; 12];
    vendor[0..4].copy_from_slice(&leaf0.ebx.to_le_bytes());
    vendor[4..8].copy_from_slice(&leaf0.edx.to_le_bytes());
    vendor[8..12].copy_from_slice(&leaf0.ecx.to_le_bytes());
    // SAFETY: single-threaded boot code, before any secondary CPU exists.
    unsafe {
        ISA = Isa {
            vendor,
            max_leaf: leaf0.eax,
        };
    }
}

/// What [`init`] found.
pub fn get() -> Isa {
    // SAFETY: written once by `init` during single-threaded boot and read-only thereafter.
    unsafe { ISA }
}

/// Print one line about the machine, on every boot, beside the other summaries.
pub fn print_summary() {
    let isa = get();
    let vendor = core::str::from_utf8(&isa.vendor).unwrap_or("<not utf-8>");
    crate::println!(
        "  cpu         : x86_64, vendor {vendor}, cpuid leaves 0..{:#x}",
        isa.max_leaf
    );
}
