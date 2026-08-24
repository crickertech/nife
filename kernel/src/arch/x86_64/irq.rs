//! **The interrupt controller, `x86_64`.** The local APIC and the IO APIC. Not built.
//!
//! # What it replaces, and the one structural difference
//!
//! aarch64 has a GIC and RISC-V a PLIC, and both are a single distributor plus per-CPU interfaces
//! found in the device tree. x86 has a **pair**: an IO APIC that takes device interrupt lines and
//! routes them, and a local APIC per CPU that receives them, and the local APIC is also how one CPU
//! interrupts another (the IPI mechanism, which is what SMP bring-up itself rides on).
//!
//! The structural difference from both is that on x86 the interrupt controller is *also the SMP
//! bring-up mechanism*. There is no PSCI `CPU_ON` and no SBI `hart_start`: a secondary CPU is
//! started by sending it INIT, then two STARTUP IPIs through the local APIC, each naming a physical
//! page below 1 MiB to begin executing at **in 16-bit real mode**. That is why this module and the
//! SMP step are one piece of work rather than two.
//!
//! Discovery is the same blocker `iommu.rs` records: which IO APIC, at what address, with what
//! global interrupt base, and which legacy IRQ is remapped where, is all in the ACPI MADT.

macro_rules! not_yet {
    ($name:literal) => {
        unimplemented!(concat!(
            "x86_64 irq::",
            $name,
            ": the APIC is not built, and discovery is blocked on ACPI MADT parsing (milestone 161)"
        ))
    };
}

/// Bring up the interrupt controller.
#[allow(dead_code)]
pub fn init() {
    not_yet!("init")
}

/// Bring up this CPU's local interrupt interface.
#[allow(dead_code)]
pub fn init_this_cpu() {
    not_yet!("init_this_cpu")
}

/// Unmask interrupt `intid` at the controller.
#[allow(dead_code)]
pub fn enable(intid: u32) {
    let _ = intid;
    not_yet!("enable")
}

/// Send a reschedule inter-processor interrupt to `target_cpu`.
#[allow(dead_code)]
pub fn send_reschedule(target_cpu: usize) {
    let _ = target_cpu;
    not_yet!("send_reschedule")
}
