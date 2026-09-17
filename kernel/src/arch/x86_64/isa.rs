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

use core::arch::x86_64::CpuidResult;

/// **`CPUID`, spelled so that two toolchains a year apart both accept it** (milestone 304).
///
/// `core::arch::x86_64::__cpuid` is a *safe* function on the toolchain this tree pins, because the
/// instruction has no precondition on a 64-bit part; every call site here used to read it bare and
/// say so in a comment. It was an `unsafe fn` until upstream made it safe, and **Kani bundles its
/// own rustc**, ten months behind ours (`kani-0.67.0` pins `nightly-2025-11-21`, rustc 1.93.0). So
/// the bare calls were four `E0133`s under the prover, and `arch/x86_64/` did not compile there at
/// all: the whole subtree was out of reach of the model checker for four missing keywords.
///
/// The `unsafe` block satisfies the old toolchain; `allow(unused_unsafe)` satisfies the new one,
/// where the block is redundant and `script/lint`'s `-D warnings` would otherwise reject it. That
/// allow is the **exception this file owes a reader** (AGENTS.md's ladder): it is load-bearing for
/// the prover and a foot gun for anyone who reads it as "this call needs auditing". Delete it when
/// Kani's pinned rustc passes 2026-02-ish, and the bare call comes back.
///
/// Name provisional (milestone 304): `cpuid` is the instruction's own mnemonic, which AGENTS.md's
/// naming section keeps rather than expands.
#[allow(unused_unsafe)]
pub(super) fn cpuid(leaf: u32) -> CpuidResult {
    // SAFETY: `CPUID` is architected on every 64-bit x86 part and has no precondition; a leaf the
    // part does not implement answers with another leaf's data rather than faulting, which is why
    // the callers that need one do a max-leaf check of their own. Under the pinned toolchain this
    // block is redundant, which is what the allow above is for.
    unsafe { core::arch::x86_64::__cpuid(leaf) }
}

/// `CPUID` with a subleaf. Same toolchain-skew reasoning as [`cpuid`]; see its comment.
#[allow(unused_unsafe)]
pub(super) fn cpuid_count(leaf: u32, sub_leaf: u32) -> CpuidResult {
    // SAFETY: as [`cpuid`].
    unsafe { core::arch::x86_64::__cpuid_count(leaf, sub_leaf) }
}

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
    // Leaf 0 needs no maximum-leaf check first, because leaf 0 is what reports the maximum. See
    // `cpuid` above for why this is a helper rather than the bare intrinsic.
    let leaf0 = cpuid(0);
    let mut vendor = [0u8; 12];
    vendor[0..4].copy_from_slice(&leaf0.ebx.to_le_bytes());
    vendor[4..8].copy_from_slice(&leaf0.edx.to_le_bytes());
    vendor[8..12].copy_from_slice(&leaf0.ecx.to_le_bytes());

    // Leaf 7 needs a maximum-leaf check first, unlike leaf 0: a part that does not implement it
    // answers with whatever it does implement's data rather than refusing, so an unchecked read
    // would misattribute another leaf's bits to RDSEED.
    let rdseed = leaf0.eax >= 7 && (cpuid_count(7, 0).ebx & (1 << 18)) != 0;

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
/// `components/src/entropy.rs::instr` for the identical constant and reasoning on the userspace side,
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

/// `CPUID` leaf `0x15`'s TSC/core-crystal-clock ratio, converted to a TSC rate in Hz, on the
/// parts that report one directly (milestone 161's `cntfrq` follow-up: the "ask the CPU first"
/// half of `arch::x86_64::timer::init_frequency`, whose docs explain why the fallback -
/// calibrating against the PIT - exists at all).
///
/// `None` in two cases, and both are real "this part does not say" answers rather than errors:
/// the part's own maximum leaf (from [`init`]'s leaf 0 read) is below `0x15`, so reading it would
/// misattribute a lower leaf's bits the same way an unchecked leaf-7 read would (see [`init`]'s
/// own comment on `rdseed`); or the part implements the leaf but leaves `ECX` (the crystal
/// clock's own frequency) at zero, which the SDM defines as "not enumerated" and which would
/// otherwise need a vendor-specific model-to-crystal table this crate does not carry (Linux's
/// `native_calibrate_tsc` has one; this port does not, and falls back to calibration instead).
///
/// Measured empirically under this project's own QEMU invocation (`-cpu max`, TCG): `max_leaf` is
/// `0xd`, so this returns `None` on every boot this kernel currently runs, and
/// `init_frequency`'s PIT-calibration fallback is what every test and every boot print's number
/// actually comes from today. The leaf is read anyway, for real hardware (milestone 87's Dell)
/// where it may be populated.
pub fn tsc_crystal_hz() -> Option<u64> {
    if get().max_leaf < 0x15 {
        return None;
    }
    // Leaf 0x15 carries no precondition beyond the max-leaf check just above.
    let leaf = cpuid(0x15);
    if leaf.eax == 0 || leaf.ebx == 0 || leaf.ecx == 0 {
        return None;
    }
    // SDM Vol. 3B §19.7.3: TSC frequency = ECX (crystal Hz) * EBX (numerator) / EAX (denominator).
    Some((leaf.ecx as u64) * (leaf.ebx as u64) / (leaf.eax as u64))
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
