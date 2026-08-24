//! **What CPU is this**, `x86_64`. The third implementation of the arch contract aarch64 answers from
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

use core::arch::x86_64::{__cpuid, __cpuid_count};

/// What this machine is, as far as this port has learned to ask.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Isa {
    /// The 12-character vendor string CPUID leaf 0 returns: "`GenuineIntel`", "`AuthenticAMD`", or on
    /// QEMU whatever `-cpu` was asked for.
    pub vendor: [u8; 12],
    /// The maximum standard CPUID leaf this part answers.
    pub max_leaf: u32,
    /// `CPUID` leaf 7, subleaf 0, `EBX` bit 18: does this part implement `RDSEED` (milestone 162)?
    /// Checked before [`draw_rdseed`] is ever called, the same discipline aarch64's
    /// `ID_AA64ISAR0_EL1.RNDR` check is (see `kernel/src/arch/aarch64/isa.rs`): unlike aarch64,
    /// `RDSEED` on an unsupporting part is simply `#UD` rather than a specific "not implemented"
    /// trap, so there is no honest way to learn this except by asking first.
    pub rdseed: bool,
}

static mut ISA: Isa = Isa {
    vendor: [0; 12],
    max_leaf: 0,
    rdseed: false,
};

/// Read what this machine is. `boot_info_pointer` is ignored: the argument is the portable arch
/// contract's (both other architectures are handed a device tree), and x86 answers from the
/// instruction set itself. Named rather than dropped so the seam stays one shape across three
/// architectures.
pub fn init(boot_info_pointer: usize) {
    let _ = boot_info_pointer;
    // `__cpuid` is a safe function in `core::arch::x86_64` (the instruction has no precondition on
    // a 64-bit part), so there is no `unsafe` block here and none is needed. Leaf 0 in particular
    // needs no maximum-leaf check first, because leaf 0 is what reports the maximum.
    let leaf0 = __cpuid(0);
    let mut vendor = [0u8; 12];
    vendor[0..4].copy_from_slice(&leaf0.ebx.to_le_bytes());
    vendor[4..8].copy_from_slice(&leaf0.edx.to_le_bytes());
    vendor[8..12].copy_from_slice(&leaf0.ecx.to_le_bytes());

    // Leaf 7 needs a maximum-leaf check first, unlike leaf 0: a part that does not implement it
    // answers with whatever it does implement's data rather than refusing, so an unchecked read
    // would misattribute another leaf's bits to RDSEED.
    let rdseed = leaf0.eax >= 7 && (__cpuid_count(7, 0).ebx & (1 << 18)) != 0;

    // SAFETY: single-threaded boot code, before any secondary CPU exists.
    unsafe {
        ISA = Isa {
            vendor,
            max_leaf: leaf0.eax,
            rdseed,
        };
    }
}

/// **Draw eight bytes with `RDSEED`, retrying a transient "no data this cycle" result.**
///
/// `None` if [`get`]`().rdseed` is false (never execute the instruction without checking first: on
/// a part that lacks it, `RDSEED` is `#UD`, and this kernel has no exception recovery path for a
/// probe that was told the answer already) or if the source stayed dry across every attempt.
///
/// The retry count and the `pause` between attempts are Intel's own guidance for `RDSEED`
/// specifically (DRNG Software Implementation Guide rev. 2.2, §5.3.1.2): an "asynchronous
/// application" should give up after "somewhere between 1 and 100" retries. This is a one-shot
/// boot-tour probe rather than a service under load, so the high end costs nothing; see
/// `user/src/entropy.rs::instr` for the identical constant and reasoning on the userspace side,
/// which this kernel-side copy exists only because ring 3 does not exist yet (milestone 161).
pub fn draw_rdseed() -> Option<u64> {
    if !get().rdseed {
        return None;
    }
    const RETRIES: u32 = 100;
    for _ in 0..RETRIES {
        let v: u64;
        let ok: u8;
        // SAFETY: `rdseed` is unprivileged at any ring and touches no memory; `get().rdseed` above
        // confirmed CPUID leaf 7 EBX bit 18, so the instruction is not `#UD` here.
        unsafe {
            core::arch::asm!(
                "rdseed {v}",
                "setc {ok}",
                v = out(reg) v,
                ok = out(reg_byte) ok,
                options(nomem, nostack),
            );
        }
        if ok != 0 {
            return Some(v);
        }
        // SAFETY: `pause` touches no memory and has no failure mode.
        unsafe { core::arch::asm!("pause", options(nomem, nostack)) };
    }
    None
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
