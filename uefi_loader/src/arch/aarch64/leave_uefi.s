// Leaving UEFI on aarch64: the last thing the loader does, after ExitBootServices.
//
// THE CONTRACT (AAPCS64, `extern "C"`):
//
//   x0 = the kernel's entry point, physical.
//   x1 = the device tree the kernel is to read, physical.
//
// It does not return. The caller has already cleaned and invalidated the data cache to the point of
// coherency over the kernel, the archive and the device tree, so what the kernel reads with its
// caches off is what the loader wrote.
//
// WHAT IT LEAVES is the Linux arm64 boot state the kernel's boot.s expects (and that U-Boot's
// `booti` produces): interrupts masked, the MMU off, the data cache off, x0 = the device tree,
// x1..x3 zero, at whatever exception level the firmware ran us at. The kernel drops itself from EL2
// to EL1 (`enter_el1` in kernel/src/arch/aarch64/boot.s), so this does not.
//
// WHY TURNING THE MMU OFF HERE IS SAFE: UEFI runs with an identity map (UEFI 2.10, 2.3.6), so the
// instruction after the `msr sctlr` is fetched from the same physical address with translation on
// or off. Nothing below touches memory.

.text
// No `.type` or `.size`: the UEFI target is PE/COFF, and those are ELF directives.
.global aarch64_leave_uefi
aarch64_leave_uefi:
    msr     daifset, #0xf           // no interrupt may arrive at vectors that are about to go

    mov     x19, x0                 // entry
    mov     x20, x1                 // device tree

    mrs     x9, CurrentEL
    lsr     x9, x9, #2
    cmp     x9, #2
    b.eq    1f

    // EL1: SCTLR_EL1.M (bit 0) off, SCTLR_EL1.C (bit 2) off.
    mrs     x10, sctlr_el1
    bic     x10, x10, #(1 << 0)
    bic     x10, x10, #(1 << 2)
    msr     sctlr_el1, x10
    isb
    b       2f

1:  // EL2: the same two bits in SCTLR_EL2, which is the regime a U-Boot `bootefi` runs us in.
    mrs     x10, sctlr_el2
    bic     x10, x10, #(1 << 0)
    bic     x10, x10, #(1 << 2)
    msr     sctlr_el2, x10
    isb

2:  // The instruction cache may hold lines of the kernel's addresses from before the copy.
    ic      iallu
    dsb     nsh
    isb

    mov     x0, x20
    mov     x1, xzr
    mov     x2, xzr
    mov     x3, xzr
    br      x19
